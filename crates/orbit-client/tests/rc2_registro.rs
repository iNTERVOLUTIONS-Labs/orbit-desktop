//! Segunda ronda adversaria antes de la 0.1.2 — un cerrojo con fecha del futuro
//! bloqueaba los guardados.
//!
//! `CerrojoDeFichero::tomar` decidía «abandonado» con `modified().elapsed()`, y
//! `elapsed()` es `Err` cuando la fecha está en el futuro (reloj atrasado tras
//! un cambio de hora, NTP que corrige hacia atrás, un perfil en red con otro
//! reloj). `.ok()` lo convertía en «no es viejo», así que un cerrojo que dejó
//! un proceso abortado (`panic = "abort"` en release) no caducaba nunca hasta
//! que el reloj alcanzara esa fecha, y cada guardado fallaba a los 5 s con «otra
//! ventana de Orbit Desktop está guardando la lista ahora mismo».
//!
//! Empezó como sonda que fallaba con el fallo; se queda de regresión. Binario
//! propio: cambia `XDG_CONFIG_HOME`, que es del proceso entero.

use std::time::{Duration, Instant, SystemTime};

use orbit_client::registro::{self, ServidorGuardado};

#[test]
fn rc2_un_cerrojo_con_fecha_futura_no_bloquea_para_siempre() {
    let base = std::env::temp_dir().join(format!("orbit-rc2-reg-futuro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("orbit-desktop")).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &base);
    let ruta = registro::ruta().unwrap();
    let cerrojo = ruta.with_extension("json.cerrojo");

    // Lo que deja un proceso que murió con el cerrojo puesto, con la fecha una
    // hora por delante del reloj de ahora.
    let f = std::fs::File::create(&cerrojo).unwrap();
    f.set_modified(SystemTime::now() + Duration::from_secs(3600))
        .unwrap();
    drop(f);

    let t0 = Instant::now();
    let r = registro::guardar(
        ServidorGuardado {
            alias: "srv1".into(),
            host: "10.0.0.5".into(),
            usuario: "root".into(),
            puerto: 22,
            clave: None,
            binario: None,
        },
        &[],
    );
    let tardo = t0.elapsed();
    let cerrojo_suelto = !cerrojo.exists();
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        r.is_ok(),
        "guardar falla tras {tardo:?} por un cerrojo con fecha futura: {r:?}"
    );
    assert!(tardo < Duration::from_secs(2), "{tardo:?}");
    assert!(cerrojo_suelto, "el cerrojo nuevo se suelta al terminar");
}
