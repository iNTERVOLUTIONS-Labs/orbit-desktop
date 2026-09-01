//! El vocabulario de estados es el mismo en las dos interfaces.
//!
//! La precedencia ya vivía aquí —`Estado::salud()`, con su orden escrito— pero
//! las palabras estaban sólo en la interfaz. Mientras hubo una interfaz eso no
//! se notaba; con dos es la forma exacta en que se disuelve el activo más
//! valioso del producto: basta con que una diga «parado» donde la otra dice «no
//! aplica» para que la distinción entre «no hay proceso» y «el proceso se ha
//! caído» deje de existir.
//!
//! La prueba gemela está en `ui/tests/honestidad.test.ts` y lee el mismo
//! fichero.

use orbit_client::contrato::Salud;

fn fixture() -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/contrato/vocabulario.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn las_palabras_son_las_del_fichero_compartido() {
    let f = fixture();
    let esperados = f["estados"].as_array().unwrap();

    // En el mismo orden, que es el de la precedencia: lo que impide servir gana
    // sobre lo que sólo degrada.
    let reales = [
        Salud::SinVhost,
        Salud::Mantenimiento,
        Salud::SinProceso,
        Salud::Activa,
        Salud::Parada,
        Salud::Desconocida("lo que sea"),
    ];
    assert_eq!(reales.len(), esperados.len(), "seis estados, ni uno más");

    for (real, esperado) in reales.iter().zip(esperados) {
        let id = esperado["id"].as_str().unwrap();
        assert_eq!(real.id(), id);
        assert_eq!(
            real.glifo(),
            esperado["glifo"].as_str().unwrap(),
            "glifo de {id}"
        );
        assert_eq!(
            real.texto(),
            esperado["texto"].as_str().unwrap(),
            "texto de {id}"
        );
        assert_eq!(real.voz(), esperado["voz"].as_str().unwrap(), "voz de {id}");
        assert_eq!(
            real.frase(),
            esperado["frase"].as_str().unwrap(),
            "frase de {id}"
        );
    }
}

/// Los dos neutros tienen glifos **distintos**, y ninguno de los dos es el del
/// resto. «No aplica» y «no se sabe» son dos cosas distintas: la primera es una
/// respuesta y la segunda es un hueco.
#[test]
fn los_dos_neutros_no_se_confunden() {
    assert_ne!(Salud::SinProceso.glifo(), Salud::Desconocida("x").glifo());
    assert_ne!(Salud::SinProceso.voz(), Salud::Desconocida("x").voz());

    let glifos: Vec<&str> = [
        Salud::SinVhost,
        Salud::Mantenimiento,
        Salud::SinProceso,
        Salud::Activa,
        Salud::Parada,
        Salud::Desconocida("x"),
    ]
    .iter()
    .map(|s| s.glifo())
    .collect();
    let mut unicos = glifos.clone();
    unicos.sort_unstable();
    unicos.dedup();
    assert_eq!(unicos.len(), glifos.len(), "dos estados con el mismo glifo");
}

/// La voz nunca es un glifo. `—` se lee «raya», y eso no es «no aplica».
#[test]
fn lo_que_se_anuncia_es_una_palabra() {
    for s in [
        Salud::SinVhost,
        Salud::Mantenimiento,
        Salud::SinProceso,
        Salud::Activa,
        Salud::Parada,
        Salud::Desconocida("x"),
    ] {
        assert!(
            s.voz().chars().any(|c| c.is_alphabetic()),
            "«{}» no se puede leer en voz alta",
            s.voz()
        );
    }
}

/// El rótulo nunca repite el glifo.
///
/// La ventana ya se comió ese defecto —una fila que decía «— —»— y lo cazó
/// mirando una captura, no una prueba. El cliente de terminal lo cometió otra
/// vez, por su cuenta y a la primera, que es el argumento entero de que la regla
/// viva aquí.
#[test]
fn el_rotulo_no_pinta_el_glifo_dos_veces() {
    for s in [
        Salud::SinVhost,
        Salud::Mantenimiento,
        Salud::SinProceso,
        Salud::Activa,
        Salud::Parada,
        Salud::Desconocida("x"),
    ] {
        let r = s.rotulo();
        assert!(
            !r.contains(&format!("{g} {g}", g = s.glifo())),
            "«{r}» repite el glifo"
        );
        // Y sigue diciendo algo: un rótulo vacío no es una simplificación.
        assert!(!r.trim().is_empty());
    }
    assert_eq!(Salud::SinProceso.rotulo(), "—");
    assert_eq!(Salud::Activa.rotulo(), "● activo");
}
