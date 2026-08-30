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
