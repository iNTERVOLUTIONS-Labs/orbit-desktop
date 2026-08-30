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

use orbit_client::comando::Comando;
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
            desplegar,
            cancelar,
            servidores_del_config,
        ])
        .run(tauri::generate_context!())
        .expect("no he podido abrir la ventana");
}
