//! La orden que se enseña y la orden que se ejecuta son la misma.
//!
//! El asistente de web nueva construye su orden dos veces: aquí, que es la que
//! sale por el `ssh`, y en la interfaz, que la enseña en el repaso para que
//! alguien pueda leerla antes de pulsar. Una pantalla cuyo único argumento es
//! «mira lo que va a pasar antes de que pase» no puede enseñar una cosa y
//! ejecutar otra, y sin nada que las ate eso se separa solo en el primer
//! cambio.
//!
//! Las dos se comparan contra `tests/contrato/orden-de-new.json`. La prueba
//! gemela está en `ui/tests/asistente.test.ts` y lee el mismo fichero.

use orbit_client::comando::{AjustesDeteccion, Anulacion, Comando, WebNueva};

fn fixture() -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/contrato/orden-de-new.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn cadenas(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn desde(e: &serde_json::Value) -> Comando {
    let txt = |k: &str| e[k].as_str().map(|s| s.to_string());
    let aj = &e["ajustes"];
    // El tri-estado viaja como `null` (callar), `["vacia"]` (vacío a propósito)
    // o una cadena. Tres formas distintas porque son tres respuestas distintas.
    let tri = |k: &str| -> Option<Anulacion> {
        match &aj[k] {
            serde_json::Value::Null => None,
            serde_json::Value::Array(_) => Some(Anulacion::Vacia),
            serde_json::Value::String(s) => Some(Anulacion::Valor(s.clone())),
            otro => panic!("no sé leer {otro:?}"),
        }
    };
    Comando::Nueva(Box::new(WebNueva {
        nombre: e["nombre"].as_str().unwrap().into(),
        repo: e["repo"].as_str().unwrap().into(),
        rama: e["rama"].as_str().unwrap().into(),
        dominio: e["dominio"].as_str().unwrap().into(),
        alias: cadenas(&e["alias"]),
        correo: txt("correo"),
        base_de_datos: e["base_de_datos"].as_bool().unwrap(),
        https: e["https"].as_bool().unwrap(),
        ajustes: AjustesDeteccion {
            carpeta: aj["carpeta"].as_str().map(String::from),
            tipo: aj["tipo"].as_str().map(String::from),
            build: tri("build"),
            arranque: tri("arranque"),
            outdir: tri("outdir"),
        },
    }))
}

#[test]
fn el_caso_completo_produce_el_argv_del_fichero() {
    let f = fixture();
    let v = desde(&f["entrada"]).argv("orbit").unwrap();
    assert_eq!(v[0], "orbit");
    assert_eq!(v[1..].to_vec(), cadenas(&f["argv"]));
}

#[test]
fn el_caso_normal_no_lleva_ni_una_bandera_de_mas() {
    let f = fixture();
    let m = &f["minimo"];
    let v = desde(&m["entrada"]).argv("orbit").unwrap();
    assert_eq!(v[1..].to_vec(), cadenas(&m["argv"]));
}
