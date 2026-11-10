//! Descubrir servidores en `~/.ssh/config`.
//!
//! **La decisión que ordena este módulo: no reimplementamos el parser de
//! OpenSSH. Se lo preguntamos.**
//!
//! Es la misma regla de §4.1 de la arquitectura, aplicada un nivel más abajo.
//! Un `~/.ssh/config` de verdad tiene `Match`, `Include` recursivos, patrones
//! con comodines y negaciones, `%h`/`%p`/`%r` expandiéndose en unos sitios y no
//! en otros, y una precedencia de «gana la primera aparición» que sorprende a
//! todo el mundo. Los parsers de terceros que existen cubren un trozo, y el
//! trozo que no cubren es el que usa el bastión corporativo del cliente que
//! nadie te va a contar.
//!
//! Así que **el valor efectivo de cualquier opción lo da `ssh -G`**, que es el
//! propio OpenSSH resolviendo su configuración con sus reglas y sus parches.
//!
//! Lo único que sí hay que leer del fichero es **la lista de alias**, porque
//! `ssh` no tiene un comando que los enumere. Y las dos mitades hacen falta,
//! comprobado: `ssh -G loquesea` **no falla** para un alias inexistente — se
//! inventa `hostname loquesea` y sale con 0. O sea que `-G` sabe resolver y no
//! sabe decir si algo existía.
//!
//! Lo que se guarda de todo esto es **el alias y nada más**. Lo demás se
//! enseña al importar, para que quien lo hace sepa a qué está apuntando, y
//! luego se tira: duplicarlo en nuestra configuración crearía dos verdades, y
//! la nuestra quedaría vieja en cuanto el usuario tocara su fichero.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Un alias tal y como aparece en el fichero, con lo que `ssh` dice que
/// significa. Es lo que se le enseña a alguien antes de que lo importe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AliasSsh {
    /// Lo único que se guarda.
    pub alias: String,
    pub hostname: Option<String>,
    pub usuario: Option<String>,
    pub puerto: Option<u16>,
    /// Que exista o no cambia lo que se puede prometer sobre la latencia: por
    /// un salto, el saludo se paga dos veces.
    pub salto: Option<String>,
    /// Si la clave está cifrada no se puede saber sin leerla, y no la leemos.
    /// Sólo se guarda la ruta, para poder decir cuál usaría.
    pub identidad: Option<String>,
}

/// Los alias del fichero, en orden de aparición. **Sólo este texto**: los
/// `Include` no se siguen aquí (para eso, [`alias_de`]).
///
/// Se descartan los que **no son un destino**: los patrones con comodín
/// (`Host *`, `Host *.interno`) y las negaciones (`Host !prod`) existen para
/// aplicar opciones a un conjunto, no para conectarse a nada. Ofrecer `*` como
/// servidor importable sería ofrecer una regla como si fuera una máquina.
pub fn alias_del_fichero(texto: &str) -> Vec<String> {
    let mut fuera = Vec::new();
    for linea in texto.lines() {
        if let Some(Linea::Host(patrones)) = leer_linea(linea) {
            anadir_patrones(&patrones, &mut fuera);
        }
    }
    fuera
}

/// Una línea de `ssh_config` que importa aquí, ya partida en palabras.
enum Linea {
    Host(Vec<String>),
    Include(Vec<String>),
}

/// La palabra clave y su valor. OpenSSH admite `Clave valor`, `Clave=valor` y
/// `Clave = valor`; la primera versión sólo partía por espacio, y un
/// `Host=prod` era invisible —ni se ofrecía ni contaba al comprobar choques—.
///
/// El valor se parte con [`palabras`] y no por espacios: con
/// `split_whitespace`, `Host web # el de producción` daba los alias `#`, `el`,
/// `de` y `producción` —y esta lista es también la que decide a qué se deja
/// conectar la ventana—, `Host "web"` conservaba las comillas y un `Include`
/// entre comillas con un espacio no se seguía.
fn leer_linea(linea: &str) -> Option<Linea> {
    let l = linea.trim();
    if l.is_empty() || l.starts_with('#') {
        return None;
    }
    let fin = l.find(|c: char| c.is_whitespace() || c == '=')?;
    let (clave, resto) = l.split_at(fin);
    let resto = resto.trim_start();
    let resto = resto.strip_prefix('=').unwrap_or(resto).trim_start();
    // La palabra clave no distingue mayúsculas: `Host`, `host` y `HOST` son lo
    // mismo para OpenSSH, y un cliente que sólo mirara `Host` se dejaría fuera
    // el fichero de alguien sin decir por qué.
    let es = |k: &str| clave.eq_ignore_ascii_case(k);
    if !(es("host") || es("include")) {
        return None;
    }
    // Una comilla sin cerrar: OpenSSH rechaza la línea («invalid quotes») y
    // aquí tampoco se lee nada de ella.
    let p = palabras(resto)?;
    Some(if es("host") {
        Linea::Host(p)
    } else {
        Linea::Include(p)
    })
}

/// Una línea partida como la parte OpenSSH (`argv_split` de `misc.c`, con
/// `terminate_on_comment`, desde la 8.7): comillas dobles y simples; `\` que
/// escapa una comilla, otra `\` o —fuera de comillas— un espacio, y que delante
/// de cualquier otra cosa se queda tal cual (así una ruta de Windows sobrevive);
/// y un `#` **al principio de una palabra** acaba la línea, pegado a otra cosa
/// (`web#1`) no. `None` si queda una comilla abierta.
fn palabras(s: &str) -> Option<Vec<String>> {
    let c: Vec<char> = s.chars().collect();
    let mut fuera = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == ' ' || c[i] == '\t' {
            i += 1;
            continue;
        }
        if c[i] == '#' {
            break;
        }
        let mut palabra = String::new();
        let mut comilla: Option<char> = None;
        while i < c.len() {
            let x = c[i];
            let escapa =
                |n: char| n == '\'' || n == '"' || n == '\\' || (comilla.is_none() && n == ' ');
            match c.get(i + 1) {
                Some(&n) if x == '\\' && escapa(n) => {
                    palabra.push(n);
                    i += 1;
                }
                _ if comilla.is_none() && (x == ' ' || x == '\t') => break,
                _ if comilla.is_none() && (x == '"' || x == '\'') => comilla = Some(x),
                _ if comilla == Some(x) => comilla = None,
                _ => palabra.push(x),
            }
            i += 1;
        }
        if comilla.is_some() {
            return None;
        }
        fuera.push(palabra);
    }
    Some(fuera)
}

fn anadir_patrones(patrones: &[String], fuera: &mut Vec<String>) {
    for patron in patrones {
        if patron.is_empty()
            || patron.contains('*')
            || patron.contains('?')
            || patron.starts_with('!')
        {
            continue;
        }
        // Y uno que empiece por guion **no es un destino: es una opción**.
        // `ssh -G -oAlgo=x` le pasaría a OpenSSH una opción en el sitio donde
        // esperábamos un nombre. Aquí el riesgo es pequeño —el fichero es del
        // propio usuario y `-G` no conecta con nada— pero dejarlo pasar es
        // apostar a que nadie encuentre una opción que importe, y esa apuesta
        // se pierde una vez y para siempre.
        if patron.starts_with('-') {
            continue;
        }
        if !fuera.iter().any(|x: &String| x == patron) {
            fuera.push(patron.clone());
        }
    }
}

/// Hasta dónde se siguen `Include` dentro de `Include`. Es el tope de OpenSSH
/// (`READCONF_MAX_DEPTH`); pasado, OpenSSH aborta con «Too many recursive
/// configuration includes» y aquí simplemente se deja de bajar.
const PROFUNDIDAD: usize = 16;

/// Cuántos ficheros se leen como mucho en total. Con la profundidad sola no
/// bastaba: tres ficheros que se incluyen entre sí con un comodín eran 3^16
/// lecturas, y `alias_de` está en el camino de **cada orden** de la ventana
/// (`servidor_del_config`), que se colgaba entera. Un `~/.ssh/config` de verdad
/// son unos pocos ficheros.
const TOPE_DE_FICHEROS: usize = 256;

/// Los alias de `texto` **y de lo que incluya**, en el orden en que OpenSSH los
/// leería: un `Include` mete el contenido en su sitio. Las rutas relativas son
/// relativas a `base` (`~/.ssh` para el fichero del usuario, como en OpenSSH),
/// `~` es el HOME y los comodines se expanden en orden alfabético, como hace
/// `glob(3)`. Un `Include` que no casa con nada no es un error, igual que allí.
///
/// **Cada fichero se lee una vez** (`vistos`, por ruta canónica). Un ciclo se
/// corta ahí, y no se pierde ningún alias: no se repiten, así que una segunda
/// lectura del mismo fichero no podía añadir ninguno. Se descartó un conjunto
/// por cadena de inclusión, que corta los ciclos pero deja pasar los caminos
/// distintos al mismo fichero: con n ficheros que se incluyen entre sí, n!
/// lecturas.
fn alias_con_include(
    texto: &str,
    base: &Path,
    casa: Option<&Path>,
    profundidad: usize,
    vistos: &mut std::collections::HashSet<PathBuf>,
    fuera: &mut Vec<String>,
) {
    for linea in texto.lines() {
        match leer_linea(linea) {
            Some(Linea::Host(p)) => anadir_patrones(&p, fuera),
            Some(Linea::Include(rutas)) if profundidad < PROFUNDIDAD => {
                for r in &rutas {
                    let completa = match (r.strip_prefix("~/").or(r.strip_prefix("~\\")), casa) {
                        (Some(resto), Some(h)) => h.join(resto),
                        (Some(_), None) => continue,
                        (None, _) if Path::new(r).is_absolute() || r.starts_with('/') => {
                            PathBuf::from(r)
                        }
                        (None, _) => base.join(r),
                    };
                    for f in expandir(&completa) {
                        if vistos.len() >= TOPE_DE_FICHEROS {
                            return;
                        }
                        let clave = std::fs::canonicalize(&f).unwrap_or_else(|_| f.clone());
                        if !vistos.insert(clave) {
                            continue;
                        }
                        if let Ok(t) = std::fs::read_to_string(&f) {
                            alias_con_include(&t, base, casa, profundidad + 1, vistos, fuera);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Un comodín de ruta (`*` y `?`, en cualquier tramo), sin dependencias. Como
/// en `glob(3)`, un comodín no casa con un nombre que empiece por punto.
fn expandir(ruta: &Path) -> Vec<PathBuf> {
    let mut actuales = vec![PathBuf::new()];
    for comp in ruta.components() {
        let trozo = comp.as_os_str().to_string_lossy();
        if !(trozo.contains('*') || trozo.contains('?')) {
            for a in actuales.iter_mut() {
                a.push(comp.as_os_str());
            }
            continue;
        }
        let mut siguientes = Vec::new();
        for a in &actuales {
            let Ok(entradas) = std::fs::read_dir(if a.as_os_str().is_empty() {
                Path::new(".")
            } else {
                a
            }) else {
                continue;
            };
            let mut nombres: Vec<String> = entradas
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.') && casa_comodin(&trozo, n))
                .collect();
            nombres.sort();
            siguientes.extend(nombres.into_iter().map(|n| a.join(n)));
        }
        actuales = siguientes;
    }
    actuales.retain(|p| p.is_file());
    actuales
}

fn casa_comodin(patron: &str, nombre: &str) -> bool {
    fn ir(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => ir(&p[1..], n) || (!n.is_empty() && ir(p, &n[1..])),
            (Some('?'), Some(_)) => ir(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a == b => ir(&p[1..], &n[1..]),
            _ => false,
        }
    }
    let p: Vec<char> = patron.chars().collect();
    let n: Vec<char> = nombre.chars().collect();
    ir(&p, &n)
}

/// Lo que `ssh` dice que significa un alias. Le preguntamos a él.
pub fn resolver(alias: &str, config: Option<&Path>) -> Option<AliasSsh> {
    let salida = orden_g(alias, config).output().ok()?;
    if !salida.status.success() {
        return None;
    }
    leer_g(alias, &String::from_utf8_lossy(&salida.stdout))
}

/// El `ssh` que se lanza, aquí y en el transporte.
///
/// En Windows, el de OpenSSH del sistema si está, y no el primero del `PATH`.
/// Con Git para Windows instalado, el primero suele ser el de MSYS
/// (`Git\usr\bin\ssh.exe`), cuyo arranque vuelve a partir la línea de órdenes
/// y se come las barras invertidas: un argumento escapado llegaba al servidor
/// cambiado. Y su agente no es el servicio `ssh-agent` de Windows, que es el
/// que se le pide a la gente que arranque.
///
/// `ORBIT_DESKTOP_SSH` lo cambia por otro, para quien use uno distinto a
/// propósito y para las pruebas.
pub fn binario_ssh() -> std::ffi::OsString {
    if let Some(propio) = std::env::var_os("ORBIT_DESKTOP_SSH").filter(|v| !v.is_empty()) {
        return propio;
    }
    #[cfg(windows)]
    if let Some(raiz) = std::env::var_os("SystemRoot") {
        let p = Path::new(&raiz)
            .join("System32")
            .join("OpenSSH")
            .join("ssh.exe");
        if p.is_file() {
            return p.into_os_string();
        }
    }
    "ssh".into()
}

/// La consulta, sin lanzarla: así se puede comprobar su `argv` en una prueba.
fn orden_g(alias: &str, config: Option<&Path>) -> Command {
    let mut cmd = Command::new(binario_ssh());
    sin_ventana(&mut cmd);
    if let Some(c) = config {
        cmd.arg("-F").arg(c);
    }
    // '-G' imprime la configuración efectiva y no conecta con nada. Es
    // importante que no conecte: enumerar los servidores de alguien no puede
    // ser una operación que hable con ellos.
    //
    // Y '--' antes del alias: el que llega de la interfaz no ha pasado por
    // `alias_del_fichero`, y uno que empiece por guion se leería como una
    // opción —`-oProxyCommand=…` es ejecutar una orden local—. Con el '--' es
    // un nombre y nada más.
    cmd.arg("-G").arg("--").arg(alias);
    cmd
}

/// Lo que contestó `ssh -G`, en un [`AliasSsh`].
fn leer_g(alias: &str, texto: &str) -> Option<AliasSsh> {
    let mut a = AliasSsh {
        alias: alias.to_string(),
        hostname: None,
        usuario: None,
        puerto: None,
        salto: None,
        identidad: None,
    };
    for linea in texto.lines() {
        let (clave, valor) = match linea.split_once(' ') {
            Some(v) => v,
            None => continue,
        };
        let valor = valor.trim();
        match clave {
            "hostname" => a.hostname = Some(valor.to_string()),
            "user" => a.usuario = Some(valor.to_string()),
            "port" => a.puerto = valor.parse().ok(),
            // 'none' es lo que imprime OpenSSH cuando no hay salto. Guardarlo
            // como si fuera un host haría que la interfaz anunciara un salto
            // inexistente y prometiera una latencia peor de la real.
            "proxyjump" if valor != "none" => a.salto = Some(valor.to_string()),
            // Puede haber varias; se queda la primera, que es la que `ssh`
            // probaría antes.
            "identityfile" if a.identidad.is_none() => a.identidad = Some(valor.to_string()),
            _ => {}
        }
    }
    Some(a)
}

/// Los nombres por los que `ssh` llama a un destino en sus mensajes: el
/// `HostName`, el `HostKeyAlias` y cada salto de `ProxyJump`, tal como los
/// resuelve `ssh -G`. Vacío si no contesta.
///
/// Lo usa el transporte para saber si un «connect to host …» con 255 es del
/// `ssh` propio o de uno que lanza la app: el alias `web` sale en el mensaje
/// como `10.0.0.1`, y sólo `ssh` sabe que son el mismo.
pub fn nombres_de(destino: &str, config: Option<&Path>) -> Vec<String> {
    let Ok(salida) = orden_g(destino, config).output() else {
        return Vec::new();
    };
    if !salida.status.success() {
        return Vec::new();
    }
    let mut fuera = Vec::new();
    for linea in String::from_utf8_lossy(&salida.stdout).lines() {
        let Some((clave, valor)) = linea.split_once(' ') else {
            continue;
        };
        let valor = valor.trim();
        match clave {
            "hostname" => fuera.push(valor.to_string()),
            "hostkeyalias" | "proxyjump" if valor != "none" => {
                // `ProxyJump` es una lista: `[usuario@]host[:puerto]` o
                // `ssh://…`, separados por comas.
                for salto in valor.split(',') {
                    let s = salto.trim().trim_start_matches("ssh://");
                    let s = s.rsplit('@').next().unwrap_or(s);
                    let s = match s.strip_prefix('[') {
                        Some(r) => r.split(']').next().unwrap_or(r),
                        None => s.split(':').next().unwrap_or(s),
                    };
                    fuera.push(s.to_string());
                }
            }
            _ => {}
        }
    }
    fuera
}

/// Los servidores que se le pueden ofrecer a alguien para importar.
///
/// No se conecta con ninguno: enumerar no es visitar. Saber si en un alias hay
/// un Orbit es otra pregunta, se hace después y una a una, porque cuesta una
/// conexión y porque hacerlo aquí convertiría abrir una pantalla en abrir
/// cuarenta sesiones SSH.
///
/// Las resoluciones van **en paralelo**, y en el orden del fichero. Una a una
/// eran N procesos `ssh -G` en fila —con veinte alias, de medio segundo a un
/// segundo en Windows, donde lanzar un proceso es caro— y la ventana esperaba
/// la suma. Son consultas locales e independientes, así que no hay nada que
/// coordinar: un hilo por alias, por tandas para que un fichero con doscientos
/// `Host` no lance doscientos procesos de golpe.
pub fn descubrir(ruta: &Path) -> Vec<AliasSsh> {
    // Que no haya `~/.ssh/config` no es un error: mucha gente no lo tiene. Se
    // devuelve una lista vacía y la interfaz ofrece añadir a mano.
    let Ok(alias) = alias_de(ruta) else {
        return Vec::new();
    };
    let config = config_para(ruta, ruta_por_defecto().as_deref());
    let mut fuera = Vec::with_capacity(alias.len());
    for tanda in alias.chunks(TANDA) {
        std::thread::scope(|s| {
            let hilos: Vec<_> = tanda
                .iter()
                .map(|a| s.spawn(move || resolver(a, config)))
                .collect();
            // Se recogen en el orden en que se lanzaron, que es el del fichero.
            fuera.extend(hilos.into_iter().filter_map(|h| h.join().ok().flatten()));
        });
    }
    fuera
}

/// El `-F` con el que se resuelve: **ninguno** para el fichero de siempre.
///
/// Con `-F ~/.ssh/config`, OpenSSH se salta el `ssh_config` del sistema, y el
/// transporte no pasa `-F`: la importación enseñaba un usuario, un puerto o un
/// salto distintos de los que luego usaba la conexión. Con otro fichero (el
/// `-F` del cliente de terminal) sí se pasa, porque es el que se va a usar.
fn config_para<'a>(ruta: &'a Path, por_defecto: Option<&Path>) -> Option<&'a Path> {
    if por_defecto == Some(ruta) {
        None
    } else {
        Some(ruta)
    }
}

/// Cuántos `ssh -G` a la vez. No es una cifra afinada: es el orden de magnitud
/// a partir del cual lanzar más deja de ganar tiempo y empieza a ser ruido.
const TANDA: usize = 16;

/// Los alias del fichero **sin resolver ninguno**.
///
/// Para quien sólo necesita los nombres —comprobar que un servidor añadido a
/// mano no choca con un `Host`— resolver cada uno con `ssh -G` era lanzar N
/// procesos para tirar todo lo que contestaban.
///
/// Distingue «no hay fichero», que es una lista vacía, de «hay fichero y no se
/// puede leer», que es un error: con una lista vacía por error, la comprobación
/// de choque dejaría pasar justo el alias que `ssh` resolvería distinto.
///
/// Sigue los `Include` (ver [`alias_con_include`]): un `Host` que vive en
/// `~/.ssh/config.d/` es tan del usuario como uno del fichero principal, y sin
/// esto ni se ofrecía ni contaba al comprobar que un alias guardado no choca.
pub fn alias_de(ruta: &Path) -> std::io::Result<Vec<String>> {
    let texto = match std::fs::read_to_string(ruta) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let h = casa();
    // Relativo a `~/.ssh`, como OpenSSH con un fichero de usuario; sin HOME, a
    // la carpeta del propio fichero, que es lo menos sorprendente.
    let base = h
        .as_ref()
        .map(|h| h.join(".ssh"))
        .or_else(|| ruta.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let mut fuera = Vec::new();
    // El propio fichero cuenta como leído: uno que se incluye a sí mismo se
    // corta a la primera.
    let mut vistos = std::collections::HashSet::new();
    vistos.insert(std::fs::canonicalize(ruta).unwrap_or_else(|_| ruta.to_path_buf()));
    alias_con_include(&texto, &base, h.as_deref(), 0, &mut vistos, &mut fuera);
    Ok(fuera)
}

/// Dónde vive el fichero del usuario.
///
/// En Windows, **`USERPROFILE` antes que `HOME`**, porque es lo que hace el
/// OpenSSH de Windows: lee `%USERPROFILE%\.ssh\config` diga lo que diga `HOME`.
/// La versión anterior sólo miraba `HOME`, que en un Windows normal no existe:
/// la importación salía vacía y, peor, la comprobación de choque al guardar un
/// servidor a mano no veía ningún `Host` y dejaba pasar el alias que `ssh`
/// resolvería distinto — el accidente más caro de un cliente multiservidor. Y
/// con un `HOME` de Git Bash o MSYS apuntando a otro sitio, el fichero que se
/// leía no era el que lee `ssh.exe`.
pub fn ruta_por_defecto() -> Option<std::path::PathBuf> {
    // Variables y no un crate de directorios: es una variable, no una biblioteca.
    casa().map(|h| h.join(".ssh").join("config"))
}

fn casa() -> Option<std::path::PathBuf> {
    let var = |n: &str| std::env::var_os(n).filter(|v| !v.is_empty());
    #[cfg(windows)]
    let h = var("USERPROFILE").or_else(|| var("HOME"));
    #[cfg(not(windows))]
    let h = var("HOME");
    h.map(std::path::PathBuf::from)
}

/// Que `ssh -G` no abra una consola en Windows: ver `sin_ventana` en el
/// transporte, que es donde está el porqué.
///
/// Repetido a propósito. Este fichero no puede nombrar al transporte —la prueba
/// de arquitectura lo impide, porque enumerar no es visitar— y la alternativa,
/// un tercer módulo con permiso para lanzar procesos, ensanchaba esa lista para
/// ahorrar cinco líneas.
fn sin_ventana(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

#[cfg(test)]
mod tests {
    use super::*;

    const EJEMPLO: &str = "\
# Un comentario
Host bastion
  HostName bastion.ejemplo.com
  User dave

host produccion
  HostName 10.0.0.5
  ProxyJump bastion

HOST pruebas staging
  HostName 10.0.0.9

Host *
  ServerAliveInterval 30

Host *.interno
  User root

Host !prod
  User nadie
";

    #[test]
    fn se_leen_los_alias_en_orden() {
        assert_eq!(
            alias_del_fichero(EJEMPLO),
            ["bastion", "produccion", "pruebas", "staging"]
        );
    }

    #[test]
    fn la_palabra_clave_no_distingue_mayusculas() {
        // `Host`, `host` y `HOST` son lo mismo para OpenSSH. Un cliente que sólo
        // mirara `Host` se dejaría fuera el fichero de alguien sin decir por qué.
        assert!(alias_del_fichero(EJEMPLO).contains(&"produccion".to_string()));
        assert!(alias_del_fichero(EJEMPLO).contains(&"pruebas".to_string()));
    }

    #[test]
    fn una_linea_puede_declarar_varios_alias() {
        let v = alias_del_fichero(EJEMPLO);
        assert!(v.contains(&"pruebas".to_string()) && v.contains(&"staging".to_string()));
    }

    #[test]
    fn los_comodines_y_las_negaciones_no_son_destinos() {
        // `Host *` existe para aplicar opciones a un conjunto, no para
        // conectarse a nada. Ofrecerlo como servidor importable sería ofrecer
        // una regla como si fuera una máquina.
        let v = alias_del_fichero(EJEMPLO);
        assert!(!v.iter().any(|x| x.contains('*')));
        assert!(!v.iter().any(|x| x.starts_with('!')));
    }

    #[test]
    fn un_alias_que_empieza_por_guion_no_es_un_destino() {
        // Sería una opción de ssh en el sitio donde va un nombre.
        assert_eq!(
            alias_del_fichero("Host -oProxyCommand=evil\n  HostName x\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn los_comentarios_y_las_lineas_vacias_no_estorban() {
        assert_eq!(
            alias_del_fichero("# Host falso\n\n   \n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn un_fichero_que_no_existe_da_una_lista_vacia_y_no_un_error() {
        // Mucha gente no tiene ~/.ssh/config, y eso no es un fallo.
        assert!(descubrir(Path::new("/no/existe/config")).is_empty());
    }

    #[test]
    fn le_preguntamos_a_ssh_y_no_al_fichero() {
        // La prueba de que la resolución la hace OpenSSH: se le da un fichero
        // con un alias y se comprueba que lo que vuelve son los valores
        // efectivos, no los literales del texto.
        let dir = std::env::temp_dir().join("orbit-descubrir-test");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("config");
        std::fs::write(
            &f,
            "Host uno\n  HostName 10.1.2.3\n  User ana\n  Port 2200\n\
             Host dos\n  HostName 10.9.9.9\n  ProxyJump uno\n",
        )
        .unwrap();

        let uno = resolver("uno", Some(&f)).expect("ssh -G tiene que contestar");
        assert_eq!(uno.hostname.as_deref(), Some("10.1.2.3"));
        assert_eq!(uno.usuario.as_deref(), Some("ana"));
        assert_eq!(uno.puerto, Some(2200));
        assert_eq!(uno.salto, None, "sin salto es None, nunca la cadena «none»");

        let dos = resolver("dos", Some(&f)).unwrap();
        assert_eq!(dos.salto.as_deref(), Some("uno"));

        let todos = descubrir(&f);
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].alias, "uno");
    }

    #[test]
    fn un_alias_con_guion_es_un_nombre_y_no_una_opcion_de_ssh() {
        // Sin el `--`, `ssh -G -oUser=intruso` lee una opción y no un nombre:
        // falta el destino y no contesta, o —con la opción adecuada— hace lo
        // que diga la opción. Con él, el alias es un host —raro, pero un host—
        // y la opción no se aplica.
        let args: Vec<String> = orden_g("-oProxyCommand=calc", None)
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["-G", "--", "-oProxyCommand=calc"]);
        // Y de verdad: el OpenSSH de ahora rechaza ese nombre, o lo resuelve a
        // sí mismo; lo que no hace nunca es aplicar la opción.
        if let Some(a) = resolver("-oUser=intruso", None) {
            assert_ne!(a.usuario.as_deref(), Some("intruso"));
        }
    }

    #[test]
    fn los_alias_sin_resolver_distinguen_ausente_de_ilegible() {
        assert_eq!(
            alias_de(Path::new("/no/existe/config")).unwrap(),
            Vec::<String>::new()
        );
        let dir = std::env::temp_dir().join(format!("orbit-alias-de-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("config");
        std::fs::write(&f, "Host uno dos\n  HostName x\n").unwrap();
        assert_eq!(alias_de(&f).unwrap(), ["uno", "dos"]);
        // Una carpeta donde se esperaba un fichero no se puede leer, y eso no
        // es «no hay ningún Host».
        assert!(alias_de(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn en_paralelo_se_conserva_el_orden_del_fichero() {
        let dir = std::env::temp_dir().join(format!("orbit-paralelo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("config");
        // Más de una tanda, para que el corte entre tandas también se pruebe.
        let nombres: Vec<String> = (0..40).map(|i| format!("h{i:02}")).collect();
        let texto: String = nombres
            .iter()
            .map(|n| format!("Host {n}\n  HostName 10.0.0.1\n"))
            .collect();
        std::fs::write(&f, texto).unwrap();
        let v: Vec<String> = descubrir(&f).into_iter().map(|a| a.alias).collect();
        assert_eq!(v, nombres);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_forma_con_igual_tambien_declara_alias() {
        // `Host=prod` y `Host = a b` son tan válidos para OpenSSH como
        // `Host prod`, y antes eran invisibles.
        let cfg = "Host=prod\n  HostName 1.2.3.4\nhost\tstaging\nHOST = uno dos\nHostName=x\n";
        assert_eq!(alias_del_fichero(cfg), ["prod", "staging", "uno", "dos"]);
    }

    #[test]
    fn los_include_se_siguen_con_comodines_rutas_relativas_y_tope() {
        let dir = std::env::temp_dir().join(format!("orbit-include-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.join("config.d");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("b.conf"), "Host desde-b\n").unwrap();
        std::fs::write(d.join("a.conf"), "Host desde-a\nInclude=otro\n").unwrap();
        std::fs::write(d.join(".oculto"), "Host oculto\n").unwrap();
        std::fs::write(dir.join("otro"), "Host anidado\n").unwrap();
        // Uno que se incluye a sí mismo: el tope de profundidad lo corta.
        std::fs::write(dir.join("bucle"), "Host bucle\nInclude bucle\n").unwrap();
        let principal = dir.join("config");
        std::fs::write(
            &principal,
            "Host primero\nInclude config.d/*.conf\nInclude bucle no-existe\nHost ultimo\n",
        )
        .unwrap();

        let mut v = Vec::new();
        let t = std::fs::read_to_string(&principal).unwrap();
        alias_con_include(&t, &dir, None, 0, &mut Default::default(), &mut v);
        // En orden de lectura: el contenido incluido va en su sitio, y los
        // ficheros de un comodín por orden alfabético.
        // `.oculto` no casa con `*` (glob no casa con el punto inicial) y
        // `no-existe` no es un error.
        assert_eq!(
            v,
            ["primero", "desde-a", "anidado", "desde-b", "bucle", "ultimo"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rc2_las_palabras_se_parten_como_las_parte_openssh() {
        let p = |s: &str| palabras(s);
        assert_eq!(p("web # el de produccion").unwrap(), ["web"]);
        assert_eq!(p("web#1 otro").unwrap(), ["web#1", "otro"]);
        assert_eq!(p("\"web\" 'db uno'").unwrap(), ["web", "db uno"]);
        assert_eq!(p(r#"con\ espacio a\"b"#).unwrap(), ["con espacio", "a\"b"]);
        // Una barra delante de otra cosa se queda: una ruta de Windows llega
        // entera, y `\\` es una sola.
        assert_eq!(
            p(r"C:\Users\x\.ssh\extra").unwrap(),
            [r"C:\Users\x\.ssh\extra"]
        );
        assert_eq!(p(r"a\\b").unwrap(), [r"a\b"]);
        assert_eq!(p("\"sin cerrar"), None);
        assert_eq!(p("  \t ").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn rc2_un_host_con_comentario_o_comillas_da_sus_alias_y_nada_mas() {
        let cfg = "Host web # el de produccion\n\
                   Host \"db\" 'cola uno' *.interno !prod \"\"\n\
                   Host \"roto\n\
                   Include \"sin cerrar\n";
        assert_eq!(alias_del_fichero(cfg), ["web", "db", "cola uno"]);
    }

    #[test]
    fn rc2_los_include_que_se_incluyen_entre_si_se_leen_una_vez() {
        let dir = std::env::temp_dir().join(format!("orbit-rc2-ciclo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.join("d");
        std::fs::create_dir_all(&d).unwrap();
        let patron = format!("{}/*.conf", d.to_string_lossy().replace('\\', "/"));
        for i in 1..=8 {
            std::fs::write(
                d.join(format!("{i}.conf")),
                format!("Include \"{patron}\"\nHost h{i}\n"),
            )
            .unwrap();
        }
        let cfg = dir.join("config");
        // Y se incluye a sí mismo, por ruta absoluta: una relativa sería
        // relativa a `~/.ssh`, que es el del que ejecuta la prueba.
        let a_si = cfg.to_string_lossy().replace('\\', "/");
        std::fs::write(
            &cfg,
            format!("Include {patron}\nInclude {a_si}\nHost web\n"),
        )
        .unwrap();
        let t = std::time::Instant::now();
        let v = alias_de(&cfg).unwrap();
        assert!(
            t.elapsed() < std::time::Duration::from_secs(2),
            "{:?}",
            t.elapsed()
        );
        // El orden de OpenSSH: 1.conf incluye a los demás antes de su Host.
        assert_eq!(v, ["h8", "h7", "h6", "h5", "h4", "h3", "h2", "h1", "web"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_comodin_de_ruta_es_el_de_glob() {
        assert!(casa_comodin("*.conf", "a.conf"));
        assert!(casa_comodin("a?c", "abc"));
        assert!(!casa_comodin("*.conf", "a.conf.bak"));
        assert!(casa_comodin("*", ""));
    }

    #[test]
    fn el_fichero_de_siempre_se_resuelve_sin_menos_f() {
        // Con `-F ~/.ssh/config` OpenSSH se salta el ssh_config del sistema,
        // y la conexión (que no pasa `-F`) sí lo lee.
        let d = Path::new("/home/a/.ssh/config");
        assert_eq!(config_para(d, Some(d)), None);
        let otro = Path::new("/proyecto/ssh_config");
        assert_eq!(config_para(otro, Some(d)), Some(otro));
        assert_eq!(config_para(otro, None), Some(otro));
        let args: Vec<String> = orden_g("prod", None)
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(!args.iter().any(|a| a == "-F"), "{args:?}");
    }

    #[test]
    fn un_alias_inventado_no_lo_caza_ssh_g() {
        // El motivo por el que hacen falta las DOS mitades: `ssh -G` no falla
        // para un alias que no existe — se inventa `hostname <lo que sea>` y
        // sale con 0. Sabe resolver y no sabe decir si algo existía, así que la
        // lista tiene que salir del fichero.
        let a = resolver("estealiasnoexisteenningunsitio", None);
        assert!(a.is_some());
        assert_eq!(
            a.unwrap().hostname.as_deref(),
            Some("estealiasnoexisteenningunsitio")
        );
    }
}
