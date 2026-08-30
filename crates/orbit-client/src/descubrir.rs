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

use std::path::Path;
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

/// Los alias del fichero, en orden de aparición.
///
/// Se descartan los que **no son un destino**: los patrones con comodín
/// (`Host *`, `Host *.interno`) y las negaciones (`Host !prod`) existen para
/// aplicar opciones a un conjunto, no para conectarse a nada. Ofrecer `*` como
/// servidor importable sería ofrecer una regla como si fuera una máquina.
pub fn alias_del_fichero(texto: &str) -> Vec<String> {
    let mut fuera = Vec::new();
    for linea in texto.lines() {
        let l = linea.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        // La palabra clave no distingue mayúsculas: `Host`, `host` y `HOST` son
        // lo mismo para OpenSSH, y un cliente que sólo mirara `Host` se dejaría
        // fuera el fichero de alguien sin decir por qué.
        let (clave, resto) = match l.split_once(char::is_whitespace) {
            Some(v) => v,
            None => continue,
        };
        if !clave.eq_ignore_ascii_case("host") {
            continue;
        }
        for patron in resto.split_whitespace() {
            if patron.contains('*') || patron.contains('?') || patron.starts_with('!') {
                continue;
            }
            // Y uno que empiece por guion **no es un destino: es una opción**.
            // `ssh -G -oAlgo=x` le pasaría a OpenSSH una opción en el sitio
            // donde esperábamos un nombre. Aquí el riesgo es pequeño —el
            // fichero es del propio usuario y `-G` no conecta con nada— pero
            // dejarlo pasar es apostar a que nadie encuentre una opción que
            // importe, y esa apuesta se pierde una vez y para siempre. Se cortó
            // al ver que la regla de arquitectura señalaba este fichero.
            if patron.starts_with('-') {
                continue;
            }
            if !fuera.iter().any(|x: &String| x == patron) {
                fuera.push(patron.to_string());
            }
        }
    }
    fuera
}

/// Lo que `ssh` dice que significa un alias. Le preguntamos a él.
pub fn resolver(alias: &str, config: Option<&Path>) -> Option<AliasSsh> {
    let mut cmd = Command::new("ssh");
    if let Some(c) = config {
        cmd.arg("-F").arg(c);
    }
    // '-G' imprime la configuración efectiva y no conecta con nada. Es
    // importante que no conecte: enumerar los servidores de alguien no puede
    // ser una operación que hable con ellos.
    cmd.arg("-G").arg(alias);
    let salida = cmd.output().ok()?;
    if !salida.status.success() {
        return None;
    }
    let texto = String::from_utf8_lossy(&salida.stdout);

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

/// Los servidores que se le pueden ofrecer a alguien para importar.
///
/// No se conecta con ninguno: enumerar no es visitar. Saber si en un alias hay
/// un Orbit es otra pregunta, se hace después y una a una, porque cuesta una
/// conexión y porque hacerlo aquí convertiría abrir una pantalla en abrir
/// cuarenta sesiones SSH.
pub fn descubrir(ruta: &Path) -> Vec<AliasSsh> {
    let texto = match std::fs::read_to_string(ruta) {
        Ok(t) => t,
        // Que no haya `~/.ssh/config` no es un error: mucha gente no lo tiene.
        // Se devuelve una lista vacía y la interfaz ofrece añadir a mano.
        Err(_) => return Vec::new(),
    };
    alias_del_fichero(&texto)
        .into_iter()
        .filter_map(|a| resolver(&a, Some(ruta)))
        .collect()
}

/// Dónde vive el fichero del usuario.
pub fn ruta_por_defecto() -> Option<std::path::PathBuf> {
    // `$HOME` y no un crate de directorios: es una variable, no una biblioteca.
    std::env::var_os("HOME").map(|h| Path::new(&h).join(".ssh").join("config"))
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
