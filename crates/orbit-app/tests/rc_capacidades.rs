//! Los permisos de Tauri de la ventana: que existan y que no crezcan.
//!
//! Sin `capabilities/`, Tauri 2 deja pasar los comandos de `lib.rs` pero
//! rechaza cualquier `plugin:*`, también `plugin:event|listen`: en la ventana
//! real el `listen('orbit://progreso')` lanzaba, y crear, instalar y la pasada
//! no llegaban a empezar. Las pruebas de la interfaz no lo veían porque no hay
//! Tauri debajo.
//!
//! Y al revés: cada permiso que se añada aquí es superficie que gana un
//! renderizador comprometido. La lista blanca de abajo es lo que usa `ui/src`
//! (`@tauri-apps/api/event`: `listen` y su `unlisten`); ampliarla tiene que
//! verse en un diff de esta prueba.

use std::path::PathBuf;

const PERMITIDOS: [&str; 2] = ["core:event:allow-listen", "core:event:allow-unlisten"];

fn caja() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(p: &std::path::Path) -> serde_json::Value {
    let t = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&t).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn permisos(c: &serde_json::Value) -> Vec<String> {
    c["permissions"]
        .as_array()
        .expect("«permissions» es una lista")
        .iter()
        .map(|p| {
            // Un permiso con `allow`/`deny` propio es un objeto: ámbitos a
            // medida que esta prueba no sabe revisar, así que no se aceptan.
            p.as_str()
                .unwrap_or_else(|| panic!("permiso con ámbito propio: {p}"))
                .to_string()
        })
        .collect()
}

#[test]
fn rc_la_ventana_tiene_sus_permisos_y_ninguno_mas() {
    let dir = caja().join("capabilities");
    let ficheros: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("falta {}: {e}", dir.display()))
        .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        ficheros,
        ["default.json"],
        "una capacidad más es superficie más: revísala y añádela aquí"
    );

    let c = json(&dir.join("default.json"));
    assert_eq!(c["identifier"], "default");
    assert_eq!(c["windows"], serde_json::json!(["main"]));
    // Ni páginas remotas ni webviews sueltos con estos permisos.
    assert!(c.get("remote").is_none(), "una capacidad para URL remotas");
    assert!(c.get("webviews").is_none());

    let p = permisos(&c);
    for x in &p {
        assert!(
            PERMITIDOS.contains(&x.as_str()),
            "permiso fuera de la lista: {x}"
        );
    }
    for x in PERMITIDOS {
        assert!(
            p.iter().any(|y| y == x),
            "falta {x}: sin él el progreso no llega"
        );
    }
}

#[test]
fn rc_la_configuracion_no_mete_otras_capacidades() {
    let conf = json(&caja().join("tauri.conf.json"));
    let ventanas = conf["app"]["windows"].as_array().expect("hay ventanas");
    assert_eq!(ventanas.len(), 1);
    // La capacidad es para `main`: una ventana con otra etiqueta se quedaría
    // sin progreso, y en silencio.
    assert_eq!(ventanas[0]["label"], "main");
    // Capacidades en línea en la configuración se saltarían la prueba de arriba.
    let seg = &conf["app"]["security"];
    if let Some(cs) = seg.get("capabilities") {
        for c in cs.as_array().expect("lista") {
            assert_eq!(c, "default", "capacidad en tauri.conf.json: {c}");
        }
    }
    // Y ningún plugin con permisos propios.
    if let Some(p) = conf.get("plugins") {
        assert!(p.as_object().is_some_and(|o| o.is_empty()), "plugins: {p}");
    }
}

/// Lo que `tauri-build` genera a partir de `capabilities/` al compilar. Es lo
/// que lleva el binario, así que se mira también: un `{}` aquí era el fallo.
#[test]
fn rc_lo_generado_al_compilar_lleva_la_capacidad() {
    let g = caja().join("gen").join("schemas").join("capabilities.json");
    let c = json(&g);
    let d = &c["default"];
    assert_eq!(d["windows"], serde_json::json!(["main"]), "{c}");
    let mut p = permisos(d);
    p.sort();
    let mut esperados: Vec<String> = PERMITIDOS.iter().map(|s| s.to_string()).collect();
    esperados.sort();
    assert_eq!(p, esperados);
    assert_eq!(c.as_object().unwrap().len(), 1, "{c}");
}
