//! Instalar Orbit en un servidor.
//!
//! # La regla que esto rompe, y por qué se rompe
//!
//! Todo el resto del cliente cumple una regla: **no escribe en el servidor nada
//! que no sea una invocación de `orbit`**. Esto no la cumple, y la primera
//! versión del alta de servidores se negaba a hacerlo por eso — enseñaba el
//! comando para copiar y que lo ejecutara una persona.
//!
//! Al mirarla de nuevo, esa negativa se sostenía mal. **El comando que se
//! ejecuta aquí es exactamente el mismo que se le pedía a alguien que copiara y
//! pegara.** La diferencia no era de seguridad sino de quién teclea, y a cambio
//! un cliente de escritorio para una herramienta de despliegue no podía poner
//! en marcha un servidor — que es la mitad del trabajo.
//!
//! Lo que sí cambia respecto de copiar y pegar, y es lo que hay que cuidar:
//!
//! * Se enseña **la secuencia literal** antes de ejecutarla. La misma regla que
//!   el repaso del asistente de web nueva.
//! * Se comprueban los requisitos **antes**, no a mitad. Un `sudo` que pide
//!   contraseña sin terminal donde escribirla cuelga la instalación en el peor
//!   momento: con medio sistema tocado.
//! * Y al terminar **no se cree ni al código de salida ni a la prosa**: se le
//!   vuelve a preguntar al servidor con `orbit version --json`, que es la misma
//!   decisión que con `orbit new`.
//!
//! # La secuencia, que no es la que yo creía
//!
//! El cliente decía «cópiate esto»:
//!
//! ```text
//! curl -fsSL https://…/install.sh | sudo bash
//! ```
//!
//! **Y eso no funciona.** `install.sh` lee el fichero `orbit` que tiene al lado
//! —lo necesita para la versión, para el núcleo de idiomas y para instalarlo en
//! `/usr/local/bin`— y muere en su línea 468 con «No encuentro el fichero
//! 'orbit' junto a install.sh». Por una tubería no hay ningún fichero al lado.
//!
//! Me lo inventé en vez de leer el README, que dice la secuencia de verdad:
//! clonar, entrar y ejecutar. Es la que está aquí.

/// Lo que hace falta en el otro lado antes de empezar.
///
/// Se comprueba en **una sola** conexión y antes de tocar nada. Descubrir a
/// mitad de una instalación que falta `git` deja un servidor a medias y a quien
/// mira sin saber qué ha quedado hecho.
pub const COMPROBACION: &str = "\
command -v git >/dev/null 2>&1 && echo git=si || echo git=no; \
command -v sudo >/dev/null 2>&1 && echo sudo=si || echo sudo=no; \
[ \"$(id -u)\" = 0 ] && echo root=si || echo root=no; \
sudo -n true >/dev/null 2>&1 && echo sudonp=si || echo sudonp=no; \
command -v orbit >/dev/null 2>&1 && echo orbit=si || echo orbit=no; \
. /etc/os-release 2>/dev/null; echo so=${ID:-?}-${VERSION_ID:-?}";

/// Lo que contestó la comprobación.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Requisitos {
    pub git: bool,
    pub sudo: bool,
    /// Ya se entra como root: entonces `sudo` no hace falta para nada.
    pub root: bool,
    /// `sudo` sin contraseña. Sin esto y sin ser root **no se puede instalar
    /// desde aquí**, porque no hay terminal donde escribir la contraseña.
    pub sudo_sin_contrasena: bool,
    /// Ya está instalado. No es un impedimento —reinstalar actualiza— pero
    /// cambia lo que hay que decirle a quien mira.
    pub ya_instalado: bool,
    /// `ubuntu-24.04`, `debian-12`… Tal cual lo dice el servidor.
    pub sistema: String,
}

impl Requisitos {
    pub fn leer(salida: &str) -> Self {
        let mut r = Self::default();
        for linea in salida.lines() {
            let Some((k, v)) = linea.trim().split_once('=') else {
                continue;
            };
            let si = v == "si";
            match k {
                "git" => r.git = si,
                "sudo" => r.sudo = si,
                "root" => r.root = si,
                "sudonp" => r.sudo_sin_contrasena = si,
                "orbit" => r.ya_instalado = si,
                "so" => r.sistema = v.to_string(),
                _ => {}
            }
        }
        r
    }

    /// Si se puede elevar sin que nadie teclee nada.
    pub fn puede_elevar(&self) -> bool {
        self.root || (self.sudo && self.sudo_sin_contrasena)
    }

    /// Lo que impide instalar, con su explicación. Vacío quiere decir que se
    /// puede.
    pub fn impedimentos(&self) -> Vec<Impedimento> {
        let mut v = Vec::new();
        if !self.git {
            v.push(Impedimento {
                clase: "git",
                que:
                    "No hay `git` en el servidor, y la instalación empieza clonando el repositorio."
                        .into(),
                arreglo: Some("sudo apt-get update && sudo apt-get install -y git".into()),
            });
        }
        if !self.puede_elevar() {
            v.push(Impedimento {
                clase: "sudo",
                que: if self.sudo {
                    "Ese usuario necesita contraseña para `sudo`, y aquí no hay terminal donde escribirla."
                        .into()
                } else {
                    "Ese usuario no tiene `sudo`, y el instalador necesita root.".into()
                },
                // No se ofrece «arreglo» ejecutable: dar sudo sin contraseña es
                // una decisión sobre la seguridad del servidor de alguien, y no
                // es nuestra. Se dice cuál es la salida y la toma quien manda.
                arreglo: None,
            });
        }
        // El sistema NO es un impedimento aunque no lo reconozcamos. El
        // instalador soporta Ubuntu 24.04 y Debian 12, y en otro puede que
        // funcione: negarse por adelantado sería decidir por alguien sobre su
        // propia máquina. Se avisa y se deja seguir.
        v
    }

    /// Lo que no impide pero conviene saber.
    pub fn avisos(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !self.sistema.is_empty()
            && !self.sistema.starts_with("ubuntu-24")
            && !self.sistema.starts_with("debian-12")
        {
            v.push(format!(
                "El instalador está probado en Ubuntu 24.04 y Debian 12, y este servidor dice ser «{}». \
                 Puede funcionar; si no, lo dirá él y no habrá tocado gran cosa.",
                self.sistema
            ));
        }
        if self.ya_instalado {
            v.push(
                "Ya hay un `orbit` en este servidor. Volver a instalar lo actualiza y \
                 **no toca las apps ni sus datos**, pero es una decisión distinta de instalarlo."
                    .into(),
            );
        }
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impedimento {
    /// Identificador estable, para decidir sin leer el texto.
    pub clase: &'static str,
    pub que: String,
    /// Qué haría falta, cuando hay algo que se pueda decir. `None` cuando la
    /// salida es una decisión de quien manda en el servidor y no una orden.
    pub arreglo: Option<String>,
}

/// De dónde se clona.
pub const REPOSITORIO: &str = "https://github.com/iNTERVOLUTIONS-Labs/orbit.git";

/// La secuencia literal, tal como la documenta el README de Orbit.
///
/// Se genera en vez de escribirse a mano en la interfaz porque **es lo que se
/// va a ejecutar**, y una pantalla que enseña una cosa y ejecuta otra es el
/// único fallo que anula por completo a una pantalla de confirmación.
///
/// `/tmp` y no el HOME: es un clon de usar y tirar, y dejarlo en la carpeta
/// personal de alguien es dejarle basura que no ha pedido. Se borra al terminar,
/// salga bien o mal.
pub fn pasos() -> Vec<String> {
    vec![
        "rm -rf /tmp/orbit-instalacion".into(),
        format!("git clone --depth 1 {REPOSITORIO} /tmp/orbit-instalacion"),
        "cd /tmp/orbit-instalacion && sudo bash install.sh".into(),
        "rm -rf /tmp/orbit-instalacion".into(),
    ]
}

/// La secuencia en una sola línea, que es como viaja.
///
/// Encadenada con `&&` salvo el borrado final, que va con `;` **a propósito**:
/// el clon temporal se limpia aunque la instalación falle. Un directorio de
/// basura en `/tmp` tras un fallo es la clase de resto que nadie recuerda que
/// está ahí.
///
/// `sudo bash` sin `-n`: los requisitos ya han comprobado que se puede elevar
/// sin contraseña, y con `-n` un `sudo` configurado de otra forma fallaría por
/// un motivo distinto del real.
pub fn orden() -> String {
    let p = pasos();
    format!("{} && {} && {} ; {}", p[0], p[1], p[2], p[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_secuencia_es_la_del_readme_y_no_una_tuberia() {
        // `curl … | sudo bash` NO funciona: install.sh lee el fichero `orbit`
        // que tiene al lado y muere si no está. Por una tubería no hay ningún
        // fichero al lado. Esta prueba existe porque ese comando estuvo puesto.
        let o = orden();
        assert!(o.contains("git clone"));
        assert!(o.contains("install.sh"));
        assert!(
            !o.contains("curl"),
            "por tubería el instalador no encuentra «orbit»"
        );
        assert!(!o.contains('|'), "nada de tuberías hacia bash");
    }

    #[test]
    fn el_clon_temporal_se_limpia_aunque_falle() {
        let o = orden();
        // El último borrado va con `;` y no con `&&`: si la instalación falla,
        // igual hay que llevarse la basura.
        let (antes, despues) = o.rsplit_once(" ; ").expect("el último paso va con ;");
        assert!(despues.contains("rm -rf"));
        assert!(antes.contains("install.sh"));
    }

    #[test]
    fn sin_git_no_se_puede_y_se_dice_como_conseguirlo() {
        let r = Requisitos {
            git: false,
            root: true,
            ..Default::default()
        };
        let i = r.impedimentos();
        assert_eq!(i.len(), 1);
        assert_eq!(i[0].clase, "git");
        assert!(i[0].arreglo.as_deref().unwrap().contains("install -y git"));
    }

    /// Sin terminal no hay dónde escribir una contraseña, así que un `sudo` que
    /// la pide es un impedimento **antes** de empezar y no un fallo a mitad.
    #[test]
    fn un_sudo_con_contrasena_impide_antes_de_tocar_nada() {
        let r = Requisitos {
            git: true,
            sudo: true,
            sudo_sin_contrasena: false,
            ..Default::default()
        };
        let i = r.impedimentos();
        assert_eq!(i[0].clase, "sudo");
        // Y no se ofrece un arreglo ejecutable: dar sudo sin contraseña es una
        // decisión sobre la seguridad del servidor de alguien.
        assert!(i[0].arreglo.is_none());
    }

    #[test]
    fn entrando_como_root_no_hace_falta_sudo() {
        let r = Requisitos {
            git: true,
            root: true,
            sudo: false,
            sudo_sin_contrasena: false,
            ..Default::default()
        };
        assert!(r.puede_elevar());
        assert!(r.impedimentos().is_empty());
    }

    /// Un sistema que no reconocemos **no impide**: negarse por adelantado
    /// sería decidir por alguien sobre su propia máquina.
    #[test]
    fn un_sistema_desconocido_avisa_pero_no_para() {
        let r = Requisitos {
            git: true,
            root: true,
            sistema: "fedora-41".into(),
            ..Default::default()
        };
        assert!(r.impedimentos().is_empty());
        assert!(r.avisos().iter().any(|a| a.contains("fedora-41")));
    }

    #[test]
    fn un_orbit_que_ya_esta_avisa_de_que_esto_es_actualizar() {
        let r = Requisitos {
            git: true,
            root: true,
            ya_instalado: true,
            sistema: "ubuntu-24.04".into(),
            ..Default::default()
        };
        assert!(r.impedimentos().is_empty());
        let a = r.avisos().join(" ");
        assert!(a.contains("actualiza"));
        assert!(a.contains("no toca las apps"));
    }

    #[test]
    fn la_comprobacion_se_lee_entera() {
        let r =
            Requisitos::leer("git=si\nsudo=si\nroot=no\nsudonp=si\norbit=no\nso=ubuntu-24.04\n");
        assert!(r.git && r.sudo && !r.root && r.sudo_sin_contrasena && !r.ya_instalado);
        assert_eq!(r.sistema, "ubuntu-24.04");
        assert!(r.puede_elevar());
    }

    #[test]
    fn una_salida_con_ruido_no_rompe_la_lectura() {
        // Por el mismo canal puede venir el motd del servidor. Lo que no tiene
        // forma de `clave=valor` se ignora sin más.
        let r = Requisitos::leer("Bienvenido a Ubuntu\ngit=si\n\n  root=si  \nbasura");
        assert!(r.git && r.root);
    }
}
