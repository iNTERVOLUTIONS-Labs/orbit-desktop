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
    /// Un valor para `deploy --env` que el servidor rechazaría. **Lleva la
    /// clave y el motivo, nunca el valor**: este error acaba en pantalla y en
    /// los informes de fallo, y el valor suele ser una contraseña.
    ValorDeEntorno {
        clave: String,
        motivo: &'static str,
    },
    /// La misma variable dos veces en un despliegue. El servidor se quedaría
    /// con una de las dos sin decir cuál, y quien lo pidió no sabría cuál.
    ClaveRepetida(String),
    /// El motivo de una congelación que no cabe en una línea del testigo.
    Motivo(&'static str),
    /// El nombre de la app es también una palabra de Orbit **en esa orden**, y
    /// el servidor la leería como palabra y no como nombre: `checks on no`
    /// cambia la primera app de la lista, `lock help` contesta la ayuda con 0.
    /// `orden` es el subcomando tal cual, para el mensaje.
    NombreEsPalabraDeOrbit {
        app: String,
        orden: &'static str,
    },
    /// Un ajuste de detección (`--build`, `--start`…) con un salto de línea o un
    /// carácter de control. Lleva la bandera, no el valor.
    Ajuste(&'static str),
    /// Un único argumento con caracteres de shell **y** un `=`: suelto, Orbit
    /// lo pasaría a `bash -lc`; protegido con `env`, `env` lo leería como una
    /// variable y volcaría el entorno de la app. No hay forma literal de
    /// mandarlo, así que se dice.
    ExecAmbiguo(String),
    /// Un valor que el servidor leería como **su** bandera de idioma:
    /// `--lang`, `--idioma`, `--lang=xx` o `--idioma=xx`. `main()` de Orbit se
    /// los quita a todas las órdenes salvo `exec`, estén donde estén (orbit
    /// ~19410, `_lang_strip`), así que como motivo de `lock` o valor de
    /// `--since` o `--build` desaparecen —y se llevan el siguiente argumento, o
    /// cambian el idioma de la respuesta—. Lleva el valor: no es secreto.
    ValorEsIdioma(String),
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
            Self::ValorDeEntorno { clave, motivo } => {
                write!(f, "el valor de {clave} {motivo}")
            }
            Self::ClaveRepetida(c) => write!(f, "{c} va dos veces en el mismo despliegue"),
            Self::Motivo(m) => write!(f, "el motivo {m}"),
            Self::NombreEsPalabraDeOrbit { app, orden } => {
                let efecto = match *orden {
                    "autodeploy checks" => {
                        "la leería como el on/off de la orden y no como el nombre, y el cambio \
                         iría a parar a la primera app de la lista"
                    }
                    "metrics" => {
                        "contestaría con las métricas de Prometheus y no con las de la app"
                    }
                    _ => "contestaría con su ayuda y no haría nada, aunque parecería que sí",
                };
                let como = if *orden == "metrics" {
                    "«orbit metrics» sin nombre, que la incluye junto a las demás".to_string()
                } else {
                    format!("«orbit {orden}» sin nombre, en un terminal, y elígela de la lista")
                };
                write!(
                    f,
                    "«{app}» es también una palabra de «orbit {orden}»: el servidor {efecto}. \
                     Hazlo en el servidor con {como}"
                )
            }
            Self::Ajuste(b) => write!(
                f,
                "el valor de {b} tiene que caber en una línea: lleva un salto de línea o un \
                 carácter de control, y el servidor lo guarda en ficheros que se leen por líneas"
            ),
            Self::ExecAmbiguo(t) => write!(
                f,
                "«{t}» lleva caracteres de shell y un «=»: suelto, Orbit lo pasaría a un shell, \
                 y con «env» delante se leería como una variable. Si es una orden de shell, usa \
                 el modo shell; si no, sepárala en argumentos"
            ),
            Self::ValorEsIdioma(v) => write!(
                f,
                "«{v}» no puede ir como valor: el servidor lo leería como su opción de idioma y                  se lo quitaría a la orden. Escríbelo de otra forma (sin los dos guiones, por                  ejemplo)"
            ),
            Self::Escapado(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ErrorForma {}

/// Lo que `_lang_strip` del servidor se lleva de cualquier posición.
fn lo_leeria_como_idioma(a: &str) -> bool {
    a == "--lang" || a == "--idioma" || a.starts_with("--lang=") || a.starts_with("--idioma=")
}

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
    // `is_control` y no `is_ascii_control`: un C1 (U+0080–U+009F) o un U+2028
    // también parten una línea para alguien, y la rama acaba en el `.conf` de
    // la app y en la prosa de cada despliegue.
    !s.chars().any(|c| {
        c.is_control() || c.is_whitespace() || matches!(c, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
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
    // Ni controles de fuera de ASCII (C1) ni espacios Unicode: una URL de
    // verdad no los lleva, y el repo acaba en el `.conf` de la app.
    if s.chars().any(|c| c.is_control() || c.is_whitespace()) {
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

/// El correo del certificado. Deliberadamente laxo con la forma: quien valida
/// de verdad es Let's Encrypt, y una expresión regular estricta aquí sólo
/// consigue rechazar direcciones que existen.
///
/// **Laxo no es «cualquier cosa»**: ni espacios ni caracteres de control (C0,
/// DEL, C1). `orbit new --email` lo guarda con `_conf_set` en
/// `/etc/orbit/orbit.conf`, que reescribe el fichero **línea a línea** y que
/// root carga con `source`: con un salto de línea dentro, el siguiente cambio
/// de esa clave dejaba suelta la línea de después, y un `$(…)` en ella se
/// ejecutaba como root.
pub fn correo_valido(s: &str) -> bool {
    if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    match s.split_once('@') {
        Some((u, d)) => !u.is_empty() && !s.starts_with('-') && dominio_valido(d),
        None => false,
    }
}

/// Un valor que **no se enseña**: el de una variable de entorno que viaja con
/// `deploy --env`.
///
/// Es un tipo y no una `String` para que olvidarse sea difícil: su `Debug`
/// dice «oculto», no implementa `Display`, y la única forma de leerlo es
/// [`Secreto::exponer`], que se ve en una revisión. Un `{:?}` de un
/// [`Comando`] en un mensaje de error o en un registro —la forma más corriente
/// de filtrar una contraseña— no lo saca.
#[derive(Clone, PartialEq, Eq)]
pub struct Secreto(String);

impl Secreto {
    pub fn nuevo(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// El valor de verdad. Sólo lo llama quien construye el `argv`.
    pub fn exponer(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secreto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secreto(«oculto»)")
    }
}

/// Lo que se enseña en lugar de un valor. Siempre igual, y no del largo del
/// valor: la longitud de una contraseña también es un dato sobre ella.
pub const OCULTO: &str = "••••••";

/// Una variable que se pone en el `.env` **antes del build** con
/// `orbit deploy --env CLAVE=valor`. 1.3.8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableNueva {
    pub clave: String,
    pub valor: Secreto,
}

/// Las mismas reglas que `_env_value_problem` en el servidor, para que un
/// valor que no se puede guardar se diga aquí y no a mitad de un despliegue:
/// ni saltos de línea —un `.env` es una variable por línea— ni comilla simple,
/// que bash y systemd escapan distinto. Y ni un byte nulo, que no cruza un
/// `argv`.
fn problema_del_valor(v: &str) -> Option<&'static str> {
    if v.contains('\n') || v.contains('\r') {
        return Some("lleva saltos de línea, y en un .env cada variable es una línea");
    }
    if v.contains('\'') {
        return Some("lleva una comilla simple, y systemd y bash no la escapan igual");
    }
    if v.contains('\0') {
        return Some("lleva un byte nulo");
    }
    None
}

/// El motivo de `orbit lock -m`. El servidor ya cambia los saltos de línea por
/// espacios y lo corta en 200: aquí se rechaza en vez de recortarse, porque un
/// motivo recortado en silencio dice otra cosa que el que se escribió.
pub fn motivo_valido(m: &str) -> Result<(), ErrorForma> {
    if m.chars().count() > 200 {
        return Err(ErrorForma::Motivo("pasa de 200 caracteres"));
    }
    // `is_control` ya cubre C0, DEL y C1; U+2028 y U+2029 no son control pero
    // sí parten la línea en un visor, y el motivo se enseña en una.
    if m.chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(ErrorForma::Motivo("tiene que caber en una línea"));
    }
    Ok(())
}

/// Las palabras que `orbit autodeploy checks` lee como el interruptor **en
/// cualquier posición** (`orbit` ~14727): con una app que se llame así no queda
/// nombre, y sin terminal `pick_app` elige la primera por orden alfabético.
pub const PALABRAS_DE_CHECKS: &[&str] =
    &["on", "yes", "si", "sí", "enable", "off", "no", "disable"];

/// Lo que `lock`, `unlock` y `remove` leen como petición de ayuda en el sitio
/// del nombre: contestan el uso y salen con 0, y la interfaz lo daría por hecho.
pub const PALABRAS_DE_AYUDA: &[&str] = &["help", "ayuda"];

/// `metrics prom` es un subcomando y gana a la app (`orbit` ~8794).
pub const PALABRAS_DE_METRICS: &[&str] = &["prom"];

/// Las palabras de Orbit que **esa orden** leería en el sitio del nombre de la
/// app. Vacío para las órdenes que no tienen ninguna. Es lo que replica la
/// interfaz (`PALABRAS_DE_ORBIT` en `ui/src/lib/despliegue.ts`).
///
/// Se descartó prohibir esos nombres en general: `no` o `prom` son apps
/// válidas para el servidor, y en el resto del catálogo funcionan bien.
pub fn palabras_de_orbit(c: &Comando) -> &'static [&'static str] {
    match c {
        Comando::EsperarCi { .. } => PALABRAS_DE_CHECKS,
        Comando::Congelar { .. }
        | Comando::Descongelar { .. }
        | Comando::Retirar { .. }
        | Comando::RetirarYBorrar { .. } => PALABRAS_DE_AYUDA,
        Comando::Metricas { app: Some(_) } => PALABRAS_DE_METRICS,
        _ => &[],
    }
}

/// Si el nombre de la app de `c` choca con una palabra de Orbit en esa orden.
pub fn choca_con_palabra_de_orbit(c: &Comando) -> bool {
    choque(c).is_some()
}

/// El nombre que choca y el subcomando con el que choca, para el mensaje.
fn choque(c: &Comando) -> Option<(&str, &'static str)> {
    let (app, orden) = match c {
        Comando::EsperarCi { app, .. } => (app, "autodeploy checks"),
        Comando::Congelar { app, .. } => (app, "lock"),
        Comando::Descongelar { app } => (app, "unlock"),
        Comando::Retirar { app } | Comando::RetirarYBorrar { app } => (app, "remove"),
        Comando::Metricas { app: Some(app) } => (app, "metrics"),
        _ => return None,
    };
    palabras_de_orbit(c)
        .contains(&app.as_str())
        .then_some((app.as_str(), orden))
}

/// Un ajuste de detección que el servidor pueda guardar: una línea, sin
/// controles. `--start` acaba en la unidad de systemd y `--build` en el `.conf`
/// de la app; un salto de línea ahí es una directiva o una variable de más.
fn ajuste_valido(v: &str, bandera: &'static str) -> Result<(), ErrorForma> {
    if v.chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(ErrorForma::Ajuste(bandera));
    }
    Ok(())
}

/// Los caracteres por los que `cmd_exec` (`orbit` ~13355) manda un argumento
/// único a `bash -lc`: `[[:space:]|&;<>()$\`]`. Aquí con `is_whitespace`, que
/// es un superconjunto de `[[:space:]]` en cualquier locale: pasarse sólo
/// cuesta un `env` de más.
fn lo_pasaria_a_un_shell(s: &str) -> bool {
    s.chars()
        .any(|c| c.is_whitespace() || c.is_control() || "|&;<>()$`".contains(c))
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
    ///
    /// Con **un solo** argumento que lleve metacaracteres, el `argv` sale con
    /// `/usr/bin/env --` delante: si no, la regla del argumento único del
    /// servidor lo mandaría a `bash -lc` (ver [`Comando::Ejecutar`]).
    Argumentos(Vec<String>),
    /// El texto entero, para un shell: sale como `/bin/bash -lc <texto>`, así
    /// que `&&`, `|`, `$(…)`, `~` y los comodines **sí** hacen lo que parece.
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
        /// Lo que se pone en el `.env` antes del build (`--env`). Es la forma de
        /// dar una variable **obligatoria** sin terminal: sin ella el despliegue
        /// se para antes del build, porque con `--json` no hay a quién
        /// preguntar. Vacío casi siempre.
        ///
        /// **Viaja en el `argv`**, y eso tiene un coste que no se puede tapar
        /// desde aquí: mientras dura el despliegue, el valor se ve en `ps` del
        /// servidor, y si el usuario SSH no es root, `sudo` lo apunta en su
        /// registro. Orbit no ofrece hoy otra vía —`--env-file` exige un
        /// fichero en el servidor, y escribir uno es justo lo que este cliente
        /// no hace—. Está en THREAT-MODEL, y la pantalla lo dice.
        entorno: Vec<VariableNueva>,
        /// `--force`: desplegar aunque esté congelada. **No la descongela**: se
        /// despliega esa vez y la congelación sigue. Es una decisión, y por eso
        /// en la interfaz es un botón aparte, nunca una casilla.
        forzar: bool,
        /// Mandar `entorno` por la **entrada estándar** (`--env-stdin`, Orbit
        /// 1.4.9) en vez de por el `argv`. Con esto el valor viaja por el canal
        /// cifrado de SSH y no toca ni `ps` ni el registro de `sudo`: es lo que
        /// cierra THREAT-MODEL T-13. Lo decide quien sabe la versión del
        /// servidor; con uno anterior, `--env-stdin` no existe y el servidor
        /// se negaría entero —que es un fallo seguro, no una fuga—.
        por_entrada: bool,
    },
    /// Una pasada por todas las apps del servidor.
    ///
    /// **`solo_si_cambia` no es una opción: es la mitad del significado de la
    /// orden**, y las dos mitades son operaciones distintas.
    ///
    /// * Con él (`--if-changed`), Orbit le pregunta primero al remoto de cada
    ///   app si hay algo nuevo y sólo despliega las que se han movido. Es lo
    ///   que se hace a diario, y lo barato: preguntar cuesta un `git ls-remote`
    ///   por app.
    /// * Sin él, **se despliega todo, haya cambiado o no**. Con cuarenta apps
    ///   eso son cuarenta builds completos y cuarenta releases nuevas de un
    ///   código idéntico. Existe porque hay un caso real en el que hace falta
    ///   —recompilar todo tras cambiar la versión de Node o un `.env`
    ///   compartido— y ese caso no se puede expresar de otra forma.
    ///
    /// Y tiene una consecuencia que la interfaz **tiene que enseñar**: cuatro de
    /// los seis finales sólo existen en la primera rama. `unchanged`,
    /// `unreachable` y `gone` salen de preguntarle al remoto, y sin
    /// `--if-changed` no se le pregunta a nadie. Una pasada completa sólo puede
    /// terminar en `deployed` o en `failed`.
    DesplegarTodo {
        progreso: bool,
        solo_si_cambia: bool,
    },
    /// Lo que la app declara frente a lo que tiene, **sin un solo valor**.
    /// 1.3.8. Es la comprobación previa al despliegue: si falta una
    /// obligatoria, la interfaz la pide antes de lanzar nada.
    EntornoComprobar {
        app: String,
    },
    /// Las tareas programadas que declara el `orbit.json`. 1.3.9.
    Tareas {
        app: String,
    },
    /// Congela los despliegues de una app. 1.4.3.
    ///
    /// El autodespliegue y `deploy --all` la saltan, y `deploy` a mano se niega
    /// sin `--force`. **El rollback sigue permitido**: es la salida de
    /// emergencia. Sin `--json`, porque `lock` no lo habla: con él delante,
    /// el servidor se niega entero.
    Congelar {
        app: String,
        motivo: Option<String>,
    },
    Descongelar {
        app: String,
    },
    /// Que el autodespliegue y `deploy --all` esperen a la CI de GitHub.
    /// 1.4.3. `orbit deploy` a mano no espera nunca: sólo avisa.
    EsperarCi {
        app: String,
        esperar: bool,
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
    /// remoto, el cliente **nunca construye una cadena de shell**. El texto del
    /// modo shell viaja como un argumento más detrás de `/bin/bash -lc`, y los
    /// dos caminos van por el mismo escapador.
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

        // Antes de construir nada: un nombre que el servidor leería como una
        // palabra suya retarga la orden o la convierte en «ayuda, rc 0».
        if let Some((app, orden)) = choque(self) {
            return Err(ErrorForma::NombreEsPalabraDeOrbit {
                app: app.to_string(),
                orden,
            });
        }

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
            Self::Desplegar {
                app,
                progreso,
                entorno,
                forzar,
                por_entrada,
            } => {
                json(&mut v);
                v.push("deploy".into());
                v.push(app_ok(app)?);
                if *progreso {
                    v.push("--progress".into());
                }
                if *forzar {
                    v.push("--force".into());
                }
                let mut vistas: Vec<&str> = Vec::new();
                for e in entorno {
                    if !clave_de_entorno_valida(&e.clave) {
                        return Err(ErrorForma::ClaveDeEntorno(e.clave.clone()));
                    }
                    if vistas.contains(&e.clave.as_str()) {
                        return Err(ErrorForma::ClaveRepetida(e.clave.clone()));
                    }
                    vistas.push(&e.clave);
                    if let Some(motivo) = problema_del_valor(e.valor.exponer()) {
                        return Err(ErrorForma::ValorDeEntorno {
                            clave: e.clave.clone(),
                            motivo,
                        });
                    }
                    if !*por_entrada {
                        // Un solo argumento `CLAVE=valor`, como lo parte el
                        // servidor (`${kv%%=*}`): un `=` dentro del valor llega
                        // entero, porque sólo se corta por el primero.
                        v.push("--env".into());
                        v.push(format!("{}={}", e.clave, e.valor.exponer()));
                    }
                }
                // Por la entrada: en el `argv` sólo va la bandera, y los
                // valores los escribe el transporte por stdin (ver
                // [`Comando::entrada`]). Sin variables no se pasa: `--env-stdin`
                // gasta la entrada y apaga las preguntas, y no hay nada que leer.
                if *por_entrada && !entorno.is_empty() {
                    v.push("--env-stdin".into());
                }
            }
            Self::EntornoComprobar { app } => {
                json(&mut v);
                v.extend(["env".into(), "check".into()]);
                v.push(app_ok(app)?);
            }
            Self::Tareas { app } => {
                json(&mut v);
                v.extend(["cron".into(), "list".into()]);
                v.push(app_ok(app)?);
            }
            Self::Congelar { app, motivo } => {
                v.push("lock".into());
                v.push(app_ok(app)?);
                if let Some(m) = motivo {
                    motivo_valido(m)?;
                    // Con su bandera y no suelto detrás del nombre, aunque el
                    // servidor acepte las dos: así un motivo que empiece por
                    // guion es el valor de `-m` y no una opción que no existe.
                    if !m.trim().is_empty() {
                        v.push("-m".into());
                        v.push(m.clone());
                    }
                }
            }
            Self::Descongelar { app } => {
                v.push("unlock".into());
                v.push(app_ok(app)?);
            }
            Self::EsperarCi { app, esperar } => {
                v.extend(["autodeploy".into(), "checks".into()]);
                v.push(if *esperar { "on" } else { "off" }.into());
                v.push(app_ok(app)?);
            }
            Self::DesplegarTodo {
                progreso,
                solo_si_cambia,
            } => {
                json(&mut v);
                v.extend(["deploy".into(), "--all".into()]);
                if *solo_si_cambia {
                    v.push("--if-changed".into());
                }
                if *progreso {
                    v.push("--progress".into());
                }
                // Nunca '--auto'. Esa bandera es la del autodespliegue: filtra a
                // las apps que lo tienen activado y además marca la vigilancia
                // como si el disparo hubiera venido del temporizador. Un cliente
                // que la pasara estaría falsificando el origen de la pasada en
                // el historial del servidor.
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
                        // La regla del argumento único del servidor: uno solo
                        // con metacaracteres va a `bash -lc`, y entonces este
                        // modo dejaría de ser «sin shell». Con `env --` delante
                        // llegan dos o más y se ejecuta tal cual: `env -- 'ls;id'`
                        // es «no existe», nunca un shell. El `--` es por un
                        // primer argumento con guion, que `env` leería como
                        // opción. Lo que `env` no deja pasar es un `=`: lo toma
                        // por una variable y, sin orden detrás, vuelca el
                        // entorno —con los secretos de la app—, así que eso se
                        // rechaza en vez de mandarse.
                        //
                        // Por su ruta, `/usr/bin/env`: el servidor lo ejecuta con
                        // el PATH de la app, que pone delante `node_modules/.bin`
                        // y `.venv/bin`, y un `env` de una dependencia sería el
                        // que recibiera la orden.
                        if let [unico] = args.as_slice() {
                            if lo_pasaria_a_un_shell(unico) {
                                if unico.contains('=') {
                                    return Err(ErrorForma::ExecAmbiguo(unico.clone()));
                                }
                                v.push("/usr/bin/env".into());
                                v.push("--".into());
                            }
                        }
                        v.extend(args.iter().cloned());
                    }
                    ModoDeExec::Shell(texto) => {
                        if texto.trim().is_empty() {
                            return Err(ErrorForma::ExecSinComando);
                        }
                        // El shell lo ponemos nosotros, y no la regla del
                        // argumento único del servidor: ésa sólo va a `bash -lc`
                        // si el texto lleva espacio o un metacarácter, y
                        // `~/bin/backup.sh` o `*.sh` se ejecutaban tal cual, sin
                        // expandir nada. Con tres argumentos el servidor ejecuta
                        // el `argv` tal cual, y es exactamente lo que habría
                        // hecho él (`argv=(bash -lc "$1")`, mismo usuario y
                        // mismo entorno de `_run_in_dir`), con `bash` por su ruta
                        // por lo mismo que `env` arriba.
                        v.extend(["/bin/bash".into(), "-lc".into(), texto.clone()]);
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
                for (valor, bandera) in [
                    (ajustes.carpeta.as_deref(), "--appdir"),
                    (ajustes.tipo.as_deref(), "--type"),
                ] {
                    if let Some(x) = valor {
                        ajuste_valido(x, bandera)?;
                    }
                }
                for (anulacion, bandera) in [
                    (&ajustes.build, "--build"),
                    (&ajustes.arranque, "--start"),
                    (&ajustes.outdir, "--outdir"),
                ] {
                    if let Some(Anulacion::Valor(x)) = anulacion {
                        ajuste_valido(x, bandera)?;
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
        // Al final y para todas, en vez de en cada campo: el día que se añada
        // uno nuevo con texto libre ya está cubierto. `exec` no, porque ahí el
        // servidor no los quita: son la orden de otro. Y ninguna orden de aquí
        // lleva un `--`, que es lo único tras lo cual el servidor deja de
        // buscarlos.
        if !matches!(self, Self::Ejecutar { .. }) {
            if let Some(a) = v[1..].iter().find(|a| lo_leeria_como_idioma(a)) {
                return Err(ErrorForma::ValorEsIdioma(a.clone()));
            }
        }
        Ok(v)
    }

    /// La orden ya escapada, lista para el shell remoto.
    pub fn linea(&self, binario: &str) -> Result<String, ErrorForma> {
        let argv = self.argv(binario)?;
        shquote::build(&argv).map_err(ErrorForma::Escapado)
    }

    /// Lo que hay que escribir por la **entrada estándar** de la orden, si algo.
    ///
    /// Sólo un despliegue con `por_entrada` y variables lo usa: una línea
    /// `CLAVE="valor"` por variable, que es lo que lee `--env-stdin` —el mismo
    /// formato que `--env-file`—. Las comillas dobles no son estilo: Orbit pasa
    /// cada valor por `_env_unquote`, que **sin comillas** corta un ` #` como
    /// comentario y quita los espacios del final, así que `p@ss #1` llegaría
    /// como `p@ss`. Entre comillas dobles sólo se interpretan `\"` y `\\`, y
    /// por eso son los dos únicos que se escapan. Una comilla simple, un salto
    /// de línea o un nulo ya los rechaza [`Comando::argv`] antes de llegar aquí.
    ///
    /// Es un [`Secreto`]: lleva los valores.
    pub fn entrada(&self) -> Option<Secreto> {
        match self {
            Self::Desplegar {
                entorno,
                por_entrada: true,
                ..
            } if !entorno.is_empty() => {
                let mut s = String::new();
                for e in entorno {
                    let v = e.valor.exponer().replace('\\', "\\\\").replace('"', "\\\"");
                    s.push_str(&e.clave);
                    s.push_str("=\"");
                    s.push_str(&v);
                    s.push_str("\"\n");
                }
                Some(Secreto::nuevo(s))
            }
            _ => None,
        }
    }

    /// El `argv` **para enseñarlo**: el mismo que [`Comando::argv`], con cada
    /// valor de `--env` cambiado por [`OCULTO`].
    ///
    /// Sale del `argv` de verdad y no de una plantilla aparte, a propósito: la
    /// hoja de comando existe para que lo que se lee sea lo que se ejecuta, y
    /// dos funciones que construyen la misma orden acaban construyendo dos.
    /// Sólo se tapa lo que va detrás de un `--env`, y sólo después del `=`: la
    /// clave se ve, porque decidir si se manda hace falta leerla.
    pub fn argv_visible(&self, binario: &str) -> Result<Vec<String>, ErrorForma> {
        let mut v = self.argv(binario)?;
        if matches!(self, Self::Desplegar { .. }) {
            let mut tras_env = false;
            for a in v.iter_mut() {
                if tras_env {
                    if let Some((k, _)) = a.split_once('=') {
                        *a = format!("{k}={OCULTO}");
                    }
                    tras_env = false;
                } else {
                    tras_env = a == "--env";
                }
            }
        }
        Ok(v)
    }

    /// La línea que se enseña en la hoja de comando: escapada como la de
    /// verdad, y con los valores secretos tapados.
    pub fn linea_visible(&self, binario: &str) -> Result<String, ErrorForma> {
        shquote::build(&self.argv_visible(binario)?).map_err(ErrorForma::Escapado)
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
        assert_eq!(uno, [B, "exec", "web", "/bin/bash", "-lc", "ls -la"]);
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
        assert_eq!(v.len(), 6);
        assert_eq!(v[5], sucio);
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
            progreso: true,
            entorno: vec![],
            forzar: false,
            por_entrada: false,
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

    // ── La pasada ───────────────────────────────────────────────────────────

    /// Las dos mitades de `deploy --all` son dos órdenes distintas, y por eso en
    /// la interfaz son dos entradas y no una casilla: `--if-changed` pregunta
    /// antes y despliega lo que se ha movido; sin él se recompila todo.
    #[test]
    fn preguntar_antes_y_recompilarlo_todo_son_dos_ordenes() {
        let barata = Comando::DesplegarTodo {
            progreso: true,
            solo_si_cambia: true,
        }
        .argv("orbit")
        .unwrap();
        let cara = Comando::DesplegarTodo {
            progreso: true,
            solo_si_cambia: false,
        }
        .argv("orbit")
        .unwrap();

        assert!(barata.contains(&"--if-changed".to_string()));
        assert!(!cara.contains(&"--if-changed".to_string()));
        assert_ne!(barata, cara);
    }

    /// `--auto` no sale nunca de aquí. No es una bandera de conveniencia: filtra
    /// a las apps con autodespliegue activado y marca la vigilancia como si la
    /// pasada la hubiera lanzado el temporizador. Pasarla sería falsificar el
    /// origen de la pasada en el historial del servidor.
    #[test]
    fn el_cliente_no_finge_ser_el_autodespliegue() {
        for progreso in [true, false] {
            for solo_si_cambia in [true, false] {
                let v = Comando::DesplegarTodo {
                    progreso,
                    solo_si_cambia,
                }
                .argv("orbit")
                .unwrap();
                assert!(!v.contains(&"--auto".to_string()));
                assert!(!v.contains(&"--quiet".to_string()));
            }
        }
    }

    /// `--progress` sólo tiene sentido con `--json`, y el servidor lo rechaza si
    /// no lo lleva. Aquí van siempre juntos porque el catálogo pone `--json` en
    /// todo lo que lo tiene, pero conviene fijarlo: es una regla del otro lado.
    #[test]
    fn el_progreso_de_la_pasada_viaja_con_json() {
        let v = Comando::DesplegarTodo {
            progreso: true,
            solo_si_cambia: true,
        }
        .argv("orbit")
        .unwrap();
        let i = v.iter().position(|x| x == "--progress").unwrap();
        let j = v.iter().position(|x| x == "--json").unwrap();
        assert!(j < i, "'--json' va delante del subcomando");
    }

    // ── 1.3.8 a 1.4.8 ───────────────────────────────────────────────────────

    fn por_entrada(entorno: Vec<(&str, &str)>) -> Comando {
        match desplegar_con(entorno, false) {
            Comando::Desplegar {
                app,
                progreso,
                entorno,
                forzar,
                ..
            } => Comando::Desplegar {
                app,
                progreso,
                entorno,
                forzar,
                por_entrada: true,
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn por_la_entrada_el_argv_no_lleva_ni_un_valor_ni_una_clave() {
        let c = por_entrada(vec![("DB_PASSWORD", "hunter2"), ("MODE", "a=b")]);
        let v = c.argv(B).unwrap();
        assert_eq!(
            v,
            [B, "--json", "deploy", "tienda", "--progress", "--env-stdin"]
        );
        assert!(!v.join(" ").contains("hunter2"));
        assert!(!v.iter().any(|a| a == "--env"));
        // Lo que se enseña es lo mismo: no hay nada que tapar.
        assert_eq!(c.argv_visible(B).unwrap(), v);
    }

    #[test]
    fn la_entrada_va_entre_comillas_dobles_para_que_orbit_no_la_recorte() {
        let c = por_entrada(vec![
            ("A", r#"p@ss w"ord #no-es-comentario  "#),
            ("B", r"back\slash\"),
            ("C", ""),
        ]);
        let e = c.entrada().expect("hay entrada");
        assert_eq!(
            e.exponer(),
            "A=\"p@ss w\\\"ord #no-es-comentario  \"\nB=\"back\\\\slash\\\\\"\nC=\"\"\n"
        );
        // Y no se deja imprimir.
        assert!(!format!("{e:?}").contains("p@ss"));
    }

    #[test]
    fn sin_variables_o_por_el_argv_no_hay_entrada() {
        assert!(por_entrada(vec![]).entrada().is_none());
        assert!(!por_entrada(vec![])
            .argv(B)
            .unwrap()
            .contains(&"--env-stdin".to_string()));
        assert!(desplegar_con(vec![("A", "1")], false).entrada().is_none());
        assert!(Comando::Lista.entrada().is_none());
    }

    #[test]
    fn por_la_entrada_tambien_se_valida_antes_de_mandar() {
        assert!(matches!(
            por_entrada(vec![("A", "dos\nlíneas")]).argv(B),
            Err(ErrorForma::ValorDeEntorno { .. })
        ));
    }

    fn desplegar_con(entorno: Vec<(&str, &str)>, forzar: bool) -> Comando {
        Comando::Desplegar {
            app: "tienda".into(),
            progreso: true,
            entorno: entorno
                .into_iter()
                .map(|(k, v)| VariableNueva {
                    clave: k.into(),
                    valor: Secreto::nuevo(v),
                })
                .collect(),
            forzar,
            por_entrada: false,
        }
    }

    #[test]
    fn un_despliegue_normal_no_lleva_ni_env_ni_force() {
        let v = desplegar_con(vec![], false).argv(B).unwrap();
        assert_eq!(v, [B, "--json", "deploy", "tienda", "--progress"]);
    }

    #[test]
    fn las_variables_viajan_como_un_argumento_clave_valor() {
        let v = desplegar_con(vec![("DATABASE_URL", "postgres://u:p@h/db?a=b")], false)
            .argv(B)
            .unwrap();
        let i = v.iter().position(|x| x == "--env").unwrap();
        // Un `=` dentro del valor llega entero: el servidor corta por el primero.
        assert_eq!(v[i + 1], "DATABASE_URL=postgres://u:p@h/db?a=b");
    }

    #[test]
    fn forzar_se_ve_en_la_orden() {
        let v = desplegar_con(vec![], true).argv(B).unwrap();
        assert!(v.contains(&"--force".to_string()));
    }

    #[test]
    fn un_valor_que_el_servidor_rechazaria_se_dice_aqui_y_sin_el_valor() {
        for malo in ["a\nb", "a\rb", "it's", "a\0b"] {
            let e = desplegar_con(vec![("CLAVE", malo)], false)
                .argv(B)
                .unwrap_err();
            assert!(matches!(e, ErrorForma::ValorDeEntorno { .. }), "{malo:?}");
            // El mensaje nombra la clave y nunca el valor.
            let m = e.to_string();
            assert!(m.contains("CLAVE"));
            assert!(!m.contains(malo), "el error enseña el valor: {m}");
        }
    }

    #[test]
    fn una_clave_inventada_o_repetida_no_sale_de_aqui() {
        assert!(matches!(
            desplegar_con(vec![("MAL-CLAVE", "x")], false).argv(B),
            Err(ErrorForma::ClaveDeEntorno(_))
        ));
        assert!(matches!(
            desplegar_con(vec![("A", "1"), ("A", "2")], false).argv(B),
            Err(ErrorForma::ClaveRepetida(_))
        ));
    }

    #[test]
    fn la_orden_que_se_ensena_tapa_los_valores_y_nada_mas() {
        let c = desplegar_con(vec![("DB_PASSWORD", "hunter2"), ("MODE", "a=b")], true);
        let visible = c.argv_visible(B).unwrap();
        let de_verdad = c.argv(B).unwrap();
        let unida = visible.join(" ");
        assert!(!unida.contains("hunter2"));
        assert!(!unida.contains("a=b"));
        assert!(unida.contains(&format!("DB_PASSWORD={OCULTO}")));
        assert!(unida.contains(&format!("MODE={OCULTO}")));
        // El resto, idéntico: lo que se lee es lo que se ejecuta.
        assert_eq!(visible.len(), de_verdad.len());
        for (a, b) in visible.iter().zip(&de_verdad) {
            if !a.contains(OCULTO) {
                assert_eq!(a, b);
            }
        }
        assert!(!c.linea_visible(B).unwrap().contains("hunter2"));
    }

    #[test]
    fn un_comando_con_secretos_no_los_suelta_por_debug() {
        // La forma más corriente de filtrar una contraseña: un `{:?}` en un
        // mensaje de error o en un registro.
        let c = desplegar_con(vec![("DB_PASSWORD", "hunter2")], false);
        let d = format!("{c:?}");
        assert!(!d.contains("hunter2"), "{d}");
        assert!(d.contains("DB_PASSWORD"));
    }

    #[test]
    fn congelar_lleva_el_motivo_con_su_bandera_y_sin_json() {
        let v = Comando::Congelar {
            app: "tienda".into(),
            motivo: Some("-black friday".into()),
        }
        .argv(B)
        .unwrap();
        // `lock` no habla JSON: con `--json` delante el servidor se niega.
        assert_eq!(v, [B, "lock", "tienda", "-m", "-black friday"]);
        let sin = Comando::Congelar {
            app: "tienda".into(),
            motivo: None,
        }
        .argv(B)
        .unwrap();
        assert_eq!(sin, [B, "lock", "tienda"]);
        // Un motivo en blanco es no decir motivo.
        let blanco = Comando::Congelar {
            app: "tienda".into(),
            motivo: Some("   ".into()),
        }
        .argv(B)
        .unwrap();
        assert_eq!(blanco, [B, "lock", "tienda"]);
    }

    #[test]
    fn un_motivo_que_no_cabe_en_el_testigo_se_rechaza_en_vez_de_recortarse() {
        for malo in ["dos\nlíneas".to_string(), "x".repeat(201)] {
            let r = Comando::Congelar {
                app: "tienda".into(),
                motivo: Some(malo),
            }
            .argv(B);
            assert!(matches!(r, Err(ErrorForma::Motivo(_))));
        }
        // 200 justos caben, y con acentos se cuentan caracteres, no bytes.
        assert!(motivo_valido(&"á".repeat(200)).is_ok());
    }

    #[test]
    fn las_ordenes_nuevas_llevan_la_app_y_json_donde_lo_hablan() {
        let ec = Comando::EntornoComprobar { app: "web".into() }
            .argv(B)
            .unwrap();
        assert_eq!(ec, [B, "--json", "env", "check", "web"]);
        let t = Comando::Tareas { app: "web".into() }.argv(B).unwrap();
        assert_eq!(t, [B, "--json", "cron", "list", "web"]);
        let u = Comando::Descongelar { app: "web".into() }.argv(B).unwrap();
        assert_eq!(u, [B, "unlock", "web"]);
        let ci = Comando::EsperarCi {
            app: "web".into(),
            esperar: true,
        }
        .argv(B)
        .unwrap();
        assert_eq!(ci, [B, "autodeploy", "checks", "on", "web"]);
        let no = Comando::EsperarCi {
            app: "web".into(),
            esperar: false,
        }
        .argv(B)
        .unwrap();
        assert_eq!(no[3], "off");
        // Y ninguna admite una app sin nombre.
        for c in [
            Comando::EntornoComprobar { app: String::new() },
            Comando::Tareas { app: String::new() },
            Comando::Descongelar { app: String::new() },
        ] {
            assert!(c.argv(B).is_err());
        }
    }

    // ── Ronda adversaria: nombres que son palabras de Orbit ─────────────────

    #[test]
    fn una_app_que_se_llama_como_el_interruptor_de_checks_no_retarga_la_orden() {
        // `checks on no` deja al servidor sin nombre y, sin terminal, cambia la
        // primera app por orden alfabético.
        for kw in PALABRAS_DE_CHECKS {
            for esperar in [true, false] {
                let c = Comando::EsperarCi {
                    app: (*kw).into(),
                    esperar,
                };
                assert!(choca_con_palabra_de_orbit(&c), "{kw}");
                let e = c.argv(B).unwrap_err();
                assert!(
                    matches!(
                        &e,
                        ErrorForma::NombreEsPalabraDeOrbit {
                            orden: "autodeploy checks",
                            ..
                        }
                    ),
                    "{kw}: {e:?}"
                );
                let m = e.to_string();
                assert!(
                    m.contains(&format!("«{kw}»")) && m.contains("primera app"),
                    "{m}"
                );
            }
        }
        // Una app normal sigue funcionando, y `no` sigue siendo válida en otra
        // orden donde no choca.
        assert!(Comando::EsperarCi {
            app: "nota".into(),
            esperar: true
        }
        .argv(B)
        .is_ok());
        assert!(Comando::Info { app: "no".into() }.argv(B).is_ok());
        assert!(Comando::Congelar {
            app: "no".into(),
            motivo: None
        }
        .argv(B)
        .is_ok());
    }

    #[test]
    fn help_y_ayuda_no_se_mandan_a_lock_unlock_ni_remove() {
        for kw in PALABRAS_DE_AYUDA {
            let app = || (*kw).to_string();
            for c in [
                Comando::Congelar {
                    app: app(),
                    motivo: Some("x".into()),
                },
                Comando::Descongelar { app: app() },
                Comando::Retirar { app: app() },
                Comando::RetirarYBorrar { app: app() },
            ] {
                let e = c.argv(B).unwrap_err();
                assert!(
                    matches!(e, ErrorForma::NombreEsPalabraDeOrbit { .. }),
                    "{c:?}"
                );
                assert!(e.to_string().contains("sin nombre"), "{e}");
            }
            // En info o deploy, `help` es una app como otra cualquiera.
            assert!(Comando::Info { app: app() }.argv(B).is_ok());
        }
    }

    #[test]
    fn metrics_prom_es_prometheus_y_no_la_app() {
        let c = Comando::Metricas {
            app: Some("prom".into()),
        };
        assert_eq!(palabras_de_orbit(&c), PALABRAS_DE_METRICS);
        let e = c.argv(B).unwrap_err();
        assert!(e.to_string().contains("Prometheus"), "{e}");
        // Sin nombre no hay choque: es la forma de verla en el servidor.
        assert!(Comando::Metricas { app: None }.argv(B).is_ok());
        assert!(palabras_de_orbit(&Comando::Lista).is_empty());
    }

    // ── Ronda adversaria: controles y saltos de línea ───────────────────────

    #[test]
    fn un_correo_con_salto_de_linea_no_llega_al_orbit_conf_de_root() {
        for malo in [
            "a\n$(id>/tmp/p)\n#@example.com",
            "a\r@example.com",
            "a b@example.com",
            "a\t@example.com",
            "a\u{7f}@example.com",
            "a\u{85}@example.com", // NEL, C1
            "a\u{2028}@example.com",
            "a@example.com\n",
        ] {
            assert!(!correo_valido(malo), "{malo:?}");
            let mut c = nueva();
            tocar(&mut c, |n| n.correo = Some(malo.into()));
            assert!(matches!(c.argv(B), Err(ErrorForma::Correo(_))), "{malo:?}");
        }
        assert!(correo_valido("dave+certs@ejemplo.com"));
        assert!(correo_valido("ñandú@ejemplo.com"));
    }

    #[test]
    fn repo_y_rama_rechazan_controles_de_fuera_de_ascii() {
        for malo in [
            "https://github.com/a/b\u{85}x",
            "https://github.com/a/b\u{2028}",
            "a/b\u{9b}",
        ] {
            assert!(!repo_valido(malo), "{malo:?}");
        }
        for malo in ["main\u{85}", "ma\u{2028}in", "ma\u{a0}in", "ma\nin"] {
            assert!(!rama_valida(malo), "{malo:?}");
        }
        assert!(rama_valida("feature/ñandú"));
    }

    #[test]
    fn los_ajustes_de_deteccion_tienen_que_caber_en_una_linea() {
        // `--start` acaba en la unidad de systemd: un salto de línea ahí es una
        // directiva de más.
        type Caso = (&'static str, fn(&mut WebNueva));
        let casos: [Caso; 5] = [
            ("--start", |n| {
                n.ajustes.arranque = Some(Anulacion::Valor("node a\nExecStartPre=/bin/id".into()))
            }),
            ("--build", |n| {
                n.ajustes.build = Some(Anulacion::Valor("pnpm build\r".into()))
            }),
            ("--outdir", |n| {
                n.ajustes.outdir = Some(Anulacion::Valor("dist\u{85}".into()))
            }),
            ("--appdir", |n| n.ajustes.carpeta = Some("web\n".into())),
            ("--type", |n| n.ajustes.tipo = Some("node\u{1b}[2J".into())),
        ];
        for (bandera, f) in casos {
            let mut c = nueva();
            tocar(&mut c, f);
            assert_eq!(c.argv(B), Err(ErrorForma::Ajuste(bandera)));
        }
        let mut bien = nueva();
        tocar(&mut bien, |n| {
            n.ajustes.build = Some(Anulacion::Valor("pnpm build && pnpm test".into()))
        });
        assert!(bien.argv(B).is_ok());
    }

    #[test]
    fn un_motivo_con_separador_de_linea_unicode_tampoco_cabe() {
        assert!(motivo_valido("a\u{2028}b").is_err());
        assert!(motivo_valido("a\u{85}b").is_err());
    }

    // ── Ronda adversaria: el argumento único de exec ────────────────────────

    #[test]
    fn un_argumento_suelto_con_metacaracteres_no_llega_a_bash_lc() {
        for unico in [
            "id;whoami",
            "$(id)",
            "a|b",
            "x&&y",
            "`id`",
            "cat<x",
            "-x;y",
            "a\tb",
        ] {
            let v = Comando::Ejecutar {
                app: "web".into(),
                modo: ModoDeExec::Argumentos(vec![unico.into()]),
            }
            .argv(B)
            .unwrap();
            assert_eq!(
                v,
                [B, "exec", "web", "/usr/bin/env", "--", unico],
                "{unico:?}"
            );
            // Y es distinto del modo shell, que sí va a bash -lc.
            let s = Comando::Ejecutar {
                app: "web".into(),
                modo: ModoDeExec::Shell(unico.into()),
            }
            .argv(B)
            .unwrap();
            assert_eq!(s, [B, "exec", "web", "/bin/bash", "-lc", unico]);
        }
    }

    #[test]
    fn exec_sin_metacaracteres_o_con_varios_argumentos_no_cambia() {
        let uno = |a: Vec<&str>| {
            Comando::Ejecutar {
                app: "web".into(),
                modo: ModoDeExec::Argumentos(a.into_iter().map(String::from).collect()),
            }
            .argv(B)
            .unwrap()
        };
        assert_eq!(uno(vec!["ls"]), [B, "exec", "web", "ls"]);
        assert_eq!(uno(vec!["a=b"]), [B, "exec", "web", "a=b"]);
        // Con dos o más el servidor ya ejecuta tal cual.
        assert_eq!(uno(vec!["ls", "a;b"]), [B, "exec", "web", "ls", "a;b"]);
    }

    #[test]
    fn rc2_un_valor_que_el_servidor_leeria_como_idioma_no_se_manda() {
        for m in ["--lang", "--idioma", "--lang=en", "--idioma=fr"] {
            let lock = Comando::Congelar {
                app: "web".into(),
                motivo: Some(m.into()),
            };
            assert_eq!(lock.argv(B), Err(ErrorForma::ValorEsIdioma(m.into())));
            let logs = Comando::Logs {
                app: "web".into(),
                desde: Some(m.into()),
                lineas: None,
                seguir: false,
                solo_nginx: false,
            };
            assert!(logs.argv(B).is_err(), "{m}");
            // En `exec` sí: ahí el servidor no los toca.
            let exec = Comando::Ejecutar {
                app: "web".into(),
                modo: ModoDeExec::Argumentos(vec!["node".into(), m.into()]),
            };
            assert!(exec.argv(B).is_ok(), "{m}");
        }
        // Lo que se le parece y no es, pasa.
        for m in ["--language", "lang", "-- lang", "--lang-x", " --lang"] {
            let lock = Comando::Congelar {
                app: "web".into(),
                motivo: Some(m.into()),
            };
            assert!(lock.argv(B).is_ok(), "{m:?}");
        }
    }

    #[test]
    fn un_argumento_suelto_con_igual_y_metacaracteres_se_rechaza() {
        // `env 'x=1;id'` no ejecuta nada: vuelca el entorno de la app.
        let r = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Argumentos(vec!["x=1;id".into()]),
        }
        .argv(B);
        assert_eq!(r, Err(ErrorForma::ExecAmbiguo("x=1;id".into())));
    }
}
