//! Lo que cambió en el contrato entre Orbit 1.3.6 y 1.4.8.
//!
//! Las respuestas salen del Orbit de verdad, ejecutado en el banco de
//! `tools/bench` (ver `tests/fakeserver/fixtures/*-1.4*.json`), y se prueban las
//! dos mitades de la misma promesa:
//!
//! 1. **Lo nuevo se lee.** La congelación con su quién y su porqué, la puerta de
//!    la CI, el tipo de certificado, los cinco finales nuevos de la pasada, lo
//!    que falta del `.env` y las tareas programadas.
//! 2. **Lo viejo sigue leyéndose, y su silencio no se convierte en un valor.**
//!    Un 1.3.6 no dice `locked`, y eso no es «no está congelada». Un parque de
//!    servidores tarda meses en actualizarse: hablar con uno viejo es el caso
//!    normal.

use orbit_client::comando::{Comando, Secreto, VariableNueva};
use orbit_client::contrato::*;
use orbit_client::transporte::{ejecutar_local, ErrorTransporte, Respuesta};

fn falso(caso: &str, c: &Comando) -> Result<Respuesta, ErrorTransporte> {
    falso_con(caso, c, &[])
}

fn falso_con(
    caso: &str,
    c: &Comando,
    extra: &[(&str, &str)],
) -> Result<Respuesta, ErrorTransporte> {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let bin = raiz.join("tests/fakeserver/orbit");
    let mut env: Vec<(&str, &str)> = vec![("ORBIT_FAKE_CASE", caso)];
    env.extend_from_slice(extra);
    ejecutar_local(bin.to_str().unwrap(), c, &env)
}

fn desplegar(entorno: Vec<VariableNueva>, forzar: bool) -> Comando {
    Comando::Desplegar {
        app: "tienda".into(),
        progreso: false,
        entorno,
        forzar,
    }
}

// ── la lista ───────────────────────────────────────────────────────────────

#[test]
fn la_congelacion_llega_con_quien_cuando_y_por_que() {
    let l: Lista = falso("orbit-1.4", &Comando::Lista).unwrap().leer().unwrap();
    let tienda = l.apps.iter().find(|a| a.name == "tienda").unwrap();
    assert_eq!(tienda.state.congelada(), Some(true));
    let c = tienda
        .state
        .lock
        .clone()
        .flatten()
        .expect("congelada trae su lock");
    assert_eq!(c.by.as_deref(), Some("dabellan"));
    assert!(c.reason.unwrap().contains("black friday"));

    // Un `touch` a mano del testigo: congelada, y sin quién ni porqué. Que
    // falten no la descongela.
    let portal = l.apps.iter().find(|a| a.name == "portal").unwrap();
    assert_eq!(portal.state.congelada(), Some(true));
    let p = portal.state.lock.clone().flatten().unwrap();
    assert_eq!(p.by, None);
    assert_eq!(p.reason, None);

    // Y una sin congelar lo dice: `Some(false)` y `lock: null`.
    let blog = l.apps.iter().find(|a| a.name == "blog").unwrap();
    assert_eq!(blog.state.congelada(), Some(false));
    assert_eq!(blog.state.lock, Some(None));
}

#[test]
fn congelar_no_cambia_lo_que_se_sirve() {
    // Una app congelada atiende igual: la congelación es de los despliegues.
    let l: Lista = falso("orbit-1.4", &Comando::Lista).unwrap().leer().unwrap();
    let tienda = l.apps.iter().find(|a| a.name == "tienda").unwrap();
    assert_eq!(tienda.state.salud(), Salud::Activa);
}

#[test]
fn el_tipo_de_certificado_distingue_el_de_origen() {
    let l: Lista = falso("orbit-1.4", &Comando::Lista).unwrap().leer().unwrap();
    let tipo = |n: &str| {
        l.apps
            .iter()
            .find(|a| a.name == n)
            .unwrap()
            .state
            .cert_type
            .clone()
    };
    assert_eq!(tipo("api"), Some(Some("origin".into())));
    assert_eq!(tipo("tienda"), Some(Some("letsencrypt".into())));
    // Sin certificado: el servidor lo dice, y dice que no hay.
    assert_eq!(tipo("portal"), Some(None));
}

#[test]
fn un_servidor_viejo_sigue_leyendose_y_no_dice_lo_que_no_sabe() {
    // La lista de un 1.3.6: sin `locked`, sin `cert_type`, sin `checks`.
    let l: Lista = falso("sano", &Comando::Lista).unwrap().leer().unwrap();
    for a in &l.apps {
        assert_eq!(a.state.congelada(), None, "{}: no sabe congelar", a.name);
        assert_eq!(a.state.cert_type, None);
        assert_eq!(a.state.checks, None);
    }
}

#[test]
fn el_detalle_de_un_1_4_trae_la_configuracion_nueva_como_cadenas() {
    let i: Info = falso(
        "orbit-1.4",
        &Comando::Info {
            app: "tienda".into(),
        },
    )
    .unwrap()
    .leer()
    .unwrap();
    // Todo lo de `config` son cadenas, también lo que llegó después.
    assert_eq!(i.app.config["assets"], "/_next/static/:.next/static");
    assert_eq!(i.app.config["health_path"], "/up");
    assert_eq!(i.app.config["limit_mem"], "512M");
    assert_eq!(i.app.state.cert_days, Some(61));
    assert_eq!(i.app.state.checks, Some(true));
}

// ── la pasada ──────────────────────────────────────────────────────────────

#[test]
fn los_once_finales_se_cuentan_por_separado() {
    // La pasada sale con 1 —hay fallos— y el objeto llega igual.
    let r = falso(
        "orbit-1.4",
        &Comando::DesplegarTodo {
            progreso: false,
            solo_si_cambia: true,
        },
    )
    .unwrap();
    let l: Lote = r.leer().unwrap();
    assert_eq!(l.total, 10);
    assert_eq!(
        (l.locked, l.ignored, l.ci_pending, l.ci_failed, l.ci_unknown),
        (Some(1), Some(1), Some(1), Some(1), Some(1))
    );
    // Los recuentos reconstruyen el total, y ninguna agrupación lo hace.
    let suma = l.deployed
        + l.failed
        + l.unchanged
        + l.unreachable
        + l.gone
        + l.skipped
        + l.locked.unwrap()
        + l.ignored.unwrap()
        + l.ci_pending.unwrap()
        + l.ci_failed.unwrap()
        + l.ci_unknown.unwrap();
    assert_eq!(suma, l.total);
    assert!(!l.ok);

    // Cada fila dice cuál, y las de las puertas no traen objeto de deploy: no
    // se ha desplegado nada, y el motivo va en `error`.
    let congelada = l.apps.iter().find(|a| a.app == "congelada").unwrap();
    assert_eq!(congelada.status, FinalDespliegue::Locked);
    assert!(congelada.result.is_none());
    assert!(congelada.error.as_deref().unwrap().contains("dabellan"));
    let muda = l.apps.iter().find(|a| a.app == "ci-muda").unwrap();
    assert_eq!(muda.status, FinalDespliegue::CiUnknown);
}

#[test]
fn no_poder_preguntar_a_la_ci_pone_la_pasada_en_rojo() {
    // Nada ha fallado y nada está sin contacto: lo único que la pone en rojo es
    // no haber podido preguntar a GitHub. Es el `unreachable` de la CI.
    let l: Lote = falso(
        "lote-ci-muda",
        &Comando::DesplegarTodo {
            progreso: false,
            solo_si_cambia: true,
        },
    )
    .unwrap()
    .leer()
    .unwrap();
    assert_eq!((l.failed, l.unreachable, l.gone), (0, 0, 0));
    assert_eq!(l.ci_unknown, Some(1));
    assert!(!l.ok);
}

#[test]
fn la_pasada_de_un_servidor_viejo_no_inventa_los_recuentos_nuevos() {
    let l: Lote = falso(
        "lote-seis",
        &Comando::DesplegarTodo {
            progreso: false,
            solo_si_cambia: true,
        },
    )
    .unwrap()
    .leer()
    .unwrap();
    // Un 1.3.6 no puede producir `locked`: su recuento no es cero, no existe.
    assert_eq!(l.locked, None);
    assert_eq!(l.ci_unknown, None);
}

// ── el despliegue ──────────────────────────────────────────────────────────

#[test]
fn un_despliegue_congelado_se_niega_con_su_motivo_y_sin_tocar_nada() {
    let d: Despliegue = falso("congelada", &desplegar(vec![], false))
        .expect("el objeto llega aunque el rc sea 1")
        .leer()
        .unwrap();
    assert!(!d.ok);
    // Ni release ni paso fallido: no se ha empezado nada. Es lo que lo separa
    // de un despliegue que falla, y la interfaz lo tiene que pintar distinto.
    assert_eq!(d.release, None);
    assert_eq!(d.failed_step, None);
    assert!(d.error.unwrap().contains("congelada"));
    assert_eq!(d.env_missing, Some(vec![]));
}

#[test]
fn lo_que_falta_del_env_llega_en_el_objeto_y_no_hay_que_leer_la_prosa() {
    let d: Despliegue = falso("faltan-variables", &desplegar(vec![], false))
        .unwrap()
        .leer()
        .unwrap();
    assert!(!d.ok);
    // Se para antes del build: el paso es el de la release.
    assert_eq!(d.failed_step.as_deref(), Some("release"));
    assert_eq!(
        d.env_missing,
        Some(vec!["DATABASE_URL".to_string(), "MAIL_FROM".to_string()])
    );
}

#[test]
fn un_despliegue_de_un_servidor_viejo_no_dice_que_no_falte_nada() {
    let d: Despliegue = falso("deploy-progreso", &desplegar(vec![], false))
        .unwrap()
        .leer()
        .unwrap();
    assert_eq!(
        d.env_missing, None,
        "un 1.3.6 no lo cuenta, que no es «nada»"
    );
}

#[test]
fn las_variables_llegan_al_servidor_como_un_argumento_cada_una() {
    // Lo que se comprueba es lo que le llegó al servidor, no lo que contestó.
    let dir = std::env::temp_dir().join(format!("orbit14-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("argv.log");
    let _ = std::fs::remove_file(&log);
    let valor = r#"p@ss w"ord;$(id)`x`=y"#;
    let c = desplegar(
        vec![VariableNueva {
            clave: "DB_PASSWORD".into(),
            valor: Secreto::nuevo(valor),
        }],
        true,
    );
    falso_con(
        "orbit-1.4",
        &c,
        &[("ORBIT_FAKE_LOG", log.to_str().unwrap())],
    )
    .unwrap();
    let bruto = std::fs::read(&log).unwrap();
    let argv: Vec<String> = String::from_utf8_lossy(&bruto)
        .split('\0')
        .map(String::from)
        .collect();
    let i = argv.iter().position(|a| a == "--env").unwrap();
    assert_eq!(argv[i + 1], format!("DB_PASSWORD={valor}"));
    assert!(argv.contains(&"--force".to_string()));
    let _ = std::fs::remove_dir_all(&dir);
}

// ── el entorno y las tareas ────────────────────────────────────────────────

#[test]
fn la_comprobacion_del_entorno_dice_que_falta_sin_ensenar_un_valor() {
    let r = falso(
        "orbit-1.4",
        &Comando::EntornoComprobar {
            app: "tienda".into(),
        },
    )
    .unwrap();
    // Ningún campo de la respuesta puede llevar un valor.
    let v: serde_json::Value = serde_json::from_str(r.objeto().unwrap()).unwrap();
    for x in v["vars"].as_array().unwrap() {
        let claves: Vec<&String> = x.as_object().unwrap().keys().collect();
        assert_eq!(claves, ["key", "required", "set", "source"]);
    }
    let e: ComprobacionDeEntorno = r.leer().unwrap();
    assert!(!e.ok);
    assert_eq!(e.obligatorias_sin_valor(), ["DATABASE_URL"]);
    // Las del `.env.example` nunca son obligatorias: sólo se ofrecen.
    let ejemplo = e.vars.iter().find(|v| v.source == "example").unwrap();
    assert!(!ejemplo.required);
}

#[test]
fn las_tareas_traen_su_ultima_y_su_proxima_pasada_y_un_null_es_nunca() {
    let t: Tareas = falso(
        "orbit-1.4",
        &Comando::Tareas {
            app: "tienda".into(),
        },
    )
    .unwrap()
    .leer()
    .unwrap();
    assert_eq!(t.tasks.len(), 3);
    let informe = t.tasks.iter().find(|x| x.name == "informe").unwrap();
    assert_eq!(informe.last_run, None, "no ha corrido nunca");
    assert_eq!(informe.result, None);
    assert!(informe.next_run.is_some());
}

// ── las órdenes sin JSON ───────────────────────────────────────────────────

#[test]
fn congelar_contesta_en_prosa_y_eso_no_es_un_error() {
    let r = falso(
        "orbit-1.4",
        &Comando::Congelar {
            app: "tienda".into(),
            motivo: Some("black friday".into()),
        },
    )
    .unwrap();
    assert_eq!(r.codigo, 0);
    // Sin `--json` la prosa sale por stdout (`_ui_route`).
    assert!(r.stdout.contains("congelada"));
}

#[test]
fn un_rechazo_sin_objeto_ensena_la_frase_de_orbit_y_no_un_codigo() {
    let e = falso(
        "lock-redireccion",
        &Comando::Congelar {
            app: "viejo.com".into(),
            motivo: None,
        },
    )
    .unwrap_err();
    assert!(matches!(e, ErrorTransporte::Orbit { codigo: 1, .. }));
    assert_eq!(
        e.to_string(),
        "'viejo.com' es una redirección: no tiene código que desplegar."
    );
}
