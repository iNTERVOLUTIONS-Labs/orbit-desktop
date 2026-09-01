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

use orbit_client::comando::{AjustesDeteccion, Anulacion, Comando, ModoDeExec, WebNueva};
use orbit_client::descubrir;
use orbit_client::transporte::{self, EnCurso, Servidor};
use std::collections::HashMap;
use std::sync::Mutex;
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

fn servidor(alias: &str, binario: Option<String>) -> Servidor {
    let mut s = Servidor::nuevo(alias, alias);
    if let Some(b) = binario {
        s.binario = b;
    }
    s
}

/// Un comando cualquiera del catálogo, ejecutado y devuelto **sin interpretar**.
///
/// La interfaz recibe el objeto tal cual llegó del servidor. No se aplana, no
/// se renombra y no se rellena nada: los tipos ya están en `contrato.rs` y en
/// su gemelo de TypeScript, y meter aquí una tercera forma sería inventar un
/// tercer contrato.
fn pedir(alias: &str, binario: Option<String>, c: Comando) -> Resultado {
    let s = servidor(alias, binario);
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    let texto = r.objeto()?;
    serde_json::from_str(texto).map_err(|e| ErrorParaLaInterfaz {
        clase: "json",
        mensaje: e.to_string(),
        detalle: None,
    })
}

/// Dónde vive el socket de `ControlMaster`.
///
/// En `XDG_RUNTIME_DIR` y no en `/tmp`: es un directorio propio del usuario,
/// que el sistema limpia al cerrar la sesión. Un socket de control **es una
/// sesión root guardada en un fichero** —con él abierto se puede abrir un canal
/// sin la clave, comprobado— así que dónde vive y con qué permisos no es un
/// detalle de higiene.
fn dir_control() -> Option<String> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")?;
    let d = std::path::Path::new(&base).join("orbit-desktop");
    std::fs::create_dir_all(&d).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700)).ok()?;
    }
    Some(d.to_string_lossy().into_owned())
}

// ── el catálogo, y nada más ────────────────────────────────────────────────

#[tauri::command]
fn version(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Version)
}

/// La portada se alimenta de **esto** y no de `list`.
///
/// `status --json` trae el array de apps completo e **idéntico** al de
/// `list --json` —comprobado comparando los dos objetos— así que una llamada de
/// 389 ms sustituye a dos que suman 695. Un 44 % menos en la carga que forma la
/// primera impresión, y no cuesta nada: es elegir bien qué comando se pide.
#[tauri::command]
fn portada(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Estado)
}

#[tauri::command]
fn lista(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Lista)
}

#[tauri::command]
fn info(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Info { app })
}

#[tauri::command]
fn doctor(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Doctor)
}

/// El log de una app, **crudo**.
///
/// Devuelve el NDJSON tal cual y no una lista ya parseada, y es a propósito:
/// `logs` es la única excepción del contrato —un flujo, no un objeto— y
/// convertirlo aquí obligaría a inventar una tercera forma entre el servidor y
/// la interfaz. El lector vive en `flujo.rs` y tiene su gemelo en TypeScript,
/// que son dos y no tres.
#[tauri::command]
fn logs(
    alias: String,
    app: String,
    desde: Option<String>,
    binario: Option<String>,
) -> Result<String, ErrorParaLaInterfaz> {
    let s = servidor(&alias, binario);
    let c = Comando::Logs {
        app,
        desde,
        lineas: Some(500),
        // Nunca en vivo por este camino: un flujo que no termina no se puede
        // devolver de una llamada que sí. La pantalla en vivo es otra cosa.
        seguir: false,
        solo_nginx: false,
    };
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(r.stdout)
}

/// Aplica lo que el servidor sabe arreglar sin decidir nada por nadie.
///
/// Existe desde que `orbit doctor --fix --json --yes` funciona. Estuvo
/// documentado y muerto durante versiones —`--yes` no era una bandera global y
/// no había forma de dársela— así que contra un servidor más viejo esto falla,
/// y la interfaz vuelve a enseñar la orden para copiar. Por eso la capacidad se
/// **pregunta** y no se supone.
#[tauri::command]
fn doctor_arreglar(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::DoctorArreglar)
}

/// Los despliegues vivos, para poder cancelarlos.
///
/// Se pueden lanzar **varios a la vez**, en apps distintas y en servidores
/// distintos: cada uno es un proceso SSH independiente y nada en el servidor
/// los coordina, ni falta. La clave es `servidor:app` y no la app sola, porque
/// `tienda` existe en tres servidores y son tres despliegues distintos.
#[derive(Default)]
pub struct Vivos(Mutex<HashMap<String, EnCurso>>);

/// Despliega **sirviendo el progreso mientras ocurre**.
///
/// Cada línea de `--progress` sale como un evento `orbit://progreso` en cuanto
/// llega. Leerlo entero al terminar convertiría tres minutos de información en
/// un bloque de texto que llega cuando ya no sirve.
///
/// Lo que **no** hace, y es lo que importa: si el contacto se pierde, no dice
/// que el despliegue haya fallado. **No lo sabe.** El proceso sigue en el
/// servidor y la respuesta honesta es «he perdido el contacto; el estado es
/// desconocido», con `orbit info` como forma de averiguarlo. Y no reintenta
/// solo: un `deploy` reintentado sobre uno en curso es, en el mejor caso, dos
/// releases.
#[tauri::command]
async fn desplegar(
    ventana: tauri::Window,
    vivos: tauri::State<'_, Vivos>,
    alias: String,
    app: String,
    binario: Option<String>,
) -> Resultado {
    let clave = format!("{alias}:{app}");
    let mando = EnCurso::nuevo();
    vivos.0.lock().unwrap().insert(clave.clone(), mando.clone());

    let s = servidor(&alias, binario);
    let c = Comando::Desplegar {
        app: app.clone(),
        progreso: true,
    };
    let ctrl = dir_control();
    let clave_ev = clave.clone();

    // En un hilo aparte: la llamada bloquea minutos, y bloquear el hilo de la
    // interfaz durante un build es la diferencia entre una ventana viva y una
    // que Windows marca como «no responde».
    let r = tauri::async_runtime::spawn_blocking(move || {
        transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |linea| {
            // El evento lleva la clave para poder atribuirlo: con varios
            // despliegues a la vez, una línea sin dueño no vale para nada.
            let _ = ventana.emit("orbit://progreso", (clave_ev.clone(), linea));
        })
    })
    .await;

    vivos.0.lock().unwrap().remove(&clave);

    let r = match r {
        Ok(v) => v?,
        Err(e) => {
            return Err(ErrorParaLaInterfaz {
                clase: "hilo",
                mensaje: format!("el despliegue se ha interrumpido: {e}"),
                detalle: None,
            })
        }
    };
    let texto = r.objeto()?;
    serde_json::from_str(texto).map_err(|e| ErrorParaLaInterfaz {
        clase: "json",
        mensaje: e.to_string(),
        detalle: None,
    })
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
    let mando = EnCurso::nuevo();
    vivos.0.lock().unwrap().insert(clave.clone(), mando.clone());

    let s = servidor(&alias, binario);
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
    let clave_ev = clave.clone();

    let r = tauri::async_runtime::spawn_blocking(move || {
        transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |linea| {
            let _ = ventana.emit("orbit://progreso", (clave_ev.clone(), linea));
        })
    })
    .await;

    vivos.0.lock().unwrap().remove(&clave);

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
    let mando = EnCurso::nuevo();
    vivos.0.lock().unwrap().insert(clave.clone(), mando.clone());

    let s = servidor(&alias, binario);
    let c = Comando::DesplegarTodo {
        progreso: true,
        solo_si_cambia,
    };
    let ctrl = dir_control();
    let clave_ev = clave.clone();

    let r = tauri::async_runtime::spawn_blocking(move || {
        transporte::ejecutar_en_vivo(&s, &c, ctrl.as_deref(), &[], mando, move |linea| {
            let _ = ventana.emit("orbit://progreso", (clave_ev.clone(), linea));
        })
    })
    .await;

    vivos.0.lock().unwrap().remove(&clave);

    let r = match r {
        Ok(v) => v?,
        Err(e) => {
            return Err(ErrorParaLaInterfaz {
                clase: "hilo",
                mensaje: format!("la pasada se ha interrumpido: {e}"),
                detalle: None,
            })
        }
    };
    let texto = r.objeto()?;
    serde_json::from_str(texto).map_err(|e| ErrorParaLaInterfaz {
        clase: "json",
        mensaje: e.to_string(),
        detalle: None,
    })
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
    let destino = alias.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let del_dominio = direcciones(&nombre);
        // A dónde va el `ssh`: se lo preguntamos a `ssh -G`, que es quien lo
        // sabe, en vez de leer el fichero por nuestra cuenta.
        let del_servidor = descubrir::resolver(&destino, None)
            .and_then(|a| a.hostname)
            .map(|h| direcciones(&h))
            .unwrap_or_default();

        // «Coinciden» sólo se afirma cuando hay las dos listas y comparten algo.
        // Con cualquiera de las dos vacía la respuesta es «no lo sé», que no es
        // «no coinciden».
        let coinciden = if del_dominio.is_empty() || del_servidor.is_empty() {
            None
        } else {
            Some(del_dominio.iter().any(|d| del_servidor.contains(d)))
        };

        Resolucion {
            del_dominio,
            del_servidor,
            coinciden,
        }
    })
    .await
    .map_err(|e| ErrorParaLaInterfaz {
        clase: "hilo",
        mensaje: format!("no he podido resolver el nombre: {e}"),
        detalle: None,
    })
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
fn entorno(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::EntornoLista { app })
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
fn entorno_valor(
    alias: String,
    app: String,
    clave: String,
    binario: Option<String>,
) -> Result<String, ErrorParaLaInterfaz> {
    let s = servidor(&alias, binario);
    let c = Comando::EntornoValor { app, clave };
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    // Pelado y sin adornos, que es como lo imprime Orbit. El salto final se
    // quita porque es del `printf`, no del valor.
    Ok(r.stdout.trim_end_matches('\n').to_string())
}

/// El monitor. **Tarda ~2,1 s con 40 apps**, y no es lentitud: la CPU es la
/// diferencia entre dos lecturas del cgroup, así que una foto suelta tiene que
/// esperar a la segunda a propósito.
#[tauri::command]
fn monitor(alias: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Top)
}

/// El tráfico de una app, del log que nginx ya escribe.
///
/// Sin cookies, sin JavaScript y sin nada nuevo corriendo: es lo que hace que
/// una analítica quepa aquí cuando un panel web no cabe. Y el dato lleva ahí
/// desde la primera visita.
#[tauri::command]
fn trafico(
    alias: String,
    app: String,
    desde: Option<String>,
    binario: Option<String>,
) -> Resultado {
    pedir(&alias, binario, Comando::Trafico { app, desde })
}

/// El histórico de despliegues de una app.
#[tauri::command]
fn metricas(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Metricas { app: Some(app) })
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
/// `shell: true` manda el texto como **un** argumento y el servidor lo pasa a
/// `bash -lc`; `false` manda los argumentos separados y no hay shell. La
/// diferencia se elige en la pantalla y se ve, en vez de deducirla de la
/// heurística del servidor: quien escribe tiene que poder predecir cuándo su
/// `&&` se ejecuta y cuándo se pasa como texto.
fn correr(
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
    let s = servidor(&alias, binario);
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
fn detalle(alias: String, app: String, binario: Option<String>) -> Resultado {
    pedir(&alias, binario, Comando::Info { app })
}

/// Retira una app **sin borrar sus datos**. Reversible.
#[tauri::command]
fn retirar(alias: String, app: String, binario: Option<String>) -> Resultado {
    // 'remove' no habla JSON, así que la respuesta es prosa. Se devuelve tal
    // cual y la interfaz vuelve a preguntar por el estado, que es la regla:
    // lo que cuenta es cómo queda el servidor, no lo que dijo el comando.
    let s = servidor(&alias, binario);
    let c = Comando::Retirar { app };
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(serde_json::json!({ "salida": r.stderr, "codigo": r.codigo }))
}

/// Retira una app **y borra sus datos**. Irreversible.
///
/// Aquí no hay red debajo: `orbit remove -y --purge` no pregunta nada, así que
/// toda la protección está en la pantalla que llama a esto.
#[tauri::command]
fn retirar_y_borrar(alias: String, app: String, binario: Option<String>) -> Resultado {
    let s = servidor(&alias, binario);
    let c = Comando::RetirarYBorrar { app };
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(serde_json::json!({ "salida": r.stderr, "codigo": r.codigo }))
}

/// Vuelve a una release anterior. La release es **obligatoria**: sin ella y sin
/// terminal, `orbit rollback` aborta — y hace bien, porque el «valor por
/// defecto» sería la que ya está activa.
#[tauri::command]
fn revertir(alias: String, app: String, release: String, binario: Option<String>) -> Resultado {
    let s = servidor(&alias, binario);
    let c = Comando::Revertir { app, release };
    let r = transporte::ejecutar(&s, &c, dir_control().as_deref(), &[])?;
    Ok(serde_json::json!({ "salida": r.stderr, "codigo": r.codigo }))
}

/// Qué hay al otro lado de un alias, antes de fiarse de nada.
///
/// Sin esta clasificación, quien añade un servidor ve «error» y no sabe si es
/// **su clave, su red o su servidor**. Es la diferencia entre un producto y una
/// demo, y por eso es un comando propio y no un efecto secundario de la primera
/// pantalla que se abra.
#[tauri::command]
fn saludar(
    alias: String,
    binario: Option<String>,
) -> Result<serde_json::Value, ErrorParaLaInterfaz> {
    let s = servidor(&alias, binario);
    let c = Comando::Version;
    let saludo = match transporte::ejecutar(&s, &c, dir_control().as_deref(), &[]) {
        Ok(r) => orbit_client::saludo::clasificar(&r),
        // Un fallo de `orbit` con salida también se clasifica: un Orbit a
        // medias sale con 1 y su prosa es justamente el dato.
        Err(transporte::ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => orbit_client::saludo::clasificar(&transporte::Respuesta {
            stdout,
            stderr,
            codigo,
        }),
        Err(e) => orbit_client::saludo::de_error(&e),
    };
    Ok(serde_json::json!({
        "clase": clase_de(&saludo),
        "version": version_de(&saludo),
        "contrato": contrato_de(&saludo),
        "motivo": motivo_de(&saludo),
        "puede_operar": saludo.permite_operar(),
        "puede_leer": saludo.permite_leer(),
        "orden_de_instalacion": orbit_client::saludo::ORDEN_DE_INSTALACION,
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
#[tauri::command]
fn servidores_del_config() -> Result<Vec<descubrir::AliasSsh>, ErrorParaLaInterfaz> {
    let ruta = descubrir::ruta_por_defecto().ok_or(ErrorParaLaInterfaz {
        clase: "sin-home",
        mensaje: "no sé dónde está tu carpeta personal".into(),
        detalle: None,
    })?;
    Ok(descubrir::descubrir(&ruta))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn ejecutar() {
    tauri::Builder::default()
        .manage(Vivos::default())
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
            monitor,
            trafico,
            metricas,
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
