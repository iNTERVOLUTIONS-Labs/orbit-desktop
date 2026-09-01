//! El mismo núcleo, en un terminal.
//!
//! # Por qué existe, que no es obvio
//!
//! La pregunta que decide la forma de este programa es: **¿qué da esto que no dé
//! `ssh servidor orbit list`?** Si la respuesta fuera «nada», sobraría — y lo
//! honesto sería no escribirlo.
//!
//! La respuesta es **el abanico**. `orbit list` corre en un servidor. Lo que
//! este cliente sabe hacer, y la orden no, es preguntarle a los diez a la vez y
//! poner las respuestas en una tabla con el mismo vocabulario de estados: los
//! servidores salen del `~/.ssh/config`, las conexiones se multiplexan con el
//! `ControlMaster` que ya tiene abierto la ventana, y **un servidor que no
//! contesta sale diciendo que no contesta**, no con cero apps.
//!
//! Todo lo demás —una app suelta, un log, un despliegue— se hace mejor con
//! `orbit` por `ssh`, y por eso no está aquí. Un cliente de terminal que
//! reenvuelva órdenes que ya existen es una capa que sólo puede quedarse vieja.
//!
//! # Lo que NO es
//!
//! **No es una interfaz de pantalla completa.** No hay modo crudo, ni teclas, ni
//! redibujado: imprime y termina. Eso no es una limitación que haya que levantar
//! algún día — es lo que la hace utilizable donde de verdad hace falta un
//! cliente de terminal, que es dentro de otro `ssh`, en un `watch`, o en la
//! salida de un script.
//!
//! # Qué demuestra
//!
//! El `Cargo.toml` del núcleo lleva desde el primer día una afirmación sin
//! comprobar: *«una TUI podría reutilizarlo sin duplicar nada»*. Esto es lo que
//! había que escribir para saber si era verdad. Lo que salió está en
//! `ROADMAP.md`: era verdad para el transporte, el contrato y la resolución del
//! estado, y **era falso para el vocabulario** —las palabras vivían sólo en la
//! interfaz— y para el directorio de los sockets de control. Las dos cosas se
//! movieron al núcleo, que es donde tenían que estar desde que hubo dos
//! interfaces.

use std::io::{IsTerminal, Write};

use orbit_client::comando::Comando;
use orbit_client::contrato::{Lista, Salud};
use orbit_client::descubrir;
use orbit_client::transporte::{self, dir_control, EnCurso, Servidor};
use orbit_client::AliasSsh;

fn main() {
    let crudos: Vec<String> = std::env::args().skip(1).collect();
    let (opciones, args) = match Opciones::leer(&crudos) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let orden = args.first().map(String::as_str).unwrap_or("ayuda");

    let rc = match orden {
        "estado" => estado(&opciones, &args[1..]),
        "servidores" => servidores(&opciones),
        "desplegar" => desplegar(&opciones, &args[1..]),
        "ayuda" | "-h" | "--help" | "help" => {
            ayuda();
            0
        }
        otro => {
            eprintln!("No entiendo «{otro}».");
            ayuda();
            2
        }
    };
    std::process::exit(rc);
}

/// Lo que vale para cualquier orden.
///
/// De momento sólo `-F`, y no es un gancho para las pruebas: es lo que hace
/// `ssh`, y sin ello no hay forma de apuntar este programa a otro fichero —ni
/// para probarlo, ni para usar el config de un proyecto en vez del personal.
#[derive(Default)]
struct Opciones {
    config: Option<String>,
    /// Dónde está `orbit` en el otro lado. Por defecto `/usr/local/bin/orbit`,
    /// que es donde lo pone su instalador. **Siempre absoluta**: resolverlo por
    /// `PATH` sería dejar que el `PATH` del otro lado eligiera qué se ejecuta.
    binario: Option<String>,
}

impl Opciones {
    /// Devuelve las opciones y **el resto de los argumentos**, en orden.
    ///
    /// `-F` puede ir delante o detrás de la orden, como en `ssh`: quien lo
    /// escribe no tiene por qué acordarse de cuál de las dos cosas espera este
    /// programa.
    fn leer(args: &[String]) -> Result<(Self, Vec<String>), String> {
        let mut o = Self::default();
        let mut resto = Vec::new();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-F" | "--config" => {
                    i += 1;
                    match args.get(i) {
                        Some(v) => o.config = Some(v.clone()),
                        None => return Err("«-F» necesita la ruta de un fichero.".into()),
                    }
                }
                "--orbit" => {
                    i += 1;
                    match args.get(i) {
                        Some(v) if v.starts_with('/') => o.binario = Some(v.clone()),
                        Some(v) => {
                            return Err(format!(
                            "«--orbit {v}» tiene que ser una ruta absoluta: resolverla por PATH \
                                 sería dejar que el PATH del otro lado eligiera qué se ejecuta."
                        ))
                        }
                        None => return Err("«--orbit» necesita una ruta absoluta.".into()),
                    }
                }
                otro => resto.push(otro.to_string()),
            }
            i += 1;
        }
        Ok((o, resto))
    }

    /// Un servidor, con el fichero de configuración que toque.
    fn servidor(&self, alias: &str) -> Servidor {
        let mut s = Servidor::nuevo(alias, alias);
        s.config_ssh = self.config.clone();
        if let Some(b) = &self.binario {
            s.binario = b.clone();
        }
        s
    }
}

fn ayuda() {
    eprintln!(
        "\
orbit-desktop — el núcleo de Orbit Desktop, en un terminal

  estado [servidor…]        las apps de todos tus servidores, en una tabla
  servidores                los alias de tu ~/.ssh/config, SIN hablar con ninguno
  desplegar <servidor> <app>  despliega, sirviendo el progreso

  -F <fichero>              usa otro ~/.ssh/config, como «ssh -F»
  --orbit <ruta>            dónde está «orbit» en el otro lado (ruta absoluta)

Sin argumentos, «estado» pregunta a todos los servidores de tu ~/.ssh/config.

Esto no reemplaza a «orbit»: para una app suelta, un log o un exec, «ssh
servidor orbit …» es mejor. Lo que hace esto y esa orden no puede es preguntar a
varios servidores a la vez."
    );
}

// ── Presentación ────────────────────────────────────────────────────────────

/// El color se apaga solo cuando no hay un terminal detrás.
///
/// Es la misma lección que Orbit se comió con su ruedecita: redirigido a un
/// fichero, cada código de escape se queda escrito y tapa lo único que
/// importaba de ese log.
struct Pinta(bool);

impl Pinta {
    fn nueva() -> Self {
        // `NO_COLOR` porque existe y cuesta tres caracteres respetarlo.
        Self(std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
    }

    fn de(&self, s: Salud, texto: &str) -> String {
        if !self.0 {
            return texto.to_string();
        }
        // Los mismos conceptos que la hoja de estilos de la ventana, con los
        // ocho colores del terminal. No se afinan más: un terminal con un tema
        // claro y otro con uno oscuro tienen que poder leerlos los dos, y los
        // básicos son los únicos que el usuario ha configurado a su gusto.
        let c = match s {
            Salud::SinVhost => "31", // rojo
            Salud::Parada => "31",
            Salud::Mantenimiento => "33", // ámbar
            Salud::Activa => "32",        // verde
            Salud::SinProceso => "90",    // gris: no aplica NO es un fallo
            Salud::Desconocida(_) => "90",
        };
        format!("\x1b[{c}m{texto}\x1b[0m")
    }

    fn tenue(&self, texto: &str) -> String {
        if self.0 {
            format!("\x1b[90m{texto}\x1b[0m")
        } else {
            texto.to_string()
        }
    }
}

/// Cuánto ocupa en pantalla, que no es `len()`.
///
/// Los glifos del vocabulario son de fuera de ASCII y `len()` cuenta bytes: con
/// él, la columna del estado salía desalineada exactamente en las filas que hay
/// que mirar. No se aproxima el ancho de los CJK ni de los emoji porque aquí no
/// hay ninguno — lo que hay son seis símbolos conocidos y nombres de app, que la
/// regla del servidor limita a `[a-z0-9._-]`.
fn ancho(s: &str) -> usize {
    s.chars().count()
}

fn relleno(s: &str, n: usize) -> String {
    let a = ancho(s);
    if a >= n {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(n - a))
    }
}

// ── Órdenes ─────────────────────────────────────────────────────────────────

/// Los alias del `~/.ssh/config`.
///
/// Que no haya fichero **no es un error**: mucha gente no lo tiene, y el núcleo
/// ya devuelve una lista vacía en vez de fallar. Aquí se respeta esa decisión en
/// vez de convertirla en un mensaje de error.
fn alias_del_config(o: &Opciones) -> Vec<AliasSsh> {
    match &o.config {
        Some(c) => descubrir::descubrir(std::path::Path::new(c)),
        None => match descubrir::ruta_por_defecto() {
            Some(r) => descubrir::descubrir(&r),
            None => Vec::new(),
        },
    }
}

fn servidores(o: &Opciones) -> i32 {
    let alias = alias_del_config(o);
    if alias.is_empty() {
        // Una colección vacía es una respuesta, y se dice como tal.
        println!("No hay ningún Host en tu ~/.ssh/config, o el fichero no existe.");
        println!("No es un error: mucha gente no lo tiene.");
        return 0;
    }

    let p = Pinta::nueva();
    // Enumerar no es visitar. Decirlo aquí y no en la documentación: quien lea
    // esta lista se va a preguntar por qué no dice si están vivos.
    println!(
        "{}",
        p.tenue("Salen de tu ~/.ssh/config. Enumerarlos no habla con ninguno.")
    );
    let n = alias.iter().map(|a| ancho(&a.alias)).max().unwrap_or(0);
    for a in &alias {
        println!("{}  {}", relleno(&a.alias, n), p.tenue(&donde(a)));
    }
    0
}

fn donde(a: &AliasSsh) -> String {
    match &a.hostname {
        None => "lo resuelve tu ~/.ssh/config".into(),
        Some(h) => {
            let mut s = String::new();
            if let Some(u) = &a.usuario {
                s.push_str(u);
                s.push('@');
            }
            s.push_str(h);
            if let Some(pt) = a.puerto {
                if pt != 22 {
                    s.push_str(&format!(":{pt}"));
                }
            }
            if let Some(j) = &a.salto {
                s.push_str(&format!(" · por {j}"));
            }
            s
        }
    }
}

/// Lo que contestó un servidor. **Tres casos y no dos**: hay apps, no hay
/// ninguna, o no se ha podido preguntar — y el tercero no es el segundo.
enum Respuesta {
    Apps(Vec<orbit_client::App>),
    NoContesta(String),
}

fn estado(o: &Opciones, args: &[String]) -> i32 {
    let elegidos: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();

    let alias: Vec<String> = if elegidos.is_empty() {
        alias_del_config(o).into_iter().map(|a| a.alias).collect()
    } else {
        elegidos
    };

    if alias.is_empty() {
        println!("No hay ningún servidor en tu ~/.ssh/config.");
        return 0;
    }

    // El abanico, que es la razón de ser de este programa. Un hilo por servidor
    // y no una cola: lo que hacen es esperar a un `ssh`, así que ponerlos en
    // serie multiplicaría la espera por el número de servidores sin ahorrar
    // nada. Con diez servidores es la diferencia entre tres segundos y treinta.
    let ctrl = dir_control();
    let mut respuestas: Vec<(String, Respuesta)> = std::thread::scope(|s| {
        let hilos: Vec<_> = alias
            .iter()
            .map(|a| {
                let ctrl = ctrl.clone();
                s.spawn(move || {
                    let srv = o.servidor(a);
                    let r = transporte::ejecutar(&srv, &Comando::Estado, ctrl.as_deref(), &[]);
                    let respuesta = match r.and_then(|r| r.leer::<Lista>()) {
                        Ok(l) => Respuesta::Apps(l.apps),
                        Err(e) => Respuesta::NoContesta(e.to_string()),
                    };
                    (a.clone(), respuesta)
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // En el orden en que se pidieron, no en el que contestaron: una tabla que
    // cambia de orden entre dos ejecuciones no se puede comparar de un vistazo.
    respuestas.sort_by_key(|(a, _)| alias.iter().position(|x| x == a).unwrap_or(usize::MAX));

    let p = Pinta::nueva();
    let ancho_srv = alias.iter().map(|a| ancho(a)).max().unwrap_or(0).max(9);
    let ancho_app = respuestas
        .iter()
        .filter_map(|(_, r)| match r {
            Respuesta::Apps(v) => v.iter().map(|a| ancho(&a.name)).max(),
            _ => None,
        })
        .max()
        .unwrap_or(3)
        .max(3);

    println!(
        "{}",
        p.tenue(&format!(
            "{}  {}  {}",
            relleno("SERVIDOR", ancho_srv),
            relleno("APP", ancho_app),
            "ESTADO"
        ))
    );

    let mut mudos = 0;
    let mut total = 0;
    for (a, r) in &respuestas {
        match r {
            Respuesta::NoContesta(motivo) => {
                mudos += 1;
                // **No es una fila vacía.** «No he podido preguntar» y «no tiene
                // apps» son dos cosas distintas, y confundirlas es el fallo que
                // ya costó que un remoto caído se anunciara como «nada que
                // hacer» durante días.
                println!(
                    "{}  {}",
                    relleno(a, ancho_srv),
                    p.de(Salud::SinVhost, "no contesta")
                );
                println!("{}  {}", relleno("", ancho_srv), p.tenue(motivo));
            }
            Respuesta::Apps(apps) if apps.is_empty() => {
                println!("{}  {}", relleno(a, ancho_srv), p.tenue("sin apps todavía"));
            }
            Respuesta::Apps(apps) => {
                for app in apps {
                    total += 1;
                    let s = app.state.salud();
                    // `rotulo()` y no glifo + texto: en los dos estados neutros
                    // el texto ES el glifo, y pintarlos los dos deja una fila
                    // que dice «— —». La ventana ya se comió ese defecto, y
                    // aquí se volvió a cometer a la primera — por eso la regla
                    // vive en el núcleo y no en cada interfaz.
                    println!(
                        "{}  {}  {}",
                        relleno(a, ancho_srv),
                        relleno(&app.name, ancho_app),
                        p.de(s, &s.rotulo()),
                    );
                }
            }
        }
    }

    println!();
    println!(
        "{}",
        p.tenue(&format!(
            "{total} apps en {} de {} servidores.",
            respuestas.len() - mudos,
            respuestas.len()
        ))
    );
    if mudos > 0 {
        // El recuento de los mudos va aparte y siempre, también cuando es cero
        // en la frase de arriba: es el dato que se pierde al resumir.
        println!(
            "{}",
            p.de(
                Salud::SinVhost,
                &format!(
                    "{mudos} {} no {}. Lo que tengan no se sabe.",
                    if mudos == 1 { "servidor" } else { "servidores" },
                    if mudos == 1 { "contesta" } else { "contestan" }
                )
            )
        );
    }

    // Un servidor mudo NO es un error del programa: la respuesta es correcta y
    // dice lo que sabe. El código de salida lo refleja para poder encadenarlo,
    // pero con su propio valor, distinto del 1 de «no he podido ni empezar».
    if mudos > 0 {
        3
    } else {
        0
    }
}

fn desplegar(o: &Opciones, args: &[String]) -> i32 {
    let (alias, app) = match (args.first(), args.get(1)) {
        (Some(a), Some(b)) => (a.clone(), b.clone()),
        _ => {
            eprintln!("uso: orbit-desktop desplegar <servidor> <app>");
            return 2;
        }
    };

    let p = Pinta::nueva();
    let srv = o.servidor(&alias);
    let c = Comando::Desplegar {
        app: app.clone(),
        progreso: true,
    };

    println!("{}", p.tenue(&format!("Desplegando {app} en {alias}…")));

    let mando = EnCurso::nuevo();
    let r = transporte::ejecutar_en_vivo(
        &srv,
        &c,
        dir_control().as_deref(),
        &[],
        mando,
        move |linea| {
            // Las líneas del progreso son NDJSON mezclado con la prosa que Orbit
            // le cuenta a una persona. Aquí se enseñan **las dos tal cual**: el
            // lector de sucesos vive en el núcleo y sirve para pintar una barra,
            // y en un terminal sin redibujado una barra no tiene dónde vivir.
            let t = linea.trim();
            if t.starts_with('{') {
                if let Some(paso) = paso_de(t) {
                    println!("  {paso}");
                }
            } else if !t.is_empty() {
                println!("  {t}");
            }
            let _ = std::io::stdout().flush();
        },
    );

    match r {
        Ok(resp) => {
            match resp.leer::<orbit_client::contrato::Despliegue>() {
                Ok(d) if d.ok => {
                    println!(
                        "{}",
                        p.de(
                            Salud::Activa,
                            &format!("Publicada: {}", d.release.unwrap_or_default())
                        )
                    );
                    0
                }
                Ok(d) => {
                    // Un despliegue que falla y revierte **no deja la web
                    // caída**, y decir sólo «ha fallado» esconde justo eso.
                    let paso = d.failed_step.unwrap_or_else(|| "?".into());
                    if let Some(previa) = d.previous {
                        println!(
                            "{}",
                            p.de(
                                Salud::Mantenimiento,
                                &format!(
                                    "Falló en {paso} y volvió a {previa}: la web sigue en pie."
                                )
                            )
                        );
                    } else {
                        println!("{}", p.de(Salud::SinVhost, &format!("Falló en {paso}.")));
                    }
                    1
                }
                Err(e) => {
                    eprintln!("no he entendido la respuesta: {e}");
                    1
                }
            }
        }
        Err(e) => {
            // Perder el contacto **no es que haya fallado**: el proceso sigue en
            // el servidor y el estado es desconocido. Decir «ha fallado» sería
            // afirmar algo que no se sabe, y decir «ha ido bien» también.
            eprintln!("{e}");
            eprintln!("Si se ha perdido el contacto, el despliegue puede seguir en el servidor.");
            eprintln!("Para averiguarlo:  ssh {alias} orbit info {app}");
            4
        }
    }
}

/// Un suceso de progreso, resumido en una línea legible.
///
/// Se usa el lector del núcleo en vez de mirar el JSON a mano: es el mismo que
/// alimenta la barra de la ventana, y tiene ya resuelto que **una línea rota no
/// tumba un despliegue**.
fn paso_de(linea: &str) -> Option<String> {
    let (sucesos, _) = orbit_client::leer_progreso(linea);
    let s = sucesos.first()?;
    let estado = s.status.as_deref().unwrap_or("");
    match s.event.as_str() {
        "step" => {
            let paso = s.step.as_deref()?;
            let como = match estado {
                "start" => "…",
                "ok" => "✔",
                _ => estado,
            };
            Some(format!("{como} {paso}"))
        }
        // Una pasada por varias apps también pasa por aquí: los dos niveles
        // viajan por el mismo canal y los dos traen `app`.
        "app" => Some(format!("{} {}", s.app.as_deref().unwrap_or("?"), estado)),
        _ => None,
    }
}
