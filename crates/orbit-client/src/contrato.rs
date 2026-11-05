//! El contrato `--json`, tipado.
//!
//! La regla que ordena todo este módulo está en `ARCHITECTURE §13.1` de Orbit:
//! **lo que no existe es `null`, no cero ni cadena vacía.** El puerto de una web
//! estática es `null` porque no tiene puerto; el 0 sería un puerto. Su `service`
//! también es `null` y no `"stopped"`, porque no hay ningún proceso que
//! arrancar — *confundir «no aplica» con «está caída» pinta una alarma roja
//! donde no pasa nada, y eso enseña a la gente a ignorar las alarmas*.
//!
//! Aquí eso se traduce en `Option<T>` en todos los sitios donde el contrato dice
//! `|null`, y en que **ningún campo de la línea base lleva `#[serde(default)]`**.
//! La línea base es lo que ya hablaba Orbit 1.3.6, el primer servidor contra el
//! que se escribió este cliente: un campo de ahí que falte es un error de la
//! respuesta, no un cero, porque la promesa de Orbit es que los campos se añaden
//! y nunca se renombran.
//!
//! **Los campos que llegaron después son otra cosa, y tienen otro tipo.** Un
//! parque de servidores tarda meses en actualizarse, así que hablar con un 1.3.6
//! es el caso normal y no puede ser un error. Pero tampoco se puede leer su
//! silencio como un valor: que un 1.3.6 no diga `locked` **no quiere decir que
//! la app no esté congelada**, quiere decir que ese servidor no sabe congelar.
//! Así que cada campo nuevo lleva **un nivel más de `Option`** que su tipo del
//! contrato, y el de fuera es «¿lo dice este servidor?»:
//!
//! | En el JSON              | `locked: Option<bool>` | `cert_type: Option<Option<String>>` |
//! |-------------------------|------------------------|-------------------------------------|
//! | el campo no está        | `None`                 | `None`                              |
//! | `null`                  | —                      | `Some(None)`                        |
//! | un valor                | `Some(v)`              | `Some(Some(v))`                     |
//!
//! Es la misma regla de siempre —lo que no se sabe no se pinta como un valor—
//! aplicada a un eje que la línea base no tenía: la versión del servidor.
//!
//! Los campos **desconocidos sí se ignoran**, en silencio y sin avisar, porque
//! ésa es la otra mitad de la misma promesa.

use serde::{Deserialize, Deserializer};

/// Lee un campo que **puede faltar** y **puede ser `null`**, sin confundir las
/// dos cosas. Va con `#[serde(default, deserialize_with = "presente")]`: si el
/// campo no está, `default` deja `None`; si está, esto lo envuelve en `Some`,
/// también cuando su valor es `null`.
///
/// Existe porque serde, a secas, colapsa «no está» y «es `null`» en el mismo
/// `None` —ya costó un fallo en el puente de la interfaz, con las anulaciones
/// del asistente— y aquí esa diferencia es justo la que separa «este servidor
/// es anterior a la 1.4.7» de «esta app no tiene certificado».
fn presente<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// La versión del contrato que este cliente entiende.
pub const CONTRATO_CONOCIDO: u32 = 1;

/// El saludo. Es lo primero que se pregunta y lo único que se puede preguntar
/// antes de fiarse del resto.
///
/// Publica **dos** versiones y no es redundancia: Orbit puede subir de versión
/// sin que el contrato cambie —de hecho es lo normal— y un cliente que las
/// confundiera se negaría a hablar con un servidor perfectamente compatible.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub schema: u32,
    /// La de Orbit: semver. Se compara por componentes, nunca como cadena —
    /// `1.3.6` es menor que `1.10.0` y comparar texto da lo contrario.
    pub version: String,
    /// La del contrato: un entero, y se compara como entero.
    pub contract: u32,
}

/// Qué se puede hacer con un servidor, según la versión de contrato que hable.
///
/// Existe porque un parque de servidores tarda meses en actualizarse, así que
/// hablar con uno más viejo o más nuevo **es el caso normal**, no la excepción.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibilidad {
    /// Habla nuestro contrato. Todo disponible.
    Exacta,
    /// Habla un contrato más nuevo. Se avisa y se sigue con lo que se entienda:
    /// negarse sería la peor forma de romper algo que todavía funcionaba, y las
    /// tres garantías permanentes del contrato siguen valiendo —la forma de
    /// `version --json`, la separación de canales, y que `null` siga siendo
    /// `null` y nunca un cero.
    MasNuevo { suyo: u32 },
    /// Habla un contrato más viejo. No debería ocurrir mientras sólo exista el 1,
    /// y si ocurre se trata como incompatible en vez de adivinar.
    MasViejo { suyo: u32 },
}

impl Version {
    pub fn compatibilidad(&self) -> Compatibilidad {
        match self.contract.cmp(&CONTRATO_CONOCIDO) {
            std::cmp::Ordering::Equal => Compatibilidad::Exacta,
            std::cmp::Ordering::Greater => Compatibilidad::MasNuevo {
                suyo: self.contract,
            },
            std::cmp::Ordering::Less => Compatibilidad::MasViejo {
                suyo: self.contract,
            },
        }
    }

    /// La versión de Orbit, por componentes. `None` si no es semver — que es una
    /// respuesta, no un fallo: un servidor puede llevar una versión de
    /// desarrollo y eso no impide hablar con él.
    pub fn semver(&self) -> Option<(u32, u32, u32)> {
        let base = self.version.split(['-', '+']).next()?;
        let mut it = base.split('.');
        let a = it.next()?.parse().ok()?;
        let b = it.next()?.parse().ok()?;
        let c = it.next().unwrap_or("0").parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Some((a, b, c))
    }

    /// Si este servidor sabe hacer algo que no estaba en la línea base.
    ///
    /// Por **versión de Orbit** y no por contrato: el contrato sigue siendo el 1
    /// —los campos y las órdenes se han añadido sin romper nada, que es
    /// justamente lo que el contrato promete— así que el número que dice qué hay
    /// es el otro.
    ///
    /// Con una versión que no es semver la respuesta es **no**: no se sabe, y
    /// ofrecer un botón que el servidor no entiende es peor que no ofrecerlo.
    /// Lo que se lee de una respuesta —`locked`, `cert_type`— no pasa por aquí:
    /// ahí manda que el campo esté, que es más fiable que cualquier tabla.
    pub fn sabe(&self, c: Capacidad) -> bool {
        match self.semver() {
            Some(v) => v >= c.desde(),
            None => false,
        }
    }
}

/// Lo que un servidor puede saber hacer según su versión. Cada una dice desde
/// cuándo, y el número sale del CHANGELOG de Orbit, comprobado contra el código.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capacidad {
    /// `orbit env check --json`, `deploy --env K=V` y `env_missing` en el
    /// objeto del despliegue. 1.3.8.
    EntornoDeclarado,
    /// `orbit cron list --json`: las tareas que declara el `orbit.json`. 1.3.9.
    Tareas,
    /// `orbit lock|unlock`, `deploy --force` y `state.locked`. 1.4.3.
    Congelar,
    /// `orbit autodeploy checks on|off` y `state.checks`. 1.4.3.
    EsperarCi,
    /// `state.cert_type`: de quién es el certificado. 1.4.7.
    TipoDeCertificado,
    /// `deploy --env-stdin`: las variables por la entrada estándar, sin tocar
    /// el `argv`. 1.4.9. Es lo que cierra THREAT-MODEL T-13.
    EntornoPorEntrada,
}

impl Capacidad {
    pub fn desde(&self) -> (u32, u32, u32) {
        match self {
            Self::EntornoDeclarado => (1, 3, 8),
            Self::Tareas => (1, 3, 9),
            Self::Congelar | Self::EsperarCi => (1, 4, 3),
            Self::TipoDeCertificado => (1, 4, 7),
            Self::EntornoPorEntrada => (1, 4, 9),
        }
    }
}

/// Lo que Orbit **observa** de una app, preguntándole a systemd, al disco y a
/// los certificados. Es lo contrario de `config`, que es el fichero tal cual y
/// donde todos los valores son cadenas porque en el fichero lo son.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Estado {
    /// `None` para una web estática: no hay proceso que arrancar. **No es
    /// `"stopped"`**, y pintarlo así sería inventar una avería.
    pub service: Option<String>,
    /// `None` para una web estática. El 0 sería un puerto.
    pub port: Option<u16>,
    pub ssl: bool,
    /// `None` = «no lo he mirado en esta llamada», que no es «no hay
    /// certificado». Sólo `orbit info` lo calcula; `list` y `status` lo dejan a
    /// `null` porque mirarlo cuesta.
    ///
    /// Puede ser **negativo**, y eso es real: un certificado caducado. Se pinta
    /// como caducado, no se desborda en silencio.
    pub cert_days: Option<i32>,
    pub maintenance: bool,
    /// **Lo primero que hay que mirar.** Con `false` hay app registrada y no hay
    /// vhost: nginx no atiende el dominio en absoluto —ni siquiera la página de
    /// mantenimiento—, el visitante recibe la conexión cerrada y `curl` dice
    /// `000`, ni 404 ni 502. Ningún otro campo de aquí describe lo que recibe
    /// una visita.
    pub served: bool,
    pub autodeploy: bool,
    pub queue: bool,
    pub releases: Option<u32>,
    pub last_deploy: Option<String>,
    pub last_deploy_sha: Option<String>,

    // ── Lo que llegó después de la línea base ──────────────────────────────
    // El `Option` de fuera es «¿lo dice este servidor?». Ver la cabecera.
    /// De quién es el certificado: `"letsencrypt"`, `"origin"` (Cloudflare,
    /// que **sólo vale detrás de su proxy**) o `null` sin certificado. 1.4.7.
    #[serde(default, deserialize_with = "presente")]
    pub cert_type: Option<Option<String>>,
    /// Que el autodespliegue y `deploy --all` esperan a la CI de GitHub. 1.4.3.
    #[serde(default)]
    pub checks: Option<bool>,
    /// Que los despliegues están congelados (`orbit lock`). 1.4.3.
    ///
    /// **No es un estado de lo que se sirve**: una app congelada atiende igual.
    /// Por eso no entra en [`Salud`] y va aparte, como faceta.
    #[serde(default)]
    pub locked: Option<bool>,
    /// Quién, cuándo y por qué. `Some(None)` sin congelación —«lo que no
    /// existe es `null`»— y cada campo de dentro también puede ser `null`: un
    /// `touch` a mano del testigo no dice quién.
    #[serde(default, deserialize_with = "presente")]
    pub lock: Option<Option<Congelacion>>,
}

/// La congelación de una app, tal como la cuenta `state.lock`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Congelacion {
    pub at: Option<String>,
    pub by: Option<String>,
    pub reason: Option<String>,
}

impl Estado {
    /// Si está congelada, con los tres valores posibles y no dos: `None` es
    /// «este servidor no sabe congelar», que no es «no está congelada».
    pub fn congelada(&self) -> Option<bool> {
        self.locked
    }
}

/// Cómo se pinta el estado de una app, resuelto **con un orden de precedencia
/// escrito**, porque varios pueden ser ciertos a la vez y hay que decidir una
/// vez y no en cada pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Salud {
    /// Hay app y no hay vhost. Gana sobre todo lo demás, incluido el
    /// mantenimiento: sin vhost tampoco se sirve la página de 503.
    SinVhost,
    /// Alguien la bajó a propósito. No es una avería.
    Mantenimiento,
    /// Web estática o PHP: no hay proceso. **No aplica**, que no es «caída».
    SinProceso,
    Activa,
    Parada,
    /// systemd contestó algo que no esperábamos. Se dice, no se traduce a
    /// «parada»: un estado desconocido pintado como conocido es una mentira.
    Desconocida(&'static str),
}

impl Salud {
    /// El identificador estable, que es con lo que se decide **sin leer el
    /// texto**. Es el mismo que usa la hoja de estilos de la interfaz.
    pub fn id(&self) -> &'static str {
        match self {
            Self::SinVhost => "sin-vhost",
            Self::Mantenimiento => "mantenimiento",
            Self::SinProceso => "no-aplica",
            Self::Activa => "activo",
            Self::Parada => "parado",
            Self::Desconocida(_) => "desconocido",
        }
    }

    /// El símbolo. **Nunca va solo**: es lo que sobrevive a una captura en
    /// blanco y negro y al daltonismo, pero no dice qué hacer.
    pub fn glifo(&self) -> &'static str {
        VOCABULARIO[self.indice()].1
    }

    /// Lo que se pinta al lado del glifo. En los dos estados neutros **es** el
    /// glifo: pintar los dos dejaba una fila que decía «— —».
    pub fn texto(&self) -> &'static str {
        VOCABULARIO[self.indice()].2
    }

    /// Lo que se anuncia y lo que se lee en voz alta. «raya» no es «no aplica»,
    /// así que los neutros tienen aquí su palabra entera.
    pub fn voz(&self) -> &'static str {
        VOCABULARIO[self.indice()].3
    }

    /// Qué hacer. El color sin la frase no dice nada.
    pub fn frase(&self) -> &'static str {
        VOCABULARIO[self.indice()].4
    }

    /// Glifo y texto juntos, **sin repetirse**.
    ///
    /// En los dos estados neutros el texto ES el glifo, así que pintar los dos
    /// deja una fila que dice «— —». La ventana ya se comió ese defecto y lo
    /// cazó **mirando una captura, no una prueba**: el DOM era correcto y la
    /// pantalla, absurda.
    ///
    /// Que el cliente de terminal lo cometiera otra vez, por su cuenta y a la
    /// primera, es el argumento entero de que esto viva en el núcleo: la regla
    /// no es de una interfaz, es del vocabulario.
    pub fn rotulo(&self) -> String {
        if self.texto() == self.glifo() {
            self.glifo().to_string()
        } else {
            format!("{} {}", self.glifo(), self.texto())
        }
    }

    fn indice(&self) -> usize {
        match self {
            Self::SinVhost => 0,
            Self::Mantenimiento => 1,
            Self::SinProceso => 2,
            Self::Activa => 3,
            Self::Parada => 4,
            Self::Desconocida(_) => 5,
        }
    }
}

/// El vocabulario, en el orden de `tests/contrato/vocabulario.json`.
///
/// **Vive aquí y no en la interfaz** porque hay dos interfaces —la ventana y el
/// terminal— y basta con que una diga «parado» donde la otra dice «no aplica»
/// para que la distinción entre «no hay proceso» y «el proceso se ha caído»
/// deje de existir. Esa distinción es la razón de ser de la tabla.
///
/// Una prueba lo compara contra el fichero compartido, y otra igual en la
/// interfaz hace lo mismo con el suyo.
const VOCABULARIO: [(&str, &str, &str, &str, &str); 6] = [
    (
        "sin-vhost",
        "⊘",
        "sin vhost",
        "sin vhost",
        "nginx no tiene el vhost. La conexión se cierra: ni 404 ni 502.",
    ),
    (
        "mantenimiento",
        "▲",
        "mantenimiento",
        "mantenimiento",
        "nginx devuelve 503 con tu página de «volvemos enseguida».",
    ),
    (
        "no-aplica",
        "—",
        "—",
        "no aplica",
        "Web estática: no hay ningún proceso que arrancar. Esto no es un fallo.",
    ),
    ("activo", "●", "activo", "activo", "El servicio responde."),
    (
        "parado",
        "✕",
        "parado",
        "parado",
        "El servicio existe y no está corriendo.",
    ),
    ("desconocido", "·", "·", "no se sabe", "No se sabe todavía."),
];

impl Estado {
    pub fn salud(&self) -> Salud {
        if !self.served {
            return Salud::SinVhost;
        }
        if self.maintenance {
            return Salud::Mantenimiento;
        }
        match self.service.as_deref() {
            None => Salud::SinProceso,
            Some("active") => Salud::Activa,
            Some("inactive") | Some("failed") | Some("stopped") => Salud::Parada,
            Some(_) => Salud::Desconocida("estado de servicio no reconocido"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct App {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub domain: String,
    pub aliases: Vec<String>,
    pub state: Estado,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Lista {
    pub schema: u32,
    pub apps: Vec<App>,
}

/// Los finales de `orbit deploy --all`: **once** desde la 1.4.3.
///
/// Eran seis, y son seis y no dos a propósito por un fallo real: confundir «no
/// hay cambios» con «no he podido preguntar» hizo que un remoto caído se
/// anunciara como «nada que hacer» cada cinco minutos. El contrato los separa
/// para que un cliente no pueda repetirlo, así que **agruparlos está prohibido**
/// — y aquí no hay ningún método que devuelva un booleano por final, para que
/// ni siquiera sea cómodo hacerlo.
///
/// La 1.4.3 añadió los cinco de las puertas, y cada uno dice una cosa distinta
/// de las otras diez: congelada a propósito, sin cambios en lo vigilado, y los
/// tres de la CI —esperando, en rojo, y **sin poder preguntar**, que es el
/// `unreachable` de la CI y por eso pone `ok` a `false`—.
///
/// Y **un final que no conocemos no rompe la respuesta**: llega como
/// [`FinalDespliegue::Otro`] con su nombre. La primera versión de este enumerado
/// era cerrado, y el día que el servidor añadió el séptimo final el lote entero
/// dejó de leerse —una pasada de cuarenta apps convertida en «no he entendido la
/// respuesta» por una sola fila—. Es exactamente el modo de fallo que la promesa
/// de «los campos se añaden» existe para evitar, y un valor nuevo de un campo
/// que ya existía es la misma clase de añadido.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "String")]
pub enum FinalDespliegue {
    Deployed,
    Failed,
    Unchanged,
    Unreachable,
    Gone,
    Skipped,
    /// Congelada con `orbit lock`. Ni se pregunta al remoto. 1.4.3.
    Locked,
    /// Hay commits nuevos y ninguno toca las rutas vigiladas (`watch`). 1.4.3.
    Ignored,
    /// La CI del commit sigue corriendo. 1.4.3.
    CiPending,
    /// La CI del commit ha fallado: no se despliega. 1.4.3.
    CiFailed,
    /// **No se ha podido preguntar a GitHub.** No es «pendiente» ni «en rojo»:
    /// es no saber, y por eso pone `ok` a `false` igual que `unreachable`.
    CiUnknown,
    /// Un final que este cliente no conoce todavía, con su nombre tal cual.
    Otro(String),
}

impl From<String> for FinalDespliegue {
    fn from(s: String) -> Self {
        match s.as_str() {
            "deployed" => Self::Deployed,
            "failed" => Self::Failed,
            "unchanged" => Self::Unchanged,
            "unreachable" => Self::Unreachable,
            "gone" => Self::Gone,
            "skipped" => Self::Skipped,
            "locked" => Self::Locked,
            "ignored" => Self::Ignored,
            "ci_pending" => Self::CiPending,
            "ci_failed" => Self::CiFailed,
            "ci_unknown" => Self::CiUnknown,
            _ => Self::Otro(s),
        }
    }
}

impl FinalDespliegue {
    /// El nombre tal como lo escribe el servidor.
    pub fn como_texto(&self) -> &str {
        match self {
            Self::Deployed => "deployed",
            Self::Failed => "failed",
            Self::Unchanged => "unchanged",
            Self::Unreachable => "unreachable",
            Self::Gone => "gone",
            Self::Skipped => "skipped",
            Self::Locked => "locked",
            Self::Ignored => "ignored",
            Self::CiPending => "ci_pending",
            Self::CiFailed => "ci_failed",
            Self::CiUnknown => "ci_unknown",
            Self::Otro(s) => s,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Commit {
    pub sha: Option<String>,
    pub subject: Option<String>,
    #[serde(rename = "ref")]
    pub referencia: Option<String>,
}

/// El resultado de un despliegue. **Un despliegue que falla también contesta**:
/// un `trap … EXIT` en el servidor emite el objeto con `ok:false` y
/// `failed_step`, así que un rc distinto de 0 no es motivo para descartar
/// stdout — al revés, ahí está el único motivo que se le puede enseñar a nadie.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Despliegue {
    pub schema: u32,
    pub app: String,
    pub ok: bool,
    pub release: Option<String>,
    /// La release anterior. Viaja aquí para poder ofrecer el rollback sin una
    /// segunda llamada.
    pub previous: Option<String>,
    pub commit: Commit,
    /// Salió mal y se volvió atrás.
    pub rolled_back: bool,
    /// Orbit arregló el build por su cuenta y reintentó. Es **distinto** de
    /// `rolled_back` y se enseña distinto: los dos campos existen justamente
    /// para que un panel pueda distinguir lo que es distinto.
    pub recovered: bool,
    pub duration_s: u64,
    pub failed_step: Option<String>,
    pub error: Option<String>,
    /// Las variables que siguen sin valor tras el despliegue —las obligatorias
    /// y las que no—. 1.3.8, y `None` en un servidor anterior: no es «no falta
    /// ninguna», es que ese servidor no lo cuenta.
    ///
    /// Es lo que un cliente tiene que pedirle a quien mira. Cuando falta una
    /// obligatoria el despliegue se para **antes del build** con `ok:false`, y
    /// la forma de seguir es volver a lanzarlo con `--env CLAVE=valor`.
    #[serde(default)]
    pub env_missing: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppDelLote {
    pub app: String,
    pub status: FinalDespliegue,
    /// El objeto de `deploy <app> --json` **sin recortar**. Es `None` en las
    /// apps que no se han desplegado, y entonces el motivo va en `error`: un
    /// `null` es una respuesta, un objeto a medias no.
    pub result: Option<Despliegue>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Lote {
    pub schema: u32,
    pub apps: Vec<AppDelLote>,
    pub total: u32,
    pub deployed: u32,
    pub failed: u32,
    pub unchanged: u32,
    pub unreachable: u32,
    pub gone: u32,
    pub skipped: u32,
    // Los cinco recuentos de las puertas, 1.4.3. `None` en un servidor
    // anterior: ése no puede producir esos finales, y un cero diría que ha
    // mirado.
    #[serde(default)]
    pub locked: Option<u32>,
    #[serde(default)]
    pub ignored: Option<u32>,
    #[serde(default)]
    pub ci_pending: Option<u32>,
    #[serde(default)]
    pub ci_failed: Option<u32>,
    #[serde(default)]
    pub ci_unknown: Option<u32>,
    /// La **misma regla que el código de salida**: ni fallidas, ni mudas, ni
    /// ramas desaparecidas, ni CI a la que no se ha podido preguntar. Existe
    /// para que un cliente que mire el objeto y otro que mire el rc no puedan
    /// discrepar nunca.
    pub ok: bool,
    pub duration_s: u64,
}

/// `orbit env check <app> --json`: lo que la app declara frente a lo que tiene,
/// **sin un solo valor**. 1.3.8.
///
/// Mira la release que **está sirviendo**, no la que se va a desplegar: si el
/// commit nuevo declara una variable más, esto no la ve todavía. Por eso es una
/// comprobación previa y no la última palabra — la última la dice el propio
/// despliegue con `env_missing`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ComprobacionDeEntorno {
    pub schema: u32,
    pub app: String,
    /// El fichero, relativo a `shared/`. `.env` salvo que el `orbit.json` diga
    /// otro.
    pub file: String,
    /// Ninguna obligatoria sin valor.
    pub ok: bool,
    pub vars: Vec<VariableDeclarada>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct VariableDeclarada {
    pub key: String,
    /// Si el `.env` la tiene. **Nunca el valor.**
    pub set: bool,
    pub required: bool,
    /// `"orbit.json"` o `"example"` (el `.env.example` del repositorio). Las
    /// del ejemplo nunca son obligatorias: sólo se ofrecen.
    pub source: String,
}

impl ComprobacionDeEntorno {
    /// Las que pararían el próximo despliegue antes del build.
    pub fn obligatorias_sin_valor(&self) -> Vec<&str> {
        self.vars
            .iter()
            .filter(|v| v.required && !v.set)
            .map(|v| v.key.as_str())
            .collect()
    }
}

/// `orbit cron list <app> --json`: las tareas que declara el `orbit.json`, con
/// la última y la próxima pasada. 1.3.9.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Tareas {
    pub schema: u32,
    pub app: String,
    pub tasks: Vec<Tarea>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Tarea {
    pub name: String,
    /// Tal como lo da systemd (`Wed 2026-11-05 12:00:00 UTC`), **no ISO-8601**.
    /// `null` si no ha corrido nunca: no es una fecha vacía.
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    /// El `Result` de systemd de la última pasada (`success`, `exit-code`…).
    pub result: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Comprobacion {
    pub id: String,
    pub level: String,
    pub message: String,
    pub fix: Option<String>,
    /// Si `orbit doctor --fix` se encargaría de esto. **El botón sólo se enseña
    /// aquí**: uno que no hace nada es peor que ninguno, y este campo existe
    /// exactamente para poder distinguirlo.
    pub fixable: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ResumenDoctor {
    pub ok: u32,
    pub warn: u32,
    pub error: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Doctor {
    pub schema: u32,
    pub checks: Vec<Comprobacion>,
    pub summary: ResumenDoctor,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
    /// El fichero clave a clave, y **por eso todos los valores son cadenas**: en
    /// el fichero lo son. Los datos con tipo están en `state`.
    pub config: std::collections::BTreeMap<String, String>,
    pub state: Estado,
    pub releases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Info {
    pub schema: u32,
    pub app: AppInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estado(json: &str) -> Estado {
        serde_json::from_str(json).unwrap()
    }

    const BASE: &str = r#"{"service":null,"port":null,"ssl":false,"cert_days":null,
        "maintenance":false,"served":true,"autodeploy":false,"queue":false,
        "releases":1,"last_deploy":null,"last_deploy_sha":null}"#;

    #[test]
    fn una_estatica_no_esta_parada() {
        // El fallo que el contrato existe para evitar: pintar una alarma roja
        // donde no pasa nada.
        assert_eq!(estado(BASE).salud(), Salud::SinProceso);
    }

    #[test]
    fn sin_vhost_gana_sobre_todo_lo_demas() {
        let e = estado(
            &BASE
                .replace("\"served\":true", "\"served\":false")
                .replace("\"maintenance\":false", "\"maintenance\":true"),
        );
        // Aunque esté en mantenimiento: sin vhost no se sirve ni el 503.
        assert_eq!(e.salud(), Salud::SinVhost);
    }

    #[test]
    fn un_estado_de_systemd_que_no_conocemos_no_se_traduce_a_parada() {
        let e = estado(&BASE.replace("\"service\":null", "\"service\":\"activating\""));
        assert!(matches!(e.salud(), Salud::Desconocida(_)));
    }

    #[test]
    fn un_campo_desconocido_se_ignora_sin_ruido() {
        // Los campos se añaden, nunca se renombran. Nuestra mitad es no romper.
        let e = estado(&BASE.replace("\"ssl\":false", "\"ssl\":false,\"del_futuro\":42"));
        assert!(!e.ssl);
    }

    #[test]
    fn un_campo_que_falta_es_un_error_de_la_respuesta_no_un_cero() {
        let sin_served = BASE.replace("\"served\":true,", "");
        assert!(serde_json::from_str::<Estado>(&sin_served).is_err());
    }

    #[test]
    fn null_en_un_numero_no_se_lee_como_cero() {
        let e = estado(&BASE.replace("\"releases\":1", "\"releases\":null"));
        assert_eq!(e.releases, None);
        // Y no hay forma de confundirlo con 0 sin escribirlo a mano.
        assert_ne!(e.releases, Some(0));
    }

    #[test]
    fn un_certificado_caducado_da_dias_negativos_y_es_real() {
        let e = estado(&BASE.replace("\"cert_days\":null", "\"cert_days\":-5"));
        assert_eq!(e.cert_days, Some(-5));
    }

    #[test]
    fn semver_se_compara_por_componentes_no_como_texto() {
        let v = |s: &str| Version {
            schema: 1,
            version: s.into(),
            contract: 1,
        };
        assert!(v("1.3.6").semver() < v("1.10.0").semver());
        assert_eq!(v("1.3.6").semver(), Some((1, 3, 6)));
        assert_eq!(v("2.0.0-rc1").semver(), Some((2, 0, 0)));
        assert_eq!(v("no-es-semver").semver(), None);
    }

    #[test]
    fn version_y_contrato_son_ejes_distintos() {
        // Orbit puede subir de versión sin que el contrato cambie, y es lo
        // normal. Confundirlos haría que rechazáramos un servidor compatible.
        let v = Version {
            schema: 1,
            version: "9.9.9".into(),
            contract: 1,
        };
        assert_eq!(v.compatibilidad(), Compatibilidad::Exacta);
    }

    #[test]
    fn un_contrato_mas_nuevo_no_nos_hace_negarnos() {
        let v = Version {
            schema: 1,
            version: "2.1.0".into(),
            contract: 9,
        };
        assert_eq!(v.compatibilidad(), Compatibilidad::MasNuevo { suyo: 9 });
    }

    #[test]
    fn los_once_finales_son_once_y_distintos() {
        let j = r#"["deployed","failed","unchanged","unreachable","gone","skipped",
                    "locked","ignored","ci_pending","ci_failed","ci_unknown"]"#;
        let f: Vec<FinalDespliegue> = serde_json::from_str(j).unwrap();
        assert_eq!(f.len(), 11);
        // Y son todos distintos entre sí: nada de agrupar.
        for i in 0..f.len() {
            for k in (i + 1)..f.len() {
                assert_ne!(f[i], f[k]);
            }
        }
        // Ninguno de los conocidos cae en «otro».
        assert!(!f.iter().any(|x| matches!(x, FinalDespliegue::Otro(_))));
        // Y el nombre vuelve tal cual, que es lo que pinta el terminal.
        assert_eq!(f[10].como_texto(), "ci_unknown");
    }

    #[test]
    fn un_final_que_no_conocemos_no_tumba_el_lote() {
        // El modo de fallo de la primera versión: un enumerado cerrado hacía
        // que un final nuevo convirtiera la pasada entera en un error.
        let f: FinalDespliegue = serde_json::from_str(r#""queued""#).unwrap();
        assert_eq!(f, FinalDespliegue::Otro("queued".into()));
        assert_eq!(f.como_texto(), "queued");
    }

    #[test]
    fn un_servidor_viejo_no_dice_si_esta_congelada_y_eso_no_es_un_no() {
        // La línea base (1.3.6) no tiene `locked`. Leerlo como `false` diría
        // «no está congelada» de un servidor que no sabe congelar.
        let e = estado(BASE);
        assert_eq!(e.locked, None);
        assert_eq!(e.congelada(), None);
        assert_eq!(e.cert_type, None, "ausente no es «sin certificado»");
        assert_eq!(e.lock, None);
    }

    #[test]
    fn ausente_y_null_son_dos_respuestas_distintas() {
        let nuevo = BASE.replace(
            "\"releases\":1",
            "\"releases\":1,\"cert_type\":null,\"checks\":false,\"locked\":false,\"lock\":null",
        );
        let e = estado(&nuevo);
        // Un 1.4.8 sin certificado y sin congelar: lo dice, y dice que no.
        assert_eq!(e.cert_type, Some(None));
        assert_eq!(e.locked, Some(false));
        assert_eq!(e.lock, Some(None));
        assert_eq!(e.checks, Some(false));
    }

    #[test]
    fn la_congelacion_trae_quien_cuando_y_por_que_y_cada_uno_puede_faltar() {
        let j = BASE.replace(
            "\"releases\":1",
            "\"releases\":1,\"locked\":true,\"lock\":{\"at\":\"2026-11-05T09:00:00+00:00\",\"by\":null,\"reason\":null}",
        );
        let e = estado(&j);
        assert_eq!(e.congelada(), Some(true));
        let l = e.lock.clone().unwrap().unwrap();
        assert_eq!(l.at.as_deref(), Some("2026-11-05T09:00:00+00:00"));
        // Un `touch` a mano no dice quién, y eso no se rellena con nada.
        assert_eq!(l.by, None);
        assert_eq!(l.reason, None);
        // Y congelar no cambia lo que se sirve.
        assert_eq!(e.salud(), Salud::SinProceso);
    }

    #[test]
    fn la_capacidad_sale_de_la_version_de_orbit_y_no_del_contrato() {
        let v = |s: &str| Version {
            schema: 1,
            version: s.into(),
            contract: 1,
        };
        assert!(!v("1.3.6").sabe(Capacidad::Congelar));
        assert!(v("1.4.3").sabe(Capacidad::Congelar));
        assert!(v("1.4.8").sabe(Capacidad::TipoDeCertificado));
        assert!(!v("1.4.6").sabe(Capacidad::TipoDeCertificado));
        assert!(v("1.3.8").sabe(Capacidad::EntornoDeclarado));
        assert!(!v("1.4.8").sabe(Capacidad::EntornoPorEntrada));
        assert!(v("1.4.9").sabe(Capacidad::EntornoPorEntrada));
        assert!(v("1.5.0").sabe(Capacidad::EntornoPorEntrada));
        assert!(!v("1.3.7").sabe(Capacidad::EntornoDeclarado));
        assert!(
            v("1.10.0").sabe(Capacidad::Tareas),
            "por componentes, no como texto"
        );
        // Una versión que no es semver no se sabe, y lo que no se sabe no se
        // ofrece.
        assert!(!v("desarrollo").sabe(Capacidad::Congelar));
    }
}
