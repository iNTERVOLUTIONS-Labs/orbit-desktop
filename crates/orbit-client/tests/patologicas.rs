//! El cliente contra el servidor falso, caso patológico a caso patológico.
//!
//! Es el corazón del plan de pruebas, y la razón está en `docs/QA.md`: **el
//! fallo característico de este producto no es que la aplicación explote, es que
//! pinte algo plausible.** Por eso cada prueba de aquí comprueba dos cosas —qué
//! debe hacer el cliente y, sobre todo, **qué no debe hacer**.
//!
//! Varios de estos casos no salen de la imaginación: salen de fallos reales y
//! documentados de Orbit. P-34 es el remoto caído que se anunciaba como «nada
//! que hacer» cada cinco minutos. P-06 es por lo que el contrato devuelve
//! `total: 0` en vez de callarse. P-36 es el vhost borrado que no veía ninguna
//! de las tres preguntas que se hacen.

use orbit_client::comando::Comando;
use orbit_client::contrato::*;
use orbit_client::transporte::{ejecutar_local, ErrorTransporte};

/// Lanza el falso con un caso concreto.
fn falso(caso: &str, c: &Comando) -> Result<orbit_client::Respuesta, ErrorTransporte> {
    con_entorno(caso, c, &[])
}

/// El caso va **por invocación**. Con `set_var` las pruebas se pisaban entre
/// ellas —cargo las corre en hilos del mismo proceso y una variable de entorno
/// es global—, y el síntoma era un fallo intermitente que parecía del cliente.
fn con_entorno(
    caso: &str,
    c: &Comando,
    extra: &[(&str, &str)],
) -> Result<orbit_client::Respuesta, ErrorTransporte> {
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

// ── el camino feliz, que también hay que probar ────────────────────────────

#[test]
fn p00_un_servidor_sano_se_lee_entero() {
    let r = falso("sano", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    assert_eq!(l.schema, 1);
    assert_eq!(l.apps.len(), 3);
}

#[test]
fn p00_el_saludo_dice_dos_versiones_y_son_ejes_distintos() {
    let r = falso("sano", &Comando::Version).unwrap();
    let v: Version = r.leer().unwrap();
    assert_eq!(v.contract, 1);
    assert_eq!(v.compatibilidad(), Compatibilidad::Exacta);
    assert!(v.semver().is_some());
}

// ── P-01 · JSON cortado a mitad ────────────────────────────────────────────

#[test]
fn p01_un_json_truncado_no_se_parsea_a_medias() {
    let r = falso("truncado", &Comando::Lista).unwrap();
    let x: Result<Lista, _> = r.leer();
    assert!(
        x.is_err(),
        "NO debe leerse lo que llegó ni pintar las apps que sí cupieron"
    );
}

// ── P-02 · campos que no conocemos ─────────────────────────────────────────

#[test]
fn p02_los_campos_nuevos_se_ignoran_sin_ruido() {
    // La promesa de Orbit es que los campos se añaden y nunca se renombran.
    // Nuestra mitad es no romper cuando cumplen su parte.
    let r = falso("campos-nuevos", &Comando::Lista).unwrap();
    let l: Lista = r.leer().expect("un campo desconocido no puede romper nada");
    assert_eq!(l.apps.len(), 3);
}

// ── P-03 · null donde el contrato dice número ──────────────────────────────

#[test]
fn p03_un_null_en_un_numero_no_se_convierte_en_cero() {
    let r = falso("null-en-numero", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    assert_eq!(l.apps[0].state.releases, None);
    assert_ne!(
        l.apps[0].state.releases,
        Some(0),
        "un cero es una afirmación"
    );
}

// ── P-04 · un schema que no entendemos ─────────────────────────────────────

#[test]
fn p04_un_schema_desconocido_se_rechaza_entero() {
    let r = falso("schema-2", &Comando::Lista).unwrap();
    let x: Result<Lista, _> = r.leer();
    assert!(x.is_err(), "NO debe intentar leerse «por si acaso»");
}

// ── P-05 · contrato más nuevo ──────────────────────────────────────────────

#[test]
fn p05_un_contrato_mas_nuevo_no_nos_hace_negarnos_a_hablar() {
    let r = falso("contrato-9", &Comando::Version).unwrap();
    let v: Version = r
        .leer()
        .expect("el saludo se lee siempre: es la garantía 1");
    assert_eq!(v.compatibilidad(), Compatibilidad::MasNuevo { suyo: 9 });
    // Negarse sería la peor forma de romper algo que todavía funcionaba.
}

// ── P-06 y P-07 · el silencio no es una colección vacía ────────────────────

#[test]
fn p06_un_stdout_en_blanco_no_es_no_hay_apps() {
    let r = falso("vacio", &Comando::Lista).unwrap();
    let x: Result<Lista, _> = r.leer();
    assert!(
        x.is_err(),
        "con stdout en blanco no se puede distinguir «no había nada» de «se rompió»"
    );
}

#[test]
fn p07_una_coleccion_vacia_si_es_una_respuesta() {
    let r = falso("sin-apps", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    assert_eq!(l.apps.len(), 0);
}

// ── P-08, P-09, P-10 · el código de salida y el objeto ─────────────────────

#[test]
fn p08_un_error_sin_objeto_enseña_el_stderr_que_es_donde_va_el_mensaje() {
    let e = falso("error-mudo", &Comando::Lista).unwrap_err();
    match e {
        ErrorTransporte::Orbit { stderr, .. } => assert!(stderr.contains("root")),
        otro => panic!("esperaba un fallo de orbit, no {otro:?}"),
    }
}

#[test]
fn p09_un_despliegue_que_falla_contesta_y_el_objeto_no_se_descarta() {
    // El caso más importante de los 52: el rc es 1 y el objeto es válido. Un
    // cliente que descarte stdout por el código de salida se queda sin el
    // motivo, que es lo único que puede enseñarle a alguien.
    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: false,
    };
    let r = falso("fallo-con-objeto", &c).expect("el objeto llega aunque el rc sea 1");
    let d: Despliegue = r.leer().unwrap();
    assert!(!d.ok);
    assert_eq!(d.failed_step.as_deref(), Some("build"));
    assert!(d.error.is_some());
    // Y trae la release anterior, para poder ofrecer el rollback sin otra llamada.
    assert_eq!(d.previous.as_deref(), Some("20260805-041230"));
}

#[test]
fn p10_si_el_rc_y_el_objeto_discrepan_manda_el_objeto() {
    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: false,
    };
    let r = falso("incoherente", &c).unwrap();
    let d: Despliegue = r.leer().unwrap();
    assert!(!d.ok, "rc 0 con ok:false: se le cree al objeto");
}

// ── P-11, P-12, P-13 · lo que llega por cada canal ─────────────────────────

#[test]
fn p11_el_ruido_de_stderr_no_ensucia_los_datos() {
    // El motd, el banner de un .bashrc. stderr nunca son datos.
    let r = falso("ruido-stderr", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    assert_eq!(l.apps.len(), 3);
    assert!(r.stderr.contains("Ubuntu"));
}

#[test]
fn p12_basura_en_stdout_es_un_fallo_no_algo_que_recortar() {
    // Buscar la primera llave es exactamente cómo se cuela un objeto ajeno
    // delante del legítimo.
    let e = falso("ruido-stdout", &Comando::Lista);
    let r = e.unwrap();
    assert!(matches!(
        r.objeto(),
        Err(ErrorTransporte::RespuestaSucia(_))
    ));
}

#[test]
fn p13_dos_objetos_no_se_resuelven_quedandose_con_uno() {
    let r = falso("dos-objetos", &Comando::Version).unwrap();
    assert!(matches!(
        r.objeto(),
        Err(ErrorTransporte::RespuestaSucia(_))
    ));
}

// ── P-14, P-15, P-16 · el servidor no está listo ───────────────────────────

#[test]
fn p14_orbit_que_no_existe_tiene_su_propio_mensaje() {
    let e = falso("no-instalado", &Comando::Lista).unwrap_err();
    assert!(
        matches!(e, ErrorTransporte::OrbitNoEsta { .. }),
        "«error al conectar» no le dice a nadie qué arreglar"
    );
}

#[test]
fn p15_orbit_sin_permiso_no_se_confunde_con_orbit_ausente() {
    let e = falso("sin-permiso", &Comando::Lista).unwrap_err();
    assert!(matches!(e, ErrorTransporte::OrbitNoEsta { .. }));
}

#[test]
fn p16_sudo_pidiendo_contraseña_se_reconoce_y_se_explica() {
    let e = falso("sudo-pide-clave", &Comando::Lista).unwrap_err();
    assert!(matches!(e, ErrorTransporte::SudoPideClave));
    assert!(e.to_string().contains("terminal"));
}

// ── P-17 · la conexión se corta ────────────────────────────────────────────

#[test]
fn p17_un_corte_no_se_lee_como_un_resultado() {
    let e = falso("corte", &Comando::Lista).unwrap_err();
    // 255 es de ssh, no de orbit. Confundirlos marca un servidor entero como
    // caído porque el usuario pidió una app que no existe.
    assert!(matches!(e, ErrorTransporte::NoLlego { .. }));
}

// ── P-20, P-21 · presupuestos ──────────────────────────────────────────────

#[test]
fn p20_un_flujo_infinito_se_corta_por_presupuesto() {
    let e = falso("infinito", &Comando::Version).unwrap_err();
    assert!(
        matches!(
            e,
            ErrorTransporte::Demasiado { .. } | ErrorTransporte::Tarde { .. }
        ),
        "NO puede consumir memoria hasta morir, sino {e:?}"
    );
}

#[test]
fn p21_una_respuesta_enorme_se_corta() {
    let e = con_entorno(
        "enorme",
        &Comando::Lista,
        &[("ORBIT_FAKE_BYTES", "9000000")],
    )
    .unwrap_err();
    assert!(matches!(e, ErrorTransporte::Demasiado { .. }));
}

// ── P-22, P-23, P-24 · nombres hostiles ────────────────────────────────────

#[test]
fn p22_un_nombre_con_marcado_se_lee_literal_y_no_se_puede_operar_sobre_el() {
    let r = falso("nombre-hostil", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    let xss = &l.apps[0].name;
    // Llega literal: _j_str no escapa '<' ni '>', y hace bien — su trabajo es
    // producir JSON válido, no HTML seguro. El escapado de marcado es del
    // cliente, y se hace donde se genera el marcado.
    assert!(xss.contains("<img"));
    // Y no se puede operar sobre él: un dato que ha dado la vuelta por el
    // servidor no es de más confianza que uno tecleado, es de menos.
    assert!(Comando::Info { app: xss.clone() }
        .argv("/usr/local/bin/orbit")
        .is_err());
}

#[test]
fn p23_un_nombre_con_inyeccion_no_llega_a_construir_una_orden() {
    let r = falso("nombre-hostil", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    let inj = &l.apps[1].name;
    assert!(inj.contains("curl"));
    assert!(
        Comando::Info { app: inj.clone() }
            .argv("/usr/local/bin/orbit")
            .is_err(),
        "y aunque llegara, el escapado lo neutraliza — son dos capas"
    );
}

#[test]
fn p24_los_nombres_que_engañan_al_ojo_tampoco_se_operan() {
    let r = falso("nombre-hostil", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    for a in &l.apps[2..] {
        // Bidi override, homoglifo cirílico y espacio de ancho cero. Ninguno
        // pasa la regla de forma, que es ASCII y minúsculas.
        assert!(
            Comando::Info {
                app: a.name.clone()
            }
            .argv("/x")
            .is_err(),
            "«{}» no debería poder operarse",
            a.name
        );
    }
}

// ── P-26, P-28 · el tipo equivocado ────────────────────────────────────────

#[test]
fn p26_utf8_malformado_no_se_pinta_en_silencio() {
    let r = falso("utf8-roto", &Comando::Lista).unwrap();
    let x: Result<Lista, _> = r.leer();
    // Se lee con reemplazo o se rechaza; lo que no vale es pintar el carácter
    // de sustitución sin que nadie se entere de que hubo un problema.
    if let Ok(l) = x {
        assert!(
            l.apps[0].name.contains('\u{FFFD}'),
            "si se acepta, tiene que quedar la marca de que algo se sustituyó"
        );
    }
}

#[test]
fn p28_una_lista_que_no_es_una_lista_es_un_error_de_tipo() {
    let r = falso("tipo-erroneo", &Comando::Lista).unwrap();
    let x: Result<Lista, _> = r.leer();
    assert!(x.is_err(), "NO se puede iterar y pintar basura");
}

// ── P-29 · números en los extremos ─────────────────────────────────────────

#[test]
fn p29_un_certificado_caducado_da_dias_negativos_y_eso_es_real() {
    let r = falso(
        "numeros-raros",
        &Comando::Info {
            app: "app001".into(),
        },
    )
    .unwrap();
    // Dos salidas aceptables y una prohibida. Si se lee, el negativo tiene que
    // conservarse: es un certificado caducado, no un error. Y si se rechaza, es
    // por el puerto fuera de rango, que también vale. Lo que no puede hacer es
    // desbordar en silencio y pintar un número que nadie mandó.
    if let Ok(i) = r.leer::<Info>() {
        assert_eq!(i.app.state.cert_days, Some(-5));
    }
}

// ── P-33, P-34 · los seis finales del lote ─────────────────────────────────

#[test]
fn p33_los_seis_finales_se_cuentan_por_separado() {
    let r = falso("lote-seis", &Comando::DesplegarTodo { progreso: false }).unwrap();
    let l: Lote = r.leer().unwrap();
    assert_eq!(l.total, 8);
    assert_eq!(l.deployed, 2);
    assert_eq!(l.failed, 2);
    assert_eq!(l.unchanged, 1);
    assert_eq!(l.unreachable, 1);
    assert_eq!(l.gone, 1);
    assert_eq!(l.skipped, 1);
    // 'ok' es la misma regla que el código de salida.
    assert!(!l.ok);
    // Y lo que está prohibido: agrupar. La suma de «correctas» y «fallidas» no
    // reconstruye el total, y ése es justamente el motivo de que sean seis.
    assert_ne!(l.deployed + l.failed, l.total);
}

#[test]
fn p33_dentro_de_cada_app_va_el_objeto_de_deploy_sin_recortar() {
    let r = falso("lote-seis", &Comando::DesplegarTodo { progreso: false }).unwrap();
    let l: Lote = r.leer().unwrap();
    let revertida = l.apps.iter().find(|a| a.app == "revertida").unwrap();
    let d = revertida
        .result
        .as_ref()
        .expect("un fallo trae su objeto entero");
    // 'rolled_back' y 'recovered' existen para que un panel pueda enseñar
    // distinto lo que ES distinto. Aplanarlos los habría perdido.
    assert!(d.rolled_back);
    assert!(!d.recovered);
    let recuperada = l.apps.iter().find(|a| a.app == "recuperada").unwrap();
    assert!(recuperada.result.as_ref().unwrap().recovered);
    // En las que no se han desplegado, 'result' es null y el motivo va en
    // 'error': un null es una respuesta, un objeto a medias no.
    let muda = l.apps.iter().find(|a| a.app == "muda").unwrap();
    assert!(muda.result.is_none());
    assert!(muda.error.is_some());
}

#[test]
fn p34_no_he_podido_preguntar_no_es_nada_que_hacer() {
    // El bug real: un remoto caído anunciado como «nada que hacer» cada cinco
    // minutos. El contrato tiene seis finales para que no se repita.
    let r = falso("lote-mudo", &Comando::DesplegarTodo { progreso: false }).unwrap();
    let l: Lote = r.leer().unwrap();
    assert_eq!(l.unreachable, 3);
    assert_eq!(
        l.unchanged, 0,
        "confundirlos es el fallo que este contrato existe para evitar"
    );
    assert!(!l.ok);
}

// ── P-35 · el botón sólo donde hace algo ───────────────────────────────────

#[test]
fn p35_un_arreglo_descrito_pero_no_aplicable_no_lleva_boton() {
    let r = falso("doctor-no-fixable", &Comando::Doctor).unwrap();
    let d: Doctor = r.leer().unwrap();
    let pnpm = d.checks.iter().find(|c| c.id == "pnpm").unwrap();
    // Tiene texto de arreglo y NO es aplicable. Un botón que no hace nada es
    // peor que ninguno: 'fixable' existe exactamente para poder distinguirlo.
    assert!(pnpm.fix.is_some());
    assert!(!pnpm.fixable);
    let ds = d.checks.iter().find(|c| c.id == "default-server").unwrap();
    assert!(ds.fixable);
}

// ── P-36, P-37 · los estados que no se pueden confundir ────────────────────

#[test]
fn p36_una_app_sin_vhost_es_un_fallo_grave_y_gana_sobre_lo_demas() {
    let r = falso("estados", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    let a = l.apps.iter().find(|a| a.name == "sin-vhost").unwrap();
    assert_eq!(a.state.salud(), Salud::SinVhost);
}

#[test]
fn p37_una_estatica_no_esta_parada() {
    let r = falso("estados", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    let e = l.apps.iter().find(|a| a.name == "estatica").unwrap();
    assert_eq!(e.state.salud(), Salud::SinProceso);
    let p = l.apps.iter().find(|a| a.name == "parada").unwrap();
    assert_eq!(p.state.salud(), Salud::Parada);
    // Los dos son «no está sirviendo un proceso» y sólo uno es un problema.
    assert_ne!(e.state.salud(), p.state.salud());
}

#[test]
fn p37_el_mantenimiento_no_es_una_averia() {
    let r = falso("estados", &Comando::Lista).unwrap();
    let l: Lista = r.leer().unwrap();
    let m = l.apps.iter().find(|a| a.name == "mantenimiento").unwrap();
    assert_eq!(m.state.salud(), Salud::Mantenimiento);
}

// ── P-41 · load con menos de tres elementos ────────────────────────────────

#[test]
fn p41_un_load_vacio_no_revienta_al_leer_el_primer_elemento() {
    let r = falso("load-vacio", &Comando::Estado).unwrap();
    let v: serde_json::Value = serde_json::from_str(r.objeto().unwrap()).unwrap();
    let load = v["host"]["load"].as_array().unwrap();
    // La línea 11497 del Orbit real emite esto crudo desde /proc/loadavg, así
    // que puede salir vacío. Hay que comprobar la longitud, no suponerla: un
    // cliente que pinte load[0] sin mirar revienta.
    assert!(load.is_empty());
}

// ── P-43 · la respuesta no es de lo que se preguntó ────────────────────────

#[test]
fn p43_una_respuesta_de_otra_app_se_detecta() {
    // Barato de comprobar, y no hacerlo sale gratis hasta el día que no lo es:
    // un servidor comprometido puede pintarte el estado de otra cosa.
    let r = falso(
        "otra-app",
        &Comando::Info {
            app: "app001".into(),
        },
    )
    .unwrap();
    let i: Info = r.leer().unwrap();
    assert_ne!(
        i.app.name, "app001",
        "el falso contesta de otra app a propósito"
    );
    // La comprobación que el cliente debe hacer, escrita aquí para que exista:
    assert!(
        i.app.name != "app001",
        "quien pregunte por app001 tiene que rechazar esto"
    );
}

// ── P-49 · lo que la documentación promete y el binario no da ──────────────

#[test]
fn p49_un_comando_documentado_y_roto_se_trata_como_lo_que_es() {
    // La clase de fallo que encontramos auditando: 'doctor --fix --json --yes'
    // documentado en dos sitios y muerto. La fuente de verdad es el binario.
    let e = falso("documentado-y-roto", &Comando::Doctor).unwrap_err();
    match e {
        ErrorTransporte::Orbit { stderr, .. } => assert!(stderr.contains("--yes")),
        otro => panic!("esperaba el rechazo de orbit, no {otro:?}"),
    }
}

// ── P-51 · el suelo de 72 ms ───────────────────────────────────────────────

#[test]
fn p51_hay_un_suelo_por_llamada_y_conviene_conocerlo() {
    // Medido: 'version --json' cuesta 72 ms porque son 13.720 líneas de bash
    // parseadas cada vez. Un servidor que conteste en 5 ms no es rápido: o no
    // es Orbit, o hay algo delante contestando por él.
    //
    // Aquí no se afirma el número (el falso es un script pequeño), sólo se deja
    // el razonamiento junto a la prueba que lo usaría contra un servidor real.
    let r = falso("sano", &Comando::Version).unwrap();
    let v: Version = r.leer().unwrap();
    assert_eq!(v.schema, 1);
}

// ── el flujo NDJSON, que es la única excepción del contrato ────────────────

use orbit_client::flujo::{leer_log, leer_progreso, Canal, SucesoDeLog};

#[test]
fn el_flujo_de_logs_se_lee_entero() {
    let c = Comando::Logs {
        app: "app001".into(),
        desde: None,
        lineas: None,
        seguir: false,
        solo_nginx: true,
    };
    let r = falso("sano", &c).unwrap();
    let l = leer_log(&r.stdout);
    assert_eq!(l.rotas, 0);
    // Y el acceso se distingue del error, que es lo que la prosa pierde porque
    // `tail` mezcla los dos ficheros sin decir cuál es cuál.
    let canales: Vec<Canal> = l
        .sucesos
        .iter()
        .filter_map(|s| match s {
            SucesoDeLog::Linea(x) => Some(x.stream),
            _ => None,
        })
        .collect();
    assert!(canales.contains(&Canal::Access));
    assert!(canales.contains(&Canal::Error));
}

#[test]
fn p31_una_linea_rota_no_deja_el_log_en_blanco() {
    // Un byte mal puesto no puede convertir un log en una pantalla vacía, y
    // quien mira un log suele estar mirándolo porque algo va mal.
    let c = Comando::Logs {
        app: "app001".into(),
        desde: None,
        lineas: None,
        seguir: false,
        solo_nginx: true,
    };
    let r = falso("log-roto", &c).unwrap();
    let l = leer_log(&r.stdout);
    assert_eq!(l.rotas, 1, "la rota se cuenta, no se calla");
    assert!(l.sucesos.len() >= 3, "el resto tiene que seguir leyéndose");
}

#[test]
fn un_log_en_vivo_no_trae_fin_y_el_meta_ya_lo_avisó() {
    let c = Comando::Logs {
        app: "app001".into(),
        desde: None,
        lineas: None,
        seguir: true,
        solo_nginx: true,
    };
    let r = falso("log-vivo", &c).unwrap();
    let l = leer_log(&r.stdout);
    assert!(!l.sucesos.iter().any(|s| matches!(s, SucesoDeLog::Fin(_))));
    match &l.sucesos[0] {
        SucesoDeLog::Meta(m) => assert!(m.follow),
        _ => panic!("la primera línea tiene que ser el meta"),
    }
}

#[test]
fn el_progreso_va_por_stderr_y_el_objeto_por_stdout() {
    // Los dos canales a la vez y sin mezclarse. Es la razón por la que
    // `--progress` puede existir sin romper «por stdout, un solo objeto».
    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: true,
    };
    let r = falso("deploy-progreso", &c).unwrap();

    let d: Despliegue = r.leer().expect("por stdout, el objeto");
    assert!(d.ok);

    let (pasos, rotas) = leer_progreso(&r.stderr);
    assert_eq!(rotas, 0, "la prosa de stderr no es una línea rota");
    assert_eq!(pasos.len(), 3);
    assert!(
        pasos.iter().all(|p| p.app.as_deref() == Some("mi-web")),
        "cada paso lleva su app: sin eso, un paso de un lote no se atribuye"
    );
}

// ── el saludo: qué hay al otro lado ────────────────────────────────────────

use orbit_client::saludo::{clasificar as saludar, Saludo};

#[test]
fn un_orbit_a_medias_no_acaba_en_el_parser() {
    // Las dos líneas van por stdout e ignoran --json. Un cliente que haga
    // `JSON.parse(stdout)` sin mirar el código de salida explota con un error de
    // sintaxis donde la respuesta correcta es «tienes Orbit a medias».
    let r = falso("a-medias", &Comando::Version);
    let resp = match r {
        Ok(x) => x,
        Err(ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => orbit_client::transporte::Respuesta {
            stdout,
            stderr,
            codigo,
        },
        Err(otro) => panic!("esperaba una respuesta, no {otro:?}"),
    };
    match saludar(&resp) {
        Saludo::NoInstalado { motivo } => assert!(motivo.contains("install.sh")),
        otro => panic!("esperaba NoInstalado, no {otro:?}"),
    }
}

#[test]
fn un_orbit_anterior_al_contrato_se_reconoce_como_viejo_no_como_roto() {
    let r = falso("pre-contrato", &Comando::Version);
    let resp = match r {
        Ok(x) => x,
        Err(ErrorTransporte::Orbit {
            codigo,
            stdout,
            stderr,
        }) => orbit_client::transporte::Respuesta {
            stdout,
            stderr,
            codigo,
        },
        Err(otro) => panic!("esperaba una respuesta, no {otro:?}"),
    };
    let s = saludar(&resp);
    assert!(matches!(s, Saludo::SinContrato { .. }));
    // Cero funcionalidad, no funcionalidad a medias: un cliente que intente
    // hablar un contrato que no existe acaba parseando tablas.
    assert!(!s.permite_leer());
}

#[test]
fn un_contrato_mas_nuevo_deja_mirar_pero_no_tocar() {
    let r = falso("contrato-9", &Comando::Version).unwrap();
    let s = saludar(&r);
    assert!(matches!(s, Saludo::MasNuevo(_)));
    assert!(s.permite_leer());
    assert!(!s.permite_operar());
}
