//! Los servidores que alguien añade a mano.
//!
//! # Por qué existe, y por qué no existía
//!
//! Hasta ahora los servidores salían **sólo** de `~/.ssh/config`, y eso era un
//! error de base disfrazado de decisión elegante. El razonamiento era bueno —el
//! `~/.ssh/config` es la única verdad sobre cómo llega alguien a sus
//! servidores— y la conclusión era falsa: **mucha gente no tiene ese fichero**,
//! y en Windows casi nadie. La aplicación abría vacía y sin salida.
//!
//! Así que hay dos fuentes y se juntan: el `~/.ssh/config`, que se lee y no se
//! toca, y esta lista, que es de la aplicación.
//!
//! # Qué se guarda, y qué no
//!
//! Esto **rompe a medias** la línea del modelo de amenazas que decía que el
//! cliente no persiste nada, así que conviene ser exacto sobre qué cambia:
//!
//! * Se guarda **cómo llegar a un servidor**: alias, host, usuario, puerto y la
//!   **ruta** de una clave. Es exactamente la misma clase de dato que ya vive en
//!   el `~/.ssh/config` de cualquiera, en claro, desde siempre.
//! * **No** se guarda nada que haya dicho un servidor. Ni una app, ni un estado,
//!   ni un log, ni un número. Eso sigue sin tocar el disco.
//! * **No** se guarda ninguna credencial. Ni el contenido de una clave, ni su
//!   frase de paso, ni una contraseña. La ruta sí; lo que hay dentro se lo pide
//!   `ssh` al agente, como siempre.
//!
//! El fichero se escribe con permisos `0600`. No porque lleve secretos —no los
//! lleva— sino porque la lista de las máquinas de alguien y de cómo entra en
//! ellas es un mapa que no tiene por qué leer otro usuario de la misma máquina.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Un servidor añadido a mano.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServidorGuardado {
    /// Cómo se llama en la aplicación. Es la clave, y no puede chocar con un
    /// `Host` del `~/.ssh/config`: si chocara, `ssh` resolvería el del fichero
    /// y la aplicación estaría hablando con una máquina distinta de la que
    /// enseña. Eso lo comprueba quien guarda, no esto.
    pub alias: String,
    /// IP o nombre. Lo que iría en `HostName`.
    pub host: String,
    pub usuario: String,
    pub puerto: u16,
    /// La **ruta** de la clave. Jamás su contenido ni su frase de paso.
    ///
    /// `None` quiere decir «la que use `ssh` por su cuenta», que es lo normal
    /// cuando hay un agente cargado.
    pub clave: Option<String>,
    /// Dónde está `orbit` en el otro lado, si no está donde lo deja su
    /// instalador. Siempre absoluta: resolverlo por `PATH` sería dejar que el
    /// `PATH` remoto eligiera qué se ejecuta.
    pub binario: Option<String>,
}

impl ServidorGuardado {
    /// El destino tal cual se le pasa a `ssh`.
    pub fn destino(&self) -> String {
        format!("{}@{}", self.usuario, self.host)
    }
}

/// Qué le pasa a un servidor que se intenta guardar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorDeRegistro {
    /// El alias no tiene forma de alias.
    Alias(String),
    /// Ya hay uno guardado con ese nombre.
    Repetido(String),
    /// Hay un `Host` con ese nombre en el `~/.ssh/config`.
    ///
    /// **No se deja pasar**, y no es puntilloso: `ssh` resolvería el del
    /// fichero, así que la aplicación enseñaría un servidor y hablaría con
    /// otro. Es el accidente más caro de un cliente multiservidor.
    ChocaConSshConfig(String),
    Host(String),
    Usuario(String),
    NoSePudoEscribir(String),
}

impl std::fmt::Display for ErrorDeRegistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias(a) => write!(
                f,
                "«{a}» no vale como nombre: letras, números, punto, guion y guion bajo"
            ),
            Self::Repetido(a) => write!(f, "ya tienes un servidor llamado «{a}»"),
            Self::ChocaConSshConfig(a) => write!(
                f,
                "«{a}» ya existe en tu ~/.ssh/config, y ssh usaría ése: elige otro nombre"
            ),
            Self::Host(h) => write!(f, "«{h}» no tiene forma de dirección ni de dominio"),
            Self::Usuario(u) => write!(f, "«{u}» no tiene forma de nombre de usuario"),
            Self::NoSePudoEscribir(e) => write!(f, "no he podido guardar la lista: {e}"),
        }
    }
}

impl std::error::Error for ErrorDeRegistro {}

/// La regla de forma del alias.
///
/// Es la de `ssh`, no la de las apps: aquí no manda el servidor de Orbit sino
/// el `ssh` local. Se prohíbe el guion inicial por el motivo de siempre —un
/// argumento que empieza por guion se lo come el analizador de opciones— y los
/// comodines de `ssh_config` (`*` y `?`), que convertirían este alias en un
/// patrón que casa con otros.
pub fn alias_valido(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with('-')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Un host: una IP o un nombre. Deliberadamente laxo con los nombres —quien
/// valida de verdad es el resolutor— y estricto con lo que rompe el `argv`.
pub fn host_valido(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && !s.starts_with('-')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
}

/// Un usuario de sistema. La regla de POSIX, con el guion inicial fuera.
pub fn usuario_valido(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && !s.starts_with('-')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Dónde vive la lista.
///
/// En `~/.config/orbit-desktop/` y **no** dentro de `~/.ssh/`: ese directorio es
/// de `ssh` y meterle un fichero nuestro es pedir que algún día alguien lo
/// confunda con configuración suya.
pub fn ruta() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => casa()?.join(".config"),
    };
    Some(base.join("orbit-desktop").join("servidores.json"))
}

fn casa() -> Option<PathBuf> {
    // `USERPROFILE` antes que `HOME` no: en Windows con Git Bash existen las
    // dos y `HOME` es la que apunta donde la gente espera.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Lee la lista. Que no exista **no es un error**: es una lista vacía, y es lo
/// que hay la primera vez que alguien abre la aplicación.
pub fn leer() -> Vec<ServidorGuardado> {
    let Some(p) = ruta() else { return Vec::new() };
    let Ok(texto) = std::fs::read_to_string(&p) else {
        return Vec::new();
    };
    // Un fichero corrupto tampoco tumba la aplicación: se ignora y se sigue con
    // los del ~/.ssh/config. Perder la lista es malo; no arrancar es peor.
    serde_json::from_str(&texto).unwrap_or_default()
}

/// Escribe la lista entera.
///
/// Con `0600` y **atómicamente**: se escribe a un temporal y se renombra. Un
/// corte de luz a mitad de un `write` deja un JSON partido, y eso es la lista de
/// servidores de alguien.
pub fn escribir(lista: &[ServidorGuardado]) -> Result<(), ErrorDeRegistro> {
    let p = ruta().ok_or_else(|| {
        ErrorDeRegistro::NoSePudoEscribir("no sé dónde está tu carpeta personal".into())
    })?;
    let dir = p
        .parent()
        .ok_or_else(|| ErrorDeRegistro::NoSePudoEscribir("ruta sin carpeta".into()))?;
    std::fs::create_dir_all(dir).map_err(|e| ErrorDeRegistro::NoSePudoEscribir(e.to_string()))?;

    let texto = serde_json::to_string_pretty(lista)
        .map_err(|e| ErrorDeRegistro::NoSePudoEscribir(e.to_string()))?;

    let tmp = p.with_extension("json.nuevo");
    std::fs::write(&tmp, texto.as_bytes())
        .map_err(|e| ErrorDeRegistro::NoSePudoEscribir(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }

    std::fs::rename(&tmp, &p).map_err(|e| ErrorDeRegistro::NoSePudoEscribir(e.to_string()))?;
    Ok(())
}

/// Añade uno, comprobando lo que hay que comprobar antes de escribir.
///
/// `alias_del_config` son los `Host` del `~/.ssh/config`, que quien llama ya
/// tiene: se pasan en vez de leerse aquí para que esta función no dependa del
/// disco y se pueda probar.
pub fn anadir(
    lista: &mut Vec<ServidorGuardado>,
    nuevo: ServidorGuardado,
    alias_del_config: &[String],
) -> Result<(), ErrorDeRegistro> {
    if !alias_valido(&nuevo.alias) {
        return Err(ErrorDeRegistro::Alias(nuevo.alias));
    }
    if !host_valido(&nuevo.host) {
        return Err(ErrorDeRegistro::Host(nuevo.host));
    }
    if !usuario_valido(&nuevo.usuario) {
        return Err(ErrorDeRegistro::Usuario(nuevo.usuario));
    }
    if lista.iter().any(|s| s.alias == nuevo.alias) {
        return Err(ErrorDeRegistro::Repetido(nuevo.alias));
    }
    if alias_del_config.contains(&nuevo.alias) {
        return Err(ErrorDeRegistro::ChocaConSshConfig(nuevo.alias));
    }
    lista.push(nuevo);
    Ok(())
}

pub fn olvidar(lista: &mut Vec<ServidorGuardado>, alias: &str) {
    lista.retain(|s| s.alias != alias);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(alias: &str) -> ServidorGuardado {
        ServidorGuardado {
            alias: alias.into(),
            host: "10.0.0.5".into(),
            usuario: "root".into(),
            puerto: 22,
            clave: None,
            binario: None,
        }
    }

    #[test]
    fn un_alias_que_choca_con_el_ssh_config_no_se_guarda() {
        // `ssh` resolvería el del fichero, así que la aplicación enseñaría un
        // servidor y hablaría con otro. Es el accidente más caro de un cliente
        // multiservidor y aquí es trivial de provocar sin querer.
        let mut l = Vec::new();
        let r = anadir(&mut l, s("vps-ovh"), &["vps-ovh".to_string()]);
        assert!(matches!(r, Err(ErrorDeRegistro::ChocaConSshConfig(_))));
        assert!(l.is_empty());
    }

    #[test]
    fn ni_uno_repetido() {
        let mut l = vec![s("uno")];
        assert!(matches!(
            anadir(&mut l, s("uno"), &[]),
            Err(ErrorDeRegistro::Repetido(_))
        ));
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn el_guion_inicial_y_los_comodines_fuera() {
        // Un guion inicial se lo come el analizador de opciones de `ssh`; un
        // comodín convierte el alias en un patrón que casa con otros hosts.
        assert!(!alias_valido("-oProxyCommand=algo"));
        assert!(!alias_valido("prod-*"));
        assert!(!alias_valido("prod?"));
        assert!(alias_valido("prod"));
        assert!(alias_valido("vps-ovh.2"));
    }

    #[test]
    fn el_host_admite_ipv6_y_rechaza_lo_que_rompe_el_argv() {
        assert!(host_valido("2001:db8::1"));
        assert!(host_valido("srv.ejemplo.com"));
        assert!(!host_valido("-oX=1"));
        assert!(!host_valido("con espacio"));
        assert!(!host_valido(""));
    }

    #[test]
    fn el_destino_es_lo_que_espera_ssh() {
        assert_eq!(s("x").destino(), "root@10.0.0.5");
    }

    /// Lo que **no** se guarda. Es una prueba sobre la forma del tipo, y existe
    /// para que el día que alguien añada un campo `contrasena` tenga que borrar
    /// esta prueba a propósito.
    #[test]
    fn el_tipo_no_tiene_dónde_guardar_una_credencial() {
        let j = serde_json::to_string(&s("x")).unwrap();
        for prohibido in ["password", "contrasena", "passphrase", "secret", "token"] {
            assert!(
                !j.contains(prohibido),
                "el registro no guarda «{prohibido}»"
            );
        }
        // Y la clave es una ruta, no un contenido.
        let con = ServidorGuardado {
            clave: Some("/home/quien/.ssh/id_ed25519".into()),
            ..s("y")
        };
        let j = serde_json::to_string(&con).unwrap();
        assert!(j.contains("id_ed25519"));
        assert!(!j.contains("PRIVATE KEY"));
    }

    #[test]
    fn olvidar_quita_solo_el_que_se_dice() {
        let mut l = vec![s("uno"), s("dos")];
        olvidar(&mut l, "uno");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].alias, "dos");
    }
}
