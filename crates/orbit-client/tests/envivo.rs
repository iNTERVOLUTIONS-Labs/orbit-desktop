//! El canal en vivo: servir el progreso mientras ocurre.
//!
//! La diferencia entre esto y leer el flujo al terminar no es de estilo: un
//! despliegue tarda minutos, y un bloque de texto que llega al final es
//! información que llega cuando ya no sirve.
//!
//! Estas pruebas usan pausas de verdad, que es lo único que distingue las dos
//! cosas. Sin ellas, «se sirve según llega» y «se lee entero al final» se ven
//! exactamente igual y la prueba no probaría nada.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use orbit_client::comando::Comando;
use orbit_client::contrato::Despliegue;
use orbit_client::flujo::leer_progreso;
use orbit_client::transporte::{ejecutar_en_vivo_local, EnCurso};

fn falso() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fakeserver/orbit")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn el_progreso_llega_segun_ocurre_y_no_al_final() {
    let llegadas: Arc<Mutex<Vec<(u128, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let acc = Arc::clone(&llegadas);
    let t0 = Instant::now();

    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: true,
    };
    let r = ejecutar_en_vivo_local(
        &falso(),
        &c,
        &[
            ("ORBIT_FAKE_CASE", "deploy-lento"),
            ("ORBIT_FAKE_PAUSA", "0.15"),
        ],
        EnCurso::nuevo(),
        move |l| {
            acc.lock().unwrap().push((t0.elapsed().as_millis(), l));
        },
    )
    .expect("el despliegue tiene que terminar bien");

    let v = llegadas.lock().unwrap();
    assert_eq!(v.len(), 12, "seis pasos, dos sucesos cada uno");

    // La prueba de que se sirve y no se acumula: la primera línea llega **mucho
    // antes** que la última. Si se leyera entero al final, las dos llegarían a
    // la vez.
    let primera = v.first().unwrap().0;
    let ultima = v.last().unwrap().0;
    assert!(
        ultima - primera > 400,
        "las líneas llegaron todas juntas ({primera} ms → {ultima} ms): eso es leer al final, no servir"
    );

    // Y por stdout, el objeto, sin haberse mezclado con nada.
    let d: Despliegue = r.leer().unwrap();
    assert!(d.ok);
}

#[test]
fn lo_servido_y_lo_acumulado_dicen_lo_mismo() {
    // Las líneas que se sirven una a una y el stderr completo tienen que
    // coincidir: si divergieran, la pantalla en vivo y la vista cruda contarían
    // historias distintas del mismo despliegue.
    let vistas: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let acc = Arc::clone(&vistas);
    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: true,
    };
    let r = ejecutar_en_vivo_local(
        &falso(),
        &c,
        &[
            ("ORBIT_FAKE_CASE", "deploy-lento"),
            ("ORBIT_FAKE_PAUSA", "0.02"),
        ],
        EnCurso::nuevo(),
        move |l| acc.lock().unwrap().push(l),
    )
    .unwrap();

    let servidas = vistas.lock().unwrap().join("\n");
    let (a, _) = leer_progreso(&servidas);
    let (b, _) = leer_progreso(&r.stderr);
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len(), 12);
}

#[test]
fn se_puede_cancelar_un_despliegue_en_curso() {
    // El mando se crea ANTES de llamar, que es lo que hace posible cancelar:
    // devolverlo al terminar lo hacía inútil, porque para entonces ya no había
    // nada que cancelar. Lo destapó escribir esta prueba, no revisar el diseño.
    let mando = EnCurso::nuevo();
    let otro = mando.clone();
    let visto = Arc::new(Mutex::new(0usize));
    let acc = Arc::clone(&visto);

    let inicio = Instant::now();
    let hilo = std::thread::spawn(move || {
        let c = Comando::Desplegar {
            app: "mi-web".into(),
            progreso: true,
        };
        ejecutar_en_vivo_local(
            &falso(),
            &c,
            &[("ORBIT_FAKE_CASE", "deploy-eterno")],
            otro,
            move |_| {
                *acc.lock().unwrap() += 1;
            },
        )
    });

    // Se le da tiempo a arrancar y a emitir su primer paso, y se cancela desde
    // fuera — que es exactamente lo que hará la interfaz cuando alguien pulse
    // el botón.
    std::thread::sleep(std::time::Duration::from_millis(400));
    mando.cancelar();

    let r = hilo.join().expect("el hilo no puede entrar en pánico");
    let r = r.expect("cancelar NO es fallar: tiene que volver con una respuesta");

    // Un canal en vivo que no se pueda cerrar deja un proceso colgado por cada
    // despliegue que alguien abandone.
    assert!(
        inicio.elapsed() < std::time::Duration::from_secs(20),
        "no puede tardar el plazo entero: se cancela, no se espera"
    );
    assert!(
        *visto.lock().unwrap() >= 1,
        "el primer paso sí llegó antes de cancelar"
    );

    // Y lo importante: cancelado no es fallido. El proceso remoto puede haber
    // terminado su paso antes de morir, así que afirmar «ha fallado» sería
    // afirmar algo que no se sabe — el mismo error que el contrato de seis
    // finales existe para que un cliente no cometa.
    assert!(mando.cancelado());
    assert!(
        r.stdout.trim().is_empty(),
        "un despliegue cancelado no trae objeto final, y eso NO se lee como un fallo"
    );
}

/// La orden más larga del catálogo es la única que habla por stdout, y servir
/// la tubería equivocada no da un error: da una pantalla muda durante tres
/// minutos, que es la clase de fallo que sólo aparece en producción.
///
/// Esta prueba existe porque el primer transporte servía stderr siempre. Contra
/// todo el resto del catálogo eso era correcto —`--json` manda la prosa a
/// stderr— y por eso pasó desapercibido: el único contraejemplo es `new`.
#[test]
fn la_prosa_de_new_llega_aunque_salga_por_stdout() {
    use orbit_client::comando::{AjustesDeteccion, Vena};

    let c = Comando::Nueva(Box::new(orbit_client::comando::WebNueva {
        nombre: "mi-web".into(),
        repo: "usuario/mi-web".into(),
        rama: "main".into(),
        dominio: "mi-web.ejemplo.com".into(),
        alias: vec![],
        correo: None,
        base_de_datos: false,
        https: true,
        ajustes: AjustesDeteccion::default(),
    }));
    assert_eq!(
        c.vena_humana(),
        Vena::Stdout,
        "sin --json, _ui_route deja UI_FD=1"
    );

    let llegadas: Arc<Mutex<Vec<(u128, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let acc = Arc::clone(&llegadas);
    let t0 = Instant::now();

    let r = ejecutar_en_vivo_local(
        &falso(),
        &c,
        &[
            ("ORBIT_FAKE_CASE", "new-lento"),
            ("ORBIT_FAKE_PAUSA", "0.15"),
        ],
        EnCurso::nuevo(),
        move |l| acc.lock().unwrap().push((t0.elapsed().as_millis(), l)),
    );

    // `new` termina en 1 con la aplicación creada, así que el transporte no
    // puede tratar ese código como un fallo del que no se saca nada: la prosa
    // ya ha llegado y el estado real se pregunta después con `info --json`.
    let salida = match r {
        Ok(r) => r.stdout,
        Err(orbit_client::transporte::ErrorTransporte::Orbit { stdout, .. }) => stdout,
        Err(e) => panic!("no debería ser un fallo de transporte: {e}"),
    };

    let v = llegadas.lock().unwrap();
    assert_eq!(v.len(), 6, "seis líneas de prosa, servidas");
    assert!(
        v[0].1.contains("Clonando"),
        "la primera línea es la del clon, y llegó: {:?}",
        v[0].1
    );

    // Y llegaron **mientras ocurría**, no de golpe al final.
    let (primera, ultima) = (v.first().unwrap().0, v.last().unwrap().0);
    assert!(
        ultima > primera + 300,
        "servidas según ocurren: primera a {primera} ms, última a {ultima} ms"
    );

    // La `Respuesta` no intercambia nada: lo que salió por stdout sigue
    // estando en `stdout`.
    assert!(salida.contains("Detectado: next"));
}

/// El encaminamiento se deduce del propio `argv` y no se apunta a mano en una
/// lista paralela, así que vale igual para las órdenes que se añadan mañana.
/// Lo que se comprueba aquí es que la deducción coincide con la regla de
/// `_ui_route` en los dos sentidos.
#[test]
fn quien_lleva_json_habla_por_stderr_y_quien_no_por_stdout() {
    use orbit_client::comando::{AjustesDeteccion, ModoDeExec, Vena};

    let muestra = [
        Comando::Version,
        Comando::Lista,
        Comando::Estado,
        Comando::Doctor,
        Comando::DoctorArreglar,
        Comando::Top,
        Comando::Info { app: "web".into() },
        Comando::Desplegar {
            app: "web".into(),
            progreso: true,
        },
        Comando::EntornoLista { app: "web".into() },
        Comando::EntornoValor {
            app: "web".into(),
            clave: "PORT".into(),
        },
        Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Argumentos(vec!["ls".into()]),
        },
        Comando::Retirar { app: "web".into() },
        Comando::RetirarYBorrar { app: "web".into() },
        Comando::Revertir {
            app: "web".into(),
            release: "20260830-120000".into(),
        },
        Comando::Nueva(Box::new(orbit_client::comando::WebNueva {
            nombre: "web".into(),
            repo: "usuario/web".into(),
            rama: "main".into(),
            dominio: "web.ejemplo.com".into(),
            alias: vec![],
            correo: None,
            base_de_datos: false,
            https: true,
            ajustes: AjustesDeteccion::default(),
        })),
    ];

    let mut por_stdout = 0;
    for c in &muestra {
        let argv = c.argv("orbit").expect("la muestra es toda válida");
        let esperada = if argv.iter().any(|a| a == "--json") {
            Vena::Stderr
        } else {
            Vena::Stdout
        };
        if esperada == Vena::Stdout {
            por_stdout += 1;
        }
        assert_eq!(c.vena_humana(), esperada, "en {c:?}");
    }

    // Y que la muestra cubre de verdad los dos lados: una prueba en la que
    // todas las órdenes cayeran del mismo lado pasaría sin comprobar nada.
    assert_eq!(
        por_stdout, 6,
        "las seis sin --json: env get, exec, remove, remove --purge, rollback y new"
    );
}
