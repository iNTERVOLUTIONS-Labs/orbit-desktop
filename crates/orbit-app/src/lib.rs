//! El envoltorio de escritorio: la ventana y el puente hacia el núcleo.
//!
//! Esta caja es **delgada a propósito**. Declara los comandos que la interfaz
//! puede invocar y no hace nada más: el transporte, el contrato y el escapado
//! viven en `orbit-client`, que no sabe que Tauri existe.
//!
//! Eso no es orden por orden. Es lo que hace que:
//!
//!  · la lógica se pruebe con `cargo test` sin levantar una ventana,
//!  · un cambio de envoltorio —o de framework de interfaz— no la toque,
//!  · y el catálogo de lo que la interfaz puede pedirle al servidor siga
//!    siendo **una lista finita en un fichero**, que es lo que hace auditable
//!    la regla nº 1: la interfaz sólo invoca `orbit`.
//!
//! Y hay una frontera de seguridad además de una de diseño. En Tauri el
//! renderizador **no tiene Node**: no hay `fs`, ni `child_process`, ni forma de
//! tocar el disco. Todo lo que la interfaz puede hacer pasa por los comandos
//! declarados aquí abajo. Si mañana una dependencia de npm se compromete —y
//! pasa cada pocos meses—, en Electron podría leer `~/.ssh/id_ed25519`; aquí no
//! puede, porque no hay API para eso salvo la que se le dé.
//!
//! **La lista de comandos de este fichero ES la superficie.** Si crece, crece
//! la superficie, y hay que verlo en un diff.

use orbit_client::comando::{
    AjustesDeteccion, Anulacion, Comando, ModoDeExec, Secreto, VariableNueva, WebNueva,
};
use orbit_client::descubrir;
use orbit_client::{instalar, registro};
// `dir_control` viene del núcleo: dónde viven los sockets de ControlMaster
// es política del transporte, y compartirlo con el cliente de terminal es
// justo lo que hace que el segundo reutilice la conexión del primero.
use orbit_client::contrato::Version;
use orbit_client::transporte::{self, dir_control, EnCurso, Servidor};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Un fallo, ya en palabras que se le pueden enseñar a alguien.
///
/// Se traduce aquí y no en la interfaz porque los mensajes son parte del
/// contrato con el usuario, y repartirlos por las pantallas es cómo acaban
/// diciendo cosas distintas de lo mismo.
#[derive(serde::Serialize)]
pub struct ErrorParaLaInterfaz {
    /// Un identificador estable, para que la interfaz pueda decidir **sin
    /// leer el texto**: enseñar la pantalla bloqueante de la clave de host es
    /// una decisión, y tomarla comparando una cadena traducida es cómo se
    /// rompe al traducirla.
    pub clase: &'static str,
    pub mensaje: String,
    /// El detalle crudo, para poder copiarlo. En la clave de host cambiada,
    /// aquí va la huella y la línea del `known_hosts` que hay que quitar.
    pub detalle: Option<String>,
}

impl From<transporte::ErrorTransporte> for ErrorParaLaInterfaz {
    fn from(e: transporte::ErrorTransporte) -> Self {
        use transporte::ErrorTransporte as E;
        let clase = match &e {
            E::Forma(_) => "forma",
            E::NoSePudoLanzar(_) => "no-se-pudo-lanzar",
            E::NoLlego { .. } => "no-llego",
            E::ClaveDeHostCambiada { .. } => "clave-de-host-cambiada",
            E::OrbitNoEsta { .. } => "orbit-no-esta",
            E::SudoPideClave => "sudo-pide-clave",
            E::Orbit { .. } => "orbit",
            E::Demasiado { .. } => "demasiado",
            E::Tarde { .. } => "tarde",
            E::RespuestaSucia(_) => "respuesta-sucia",
            E::Json(_) => "json",
        };
        let detalle = match &e {
            E::ClaveDeHostCambiada { detalle } => Some(detalle.clone()),
            E::Orbit { stderr, .. } | E::NoLlego { stderr, .. } => Some(stderr.clone()),
            _ => None,
        };
        Self {
            clase,
            mensaje: e.to_string(),
            detalle,
        }
    }
}

type Resultado = Result<serde_json::Value, ErrorParaLaInterfaz>;

/// Un servidor que SÓLO puede estar en el `~/.ssh/config`.
///
/// No se llama directamente desde ninguna orden: la puerta es
/// [`servidor_por_alias`], que mira además la lista propia. Un alias añadido a
/// mano que pasara por aquí acabaría en un `ssh alias` que no resuelve nada, y
/// el fallo diría «no he llegado al servidor» — que manda a buscar el problema
/// a la red cuando está en el cliente.
///
/// **Y sólo si es un `Host` del fichero.** Antes valía cualquier nombre que no
/// empezara por guion, y `user@atacante.example` también lo es: un renderizador
/// comprometido podía hacer que `ssh` le ofreciera las claves del agente a un
/// host cualquiera. La lista es la misma que ve la interfaz
/// (`descubrir::alias_de`), así que no se pierde ningún servidor que se pueda
/// elegir; uno añadido a mano entra por [`guardado`], antes de llegar aquí.
fn servidor_del_config(
    alias: &str,
    binario: Option<String>,
) -> Result<Servidor, ErrorParaLaInterfaz> {
    let del_config = match descubrir::ruta_por_defecto() {
        Some(r) => descubrir::alias_de(&r).map_err(|e| ErrorParaLaInterfaz {
            clase: "registro",
            mensaje: format!("no he podido leer tu ~/.ssh/config para buscar «{alias}»: {e}"),
            detalle: None,
        })?,
        None => Vec::new(),
    };
    servidor_si_esta(alias, binario, &del_config)
}

fn servidor_si_esta(
    alias: &str,
    binario: Option<String>,
    del_config: &[String],
) -> Result<Servidor, ErrorParaLaInterfaz> {
    if !del_config.iter().any(|a| a == alias) {
        return Err(ErrorParaLaInterfaz {
            clase: "alias-desconocido",
            mensaje: format!(
                "«{alias}» no es ninguno de tus servidores: ni lo has añadido aquí ni es un Host \
                 de tu ~/.ssh/config"
            ),
            detalle: None,
        });
    }
    let mut s = Servidor::nuevo(alias, alias);
    if let Some(b) = binario {
        s.binario = b;
    }
    Ok(s)
}

/// Un comando cualquiera del catálogo, ejecutado y devuelto **sin interpretar**.
///
/// La interfaz recibe el objeto tal cual llegó del servidor. No se aplana, no
/// se renombra y no se rellena nada: los tipos ya están en `contrato.rs` y en
/// su gemelo de TypeScript, y meter aquí una tercera forma sería inventar un
/// tercer contrato.
///
/// Sin interpretar no es sin comprobar: pasa por `objeto_del_contrato`, que
/// mira el `schema` como lo mira el terminal. Antes no se miraba, y un servidor
/// con `schema:2` se pintaba como si hablara el 1.
fn pedir(alias: &str, binario: Option<String>, c: Comando) -> Resultado {
    let s = servidor_por_alias(alias, binario)?;
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(r.objeto_del_contrato(&c)?)
}

/// Lo bloqueante, **fuera del hilo de la ventana**.
///
/// Un `#[tauri::command]` sin `async` corre en el hilo principal, que es el que
/// pinta la ventana y atiende al ratón. La primera versión de este fichero
/// tenía así casi todo el catálogo, y eso tenía dos consecuencias que no se
/// veían contra el servidor falso, que contesta en milisegundos:
///
///  · **La ventana se congelaba mientras esperaba al servidor.** `doctor` son
///    1,4 s con 40 apps y puede llegar a dos minutos con un resolutor lento;
///    `top`, 2,1 s cada vez que el monitor refresca; un saludo sin
///    multiplexar, un cuarto de segundo. Todo ese tiempo, ni un clic.
///  · **Las peticiones iban en fila**, aunque la interfaz las lanzara a la vez:
///    el `Promise.all` del tráfico y las métricas esperaba la suma de las dos,
///    no la más lenta. Con el multiplexado, dos órdenes al mismo servidor
///    comparten la conexión y pueden ir en paralelo de verdad.
///
/// Se usa `spawn_blocking` y no `async` a secas porque el transporte bloquea
/// —espera a un proceso `ssh`— y un hilo del runtime asíncrono bloqueado es
/// otra forma de la misma fila.
async fn aparte<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ErrorParaLaInterfaz> + Send + 'static,
) -> Result<T, ErrorParaLaInterfaz> {
    match tauri::async_runtime::spawn_blocking(f).await {
        Ok(r) => r,
        Err(e) => Err(ErrorParaLaInterfaz {
            clase: "hilo",
            mensaje: format!("la petición se ha interrumpido: {e}"),
            detalle: None,
        }),
    }
}

/// [`pedir`], fuera del hilo de la ventana.
async fn pedir_aparte(alias: String, binario: Option<String>, c: Comando) -> Resultado {
    aparte(move || pedir(&alias, binario, c)).await
}

/// La prosa de una orden sin `--json`, entera.
///
/// Sin `--json`, Orbit manda a **stdout** lo que le cuenta a una persona
/// (`_ui_route`) y a stderr sólo los avisos y los errores. La primera versión
/// de `retirar` y `revertir` devolvía sólo stderr, así que de un `remove` que
/// salía bien llegaba una cadena vacía: la confirmación de lo que se había
/// hecho se perdía justo en las dos órdenes que cambian algo.
fn prosa(r: &transporte::Respuesta) -> String {
    let mut s = r.stdout.trim_end().to_string();
    let e = r.stderr.trim_end();
    if !e.is_empty() {
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str(e);
    }
    s
}

/// Una orden del catálogo que contesta en prosa —`remove`, `rollback`, `lock`—,
/// devuelta como `{salida, codigo}`. Si sale con error, el error es la frase de
/// Orbit (ver `ErrorTransporte::Orbit`), no «ha salido con 1».
fn pedir_prosa(alias: &str, binario: Option<String>, c: Comando) -> Resultado {
    let s = servidor_por_alias(alias, binario)?;
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(serde_json::json!({ "salida": prosa(&r), "codigo": r.codigo }))
}

// ── el catálogo, y nada más ────────────────────────────────────────────────

/// `version --json`. Lo que conteste se apunta en [`Versiones`].
#[tauri::command]
async fn version(
    versiones: tauri::State<'_, Versiones>,
    alias: String,
    binario: Option<String>,
) -> Resultado {
    let cache = versiones.inner().clone();
    aparte(move || {
        let s = servidor_por_alias(&alias, binario)?;
        let c = Comando::Version;
        let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
        let v = r.objeto_del_contrato(&c)?;
        match serde_json::from_value::<Version>(v.clone()) {
            Ok(x) => cache.apuntar(&s, &x),
            Err(_) => cache.olvidar(&s),
        }
        Ok(v)
    })
    .await
}

/// La portada se alimenta de **esto** y no de `list`.
///
/// `status --json` trae el array de apps completo e **idéntico** al de
/// `list --json` —comprobado comparando los dos objetos— así que una llamada de
/// 389 ms sustituye a dos que suman 695. Un 44 % menos en la carga que forma la
/// primera impresión, y no cuesta nada: es elegir bien qué comando se pide.
#[tauri::command]
async fn portada(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Estado).await
}

#[tauri::command]
async fn lista(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Lista).await
}

#[tauri::command]
async fn info(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Info { app }).await
}

#[tauri::command]
async fn doctor(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Doctor).await
}

/// El log de una app, **crudo**.
///
/// Devuelve el NDJSON tal cual y no una lista ya parseada, y es a propósito:
/// `logs` es la única excepción del contrato —un flujo, no un objeto— y
/// convertirlo aquí obligaría a inventar una tercera forma entre el servidor y
/// la interfaz. El lector vive en `flujo.rs` y tiene su gemelo en TypeScript,
/// que son dos y no tres.
#[tauri::command]
async fn logs(
    alias: String,
    app: String,
    desde: Option<String>,
    binario: Option<String>,
) -> Result<String, ErrorParaLaInterfaz> {
    aparte(move || {
        let s = servidor_por_alias(&alias, binario)?;
        let c = Comando::Logs {
            app,
            desde,
            lineas: Some(500),
            // Nunca en vivo por este camino: un flujo que no termina no se
            // puede devolver de una llamada que sí. La pantalla en vivo es
            // otra cosa.
            seguir: false,
            solo_nginx: false,
        };
        let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
        Ok(r.stdout)
    })
    .await
}

/// Aplica lo que el servidor sabe arreglar sin decidir nada por nadie.
///
/// Existe desde que `orbit doctor --fix --json --yes` funciona. Estuvo
/// documentado y muerto durante versiones —`--yes` no era una bandera global y
/// no había forma de dársela— así que contra un servidor más viejo esto falla,
/// y la interfaz vuelve a enseñar la orden para copiar. Por eso la capacidad se
/// **pregunta** y no se supone.
#[tauri::command]
async fn doctor_arreglar(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::DoctorArreglar).await
}

/// Los despliegues vivos, para poder cancelarlos.
///
/// Se pueden lanzar **varios a la vez**, en apps distintas y en servidores
/// distintos: cada uno es un proceso SSH independiente y nada en el servidor
/// los coordina, ni falta. La clave es `servidor:app` y no la app sola, porque
/// `tienda` existe en tres servidores y son tres despliegues distintos.
///
/// Lo que **no** se puede es lanzar dos con la misma clave. Antes el segundo
/// pisaba al primero en el registro: un doble clic en «Desplegar» eran dos
/// despliegues de la misma app —dos builds peleándose por la caché de git, lo
/// que `ejecutar_en_vivo` se niega a hacer reintentando—, el primero se quedaba
/// sin forma de cancelarse, y al terminar borraba del registro el mando del
/// segundo, que tampoco se podía cancelar ya.
#[derive(Default)]
pub struct Vivos(Mutex<HashMap<String, EnCurso>>);

impl Vivos {
    /// Apunta un trabajo como vivo, **o se niega si ya hay uno con esa clave**.
    ///
    /// La comprobación y el apunte van bajo el mismo cerrojo: mirar y luego
    /// apuntar en dos pasos dejaría pasar justo el doble clic.
    fn apuntar(&self, clave: String, ocupado: String) -> Result<Turno<'_>, ErrorParaLaInterfaz> {
        use std::collections::hash_map::Entry;
        let mut m = self.0.lock().unwrap();
        match m.entry(clave.clone()) {
            Entry::Occupied(_) => Err(ErrorParaLaInterfaz {
                clase: "en-curso",
                mensaje: ocupado,
                detalle: None,
            }),
            Entry::Vacant(v) => {
                let mando = EnCurso::nuevo();
                v.insert(mando.clone());
                Ok(Turno {
                    vivos: self,
                    clave,
                    mando,
                })
            }
        }
    }
}

/// Un trabajo apuntado en [`Vivos`]. Se desapunta **solo, al soltarse**, pase
/// lo que pase en medio: con un `remove` a mano al final, cada `?` entre medias
/// era un despliegue que seguía «vivo» en el registro para siempre.
struct Turno<'a> {
    vivos: &'a Vivos,
    clave: String,
    mando: EnCurso,
}

impl Drop for Turno<'_> {
    fn drop(&mut self) {
        // Sólo si sigue siendo el suyo. Hoy no puede no serlo —`apuntar` no
        // deja apuntar otro con la misma clave—, pero quitar «el que haya» es
        // exactamente el fallo que dejaba a un despliegue sin poder cancelarse.
        if let Ok(mut m) = self.vivos.0.lock() {
            if m.get(&self.clave)
                .is_some_and(|x| x.es_el_mismo(&self.mando))
            {
                m.remove(&self.clave);
            }
        }
    }
}

// ── el progreso, por tandas ────────────────────────────────────────────────

/// El evento por el que llega el progreso: `(clave, líneas)`.
///
/// `clave` es `servidor:app` (o `servidor:*`, `servidor:!instalar`) y `líneas`
/// van en el orden en que llegaron. En la interfaz es `[string, string[]]`.
pub const EVENTO_DE_PROGRESO: &str = "orbit://progreso";

/// Lo que puede esperar una línea antes de salir. Menos de lo que se nota, y
/// basta para que un build hablador vaya en tandas de decenas.
const TANDA_ESPERA: Duration = Duration::from_millis(50);
/// Y lo que cabe en una antes de salir sin esperar.
const TANDA_LINEAS: usize = 64;

/// Las líneas de progreso, **en tandas**.
///
/// Iba un evento por línea, y cada evento es un despacho al webview que
/// serializa, cruza el IPC y despierta a los oyentes: un build de 30 000 líneas
/// eran 30 000 despachos. Ahora sale una tanda cada 50 ms como mucho, o en
/// cuanto se juntan 64 líneas, y siempre una al final con lo que quede.
///
/// Un hilo propio y no «vaciar al llegar la siguiente»: con eso, la última
/// línea de un servidor que se queda callado a mitad de build —el paso largo,
/// justo el que se mira— no saldría hasta la siguiente, minutos después. El
/// hilo vacía por plazo aunque no llegue nada más.
///
/// Sólo emite el hilo, así que el orden es el de llegada y no se pierde nada:
/// lo que se echa después de cerrar no existe, porque se cierra al terminar la
/// orden, cuando ya no hay quien eche.
struct Tanda {
    buzon: Buzon,
    hilo: Option<std::thread::JoinHandle<()>>,
}

struct Pendiente {
    lineas: Vec<String>,
    desde: Option<Instant>,
    cerrada: bool,
}

#[derive(Clone)]
struct Buzon(Arc<(Mutex<Pendiente>, Condvar)>);

impl Buzon {
    fn pendiente(&self) -> std::sync::MutexGuard<'_, Pendiente> {
        // Un pánico en el emisor no puede dejar mudo el resto del despliegue.
        self.0 .0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn echar(&self, linea: String) {
        let mut p = self.pendiente();
        if p.lineas.is_empty() {
            p.desde = Some(Instant::now());
        }
        p.lineas.push(linea);
        // Sólo se despierta al hilo cuando hay algo que decidir: la primera
        // línea pone el plazo en marcha y la 64.ª lo adelanta.
        let avisar = p.lineas.len() == 1 || p.lineas.len() >= TANDA_LINEAS;
        drop(p);
        if avisar {
            self.0 .1.notify_one();
        }
    }
}

impl Tanda {
    fn nueva(mut emitir: impl FnMut(Vec<String>) + Send + 'static) -> Self {
        let buzon = Buzon(Arc::new((
            Mutex::new(Pendiente {
                lineas: Vec::new(),
                desde: None,
                cerrada: false,
            }),
            Condvar::new(),
        )));
        let b = buzon.clone();
        let hilo = std::thread::spawn(move || loop {
            let cv = &b.0 .1;
            let mut p = b.pendiente();
            while p.lineas.is_empty() && !p.cerrada {
                p = cv.wait(p).unwrap_or_else(|e| e.into_inner());
            }
            if p.lineas.is_empty() {
                return;
            }
            let limite = p.desde.unwrap_or_else(Instant::now) + TANDA_ESPERA;
            while !p.cerrada && p.lineas.len() < TANDA_LINEAS {
                let ahora = Instant::now();
                if ahora >= limite {
                    break;
                }
                p = cv
                    .wait_timeout(p, limite - ahora)
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            let lote = std::mem::take(&mut p.lineas);
            p.desde = None;
            // Se emite sin el cerrojo: el lector de stderr no espera al IPC.
            drop(p);
            emitir(lote);
        });
        Tanda {
            buzon,
            hilo: Some(hilo),
        }
    }

    fn buzon(&self) -> Buzon {
        self.buzon.clone()
    }
}

impl Drop for Tanda {
    /// Vacía lo que quede y espera a que salga, **antes** de que la orden
    /// devuelva: la interfaz tiene que tener la última línea antes que el
    /// resultado, o pintaría el final sin el paso que lo explica.
    fn drop(&mut self) {
        self.buzon.pendiente().cerrada = true;
        self.buzon.0 .1.notify_all();
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

/// Las tandas de una clave, hacia la ventana.
fn tanda_hacia(ventana: tauri::Window, clave: String) -> Tanda {
    Tanda::nueva(move |lineas| {
        // Con varios despliegues a la vez, una línea sin dueño no vale para
        // nada: por eso cada tanda lleva su clave.
        let _ = ventana.emit(EVENTO_DE_PROGRESO, (clave.as_str(), lineas));
    })
}

// ── la versión de cada servidor, recordada un rato ─────────────────────────

/// Cuánto vale una versión leída. Basta para que desplegar varias apps con
/// variables no le pregunte al servidor antes de cada una, y es corto porque
/// el servidor se puede actualizar por fuera (`orbit update`, el terminal).
const VIDA_DE_LA_VERSION: Duration = Duration::from_secs(5 * 60);

/// La última versión leída de cada servidor.
///
/// Existe por `desplegar` con variables, que necesita la versión para decidir
/// por dónde viajan los valores y la preguntaba **cada vez**: un saludo más
/// por despliegue. Se apunta cada vez que se lee una respuesta de `version`
/// —el saludo, la de la interfaz, la de después de instalar— y se olvida al
/// instalar o cuando el saludo no da una versión.
///
/// La clave es servidor **y** binario: dos rutas de `orbit` en la misma máquina
/// pueden ser dos versiones.
///
/// Lo que se acepta: dentro de esos cinco minutos, un servidor actualizado por
/// fuera de 1.4.8 a 1.4.9 recibe las variables en el `argv` —como antes de
/// actualizar—, y uno que bajara de versión rechazaría `--env-stdin` con un
/// error que lo dice. Ninguno de los dos manda un valor a donde no debe.
#[derive(Clone, Default)]
pub struct Versiones(Arc<Mutex<HashMap<ServidorYBinario, (Version, Instant)>>>);

/// El alias y la ruta de `orbit`.
type ServidorYBinario = (String, String);

impl Versiones {
    fn clave(s: &Servidor) -> ServidorYBinario {
        (s.alias.clone(), s.binario.clone())
    }

    fn apuntar(&self, s: &Servidor, v: &Version) {
        self.apuntar_en(s, v, Instant::now());
    }

    fn apuntar_en(&self, s: &Servidor, v: &Version, cuando: Instant) {
        if let Ok(mut m) = self.0.lock() {
            m.insert(Self::clave(s), (v.clone(), cuando));
        }
    }

    fn olvidar(&self, s: &Servidor) {
        if let Ok(mut m) = self.0.lock() {
            m.remove(&Self::clave(s));
        }
    }

    fn vigente(&self, s: &Servidor) -> Option<Version> {
        self.vigente_en(s, Instant::now())
    }

    fn vigente_en(&self, s: &Servidor, ahora: Instant) -> Option<Version> {
        let m = self.0.lock().ok()?;
        let (v, cuando) = m.get(&Self::clave(s))?;
        (ahora.saturating_duration_since(*cuando) < VIDA_DE_LA_VERSION).then(|| v.clone())
    }
}

/// Despliega **sirviendo el progreso mientras ocurre**.
///
/// El progreso de `--progress` sale por `orbit://progreso` en tandas de como
/// mucho 50 ms ([`Tanda`]). Leerlo entero al terminar convertiría tres minutos
/// de información en un bloque de texto que llega cuando ya no sirve.
///
/// Lo que **no** hace, y es lo que importa: si el contacto se pierde, no dice
/// que el despliegue haya fallado. **No lo sabe.** El proceso sigue en el
/// servidor y la respuesta honesta es «he perdido el contacto; el estado es
/// desconocido», con `orbit info` como forma de averiguarlo. Y no reintenta
/// solo: un `deploy` reintentado sobre uno en curso es, en el mejor caso, dos
/// releases.
///
/// `entorno` es lo que se pone en el `.env` **antes del build**, con
/// `deploy --env`. Es la forma de dar una variable obligatoria sin terminal: sin
/// ella el servidor se para antes de compilar, porque con `--json` no tiene a
/// quién preguntar. Los valores cruzan el puente una vez, se convierten en
/// [`Secreto`] —que no se deja imprimir— y no se guardan en ningún sitio.
///
/// **Por dónde viajan los valores lo decide esto, no la interfaz.** Con Orbit
/// 1.4.9 o posterior van por la entrada estándar (`--env-stdin`) y no tocan ni
/// `ps` ni el registro de `sudo` del servidor; con 1.4.3–1.4.8 sólo existe
/// `--env`, que los pone en el `argv` (THREAT-MODEL T-13, y la hoja lo avisa).
/// La versión sale de la que este proceso leyó hace menos de cinco minutos
/// ([`Versiones`]) o, si no hay, se le pregunta al servidor justo antes —una
/// llamada corta, por la conexión multiplexada—; nunca de la que tenga la
/// interfaz, que puede ser de antes de una actualización. Y si no se puede
/// saber, **no se manda nada**: adivinar mal aquí es poner una contraseña en el
/// `argv` de un servidor que sí sabía leerla por la entrada.
///
/// `forzar` es `--force`: desplegar una app congelada **sin descongelarla**.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn desplegar(
    ventana: tauri::Window,
    vivos: tauri::State<'_, Vivos>,
    versiones: tauri::State<'_, Versiones>,
    alias: String,
    app: String,
    entorno: Option<Vec<VariableDeLaInterfaz>>,
    forzar: Option<bool>,
    binario: Option<String>,
) -> Resultado {
    let clave = format!("{alias}:{app}");

    let s = servidor_por_alias(&alias, binario)?;
    let variables: Vec<VariableNueva> = entorno
        .unwrap_or_default()
        .into_iter()
        .map(VariableNueva::from)
        .collect();
    let por_entrada = if variables.is_empty() {
        false
    } else {
        let s_v = s.clone();
        let cache = versiones.inner().clone();
        let v = aparte(move || {
            if let Some(v) = cache.vigente(&s_v) {
                return Ok(v);
            }
            let r = transporte::ejecutar(&s_v, &Comando::Version, dir_control().as_deref(), &[])?;
            let v = r.leer::<Version>().map_err(ErrorParaLaInterfaz::from)?;
            cache.apuntar(&s_v, &v);
            Ok(v)
        })
        .await
        .map_err(|e| ErrorParaLaInterfaz {
            clase: "sin-version",
            mensaje: format!(
                "no he podido saber qué versión de Orbit tiene el servidor, y sin saberlo no \
                 mando las variables: según la versión irían por la entrada o en la orden. {}",
                e.mensaje
            ),
            detalle: e.detalle,
        })?;
        variables_por_la_entrada(&v).ok_or_else(|| ErrorParaLaInterfaz {
            clase: "sin-version",
            mensaje: format!(
                "el servidor dice tener Orbit «{}», que no sé leer, y sin saber la versión \
                 no mando las variables: según cuál sea irían por la entrada o en la orden",
                v.version
            ),
            detalle: None,
        })?
    };
    let c = Comando::Desplegar {
        app: app.clone(),
        progreso: true,
        entorno: variables,
        forzar: forzar.unwrap_or(false),
        por_entrada,
    };
    // Se valida ANTES de registrar el despliegue como vivo: una variable que el
    // servidor rechazaría tiene que decirse aquí, con su nombre, y no dejar un
    // despliegue fantasma en el rail.
    c.argv(&s.binario).map_err(ErrorParaLaInterfaz::from)?;

    let turno = vivos.apuntar(
        clave.clone(),
        format!("Ya hay un despliegue de «{app}» en curso en «{alias}»."),
    )?;
    let mando = turno.mando.clone();
    let ctrl = dir_control();

    // En un hilo aparte: la llamada bloquea minutos, y bloquear el hilo de la
    // interfaz durante un build es la diferencia entre una ventana viva y una
    // que Windows marca como «no responde».
    let r = tauri::async_runtime::spawn_blocking(move || {
        let tanda = tanda_hacia(ventana, clave);
        let buzon = tanda.buzon();
        let r = transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |l| {
            buzon.echar(l)
        });
        // Lo que quede sale ANTES del resultado.
        drop(tanda);
        r.and_then(|r| r.objeto_del_contrato(&c))
    })
    .await;

    drop(turno);

    match r {
        Ok(v) => Ok(v?),
        Err(e) => Err(ErrorParaLaInterfaz {
            clase: "hilo",
            mensaje: format!("el despliegue se ha interrumpido: {e}"),
            detalle: None,
        }),
    }
}

/// Si las variables de un despliegue van por la entrada estándar (`Some(true)`),
/// por el `argv` (`Some(false)`, 1.4.3–1.4.8), o si no se sabe (`None`: una
/// versión que no es semver). En el último caso no se manda nada: tratarla como
/// vieja pondría la contraseña en el `argv` de un servidor que quizá sí sabía
/// leerla por la entrada.
fn variables_por_la_entrada(v: &orbit_client::contrato::Version) -> Option<bool> {
    v.semver()?;
    Some(v.sabe(orbit_client::contrato::Capacidad::EntornoPorEntrada))
}

/// Una variable de `deploy --env`, tal como la manda el formulario.
///
/// **Sin `Debug`, a propósito**: `valor` es casi siempre una contraseña, y un
/// `{:?}` de esto en un mensaje de error la sacaría. Se convierte en
/// [`VariableNueva`] en cuanto cruza, y ahí el valor ya es un [`Secreto`].
#[derive(serde::Deserialize)]
pub struct VariableDeLaInterfaz {
    pub clave: String,
    pub valor: String,
}

impl From<VariableDeLaInterfaz> for VariableNueva {
    fn from(v: VariableDeLaInterfaz) -> Self {
        VariableNueva {
            clave: v.clave,
            valor: Secreto::nuevo(v.valor),
        }
    }
}

/// Crea una web, sirviendo la prosa mientras ocurre.
///
/// Tres cosas la separan del resto del puente, y las tres salen de leer
/// `cmd_new` en vez de suponerlo:
///
/// 1. **No devuelve un objeto.** `orbit new` no tiene `--json`, así que aquí se
///    devuelve el texto tal cual y el estado real lo pregunta la interfaz
///    después con `info --json`. Analizar la prosa sería atarse a unas frases
///    en castellano que pueden cambiar en cualquier versión.
/// 2. **Su código de salida no es informativo.** `new` despliega por dentro, y
///    puede terminar en 1 con la aplicación creada, registrada y con vhost. Por
///    eso un `rc` distinto de cero **no se convierte en error**: se devuelve
///    con la salida y quien decide es la pantalla, que va a preguntar de todas
///    formas.
/// 3. **Su prosa sale por stdout**, no por stderr como la del resto. Eso no se
///    decide aquí: lo decide `Comando::vena_humana`, a partir de si la orden
///    lleva `--json`.
///
/// Lo que sí sigue siendo un error es no llegar al servidor, o que la clave de
/// host haya cambiado: eso no es «new ha terminado regular», es que no ha
/// llegado a ejecutarse nada.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn crear(
    ventana: tauri::Window,
    vivos: tauri::State<'_, Vivos>,
    alias: String,
    nombre: String,
    repo: String,
    rama: String,
    dominio: String,
    alias_dominio: Vec<String>,
    correo: Option<String>,
    base_de_datos: bool,
    https: bool,
    ajustes: AjustesParaLaInterfaz,
    binario: Option<String>,
) -> Result<Creacion, ErrorParaLaInterfaz> {
    // Misma clave que un despliegue —`servidor:app`— porque es cancelable por
    // la misma vía y porque durante `new` no puede haber además un `deploy` de
    // la misma app: todavía no existe.
    let clave = format!("{alias}:{nombre}");
    let s = servidor_por_alias(&alias, binario)?;
    let turno = vivos.apuntar(
        clave.clone(),
        format!("Ya hay un despliegue de «{nombre}» en curso en «{alias}»."),
    )?;
    let mando = turno.mando.clone();

    let c = Comando::Nueva(Box::new(WebNueva {
        nombre: nombre.clone(),
        repo,
        rama,
        dominio,
        alias: alias_dominio,
        correo,
        base_de_datos,
        https,
        ajustes: ajustes.into(),
    }));
    let ctrl = dir_control();

    let r = tauri::async_runtime::spawn_blocking(move || {
        let tanda = tanda_hacia(ventana, clave);
        let buzon = tanda.buzon();
        let r = transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |l| {
            buzon.echar(l)
        });
        drop(tanda);
        r
    })
    .await;

    drop(turno);

    match r {
        Ok(Ok(r)) => Ok(Creacion {
            codigo: r.codigo,
            salida: r.stdout,
        }),
        // Un `rc` distinto de cero sin objeto JSON llega aquí como
        // `ErrorTransporte::Orbit`, y para el resto del catálogo eso es
        // exactamente lo que es: un fallo. Para `new` no: es el caso normal de
        // una web creada a la que le falta el certificado.
        Ok(Err(transporte::ErrorTransporte::Orbit { codigo, stdout, .. })) => Ok(Creacion {
            codigo,
            salida: stdout,
        }),
        Ok(Err(e)) => Err(e.into()),
        Err(e) => Err(ErrorParaLaInterfaz {
            clase: "hilo",
            mensaje: format!("la creación se ha interrumpido: {e}"),
            detalle: None,
        }),
    }
}

/// Lo que devuelve `crear`: el texto y el código, **sin interpretar ninguno de
/// los dos**.
#[derive(serde::Serialize)]
pub struct Creacion {
    pub codigo: i32,
    pub salida: String,
}

/// Una anulación tal como cruza el puente: **etiquetada**, no implícita.
///
/// La primera versión de esto era un `Option<Option<String>>`, con la idea de
/// que el campo ausente fuese «callar» y un `null` explícito fuese «vacío a
/// propósito». No funciona, y la prueba de abajo enseña por qué: serde colapsa
/// las dos cosas en el mismo `None`. O sea que el «no se compila» no llegaba
/// nunca al servidor **y el repaso habría enseñado una orden que no era la que
/// se ejecutaba**, que es justo el fallo que el repaso existe para hacer
/// imposible.
///
/// Así que la etiqueta se escribe. Es el mismo tipo que ya tiene la interfaz
/// —una unión discriminada por `modo`— y cruza tal cual, sin que nadie tenga
/// que acordarse de qué significa la ausencia de un campo.
#[derive(serde::Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(tag = "modo", rename_all = "lowercase")]
pub enum AnulacionDeLaInterfaz {
    #[default]
    Detectar,
    Vacia,
    Valor {
        valor: String,
    },
}

impl From<AnulacionDeLaInterfaz> for Option<Anulacion> {
    fn from(a: AnulacionDeLaInterfaz) -> Self {
        match a {
            AnulacionDeLaInterfaz::Detectar => None,
            AnulacionDeLaInterfaz::Vacia => Some(Anulacion::Vacia),
            AnulacionDeLaInterfaz::Valor { valor } => Some(Anulacion::Valor(valor)),
        }
    }
}

/// Los ajustes de detección tal como los manda la interfaz.
#[derive(serde::Deserialize, Default, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct AjustesParaLaInterfaz {
    pub carpeta: Option<String>,
    pub tipo: Option<String>,
    pub build: AnulacionDeLaInterfaz,
    pub arranque: AnulacionDeLaInterfaz,
    pub outdir: AnulacionDeLaInterfaz,
}

impl From<AjustesParaLaInterfaz> for AjustesDeteccion {
    fn from(a: AjustesParaLaInterfaz) -> Self {
        AjustesDeteccion {
            carpeta: a.carpeta,
            tipo: a.tipo,
            build: a.build.into(),
            arranque: a.arranque.into(),
            outdir: a.outdir.into(),
        }
    }
}

/// Una pasada por todas las apps del servidor, sirviendo el progreso.
///
/// La clave del registro de cancelables es `alias:*`, y el `*` no es una
/// convención frágil: **ningún nombre de app puede empezar por `*`** —la regla
/// del servidor es `^[a-z0-9][a-z0-9._-]{0,39}$`— así que no puede chocar con
/// la clave de un despliegue suelto. Que se pueda cancelar importa aquí más que
/// en ningún otro sitio: una pasada sobre cuarenta apps es lo más largo que
/// este cliente puede lanzar.
///
/// Y cancelar una pasada **no deshace lo ya desplegado**: para las apps que ya
/// terminaron, terminaron. Lo que para es el bucle. Eso lo tiene que decir la
/// pantalla, porque es lo contrario de lo que sugiere la palabra «cancelar».
#[tauri::command]
async fn desplegar_todo(
    ventana: tauri::Window,
    vivos: tauri::State<'_, Vivos>,
    alias: String,
    solo_si_cambia: bool,
    binario: Option<String>,
) -> Resultado {
    let clave = format!("{alias}:*");
    let s = servidor_por_alias(&alias, binario)?;
    let turno = vivos.apuntar(
        clave.clone(),
        format!("Ya hay una pasada por todas las apps en curso en «{alias}»."),
    )?;
    let mando = turno.mando.clone();

    let c = Comando::DesplegarTodo {
        progreso: true,
        solo_si_cambia,
    };
    let ctrl = dir_control();

    let r = tauri::async_runtime::spawn_blocking(move || {
        let tanda = tanda_hacia(ventana, clave);
        let buzon = tanda.buzon();
        let r = transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |l| {
            buzon.echar(l)
        });
        drop(tanda);
        r.and_then(|r| r.objeto_del_contrato(&c))
    })
    .await;

    drop(turno);

    match r {
        Ok(v) => Ok(v?),
        Err(e) => Err(ErrorParaLaInterfaz {
            clase: "hilo",
            mensaje: format!("la pasada se ha interrumpido: {e}"),
            detalle: None,
        }),
    }
}

/// A dónde apunta un nombre, y a dónde apunta el servidor.
///
/// Existe para responder **antes de crear** la única pregunta del asistente que
/// no se puede contestar desde el contrato: una web perfectamente publicada
/// cuyo dominio no apunta a este servidor se ve, desde dentro, exactamente
/// igual que una que sí. Es el final F7, y descubrirlo después de tres minutos
/// de build es descubrirlo tarde.
///
/// Tres cosas que esto **no** es, y que la pantalla tiene que decir tal cual:
///
/// * No es una consulta de DNS de verdad. Es el resolutor del sistema, con su
///   caché y su `/etc/hosts` detrás, así que responde **lo que ve esta
///   máquina** — que puede no ser lo que ve el resto del mundo, sobre todo con
///   un registro recién cambiado.
/// * No mira los registros, mira las direcciones. No distingue un `A` de un
///   `CNAME` que acabe en el sitio correcto, y para lo que se pregunta aquí eso
///   da igual.
/// * No prueba nada sobre el servidor. Compara dos listas de direcciones; si el
///   servidor está detrás de un proxy o de un CDN, no coincidirán y **no será
///   un error**. Por eso el resultado se enseña como un aviso y nunca impide
///   seguir.
///
/// Va en un hilo aparte porque resolver un nombre bloquea, y bloquear el hilo
/// de la interfaz mientras alguien escribe en un campo se nota.
#[tauri::command]
async fn resolver(nombre: String, alias: String) -> Result<Resolucion, ErrorParaLaInterfaz> {
    validar_alias(&alias)?;
    aparte(move || {
        let del_dominio = direcciones(&nombre);
        // A dónde va el `ssh`. Si el servidor se añadió a mano, el host es el
        // que se guardó: preguntarle a `ssh -G` por un alias que no está en el
        // `~/.ssh/config` no falla —se inventa `hostname <alias>`—, así que la
        // respuesta era no encontrar nada o, con un dominio de búsqueda de una
        // red corporativa, las IP de otra máquina, y la pantalla avisaba de un
        // «no coincide» falso. Para los del fichero sí se le pregunta a
        // `ssh -G`, que es quien lo sabe, en vez de leer el fichero por nuestra
        // cuenta.
        let host = match guardado(&alias) {
            Some(g) => Some(g.host),
            None => descubrir::resolver(&alias, None).and_then(|a| a.hostname),
        };
        let del_servidor = host.map(|h| direcciones(&h)).unwrap_or_default();

        // «Coinciden» sólo se afirma cuando hay las dos listas y comparten algo.
        // Con cualquiera de las dos vacía la respuesta es «no lo sé», que no es
        // «no coinciden».
        let coinciden = if del_dominio.is_empty() || del_servidor.is_empty() {
            None
        } else {
            Some(del_dominio.iter().any(|d| del_servidor.contains(d)))
        };

        Ok(Resolucion {
            del_dominio,
            del_servidor,
            coinciden,
        })
    })
    .await
}

/// Las direcciones de un nombre, ordenadas y sin repetir para que dos
/// respuestas iguales se vean iguales.
fn direcciones(nombre: &str) -> Vec<String> {
    use std::net::ToSocketAddrs;
    // El puerto da igual y es obligatorio en la interfaz de `ToSocketAddrs`.
    // Una dirección literal se resuelve a sí misma, que es justo lo que hace
    // falta cuando el ~/.ssh/config lleva una IP.
    let mut v: Vec<String> = match (nombre, 443u16).to_socket_addrs() {
        Ok(it) => it.map(|s| s.ip().to_string()).collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v.dedup();
    v
}

#[derive(serde::Serialize)]
pub struct Resolucion {
    pub del_dominio: Vec<String>,
    pub del_servidor: Vec<String>,
    /// `None` es «no lo sé», y **no se pinta como «no»**: sin una de las dos
    /// listas no hay comparación que hacer.
    pub coinciden: Option<bool>,
}

// ── los servidores que se añaden a mano ────────────────────────────────────
//
// Hasta ahora salían **sólo** del `~/.ssh/config`, y eso era un error de base
// disfrazado de decisión elegante: mucha gente no tiene ese fichero, y en
// Windows casi nadie. La aplicación abría vacía y sin salida.

/// Un servidor de la lista propia, tal como cruza el puente.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ServidorParaLaInterfaz {
    pub alias: String,
    pub host: String,
    pub usuario: String,
    pub puerto: u16,
    /// La **ruta** de la clave. Nunca su contenido ni su frase de paso: eso no
    /// cruza este puente en ninguna dirección.
    pub clave: Option<String>,
    pub binario: Option<String>,
}

impl From<registro::ServidorGuardado> for ServidorParaLaInterfaz {
    fn from(s: registro::ServidorGuardado) -> Self {
        Self {
            alias: s.alias,
            host: s.host,
            usuario: s.usuario,
            puerto: s.puerto,
            clave: s.clave,
            binario: s.binario,
        }
    }
}

impl From<ServidorParaLaInterfaz> for registro::ServidorGuardado {
    fn from(s: ServidorParaLaInterfaz) -> Self {
        Self {
            alias: s.alias,
            host: s.host,
            usuario: s.usuario,
            puerto: s.puerto,
            clave: s.clave,
            binario: s.binario,
        }
    }
}

fn del_registro(e: registro::ErrorDeRegistro) -> ErrorParaLaInterfaz {
    ErrorParaLaInterfaz {
        clase: "registro",
        mensaje: e.to_string(),
        detalle: None,
    }
}

/// La lista propia. Si existe y no se puede leer **es un error**, no una lista
/// vacía: la interfaz sigue con los del `~/.ssh/config`, y nadie escribe encima
/// de un fichero que se podía arreglar (ver `registro::leer`).
#[tauri::command]
fn servidores_guardados() -> Result<Vec<ServidorParaLaInterfaz>, ErrorParaLaInterfaz> {
    Ok(registro::leer()
        .map_err(del_registro)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Guarda un servidor nuevo, fuera del hilo de la ventana.
///
/// Era síncrono y resolvía cada `Host` del `~/.ssh/config` con un `ssh -G` en
/// fila, en el hilo que pinta: con veinte alias, la ventana congelada medio
/// segundo o más al pulsar «guardar», para usar sólo los nombres. Ahora los
/// nombres se leen del fichero sin lanzar nada, y lo poco que queda va aparte.
#[tauri::command]
async fn guardar_servidor(
    servidor: ServidorParaLaInterfaz,
) -> Result<Vec<ServidorParaLaInterfaz>, ErrorParaLaInterfaz> {
    aparte(move || guardar_servidor_aqui(servidor)).await
}

fn guardar_servidor_aqui(
    servidor: ServidorParaLaInterfaz,
) -> Result<Vec<ServidorParaLaInterfaz>, ErrorParaLaInterfaz> {
    if let Some(b) = &servidor.binario {
        validar_binario(b)?;
    }
    // Los alias del `~/.ssh/config` se leen aquí y se pasan: un alias repetido
    // haría que `ssh` resolviera el del fichero y la aplicación enseñara un
    // servidor mientras habla con otro. Un fichero que existe y no se puede
    // leer **para el guardado**: con una lista vacía por error, la comprobación
    // dejaría pasar justo el choque que existe para impedir.
    let del_config: Vec<String> = match descubrir::ruta_por_defecto() {
        Some(r) => descubrir::alias_de(&r).map_err(|e| ErrorParaLaInterfaz {
            clase: "registro",
            mensaje: format!(
                "no he podido leer tu ~/.ssh/config para comprobar que «{}» no choca con \
                 ninguno de sus Host: {e}",
                servidor.alias
            ),
            detalle: None,
        })?,
        None => Vec::new(),
    };

    // Leer, añadir y escribir bajo un solo cerrojo: en tres llamadas sueltas,
    // dos guardados a la vez —un doble clic— perdían uno o fallaban con un
    // «no he podido guardar» que no era verdad.
    let lista = registro::guardar(servidor.into(), &del_config).map_err(del_registro)?;
    Ok(lista.into_iter().map(Into::into).collect())
}

#[tauri::command]
fn olvidar_servidor(alias: String) -> Result<Vec<ServidorParaLaInterfaz>, ErrorParaLaInterfaz> {
    // Si la lista no se puede leer, no se escribe: olvidar uno sobre una lista
    // leída como vacía era borrarlos todos.
    // Y bajo el mismo cerrojo que guardar, por lo mismo.
    let lista = registro::quitar(&alias).map_err(del_registro)?;
    Ok(lista.into_iter().map(Into::into).collect())
}

/// El servidor guardado a mano con ese alias, si lo hay.
///
/// Una lista que no se puede leer cuenta aquí como vacía, y es la única
/// excepción: esto sólo **lee** para conectar, y negarse dejaría sin servicio
/// también a los del `~/.ssh/config`. Escribir sí se niega (ver
/// `guardar_servidor`).
fn guardado(alias: &str) -> Option<registro::ServidorGuardado> {
    registro::leer()
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.alias == alias)
}

/// Lo que se le acepta a la interfaz como alias.
///
/// El alias acaba de destino de `ssh`, y la interfaz es la frontera que el
/// encabezado de este fichero promete defender: si una dependencia de npm se
/// compromete, lo único que puede hacer es llamar a estos comandos. Un alias
/// `-oProxyCommand=…` o `-F\\host\share\cfg` era, hasta ahora, una opción de
/// `ssh` —ejecutar una orden local— en el sitio donde iba un nombre.
///
/// El transporte ya pone `--` delante del destino, que es lo que cierra el
/// agujero; esto es la segunda capa, y además da un error que se entiende en
/// vez de un «no he llegado al servidor». No se usa `registro::alias_valido`
/// porque es más estricta de lo que admite un `~/.ssh/config`: un `Host` puede
/// llevar `:` (una IPv6) o `@`, y rechazarlo sería no poder abrir un servidor
/// que el usuario tiene. Lo que se prohíbe es lo que convierte un nombre en otra
/// cosa: el guion inicial, el espacio y los caracteres de control.
fn alias_aceptable(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 255
        && !a.starts_with('-')
        && !a.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn validar_alias(a: &str) -> Result<(), ErrorParaLaInterfaz> {
    if alias_aceptable(a) {
        Ok(())
    } else {
        Err(ErrorParaLaInterfaz {
            clase: "forma",
            mensaje: format!(
                "«{a}» no vale como nombre de servidor: no puede estar vacío, empezar por guion \
                 ni llevar espacios"
            ),
            detalle: None,
        })
    }
}

/// La ruta de `orbit` que manda la interfaz: **absoluta**, como pide el
/// cliente de terminal con `--orbit`. Resolverla por `PATH` sería dejar que el
/// `PATH` del otro lado eligiera qué se ejecuta como root.
fn binario_aceptable(b: &str) -> bool {
    b.starts_with('/') && b.len() <= 4096 && !b.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn validar_binario(b: &str) -> Result<(), ErrorParaLaInterfaz> {
    if binario_aceptable(b) {
        Ok(())
    } else {
        Err(ErrorParaLaInterfaz {
            clase: "forma",
            mensaje: format!(
                "«{b}» tiene que ser una ruta absoluta y sin espacios: resolverla por PATH sería \
                 dejar que el PATH del otro lado eligiera qué se ejecuta"
            ),
            detalle: None,
        })
    }
}

/// Construye el `Servidor` del núcleo a partir de un alias, **comprobando antes
/// lo que manda la interfaz**: el alias y, si viene, la ruta de `orbit`.
///
/// Mira **primero** la lista propia, porque un alias que está ahí no está en el
/// `~/.ssh/config` —el guardado lo impide— y así se evita una lectura del
/// fichero en el caso normal.
fn servidor_por_alias(
    alias: &str,
    binario: Option<String>,
) -> Result<Servidor, ErrorParaLaInterfaz> {
    validar_alias(alias)?;
    if let Some(b) = &binario {
        validar_binario(b)?;
    }
    if let Some(g) = guardado(alias) {
        return Ok(servidor_guardado(g));
    }
    match servidor_del_config(alias, binario) {
        Err(e) if e.clase == "alias-desconocido" => Err(apartado(alias).unwrap_or(e)),
        r => r,
    }
}

/// Si el alias está en la lista propia **pero apartado** al leerla, el error
/// que lo dice.
///
/// Sin esto, un servidor guardado por una versión anterior cuya clave ya no
/// pasa la validación (`id_ed25519` suelto, `%USERPROFILE%\…`) desaparecía, y
/// lo siguiente que se oía de él era «no es ninguno de tus servidores: ni lo has
/// añadido aquí…», que es falso. Se mira sólo cuando ya ha fallado todo lo
/// demás: es una lectura más del fichero, y en el caso normal no hace falta.
fn apartado(alias: &str) -> Option<ErrorParaLaInterfaz> {
    apartado_de(alias, registro::leer_completo().ok()?, registro::ruta())
}

fn apartado_de(
    alias: &str,
    lectura: registro::Lectura,
    ruta: Option<std::path::PathBuf>,
) -> Option<ErrorParaLaInterfaz> {
    let d = lectura
        .descartados
        .into_iter()
        .find(|d| d.alias.as_deref() == Some(alias))?;
    let donde = ruta
        .map(|p| format!(" ({})", p.display()))
        .unwrap_or_default();
    Some(ErrorParaLaInterfaz {
        clase: "registro",
        mensaje: format!(
            "«{alias}» está en tu lista de servidores{donde}, pero esa entrada no vale y no la \
             uso: {}. Vuelve a añadirlo con el mismo nombre —la nueva sustituye a la vieja— o \
             corrígela en el fichero",
            d.motivo
        ),
        detalle: None,
    })
}

/// Un servidor de la lista propia, **con la ruta de `orbit` que se guardó**.
///
/// La que mande la interfaz no cuenta y por eso ni se recibe: con
/// `binario.or(..)` cualquier llamada del renderizador elegía qué se ejecuta
/// como root en ese servidor.
fn servidor_guardado(g: registro::ServidorGuardado) -> Servidor {
    let mut s = Servidor::nuevo(&g.alias, g.destino());
    s.puerto = Some(g.puerto);
    s.clave = g.clave.clone();
    if let Some(b) = g.binario {
        s.binario = b;
    }
    s
}

// ── instalar Orbit ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct RequisitosParaLaInterfaz {
    pub git: bool,
    pub root: bool,
    pub sudo_sin_contrasena: bool,
    pub ya_instalado: bool,
    pub sistema: String,
    pub puede: bool,
    pub impedimentos: Vec<ImpedimentoParaLaInterfaz>,
    pub avisos: Vec<String>,
    /// La secuencia literal que se ejecutaría. Se manda **siempre**, también
    /// cuando hay impedimentos: quien no pueda instalar desde aquí igualmente
    /// puede copiarla y ejecutarla a mano.
    pub pasos: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ImpedimentoParaLaInterfaz {
    pub clase: String,
    pub que: String,
    pub arreglo: Option<String>,
}

/// Qué hay en el otro lado antes de instalar nada.
///
/// Una sola conexión y **antes de tocar nada**: descubrir a mitad de una
/// instalación que falta `git` deja un servidor a medias y a quien mira sin
/// saber qué ha quedado hecho.
#[tauri::command]
async fn requisitos_de_instalacion(
    alias: String,
    binario: Option<String>,
) -> Result<RequisitosParaLaInterfaz, ErrorParaLaInterfaz> {
    let s = servidor_por_alias(&alias, binario)?;
    let ctrl = dir_control();
    let r = tauri::async_runtime::spawn_blocking(move || {
        transporte::ejecutar_crudo(&s, instalar::COMPROBACION, ctrl.as_deref())
    })
    .await
    .map_err(|e| ErrorParaLaInterfaz {
        clase: "hilo",
        mensaje: e.to_string(),
        detalle: None,
    })??;

    let req = instalar::Requisitos::leer(&r.stdout);
    Ok(RequisitosParaLaInterfaz {
        git: req.git,
        root: req.root,
        sudo_sin_contrasena: req.sudo_sin_contrasena,
        ya_instalado: req.ya_instalado,
        sistema: req.sistema.clone(),
        puede: req.impedimentos().is_empty(),
        impedimentos: req
            .impedimentos()
            .into_iter()
            .map(|i| ImpedimentoParaLaInterfaz {
                clase: i.clase.to_string(),
                que: i.que,
                arreglo: i.arreglo,
            })
            .collect(),
        avisos: req.avisos(),
        pasos: instalar::pasos(),
    })
}

#[derive(serde::Serialize)]
pub struct Instalacion {
    pub codigo: i32,
    /// Lo que dijo el instalador, tal cual.
    pub salida: String,
    /// **Lo único en lo que se cree.** Sale de preguntarle a `orbit version
    /// --json` después, no del código de salida ni de la prosa: es la misma
    /// decisión que con `orbit new`, y por el mismo motivo — un instalador que
    /// termina en 1 puede haber dejado Orbit funcionando, y uno que termina en
    /// 0 no lo demuestra.
    pub version: Option<String>,
}

/// Instala Orbit, sirviendo la salida mientras ocurre.
///
/// Tarda entre cinco y diez minutos y no se puede acortar: son `apt`, Node,
/// PostgreSQL y PHP. Por eso la salida se sirve línea a línea — un bloque de
/// texto al final de diez minutos es información que llega cuando ya no sirve.
#[tauri::command]
async fn instalar_orbit(
    ventana: tauri::Window,
    vivos: tauri::State<'_, Vivos>,
    versiones: tauri::State<'_, Versiones>,
    alias: String,
    binario: Option<String>,
) -> Result<Instalacion, ErrorParaLaInterfaz> {
    // La clave lleva `!` porque ningún nombre de app puede empezar por ahí: la
    // regla del servidor obliga a `[a-z0-9]`. Igual que el `*` de la pasada.
    let clave = format!("{alias}:!instalar");
    let s = servidor_por_alias(&alias, binario.clone())?;
    // Dos instalaciones a la vez en el mismo servidor son dos `apt` peleándose
    // por el cerrojo de dpkg a mitad de instalar Node y PostgreSQL.
    let turno = vivos.apuntar(
        clave.clone(),
        format!("Ya hay una instalación de Orbit en curso en «{alias}»."),
    )?;
    let mando = turno.mando.clone();
    let ctrl = dir_control();
    let linea = instalar::orden();
    // Lo que se supiera de su versión deja de valer en cuanto se empieza.
    let cache = versiones.inner().clone();
    cache.olvidar(&s);

    let r = tauri::async_runtime::spawn_blocking(move || {
        let tanda = tanda_hacia(ventana, clave);
        let buzon = tanda.buzon();
        let r = transporte::ejecutar_crudo_en_vivo(&s, &linea, ctrl.as_deref(), mando, move |l| {
            buzon.echar(l)
        });
        drop(tanda);
        r
    })
    .await;

    drop(turno);

    let (codigo, salida) = match r {
        Ok(Ok(r)) => (r.codigo, r.stdout),
        Ok(Err(transporte::ErrorTransporte::Orbit { codigo, stdout, .. })) => (codigo, stdout),
        Ok(Err(e)) => return Err(e.into()),
        Err(e) => {
            return Err(ErrorParaLaInterfaz {
                clase: "hilo",
                mensaje: format!("la instalación se ha interrumpido: {e}"),
                detalle: None,
            })
        }
    };

    // Y ahora lo único que cuenta: preguntarle al servidor. Si contesta una
    // versión, está instalado — diga lo que diga el código de salida.
    let s2 = servidor_por_alias(&alias, binario)?;
    let ctrl2 = dir_control();
    let version = tauri::async_runtime::spawn_blocking(move || {
        let v = transporte::ejecutar(&s2, &Comando::Version, ctrl2.as_deref(), &[])
            .ok()
            .and_then(|r| r.leer::<Version>().ok());
        match &v {
            Some(v) => cache.apuntar(&s2, v),
            None => cache.olvidar(&s2),
        }
        v.map(|v| v.version)
    })
    .await
    .unwrap_or(None);

    Ok(Instalacion {
        codigo,
        salida,
        version,
    })
}

/// Cancela un despliegue en curso.
///
/// Mata el proceso, y lo que eso deja detrás **depende del paso**: interrumpir
/// un build no deja nada roto —la release nueva se descarta y `current` ni se
/// ha movido— pero interrumpir `service` o `nginx` sí puede dejar trabajo a
/// medias. Quien lo pulsa tiene que saber en cuál está, y eso lo dice la
/// pantalla, que es la que ve el paso en curso.
#[tauri::command]
fn cancelar(vivos: tauri::State<'_, Vivos>, alias: String, app: String) -> bool {
    let clave = format!("{alias}:{app}");
    match vivos.0.lock().unwrap().get(&clave) {
        Some(m) => {
            m.cancelar();
            true
        }
        None => false,
    }
}

/// Los **nombres** de las variables de una app. Nunca los valores.
///
/// `orbit env list --json` devuelve sólo `keys`, y eso no es una limitación
/// pendiente de levantar: un panel que enseñe el `.env` entero es un panel que
/// filtra la contraseña de la base de datos en la primera captura que alguien
/// pegue en un issue.
#[tauri::command]
async fn entorno(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::EntornoLista { app }).await
}

/// Lo que la app declara frente a lo que tiene, **sin un solo valor**. 1.3.8.
///
/// Es lo que se pregunta antes de desplegar: si falta una obligatoria, la
/// interfaz la pide en un formulario y la manda con `--env`, en vez de lanzar
/// un despliegue que el servidor va a parar antes del build.
#[tauri::command]
async fn comprobar_entorno(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::EntornoComprobar { app }).await
}

/// Las tareas programadas de una app, con su última y su próxima pasada. 1.3.9.
#[tauri::command]
async fn tareas(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Tareas { app }).await
}

/// Congela los despliegues de una app (`orbit lock`). 1.4.3.
///
/// El rollback sigue permitido: es la salida de emergencia. Contesta en prosa,
/// y lo que cuenta es cómo queda: la interfaz vuelve a preguntar con `info`.
#[tauri::command]
async fn congelar(
    alias: String,
    app: String,
    motivo: Option<String>,
    binario: Option<String>,
) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::Congelar { app, motivo })).await
}

#[tauri::command]
async fn descongelar(alias: String, app: String, binario: Option<String>) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::Descongelar { app })).await
}

/// Que el autodespliegue y `deploy --all` esperen a la CI de GitHub. 1.4.3.
#[tauri::command]
async fn esperar_ci(
    alias: String,
    app: String,
    esperar: bool,
    binario: Option<String>,
) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::EsperarCi { app, esperar })).await
}

/// **Un** valor, de uno en uno.
///
/// Pedir un secreto tiene que ser un acto explícito, y por eso es un comando
/// aparte y no un campo de la respuesta anterior. Va sin `--json` porque
/// `orbit env get` imprime el valor pelado, que es lo correcto.
///
/// Lo que este camino **no** hace:
///
///  · **No lo guarda en ningún sitio.** Ni caché, ni estado serializado, ni
///    fichero. El valor existe en la memoria del proceso y sólo mientras la
///    pantalla que lo pidió lo tenga a la vista.
///  · **No lo escribe en el registro.** Se anota que se pidió un valor de una
///    app, nunca cuál ni el valor — es exactamente lo que hace Orbit con
///    `logline "exec $name"`, y por el mismo motivo: un log no es sitio para
///    secretos.
#[tauri::command]
async fn entorno_valor(
    alias: String,
    app: String,
    clave: String,
    binario: Option<String>,
) -> Result<String, ErrorParaLaInterfaz> {
    aparte(move || {
        let s = servidor_por_alias(&alias, binario)?;
        let c = Comando::EntornoValor { app, clave };
        let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
        // Pelado y sin adornos, que es como lo imprime Orbit. El salto final se
        // quita porque es del `printf`, no del valor.
        Ok(r.stdout.trim_end_matches('\n').to_string())
    })
    .await
}

/// El monitor. **Tarda ~2,1 s con 40 apps**, y no es lentitud: la CPU es la
/// diferencia entre dos lecturas del cgroup, así que una foto suelta tiene que
/// esperar a la segunda a propósito.
#[tauri::command]
async fn monitor(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Top).await
}

/// El tráfico de una app, del log que nginx ya escribe.
///
/// Sin cookies, sin JavaScript y sin nada nuevo corriendo: es lo que hace que
/// una analítica quepa aquí cuando un panel web no cabe. Y el dato lleva ahí
/// desde la primera visita.
#[tauri::command]
async fn trafico(
    alias: String,
    app: String,
    desde: Option<String>,
    binario: Option<String>,
) -> Resultado {
    pedir_aparte(alias, binario, Comando::Trafico { app, desde }).await
}

// ── lo del servidor que no es una app ──────────────────────────────────────
//
// Las seis estaban en el catálogo del núcleo desde el principio y la ventana no
// podía pedirlas. Son de sólo lectura —también `verificar_copias`, que abre
// cada fichero pero no cambia nada— y la interfaz las pide al abrir cada
// sección, nunca al entrar en el servidor.

/// `backup list --json`, 1.3.7. `verified` llega siempre a `null`: «no lo he
/// comprobado en esta llamada» no es «está mal».
#[tauri::command]
async fn copias(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::CopiasLista).await
}

/// `backup verify --json`, 1.3.7. **Sale con 1 si alguna copia es mala**, con
/// el objeto entero por stdout: el transporte lo devuelve como respuesta y no
/// como fallo, que es la diferencia entre «hay una copia rota» y «no he podido
/// preguntar».
#[tauri::command]
async fn verificar_copias(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::CopiasVerificar).await
}

/// `db list --json`. Con PostgreSQL caído el servidor sale con error y **sin
/// objeto**, y así llega: no como «no hay ninguna base de datos», que es lo que
/// leería un script de copias.
#[tauri::command]
async fn bases_de_datos(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::BaseDeDatosLista).await
}

/// `redirect list --json`, de todas las apps.
#[tauri::command]
async fn redirecciones(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::RedireccionesLista { app: None }).await
}

/// `watch status --json`. `since` y `last_try` usan 0 para «no se sabe».
#[tauri::command]
async fn vigilancia(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::VigilanciaEstado).await
}

/// `queue status --json`.
#[tauri::command]
async fn colas(alias: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::ColaEstado).await
}

/// El histórico de despliegues de una app.
#[tauri::command]
async fn metricas(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Metricas { app: Some(app) }).await
}

/// Ejecuta algo dentro de una app. **La puerta trasera.**
///
/// Devuelve la salida cruda —stdout y stderr por separado— y **no la
/// interpreta**: es salida arbitraria de un proceso arbitrario, y puede traer
/// secuencias ANSI, bytes nulos o megas en una sola línea.
///
/// El registro anota **sólo que se ejecutó algo en una app**, nunca qué. Se
/// hereda de Orbit, que apunta `exec <app>` con el comentario de que un comando
/// puede llevar una contraseña delante y un log no es sitio para secretos.
#[tauri::command]
/// `shell: true` manda el texto detrás de `/bin/bash -lc`; `false` manda los
/// argumentos separados y no hay shell. La
/// diferencia se elige en la pantalla y se ve, en vez de deducirla de la
/// heurística del servidor: quien escribe tiene que poder predecir cuándo su
/// `&&` se ejecuta y cuándo se pasa como texto.
async fn correr(
    alias: String,
    app: String,
    shell: bool,
    argumentos: Vec<String>,
    binario: Option<String>,
) -> Result<SalidaDeExec, ErrorParaLaInterfaz> {
    aparte(move || correr_aqui(alias, app, shell, argumentos, binario)).await
}

fn correr_aqui(
    alias: String,
    app: String,
    shell: bool,
    argumentos: Vec<String>,
    binario: Option<String>,
) -> Result<SalidaDeExec, ErrorParaLaInterfaz> {
    let modo = if shell {
        ModoDeExec::Shell(argumentos.join(" "))
    } else {
        ModoDeExec::Argumentos(argumentos)
    };
    let s = servidor_por_alias(&alias, binario)?;
    let c = Comando::Ejecutar { app, modo };
    // Se enseña la orden exacta que se va a mandar, ya escapada. Es lo que
    // convierte «confío en la interfaz» en «he leído lo que va a pasar».
    let linea = c.linea(&s.binario).map_err(ErrorParaLaInterfaz::from)?;
    match transporte::ejecutar(&s, &c, dir_control().as_deref(), &[]) {
        Ok(r) => Ok(SalidaDeExec {
            orden: linea,
            stdout: r.stdout,
            stderr: r.stderr,
            codigo: r.codigo,
        }),
        // Un comando que sale con error NO es un fallo del transporte: es un
        // comando que salió con error, y su salida es lo que hay que enseñar.
        //
        // Y eso incluye un 127 por una errata o el aviso de un `sudo` de dentro
        // del comando: el transporte sabe que con `exec` habla un proceso
        // ajeno y ya no los convierte en «no hay orbit» ni en «sudo pide
        // contraseña» (ver `Origen` en transporte.rs), así que llegan aquí.
        // Igual un 255 —un PHP con un error fatal— o un «Host key verification
        // failed» de un `git` de dentro: sólo son «no he llegado» o «la clave
        // ha cambiado» si lo dice nuestro `ssh`, y si no, son esta salida.
        Err(transporte::ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => Ok(SalidaDeExec {
            orden: linea,
            stdout,
            stderr,
            codigo,
        }),
        Err(e) => Err(e.into()),
    }
}

#[derive(serde::Serialize)]
pub struct SalidaDeExec {
    /// La orden literal, ya escapada.
    pub orden: String,
    pub stdout: String,
    pub stderr: String,
    pub codigo: i32,
}

impl From<orbit_client::comando::ErrorForma> for ErrorParaLaInterfaz {
    fn from(e: orbit_client::comando::ErrorForma) -> Self {
        Self {
            clase: "forma",
            mensaje: e.to_string(),
            detalle: None,
        }
    }
}

/// El detalle de una app. Es de donde sale el inventario de lo que se pierde
/// al borrarla, y se pide **en ese momento**: decirle a alguien que va a perder
/// 3 releases cuando tiene 12 es peor que no decírselo.
#[tauri::command]
async fn detalle(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir_aparte(alias, binario, Comando::Info { app }).await
}

/// Retira una app **sin borrar sus datos**. Reversible.
///
/// 'remove' no habla JSON, así que la respuesta es prosa. Se devuelve tal cual
/// y la interfaz vuelve a preguntar por el estado, que es la regla: lo que
/// cuenta es cómo queda el servidor, no lo que dijo el comando.
#[tauri::command]
async fn retirar(alias: String, app: String, binario: Option<String>) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::Retirar { app })).await
}

/// Retira una app **y borra sus datos**. Irreversible.
///
/// Aquí no hay red debajo: `orbit remove -y --purge` no pregunta nada, así que
/// toda la protección está en la pantalla que llama a esto.
#[tauri::command]
async fn retirar_y_borrar(alias: String, app: String, binario: Option<String>) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::RetirarYBorrar { app })).await
}

/// Vuelve a una release anterior. La release es **obligatoria**: sin ella y sin
/// terminal, `orbit rollback` aborta — y hace bien, porque el «valor por
/// defecto» sería la que ya está activa. Y funciona también con la app
/// congelada: es la salida de emergencia, y Orbit no la bloquea.
#[tauri::command]
async fn revertir(
    alias: String,
    app: String,
    release: String,
    binario: Option<String>,
) -> Resultado {
    aparte(move || pedir_prosa(&alias, binario, Comando::Revertir { app, release })).await
}

/// Qué hay al otro lado de un alias, antes de fiarse de nada.
///
/// Sin esta clasificación, quien añade un servidor ve «error» y no sabe si es
/// **su clave, su red o su servidor**. Es la diferencia entre un producto y una
/// demo, y por eso es un comando propio y no un efecto secundario de la primera
/// pantalla que se abra.
#[tauri::command]
async fn saludar(
    versiones: tauri::State<'_, Versiones>,
    alias: String,
    binario: Option<String>,
) -> Result<serde_json::Value, ErrorParaLaInterfaz> {
    let cache = versiones.inner().clone();
    aparte(move || saludar_aqui(&cache, alias, binario)).await
}

fn saludar_aqui(
    cache: &Versiones,
    alias: String,
    binario: Option<String>,
) -> Result<serde_json::Value, ErrorParaLaInterfaz> {
    let s = servidor_por_alias(&alias, binario)?;
    let c = Comando::Version;
    let saludo = match transporte::ejecutar(&s, &c, dir_control().as_deref(), &[]) {
        Ok(r) => orbit_client::saludo::clasificar_para(&r, &s.binario),
        // Un fallo de `orbit` con salida también se clasifica: un Orbit a
        // medias sale con 1 y su prosa es justamente el dato.
        Err(transporte::ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => orbit_client::saludo::clasificar_para(
            &transporte::Respuesta {
                stdout,
                stderr,
                codigo,
                cancelado: false,
            },
            &s.binario,
        ),
        Err(e) => orbit_client::saludo::de_error(&e),
    };
    // El saludo es la lectura de la versión más fresca que hay: si da una, es
    // ésa; si no, la que hubiera ya no se sabe si vale.
    match &saludo {
        orbit_client::Saludo::Ok(v) | orbit_client::Saludo::MasNuevo(v) => cache.apuntar(&s, v),
        _ => cache.olvidar(&s),
    }
    Ok(serde_json::json!({
        "clase": clase_de(&saludo),
        "version": version_de(&saludo),
        "contrato": contrato_de(&saludo),
        "motivo": motivo_de(&saludo),
        "puede_operar": saludo.permite_operar(),
        "puede_leer": saludo.permite_leer(),
        // La de `instalar`, que es la que funciona. Aquí iba un `curl … | sudo
        // bash` que el propio instalador desmiente —install.sh lee los ficheros
        // que tiene al lado y por una tubería no los hay— y cuyo comentario
        // decía «nunca un botón de instalar» cuando ese botón ya existe. El
        // campo se queda porque es parte de la respuesta que tipa la interfaz.
        "orden_de_instalacion": instalar::orden(),
    }))
}

fn clase_de(s: &orbit_client::Saludo) -> &'static str {
    use orbit_client::Saludo as S;
    match s {
        S::Ok(_) => "ok",
        S::MasNuevo(_) => "mas-nuevo",
        S::SinContrato { .. } => "sin-contrato",
        S::NoInstalado { .. } => "no-instalado",
        S::SinPrivilegios => "sin-privilegios",
        S::NoSeLlega { .. } => "no-se-llega",
        S::ClaveDeHostCambiada { .. } => "clave-de-host-cambiada",
    }
}

fn version_de(s: &orbit_client::Saludo) -> Option<String> {
    use orbit_client::Saludo as S;
    match s {
        S::Ok(v) | S::MasNuevo(v) => Some(v.version.clone()),
        S::SinContrato { version } => version.clone(),
        _ => None,
    }
}

fn contrato_de(s: &orbit_client::Saludo) -> Option<u32> {
    use orbit_client::Saludo as S;
    match s {
        S::Ok(v) | S::MasNuevo(v) => Some(v.contract),
        _ => None,
    }
}

fn motivo_de(s: &orbit_client::Saludo) -> Option<String> {
    use orbit_client::Saludo as S;
    match s {
        S::NoInstalado { motivo } => Some((*motivo).to_string()),
        S::NoSeLlega { detalle } | S::ClaveDeHostCambiada { detalle } => Some(detalle.clone()),
        _ => None,
    }
}

/// Los alias de `~/.ssh/config`, para poder importarlos.
///
/// **No conecta con ninguno**: enumerar no es visitar. Saber si en un alias hay
/// un Orbit es otra pregunta, se hace después y de una en una, porque abrir una
/// pantalla no puede significar abrir cuarenta sesiones SSH.
///
/// Fuera del hilo de la ventana: es un `ssh -G` por alias —en paralelo, pero
/// procesos al fin y al cabo— y la versión síncrona congelaba la ventana al
/// abrirla, que es justo cuando alguien la está mirando.
#[tauri::command]
async fn servidores_del_config() -> Result<Vec<descubrir::AliasSsh>, ErrorParaLaInterfaz> {
    aparte(|| {
        let ruta = descubrir::ruta_por_defecto().ok_or(ErrorParaLaInterfaz {
            clase: "sin-home",
            mensaje: "no sé dónde está tu carpeta personal".into(),
            detalle: None,
        })?;
        Ok(descubrir::descubrir(&ruta))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn ejecutar() {
    tauri::Builder::default()
        .manage(Vivos::default())
        .manage(Versiones::default())
        .invoke_handler(tauri::generate_handler![
            version,
            portada,
            lista,
            info,
            doctor,
            doctor_arreglar,
            logs,
            entorno,
            entorno_valor,
            comprobar_entorno,
            tareas,
            congelar,
            descongelar,
            esperar_ci,
            monitor,
            trafico,
            metricas,
            copias,
            verificar_copias,
            bases_de_datos,
            redirecciones,
            vigilancia,
            colas,
            detalle,
            retirar,
            retirar_y_borrar,
            revertir,
            correr,
            desplegar,
            desplegar_todo,
            crear,
            resolver,
            cancelar,
            servidores_del_config,
            servidores_guardados,
            guardar_servidor,
            olvidar_servidor,
            requisitos_de_instalacion,
            instalar_orbit,
            saludar,
        ])
        .run(tauri::generate_context!())
        .expect("no he podido abrir la ventana");
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// La clave con la que se registra una pasada es `alias:*`, y que eso no
    /// pueda chocar con la de un despliegue suelto **no es una convención**:
    /// es una consecuencia de la regla de forma del servidor, que obliga a que
    /// un nombre de app empiece por minúscula o dígito.
    ///
    /// Se comprueba aquí porque de ello depende que «parar la pasada» no pueda
    /// parar el despliegue de otra cosa.
    #[test]
    fn el_asterisco_de_la_pasada_no_puede_ser_el_nombre_de_una_app() {
        use orbit_client::comando::nombre_de_app_valido;
        assert!(!nombre_de_app_valido("*"));
        for n in ["*", "*x", "a*", "todo *"] {
            assert!(!nombre_de_app_valido(n), "«{n}» no puede ser una app");
        }
    }

    /// Un doble clic en «Desplegar» no puede ser dos despliegues, y al terminar
    /// cada uno sólo se quita a sí mismo del registro.
    #[test]
    fn un_segundo_despliegue_con_la_misma_clave_se_rechaza() {
        let v = Vivos::default();
        let uno = v
            .apuntar("prod:tienda".into(), "ocupado".into())
            .ok()
            .expect("se apunta");
        let e = v
            .apuntar("prod:tienda".into(), "Ya hay un despliegue".into())
            .err()
            .expect("el segundo se rechaza");
        assert_eq!(e.clase, "en-curso");
        assert_eq!(e.mensaje, "Ya hay un despliegue");
        // El primero sigue siendo el que se cancela.
        assert!(v.0.lock().unwrap()["prod:tienda"].es_el_mismo(&uno.mando));
        // Otra app, u otro servidor, sí.
        let otra = v
            .apuntar("prod:blog".into(), String::new())
            .ok()
            .expect("se apunta");
        let fuera = v
            .apuntar("staging:tienda".into(), String::new())
            .ok()
            .expect("se apunta");
        drop(uno);
        assert!(!v.0.lock().unwrap().contains_key("prod:tienda"));
        assert!(v.0.lock().unwrap().contains_key("prod:blog"));
        // Y con el primero terminado, se puede volver a desplegar.
        let otra_vez = v
            .apuntar("prod:tienda".into(), String::new())
            .ok()
            .expect("se apunta");
        drop((otra, fuera, otra_vez));
        assert!(v.0.lock().unwrap().is_empty());
    }

    /// Un turno que se suelta no quita el mando de otro que ocupe su clave.
    #[test]
    fn al_terminar_solo_se_quita_el_mando_propio() {
        let v = Vivos::default();
        let uno = v
            .apuntar("prod:tienda".into(), String::new())
            .ok()
            .expect("se apunta");
        // Simula que alguien ha puesto otro mando en su sitio.
        let ajeno = EnCurso::nuevo();
        v.0.lock()
            .unwrap()
            .insert("prod:tienda".into(), ajeno.clone());
        drop(uno);
        assert!(v.0.lock().unwrap()["prod:tienda"].es_el_mismo(&ajeno));
    }

    /// Un alias que empieza por guion es una opción de `ssh` en el sitio donde
    /// va un nombre; la interfaz no puede mandarlo.
    #[test]
    fn la_interfaz_no_puede_colar_una_opcion_de_ssh_como_alias() {
        for malo in [
            "-oProxyCommand=calc",
            "-F\\\\host\\share\\cfg",
            "",
            "prod web",
            "prod\n-oX=y",
            "prod\t",
        ] {
            assert!(!alias_aceptable(malo), "«{malo}»");
            let e = servidor_por_alias(malo, None).expect_err("se rechaza");
            assert_eq!(e.clase, "forma");
        }
        // Lo que un `~/.ssh/config` admite y `registro::alias_valido` no.
        for bueno in ["prod", "vps-ovh.2", "fe80::1", "dave@bastion"] {
            assert!(alias_aceptable(bueno), "«{bueno}»");
        }
    }

    /// Uno guardado pero apartado al leer dice que está y por qué no vale, en
    /// vez de «no es ninguno de tus servidores».
    #[test]
    fn rc2_un_servidor_apartado_dice_que_esta_y_por_que_no_vale() {
        let l = registro::Lectura {
            servidores: Vec::new(),
            descartados: vec![registro::Descartado {
                alias: Some("viejo".into()),
                motivo: registro::ErrorDeRegistro::Clave("id_ed25519".into()).to_string(),
            }],
        };
        let e = apartado_de("viejo", l.clone(), Some("/h/servidores.json".into()))
            .expect("está apartado");
        assert_eq!(e.clase, "registro");
        assert!(e.mensaje.contains("está en tu lista"), "{}", e.mensaje);
        assert!(e.mensaje.contains("~/.ssh/id_ed25519"), "{}", e.mensaje);
        assert!(e.mensaje.contains("Vuelve a añadirlo"), "{}", e.mensaje);
        assert!(apartado_de("otro", l, None).is_none());
    }

    /// Un alias que no está guardado ni en el `~/.ssh/config` no llega a `ssh`:
    /// `user@atacante.example` también es un nombre sin guion.
    #[test]
    fn rc_un_alias_que_no_es_de_nadie_no_llega_a_ssh() {
        let config = vec!["prod".to_string(), "dave@bastion".to_string()];
        let e = servidor_si_esta("user@atacante.example", None, &config).expect_err("se rechaza");
        assert_eq!(e.clase, "alias-desconocido");
        let Ok(s) = servidor_si_esta("prod", Some("/opt/orbit".into()), &config) else {
            panic!("«prod» está en el fichero");
        };
        assert_eq!(s.destino, "prod");
        // Para los del fichero la ruta de la interfaz sí vale: no hay otra.
        assert_eq!(s.binario, "/opt/orbit");
        assert!(servidor_si_esta("dave@bastion", None, &config).is_ok());
    }

    /// La ruta de `orbit` de un servidor guardado es la guardada.
    #[test]
    fn rc_la_interfaz_no_elige_el_binario_de_un_servidor_guardado() {
        let g = |binario: Option<&str>| registro::ServidorGuardado {
            alias: "vps".into(),
            host: "203.0.113.7".into(),
            usuario: "dave".into(),
            puerto: 2222,
            clave: None,
            binario: binario.map(Into::into),
        };
        let s = servidor_guardado(g(Some("/opt/orbit/bin/orbit")));
        assert_eq!(s.binario, "/opt/orbit/bin/orbit");
        assert_eq!(s.destino, "dave@203.0.113.7");
        assert_eq!(s.puerto, Some(2222));
        // Sin ruta guardada, la de fábrica; nunca la de la interfaz.
        assert_eq!(servidor_guardado(g(None)).binario, "/usr/local/bin/orbit");
    }

    fn version(v: &str) -> Version {
        Version {
            schema: 1,
            version: v.into(),
            contract: 1,
        }
    }

    #[test]
    fn rc_la_version_se_recuerda_un_rato_y_por_servidor_y_binario() {
        let c = Versiones::default();
        let s = Servidor::nuevo("prod", "prod");
        let mut otro_bin = s.clone();
        otro_bin.binario = "/opt/orbit".into();
        let antes = Instant::now();
        c.apuntar_en(&s, &version("1.4.9"), antes);
        assert_eq!(
            c.vigente_en(&s, antes).map(|v| v.version),
            Some("1.4.9".into())
        );
        assert!(c.vigente_en(&otro_bin, antes).is_none());
        assert!(c
            .vigente_en(&Servidor::nuevo("staging", "staging"), antes)
            .is_none());
        // Pasada su vida, se vuelve a preguntar.
        assert!(c.vigente_en(&s, antes + VIDA_DE_LA_VERSION).is_none());
        // Y se olvida al instalar o cuando el saludo no da versión.
        c.apuntar(&s, &version("1.5.0"));
        assert!(c.vigente(&s).is_some());
        c.olvidar(&s);
        assert!(c.vigente(&s).is_none());
    }

    /// Lo que ha emitido una tanda, con cuándo llegó cada envío.
    type Visto = Arc<Mutex<Vec<(Instant, Vec<String>)>>>;

    /// Recoge lo que emite una tanda.
    fn recoger() -> (Tanda, Visto) {
        let visto = Arc::new(Mutex::new(Vec::new()));
        let v = Arc::clone(&visto);
        let t = Tanda::nueva(move |l| v.lock().unwrap().push((Instant::now(), l)));
        (t, visto)
    }

    /// Medido: 30 000 líneas eran 30 000 eventos. Ahora salen en tandas, en
    /// orden y sin perder ninguna.
    #[test]
    fn rc_el_progreso_sale_en_tandas_en_orden_y_entero() {
        let (t, visto) = recoger();
        let b = t.buzon();
        let n = 30_000;
        let t0 = Instant::now();
        for i in 0..n {
            b.echar(format!("línea {i}"));
        }
        drop(t);
        let dur = t0.elapsed();
        let visto = visto.lock().unwrap();
        let todas: Vec<&String> = visto.iter().flat_map(|(_, l)| l).collect();
        assert_eq!(todas.len(), n);
        for (i, l) in todas.iter().enumerate() {
            assert_eq!(**l, format!("línea {i}"));
        }
        let eventos = visto.len();
        eprintln!("{n} líneas → {eventos} eventos en {dur:?} (antes: {n} eventos)");
        assert!(eventos <= n / TANDA_LINEAS + 1, "{eventos} eventos");
    }

    /// Una línea sola no espera a la siguiente: un servidor callado a mitad de
    /// build enseña su última línea enseguida, sin que se cierre nada.
    #[test]
    fn rc_una_linea_sola_sale_sin_esperar_a_la_siguiente() {
        let (t, visto) = recoger();
        let b = t.buzon();
        let t0 = Instant::now();
        b.echar("compilando…".into());
        let limite = t0 + Duration::from_secs(2);
        while visto.lock().unwrap().is_empty() && Instant::now() < limite {
            std::thread::sleep(Duration::from_millis(5));
        }
        let (cuando, l) = visto.lock().unwrap().first().cloned().expect("ha salido");
        assert_eq!(l, ["compilando…"]);
        let espera = cuando - t0;
        eprintln!("una línea sola ha salido a los {espera:?}");
        // 50 ms de plazo; el margen es para una máquina de CI cargada.
        assert!(espera < Duration::from_millis(500), "{espera:?}");
        drop(t);
        // Y cerrar sin nada pendiente no emite una tanda vacía.
        assert_eq!(visto.lock().unwrap().len(), 1);
    }

    /// Lo que queda al terminar sale al cerrar, no se pierde.
    #[test]
    fn rc_lo_que_queda_sale_al_cerrar() {
        let (t, visto) = recoger();
        let b = t.buzon();
        for i in 0..10 {
            b.echar(i.to_string());
        }
        drop(t);
        let todas: Vec<String> = visto
            .lock()
            .unwrap()
            .iter()
            .flat_map(|(_, l)| l.clone())
            .collect();
        assert_eq!(todas, (0..10).map(|i| i.to_string()).collect::<Vec<_>>());
    }

    /// La forma del evento: `[clave, [líneas…]]`, que es lo que espera la
    /// interfaz.
    #[test]
    fn rc_la_forma_del_evento_es_clave_y_lineas() {
        let v = serde_json::to_value(("prod:tienda", vec!["a".to_string(), "b".into()])).unwrap();
        assert_eq!(v, serde_json::json!(["prod:tienda", ["a", "b"]]));
        assert_eq!(EVENTO_DE_PROGRESO, "orbit://progreso");
    }

    #[test]
    fn la_ruta_de_orbit_tiene_que_ser_absoluta() {
        assert!(binario_aceptable("/usr/local/bin/orbit"));
        assert!(binario_aceptable("/opt/orbit/bin/orbit"));
        for malo in [
            "orbit",
            "./orbit",
            "",
            "/usr/local/bin/orbit --help",
            "/x\n",
        ] {
            assert!(!binario_aceptable(malo), "«{malo}»");
        }
        let e = servidor_por_alias("prod", Some("orbit".into())).expect_err("se rechaza");
        assert_eq!(e.clase, "forma");
    }

    /// Una orden sin `--json` cuenta lo que ha hecho por **stdout**, y los
    /// avisos por stderr. La primera versión devolvía sólo stderr, y de un
    /// `remove` que salía bien llegaba una cadena vacía.
    #[test]
    fn la_prosa_de_una_orden_sin_json_llega_entera() {
        let r = transporte::Respuesta {
            stdout: "  ✓ «tienda» congelada\n".into(),
            stderr: "  ! sin avisos configurados\n".into(),
            codigo: 0,
            cancelado: false,
        };
        let p = prosa(&r);
        assert!(p.contains("congelada"));
        assert!(p.contains("sin avisos"));
        let solo = transporte::Respuesta {
            stdout: "hecho\n".into(),
            stderr: String::new(),
            codigo: 0,
            cancelado: false,
        };
        assert_eq!(prosa(&solo), "hecho");
    }

    /// El valor de una variable cruza el puente una vez y, en cuanto cruza, ya
    /// no se deja imprimir.
    #[test]
    fn el_valor_de_una_variable_no_sale_por_debug_al_cruzar() {
        let v: VariableDeLaInterfaz =
            serde_json::from_str(r#"{"clave":"DB_PASSWORD","valor":"hunter2"}"#).unwrap();
        let n: VariableNueva = v.into();
        assert_eq!(n.valor.exponer(), "hunter2");
        let d = format!("{n:?}");
        assert!(!d.contains("hunter2"), "{d}");
    }

    /// Con 1.4.9 los valores van por la entrada; antes, por el `argv`; y con una
    /// versión que no se entiende, por ningún sitio: no se adivina.
    #[test]
    fn las_variables_van_por_la_entrada_desde_la_1_4_9() {
        let v = |s: &str| orbit_client::contrato::Version {
            schema: 1,
            version: s.into(),
            contract: 1,
        };
        assert_eq!(variables_por_la_entrada(&v("1.4.8")), Some(false));
        assert_eq!(variables_por_la_entrada(&v("1.4.9")), Some(true));
        assert_eq!(variables_por_la_entrada(&v("1.5.0")), Some(true));
        assert_eq!(variables_por_la_entrada(&v("desarrollo")), None);
    }

    /// Los tres estados de una anulación tienen que llegar al núcleo como tres
    /// cosas distintas, y esta prueba existe porque la primera versión no lo
    /// hacía.
    ///
    /// Iban como `undefined` / `null` / cadena, dando por hecho que serde
    /// distinguiría un campo ausente de un `null` explícito en un
    /// `Option<Option<String>>`. No los distingue: los colapsa en `None`. El
    /// resultado habría sido que el repaso enseñara `--build ''` y el servidor
    /// recibiera una orden sin esa bandera — enseñar una cosa y ejecutar otra,
    /// en la pantalla cuyo único argumento es que eso no pase.
    #[test]
    fn los_tres_estados_de_una_anulacion_cruzan_el_puente_distintos() {
        let ausente: AjustesParaLaInterfaz = serde_json::from_str("{}").unwrap();
        assert_eq!(ausente.build, AnulacionDeLaInterfaz::Detectar);

        let vacia: AjustesParaLaInterfaz =
            serde_json::from_str(r#"{"build":{"modo":"vacia"}}"#).unwrap();
        assert_eq!(vacia.build, AnulacionDeLaInterfaz::Vacia);

        let valor: AjustesParaLaInterfaz =
            serde_json::from_str(r#"{"build":{"modo":"valor","valor":"pnpm build"}}"#).unwrap();
        assert_eq!(
            valor.build,
            AnulacionDeLaInterfaz::Valor {
                valor: "pnpm build".into()
            }
        );

        // Y un `null` pelado ya no es una respuesta válida. Que falle en voz
        // alta es mejor que que signifique en silencio lo que no es.
        assert!(serde_json::from_str::<AjustesParaLaInterfaz>(r#"{"build":null}"#).is_err());

        // Lo que importa de verdad: los tres producen tres órdenes distintas.
        let arg = |a: AjustesParaLaInterfaz| {
            Comando::Nueva(Box::new(WebNueva {
                nombre: "web".into(),
                repo: "u/web".into(),
                rama: "main".into(),
                dominio: "web.ejemplo.com".into(),
                alias: vec![],
                correo: None,
                base_de_datos: false,
                https: true,
                ajustes: a.into(),
            }))
            .argv("orbit")
            .unwrap()
        };
        assert!(!arg(ausente).contains(&"--build".to_string()));

        let con_vacia = arg(vacia);
        let i = con_vacia.iter().position(|x| x == "--build").unwrap();
        assert_eq!(con_vacia[i + 1], "", "«no se compila» viaja como argumento");

        assert!(arg(valor).contains(&"pnpm build".to_string()));
    }
}
