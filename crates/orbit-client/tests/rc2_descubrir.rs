//! Segunda ronda adversaria antes de la 0.1.2 — la lista de `Host`, que también
//! es la lista blanca.
//!
//! `servidor_del_config` (orbit-app) sólo deja conectar a lo que devuelve
//! `descubrir::alias_de`. Así que lo que `alias_de` lea de más es un destino que
//! nadie escribió, y lo que lea de menos es un servidor que deja de abrirse
//! («alias-desconocido»).
//!
//! OpenSSH (≥ 8.7, `argv_split`) parte cada línea respetando comillas y corta en
//! un `#` al principio de una palabra. Aquí se partía con `split_whitespace`.
//! Empezaron siendo sondas que fallaban con el fallo; se quedan de regresión.

use std::path::PathBuf;

use orbit_client::descubrir;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("orbit-rc2-descubrir-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn rc2_un_comentario_al_final_del_host_no_son_mas_alias() {
    let d = dir();
    let cfg = d.join("config-comentario");
    std::fs::write(&cfg, "Host web # el de produccion\n  HostName 10.0.0.1\n").unwrap();
    let a = descubrir::alias_de(&cfg).unwrap();
    assert_eq!(a, ["web"], "el comentario se ha leído como alias: {a:?}");
}

#[test]
fn rc2_un_include_entre_comillas_con_espacios_se_sigue() {
    let d = dir();
    let sub = d.join("con espacio");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("extra.conf"), "Host db\n  HostName 10.0.0.2\n").unwrap();
    let cfg = d.join("config-include");
    let ruta = sub.join("extra.conf").to_string_lossy().replace('\\', "/");
    std::fs::write(&cfg, format!("Include \"{ruta}\"\nHost web\n")).unwrap();
    let a = descubrir::alias_de(&cfg).unwrap();
    assert!(
        a.iter().any(|x| x == "db"),
        "el Host del fichero incluido no aparece: {a:?}"
    );
}

#[test]
fn rc2_un_host_entre_comillas_es_el_nombre_sin_comillas() {
    let d = dir();
    let cfg = d.join("config-comillas");
    std::fs::write(&cfg, "Host \"web\"\n  HostName 10.0.0.1\n").unwrap();
    let a = descubrir::alias_de(&cfg).unwrap();
    assert_eq!(a, ["web"], "{a:?}");
}

// Un `Include` con comodín que se incluye a sí mismo. OpenSSH corta con
// «Too many recursive configuration includes» (fatal en cuanto pasa de 16);
// aquí se recorría el árbol entero hasta la profundidad 16: con 3 ficheros son
// 3^16 ≈ 43 millones de lecturas. Y `servidor_del_config` lo lee **en cada
// orden** de la ventana: todas se colgaban. Ahora cada fichero se lee una vez.
#[test]
fn rc2_un_include_recursivo_con_comodin_no_cuelga() {
    let d = dir().join("ciclo");
    let sub = d.join("d");
    std::fs::create_dir_all(&sub).unwrap();
    let patron = format!("{}/*.conf", sub.to_string_lossy().replace('\\', "/"));
    for i in 1..=3 {
        std::fs::write(
            sub.join(format!("{i}.conf")),
            format!("Include {patron}\nHost h{i}\n"),
        )
        .unwrap();
    }
    let cfg = d.join("config");
    std::fs::write(&cfg, format!("Include {patron}\nHost web\n")).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(descubrir::alias_de(&cfg));
    });
    let r = rx.recv_timeout(std::time::Duration::from_secs(5));
    let a = r.expect("alias_de sigue leyendo tras 5 s").unwrap();
    assert_eq!(a, ["h3", "h2", "h1", "web"]);
}
