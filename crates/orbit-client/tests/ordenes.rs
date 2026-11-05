//! Las órdenes que se enseñan y las que se ejecutan son las mismas.
//!
//! La hoja de comando de un despliegue, de una congelación o de la puerta de la
//! CI enseña la orden antes de lanzarla, y la interfaz la construye por su
//! cuenta para enseñarla. Las dos se comparan contra
//! `tests/contrato/ordenes.json`; la gemela es `ui/tests/ordenes.test.ts`.
//!
//! Se compara `argv_visible`, que es lo que se enseña —con los valores de
//! `--env` tapados—, y se comprueba aparte que la única diferencia con el
//! `argv` de verdad son esos valores.

use orbit_client::comando::{Comando, Secreto, VariableNueva, OCULTO};

fn fixture() -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/contrato/ordenes.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn desde(o: &serde_json::Value) -> Comando {
    let app = o["app"].as_str().unwrap().to_string();
    match o["tipo"].as_str().unwrap() {
        "desplegar" => Comando::Desplegar {
            app,
            progreso: true,
            entorno: o["entorno"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| VariableNueva {
                    clave: e["clave"].as_str().unwrap().into(),
                    valor: Secreto::nuevo(e["valor"].as_str().unwrap()),
                })
                .collect(),
            forzar: o["forzar"].as_bool().unwrap(),
        },
        "congelar" => Comando::Congelar {
            app,
            motivo: o["motivo"].as_str().map(String::from),
        },
        "descongelar" => Comando::Descongelar { app },
        "esperar_ci" => Comando::EsperarCi {
            app,
            esperar: o["esperar"].as_bool().unwrap(),
        },
        otro => panic!("no sé construir «{otro}»"),
    }
}

#[test]
fn lo_que_se_ensena_es_el_fichero_y_no_lleva_ningun_valor() {
    let f = fixture();
    for caso in f["casos"].as_array().unwrap() {
        let nombre = caso["nombre"].as_str().unwrap();
        let c = desde(&caso["orden"]);
        let visible = c.argv_visible("orbit").unwrap();
        let esperado: Vec<String> = caso["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(visible[1..].to_vec(), esperado, "{nombre}");

        // Ningún valor de entrada aparece en lo que se enseña.
        if let Some(e) = caso["orden"]["entorno"].as_array() {
            for v in e {
                let valor = v["valor"].as_str().unwrap();
                assert!(
                    !visible.iter().any(|a| a.contains(valor)),
                    "{nombre}: se enseña un valor"
                );
            }
        }

        // Y lo que se ejecuta sólo difiere en los valores tapados.
        let de_verdad = c.argv("orbit").unwrap();
        assert_eq!(de_verdad.len(), visible.len(), "{nombre}");
        for (a, b) in visible.iter().zip(&de_verdad) {
            if !a.ends_with(OCULTO) {
                assert_eq!(a, b, "{nombre}");
            }
        }
    }
}
