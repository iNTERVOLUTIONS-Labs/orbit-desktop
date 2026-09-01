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
//! `|null`, y en que **no hay ningún `#[serde(default)]`**. Un campo que falte es
//! un error de la respuesta, no un cero: la promesa de Orbit es que los campos se
//! añaden y nunca se renombran, así que si el que esperábamos no está, o el
//! servidor habla otro contrato o alguien nos está contestando otra cosa.
//!
//! Los campos **desconocidos sí se ignoran**, en silencio y sin avisar, porque
//! ésa es la otra mitad de la misma promesa.

use serde::Deserialize;

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

/// Los **seis** finales de `orbit deploy --all`.
///
/// Son seis y no dos a propósito, y el motivo es un fallo real: confundir
/// «no hay cambios» con «no he podido preguntar» hizo que un remoto caído se
/// anunciara como «nada que hacer» cada cinco minutos. El contrato los separa
/// para que un cliente no pueda repetirlo, así que **agruparlos está prohibido**
/// — y aquí no hay ningún método que devuelva un booleano por final, para que
/// ni siquiera sea cómodo hacerlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FinalDespliegue {
    Deployed,
    Failed,
    Unchanged,
    Unreachable,
    Gone,
    Skipped,
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
    /// La **misma regla que el código de salida**: ni fallidas, ni mudas, ni
    /// ramas desaparecidas. Existe para que un cliente que mire el objeto y otro
    /// que mire el rc no puedan discrepar nunca.
    pub ok: bool,
    pub duration_s: u64,
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
    fn los_seis_finales_son_seis() {
        let j = r#"["deployed","failed","unchanged","unreachable","gone","skipped"]"#;
        let f: Vec<FinalDespliegue> = serde_json::from_str(j).unwrap();
        assert_eq!(f.len(), 6);
        // Y son todos distintos entre sí: nada de agrupar.
        for i in 0..f.len() {
            for k in (i + 1)..f.len() {
                assert_ne!(f[i], f[k]);
            }
        }
    }
}
