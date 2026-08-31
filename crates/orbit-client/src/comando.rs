//! El catálogo de órdenes que el cliente puede pedirle a un servidor.
//!
//! Es finito y está en un solo fichero **a propósito**: la regla nº 1 dice que
//! la interfaz sólo invoca `orbit`, y una regla así sólo es auditable si se
//! puede comprobar leyendo un sitio. Si esta lista crece, la superficie crece —
//! y crece en silencio si está repartida por veinte pantallas.
//!
//! La interfaz **no construye un `argv` a mano**. Construye un [`Comando`], y
//! este módulo lo traduce. El motivo es de corrección antes que de seguridad: un
//! `argv` escrito a mano en veinte sitios se equivoca en el sitio diecinueve, y
//! equivocarse aquí significa concatenar.
//!
//! Y hay reglas del servidor que aquí se convierten en imposibles de expresar,
//! que es mejor que convertirlas en errores en tiempo de ejecución:
//!
//! - `orbit deploy --json` **aborta sin el nombre de la app**, y `--pick` está
//!   prohibido con `--json`. Aquí el nombre es obligatorio en el tipo.
//! - Sin terminal, un comando sin app **no aborta: elige la primera por orden
//!   alfabético y sale con 0**. Con `restart` eso es reiniciar la app
//!   equivocada sin que nada lo diga. Por eso **ninguna variante de aquí puede
//!   omitir la app**.
//! - `--json` va **siempre delante**. Detrás, un comando que no lo habla se lo
//!   traga en silencio y sale con 0, porque no todos filtran lo que no conocen.
//!   Delante siempre muere, que es un comportamiento definido.

use crate::shquote;

/// Los campos con forma conocida se validan **antes** de construir la orden.
///
/// Esto es defensa en profundidad, **no** la defensa: el escapado es lo que
/// protege, y esto va encima. El orden importa —escapar siempre, validar
/// además—; quien lo hace al revés acaba con un filtro que crece cada vez que
/// alguien encuentra un carácter nuevo, que es la firma de que el diseño estaba
/// mal.
///
/// Lo que sí aporta: convierte un ataque en un mensaje de error legible, y
/// atrapa el caso que el escapado no puede atrapar — un nombre de app que llega
/// **del propio servidor**. `app_names()` en Orbit enumera los `.conf` sin
/// filtrar, así que un servidor con root comprometido puede meter ahí lo que
/// quiera. **Un dato que ha dado la vuelta por el servidor no es de más
/// confianza que uno tecleado: es de menos.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorForma {
    NombreDeApp(String),
    ClaveDeEntorno(String),
    Release(String),
    /// `orbit exec` sin comando abre un `bash` interactivo. Un cliente sin
    /// terminal no puede con eso, y fingir medio terminal es peor que no
    /// ofrecerlo: lo que se ofrece en su lugar es la orden `ssh` para pegarla
    /// en un terminal de verdad.
    ExecSinComando,
    Escapado(shquote::ErrorEscapado),
}

impl std::fmt::Display for ErrorForma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NombreDeApp(s) => write!(f, "«{s}» no tiene forma de nombre de app"),
            Self::ClaveDeEntorno(s) => write!(f, "«{s}» no tiene forma de variable de entorno"),
            Self::Release(s) => write!(f, "«{s}» no tiene forma de release"),
            Self::ExecSinComando => write!(
                f,
                "«orbit exec» sin comando abre una shell interactiva, y aquí no hay terminal"
            ),
            Self::Escapado(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ErrorForma {}

/// Copiada de `_app_name_ok` del Orbit real: `^[a-z0-9][a-z0-9._-]{0,39}$`, sin
/// `..`. Se copia en vez de inventarse porque el servidor es quien manda sobre
/// qué nombres existen.
///
/// Un nombre que no encaje **no se arregla**: un nombre arreglado ya no
/// identifica a nadie. Se enseña marcado y no se opera sobre él.
pub fn nombre_de_app_valido(s: &str) -> bool {
    if s.is_empty() || s.len() > 40 || s.contains("..") {
        return false;
    }
    let mut cs = s.chars();
    let primero = cs.next().unwrap();
    if !(primero.is_ascii_lowercase() || primero.is_ascii_digit()) {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
}

/// Copiada de `_env_key_ok`: `^[A-Za-z_][A-Za-z0-9_]*$`.
pub fn clave_de_entorno_valida(s: &str) -> bool {
    let mut cs = s.chars();
    match cs.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Deducida del formato `%Y%m%d-%H%M%S` con sufijo `-2`, `-3`… para las
/// colisiones dentro del mismo segundo.
pub fn release_valida(s: &str) -> bool {
    let (fecha, resto) = match s.split_once('-') {
        Some(v) => v,
        None => return false,
    };
    if fecha.len() != 8 || !fecha.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let (hora, sufijo) = match resto.split_once('-') {
        Some((h, s)) => (h, Some(s)),
        None => (resto, None),
    };
    if hora.len() != 6 || !hora.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    match sufijo {
        None => true,
        Some(s) => !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()),
    }
}

/// Cómo viaja lo que alguien escribe en la pantalla de `exec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModoDeExec {
    /// Argumentos separados. **No pasa por ningún shell**: el servidor ejecuta
    /// el `argv` tal cual, así que un `&&` es un argumento literal y no un
    /// operador. Es el modo por defecto porque es el que no sorprende.
    Argumentos(Vec<String>),
    /// Un solo argumento con el texto entero. El servidor lo reconoce por su
    /// regla y lo pasa a `bash -lc`, así que aquí `&&`, `|` y `$(…)` **sí**
    /// hacen lo que parece.
    Shell(String),
}

/// Qué se le puede pedir a un servidor. **Este enumerado es la superficie.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comando {
    /// El saludo. Lo primero que se pregunta siempre.
    Version,
    Lista,
    /// Trae el array de apps **completo e idéntico** al de `Lista`, comprobado
    /// carácter a carácter. Así la portada cuesta una llamada y no dos.
    Estado,
    Info {
        app: String,
    },
    Doctor,
    /// Aplica lo que se puede arreglar sin decidir nada por nadie.
    ///
    /// Lleva `--yes` porque el servidor lo exige con `--json`: sin terminal no
    /// hay a quién preguntar, y dar por hecho que alguien ha dicho que sí sería
    /// aplicar cambios en su servidor sin que los acepte. La confirmación la
    /// pide la interfaz, que sí tiene delante a una persona.
    DoctorArreglar,
    /// Tarda ~1 s a propósito: la CPU es la diferencia entre dos lecturas, y una
    /// foto suelta tiene que esperar a la segunda. Con 40 apps son ~2,1 s.
    Top,
    Metricas {
        app: Option<String>,
    },
    Trafico {
        app: String,
        desde: Option<String>,
    },
    /// Devuelve sólo los **nombres**. Los secretos no cruzan el contrato.
    EntornoLista {
        app: String,
    },
    /// Un valor, pelado. Es un acto deliberado por comando, y así debe seguir.
    EntornoValor {
        app: String,
        clave: String,
    },
    BaseDeDatosLista,
    RedireccionesLista {
        app: Option<String>,
    },
    VigilanciaEstado,
    ColaEstado,
    CopiasLista,
    CopiasVerificar,
    /// NDJSON, y es la única excepción del contrato. Con `--json` no sigue en
    /// vivo salvo que se pida.
    Logs {
        app: String,
        desde: Option<String>,
        lineas: Option<u32>,
        seguir: bool,
        solo_nginx: bool,
    },
    /// El nombre es obligatorio: con `--json` el servidor aborta sin él.
    Desplegar {
        app: String,
        progreso: bool,
    },
    DesplegarTodo {
        progreso: bool,
    },
    /// Ejecuta algo dentro de una app.
    ///
    /// **Es la puerta trasera, y se trata como tal.** No se usa para nada de la
    /// interfaz —ni para leer un fichero, ni para contar releases, ni para
    /// tapar un hueco del contrato—, porque el día que se use, el cliente deja
    /// de hablar el contrato y pasa a hablar Bash contra un servidor cuyo
    /// layout puede cambiar.
    ///
    /// Los dos modos existen porque el servidor tiene una **regla del argumento
    /// único**: si le llega uno solo y contiene metacaracteres, lo ejecuta con
    /// `bash -lc`; con dos o más, ejecuta el `argv` tal cual. O sea que
    /// `exec web "ls -la"` y `exec web ls -la` **no son lo mismo**.
    ///
    /// Aplicar esa heurística por dentro sería lo peor de las tres opciones:
    /// quien escribe no podría predecir cuándo su `&&` se ejecuta y cuándo se
    /// pasa como texto, y **una herramienta de depuración que no es predecible
    /// tampoco sirve**. Así que se elige, y se ve.
    ///
    /// Nótese lo que esto **no** es: aunque uno de los modos acabe en un shell
    /// remoto, el cliente **nunca construye una cadena de shell**. Pasa N
    /// argumentos o pasa uno, y los dos caminos van por el mismo escapador. La
    /// decisión de meter eso en un shell la toma Orbit, no nosotros.
    Ejecutar {
        app: String,
        modo: ModoDeExec,
    },
    /// Retira una app **sin borrar sus datos**. Reversible.
    ///
    /// Quita el vhost, la unidad de systemd, el pool de php-fpm y el descriptor
    /// de `/etc/orbit`. Todo eso se rehace con `orbit new` en un minuto, y por
    /// eso es una operación distinta de la de abajo — no una casilla al lado.
    ///
    /// Lleva `-y` porque el cliente no tiene terminal donde contestar, y **eso
    /// significa que la pregunta que Orbit hacía la tiene que hacer la
    /// interfaz**. `-y` quiere decir «acepta el valor por defecto», y el valor
    /// por defecto del borrado de datos es «no»: por eso esto NO borra nada.
    Retirar {
        app: String,
    },
    /// Retira una app **y borra sus datos**. Irreversible.
    ///
    /// `rm -rf /srv/apps/<app>` se lleva el `.env`, todas las releases y **las
    /// subidas de los usuarios finales** —que viven en `shared/` precisamente
    /// para sobrevivir a los despliegues— y con `--purge` también el usuario de
    /// sistema. Eso no vuelve.
    ///
    /// Y aquí está el hallazgo que cambia el diseño de toda la pantalla:
    /// **`orbit remove -y --purge` no pregunta absolutamente nada**. El «escribe
    /// el nombre» de Orbit sólo ocurre sin `-y`, y `--purge` cortocircuita la
    /// segunda pregunta. Como el cliente tiene que pasar `-y`, **toda la
    /// protección se traslada aquí**:
    ///
    /// > Aquí no hay red debajo. Si esa pantalla se equivoca, no hay una segunda
    /// > pregunta en el servidor que la pare.
    RetirarYBorrar {
        app: String,
    },
    /// La release es obligatoria: sin ella y sin terminal, `rollback` aborta —y
    /// hace bien, porque el «valor por defecto» sería la que ya está activa.
    Revertir {
        app: String,
        release: String,
    },
}

impl Comando {
    /// El `argv` completo, con la ruta absoluta del binario delante.
    ///
    /// **Nunca por `PATH`**: un `PATH` manipulado en el `.bashrc` del usuario
    /// remoto —o por quien sólo tenga escritura en su `$HOME`— redirige todos
    /// los comandos a otro binario.
    pub fn argv(&self, binario: &str) -> Result<Vec<String>, ErrorForma> {
        let app_ok = |a: &String| -> Result<String, ErrorForma> {
            if nombre_de_app_valido(a) {
                Ok(a.clone())
            } else {
                Err(ErrorForma::NombreDeApp(a.clone()))
            }
        };

        let mut v: Vec<String> = vec![binario.to_string()];
        // '--json' delante del subcomando, siempre. Es la única posición con un
        // comportamiento definido.
        let json = |v: &mut Vec<String>| v.push("--json".into());

        match self {
            Self::Version => {
                json(&mut v);
                v.push("version".into());
            }
            Self::Lista => {
                json(&mut v);
                v.push("list".into());
            }
            Self::Estado => {
                json(&mut v);
                v.push("status".into());
            }
            Self::Doctor => {
                json(&mut v);
                v.push("doctor".into());
            }
            Self::DoctorArreglar => {
                json(&mut v);
                v.extend(["doctor".into(), "--fix".into(), "--yes".into()]);
            }
            Self::Top => {
                json(&mut v);
                v.push("top".into());
            }
            Self::BaseDeDatosLista => {
                json(&mut v);
                v.extend(["db".into(), "list".into()]);
            }
            Self::VigilanciaEstado => {
                json(&mut v);
                v.extend(["watch".into(), "status".into()]);
            }
            Self::ColaEstado => {
                json(&mut v);
                v.extend(["queue".into(), "status".into()]);
            }
            Self::CopiasLista => {
                json(&mut v);
                v.extend(["backup".into(), "list".into()]);
            }
            Self::CopiasVerificar => {
                json(&mut v);
                v.extend(["backup".into(), "verify".into()]);
            }
            Self::Info { app } => {
                json(&mut v);
                v.push("info".into());
                v.push(app_ok(app)?);
            }
            Self::Metricas { app } => {
                json(&mut v);
                v.push("metrics".into());
                if let Some(a) = app {
                    v.push(app_ok(a)?);
                }
            }
            Self::Trafico { app, desde } => {
                json(&mut v);
                v.push("traffic".into());
                v.push(app_ok(app)?);
                if let Some(d) = desde {
                    v.push("--since".into());
                    v.push(d.clone());
                }
            }
            Self::EntornoLista { app } => {
                json(&mut v);
                v.extend(["env".into(), "list".into()]);
                v.push(app_ok(app)?);
            }
            Self::EntornoValor { app, clave } => {
                // Sin '--json': 'env get' imprime el valor pelado, a propósito.
                if !clave_de_entorno_valida(clave) {
                    return Err(ErrorForma::ClaveDeEntorno(clave.clone()));
                }
                v.extend(["env".into(), "get".into()]);
                v.push(app_ok(app)?);
                v.push(clave.clone());
            }
            Self::RedireccionesLista { app } => {
                json(&mut v);
                v.extend(["redirect".into(), "list".into()]);
                if let Some(a) = app {
                    v.push(app_ok(a)?);
                }
            }
            Self::Logs {
                app,
                desde,
                lineas,
                seguir,
                solo_nginx,
            } => {
                json(&mut v);
                v.push("logs".into());
                v.push(app_ok(app)?);
                if let Some(d) = desde {
                    v.push("--since".into());
                    v.push(d.clone());
                }
                if let Some(n) = lineas {
                    v.push("--lines".into());
                    v.push(n.to_string());
                }
                if *seguir {
                    v.push("--follow".into());
                }
                if *solo_nginx {
                    v.push("--nginx".into());
                }
            }
            Self::Desplegar { app, progreso } => {
                json(&mut v);
                v.push("deploy".into());
                v.push(app_ok(app)?);
                if *progreso {
                    v.push("--progress".into());
                }
            }
            Self::DesplegarTodo { progreso } => {
                json(&mut v);
                v.extend(["deploy".into(), "--all".into()]);
                if *progreso {
                    v.push("--progress".into());
                }
            }
            Self::Ejecutar { app, modo } => {
                // Sin '--json': la salida de `exec` es la del comando, sin
                // envolver y sin formatear, y eso es lo correcto. Orbit lo
                // rechaza explícitamente por delante, y por detrás se lo pasa al
                // comando — que es lo que quiere quien escribe
                // `orbit exec app mi-script --json`.
                v.push("exec".into());
                v.push(app_ok(app)?);
                match modo {
                    ModoDeExec::Argumentos(args) => {
                        if args.is_empty() {
                            // Sin comando, `orbit exec` abre un bash
                            // interactivo, y un cliente sin terminal no puede
                            // con eso. Fingir medio terminal es, en palabras de
                            // la propia documentación de Orbit, la peor
                            // solución de todas.
                            return Err(ErrorForma::ExecSinComando);
                        }
                        v.extend(args.iter().cloned());
                    }
                    ModoDeExec::Shell(texto) => {
                        if texto.trim().is_empty() {
                            return Err(ErrorForma::ExecSinComando);
                        }
                        v.push(texto.clone());
                    }
                }
            }
            Self::Retirar { app } => {
                v.push("remove".into());
                v.push(app_ok(app)?);
                v.push("-y".into());
            }
            Self::RetirarYBorrar { app } => {
                v.push("remove".into());
                v.push(app_ok(app)?);
                // Las dos banderas, y '--purge' aparte de '-y' a propósito: son
                // daños de categoría distinta y Orbit los separó por eso.
                v.push("-y".into());
                v.push("--purge".into());
            }
            Self::Revertir { app, release } => {
                if !release_valida(release) {
                    return Err(ErrorForma::Release(release.clone()));
                }
                v.push("rollback".into());
                v.push(app_ok(app)?);
                v.push(release.clone());
            }
        }
        Ok(v)
    }

    /// La orden ya escapada, lista para el shell remoto.
    pub fn linea(&self, binario: &str) -> Result<String, ErrorForma> {
        let argv = self.argv(binario)?;
        shquote::build(&argv).map_err(ErrorForma::Escapado)
    }

    /// Si la respuesta es un único objeto JSON o un flujo NDJSON. `logs` es la
    /// única excepción, y se declara aquí para que el transporte no tenga que
    /// adivinarlo.
    pub fn es_flujo(&self) -> bool {
        matches!(self, Self::Logs { .. })
    }
}

/// Los patrones que hacen que la pantalla de `exec` pida una confirmación
/// reforzada.
///
/// **Es una lista negra, y por eso su valor es pedagógico y no defensivo.** No
/// impide nada: hay mil formas de escribir un `rm`, y quien quiera saltársela lo
/// hará sin proponérselo. Lo que sí para es el **error de dedos a las tres de la
/// mañana**, que es el caso real y el único contra el que una lista así sirve.
///
/// Se documenta así a propósito para que nadie la confunda con una protección y
/// construya encima suponiendo que lo es.
pub fn parece_peligroso(texto: &str) -> Option<&'static str> {
    let t = texto.to_lowercase();
    const PATRONES: [(&str, &str); 7] = [
        ("rm -rf /", "borra recursivamente desde una ruta absoluta"),
        ("drop database", "borra una base de datos entera"),
        ("truncate", "vacía una tabla"),
        ("mkfs", "formatea un sistema de ficheros"),
        ("dd of=/dev/", "escribe directamente sobre un dispositivo"),
        ("chmod -r 777 /", "abre los permisos de todo"),
        ("> /dev/sd", "escribe sobre un disco"),
    ];
    PATRONES
        .iter()
        .find(|(p, _)| t.contains(p))
        .map(|(_, q)| *q)
}

#[cfg(test)]
mod tests {
    use super::*;
    const B: &str = "/usr/local/bin/orbit";

    #[test]
    fn el_json_va_siempre_delante() {
        let v = Comando::Lista.argv(B).unwrap();
        assert_eq!(v, [B, "--json", "list"]);
        // Detrás, un comando que no lo habla se lo traga en silencio y sale
        // con 0. Delante siempre muere. Sólo una de las dos es un contrato.
        assert_eq!(v[1], "--json");
    }

    #[test]
    fn nunca_se_invoca_por_path() {
        for c in [Comando::Version, Comando::Lista, Comando::Doctor] {
            assert!(c.argv(B).unwrap()[0].starts_with('/'));
        }
    }

    #[test]
    fn un_nombre_de_app_hostil_no_llega_a_construir_la_orden() {
        let malo = "a'; curl x.sh|sh; '".to_string();
        let r = Comando::Info { app: malo.clone() }.argv(B);
        assert_eq!(r, Err(ErrorForma::NombreDeApp(malo)));
    }

    #[test]
    fn un_nombre_con_marcado_tampoco() {
        let r = Comando::Info {
            app: "</script><img src=x>".into(),
        }
        .argv(B);
        assert!(matches!(r, Err(ErrorForma::NombreDeApp(_))));
    }

    #[test]
    fn la_regla_del_nombre_es_la_del_servidor() {
        assert!(nombre_de_app_valido("mi-web"));
        assert!(nombre_de_app_valido("viejo.com")); // las redirecciones de dominio
        assert!(nombre_de_app_valido("a"));
        assert!(nombre_de_app_valido(&"a".repeat(40)));
        assert!(!nombre_de_app_valido(&"a".repeat(41)));
        assert!(!nombre_de_app_valido("Web")); // sin mayúsculas
        assert!(!nombre_de_app_valido("-web")); // no empieza por guion
        assert!(!nombre_de_app_valido("a..b")); // travesía de rutas
        assert!(!nombre_de_app_valido(""));
        assert!(!nombre_de_app_valido("оrbit")); // o cirílica
    }

    #[test]
    fn una_release_inventada_se_rechaza() {
        assert!(release_valida("20260805-041230"));
        assert!(release_valida("20260805-041230-2"));
        assert!(!release_valida("../../etc/passwd"));
        assert!(!release_valida("current"));
        let r = Comando::Revertir {
            app: "web".into(),
            release: "current".into(),
        }
        .argv(B);
        assert!(matches!(r, Err(ErrorForma::Release(_))));
    }

    #[test]
    fn una_clave_de_entorno_inventada_se_rechaza() {
        assert!(clave_de_entorno_valida("DB_PASSWORD"));
        assert!(!clave_de_entorno_valida("DB-PASSWORD"));
        assert!(!clave_de_entorno_valida("1DB"));
        let r = Comando::EntornoValor {
            app: "web".into(),
            clave: "a b".into(),
        }
        .argv(B);
        assert!(matches!(r, Err(ErrorForma::ClaveDeEntorno(_))));
    }

    #[test]
    fn env_get_no_lleva_json_porque_imprime_el_valor_pelado() {
        let v = Comando::EntornoValor {
            app: "web".into(),
            clave: "DB_PASSWORD".into(),
        }
        .argv(B)
        .unwrap();
        assert!(!v.iter().any(|x| x == "--json"));
    }

    #[test]
    fn arreglar_lleva_el_yes_que_el_servidor_exige() {
        // Sin `--yes`, `doctor --fix --json` se niega: sin terminal no hay a
        // quién preguntar. Y ese camino estuvo documentado y MUERTO durante
        // versiones, porque `--yes` no era una bandera global — así que contra
        // un servidor viejo esto falla, y la interfaz vuelve a enseñar la orden
        // para copiar.
        let v = Comando::DoctorArreglar.argv(B).unwrap();
        assert_eq!(v, [B, "--json", "doctor", "--fix", "--yes"]);
    }

    #[test]
    fn los_dos_modos_de_exec_producen_argv_distintos() {
        // `exec web "ls -la"` y `exec web ls -la` NO son lo mismo: el primero
        // pasa por un shell y el segundo no. La diferencia se elige, no se
        // adivina — una herramienta de depuración que no es predecible tampoco
        // sirve.
        let uno = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Shell("ls -la".into()),
        }
        .argv(B)
        .unwrap();
        let otro = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Argumentos(vec!["ls".into(), "-la".into()]),
        }
        .argv(B)
        .unwrap();
        assert_eq!(uno, [B, "exec", "web", "ls -la"]);
        assert_eq!(otro, [B, "exec", "web", "ls", "-la"]);
        assert_ne!(uno, otro);
    }

    #[test]
    fn exec_no_lleva_json() {
        // La salida de `exec` es la del comando, sin envolver. Orbit rechaza
        // `--json` por delante, y por detrás se lo pasa al comando — que es lo
        // que quiere quien escribe `orbit exec app mi-script --json`.
        let v = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Shell("ls".into()),
        }
        .argv(B)
        .unwrap();
        assert!(!v.iter().any(|x| x == "--json"));
    }

    #[test]
    fn exec_sin_comando_se_rechaza_aqui() {
        // Sin comando, `orbit exec` abre un bash interactivo, y un cliente sin
        // terminal no puede con eso. Fingir medio terminal es peor que no
        // ofrecerlo.
        for modo in [
            ModoDeExec::Argumentos(vec![]),
            ModoDeExec::Shell("   ".into()),
        ] {
            let r = Comando::Ejecutar {
                app: "web".into(),
                modo,
            }
            .argv(B);
            assert_eq!(r, Err(ErrorForma::ExecSinComando));
        }
    }

    #[test]
    fn el_texto_de_exec_sobrevive_al_escapado() {
        // Es texto arbitrario por diseño, y tiene que llegar entero: la prueba
        // de propiedad lo comprueba contra cuatro shells, aquí sólo se fija que
        // viaja como UN argumento.
        let sucio = "psql \"select * from t where x = 'y'\" && echo $HOME";
        let v = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Shell(sucio.into()),
        }
        .argv(B)
        .unwrap();
        assert_eq!(v.len(), 4);
        assert_eq!(v[3], sucio);
    }

    #[test]
    fn la_lista_de_patrones_es_pedagogica_y_se_sabe() {
        // No impide nada —hay mil formas de escribir un rm— y por eso su valor
        // es parar el error de dedos a las tres de la mañana, no defender.
        assert!(parece_peligroso("rm -rf /srv/apps").is_some());
        assert!(parece_peligroso("DROP DATABASE tienda").is_some());
        assert!(parece_peligroso("php artisan migrate").is_none());
        // Y se salta sin proponérselo, que es justo lo que hay que documentar.
        assert!(parece_peligroso("cd / && rm -rf apps").is_none());
    }

    #[test]
    fn retirar_y_borrar_son_dos_ordenes_distintas() {
        // No son la misma con una bandera: son daños de categoría distinta, y
        // Orbit los separó por eso. En la interfaz son dos entradas, no una
        // casilla — una casilla junto a un botón se marca sin leerla.
        let a = Comando::Retirar {
            app: "tienda".into(),
        }
        .argv(B)
        .unwrap();
        let b = Comando::RetirarYBorrar {
            app: "tienda".into(),
        }
        .argv(B)
        .unwrap();
        assert_eq!(a, [B, "remove", "tienda", "-y"]);
        assert_eq!(b, [B, "remove", "tienda", "-y", "--purge"]);
        assert!(!a.contains(&"--purge".to_string()));
    }

    #[test]
    fn retirar_sin_purge_no_puede_borrar_datos() {
        // '-y' significa «acepta el valor por defecto», y el valor por defecto
        // del borrado de datos es «no». Si esta prueba se rompiera, una
        // operación anunciada como reversible habría dejado de serlo.
        let v = Comando::Retirar {
            app: "tienda".into(),
        }
        .argv(B)
        .unwrap();
        assert!(!v.iter().any(|x| x == "--purge"));
    }

    #[test]
    fn logs_es_el_unico_flujo() {
        assert!(Comando::Logs {
            app: "web".into(),
            desde: None,
            lineas: None,
            seguir: false,
            solo_nginx: false
        }
        .es_flujo());
        assert!(!Comando::Lista.es_flujo());
        assert!(!Comando::Desplegar {
            app: "web".into(),
            progreso: true
        }
        .es_flujo());
    }

    #[test]
    fn la_linea_queda_escapada() {
        let l = Comando::Trafico {
            app: "mi-web".into(),
            desde: Some("7d".into()),
        }
        .linea(B)
        .unwrap();
        assert_eq!(l, "/usr/local/bin/orbit --json traffic mi-web --since 7d");
    }

    #[test]
    fn el_catalogo_no_puede_expresar_una_orden_sin_app() {
        // No hay ninguna variante con la app opcional donde el servidor elegiría
        // la primera por orden alfabético. Si alguien añade una, este test no la
        // ve — pero la revisión de este fichero sí, y ése es el sitio.
        let c = Comando::Info { app: String::new() };
        assert!(c.argv(B).is_err());
    }
}
