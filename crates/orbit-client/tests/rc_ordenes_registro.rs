//! Regresión de la carrera del registro (ronda adversaria, hallazgo 5).
//!
//! Antes, `leer → anadir → escribir` iba sin cerrojo y con un temporal de
//! nombre fijo: con 24 guardados a la vez salían «no he podido guardar» falsos
//! y quedaba un solo servidor en el fichero. Ahora [`registro::guardar`] hace
//! las tres cosas bajo el mismo cerrojo, y cada guardado que dice que sí está
//! en el fichero.
//!
//! Una sola prueba en su propio binario: cambia `XDG_CONFIG_HOME`, que es
//! global al proceso y lo lee `registro::ruta()`.

use orbit_client::registro::{self, ServidorGuardado};

fn srv(alias: &str) -> ServidorGuardado {
    ServidorGuardado {
        alias: alias.into(),
        host: "10.0.0.5".into(),
        usuario: "root".into(),
        puerto: 22,
        clave: None,
        binario: None,
    }
}

#[test]
fn guardar_y_quitar_a_la_vez_no_pierden_nada() {
    let base = std::env::temp_dir().join(format!("rc-registro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &base);
    let ruta = registro::ruta().unwrap();

    // 1. Guardados concurrentes: todos los que dicen que sí, están.
    const N: usize = 24;
    let barrera = std::sync::Barrier::new(N);
    let errores = std::sync::Mutex::new(Vec::<String>::new());
    std::thread::scope(|s| {
        for i in 0..N {
            let (barrera, errores) = (&barrera, &errores);
            s.spawn(move || {
                barrera.wait();
                if let Err(e) = registro::guardar(srv(&format!("srv{i}")), &[]) {
                    errores.lock().unwrap().push(e.to_string());
                }
            });
        }
    });
    let errores = errores.into_inner().unwrap();
    assert!(errores.is_empty(), "{errores:?}");
    let l = registro::leer().unwrap();
    assert_eq!(l.len(), N, "se ha perdido algún guardado");

    // 2. Quitar y guardar a la vez, que es «olvidar» y «guardar» en el pool.
    std::thread::scope(|s| {
        for i in 0..N {
            s.spawn(move || {
                if i % 2 == 0 {
                    registro::quitar(&format!("srv{i}")).unwrap();
                } else {
                    registro::guardar(srv(&format!("nuevo{i}")), &[]).unwrap();
                }
            });
        }
    });
    let mut alias: Vec<String> = registro::leer()
        .unwrap()
        .into_iter()
        .map(|s| s.alias)
        .collect();
    alias.sort();
    let mut esperado: Vec<String> = (0..N)
        .filter(|i| i % 2 == 1)
        .flat_map(|i| [format!("srv{i}"), format!("nuevo{i}")])
        .collect();
    esperado.sort();
    assert_eq!(alias, esperado);

    // Ni temporales ni cerrojos olvidados en la carpeta.
    let restos: Vec<String> = std::fs::read_dir(ruta.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n != "servidores.json")
        .collect();
    assert!(restos.is_empty(), "{restos:?}");

    // 3. Una entrada apartada al leer se conserva al escribir otra cosa, y
    //    `quitar` es la forma de deshacerse de ella.
    std::fs::write(
        &ruta,
        r#"[{"alias":"bueno","host":"10.0.0.5","usuario":"root","puerto":22,"clave":null,"binario":null},
            {"alias":"unc","host":"10.0.0.6","usuario":"root","puerto":22,"clave":"\\\\atacante\\r\\k","binario":null}]"#,
    )
    .unwrap();
    let lectura = registro::leer_completo().unwrap();
    assert_eq!(lectura.servidores.len(), 1);
    assert_eq!(lectura.descartados[0].alias.as_deref(), Some("unc"));
    registro::guardar(srv("otro"), &[]).unwrap();
    let texto = std::fs::read_to_string(&ruta).unwrap();
    assert!(
        texto.contains("atacante"),
        "lo apartado no se borra en silencio"
    );
    assert_eq!(registro::leer().unwrap().len(), 2);
    registro::quitar("unc").unwrap();
    assert!(!std::fs::read_to_string(&ruta).unwrap().contains("atacante"));
    // Y por la API antigua tampoco se pierde lo apartado.
    std::fs::write(
        &ruta,
        r#"[{"alias":"unc","host":"10.0.0.6","usuario":"root","puerto":22,"clave":"//atacante/r/k","binario":null}]"#,
    )
    .unwrap();
    let mut l = registro::leer().unwrap();
    registro::anadir(&mut l, srv("b"), &[]).unwrap();
    registro::escribir(&l).unwrap();
    assert!(std::fs::read_to_string(&ruta).unwrap().contains("atacante"));

    // 4. Un fichero corrupto no se pisa, ni por `guardar` ni por `escribir`.
    let roto = "[{\"alias\":";
    std::fs::write(&ruta, roto).unwrap();
    assert!(matches!(
        registro::guardar(srv("z"), &[]),
        Err(registro::ErrorDeRegistro::Ilegible { .. })
    ));
    assert!(registro::escribir(&[srv("z")]).is_err());
    assert_eq!(std::fs::read_to_string(&ruta).unwrap(), roto);

    // 5. Y lo que no valdría al guardar tampoco se escribe por la puerta de
    //    atrás de `escribir`.
    std::fs::remove_file(&ruta).unwrap();
    let malo = ServidorGuardado {
        clave: Some(r"\\atacante\r\k".into()),
        ..srv("m")
    };
    assert!(matches!(
        registro::escribir(&[malo]),
        Err(registro::ErrorDeRegistro::Clave(_))
    ));
    assert!(!ruta.exists());

    let _ = std::fs::remove_dir_all(&base);
}
