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

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
    /// La ruta de la clave no es una ruta local absoluta. Ver [`clave_valida`].
    Clave(String),
    /// La ruta de `orbit` en el servidor no es absoluta. Ver [`binario_valido`].
    Binario(String),
    NoSePudoEscribir(String),
    /// La lista existe y **no se puede leer**: no es JSON, no tiene la forma, o
    /// el sistema no deja abrirla. Ver [`leer`] para por qué esto es un error y
    /// no una lista vacía.
    Ilegible {
        ruta: String,
        detalle: String,
    },
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
            Self::Clave(c) => {
                write!(
                    f,
                    "«{c}» no vale como clave: tiene que ser la ruta completa de un fichero de \
                     este ordenador, no una carpeta de red ni algo que empiece por guion"
                )?;
                // Un nombre suelto es casi siempre una clave de `~/.ssh`: `ssh`
                // lo buscaría en la carpeta desde la que se abrió la ventana.
                if !c.is_empty() && !c.contains(['/', '\\', '%']) && !c.starts_with('-') {
                    write!(f, " (¿querías decir «~/.ssh/{c}»?)")?;
                }
                Ok(())
            }
            Self::Binario(b) => write!(
                f,
                "«{b}» no vale como ruta de orbit: tiene que ser absoluta (/…) y sin espacios, \
                 o el PATH del servidor elegiría qué se ejecuta"
            ),
            Self::NoSePudoEscribir(e) => write!(f, "no he podido guardar la lista: {e}"),
            Self::Ilegible { ruta, detalle } => write!(
                f,
                "no puedo leer tu lista de servidores ({ruta}: {detalle}). No la toco para no \
                 perder lo que tenga: arréglala o muévela a otro sitio y vuelve a intentarlo"
            ),
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

/// La ruta de una clave: **local y absoluta**, o empezando por `~` (que `ssh`
/// expande al HOME de este ordenador).
///
/// Lo que se cierra con esto es una ruta UNC: `\\equipo\recurso\clave` pasada
/// como `-i` hace que Windows abra ese recurso de red, y al abrirlo **entrega
/// las credenciales NTLM del usuario** a quien lo sirva. Se rechaza también el
/// guion inicial —sería una opción de `ssh`— y los caracteres de control.
/// Relativa tampoco: dependería de desde dónde se abriera la aplicación.
pub fn clave_valida(s: &str) -> bool {
    if s.is_empty() || s.len() > 4096 || s.starts_with('-') || s.chars().any(char::is_control) {
        return false;
    }
    // `\\servidor\…`, `//servidor/…`, `\\?\UNC\…`, `\\.\pipe\…`: todo lo que
    // empieza por dos separadores sale de la máquina o a un dispositivo.
    let dos_separadores = {
        let mut cs = s.chars();
        matches!((cs.next(), cs.next()), (Some('\\' | '/'), Some('\\' | '/')))
    };
    if dos_separadores {
        return false;
    }
    if s.starts_with("~/") || s.starts_with("~\\") {
        return true;
    }
    if s.starts_with('/') {
        return true;
    }
    // `C:\…` o `C:/…`. Sólo en Windows tiene sentido; en otro sistema eso es
    // un nombre relativo raro y no una ruta.
    let b = s.as_bytes();
    cfg!(windows)
        && b.len() >= 3
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && matches!(b[2], b'\\' | b'/')
}

/// La ruta de `orbit` en el servidor: absoluta, sin espacios ni controles. Es
/// la misma regla que aplica la ventana al recibirla (`binario_aceptable`) y
/// el cliente de terminal con `--orbit`.
pub fn binario_valido(s: &str) -> bool {
    s.starts_with('/')
        && !s.starts_with("//")
        && s.len() <= 4096
        && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Todas las reglas de forma de un servidor guardado, las mismas al guardar y
/// al leer: un fichero editado a mano, o escrito por otra versión, no es de más
/// confianza que lo que se teclea.
pub fn validar(s: &ServidorGuardado) -> Result<(), ErrorDeRegistro> {
    if !alias_valido(&s.alias) {
        return Err(ErrorDeRegistro::Alias(s.alias.clone()));
    }
    if !host_valido(&s.host) {
        return Err(ErrorDeRegistro::Host(s.host.clone()));
    }
    if !usuario_valido(&s.usuario) {
        return Err(ErrorDeRegistro::Usuario(s.usuario.clone()));
    }
    if let Some(c) = s.clave.as_deref().filter(|c| !clave_valida(c)) {
        return Err(ErrorDeRegistro::Clave(c.to_string()));
    }
    if let Some(b) = s.binario.as_deref().filter(|b| !binario_valido(b)) {
        return Err(ErrorDeRegistro::Binario(b.to_string()));
    }
    Ok(())
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
    let var = |n: &str| std::env::var_os(n).filter(|v| !v.is_empty());
    // En Windows, `USERPROFILE` primero. Antes era al revés, con el argumento
    // de que en Git Bash `HOME` apunta donde la gente espera; pero la ventana
    // se abre desde el menú de inicio, donde `HOME` casi nunca existe, y desde
    // una terminal de Git Bash, donde sí. Con `HOME` primero, **la lista
    // cambiaba de sitio según desde dónde se abriera la aplicación**, y una
    // lista que a veces está y a veces no es peor que cualquier ubicación fija.
    // `USERPROFILE` existe siempre, es la carpeta que lee el `ssh.exe` del
    // sistema y, en el caso normal —sin `HOME` o con el mismo valor—, la ruta
    // no cambia para quien ya tenía la lista guardada.
    #[cfg(windows)]
    let h = var("USERPROFILE").or_else(|| var("HOME"));
    #[cfg(not(windows))]
    let h = var("HOME").or_else(|| var("USERPROFILE"));
    h.map(PathBuf::from)
}

/// Lee la lista. Que no exista **no es un error**: es una lista vacía, y es lo
/// que hay la primera vez que alguien abre la aplicación.
///
/// Que exista y no se pueda leer **sí lo es**, y antes no lo era: un fichero
/// corrupto se leía como una lista vacía, y el siguiente «guardar» o «olvidar»
/// escribía esa lista vacía encima. Es decir, se borraba en silencio un fichero
/// que se podía arreglar a mano, justo cuando alguien estaba intentando añadir
/// un servidor más.
///
/// Se eligió negarse a escribir y no hacer una copia `servidores.json.corrupto`
/// y seguir. La copia parece más amable, pero deja dos ficheros y una ventana
/// que de repente no enseña los servidores de alguien sin que nada lo explique;
/// y si el fichero estaba a medio escribir por otro proceso, la «reparación»
/// pisa lo bueno. Negarse deja una sola verdad en disco, intacta, y un mensaje
/// que dice dónde está. Quien sólo **lee** —para conectar— puede seguir con los
/// del `~/.ssh/config`; quien escribe, no.
pub fn leer() -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    Ok(leer_completo()?.servidores)
}

/// [`leer`], desde una ruta concreta. Aparte para poder probarlo sin tocar las
/// variables del proceso, que son globales y las pruebas corren en hilos.
///
/// Sólo devuelve los que pasan [`validar`]: lo que no, se aparta (ver
/// [`leer_completo_de`] para saber qué y por qué).
pub fn leer_de(p: &Path) -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    Ok(leer_completo_de(p)?.servidores)
}

/// Lo que hay en la lista, separado en lo que se puede usar y lo que no.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lectura {
    pub servidores: Vec<ServidorGuardado>,
    /// Entradas que **no se usan** y por qué. Siguen en el fichero —no se
    /// borra lo que alguien puede arreglar a mano— pero no llegan a `ssh`.
    pub descartados: Vec<Descartado>,
}

/// Una entrada apartada al leer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descartado {
    /// El alias que dice tener, si dice alguno.
    pub alias: Option<String>,
    pub motivo: String,
}

/// [`leer`], con lo apartado. Es lo que debería enseñar la interfaz: una
/// entrada que desaparece sin explicación es peor que una que no funciona.
pub fn leer_completo() -> Result<Lectura, ErrorDeRegistro> {
    match ruta() {
        Some(p) => leer_completo_de(&p),
        None => Ok(Lectura::default()),
    }
}

pub fn leer_completo_de(p: &Path) -> Result<Lectura, ErrorDeRegistro> {
    let c = leer_crudo(p)?;
    Ok(Lectura {
        servidores: c.validos,
        descartados: c
            .apartados
            .into_iter()
            .map(|a| Descartado {
                alias: a.alias,
                motivo: a.motivo,
            })
            .collect(),
    })
}

/// El fichero tal cual: lo válido tipado y lo apartado **en crudo**, para
/// volver a escribirlo intacto.
struct Crudo {
    validos: Vec<ServidorGuardado>,
    apartados: Vec<Apartado>,
}

struct Apartado {
    alias: Option<String>,
    valor: serde_json::Value,
    motivo: String,
}

/// Cada entrada por separado: una mal formada **no tumba la lista entera**,
/// que era dejar sin servidores a quien tiene uno roto. Lo que sí sigue
/// siendo un error es un fichero que no es JSON o no es una lista: eso ya no
/// se puede separar en entradas, y negarse es lo único que no pisa nada.
fn leer_crudo(p: &Path) -> Result<Crudo, ErrorDeRegistro> {
    let ilegible = |detalle: String| ErrorDeRegistro::Ilegible {
        ruta: p.display().to_string(),
        detalle,
    };
    let texto = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Crudo {
                validos: Vec::new(),
                apartados: Vec::new(),
            })
        }
        Err(e) => return Err(ilegible(e.to_string())),
    };
    let valores: Vec<serde_json::Value> =
        serde_json::from_str(&texto).map_err(|e| ilegible(e.to_string()))?;
    let mut c = Crudo {
        validos: Vec::new(),
        apartados: Vec::new(),
    };
    for valor in valores {
        let alias = valor
            .get("alias")
            .and_then(|a| a.as_str())
            .map(String::from);
        let motivo = match serde_json::from_value::<ServidorGuardado>(valor.clone()) {
            Err(e) => format!("no tiene la forma de un servidor guardado ({e})"),
            Ok(s) => match validar(&s) {
                Err(e) => e.to_string(),
                Ok(()) if c.validos.iter().any(|v| v.alias == s.alias) => {
                    "está repetido; se usa el primero".to_string()
                }
                Ok(()) => {
                    c.validos.push(s);
                    continue;
                }
            },
        };
        c.apartados.push(Apartado {
            alias,
            valor,
            motivo,
        });
    }
    Ok(c)
}

/// Escribe la lista entera.
///
/// Con `0600` y **atómicamente**: se escribe a un temporal y se renombra. Un
/// corte de luz a mitad de un `write` deja un JSON partido, y eso es la lista de
/// servidores de alguien.
///
/// Las entradas apartadas al leer se conservan (salvo las que `lista` ya
/// sustituye por alias). Y un fichero que no se puede leer **no se pisa**.
///
/// Ojo: `leer` → cambiar → `escribir` en dos llamadas **no es atómico** —otro
/// hilo puede escribir en medio y su cambio se pierde—. Para eso está
/// [`modificar`] (o [`guardar`] y [`quitar`]), que hace las tres cosas bajo el
/// mismo cerrojo.
pub fn escribir(lista: &[ServidorGuardado]) -> Result<(), ErrorDeRegistro> {
    transaccion(|c| {
        c.validos = lista.to_vec();
        Ok(())
    })
    .map(|_| ())
}

/// Lee, aplica `f` y escribe, **con la lista cerrada** para el resto de la
/// aplicación y para otra ventana de la misma persona.
///
/// Antes cada llamada hacía su `leer` y su `escribir`, con un temporal de nombre
/// fijo y sin cerrojo: dos «guardar» seguidos —un doble clic, o guardar y
/// olvidar a la vez en el pool de la ventana— daban un «no he podido guardar»
/// falso (el otro ya había renombrado el temporal) o perdían uno de los dos
/// cambios. Con 24 guardados a la vez quedaba 1 servidor en el fichero.
pub fn modificar(
    f: impl FnOnce(&mut Vec<ServidorGuardado>) -> Result<(), ErrorDeRegistro>,
) -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    transaccion(|c| f(&mut c.validos))
}

/// [`anadir`] y escribir, de una vez.
pub fn guardar(
    nuevo: ServidorGuardado,
    alias_del_config: &[String],
) -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    modificar(|l| anadir(l, nuevo, alias_del_config))
}

/// [`olvidar`] y escribir, de una vez. Se lleva también una entrada apartada con
/// ese alias: olvidar es la forma de deshacerse de una que está mal.
pub fn quitar(alias: &str) -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    transaccion(|c| {
        olvidar(&mut c.validos, alias);
        c.apartados.retain(|a| a.alias.as_deref() != Some(alias));
        Ok(())
    })
}

/// Un solo cerrojo por proceso para todo lo que escribe la lista. Los hilos de
/// la ventana se ordenan aquí; el fichero `.cerrojo` es para otro proceso.
static CERROJO: Mutex<()> = Mutex::new(());

fn transaccion(
    f: impl FnOnce(&mut Crudo) -> Result<(), ErrorDeRegistro>,
) -> Result<Vec<ServidorGuardado>, ErrorDeRegistro> {
    let no_escribe = ErrorDeRegistro::NoSePudoEscribir;
    let p = ruta().ok_or_else(|| no_escribe("no sé dónde está tu carpeta personal".into()))?;
    let dir = p
        .parent()
        .ok_or_else(|| no_escribe("ruta sin carpeta".into()))?
        .to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| no_escribe(e.to_string()))?;

    // Un hilo que entró en pánico con el cerrojo no deja la lista a medias —se
    // escribe por renombrado—, así que el envenenamiento no significa nada aquí.
    let _hilo = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let _proceso = CerrojoDeFichero::tomar(p.with_extension("json.cerrojo"))?;

    let mut c = leer_crudo(&p)?;
    f(&mut c)?;
    for s in &c.validos {
        validar(s)?;
    }
    // Lo apartado se conserva salvo que ya haya uno bueno con su alias: el
    // bueno es el que alguien acaba de guardar en su lugar.
    let validos = &c.validos;
    c.apartados.retain(|a| {
        a.alias
            .as_deref()
            .map_or(true, |x| !validos.iter().any(|v| v.alias == x))
    });

    let mut valores = Vec::with_capacity(c.validos.len() + c.apartados.len());
    for s in &c.validos {
        valores.push(serde_json::to_value(s).map_err(|e| no_escribe(e.to_string()))?);
    }
    valores.extend(c.apartados.into_iter().map(|a| a.valor));
    let texto = serde_json::to_string_pretty(&valores).map_err(|e| no_escribe(e.to_string()))?;
    escribir_atomico(&p, &dir, texto.as_bytes()).map_err(|e| no_escribe(e.to_string()))?;
    Ok(c.validos)
}

/// Temporal **con nombre propio** en la misma carpeta (el renombrado sólo es
/// atómico dentro de un mismo sistema de ficheros), `fsync` antes de
/// renombrar —sin él, tras un corte, el renombrado puede llegar al disco antes
/// que el contenido— y `0600` desde que se crea, no después.
fn escribir_atomico(p: &Path, dir: &Path, datos: &[u8]) -> std::io::Result<()> {
    // Hora en nanosegundos y un contador, no el pid: este módulo no toca
    // `std::process` (la prueba de arquitectura lo reserva al transporte). Entre
    // procesos ya ordena el `.cerrojo`, y `create_new` falla antes que pisar.
    static SERIE: AtomicU64 = AtomicU64::new(0);
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(
        "servidores.json.{ns}.{}.nuevo",
        SERIE.fetch_add(1, Ordering::Relaxed)
    ));
    let r = (|| {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            o.mode(0o600);
        }
        let mut f = o.open(&tmp)?;
        f.write_all(datos)?;
        f.sync_all()?;
        drop(f);
        renombrar(&tmp, p)?;
        // Y la carpeta, para que el renombrado mismo sobreviva a un corte.
        #[cfg(unix)]
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
        Ok(())
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// En Windows un renombrado encima de un fichero que otro programa tiene
/// abierto sin compartir (el antivirus, el indexador) falla con «acceso
/// denegado» durante unos milisegundos. Se reintenta un poco; un fallo que dura
/// más es de verdad.
fn renombrar(de: &Path, a: &Path) -> std::io::Result<()> {
    let mut intentos = 0;
    loop {
        match std::fs::rename(de, a) {
            Err(e) if cfg!(windows) && e.kind() == ErrorKind::PermissionDenied && intentos < 20 => {
                intentos += 1;
                std::thread::sleep(Duration::from_millis(25));
            }
            r => return r,
        }
    }
}

/// Cerrojo entre procesos: un fichero creado con `create_new`, que es atómico en
/// cualquier sistema. Se descartó `File::lock` porque pide Rust 1.89 y el
/// proyecto promete 1.80, y una dependencia para esto no se justifica.
///
/// Un cerrojo de un proceso que murió se queda en disco —en release se compila
/// con `panic = "abort"`, así que no hay `Drop` que lo quite—, y pasado `CADUCA`
/// se da por abandonado. Escribir la lista son milisegundos, así que el margen
/// sobra.
///
/// Dentro va un testigo propio, y por dos motivos. Al soltarlo sólo se borra si
/// el testigo es el nuestro: antes se borraba lo que hubiera, y un proceso lento
/// al que otro había dado por muerto le quitaba el cerrojo al vivo. Y uno
/// abandonado se aparta **renombrándolo** a un nombre propio en vez de borrarlo:
/// de dos procesos que lo ven abandonado a la vez sólo a uno le sale el
/// renombrado, y el otro vuelve a esperar.
struct CerrojoDeFichero {
    ruta: PathBuf,
    testigo: String,
}

impl CerrojoDeFichero {
    /// Corto, porque es lo que espera quien guarda detrás de un proceso que
    /// murió con el cerrojo puesto.
    const CADUCA: Duration = Duration::from_secs(4);
    /// **Más larga que `CADUCA`.** Era al revés (5 s contra 10), y un cierre a
    /// medias de una ventana dejaba los guardados fallando durante diez
    /// segundos con un mensaje que culpaba a otra ventana que no existía.
    const ESPERA: Duration = Duration::from_secs(7);

    fn tomar(p: PathBuf) -> Result<Self, ErrorDeRegistro> {
        let testigo = testigo();
        let inicio = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
            {
                Ok(mut f) => {
                    // Si no se puede escribir el testigo, el cerrojo vale igual
                    // para ordenar; sólo no se soltará al terminar y caducará.
                    let _ = f.write_all(testigo.as_bytes());
                    return Ok(Self { ruta: p, testigo });
                }
                // Windows contesta «acceso denegado» mientras otro lo está
                // borrando: eso también es «ocupado». Pero **sólo si el cerrojo
                // existe**: en una carpeta de sólo lectura `create_new` da el
                // mismo error, y antes se contaba como «otra ventana está
                // guardando».
                Err(e) if e.kind() == ErrorKind::AlreadyExists || existe(&p) => {
                    if abandonado(&p) {
                        apartar(&p, &testigo);
                        continue;
                    }
                    if inicio.elapsed() > Self::ESPERA {
                        return Err(ErrorDeRegistro::NoSePudoEscribir(format!(
                            "otra ventana de Orbit Desktop lleva más de {} s guardando la lista. \
                             Vuelve a intentarlo; si no hay otra abierta, borra {}",
                            Self::ESPERA.as_secs(),
                            p.display()
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
                Err(e) => {
                    return Err(ErrorDeRegistro::NoSePudoEscribir(format!(
                        "no puedo crear {}: {e}",
                        p.display()
                    )))
                }
            }
        }
    }
}

impl Drop for CerrojoDeFichero {
    fn drop(&mut self) {
        if std::fs::read_to_string(&self.ruta).is_ok_and(|t| t == self.testigo) {
            let _ = std::fs::remove_file(&self.ruta);
        }
    }
}

/// Un testigo que no repite ningún otro proceso: la hora, un contador y la
/// semilla aleatoria de `RandomState`, que la biblioteca estándar saca del
/// sistema. Sin el pid: este módulo no toca `std::process`.
fn testigo() -> String {
    use std::hash::{BuildHasher, Hasher};
    static SERIE: AtomicU64 = AtomicU64::new(0);
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(ns);
    format!(
        "{ns:x}-{:x}-{:016x}",
        SERIE.fetch_add(1, Ordering::Relaxed),
        h.finish()
    )
}

/// Si hay algo en esa ruta, aunque no se pueda mirar (Windows niega el acceso a
/// un fichero que se está borrando).
fn existe(p: &Path) -> bool {
    match std::fs::symlink_metadata(p) {
        Ok(_) => true,
        Err(e) => e.kind() != ErrorKind::NotFound,
    }
}

/// Si el cerrojo lleva más de `CADUCA` sin tocarse **en cualquier sentido del
/// reloj**. Una fecha del futuro —un reloj que NTP corrige hacia atrás, un perfil
/// en red con otro reloj— daba `Err` en `elapsed()`, se leía como «no es viejo» y
/// el cerrojo de un proceso muerto no caducaba nunca.
fn abandonado(p: &Path) -> bool {
    let Ok(m) = std::fs::metadata(p).and_then(|m| m.modified()) else {
        return false;
    };
    let edad = match m.elapsed() {
        Ok(d) => d,
        Err(e) => e.duration(),
    };
    edad > CerrojoDeFichero::CADUCA
}

/// Aparta un cerrojo abandonado con un renombrado a un nombre propio, que es
/// atómico: si otro proceso se ha adelantado, el renombrado falla y no se toca
/// nada. Si entre mirarlo y renombrarlo otro lo había tomado, lo apartado es
/// suyo y está fresco: se devuelve a su sitio con `hard_link`, que no pisa un
/// cerrojo que ya haya creado un tercero.
fn apartar(p: &Path, testigo: &str) {
    let nombre = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let aparte = p.with_file_name(format!("{nombre}.{testigo}.viejo"));
    if std::fs::rename(p, &aparte).is_err() {
        return;
    }
    if !abandonado(&aparte) {
        let _ = std::fs::hard_link(&aparte, p);
    }
    let _ = std::fs::remove_file(&aparte);
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
    // Las mismas reglas que al leer, clave y binario incluidos: antes sólo se
    // miraban alias, host y usuario, y una clave `\\equipo\recurso\x`
    // se guardaba y viajaba como `-i` en cada conexión.
    validar(&nuevo)?;
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
    fn una_lista_corrupta_es_un_error_y_no_una_lista_vacia() {
        // Antes se leía como `[]`, y el siguiente guardado escribía `[]` encima:
        // la lista de alguien, borrada en silencio.
        let dir = std::env::temp_dir().join(format!("orbit-registro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("servidores.json");

        // No existir sí es una lista vacía.
        assert_eq!(leer_de(&p), Ok(Vec::new()));

        let roto = "[{\"alias\":\"prod\",\"host\":\"10.0.0.5\",";
        std::fs::write(&p, roto).unwrap();
        assert!(matches!(leer_de(&p), Err(ErrorDeRegistro::Ilegible { .. })));
        // Y leerla no la toca.
        assert_eq!(std::fs::read_to_string(&p).unwrap(), roto);

        // Una sana se lee entera.
        std::fs::write(&p, serde_json::to_string(&vec![s("uno")]).unwrap()).unwrap();
        assert_eq!(leer_de(&p).unwrap(), vec![s("uno")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn una_clave_de_red_o_relativa_no_se_guarda() {
        // Una ruta UNC como `-i` hace que Windows entregue las credenciales
        // NTLM del usuario a quien sirva ese recurso.
        for mala in [
            r"\\atacante.example\recurso\id_ed25519",
            "//atacante.example/recurso/id_ed25519",
            r"\\?\UNC\atacante\r\k",
            r"\\.\pipe\x",
            r"\/atacante/x",
            "-oProxyCommand=calc",
            "id_ed25519",
            r".ssh\id_ed25519",
            "",
            "/home/a/.ssh/id\nx",
        ] {
            assert!(!clave_valida(mala), "{mala:?}");
            let mut l = Vec::new();
            let r = anadir(
                &mut l,
                ServidorGuardado {
                    clave: Some(mala.into()),
                    ..s("x")
                },
                &[],
            );
            assert!(matches!(r, Err(ErrorDeRegistro::Clave(_))), "{mala:?}");
        }
        for buena in ["/home/a/.ssh/id_ed25519", "~/.ssh/id_ed25519", r"~\.ssh\id"] {
            assert!(clave_valida(buena), "{buena:?}");
        }
        assert_eq!(clave_valida(r"C:\Users\a\.ssh\id"), cfg!(windows));
        assert_eq!(clave_valida("C:/Users/a/.ssh/id"), cfg!(windows));
    }

    #[test]
    fn el_binario_guardado_es_absoluto() {
        for malo in ["orbit", "bin/orbit", "//h/orbit", "/usr/bin/or bit", "/x\n"] {
            assert!(!binario_valido(malo), "{malo:?}");
            let mut l = Vec::new();
            let r = anadir(
                &mut l,
                ServidorGuardado {
                    binario: Some(malo.into()),
                    ..s("x")
                },
                &[],
            );
            assert!(matches!(r, Err(ErrorDeRegistro::Binario(_))), "{malo:?}");
        }
        assert!(binario_valido("/usr/local/bin/orbit"));
    }

    fn fichero(nombre: &str, texto: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("orbit-reg-{nombre}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("servidores.json");
        std::fs::write(&p, texto).unwrap();
        (dir, p)
    }

    #[test]
    fn al_leer_se_aparta_lo_que_no_valdria_al_guardar_y_se_dice_por_que() {
        let (dir, p) = fichero(
            "leer",
            r#"[
              {"alias":"bueno","host":"10.0.0.5","usuario":"root","puerto":22,"clave":null,"binario":null},
              {"alias":"x y","host":"-oProxyCommand=calc","usuario":"a b","puerto":22,"clave":null,"binario":"orbit"},
              {"alias":"unc","host":"10.0.0.6","usuario":"root","puerto":22,"clave":"\\\\atacante\\r\\k","binario":null},
              {"alias":"rel","host":"10.0.0.7","usuario":"root","puerto":22,"clave":null,"binario":"orbit"},
              {"alias":"roto","puerto":"veintidós"},
              {"alias":"bueno","host":"10.9.9.9","usuario":"root","puerto":22,"clave":null,"binario":null}
            ]"#,
        );
        let l = leer_completo_de(&p).unwrap();
        assert_eq!(l.servidores.len(), 1);
        assert_eq!(
            l.servidores[0].host, "10.0.0.5",
            "el primero de los repetidos"
        );
        let motivos: Vec<(Option<&str>, &str)> = l
            .descartados
            .iter()
            .map(|d| (d.alias.as_deref(), d.motivo.as_str()))
            .collect();
        assert_eq!(motivos.len(), 5, "{motivos:?}");
        assert!(motivos[0].1.contains("no vale como nombre"), "{motivos:?}");
        assert!(motivos[1].1.contains("clave"), "{motivos:?}");
        assert!(motivos[2].1.contains("ruta de orbit"), "{motivos:?}");
        assert!(motivos[3].1.contains("forma"), "{motivos:?}");
        assert!(motivos[4].1.contains("repetido"), "{motivos:?}");
        // `leer_de` sólo entrega lo que se puede usar.
        assert_eq!(leer_de(&p).unwrap(), l.servidores);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_fichero_que_no_es_una_lista_sigue_siendo_ilegible() {
        let (dir, p) = fichero("nolista", r#"{"alias":"x"}"#);
        assert!(matches!(leer_de(&p), Err(ErrorDeRegistro::Ilegible { .. })));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn carpeta(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("orbit-rc2-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rc2_un_cerrojo_abandonado_caduca_aunque_su_fecha_sea_del_futuro() {
        let dir = carpeta("futuro");
        let p = dir.join("x.cerrojo");
        for desfase in [3600i64, -3600] {
            let f = std::fs::File::create(&p).unwrap();
            let t = if desfase > 0 {
                std::time::SystemTime::now() + Duration::from_secs(desfase as u64)
            } else {
                std::time::SystemTime::now() - Duration::from_secs((-desfase) as u64)
            };
            f.set_modified(t).unwrap();
            drop(f);
            let t0 = Instant::now();
            let c = CerrojoDeFichero::tomar(p.clone()).expect("lo toma");
            assert!(t0.elapsed() < Duration::from_secs(2), "{desfase}");
            drop(c);
            assert!(!p.exists(), "se suelta al terminar");
        }
        // Y nunca quedan restos del apartado.
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rc2_soltar_el_cerrojo_no_borra_el_de_otro() {
        let dir = carpeta("testigo");
        let p = dir.join("x.cerrojo");
        let c = CerrojoDeFichero::tomar(p.clone()).unwrap();
        // Otro proceso lo dio por abandonado y puso el suyo.
        std::fs::write(&p, "de-otro").unwrap();
        drop(c);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "de-otro");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rc2_esperar_dura_mas_que_caducar() {
        // Si no, un cerrojo recién abandonado hace fallar el guardado antes
        // de caducar.
        assert!(CerrojoDeFichero::ESPERA > CerrojoDeFichero::CADUCA);
    }

    /// En una carpeta en la que no se puede escribir, el error es ése y no
    /// «otra ventana está guardando». Sólo en Unix: en Windows el atributo de
    /// sólo lectura de una carpeta no impide crear ficheros dentro.
    #[cfg(unix)]
    #[test]
    fn rc2_una_carpeta_de_solo_lectura_no_es_otra_ventana() {
        use std::os::unix::fs::PermissionsExt;
        let dir = carpeta("sololectura");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        // root escribe igual: ahí no hay nada que probar.
        if std::fs::File::create(dir.join("sonda")).is_ok() {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        let t0 = Instant::now();
        let e = CerrojoDeFichero::tomar(dir.join("x.cerrojo"))
            .err()
            .expect("falla");
        assert!(t0.elapsed() < Duration::from_secs(1));
        assert!(!e.to_string().contains("otra ventana"), "{e}");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn olvidar_quita_solo_el_que_se_dice() {
        let mut l = vec![s("uno"), s("dos")];
        olvidar(&mut l, "uno");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].alias, "dos");
    }
}
