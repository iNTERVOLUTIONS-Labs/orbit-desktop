//! El transporte: **el único sitio del código que puede invocar `ssh`**.
//!
//! Se delega en el binario `ssh` del sistema y no en una librería, y la decisión
//! está razonada en `docs/ARCHITECTURE.md §4.1`. En una frase: la pregunta no es
//! «¿qué librería SSH?» sino **«¿quién interpreta `~/.ssh/config`?»**, y la
//! respuesta honesta es que ninguna librería lo hace del todo. Delegando se
//! gana `Match`, `Include`, `ProxyJump`, `ProxyCommand`, el agente con sus
//! llaves en hardware, `ControlMaster`, y la verificación de `known_hosts` hecha
//! por OpenSSH con sus propios parches. Y **cero superficie criptográfica
//! propia**: un CVE en libssh2 sería nuestro, uno en OpenSSH es un `apt upgrade`
//! del usuario.
//!
//! Es lo que hacen VS Code Remote-SSH y `git`.
//!
//! ## Las tres cosas que hay que hacer bien aquí
//!
//! **stdout y stderr se leen a la vez, nunca en serie.** Leerlos uno detrás de
//! otro es un bloqueo mutuo en cuanto el que no se está leyendo llena su tubería
//! —64 KB en Linux—, y el Orbit real escribe por stderr todo lo dirigido a una
//! persona, así que se llena de verdad. Aquí se resuelve con un hilo por
//! descriptor, que es lo más simple que funciona y no arrastra un runtime async
//! al núcleo.
//!
//! **Por stdout va un solo objeto y por stderr nunca van datos.** Basura antes
//! del JSON es un **fallo**, no algo que recortar: buscar la primera llave es
//! exactamente cómo un servidor comprometido cuela un objeto suyo delante del
//! legítimo.
//!
//! **Hay un presupuesto de tamaño y de tiempo.** Un servidor comprometido puede
//! contestar 4 GB o gotear un byte por minuto; ninguna de las dos cosas puede
//! consumir el cliente.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::comando::{Comando, ErrorForma, Secreto, Vena};

/// Cuánto se acepta de una respuesta antes de cortar.
///
/// Medido contra un banco de 40 apps: **ninguna respuesta del contrato pasa de
/// 20 KB**, así que un tope de 8 MB no protegía de nada — era un número
/// tranquilizador. Éste deja margen de sobra para el caso legítimo más grande
/// (`traffic` con muchas rutas) y corta mucho antes de que importe.
pub const TOPE_RESPUESTA: usize = 4 * 1024 * 1024;

/// Cuánto se espera. Va **por comando** y no hay un número único, porque los
/// costes reales son de órdenes distintas: `version` son 72 ms y un despliegue
/// son tres minutos de build. Un tope único o ahoga al despliegue o no protege
/// de nada.
pub fn tope_de_tiempo(c: &Comando) -> Duration {
    match c {
        // Un flujo en vivo no tiene plazo: no terminar es lo que hace.
        Comando::Logs { seguir: true, .. } => Duration::MAX,

        // Un build de verdad tarda minutos, y ése es el trabajo. `new` despliega
        // por dentro: con los 30 s de abajo se mataba a mitad del primer build.
        Comando::Desplegar { .. } | Comando::Nueva(_) => Duration::from_secs(3600),
        // La pasada no comparte tope con un despliegue: son N builds en fila, y
        // 40 apps a 3 min ya son dos horas. Esto no protege de un servidor
        // muerto —eso lo hace `ServerAliveInterval`—, sólo de uno desbocado.
        Comando::DesplegarTodo { .. } => Duration::from_secs(6 * 3600),
        // Lo que un proceso de la app tarde, que no sabemos: una migración
        // grande son minutos. Cortar antes no para nada (ver `ESPERA_DEL_CERROJO`).
        Comando::Ejecutar { .. } => Duration::from_secs(3600),

        // Las que cambian algo y no son un build. `rollback` espera al cerrojo
        // de la app hasta `DEPLOY_LOCK_WAIT`; las demás no lo toman, pero
        // `remove --purge` para la unidad (hasta 90 s) y borra la carpeta, y
        // `doctor --fix` renueva certificados. Mismo tope para todas: cortarlas
        // con 30 s no las para, sólo hace que la interfaz mienta.
        Comando::Revertir { .. }
        | Comando::Retirar { .. }
        | Comando::RetirarYBorrar { .. }
        | Comando::Congelar { .. }
        | Comando::Descongelar { .. }
        | Comando::EsperarCi { .. }
        | Comando::DoctorArreglar => ESPERA_DEL_CERROJO + Duration::from_secs(300),

        // Abre y comprueba cada fichero de copia, uno por uno.
        Comando::CopiasVerificar => Duration::from_secs(600),

        // Medido: 1,42 s con 40 apps. Pero hace un `dig` por dominio, y 40
        // consultas contra un resolutor que agote su plazo son minutos.
        Comando::Doctor => Duration::from_secs(120),
        // Lee logs de nginx que en una web con tráfico son cientos de megas.
        Comando::Trafico { .. } => Duration::from_secs(120),

        // Medido: 2,1 s con 40 apps, porque la CPU es la diferencia entre dos
        // lecturas y hay que esperar a la segunda a propósito.
        Comando::Top => Duration::from_secs(60),
        // Medido: 936 ms con el histórico vacío — son ~7 procesos por app.
        Comando::Metricas { .. } => Duration::from_secs(60),
        // Ventana de log acotada, pero puede filtrar un fichero grande.
        Comando::Logs { .. } => Duration::from_secs(60),

        // Todo lo demás está medido por debajo de 400 ms con 40 apps, y el
        // suelo por llamada es de 72 ms. Treinta segundos son casi cien veces
        // el peor caso medido: un `version` que tarda más que eso no es lento,
        // es un servidor roto o uno que está goteando a propósito, y cuanto
        // antes se diga, mejor.
        //
        // El plazo va por comando y no hay un número único porque los costes
        // reales son de órdenes distintas: un tope único o ahoga al despliegue
        // o no protege de nada.
        _ => Duration::from_secs(30),
    }
}

/// Lo que el servidor espera al cerrojo por app antes de rendirse:
/// `DEPLOY_LOCK_WAIT`, 900 s de fábrica (orbit:436).
///
/// Importa porque **matar el `ssh` local no para la orden remota**: con `-T` no
/// hay pty y no llega ningún SIGHUP. Un `rollback` cortado a los 30 s seguía
/// esperando allí y se aplicaba minutos después de que la interfaz dijera «no
/// ha contestado» — y su `logline` de auditoría ya no lo leía nadie.
pub const ESPERA_DEL_CERROJO: Duration = Duration::from_secs(900);

/// Si la orden **cambia algo** en el servidor.
///
/// Lo pregunta quien tiene que decir qué pasó cuando no lo sabe: un plazo
/// vencido en una orden de lectura es «no ha contestado», y en una que cambia
/// algo es «puede que lo haya hecho igualmente». Sin `_` a propósito: una orden
/// nueva tiene que decidir en qué lado está.
pub fn cambia_el_servidor(c: &Comando) -> bool {
    match c {
        Comando::Version
        | Comando::Lista
        | Comando::Estado
        | Comando::Info { .. }
        | Comando::Doctor
        | Comando::Top
        | Comando::Metricas { .. }
        | Comando::Trafico { .. }
        | Comando::EntornoLista { .. }
        | Comando::EntornoValor { .. }
        | Comando::BaseDeDatosLista
        | Comando::RedireccionesLista { .. }
        | Comando::VigilanciaEstado
        | Comando::ColaEstado
        | Comando::CopiasLista
        | Comando::CopiasVerificar
        | Comando::Logs { .. }
        | Comando::EntornoComprobar { .. }
        | Comando::Tareas { .. } => false,
        // `exec` cuenta como que cambia: no sabemos qué hace, y suponer que
        // sólo lee es la suposición cara.
        Comando::DoctorArreglar
        | Comando::Desplegar { .. }
        | Comando::DesplegarTodo { .. }
        | Comando::Congelar { .. }
        | Comando::Descongelar { .. }
        | Comando::EsperarCi { .. }
        | Comando::Ejecutar { .. }
        | Comando::Retirar { .. }
        | Comando::RetirarYBorrar { .. }
        | Comando::Revertir { .. }
        | Comando::Nueva(_) => true,
    }
}

/// Si el `ok` del objeto **es el desenlace de la orden**, y por tanto tiene que
/// casar con el código de salida.
///
/// No vale para todas: `env check --json` sale con 0 y `ok:false` cuando faltan
/// variables (orbit:16049), porque ahí `ok` describe el `.env`, no la orden. Sí
/// para `deploy` (el trap pone `ok:false` si sale con error), `deploy --all`
/// (sale con error si falla, no contesta o falta la CI de alguna) y `backup
/// verify` (sale con 1 si hay alguna mala).
fn ok_es_el_desenlace(c: &Comando) -> bool {
    matches!(
        c,
        Comando::Desplegar { .. } | Comando::DesplegarTodo { .. } | Comando::CopiasVerificar
    )
}

/// Cómo se llega a un servidor. Cuando está en `~/.ssh/config`, **se guarda el
/// alias y nada más**: el usuario, el puerto, la clave y el `ProxyJump` los
/// resuelve `ssh`. Duplicar eso aquí crearía dos verdades, y la nuestra
/// quedaría vieja en cuanto el usuario tocara su config.
#[derive(Debug, Clone)]
pub struct Servidor {
    pub alias: String,
    /// El destino tal cual se le pasa a `ssh`: un alias de `~/.ssh/config`, o
    /// `usuario@host`.
    pub destino: String,
    /// Ruta **absoluta** del binario. Nunca se resuelve por `PATH`.
    pub binario: String,
    /// Multiplexado. Es la palanca de latencia más grande del producto —medido:
    /// 246 ms de saludo sin él, 13 ms con él— y a la vez una superficie: un
    /// socket de control es una sesión root guardada en un fichero, y con él
    /// abierto se puede abrir un canal **sin la clave**. Por eso el `persist` es
    /// corto y se puede apagar.
    pub multiplexar: bool,
    /// Un `~/.ssh/config` alternativo. Normalmente `None`: se usa el del
    /// usuario, que es la única verdad sobre cómo llega a sus servidores.
    pub config_ssh: Option<String>,
    /// Sólo para servidores que **no** están en `~/.ssh/config`. Cuando lo
    /// están se queda a `None` y lo resuelve `ssh`: duplicarlo aquí crearía dos
    /// verdades, y la nuestra quedaría vieja en cuanto el usuario tocara su
    /// config.
    pub puerto: Option<u16>,
    /// La **ruta** de la clave, jamás su contenido ni su frase de paso. Lo que
    /// necesite una credencial se la pide al agente.
    pub clave: Option<String>,
}

impl Servidor {
    pub fn nuevo(alias: impl Into<String>, destino: impl Into<String>) -> Self {
        Self {
            alias: alias.into(),
            destino: destino.into(),
            binario: "/usr/local/bin/orbit".into(),
            multiplexar: true,
            config_ssh: None,
            puerto: None,
            clave: None,
        }
    }

    /// Las opciones de `ssh`, y cada una está aquí por un motivo escrito.
    pub fn opciones_ssh(&self, dir_control: Option<&str>) -> Vec<String> {
        let mut o: Vec<String> = Vec::new();
        // '-F' va primero: lo que venga detrás son opciones nuestras y tienen
        // que poder ganarle a lo que diga el fichero. Normalmente es None y se
        // usa el ~/.ssh/config del usuario, que es la única verdad sobre cómo
        // llega a sus servidores.
        if let Some(c) = &self.config_ssh {
            o.push("-F".into());
            o.push(c.clone());
        }
        // Puerto y clave sólo para servidores que NO están en el config. Cuando
        // lo están, los resuelve ssh y aquí van a None.
        if let Some(p) = self.puerto {
            o.push("-p".into());
            o.push(p.to_string());
        }
        if let Some(k) = &self.clave {
            o.push("-i".into());
            o.push(k.clone());
        }
        o.extend([
            // No hay TTY y no debe haberlo: pedirlo cambiaría el comportamiento
            // del otro lado (color, animaciones, selectores interactivos).
            "-T".to_string(),
            // Nunca preguntar por la terminal. Sin esto, una clave con frase de
            // paso que no esté en el agente cuelga el proceso para siempre.
            "-o".into(),
            "BatchMode=yes".into(),
            // Acepta un host nuevo y **se niega en redondo si la clave de uno
            // conocido ha cambiado**. Ni `yes` —que hace que el primer contacto
            // falle y el usuario acabe desactivándolo entero— ni `no`, que no
            // protege de nada.
            "-o".into(),
            "StrictHostKeyChecking=accept-new".into(),
            // Reenviar el agente le daría a ese servidor la capacidad de usar la
            // clave del usuario mientras la sesión esté abierta: un servidor
            // comprometido escalando a todos los demás. Y no hace falta para
            // nada: el `git fetch` lo hace el servidor con SUS credenciales.
            "-o".into(),
            "ForwardAgent=no".into(),
            "-o".into(),
            "ClearAllForwardings=yes".into(),
            // Lo mismo para X11: un `Host * / ForwardX11 yes` del fichero del
            // usuario le abriría su servidor gráfico a una máquina en la que
            // root puede ser otro. Se apaga aquí, que gana sobre el fichero.
            "-o".into(),
            "ForwardX11=no".into(),
            // Redundante con `-T` a propósito: un `RequestTTY force` en el
            // fichero del usuario no puede ganarle a la línea de órdenes, y así
            // la intención queda escrita por su nombre y no sólo por una letra.
            "-o".into(),
            "RequestTTY=no".into(),
            // Acota el establecimiento. Sin esto, un host que se traga los SYN
            // —un VPS apagado detrás de un cortafuegos— cuesta los ~21 s del
            // connect de TCP en Windows, o más en Linux, antes de que nadie diga
            // nada; y en la instalación, que no tiene plazo, sería para siempre.
            "-o".into(),
            "ConnectTimeout=10".into(),
            // Detectan una conexión muerta —portátil que se duerme, wifi que
            // cambia, VPN que cae— en ~45 s en vez de en los ~15 minutos del
            // keepalive de TCP. Es lo que hace que un despliegue de una hora de
            // tope, o una instalación sin tope, termine con «se perdió el
            // contacto» en vez de quedarse colgado para siempre.
            "-o".into(),
            "ServerAliveInterval=15".into(),
            "-o".into(),
            "ServerAliveCountMax=3".into(),
        ]);
        if self.multiplexar {
            if let Some(d) = dir_control {
                o.extend([
                    "-o".into(),
                    "ControlMaster=auto".into(),
                    // '%C' es un hash de host, puerto, usuario y salto: no
                    // expone el hostname en una ruta que otro proceso lista.
                    "-o".into(),
                    format!("ControlPath={d}/%C"),
                    // Corto a propósito. Nunca 'yes': un master eterno es una
                    // sesión root abierta hasta que alguien reinicie. Y con una
                    // llave que pide un toque por conexión, el toque se pide una
                    // vez por 'persist' — que es la propiedad incómoda de esto.
                    "-o".into(),
                    "ControlPersist=45".into(),
                ]);
            }
        }
        o
    }

    /// La invocación entera de `ssh`: opciones, `--`, destino y la línea.
    ///
    /// Vive en un solo sitio porque las cuatro rutas que lanzan `ssh` la
    /// repetían, y la primera vez que hubo que cambiarla —el `--`— hubo que
    /// cambiarla en cuatro. Una regla que se cumple en cuatro sitios a mano se
    /// incumple en el quinto.
    ///
    /// **El `--` antes del destino no es estilo.** Sin él, un destino que empiece
    /// por guion —un alias como `-oProxyCommand=…` o `-F\\host\share\cfg`
    /// llegado desde la interfaz— lo lee `ssh` como una **opción**, y
    /// `ProxyCommand` es ejecutar una orden local. Con él, lo que venga detrás es
    /// un nombre de host y nada más, aunque empiece por guion: `ssh` falla al
    /// resolverlo, que es lo correcto. Lo pide `docs/CLIENT.md §3.2` desde el
    /// principio y el código no lo hacía.
    fn orden_ssh(&self, dir_control: Option<&str>, linea: &str) -> Command {
        // Cuál `ssh`, en un solo sitio: ver `descubrir::binario_ssh`.
        let mut cmd = Command::new(crate::descubrir::binario_ssh());
        cmd.args(self.opciones_ssh(dir_control));
        cmd.arg("--");
        cmd.arg(&self.destino);
        // Un solo argumento. OpenSSH concatena con espacios los que le sobran,
        // así que pasarle varios no separa nada — construiría una cadena y haría
        // creer que no. La cadena la construye `shquote`, escapada.
        cmd.arg(linea);
        cmd
    }
}

/// Que lanzar un proceso **no abra una consola** en Windows.
///
/// La ventana se compila con `windows_subsystem = "windows"`, y un proceso así
/// que lanza `ssh.exe` le hace a Windows crearle una consola nueva: un
/// parpadeo negro por cada saludo, cada refresco del monitor y cada alias de
/// `ssh -G`. `CREATE_NO_WINDOW` es la bandera que lo evita sin tocar nada más:
/// las tuberías siguen siendo las nuestras. Se aplica en el único sitio por el
/// que pasan todos los lanzamientos de este fichero, para que no haya uno que
/// se olvide.
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

#[derive(Debug)]
pub enum ErrorTransporte {
    /// La orden no se pudo ni construir. Es un fallo del programa, no del canal.
    Forma(ErrorForma),
    /// No se pudo lanzar `ssh`.
    NoSePudoLanzar(std::io::Error),
    /// `ssh` no llegó al servidor. Se distingue de un fallo de `orbit` a
    /// propósito: confundirlos marca un servidor entero como caído porque el
    /// usuario pidió una app que no existe.
    NoLlego {
        codigo: i32,
        stderr: String,
    },
    /// **La clave de un host conocido ha cambiado.**
    ///
    /// Tiene variante propia y no es un `NoLlego` cualquiera, porque no es un
    /// fallo de red: es exactamente lo que se ve en un ataque de suplantación.
    /// La interfaz lo trata como una pantalla bloqueante sin botón de
    /// continuar, y la única salida es editar `~/.ssh/known_hosts` a mano fuera
    /// de la aplicación. Deliberadamente incómodo: cambiar la clave de un host
    /// es raro —una reinstalación, una migración— y siempre lo sabe el usuario,
    /// mientras que un ataque de suplantación es exactamente esto.
    ///
    /// Existe porque la prueba de punta a punta lo destapó: sin ella, esto le
    /// llegaba a la interfaz como «no he llegado al servidor», que describe un
    /// problema de red y no un ataque.
    ClaveDeHostCambiada {
        detalle: String,
    },
    /// `orbit` no está en esa ruta (127) o no se puede ejecutar (126).
    OrbitNoEsta {
        ruta: String,
        stderr: String,
    },
    /// `sudo` pidió contraseña y no hay terminal donde escribirla.
    SudoPideClave,
    /// El servidor contestó, con un código distinto de 0. **No es
    /// necesariamente un fallo del transporte**: un despliegue que falla sale
    /// con 1 y con un objeto válido por stdout.
    Orbit {
        codigo: i32,
        stdout: String,
        stderr: String,
    },
    /// Se pasó del presupuesto de tamaño.
    Demasiado {
        tope: usize,
    },
    /// Se pasó del presupuesto de tiempo.
    ///
    /// `cambia` dice si la orden cambiaba algo ([`cambia_el_servidor`]). Va en
    /// el error y no en quien lo pinta porque cambia lo que es verdad: matar el
    /// `ssh` local no para la orden remota, así que de una que cambia algo no se
    /// puede decir «no ha pasado nada».
    Tarde {
        tope: Duration,
        cambia: bool,
    },
    /// Llegó algo que no es un solo objeto JSON.
    RespuestaSucia(String),
    Json(String),
}

impl std::fmt::Display for ErrorTransporte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forma(e) => write!(f, "{e}"),
            Self::NoSePudoLanzar(e) => write!(f, "no he podido lanzar ssh: {e}"),
            Self::NoLlego { stderr, .. } => write!(f, "no he llegado al servidor: {}", primera_linea(stderr)),
            Self::ClaveDeHostCambiada { .. } => write!(
                f,
                "la clave de este servidor ha cambiado. Puede ser una reinstalación, \
                 o puede ser que estés hablando con otra máquina"
            ),
            Self::OrbitNoEsta { ruta, .. } => write!(f, "no hay un orbit ejecutable en {ruta}"),
            Self::SudoPideClave => write!(
                f, "ese usuario necesita contraseña para sudo, y aquí no hay terminal donde escribirla. \
                    Desde Orbit 1.4.6 se arregla una vez, como root en el servidor: \
                    orbit admin add <usuario> (equivale a darle root, y Orbit lo dice)"),
            Self::Orbit { stderr, codigo, .. } => {
                let m = primera_linea(stderr);
                if m.is_empty() { write!(f, "orbit ha salido con {codigo}") } else { write!(f, "{m}") }
            }
            Self::Demasiado { tope } => write!(f, "la respuesta pasa de {tope} bytes y la he cortado"),
            Self::Tarde {
                tope,
                cambia: false,
            } => write!(f, "el servidor no ha contestado en {}", legible(*tope)),
            Self::Tarde { tope, cambia: true } => write!(
                f,
                "he dejado de esperar tras {}, pero la orden puede seguir en marcha en el \
                 servidor y terminar igualmente: no sé cómo ha quedado. Compruébalo antes \
                 de repetirla",
                legible(*tope)
            ),
            Self::RespuestaSucia(q) => write!(f, "por stdout no ha venido un solo objeto JSON: {q}"),
            Self::Json(e) => write!(f, "no he entendido la respuesta: {e}"),
        }
    }
}

impl std::error::Error for ErrorTransporte {}

/// Un plazo para una persona: «20 min» y no «1200s».
fn legible(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 120 {
        format!("{} min", s / 60)
    } else {
        format!("{s} s")
    }
}

fn primera_linea(s: &str) -> String {
    // El mensaje para una persona es la primera línea **con palabras**.
    //
    // La primera línea a secas no vale, y esto no es teoría: cuando la clave de
    // un host cambia, OpenSSH abre con tres líneas de arroba —el marco del
    // aviso— y la versión anterior de esta función devolvía «@@@@@@@@@@@@…».
    // O sea que la interfaz iba a enseñar un muro de arroba justo en el único
    // error del canal que el usuario tiene que leer entero. Lo cazó la prueba
    // de punta a punta contra un sshd de verdad; contra el doble local no
    // aparecía, porque un doble local no tiene claves de host.
    //
    // Y tampoco vale la última: el error más común de git trae detrás un
    // párrafo de ayuda, y quedarse con su última línea daba «and the repository
    // exists.» — un trozo de frase suelto. Es un fallo que Orbit ya cometió y
    // tuvo que arreglar dentro de su propio contrato.
    s.lines()
        .map(sin_glifo)
        .find(|l| {
            // Con contenido, y que no sea puro marco de dibujo.
            !l.is_empty() && l.chars().any(|c| c.is_alphanumeric())
        })
        .unwrap_or("")
        .to_string()
}

/// Una línea de Orbit sin su sangría ni su glifo.
///
/// Orbit escribe `  <glifo> <mensaje>` y el glifo **depende de la terminal del
/// servidor**: `✗` con UTF-8 y `x` sin él (`_ui_unicode`, la tabla `G_*`), y por
/// SSH sin terminal y con `LANG=C` —un VPS recién creado— sale el de ASCII. La
/// versión anterior sólo quitaba `✗ `, así que contra ese servidor cada error
/// empezaba por «x Faltan variables…». Se quita sólo con la sangría delante,
/// que es la forma exacta de Orbit: una frase que empiece por «x » sin ella no
/// es un glifo.
fn sin_glifo(l: &str) -> &str {
    if let Some(resto) = l.strip_prefix("  ") {
        for g in ["✗ ", "✓ ", "! ", "· ", "x ", "+ ", ". "] {
            if let Some(m) = resto.strip_prefix(g) {
                return m.trim();
            }
        }
    }
    l.trim()
}

/// Lo que llegó del otro lado, con los dos canales separados.
#[derive(Debug, Clone)]
pub struct Respuesta {
    pub stdout: String,
    /// Todo lo que el servidor le cuenta a una persona. **Nunca se parsea como
    /// datos**: con `--json`, Orbit lo manda aquí precisamente para que el
    /// objeto no se ensucie.
    pub stderr: String,
    pub codigo: i32,
    /// Que se mató a petición de quien llamaba ([`EnCurso::cancelar`]).
    ///
    /// Va aparte y no como un `codigo` especial: la versión anterior usaba
    /// `-1`, que es lo que da `code()` en Unix para un proceso muerto por
    /// señal, pero en Windows `Child::kill` deja el código 1 y un despliegue
    /// cancelado con `ok:true` salía como «el objeto y el código no casan».
    pub cancelado: bool,
}

impl Respuesta {
    /// Comprueba que por stdout haya venido **un solo objeto** y lo devuelve.
    ///
    /// No se busca la primera `{`. Es la diferencia entre un cliente y un
    /// agujero: recortar hasta la primera llave es exactamente cómo se cuela un
    /// objeto ajeno delante del legítimo.
    pub fn objeto(&self) -> Result<&str, ErrorTransporte> {
        let s = self.stdout.trim();
        if s.is_empty() {
            // Un stdout en blanco no es «no había nada»: el contrato devuelve
            // una colección vacía con `total: 0` precisamente para que se
            // puedan distinguir.
            return Err(ErrorTransporte::RespuestaSucia(
                "no ha contestado nada".into(),
            ));
        }
        if !s.starts_with('{') {
            return Err(ErrorTransporte::RespuestaSucia(
                "hay algo antes del objeto, y eso no se recorta".into(),
            ));
        }
        // Un solo documento: si detrás de lo que parsea queda algo que no sea
        // espacio, son dos objetos y eso tampoco se recorta.
        let mut de = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
        match de.next() {
            Some(Ok(_)) => {}
            Some(Err(e)) => return Err(ErrorTransporte::Json(e.to_string())),
            None => return Err(ErrorTransporte::RespuestaSucia("vacío".into())),
        }
        if s[de.byte_offset()..].trim().is_empty() {
            Ok(s)
        } else {
            Err(ErrorTransporte::RespuestaSucia(
                "han venido dos objetos".into(),
            ))
        }
    }

    /// Deserializa comprobando el `schema` antes de interpretar nada.
    pub fn leer<T: serde::de::DeserializeOwned>(&self) -> Result<T, ErrorTransporte> {
        let s = self.objeto()?;
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| ErrorTransporte::Json(e.to_string()))?;
        comprobar_schema(&v)?;
        serde_json::from_str(s).map_err(|e| ErrorTransporte::Json(e.to_string()))
    }

    /// El objeto **sin tipar**, con las mismas comprobaciones que [`leer`] y
    /// una más: que su `ok` case con el código de salida.
    ///
    /// Es el camino de la ventana, que le pasa el objeto tal cual a la interfaz
    /// —el tipado vive en el gemelo de TypeScript— y por eso se saltaba el
    /// `schema`: un servidor con `schema:2` se pintaba como si hablara el 1.
    ///
    /// Y lo del `ok`: si el objeto dice `ok:true` y la orden ha salido con error,
    /// o al revés, uno de los dos miente y no sabemos cuál. Fiarse del objeto
    /// pinta «desplegada» un despliegue que falló; fiarse del código, al revés.
    /// Un proceso cancelado no tiene código que comparar: lo ha puesto quien lo
    /// mató, no la orden.
    ///
    /// [`leer`]: Respuesta::leer
    pub fn objeto_del_contrato(&self, c: &Comando) -> Result<serde_json::Value, ErrorTransporte> {
        let s = self.objeto()?;
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| ErrorTransporte::Json(e.to_string()))?;
        comprobar_schema(&v)?;
        if ok_es_el_desenlace(c) && !self.cancelado {
            match v.get("ok") {
                // Si falta, lo dirá quien lo lea: aquí sólo se compara.
                None => {}
                Some(serde_json::Value::Bool(ok)) => {
                    if *ok != (self.codigo == 0) {
                        return Err(ErrorTransporte::Json(format!(
                            "el objeto dice «ok: {ok}» y orbit ha salido con {}: no me fío \
                             de ninguno de los dos",
                            self.codigo
                        )));
                    }
                }
                Some(otro) => {
                    return Err(ErrorTransporte::Json(format!(
                        "«ok» tendría que ser true o false y es {otro}"
                    )))
                }
            }
        }
        Ok(v)
    }
}

/// Que el objeto hable **el** `schema` que entendemos, y que lo diga.
///
/// Ausente o de otro tipo es un error, no «será el 1»: todo objeto de primer
/// nivel del contrato lo lleva (orbit, `JSON_SCHEMA`), y los tipos de
/// `contrato.rs` lo exigen igual. Sólo el camino sin tipar lo dejaba pasar.
fn comprobar_schema(v: &serde_json::Value) -> Result<(), ErrorTransporte> {
    let conocido = crate::contrato::CONTRATO_CONOCIDO as u64;
    match v.get("schema") {
        Some(x) => match x.as_u64() {
            Some(sc) if sc == conocido => Ok(()),
            Some(sc) => Err(ErrorTransporte::Json(format!(
                "el servidor habla el schema {sc} y este cliente entiende el {conocido}"
            ))),
            None => Err(ErrorTransporte::Json(format!(
                "«schema» tendría que ser un entero y es {x}"
            ))),
        },
        None => Err(ErrorTransporte::Json(
            "la respuesta no dice qué schema habla".into(),
        )),
    }
}

/// Ejecuta una orden. Es la **única** función del crate que lanza un proceso.
/// `entorno` se le pone al proceso `ssh` **local**. Si cruza o no al otro lado
/// depende de que el servidor tenga `AcceptEnv` y nosotros `SendEnv`, así que
/// **el cliente no se apoya en él para nada**: para el idioma existe
/// `orbit --lang`, que viaja como argumento y no depende de la configuración
/// del servidor. Está aquí porque el banco de pruebas lo necesita para pedirle
/// un caso concreto al servidor falso.
pub fn ejecutar(
    servidor: &Servidor,
    comando: &Comando,
    dir_control: Option<&str>,
    entorno: &[(&str, &str)],
) -> Result<Respuesta, ErrorTransporte> {
    let argv = comando
        .argv(&servidor.binario)
        .map_err(ErrorTransporte::Forma)?;
    let linea = crate::shquote::build(&argv)
        .map_err(|e| ErrorTransporte::Forma(ErrorForma::Escapado(e)))?;

    let mut cmd = servidor.orden_ssh(dir_control, &linea);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_proceso(
        cmd,
        tope_de_tiempo(comando),
        &servidor.binario,
        comando.entrada(),
        Origen::de(comando),
        cambia_el_servidor(comando),
        &Nuestro::de(servidor),
    )
}

/// El mismo camino sin `ssh`, para las pruebas: se ejecuta el binario en local.
///
/// Existe porque el 90 % de los casos patológicos —el parser, los `null`, los
/// seis finales— no necesitan un shell remoto, y probarlos en milisegundos en
/// vez de en segundos es la diferencia entre una suite que se ejecuta y una que
/// no. Lo que **no** cubre es el escapado: eso sólo se prueba contra un shell de
/// verdad, y de eso se encarga `tests/escapado.rs`.
/// `entorno` se pasa **por invocación** y no por variables del proceso. No es un
/// detalle de estilo: las pruebas corren en hilos dentro del mismo proceso, y
/// una variable de entorno es global, así que dos pruebas concurrentes se pisan
/// el caso la una a la otra. Cuesta un fallo intermitente que además parece un
/// fallo del cliente.
///
/// Y por eso el camino de `ssh` **no** tiene este parámetro: un entorno no cruza
/// una sesión SSH por defecto, así que ofrecerlo ahí sería prometer algo que no
/// pasa. Lo que sí cruza es un argumento, y para el idioma existe `orbit --lang`.
pub fn ejecutar_local(
    binario: &str,
    comando: &Comando,
    entorno: &[(&str, &str)],
) -> Result<Respuesta, ErrorTransporte> {
    let argv = comando.argv(binario).map_err(ErrorTransporte::Forma)?;
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_proceso(
        cmd,
        tope_de_tiempo(comando),
        binario,
        comando.entrada(),
        Origen::de(comando),
        cambia_el_servidor(comando),
        &Nuestro::nadie(),
    )
}

/// Un despliegue en curso, para poder cancelarlo.
///
/// Cancelar **mata el proceso remoto**, y lo que eso deja detrás depende del
/// paso en el que estuviera. Por eso el que llama tiene que saber cuál era: no
/// es lo mismo interrumpir un build —la release nueva se descarta y `current`
/// ni se ha movido, así que no queda nada roto— que interrumpir `service` o
/// `nginx`, donde sí puede quedar trabajo a medias.
///
/// El modelo de despliegues de Orbit ayuda: todo lo destructivo ocurre lo más
/// tarde posible y el symlink se mueve al final. Pero eso no convierte la
/// cancelación en gratis, y decir que lo es sería mentir.
/// **Lo crea quien llama, ANTES de llamar.** La primera versión lo devolvía la
/// función al terminar, y eso lo hacía inútil: para cuando lo tenías, no había
/// nada que cancelar. Lo destapó escribir la prueba, no revisar el diseño.
#[derive(Clone, Default)]
pub struct EnCurso {
    cancelar: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl EnCurso {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Pide que el proceso muera. Lo que deja detrás depende del paso en curso,
    /// y eso lo sabe quien mira la pantalla, no esto.
    pub fn cancelar(&self) {
        self.cancelar
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn cancelado(&self) -> bool {
        self.cancelar.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Si dos mandos son **el mismo** despliegue, no dos iguales.
    ///
    /// Lo necesita quien los registra: al terminar, un despliegue sólo puede
    /// quitar del registro su propio mando. Quitar «el que haya con esa clave»
    /// era quitar el de otro, que se quedaba sin forma de cancelarse.
    pub fn es_el_mismo(&self, otro: &EnCurso) -> bool {
        std::sync::Arc::ptr_eq(&self.cancelar, &otro.cancelar)
    }
}

/// Ejecuta una orden **sirviendo el progreso mientras ocurre**.
///
/// Existe porque `deploy --progress` emite un suceso por línea por **stderr**
/// mientras stdout espera al objeto final, y leerlo entero al terminar
/// convertiría tres minutos de información en un bloque de texto que llega
/// cuando ya no sirve.
///
/// `al_llegar` recibe cada línea de stderr **según llega**, no al final. Se
/// llama desde el hilo que lee stderr, así que tiene que ser barata: lo que
/// haga cosas caras que lo encole.
///
/// Lo que este camino **no** hace, y es lo importante:
///
///  · **No decide que un despliegue ha fallado por perder el contacto.** Si la
///    conexión se corta, el despliegue **sigue en el servidor** y el cliente ya
///    no sabe qué pasó. Devolver «falló» sería afirmar algo que no se sabe. El
///    error dice que se perdió el contacto, que es incómodo y verdadero.
///  · **No reintenta solo.** Un `deploy` reintentado sobre uno en curso es, en
///    el mejor caso, dos releases; en el peor, dos builds peleándose por la
///    caché de git.
pub fn ejecutar_en_vivo(
    servidor: &Servidor,
    comando: &Comando,
    dir_control: Option<&str>,
    entorno: &[(&str, &str)],
    en_curso: EnCurso,
    al_llegar: impl FnMut(String) + Send + 'static,
) -> Result<Respuesta, ErrorTransporte> {
    let argv = comando
        .argv(&servidor.binario)
        .map_err(ErrorTransporte::Forma)?;
    let linea = crate::shquote::build(&argv)
        .map_err(|e| ErrorTransporte::Forma(ErrorForma::Escapado(e)))?;

    let mut cmd = servidor.orden_ssh(dir_control, &linea);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_sirviendo(
        cmd,
        tope_de_tiempo(comando),
        &servidor.binario,
        comando.vena_humana(),
        comando.entrada(),
        en_curso,
        al_llegar,
        Origen::de(comando),
        cambia_el_servidor(comando),
        &Nuestro::de(servidor),
    )
}

/// Ejecuta una línea **que no es del catálogo**, sirviendo su salida.
///
/// # Esto es la excepción, y se llama así para que no se confunda
///
/// Todo el resto del transporte pasa por [`Comando`], que es un enumerado
/// cerrado: **la superficie de lo que este cliente le puede pedir a un servidor
/// cabe en una pantalla y se revisa de un vistazo**. Esa propiedad es la que
/// hace revisable el proyecto, y esta función la rompe.
///
/// Existe para **una sola cosa**: instalar Orbit en un servidor que todavía no
/// lo tiene. Ahí no hay ningún `orbit` que invocar —ése es justo el problema que
/// se va a resolver— así que el catálogo no puede expresarlo.
///
/// Tres cosas que la hacen aceptable, y que si algún día dejan de cumplirse
/// convierten esto en la puerta trasera que el proyecto no quiere:
///
/// 1. **La línea la construye [`crate::instalar`]**, no quien llama. No hay
///    ninguna forma de pasarle texto que venga de un campo de la interfaz.
/// 2. **Se enseña entera antes de ejecutarla.** Es la misma regla que el repaso
///    del asistente de web nueva.
/// 3. **No se usa para leer nada.** Lo que este cliente sabe de un servidor sale
///    del contrato y sólo del contrato. El día que alguien tape un hueco del
///    contrato con esto, el cliente deja de hablar el contrato y pasa a hablar
///    Bash contra un servidor cuyo layout puede cambiar.
///
/// La prueba de arquitectura comprueba que sólo la llama el instalador.
pub fn ejecutar_crudo_en_vivo(
    servidor: &Servidor,
    linea: &str,
    dir_control: Option<&str>,
    en_curso: EnCurso,
    al_llegar: impl FnMut(String) + Send + 'static,
) -> Result<Respuesta, ErrorTransporte> {
    let cmd = servidor.orden_ssh(dir_control, linea);
    // Sin plazo: instalar tarda entre cinco y diez minutos, y un tope corto
    // mataría el proceso a mitad de un `apt` — que es exactamente el momento en
    // que peor está un servidor.
    //
    // Y `Vena::Stdout`, porque aquí no hay `--json`: lo que dice el instalador
    // es prosa para una persona y sale por donde sale la prosa.
    ejecutar_sirviendo(
        cmd,
        Duration::MAX,
        &servidor.binario,
        Vena::Stdout,
        None,
        en_curso,
        al_llegar,
        // El instalador habla como Orbit a efectos de `sudo`: si no puede
        // elevarse, decirlo es el diagnóstico correcto. Y un «command not found»
        // de cualquier paso ya no se confunde con «no hay orbit», porque
        // `clasificar` sólo lo dice cuando el shell nombra al binario.
        Origen::Orbit,
        // Sin plazo no hay `Tarde`, pero si lo hubiera: instalar cambia todo.
        true,
        &Nuestro::nadie(),
    )
}

/// Lo mismo sin servir nada, para preguntas cortas.
///
/// Misma excepción y mismas tres condiciones que [`ejecutar_crudo_en_vivo`]. Se
/// usa para la comprobación de requisitos, que es una línea fija de
/// [`crate::instalar::COMPROBACION`].
pub fn ejecutar_crudo(
    servidor: &Servidor,
    linea: &str,
    dir_control: Option<&str>,
) -> Result<Respuesta, ErrorTransporte> {
    let cmd = servidor.orden_ssh(dir_control, linea);
    // Mismo motor que el resto, pero **sin clasificar**: ver abajo.
    let r = match ejecutar_proceso(
        cmd,
        Duration::from_secs(30),
        &servidor.binario,
        None,
        // Aquí no habla `orbit` —puede que ni exista—, así que no se le
        // atribuye ningún aviso de stderr: ver `Origen`.
        Origen::Ajena,
        // La comprobación de requisitos sólo lee.
        false,
        // Su 255 lo decide abajo, sin mirar a quién nombra: aquí no hay app
        // que pueda lanzar un `ssh` propio.
        &Nuestro::nadie(),
    ) {
        Ok(r) => r,
        // `clasificar` traduce los códigos de `orbit`, y aquí puede que ni
        // exista: un 127 «command not found» es lo NORMAL en un servidor sin
        // instalar, no un fallo. Se recupera la respuesta y se sigue.
        Err(ErrorTransporte::OrbitNoEsta { stderr, .. }) => Respuesta {
            stdout: String::new(),
            stderr,
            codigo: 127,
            cancelado: false,
        },
        Err(ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => Respuesta {
            stdout,
            stderr,
            codigo,
            cancelado: false,
        },
        Err(e) => return Err(e),
    };
    // A propósito NO pasa por `clasificar`: esa función traduce los códigos de
    // `orbit`, y aquí no hay ningún `orbit` — puede que ni exista todavía. Lo
    // único que se reconoce es no haber llegado, que lo dice `ssh` y no el otro
    // lado.
    if r.codigo == 255 {
        let e = r.stderr.to_lowercase();
        if e.contains("remote host identification has changed")
            || e.contains("host key verification failed")
        {
            return Err(ErrorTransporte::ClaveDeHostCambiada { detalle: r.stderr });
        }
        return Err(ErrorTransporte::NoLlego {
            codigo: 255,
            stderr: r.stderr,
        });
    }
    Ok(r)
}

/// Dónde viven los sockets de `ControlMaster`.
///
/// Vive en el núcleo y no en el envoltorio porque **es política del transporte**:
/// quien decide multiplexar es esto, y el directorio es parte de esa decisión.
/// Estaba en la aplicación de escritorio, y ahí sólo servía mientras hubiera una
/// sola interfaz.
///
/// Que la ventana y el terminal compartan directorio **es lo que se quiere**: el
/// segundo reutiliza la conexión que ya abrió el primero, que es la diferencia
/// medida entre 246 ms y 13 ms de saludo.
///
/// `0700`, y sin plan B si no se puede: un socket de control es una sesión SSH
/// abierta con las credenciales de quien la abrió, así que un directorio que no
/// se puede cerrar es un directorio en el que no se pone nada. Devolver `None`
/// significa «sin multiplexar», que es más lento y correcto.
///
/// **Dónde**, por orden: `$XDG_RUNTIME_DIR/orbit-desktop` —el sitio que Linux
/// tiene para esto, en memoria y sólo del usuario— y, si no existe,
/// `~/.ssh/orbit-desktop`. Lo segundo es por macOS, que no define
/// `XDG_RUNTIME_DIR`: la primera versión devolvía `None` allí, o sea que **en
/// un Mac cada orden pagaba el saludo entero**, los 246 ms en vez de 13. No se
/// usa `$TMPDIR`: en macOS es una ruta de unos cincuenta caracteres, y con el
/// hash de `%C` y el sufijo temporal que `ssh` añade al crear el socket se pasa
/// de los 104 bytes que admite una ruta de socket.
///
/// En Windows no se multiplexa: el OpenSSH de Windows no tiene `ControlMaster`.
///
/// Y la carpeta **se comprueba, no sólo se crea**: que sea una carpeta y no un
/// enlace, que sea del mismo dueño que la carpeta donde vive, y `0700`. Un
/// enlace puesto ahí por otro haría que nuestras sesiones se abrieran en un
/// sitio que no es nuestro.
pub fn dir_control() -> Option<String> {
    #[cfg(unix)]
    {
        let (base, nombre) = match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(b) if !b.is_empty() => (std::path::PathBuf::from(b), "orbit-desktop"),
            _ => (
                std::path::PathBuf::from(std::env::var_os("HOME")?).join(".ssh"),
                "orbit-desktop",
            ),
        };
        dir_privado(&base, nombre)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(unix)]
fn dir_privado(base: &std::path::Path, nombre: &str) -> Option<String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let dueno = std::fs::metadata(base).ok()?.uid();
    let d = base.join(nombre);
    match std::fs::DirBuilder::new().mode(0o700).create(&d) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return None,
    }
    // `symlink_metadata`, que no sigue el enlace: es justo lo que se mira.
    let md = std::fs::symlink_metadata(&d).ok()?;
    if !md.file_type().is_dir() || md.uid() != dueno {
        return None;
    }
    if md.mode() & 0o777 != 0o700 {
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700)).ok()?;
    }
    Some(d.to_string_lossy().into_owned())
}

/// El mismo camino sin `ssh`, para las pruebas.
pub fn ejecutar_en_vivo_local(
    binario: &str,
    comando: &Comando,
    entorno: &[(&str, &str)],
    en_curso: EnCurso,
    al_llegar: impl FnMut(String) + Send + 'static,
) -> Result<Respuesta, ErrorTransporte> {
    let argv = comando.argv(binario).map_err(ErrorTransporte::Forma)?;
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_sirviendo(
        cmd,
        tope_de_tiempo(comando),
        binario,
        comando.vena_humana(),
        comando.entrada(),
        en_curso,
        al_llegar,
        Origen::de(comando),
        cambia_el_servidor(comando),
        &Nuestro::nadie(),
    )
}

/// **De quién** es la salida que se va a clasificar.
///
/// Hace falta porque [`clasificar`] reconoce varios fallos por lo que dice
/// stderr —el aviso de `sudo`, un «command not found»— y eso sólo es fiable
/// cuando quien habla es `orbit`. Con `exec` habla un proceso cualquiera de la
/// app: un `sudo` dentro de un script de mantenimiento salía como «ese usuario
/// necesita contraseña para sudo», y una errata en el nombre de una orden
/// (127) como «no hay un orbit ejecutable». La pantalla de exec enseñaba un
/// diagnóstico falso del servidor en vez de la salida del comando, que era lo
/// único que alguien había pedido ver.
///
/// Lo que se pierde, y se acepta: si es el propio `orbit` el que no consigue
/// elevarse durante un `exec`, eso llega como salida cruda con el aviso de
/// `sudo` dentro, en vez de como la pantalla que explica `orbit admin add`. Es
/// el mismo texto, y antes de llegar a `exec` la portada ya habría dado ese
/// diagnóstico, porque falla igual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origen {
    /// Habla `orbit`, o su instalador: los avisos de stderr son suyos.
    Orbit,
    /// Habla un proceso arbitrario dentro de una app. Sólo se reconoce lo que
    /// dice `ssh` y lo que nombra al binario de Orbit por su ruta.
    Ajena,
}

impl Origen {
    fn de(c: &Comando) -> Self {
        match c {
            Comando::Ejecutar { .. } => Origen::Ajena,
            _ => Origen::Orbit,
        }
    }
}

/// Cada cuánto se mira si alguien ha pedido cancelar. Sólo existe en el camino
/// en vivo: es una bandera, no un suceso, y alguien tiene que mirarla. 50 ms es
/// menos de lo que tarda una persona en ver que el botón ha respondido, y no le
/// cuesta nada a una orden que va a durar minutos.
const LATIDO_DE_CANCELACION: Duration = Duration::from_millis(50);

/// Lo largo que puede ser una línea servida antes de entregarla a trozos.
///
/// Un evento de `--progress` son unos cientos de bytes; esto es para que un
/// servidor que mande gigas sin un salto de línea no nos llene la memoria
/// esperando a que acabe.
const TOPE_LINEA: usize = 1024 * 1024;

#[allow(clippy::too_many_arguments)]
fn ejecutar_sirviendo(
    mut cmd: Command,
    tope: Duration,
    binario: &str,
    vena: Vena,
    entrada: Option<Secreto>,
    en_curso: EnCurso,
    mut al_llegar: impl FnMut(String) + Send + 'static,
    origen: Origen,
    cambia: bool,
    nuestro: &Nuestro,
) -> Result<Respuesta, ErrorTransporte> {
    use std::sync::Arc;

    cmd.stdin(if entrada.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    sin_ventana(&mut cmd);
    let mut hijo = cmd.spawn().map_err(ErrorTransporte::NoSePudoLanzar)?;
    let h_entrada = escribir_entrada(&mut hijo, entrada);

    let so: Box<dyn std::io::Read + Send> = Box::new(hijo.stdout.take().unwrap());
    let se: Box<dyn std::io::Read + Send> = Box::new(hijo.stderr.take().unwrap());

    // Cuál se sirve a trozos y cuál se acumula lo dice la orden, no la llamada:
    // con `--json` la prosa va por stderr y stdout guarda el objeto, que no se
    // puede partir; sin `--json` es al revés y no hay objeto que proteger.
    //
    // Lo que NO se intercambia es dónde acaba cada una en la `Respuesta`:
    // `stdout` sigue siendo stdout. Los mensajes por los que se reconoce un
    // «command not found» o un cambio de clave de host los escribe ssh, no
    // Orbit, y salen por stderr pase lo que pase — clasificarlos leyendo «la
    // tubería servida» sería dejar de reconocerlos justo en la orden más larga.
    let (servida, guardada) = match vena {
        Vena::Stderr => (se, so),
        Vena::Stdout => (so, se),
    };

    // Cada lector se lleva un extremo del canal y lo suelta al terminar. Nadie
    // manda nada por él: que se desconecte ES el aviso de que las dos tuberías
    // han llegado al final, y así la espera se despierta en ese instante en vez
    // de en el siguiente sondeo.
    let (fin_tx, fin_rx) = std::sync::mpsc::channel::<()>();
    let tx = fin_tx.clone();
    let h_guardada = std::thread::spawn(move || {
        let _fin = tx;
        leer_hasta(guardada, TOPE_RESPUESTA)
    });

    // La otra se sirve línea a línea. Los dos hilos siguen siendo dos: leerlos
    // en serie es un bloqueo mutuo en cuanto el que no se lee llena su tubería.
    // El texto, y cuántos bytes se han tirado por pasar del tope.
    let acumulado = Arc::new(std::sync::Mutex::new((String::new(), 0u64)));
    let acc = Arc::clone(&acumulado);
    let h_servida = std::thread::spawn(move || {
        let _fin = fin_tx;
        servir_lineas(servida, |l| {
            if let Ok(mut a) = acc.lock() {
                // El presupuesto también aplica aquí: un servidor que gotee
                // para siempre no puede llenarnos la memoria. Lo que sobra se
                // cuenta, para poder decir que falta; la línea se sigue
                // sirviendo, porque el tope es de lo guardado, no del progreso.
                if a.0.len() < TOPE_RESPUESTA {
                    a.0.push_str(&l);
                    a.0.push('\n');
                } else {
                    a.1 += l.len() as u64 + 1;
                }
            }
            al_llegar(l);
        });
    });

    let (estado, cancelado) = match esperar(&mut hijo, &fin_rx, tope, Some(&en_curso), cambia)? {
        Fin::Salio(e) => (e, false),
        Fin::Cancelado(e) => (e, true),
    };

    let (guardada, sobra) = h_guardada.join().unwrap()?;
    let _ = h_servida.join();
    if let Some(h) = h_entrada {
        let _ = h.join();
    }
    // Lo guardado es el objeto sólo con `--json` y hablando `orbit`; si no, es
    // prosa, y la prosa cortada se enseña cortada y diciéndolo.
    let guardada = recortada(
        guardada,
        sobra,
        vena == Vena::Stderr && origen == Origen::Orbit,
    )?;
    let servida = acumulado
        .lock()
        .map(|a| con_marca_de_corte(a.0.clone(), a.1))
        .unwrap_or_default();
    // Y aquí se deshace el intercambio: cada una vuelve a su sitio.
    let (stdout, stderr) = match vena {
        Vena::Stderr => (guardada, servida),
        Vena::Stdout => (servida, guardada),
    };
    let codigo = estado.code().unwrap_or(-1);

    let r = Respuesta {
        stdout,
        stderr,
        codigo,
        cancelado,
    };
    if cancelado {
        // Cancelado NO es fallido, y no se puede clasificar como tal: el
        // proceso remoto puede haber terminado su paso antes de morir, y decir
        // «ha fallado» sería afirmar algo que no se sabe.
        return Ok(r);
    }
    clasificar(r, binario, origen, nuestro)
}

/// Lee una tubería línea a línea y entrega cada una **según llega**.
///
/// Con `read_until` y no con `lines()`, y la diferencia la pagaba quien
/// desplegaba: `lines()` devuelve un error en cuanto llega un byte que no es
/// UTF-8 —basta una herramienta de build que escriba en Latin-1— y la versión
/// anterior salía del bucle con ese error. Dejar de leer cierra la tubería: el
/// progreso se congelaba el resto del despliegue y el stderr final llegaba
/// cortado. Ahora un byte raro se convierte en `�` y se sigue, que es la regla
/// del módulo: una línea rota no aborta el flujo.
///
/// El final de línea se quita igual que lo quitaba `lines()`: el `\n` y, si va
/// delante, el `\r`.
fn servir_lineas(fuente: impl Read, mut al_llegar: impl FnMut(String)) {
    use std::io::BufRead;
    let mut lector = std::io::BufReader::new(fuente);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        // El tope por línea: lo que lo pase se entrega en trozos.
        match (&mut lector)
            .take(TOPE_LINEA as u64)
            .read_until(b'\n', &mut buf)
        {
            Ok(0) => break,
            Ok(_) => {}
            // `read_until` ya reintenta lo interrumpido; cualquier otro error
            // es que la tubería se ha ido, y entonces no hay más que leer.
            Err(_) => break,
        }
        let mut fin = buf.len();
        if buf[..fin].ends_with(b"\n") {
            fin -= 1;
            if buf[..fin].ends_with(b"\r") {
                fin -= 1;
            }
        }
        al_llegar(String::from_utf8_lossy(&buf[..fin]).into_owned());
    }
}

/// Cómo terminó la espera de un proceso que no se pasó de plazo.
enum Fin {
    Salio(std::process::ExitStatus),
    Cancelado(std::process::ExitStatus),
}

/// Espera a que el proceso termine, **bloqueando y no sondeando**, sin pasarse
/// del plazo.
///
/// La versión anterior sondeaba con `try_wait` cada 10 ms (20 ms en vivo), y eso
/// no era gratis: a una llamada multiplexada de 13 ms le sumaba hasta 10 más,
/// casi el doble, y despertaba cien veces por segundo por cada petición en
/// vuelo. Se sondeaba porque `wait` no sabe de plazos; lo que sí sabe de plazos
/// es `recv_timeout`, y lo que avisa de que el proceso ha terminado son sus
/// tuberías al cerrarse. Así que se espera al canal que sueltan los lectores.
///
/// Se descartó el hilo vigía que hace `wait` y avisa: para poder matar al
/// proceso hace falta el `Child`, y `Child::kill` y `Child::wait` lo piden los
/// dos en exclusiva. Se habría acabado con un `Mutex` alrededor y otra vez un
/// sondeo dentro.
fn esperar(
    hijo: &mut std::process::Child,
    fin: &std::sync::mpsc::Receiver<()>,
    tope: Duration,
    en_curso: Option<&EnCurso>,
    cambia: bool,
) -> Result<Fin, ErrorTransporte> {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Instant;

    // `None` es «sin plazo»: `Duration::MAX` no cabe sumado a un instante.
    let limite = Instant::now().checked_add(tope);
    let latido = en_curso.map(|_| LATIDO_DE_CANCELACION);

    // Lo que se comprueba cada vez que la espera se despierta sin noticias.
    let vigilar = |hijo: &mut std::process::Child| -> Result<Option<Fin>, ErrorTransporte> {
        if en_curso.is_some_and(EnCurso::cancelado) {
            let _ = hijo.kill();
            let e = hijo.wait().map_err(ErrorTransporte::NoSePudoLanzar)?;
            return Ok(Some(Fin::Cancelado(e)));
        }
        if limite.is_some_and(|l| Instant::now() >= l) {
            let _ = hijo.kill();
            let _ = hijo.wait();
            return Err(ErrorTransporte::Tarde { tope, cambia });
        }
        Ok(None)
    };

    loop {
        let queda = limite.map(|l| l.saturating_duration_since(Instant::now()));
        let espera = match (queda, latido) {
            (Some(q), Some(l)) => Some(q.min(l)),
            (q, l) => q.or(l),
        };
        let r = match espera {
            Some(d) => fin.recv_timeout(d),
            None => fin.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match r {
            Err(RecvTimeoutError::Disconnected) => break,
            Ok(()) => continue,
            Err(RecvTimeoutError::Timeout) => {
                if let Some(f) = vigilar(hijo)? {
                    return Ok(f);
                }
            }
        }
    }

    // Las dos tuberías cerradas casi siempre quieren decir que el proceso ha
    // terminado, pero no en el mismo instante: el cierre se ve un poco antes de
    // que el proceso se pueda recoger. Y un proceso puede cerrar sus tuberías y
    // seguir vivo. Por eso aquí sí hay un sondeo, que en el caso normal da una
    // vuelta o ninguna, y que no se salta ni el plazo ni una cancelación.
    let mut pausa = Duration::from_millis(1);
    loop {
        if let Some(e) = hijo.try_wait().map_err(ErrorTransporte::NoSePudoLanzar)? {
            return Ok(Fin::Salio(e));
        }
        if let Some(f) = vigilar(hijo)? {
            return Ok(f);
        }
        std::thread::sleep(pausa);
        pausa = (pausa * 2).min(Duration::from_millis(20));
    }
}

fn ejecutar_proceso(
    mut cmd: Command,
    tope: Duration,
    binario: &str,
    entrada: Option<Secreto>,
    origen: Origen,
    cambia: bool,
    nuestro: &Nuestro,
) -> Result<Respuesta, ErrorTransporte> {
    // Sin entrada, `null`: nadie va a contestar a un selector.
    cmd.stdin(if entrada.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    })
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    sin_ventana(&mut cmd);

    let mut hijo = cmd.spawn().map_err(ErrorTransporte::NoSePudoLanzar)?;
    let h_entrada = escribir_entrada(&mut hijo, entrada);

    // Los dos descriptores, a la vez y en hilos. Leerlos en serie es un bloqueo
    // mutuo en cuanto el que no se lee llena su tubería, y con --json el Orbit
    // real escribe por stderr **todo** lo que le cuenta a una persona.
    let so = hijo.stdout.take().unwrap();
    let se = hijo.stderr.take().unwrap();
    let (fin_tx, fin_rx) = std::sync::mpsc::channel::<()>();
    let tx = fin_tx.clone();
    let h1 = std::thread::spawn(move || {
        let _fin = tx;
        leer_hasta(so, TOPE_RESPUESTA)
    });
    let h2 = std::thread::spawn(move || {
        let _fin = fin_tx;
        leer_hasta(se, TOPE_RESPUESTA)
    });

    // Espera con tope: un servidor que gotea un byte por minuto no puede
    // tenernos aquí para siempre. Aquí no se cancela nada, así que no hay
    // latido: una sola espera hasta que se cierren las tuberías o venza el plazo.
    let estado = match esperar(&mut hijo, &fin_rx, tope, None, cambia)? {
        Fin::Salio(e) | Fin::Cancelado(e) => e,
    };

    let (stdout, sobra_o) = h1.join().unwrap()?;
    let (stderr, sobra_e) = h2.join().unwrap()?;
    if let Some(h) = h_entrada {
        let _ = h.join();
    }
    // Con `exec` stdout es la salida de un proceso cualquiera, no un objeto:
    // cortada vale más que nada. stderr nunca es el objeto.
    let stdout = recortada(stdout, sobra_o, origen == Origen::Orbit)?;
    let stderr = con_marca_de_corte(stderr, sobra_e);

    let codigo = estado.code().unwrap_or(-1);
    clasificar(
        Respuesta {
            stdout,
            stderr,
            codigo,
            cancelado: false,
        },
        binario,
        origen,
        nuestro,
    )
}

/// Si stderr está hablando **del binario de Orbit**, y no de otra cosa.
///
/// Un 127 a secas no basta: es el código de cualquier orden que no existe, y
/// con `exec` la orden es del usuario. Lo que distingue «no hay orbit» de «no
/// hay pnpm» es que el shell nombra lo que no ha encontrado: `bash:
/// /usr/local/bin/orbit: No such file or directory`, `bash: orbit: command not
/// found`, `sh: 1: /usr/local/bin/orbit: not found`.
pub(crate) fn nombra_al_binario(stderr_minusculas: &str, binario: &str) -> bool {
    let b = binario.to_lowercase();
    if stderr_minusculas.contains(&b) {
        return true;
    }
    let base = b.rsplit(['/', '\\']).next().unwrap_or(&b);
    !base.is_empty()
        && [
            "command not found",
            "not found",
            "no such file",
            "permission denied",
        ]
        .iter()
        .any(|m| stderr_minusculas.contains(&format!("{base}: {m}")))
}

/// Si stderr trae un mensaje **del cliente `ssh`** y no del proceso remoto.
///
/// Por línea y por cómo empieza, no por si aparece en cualquier sitio: «Connection
/// timed out» suelto es también lo que dice un PHP que no llega a su base de
/// datos (`SQLSTATE[HY000] [2002] Connection timed out`). El de `ssh` va
/// detrás de `ssh: connect to host …`, y eso es lo que se mira.
fn habla_el_cliente_ssh(stderr_minusculas: &str) -> bool {
    const AL_EMPEZAR: [&str; 9] = [
        "ssh:",
        "kex_exchange_identification",
        "connection closed by",
        "banner exchange",
        "timeout, server",
        "client_loop:",
        "received disconnect from",
        "bad owner or permissions",
        "stdio forwarding failed",
    ];
    const EN_CUALQUIER_SITIO: [&str; 4] = [
        "permission denied (publickey",
        "could not resolve hostname",
        "host key verification failed",
        "remote host identification has changed",
    ];
    stderr_minusculas.lines().any(|l| {
        let l = l.trim_start();
        AL_EMPEZAR.iter().any(|p| l.starts_with(p))
            || EN_CUALQUIER_SITIO.iter().any(|m| l.contains(m))
            || (l.starts_with("connection to ") && l.contains("closed by remote host"))
    })
}

/// Por qué nombres puede llamar `ssh` a **nuestro** destino en sus mensajes.
///
/// Hace falta para un 255 de `exec`: un `ssh`, `scp` o `rsync` que la app lanza
/// por su cuenta escribe exactamente lo que escribiría el nuestro —«connect to
/// host …», el aviso de clave cambiada— y sale con 255. La interfaz tiraba su
/// salida y decía que nuestro servidor no contestaba, o enseñaba la pantalla
/// de suplantación. Lo único que los distingue es de quién habla el mensaje.
struct Nuestro<'a> {
    /// Lo que se sabe sin preguntar: el alias y el host del destino.
    nombres: Vec<String>,
    /// El `known_hosts` de este ordenador, normalizado (ver [`ruta_normal`]).
    known_hosts: Option<String>,
    /// Para preguntarle a `ssh -G` el `HostName`, el `HostKeyAlias` y los
    /// saltos. Sólo se pregunta si un mensaje nombra a alguien que no está en
    /// `nombres`: es un proceso más, y en el caso normal no hace falta.
    servidor: Option<&'a Servidor>,
}

impl<'a> Nuestro<'a> {
    /// Sin destino (el camino local de las pruebas, la instalación): todo
    /// nombre es de otro.
    fn nadie() -> Self {
        Self {
            nombres: Vec::new(),
            known_hosts: None,
            servidor: None,
        }
    }

    fn de(s: &'a Servidor) -> Self {
        let known_hosts = crate::descubrir::ruta_por_defecto()
            .and_then(|c| c.parent().map(|d| d.join("known_hosts")))
            .map(|k| ruta_normal(&k.to_string_lossy()));
        Self {
            nombres: vec![nombre_limpio(&s.alias), nombre_limpio(&s.destino)],
            known_hosts,
            servidor: Some(s),
        }
    }
}

/// Un host tal como lo escribe `ssh`, comparable: sin `usuario@`, sin los
/// corchetes de `[host]:puerto` y sin la puntuación que lo sigue en la frase.
fn nombre_limpio(n: &str) -> String {
    let n = n.trim().trim_end_matches(['.', ',', ':']).to_lowercase();
    let n = n.rsplit('@').next().unwrap_or("").to_string();
    match n.strip_prefix('[').and_then(|r| r.split_once(']')) {
        Some((dentro, _)) => dentro.to_string(),
        None => n,
    }
}

/// Las barras de Windows, las dobles y las mayúsculas no cuentan: el OpenSSH de
/// Windows escribe `C:\\Users\\x/.ssh/known_hosts`, con las barras duplicadas y
/// mezcladas.
fn ruta_normal(r: &str) -> String {
    let mut fuera = String::with_capacity(r.len());
    for c in r.trim().chars() {
        let c = if c == '\\' { '/' } else { c };
        if !(c == '/' && fuera.ends_with('/')) {
            fuera.extend(c.to_lowercase());
        }
    }
    fuera
}

fn es_ip(n: &str) -> bool {
    n.parse::<std::net::IpAddr>().is_ok()
}

/// Los hosts que nombran los mensajes de `ssh` de stderr (ya en minúsculas).
fn hosts_nombrados(e: &str) -> Vec<String> {
    // Lo que va entre cada pareja es un host, en los mensajes de OpenSSH.
    const ENTRE: [(&str, &str); 10] = [
        ("connect to host ", " port"),
        ("could not resolve hostname ", ":"),
        ("host key for ", " has changed"),
        ("connection closed by ", " port"),
        ("connection reset by ", " port"),
        ("connection to ", " closed"),
        ("connection to ", " port"),
        ("timeout, server ", " not responding"),
        ("received disconnect from ", " port"),
        ("disconnected from ", " port"),
    ];
    let mut fuera = Vec::new();
    for l in e.lines() {
        let mut candidatos: Vec<&str> = ENTRE
            .iter()
            .filter_map(|(antes, despues)| {
                let resto = &l[l.find(antes)? + antes.len()..];
                Some(&resto[..resto.find(despues)?])
            })
            .collect();
        // `usuario@host: Permission denied (publickey).`
        if let Some(i) = l.find(": permission denied (") {
            candidatos.push(l[..i].trim());
        }
        for c in candidatos {
            let c = nombre_limpio(c);
            // `Connection closed by UNKNOWN port 65535` es un salto que no
            // sabe con quién hablaba: no nombra a nadie.
            if !c.is_empty() && c != "unknown" && !c.contains(char::is_whitespace) {
                fuera.push(c);
            }
        }
    }
    fuera
}

/// Los `known_hosts` que nombra el aviso de clave cambiada: «Offending … key in
/// <fichero>:<línea>» y «Add correct host key in <fichero> to get rid of…».
fn known_hosts_nombrados(e: &str) -> Vec<String> {
    let mut fuera = Vec::new();
    for l in e.lines() {
        if let Some(i) = l.find("add correct host key in ") {
            let resto = &l[i + "add correct host key in ".len()..];
            if let Some(f) = resto.find(" to get rid") {
                fuera.push(ruta_normal(&resto[..f]));
            }
        } else if l.contains("offending") {
            if let Some(i) = l.find(" key in ") {
                let resto = l[i + " key in ".len()..].trim();
                // El `:<línea>` del final; el `C:` de una unidad se queda.
                let sin_linea = match resto.rsplit_once(':') {
                    Some((f, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => f,
                    _ => resto,
                };
                fuera.push(ruta_normal(sin_linea));
            }
        }
    }
    fuera
}

/// Si un mensaje de `ssh` habla de **nuestro** destino: `Some(true)` si lo
/// nombra, `Some(false)` si sólo nombra a otros, `None` si no nombra a nadie
/// que se pueda distinguir.
///
/// Una IP que no es la nuestra sólo cuenta como «otro» si sabemos cuál es la
/// nuestra: con un `HostName` de DNS, `Connection closed by 203.0.113.7` puede
/// ser cualquiera de los dos, y entonces decide quien llama.
fn nos_nombra(e: &str, n: &Nuestro) -> Option<bool> {
    let hosts = hosts_nombrados(e);
    if hosts.is_empty() {
        let k = n.known_hosts.as_deref()?;
        return known_hosts_nombrados(e)
            .iter()
            .any(|f| f == k)
            .then_some(true);
    }
    let mut nuestros = n.nombres.clone();
    if hosts.iter().any(|h| nuestros.contains(h)) {
        return Some(true);
    }
    if let Some(s) = n.servidor {
        let config = s.config_ssh.as_deref().map(std::path::Path::new);
        nuestros.extend(
            crate::descubrir::nombres_de(&s.destino, config)
                .iter()
                .map(|x| nombre_limpio(x)),
        );
        if hosts.iter().any(|h| nuestros.contains(h)) {
            return Some(true);
        }
    }
    let sabemos_la_ip = nuestros.iter().any(|x| es_ip(x));
    hosts
        .iter()
        .any(|h| !es_ip(h) || sabemos_la_ip)
        .then_some(false)
}

/// Distinguir un fallo del canal de uno de `orbit` no es cosmética: si se
/// confunden, un servidor entero se marca como caído porque el usuario pidió una
/// app que no existe.
fn clasificar(
    r: Respuesta,
    binario: &str,
    origen: Origen,
    nuestro: &Nuestro,
) -> Result<Respuesta, ErrorTransporte> {
    if r.codigo == 0 {
        return Ok(r);
    }
    // Un rc distinto de 0 CON un objeto válido es el caso normal de un
    // despliegue que falla: el objeto trae `ok:false` y `failed_step`, y
    // descartarlo por el código de salida sería tirar el único motivo que se le
    // puede enseñar a nadie.
    //
    // Va **antes** de mirar stderr, y ése fue el fallo: un build que se cae con
    // «pnpm: command not found» en su prosa salía como «no hay un orbit
    // ejecutable», y la interfaz perdía el `failed_step`. Si por stdout ha
    // llegado un objeto, quien ha hablado es `orbit`, y lo que diga stderr es
    // prosa suya, no un diagnóstico del canal.
    if r.objeto().is_ok() {
        return Ok(r);
    }
    let e = r.stderr.to_lowercase();
    // 255 es el código propio de ssh cuando no llega. Con `orbit` sólo puede
    // ser de `ssh`: Orbit no sale con él.
    //
    // Con `exec` es también el de un PHP con un error fatal y el de un `ssh`
    // que la app lanza por su cuenta, que escribe lo mismo que el nuestro. Ahí
    // es nuestro si el mensaje **nos nombra**; si no nombra a nadie, sólo si el
    // otro lado no ha dicho nada (stdout vacío) y stderr es del cliente `ssh`,
    // que es como sale un `ssh` que no ha llegado a conectar. Ante la duda, la
    // salida del comando: equivocarse hacia ahí enseña un texto verdadero;
    // hacia el otro lado, tira la salida y acusa al servidor.
    let del_canal = r.codigo == 255
        && match origen {
            Origen::Orbit => true,
            Origen::Ajena => nos_nombra(&e, nuestro)
                .unwrap_or_else(|| r.stdout.trim().is_empty() && habla_el_cliente_ssh(&e)),
        };
    // Un cambio de clave de host va ANTES del 255 genérico: no es un problema
    // de red, es lo que se ve en una suplantación, y merece su propio camino.
    // Pero **sólo con 255**, que es como sale nuestro `ssh` cuando se niega: un
    // `git pull` dentro de `exec` contra un github.com nunca visto escribe lo
    // mismo y sale con 1, y eso no es que el servidor sea un impostor.
    if del_canal
        && (e.contains("remote host identification has changed")
            || e.contains("host key verification failed"))
    {
        return Err(ErrorTransporte::ClaveDeHostCambiada { detalle: r.stderr });
    }
    if del_canal {
        return Err(ErrorTransporte::NoLlego {
            codigo: 255,
            stderr: r.stderr,
        });
    }
    // 127 (no está) y 126 (no se puede ejecutar), **sólo si el shell habla del
    // binario de Orbit**. Un 127 a secas es cualquier orden que no existe, y con
    // `exec` la orden la ha escrito alguien: una errata no puede salir como «no
    // hay un orbit ejecutable».
    let no_esta = r.codigo == 127
        || r.codigo == 126
        || e.contains("command not found")
        || e.contains("no such file")
        || e.contains("permission denied");
    if no_esta && nombra_al_binario(&e, binario) {
        return Err(ErrorTransporte::OrbitNoEsta {
            ruta: binario.into(),
            stderr: r.stderr,
        });
    }
    // La segunda es la de `sudo -n`, que es como se eleva quien tiene un sudoers
    // que no casa con la regla de `orbit admin`: no pregunta, falla con eso.
    //
    // Sólo cuando habla `orbit`: ver [`Origen`].
    if origen == Origen::Orbit
        && (e.contains("a terminal is required to read the password")
            || e.contains("askpass")
            || e.contains("sudo: a password is required"))
    {
        return Err(ErrorTransporte::SudoPideClave);
    }
    Err(ErrorTransporte::Orbit {
        codigo: r.codigo,
        stdout: r.stdout,
        stderr: r.stderr,
    })
}

/// Escribe la entrada de la orden **y la cierra**.
///
/// En un hilo y no aquí mismo por lo de siempre: si el otro lado no lee —se
/// muere antes, o escribe mucho antes de leer— una escritura que no cabe en la
/// tubería se quedaría esperando mientras nadie lee stdout ni stderr.
///
/// Y cerrarla no es un detalle: `--env-stdin` lee **hasta el final**, y con la
/// tubería abierta Orbit esperaría para siempre a una línea que no va a llegar
/// —un despliegue colgado antes de empezar, con un tope de una hora—. El cierre
/// es el `drop` del extremo, que pasa al terminar el hilo; por SSH se convierte
/// en el EOF del canal.
///
/// Un error al escribir —el otro lado ya se ha ido— no se convierte en el error
/// de la orden: el código de salida y stderr cuentan mejor qué pasó.
fn escribir_entrada(
    hijo: &mut std::process::Child,
    entrada: Option<Secreto>,
) -> Option<std::thread::JoinHandle<()>> {
    use std::io::Write;
    let e = entrada?;
    let mut tubo = hijo.stdin.take()?;
    Some(std::thread::spawn(move || {
        let _ = tubo.write_all(e.exponer().as_bytes());
        let _ = tubo.flush();
        drop(tubo);
    }))
}

/// Lee una tubería **hasta el final**, guardando como mucho `tope` bytes.
///
/// Devuelve lo guardado y cuántos bytes se han tirado. Pasado el tope se sigue
/// leyendo y tirando, y eso es lo importante: la versión anterior volvía al
/// llegar al tope y soltaba el lector, la tubería se cerraba y el proceso del
/// otro lado moría por SIGPIPE —o se quedaba parado con la tubería llena— a
/// mitad de su trabajo. Un `exec web npm run build` muy hablador no terminaba.
/// Se descartó matar el proceso al pasar el tope: en una orden que cambia algo
/// es interrumpirla a medias, que es peor que esperar a que acabe.
fn leer_hasta(mut f: impl Read, tope: usize) -> Result<(String, u64), ErrorTransporte> {
    let mut buf = Vec::with_capacity(8192);
    let mut trozo = [0u8; 8192];
    let mut sobra = 0u64;
    loop {
        match f.read(&mut trozo) {
            Ok(0) => break,
            Ok(n) => {
                let cabe = tope.saturating_sub(buf.len()).min(n);
                buf.extend_from_slice(&trozo[..cabe]);
                sobra += (n - cabe) as u64;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(ErrorTransporte::NoSePudoLanzar(e)),
        }
    }
    // 'lossy' y no un error: un byte que no es UTF-8 en el stderr de un servidor
    // no debe impedir enseñar el resto del mensaje. En stdout, el parser de JSON
    // se quejará por su cuenta, que es donde toca.
    Ok((String::from_utf8_lossy(&buf).into_owned(), sobra))
}

/// Lo que se hace con una salida que ha pasado del tope.
///
/// Si es **el objeto**, cortado no vale nada —medio JSON no es un JSON más
/// corto— y es `Demasiado`. Si es prosa, se devuelve con la marca: cortada y
/// diciéndolo vale más que nada, y en `exec` es justo lo que se pidió ver.
fn recortada(texto: String, sobra: u64, es_el_objeto: bool) -> Result<String, ErrorTransporte> {
    if sobra > 0 && es_el_objeto {
        return Err(ErrorTransporte::Demasiado {
            tope: TOPE_RESPUESTA,
        });
    }
    Ok(con_marca_de_corte(texto, sobra))
}

/// La marca de que falta algo. Sin ella, lo cortado se leería como entero.
pub const MARCA_DE_CORTE: &str = "[Orbit Desktop ha cortado aquí la salida";

fn con_marca_de_corte(mut texto: String, sobra: u64) -> String {
    if sobra > 0 {
        if !texto.is_empty() && !texto.ends_with('\n') {
            texto.push('\n');
        }
        texto.push_str(&format!(
            "{MARCA_DE_CORTE}: {sobra} bytes más, que no se han guardado]\n"
        ));
    }
    texto
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(stdout: &str) -> Respuesta {
        Respuesta {
            stdout: stdout.into(),
            stderr: String::new(),
            codigo: 0,
            cancelado: false,
        }
    }

    #[test]
    fn un_objeto_limpio_pasa() {
        assert!(resp(r#"{"schema":1,"apps":[]}"#).objeto().is_ok());
    }

    #[test]
    fn basura_delante_del_objeto_es_un_fallo_no_algo_que_recortar() {
        let r = resp("Last login: Fri\n{\"schema\":1}");
        assert!(matches!(
            r.objeto(),
            Err(ErrorTransporte::RespuestaSucia(_))
        ));
    }

    #[test]
    fn dos_objetos_tampoco_se_recortan() {
        let r = resp(r#"{"schema":1}{"schema":1}"#);
        assert!(matches!(
            r.objeto(),
            Err(ErrorTransporte::RespuestaSucia(_))
        ));
    }

    #[test]
    fn un_stdout_en_blanco_no_es_una_coleccion_vacia() {
        assert!(resp("").objeto().is_err());
    }

    #[test]
    fn un_schema_que_no_conocemos_se_rechaza() {
        let r = resp(r#"{"schema":2,"apps":[]}"#);
        let x: Result<crate::contrato::Lista, _> = r.leer();
        assert!(x.is_err());
    }

    #[test]
    fn el_marco_de_un_aviso_de_ssh_no_es_el_mensaje() {
        // Lo que imprime OpenSSH cuando la clave de un host cambia. La primera
        // línea es el marco; el mensaje está en la segunda.
        let s = "@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
                 @    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n\
                 @@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
                 IT IS POSSIBLE THAT SOMEONE IS DOING SOMETHING NASTY!";
        let m = primera_linea(s);
        assert!(m.contains("WARNING"), "salió «{m}»");
        assert!(!m.starts_with("@@@"));
    }

    #[test]
    fn el_glifo_de_orbit_se_quita_tambien_el_de_ascii() {
        // Por SSH sin terminal y con LANG=C, Orbit escribe «x» en vez de «✗».
        let ascii = "  x Faltan variables obligatorias (DATABASE_URL).\n";
        assert_eq!(
            primera_linea(ascii),
            "Faltan variables obligatorias (DATABASE_URL)."
        );
        let utf = "  ✗ «tienda» está congelada.\n  · Para desplegar igualmente: …";
        assert_eq!(primera_linea(utf), "«tienda» está congelada.");
        // Sin la sangría de Orbit, una «x » es parte de la frase.
        assert_eq!(primera_linea("x marks the spot"), "x marks the spot");
    }

    #[test]
    fn sudo_sin_contrasena_y_sudo_n_son_el_mismo_caso() {
        for e in [
            "sudo: a terminal is required to read the password; either use the -S option",
            "sudo: a password is required",
        ] {
            let r = Respuesta {
                stdout: String::new(),
                stderr: e.into(),
                codigo: 1,
                cancelado: false,
            };
            assert!(matches!(
                clasificar(r, "/usr/local/bin/orbit", Origen::Orbit, &Nuestro::nadie()),
                Err(ErrorTransporte::SudoPideClave)
            ));
        }
        // Y el mensaje dice cómo se arregla desde 1.4.6.
        assert!(ErrorTransporte::SudoPideClave
            .to_string()
            .contains("orbit admin add"));
    }

    #[cfg(unix)]
    #[test]
    fn la_carpeta_de_los_sockets_es_privada_y_no_se_sigue_un_enlace() {
        use std::os::unix::fs::PermissionsExt;
        let base = std::env::temp_dir().join(format!("orbit-cm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let d = dir_privado(&base, "orbit-desktop").expect("se crea");
        let m = std::fs::metadata(&d).unwrap().permissions().mode() & 0o777;
        assert_eq!(m, 0o700);
        // Una segunda vez vale igual: ya existe y es nuestra.
        assert_eq!(
            dir_privado(&base, "orbit-desktop").as_deref(),
            Some(d.as_str())
        );

        // Un enlace en su sitio no se usa, aunque apunte a una carpeta nuestra.
        let otra = base.join("otra");
        std::fs::create_dir_all(&otra).unwrap();
        std::os::unix::fs::symlink(&otra, base.join("enlace")).unwrap();
        assert_eq!(dir_privado(&base, "enlace"), None);

        // Y una carpeta abierta se cierra.
        let abierta = base.join("abierta");
        std::fs::create_dir_all(&abierta).unwrap();
        std::fs::set_permissions(&abierta, std::fs::Permissions::from_mode(0o755)).unwrap();
        dir_privado(&base, "abierta").unwrap();
        let m = std::fs::metadata(&abierta).unwrap().permissions().mode() & 0o777;
        assert_eq!(m, 0o700);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn el_mensaje_es_la_primera_linea_util_no_la_ultima() {
        // El fallo que Orbit ya cometió: quedarse con la última línea de git
        // daba «and the repository exists.», un trozo de frase suelto.
        let s = "fatal: could not read Username\nPlease make sure you have the correct access rights\nand the repository exists.";
        assert!(primera_linea(s).starts_with("fatal:"));
    }

    #[test]
    fn las_opciones_de_ssh_llevan_las_cuatro_que_importan() {
        let s = Servidor::nuevo("prod", "vps.test");
        let o = s.opciones_ssh(None).join(" ");
        assert!(o.contains("BatchMode=yes"));
        assert!(o.contains("StrictHostKeyChecking=accept-new"));
        assert!(o.contains("ForwardAgent=no"));
        assert!(o.contains("ClearAllForwardings=yes"));
        // Nunca 'no' en la comprobación de host.
        assert!(!o.contains("StrictHostKeyChecking=no"));
    }

    #[test]
    fn las_opciones_de_ssh_acotan_la_conexion_y_apagan_x11() {
        // Las de docs/CLIENT.md §3.2 que faltaban: sin ellas, una red que se cae
        // a mitad de una instalación —que no tiene plazo— la cuelga para siempre.
        let o = Servidor::nuevo("prod", "vps.test")
            .opciones_ssh(None)
            .join(" ");
        for op in [
            "ConnectTimeout=10",
            "ServerAliveInterval=15",
            "ServerAliveCountMax=3",
            "ForwardX11=no",
            "RequestTTY=no",
        ] {
            assert!(o.contains(op), "falta {op}");
        }
    }

    fn args(c: &Command) -> Vec<String> {
        c.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn el_destino_va_detras_de_un_doble_guion() {
        // Sin el `--`, un alias como `-oProxyCommand=…` llegado de la interfaz
        // lo leía `ssh` como una opción: ejecutar una orden local.
        let s = Servidor::nuevo("x", "-oProxyCommand=calc");
        let a = args(&s.orden_ssh(None, "/usr/local/bin/orbit --json version"));
        let n = a.len();
        assert_eq!(a[n - 3], "--");
        assert_eq!(a[n - 2], "-oProxyCommand=calc");
        assert_eq!(a[n - 1], "/usr/local/bin/orbit --json version");
        // Y es el único `--`: las opciones van todas delante.
        assert_eq!(a.iter().filter(|x| *x == "--").count(), 1);
    }

    #[test]
    fn un_byte_que_no_es_utf8_no_corta_el_progreso() {
        // Una herramienta de build que escribe en Latin-1. Con `lines()` el
        // lector se paraba aquí y el resto del despliegue llegaba en silencio.
        let entrada: &[u8] = b"uno\r\ncaf\xe9\nlo que viene detr\xc3\xa1s\nsin salto";
        let mut v = Vec::new();
        servir_lineas(entrada, |l| v.push(l));
        assert_eq!(
            v,
            ["uno", "caf\u{fffd}", "lo que viene detrás", "sin salto"]
        );
    }

    fn r(codigo: i32, stdout: &str, stderr: &str) -> Respuesta {
        Respuesta {
            stdout: stdout.into(),
            stderr: stderr.into(),
            codigo,
            cancelado: false,
        }
    }

    const BIN: &str = "/usr/local/bin/orbit";

    #[test]
    fn un_objeto_manda_sobre_lo_que_diga_stderr() {
        // Un despliegue que falla porque falta pnpm: sale con 1, con su objeto,
        // y la prosa dice «command not found». Antes salía como «no hay un
        // orbit ejecutable» y la interfaz perdía el `failed_step`.
        let x = r(
            1,
            r#"{"schema":1,"ok":false,"failed_step":"build"}"#,
            "  ✗ sh: 1: pnpm: command not found\n",
        );
        assert!(clasificar(x, BIN, Origen::Orbit, &Nuestro::nadie()).is_ok());
        // Y lo mismo con un `sudo` dentro del build.
        let x = r(
            1,
            r#"{"schema":1,"ok":false}"#,
            "sudo: a password is required\n",
        );
        assert!(clasificar(x, BIN, Origen::Orbit, &Nuestro::nadie()).is_ok());
    }

    #[test]
    fn un_127_solo_es_orbit_no_esta_si_el_shell_nombra_a_orbit() {
        for e in [
            "bash: /usr/local/bin/orbit: No such file or directory",
            "bash: line 1: /usr/local/bin/orbit: No such file or directory",
            "sh: 1: /usr/local/bin/orbit: not found",
            "bash: orbit: command not found",
        ] {
            assert!(
                matches!(
                    clasificar(r(127, "", e), BIN, Origen::Orbit, &Nuestro::nadie()),
                    Err(ErrorTransporte::OrbitNoEsta { .. })
                ),
                "{e}"
            );
        }
        // Una errata dentro de `exec` no es que falte Orbit: es la salida del
        // comando, y es lo que hay que enseñar.
        let x = r(127, "", "bash: line 1: pnmp: command not found");
        assert!(matches!(
            clasificar(x, BIN, Origen::Ajena, &Nuestro::nadie()),
            Err(ErrorTransporte::Orbit { codigo: 127, .. })
        ));
        // Ni aunque hable `orbit`: el 127 es de otra orden.
        let x = r(127, "", "bash: pnmp: command not found");
        assert!(matches!(
            clasificar(x, BIN, Origen::Orbit, &Nuestro::nadie()),
            Err(ErrorTransporte::Orbit { .. })
        ));
        // Y un 126 de un script de la app tampoco.
        let x = r(126, "", "bash: ./mantenimiento.sh: Permission denied");
        assert!(matches!(
            clasificar(x, BIN, Origen::Ajena, &Nuestro::nadie()),
            Err(ErrorTransporte::Orbit { codigo: 126, .. })
        ));
    }

    #[test]
    fn un_sudo_dentro_de_exec_no_es_que_orbit_no_pueda_elevarse() {
        let e = "sudo: a terminal is required to read the password; either use the -S option";
        assert!(matches!(
            clasificar(r(1, "", e), BIN, Origen::Ajena, &Nuestro::nadie()),
            Err(ErrorTransporte::Orbit { codigo: 1, .. })
        ));
        assert!(matches!(
            clasificar(r(1, "", e), BIN, Origen::Orbit, &Nuestro::nadie()),
            Err(ErrorTransporte::SudoPideClave)
        ));
        // Lo de ssh se sigue reconociendo venga de donde venga.
        assert!(matches!(
            clasificar(
                r(
                    255,
                    "",
                    "ssh: connect to host x port 22: Connection refused"
                ),
                BIN,
                Origen::Ajena,
                &nuestro_vps("x"),
            ),
            Err(ErrorTransporte::NoLlego { .. })
        ));
        assert_eq!(
            Origen::de(&Comando::Ejecutar {
                app: "web".into(),
                modo: crate::comando::ModoDeExec::Argumentos(vec!["ls".into()]),
            }),
            Origen::Ajena
        );
        assert_eq!(Origen::de(&Comando::Lista), Origen::Orbit);
    }

    /// Un proceso que tarda más que su plazo se mata y se dice `Tarde`, aunque
    /// la espera ya no sondee.
    #[test]
    fn la_espera_bloqueante_respeta_el_plazo() {
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 20"]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let mut c = Command::new("sleep");
            c.arg("20");
            c
        };
        let t = std::time::Instant::now();
        let e = ejecutar_proceso(
            cmd,
            Duration::from_millis(500),
            BIN,
            None,
            Origen::Ajena,
            false,
            &Nuestro::nadie(),
        );
        assert!(matches!(e, Err(ErrorTransporte::Tarde { .. })), "{e:?}");
        assert!(t.elapsed() < Duration::from_secs(10), "{:?}", t.elapsed());

        // Y uno que termina enseguida vuelve con su salida entera: la espera se
        // despierta por el cierre de las tuberías, no por un plazo.
        let mut c = Command::new("ssh");
        c.arg("-V");
        let r = ejecutar_proceso(
            c,
            Duration::from_secs(30),
            BIN,
            None,
            Origen::Ajena,
            false,
            &Nuestro::nadie(),
        )
        .unwrap();
        assert!(r.stderr.contains("OpenSSH"), "{r:?}");
    }

    fn desplegar() -> Comando {
        Comando::Desplegar {
            app: "web".into(),
            progreso: true,
            entorno: vec![],
            forzar: false,
            por_entrada: false,
        }
    }

    /// Lo que cambia algo no se corta antes de lo que el servidor puede tardar
    /// en hacerlo: matar el `ssh` local no para la orden remota.
    #[test]
    fn rc_lo_que_cambia_algo_no_se_corta_antes_que_el_cerrojo() {
        let app = || "web".to_string();
        for c in [
            Comando::Revertir {
                app: app(),
                release: "20261101-000000".into(),
            },
            Comando::Retirar { app: app() },
            Comando::RetirarYBorrar { app: app() },
            Comando::Congelar {
                app: app(),
                motivo: None,
            },
            Comando::Descongelar { app: app() },
            Comando::EsperarCi {
                app: app(),
                esperar: true,
            },
            Comando::DoctorArreglar,
            Comando::Ejecutar {
                app: app(),
                modo: crate::comando::ModoDeExec::Argumentos(vec!["ls".into()]),
            },
        ] {
            assert!(cambia_el_servidor(&c), "{c:?}");
            assert!(
                tope_de_tiempo(&c) > ESPERA_DEL_CERROJO,
                "{c:?} se corta a {:?}",
                tope_de_tiempo(&c)
            );
        }
        // La pasada tiene su propio tope, mayor que el de un despliegue.
        let todas = Comando::DesplegarTodo {
            progreso: true,
            solo_si_cambia: false,
        };
        assert!(tope_de_tiempo(&todas) > tope_de_tiempo(&desplegar()));
        assert!(tope_de_tiempo(&desplegar()) > ESPERA_DEL_CERROJO);
        // Y lo de sólo leer conserva su plazo corto.
        assert!(!cambia_el_servidor(&Comando::Version));
        assert_eq!(tope_de_tiempo(&Comando::Version), Duration::from_secs(30));
    }

    /// Un plazo vencido en algo que cambia el servidor no dice «no ha pasado
    /// nada»: dice que no se sabe. La clase sigue siendo la misma.
    #[test]
    fn rc_un_plazo_vencido_en_algo_que_cambia_dice_que_no_se_sabe() {
        let lee = ErrorTransporte::Tarde {
            tope: Duration::from_secs(30),
            cambia: false,
        }
        .to_string();
        assert!(lee.contains("no ha contestado en 30 s"), "{lee}");
        let cambia = ErrorTransporte::Tarde {
            tope: Duration::from_secs(1200),
            cambia: true,
        }
        .to_string();
        assert!(cambia.contains("puede seguir en marcha"), "{cambia}");
        assert!(cambia.contains("no sé cómo ha quedado"), "{cambia}");
        assert!(cambia.contains("20 min"), "{cambia}");
    }

    /// Y el plazo de verdad lleva la marca: un proceso que no termina, con una
    /// orden que cambia algo, sale como «no sé cómo ha quedado».
    #[test]
    fn rc_el_plazo_vencido_lleva_si_la_orden_cambia_algo() {
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("powershell");
            c.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 20"]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let mut c = Command::new("sleep");
            c.arg("20");
            c
        };
        let e = ejecutar_proceso(
            cmd,
            Duration::from_millis(300),
            BIN,
            None,
            Origen::Ajena,
            true,
            &Nuestro::nadie(),
        );
        assert!(
            matches!(e, Err(ErrorTransporte::Tarde { cambia: true, .. })),
            "{e:?}"
        );
    }

    #[test]
    fn rc_un_255_de_exec_sin_mensaje_de_ssh_es_la_salida_del_comando() {
        // Un PHP con un error fatal: 255, y su salida es lo que hay que ver.
        let x = r(
            255,
            "Migrating: 2026_11_01_000000_crear_pedidos\n",
            "PHP Fatal error:  Uncaught PDOException: SQLSTATE[HY000] [2002] Connection timed out",
        );
        match clasificar(x, BIN, Origen::Ajena, &Nuestro::nadie()) {
            Err(ErrorTransporte::Orbit { codigo, stdout, .. }) => {
                assert_eq!(codigo, 255);
                assert!(stdout.contains("Migrating"));
            }
            otro => panic!("{otro:?}"),
        }
        // Con un mensaje que es de `ssh`, sí es no haber llegado.
        for e in [
            "ssh: connect to host vps port 22: Connection timed out",
            "ssh: Could not resolve hostname vps: Name or service not known",
            "kex_exchange_identification: read: Connection reset by peer",
            "Connection closed by 203.0.113.7 port 22",
            "dave@vps: Permission denied (publickey).",
            "Connection to vps closed by remote host.",
            "Timeout, server vps not responding.",
        ] {
            assert!(
                matches!(
                    clasificar(r(255, "", e), BIN, Origen::Ajena, &nuestro_vps("vps")),
                    Err(ErrorTransporte::NoLlego { .. })
                ),
                "{e}"
            );
        }
        // Y con `orbit` un 255 sigue siendo de `ssh`, diga lo que diga.
        assert!(matches!(
            clasificar(
                r(255, "", "algo raro"),
                BIN,
                Origen::Orbit,
                &Nuestro::nadie()
            ),
            Err(ErrorTransporte::NoLlego { .. })
        ));
    }

    #[test]
    fn rc_el_host_key_de_otro_sitio_no_es_el_del_servidor() {
        // `git pull` dentro de `exec`, contra un github.com que el servidor no
        // conoce: el mismo texto, con 1.
        let git = "Host key verification failed.\nfatal: Could not read from remote repository.";
        for o in [Origen::Ajena, Origen::Orbit] {
            assert!(
                !matches!(
                    clasificar(r(1, "", git), BIN, o, &Nuestro::nadie()),
                    Err(ErrorTransporte::ClaveDeHostCambiada { .. })
                ),
                "{o:?}"
            );
        }
        // El nuestro sale con 255, y ése sí es la pantalla bloqueante.
        for o in [Origen::Ajena, Origen::Orbit] {
            assert!(matches!(
                clasificar(
                    r(255, "", "Host key verification failed."),
                    BIN,
                    o,
                    &Nuestro::nadie()
                ),
                Err(ErrorTransporte::ClaveDeHostCambiada { .. })
            ));
        }
    }

    /// Un destino que se llama `alias`, sin `ssh -G` detrás.
    fn nuestro_vps(alias: &str) -> Nuestro<'static> {
        Nuestro {
            nombres: vec![alias.into()],
            known_hosts: Some(ruta_normal(r"C:\Users\dave\.ssh\known_hosts")),
            servidor: None,
        }
    }

    /// Un `ssh` que la app lanza por su cuenta escribe lo mismo que el nuestro
    /// y sale con 255. Sólo es nuestro si nombra a nuestro destino, o si no
    /// nombra a nadie y el otro lado no ha dicho nada.
    #[test]
    fn rc2_un_ssh_dentro_de_exec_no_es_el_nuestro() {
        let web = nuestro_vps("web");
        let otro = "ssh: connect to host db.interno port 22: Connection refused";
        for stdout in ["", "salida del comando\n"] {
            match clasificar(r(255, stdout, otro), BIN, Origen::Ajena, &web) {
                Err(ErrorTransporte::Orbit {
                    codigo: 255,
                    stdout: o,
                    ..
                }) => assert_eq!(o, stdout),
                x => panic!("{stdout:?}: {x:?}"),
            }
        }
        let aviso = "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
            @    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n\
            Offending ECDSA key in /home/deploy/.ssh/known_hosts:3\n\
            Host key for db.interno has changed and you have requested strict checking.\n\
            Host key verification failed.\n";
        assert!(matches!(
            clasificar(r(255, "", aviso), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::Orbit { codigo: 255, .. })
        ));
        // El mismo aviso, del nuestro: por el host o por nuestro known_hosts.
        let nuestro = aviso.replace("db.interno", "web");
        assert!(matches!(
            clasificar(r(255, "", &nuestro), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::ClaveDeHostCambiada { .. })
        ));
        let por_fichero = r"WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!
Add correct host key in C:\\Users\\dave/.ssh/known_hosts to get rid of this message.
Offending ECDSA key in C:\\Users\\dave/.ssh/known_hosts:7
Host key verification failed.";
        assert!(matches!(
            clasificar(r(255, "", por_fichero), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::ClaveDeHostCambiada { .. })
        ));
        // Con `[host]:puerto`.
        let puerto = "Host key for [web]:2222 has changed and you have requested strict \
                      checking.\nHost key verification failed.";
        assert!(matches!(
            clasificar(r(255, "", puerto), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::ClaveDeHostCambiada { .. })
        ));
        // Una IP cuando no sabemos la nuestra no decide: decide el stdout.
        let ip = "Connection closed by 203.0.113.7 port 22";
        assert!(matches!(
            clasificar(r(255, "", ip), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::NoLlego { .. })
        ));
        assert!(matches!(
            clasificar(r(255, "algo\n", ip), BIN, Origen::Ajena, &web),
            Err(ErrorTransporte::Orbit { .. })
        ));
        // Y si sabemos la nuestra, otra IP es otro.
        let por_ip = nuestro_vps("10.0.0.5");
        assert!(matches!(
            clasificar(r(255, "", ip), BIN, Origen::Ajena, &por_ip),
            Err(ErrorTransporte::Orbit { .. })
        ));
        // Con `orbit` nada de esto cambia: un 255 es nuestro, nombre a quien
        // nombre.
        assert!(matches!(
            clasificar(r(255, "", otro), BIN, Origen::Orbit, &web),
            Err(ErrorTransporte::NoLlego { .. })
        ));
        assert!(matches!(
            clasificar(r(255, "x", aviso), BIN, Origen::Orbit, &Nuestro::nadie()),
            Err(ErrorTransporte::ClaveDeHostCambiada { .. })
        ));
    }

    #[test]
    fn rc2_los_nombres_de_los_mensajes_de_ssh() {
        let e = "ssh: connect to host db.interno port 22: connection refused\n\
                 dave@10.0.0.5: permission denied (publickey).\n\
                 connection closed by unknown port 65535\n\
                 host key for [web]:2222 has changed and you have requested strict checking.";
        assert_eq!(hosts_nombrados(e), ["db.interno", "10.0.0.5", "web"]);
        assert_eq!(nombre_limpio("Root@Web."), "web");
        assert_eq!(
            known_hosts_nombrados(r"offending ecdsa key in c:\\users\\a/.ssh/known_hosts:12"),
            ["c:/users/a/.ssh/known_hosts"]
        );
    }

    /// Con un destino de verdad, el `HostName` del alias lo dice `ssh -G`: el
    /// mensaje nombra la IP y no el alias.
    #[test]
    fn rc2_el_hostname_del_alias_tambien_es_nuestro() {
        let dir = std::env::temp_dir().join(format!("orbit-rc2-hostname-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config");
        std::fs::write(
            &cfg,
            "Host web\n  HostName 10.9.8.7\n  ProxyJump salto.ejemplo\n",
        )
        .unwrap();
        let mut s = Servidor::nuevo("web", "web");
        s.config_ssh = Some(cfg.to_string_lossy().into_owned());
        let n = Nuestro::de(&s);
        for e in [
            "ssh: connect to host 10.9.8.7 port 22: Connection refused",
            "ssh: connect to host salto.ejemplo port 22: Connection refused",
        ] {
            assert!(
                matches!(
                    clasificar(r(255, "", e), BIN, Origen::Ajena, &n),
                    Err(ErrorTransporte::NoLlego { .. })
                ),
                "{e}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rc_pasado_el_tope_se_sigue_leyendo_hasta_el_final() {
        // Un lector que cuenta lo que se le pide: si se suelta antes del final,
        // el proceso del otro lado se queda con la tubería rota.
        struct Fuente {
            queda: usize,
        }
        impl Read for Fuente {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                let n = b.len().min(self.queda);
                b[..n].fill(b'x');
                self.queda -= n;
                Ok(n)
            }
        }
        let mut f = Fuente { queda: 100_000 };
        let (t, sobra) = leer_hasta(&mut f, 1000).unwrap();
        assert_eq!(f.queda, 0, "se ha dejado de leer antes del final");
        assert_eq!(t.len(), 1000);
        assert_eq!(sobra, 99_000);
        // Por debajo del tope, entero y sin marca.
        let (t, sobra) = leer_hasta(&b"hola"[..], 1000).unwrap();
        assert_eq!((t.as_str(), sobra), ("hola", 0));
    }

    #[test]
    fn rc_lo_cortado_se_dice_y_el_objeto_cortado_no_vale() {
        let p = recortada("mucho texto".into(), 42, false).unwrap();
        assert!(p.starts_with("mucho texto\n"));
        assert!(p.contains(MARCA_DE_CORTE) && p.contains("42 bytes"), "{p}");
        assert!(matches!(
            recortada("{\"schema\":1".into(), 1, true),
            Err(ErrorTransporte::Demasiado { .. })
        ));
        assert_eq!(recortada("entero".into(), 0, true).unwrap(), "entero");
    }

    /// Un stderr enorme de `exec` ya no tumba el comando: termina, se lee su
    /// stdout entero y stderr llega cortado con la marca.
    #[test]
    fn rc_un_stderr_enorme_no_corta_el_comando() {
        #[cfg(windows)]
        let cmd = {
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-Command",
                "$s = 'x' * 1048576; for ($i = 0; $i -lt 6; $i++) { [Console]::Error.Write($s) }; \
                 [Console]::Out.Write('fin-del-trabajo')",
            ]);
            c
        };
        #[cfg(not(windows))]
        let cmd = {
            let mut c = Command::new("sh");
            c.args([
                "-c",
                "head -c 6291456 /dev/zero | tr '\\0' x >&2; printf fin-del-trabajo",
            ]);
            c
        };
        let r = ejecutar_proceso(
            cmd,
            Duration::from_secs(60),
            BIN,
            None,
            Origen::Ajena,
            true,
            &Nuestro::nadie(),
        )
        .expect("termina");
        assert_eq!(r.codigo, 0);
        assert!(r.stdout.contains("fin-del-trabajo"), "{:?}", r.stdout);
        assert!(r.stderr.contains(MARCA_DE_CORTE));
        assert!(r.stderr.len() < TOPE_RESPUESTA + 200);
    }

    #[test]
    fn rc_el_camino_sin_tipar_comprueba_el_schema() {
        let c = Comando::Lista;
        assert!(resp(r#"{"schema":1,"apps":[]}"#)
            .objeto_del_contrato(&c)
            .is_ok());
        for malo in [
            r#"{"schema":2,"apps":[]}"#,
            r#"{"schema":"1","apps":[]}"#,
            r#"{"schema":1.5,"apps":[]}"#,
            r#"{"apps":[]}"#,
        ] {
            assert!(
                matches!(
                    resp(malo).objeto_del_contrato(&c),
                    Err(ErrorTransporte::Json(_))
                ),
                "{malo}"
            );
        }
        // `leer` aplica la misma regla.
        assert!(resp(r#"{"schema":"1","apps":[]}"#)
            .leer::<crate::contrato::Lista>()
            .is_err());
    }

    #[test]
    fn rc_un_ok_que_no_casa_con_el_codigo_es_un_error_de_respuesta() {
        let d = desplegar();
        let con = |codigo: i32, ok: &str| Respuesta {
            stdout: format!(r#"{{"schema":1,"app":"web","ok":{ok}}}"#),
            stderr: String::new(),
            codigo,
            cancelado: false,
        };
        assert!(con(0, "true").objeto_del_contrato(&d).is_ok());
        assert!(con(1, "false").objeto_del_contrato(&d).is_ok());
        assert!(matches!(
            con(1, "true").objeto_del_contrato(&d),
            Err(ErrorTransporte::Json(_))
        ));
        assert!(matches!(
            con(0, "false").objeto_del_contrato(&d),
            Err(ErrorTransporte::Json(_))
        ));
        assert!(con(0, "\"si\"").objeto_del_contrato(&d).is_err());
        // Cancelado: sin código que comparar, sea cual sea el que quede. En
        // Windows `kill` deja un 1; en Unix, ninguno (-1).
        for codigo in [1, -1] {
            let r = Respuesta {
                cancelado: true,
                ..con(codigo, "true")
            };
            assert!(r.objeto_del_contrato(&d).is_ok(), "{codigo}");
        }
        // Y un -1 que no es una cancelación ya no se perdona.
        assert!(con(-1, "true").objeto_del_contrato(&d).is_err());
        // Y `env check`, cuyo `ok` es del `.env` y no de la orden, sale con 0.
        let check = Comando::EntornoComprobar { app: "web".into() };
        assert!(con(0, "false").objeto_del_contrato(&check).is_ok());
    }

    #[test]
    fn el_persist_del_multiplexado_es_corto_y_nunca_eterno() {
        let s = Servidor::nuevo("prod", "vps.test");
        let o = s.opciones_ssh(Some("/run/user/1000/orbit")).join(" ");
        assert!(o.contains("ControlPersist=45"));
        assert!(!o.contains("ControlPersist=yes"));
        // '%C' y no el hostname: la ruta la puede listar otro proceso.
        assert!(o.contains("ControlPath=/run/user/1000/orbit/%C"));
    }

    #[test]
    fn los_topes_de_tiempo_son_por_comando() {
        // Medido: 'top' cuesta 2,1 s con 40 apps y un despliegue son minutos.
        // Un tope único o ahoga al despliegue o no protege de nada.
        // El orden sale de lo medido: version 72 ms, list 306 ms, top 2,1 s,
        // doctor 1,42 s pero con 40 `dig` detrás, y un despliegue son minutos.
        assert!(tope_de_tiempo(&Comando::Version) < tope_de_tiempo(&Comando::Top));
        assert!(tope_de_tiempo(&Comando::Top) < tope_de_tiempo(&Comando::Doctor));
        assert!(
            tope_de_tiempo(&Comando::Doctor)
                < tope_de_tiempo(&Comando::Desplegar {
                    app: "x".into(),
                    progreso: false,
                    entorno: vec![],
                    forzar: false,
                    por_entrada: false,
                })
        );
        // Y un log en vivo no tiene plazo, porque no terminar es lo que hace.
        assert_eq!(
            tope_de_tiempo(&Comando::Logs {
                app: "x".into(),
                desde: None,
                lineas: None,
                seguir: true,
                solo_nginx: false
            }),
            Duration::MAX
        );
    }
}
