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

use crate::comando::{Comando, ErrorForma, Vena};

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

        // Un build de verdad tarda minutos, y ése es el trabajo.
        Comando::Desplegar { .. } | Comando::DesplegarTodo { .. } => Duration::from_secs(3600),

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
    Tarde {
        tope: Duration,
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
                f, "ese usuario necesita contraseña para sudo, y aquí no hay terminal donde escribirla"),
            Self::Orbit { stderr, codigo, .. } => {
                let m = primera_linea(stderr);
                if m.is_empty() { write!(f, "orbit ha salido con {codigo}") } else { write!(f, "{m}") }
            }
            Self::Demasiado { tope } => write!(f, "la respuesta pasa de {tope} bytes y la he cortado"),
            Self::Tarde { tope } => write!(f, "el servidor no ha contestado en {tope:?}"),
            Self::RespuestaSucia(q) => write!(f, "por stdout no ha venido un solo objeto JSON: {q}"),
            Self::Json(e) => write!(f, "no he entendido la respuesta: {e}"),
        }
    }
}

impl std::error::Error for ErrorTransporte {}

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
        .map(|l| l.trim_start_matches("  ").trim_start_matches("✗ ").trim())
        .find(|l| {
            // Con contenido, y que no sea puro marco de dibujo.
            !l.is_empty() && l.chars().any(|c| c.is_alphanumeric())
        })
        .unwrap_or("")
        .to_string()
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
        if let Some(sc) = v.get("schema").and_then(|x| x.as_u64()) {
            if sc != crate::contrato::CONTRATO_CONOCIDO as u64 {
                return Err(ErrorTransporte::Json(format!(
                    "el servidor habla el schema {sc} y este cliente entiende el {}",
                    crate::contrato::CONTRATO_CONOCIDO
                )));
            }
        }
        serde_json::from_str(s).map_err(|e| ErrorTransporte::Json(e.to_string()))
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

    let mut cmd = Command::new("ssh");
    cmd.args(servidor.opciones_ssh(dir_control));
    cmd.arg(&servidor.destino);
    // Un solo argumento. OpenSSH concatena con espacios los que le sobran, así
    // que pasarle varios no separa nada — construiría una cadena y haría creer
    // que no. La cadena la construimos nosotros, escapada, arriba.
    cmd.arg(&linea);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_proceso(cmd, tope_de_tiempo(comando), &servidor.binario)
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
    ejecutar_proceso(cmd, tope_de_tiempo(comando), binario)
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

    let mut cmd = Command::new("ssh");
    cmd.args(servidor.opciones_ssh(dir_control));
    cmd.arg(&servidor.destino);
    cmd.arg(&linea);
    for (k, v) in entorno {
        cmd.env(k, v);
    }
    ejecutar_sirviendo(
        cmd,
        tope_de_tiempo(comando),
        &servidor.binario,
        comando.vena_humana(),
        en_curso,
        al_llegar,
    )
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
        en_curso,
        al_llegar,
    )
}

fn ejecutar_sirviendo(
    mut cmd: Command,
    tope: Duration,
    binario: &str,
    vena: Vena,
    en_curso: EnCurso,
    mut al_llegar: impl FnMut(String) + Send + 'static,
) -> Result<Respuesta, ErrorTransporte> {
    use std::io::BufRead;
    use std::sync::Arc;

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut hijo = cmd.spawn().map_err(ErrorTransporte::NoSePudoLanzar)?;

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

    let h_guardada = std::thread::spawn(move || leer_hasta(guardada, TOPE_RESPUESTA));

    // La otra se sirve línea a línea. Los dos hilos siguen siendo dos: leerlos
    // en serie es un bloqueo mutuo en cuanto el que no se lee llena su tubería.
    let acumulado = Arc::new(std::sync::Mutex::new(String::new()));
    let acc = Arc::clone(&acumulado);
    let h_servida = std::thread::spawn(move || {
        let lector = std::io::BufReader::new(servida);
        for linea in lector.lines() {
            let l = match linea {
                Ok(l) => l,
                Err(_) => break,
            };
            if let Ok(mut a) = acc.lock() {
                // El presupuesto también aplica aquí: un servidor que gotee
                // para siempre no puede llenarnos la memoria.
                if a.len() < TOPE_RESPUESTA {
                    a.push_str(&l);
                    a.push('\n');
                }
            }
            al_llegar(l);
        }
    });

    let inicio = std::time::Instant::now();
    let mut cancelado = false;

    let estado = loop {
        match hijo.try_wait().map_err(ErrorTransporte::NoSePudoLanzar)? {
            Some(e) => break e,
            None => {
                if en_curso.cancelado() {
                    cancelado = true;
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    break hijo.wait().map_err(ErrorTransporte::NoSePudoLanzar)?;
                }
                if tope != Duration::MAX && inicio.elapsed() > tope {
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    return Err(ErrorTransporte::Tarde { tope });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };

    let (guardada, corto) = h_guardada.join().unwrap()?;
    let _ = h_servida.join();
    if corto {
        return Err(ErrorTransporte::Demasiado {
            tope: TOPE_RESPUESTA,
        });
    }
    let servida = acumulado.lock().map(|a| a.clone()).unwrap_or_default();
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
    };
    if cancelado {
        // Cancelado NO es fallido, y no se puede clasificar como tal: el
        // proceso remoto puede haber terminado su paso antes de morir, y decir
        // «ha fallado» sería afirmar algo que no se sabe.
        return Ok(r);
    }
    clasificar(r, binario)
}

fn ejecutar_proceso(
    mut cmd: Command,
    tope: Duration,
    binario: &str,
) -> Result<Respuesta, ErrorTransporte> {
    cmd.stdin(Stdio::null()) // nadie va a contestar a un selector
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut hijo = cmd.spawn().map_err(ErrorTransporte::NoSePudoLanzar)?;

    // Los dos descriptores, a la vez y en hilos. Leerlos en serie es un bloqueo
    // mutuo en cuanto el que no se lee llena su tubería, y con --json el Orbit
    // real escribe por stderr **todo** lo que le cuenta a una persona.
    let so = hijo.stdout.take().unwrap();
    let se = hijo.stderr.take().unwrap();
    let h1 = std::thread::spawn(move || leer_hasta(so, TOPE_RESPUESTA));
    let h2 = std::thread::spawn(move || leer_hasta(se, TOPE_RESPUESTA));

    // Espera con tope. Se sondea en vez de bloquear porque `wait` no sabe de
    // plazos, y un servidor que gotea un byte por minuto no puede tenernos aquí
    // para siempre.
    let inicio = std::time::Instant::now();
    let estado = loop {
        match hijo.try_wait().map_err(ErrorTransporte::NoSePudoLanzar)? {
            Some(e) => break e,
            None => {
                if tope != Duration::MAX && inicio.elapsed() > tope {
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    return Err(ErrorTransporte::Tarde { tope });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };

    let (stdout, corto_o) = h1.join().unwrap()?;
    let (stderr, _) = h2.join().unwrap()?;
    if corto_o {
        return Err(ErrorTransporte::Demasiado {
            tope: TOPE_RESPUESTA,
        });
    }

    let codigo = estado.code().unwrap_or(-1);
    clasificar(
        Respuesta {
            stdout,
            stderr,
            codigo,
        },
        binario,
    )
}

/// Distinguir un fallo del canal de uno de `orbit` no es cosmética: si se
/// confunden, un servidor entero se marca como caído porque el usuario pidió una
/// app que no existe.
fn clasificar(r: Respuesta, binario: &str) -> Result<Respuesta, ErrorTransporte> {
    if r.codigo == 0 {
        return Ok(r);
    }
    let e = r.stderr.to_lowercase();
    if r.codigo == 127 || e.contains("command not found") {
        return Err(ErrorTransporte::OrbitNoEsta {
            ruta: binario.into(),
            stderr: r.stderr,
        });
    }
    if r.codigo == 126 || e.contains("permission denied") && e.contains(binario) {
        return Err(ErrorTransporte::OrbitNoEsta {
            ruta: binario.into(),
            stderr: r.stderr,
        });
    }
    if e.contains("a terminal is required to read the password") || e.contains("askpass") {
        return Err(ErrorTransporte::SudoPideClave);
    }
    // 255 es el código propio de ssh cuando no llega. Es distinto de que orbit
    // haya fallado, y por eso son dos variantes.
    // Un cambio de clave de host va ANTES del 255 genérico: no es un problema
    // de red, es lo que se ve en una suplantación, y merece su propio camino.
    if e.contains("remote host identification has changed")
        || e.contains("host key verification failed")
    {
        return Err(ErrorTransporte::ClaveDeHostCambiada { detalle: r.stderr });
    }
    if r.codigo == 255 {
        return Err(ErrorTransporte::NoLlego {
            codigo: 255,
            stderr: r.stderr,
        });
    }
    // Un rc distinto de 0 CON un objeto válido es el caso normal de un
    // despliegue que falla: el objeto trae `ok:false` y `failed_step`, y
    // descartarlo por el código de salida sería tirar el único motivo que se le
    // puede enseñar a nadie.
    if r.objeto().is_ok() {
        return Ok(r);
    }
    Err(ErrorTransporte::Orbit {
        codigo: r.codigo,
        stdout: r.stdout,
        stderr: r.stderr,
    })
}

fn leer_hasta(mut f: impl Read, tope: usize) -> Result<(String, bool), ErrorTransporte> {
    let mut buf = Vec::with_capacity(8192);
    let mut trozo = [0u8; 8192];
    loop {
        match f.read(&mut trozo) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&trozo[..n]);
                if buf.len() > tope {
                    return Ok((String::from_utf8_lossy(&buf[..tope]).into_owned(), true));
                }
            }
            Err(e) => return Err(ErrorTransporte::NoSePudoLanzar(e)),
        }
    }
    // 'lossy' y no un error: un byte que no es UTF-8 en el stderr de un servidor
    // no debe impedir enseñar el resto del mensaje. En stdout, el parser de JSON
    // se quejará por su cuenta, que es donde toca.
    Ok((String::from_utf8_lossy(&buf).into_owned(), false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(stdout: &str) -> Respuesta {
        Respuesta {
            stdout: stdout.into(),
            stderr: String::new(),
            codigo: 0,
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
                    progreso: false
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
