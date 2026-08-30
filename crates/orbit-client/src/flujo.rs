//! NDJSON: el contrato cuando la respuesta no cabe en un objeto.
//!
//! Hay **una** excepción a la regla de «por stdout, un solo objeto», y está
//! declarada: `orbit logs --json`. Es el único comando cuya salida es
//! inherentemente un flujo sin final —con `--follow` no termina nunca— así que
//! no hay un momento en el que cerrar un objeto, y una ventana de siete días
//! serían cientos de megas acumulados antes del primer byte.
//!
//! Y hay un **segundo** sitio con la misma forma y otro canal: `--progress`
//! emite un suceso por línea **por stderr** mientras un despliegue ocurre,
//! dejando stdout para el objeto final. Los dos comparten este módulo porque
//! comparten el problema —leer líneas sueltas de JSON según llegan— y no
//! comparten nada más.
//!
//! La regla que sostiene todo esto: **una línea rota no aborta el flujo.** Una
//! línea de progreso mal formada no puede tumbar un despliegue de tres minutos
//! que está yendo bien; se salta, se cuenta, y el objeto final sigue mandando.
//! Al revés sí: si lo que se rompe es el objeto final, eso es un fallo.

use serde::Deserialize;

/// La primera línea de `orbit logs --json`. Lleva el `schema` y dice qué viene
/// detrás, para que un cliente no tenga que adivinar si le va a llegar un
/// objeto o un flujo.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MetaDeLog {
    pub schema: u32,
    pub app: String,
    /// `journal` para las apps con proceso, `nginx` para las demás.
    pub source: String,
    pub unit: Option<String>,
    pub since: Option<String>,
    /// Con `--json` es `false` salvo que se pida `--follow`, igual que en
    /// `orbit top`: en modo máquina, una foto.
    pub follow: bool,
    pub lines: Option<u32>,
}

/// De qué log viene una línea.
///
/// Es lo que la salida en prosa **pierde**: `tail` mezcla el log de acceso y el
/// de error sin decir cuál es cuál, y distinguirlos es la primera pregunta de
/// cualquiera que mira un log de nginx.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Canal {
    Journal,
    Access,
    Error,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LineaDeLog {
    /// Sale del propio log y no se inventa. El de acceso lleva huso y sale con
    /// él; el de error no lo lleva y sale sin él, que quiere decir «hora local
    /// del servidor». Un log del formato viejo, sin marca, da `null` — y ése es
    /// el momento de ofrecer `orbit nginx-rebuild`.
    pub ts: Option<String>,
    pub stream: Canal,
    /// La línea tal cual. **No se estructura**: sacarle el nivel, el módulo o
    /// el código HTTP sería inventar un formato que la aplicación del usuario
    /// no ha prometido, y es el primer paso hacia un parser de logs.
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FinDeLog {
    pub lines: u32,
    /// Que se llegó al tope de `--lines`, **por fuente**: igual que
    /// `tail -n N f1 f2` da N de cada fichero. Es la misma honestidad que
    /// `requests_capped` en `top`: un número corto sin avisar se lee como «hay
    /// poco log», que suele ser lo contrario de lo que pasa.
    pub truncated: bool,
}

/// Un suceso del flujo de `logs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SucesoDeLog {
    Meta(MetaDeLog),
    Linea(LineaDeLog),
    /// **No llega si se está siguiendo en vivo**, y es correcto: un flujo que
    /// no termina no tiene final que anunciar. El cliente ya lo sabe, porque el
    /// `meta` se lo dijo con `follow: true`.
    Fin(FinDeLog),
}

/// Lo que se ha podido leer, y lo que no.
#[derive(Debug, Clone, Default)]
pub struct Lectura {
    pub sucesos: Vec<SucesoDeLog>,
    /// Cuántas líneas no se entendieron. **Se cuenta y no se calla**: si el
    /// flujo trae basura hay que poder decirlo, y si se calla, un log a medias
    /// se lee como un log completo.
    pub rotas: usize,
}

/// Lee un flujo NDJSON de `logs`.
///
/// Una línea rota se salta y se cuenta. La alternativa —abortar— convertiría un
/// byte mal puesto en una pantalla vacía, y quien mira un log suele estar
/// mirándolo justamente porque algo va mal.
pub fn leer_log(texto: &str) -> Lectura {
    let mut l = Lectura::default();
    for linea in texto.lines() {
        let t = linea.trim();
        if t.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(t) {
            Ok(v) => v,
            Err(_) => {
                l.rotas += 1;
                continue;
            }
        };
        // El suceso se decide por su campo `event` y no por su forma: adivinar
        // por los campos que trae haría que un campo nuevo cambiara el tipo de
        // un suceso, y los campos se añaden.
        let suceso = match v.get("event").and_then(|e| e.as_str()) {
            Some("meta") => serde_json::from_value(v).map(SucesoDeLog::Meta),
            Some("line") => serde_json::from_value(v).map(SucesoDeLog::Linea),
            Some("end") => serde_json::from_value(v).map(SucesoDeLog::Fin),
            _ => {
                // Un `event` que no conocemos se ignora en silencio: los
                // sucesos se añaden, igual que los campos.
                continue;
            }
        };
        match suceso {
            Ok(s) => l.sucesos.push(s),
            Err(_) => l.rotas += 1,
        }
    }
    l
}

// ── el otro flujo: el progreso de un despliegue ────────────────────────────

/// Un suceso de `orbit deploy --json --progress`, que llega **por stderr**
/// mientras stdout espera al objeto final.
///
/// `app` viaja en cada suceso porque `deploy --all --progress` mezcla los pasos
/// de varias apps por el mismo canal, y sin ese campo un paso de un lote no se
/// puede atribuir a nadie.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PasoDeDespliegue {
    /// `step` para un paso de un despliegue, `app` para el final de una app
    /// dentro de un lote. Los dos niveles se mezclan por el mismo canal, y por
    /// eso hay que mirar esto antes que nada.
    pub event: String,
    /// **Siempre se usa para atribuir**, también en un despliegue de una sola
    /// app. Es el campo que se añadió justo para que un lote pueda mezclar
    /// niveles por el mismo canal, y usarlo desde el principio evita tener dos
    /// analizadores.
    pub app: Option<String>,
    pub step: Option<String>,
    pub status: Option<String>,
    /// Segundos desde que empezó. Es lo que permite que la barra tenga su
    /// propio reloj: los sucesos llegan cuando llegan, y una barra que sólo se
    /// moviera con ellos estaría quieta noventa segundos durante el build y
    /// luego daría un salto.
    pub elapsed_s: Option<u64>,
}

/// Los seis pasos de un despliegue, en orden, con lo que pesa cada uno.
///
/// **Seis pasos no valen un sexto cada uno**: el build es típicamente el 70-85 %
/// del tiempo, así que una barra lineal se quedaría clavada en el 33 % durante
/// dos minutos y luego correría hasta el final. Estos son los pesos por defecto,
/// para cuando no hay histórico.
pub const PASOS: [(&str, &str, f32); 6] = [
    ("code", "actualizar el clon de git", 0.05),
    ("release", "copiar a la release nueva", 0.05),
    ("build", "compilar", 0.70),
    ("activate", "mover el symlink current", 0.03),
    ("service", "reiniciar y esperar al health check", 0.12),
    ("nginx", "recargar nginx", 0.05),
];

/// Los pesos afinados con el histórico de la app, si lo hay.
///
/// `build_median_s` y la mediana total salen de `orbit metrics <app> --json`. Si
/// el servidor **se calla la tendencia** —lo hace con menos de seis builds,
/// porque dos datos no son una tendencia y fingirla es peor que no tenerla— la
/// interfaz respeta ese silencio en vez de rellenarlo: se usan los pesos por
/// defecto y se dice que es una estimación sin histórico.
pub fn pesos(build_mediana_s: Option<u64>, total_mediana_s: Option<u64>) -> [f32; 6] {
    let por_defecto = [0.05, 0.05, 0.70, 0.03, 0.12, 0.05];
    let (b, t) = match (build_mediana_s, total_mediana_s) {
        (Some(b), Some(t)) if t > 0 && b <= t => (b as f32, t as f32),
        _ => return por_defecto,
    };
    let peso_build = (b / t).clamp(0.05, 0.95);
    // El resto se reparte proporcionalmente a lo que pesaban por defecto entre
    // ellos, para no inventarse una distribución nueva con un solo dato.
    let resto_defecto: f32 =
        por_defecto[0] + por_defecto[1] + por_defecto[3] + por_defecto[4] + por_defecto[5];
    let escala = (1.0 - peso_build) / resto_defecto;
    [
        por_defecto[0] * escala,
        por_defecto[1] * escala,
        peso_build,
        por_defecto[3] * escala,
        por_defecto[4] * escala,
        por_defecto[5] * escala,
    ]
}

/// Hasta dónde llega la barra con los pasos ya terminados.
///
/// **Monótona creciente, nunca retrocede.** Una barra que retrocede destruye más
/// confianza que cualquier error, así que si un cálculo diera menos que lo ya
/// pintado, manda lo ya pintado.
pub fn avance(terminados: &[String], pesos: &[f32; 6], anterior: f32) -> f32 {
    let mut a = 0.0f32;
    for (i, (nombre, _, _)) in PASOS.iter().enumerate() {
        if terminados.iter().any(|t| t == nombre) {
            a += pesos[i];
        }
    }
    a.max(anterior).clamp(0.0, 1.0)
}

/// Lee el progreso. Misma regla: **una línea rota no tumba un despliegue.**
///
/// Es la diferencia entre un cliente utilizable y uno que se cae a los dos
/// minutos y medio de un build de tres, y deja al usuario sin saber si su web
/// está publicada.
pub fn leer_progreso(texto: &str) -> (Vec<PasoDeDespliegue>, usize) {
    let mut pasos = Vec::new();
    let mut rotas = 0usize;
    for linea in texto.lines() {
        let t = linea.trim();
        // Por stderr no viene sólo NDJSON: ahí va también todo lo que Orbit le
        // cuenta a una persona. Lo que no empieza por `{` no es un suceso y no
        // es basura — es prosa, y se ignora sin contarla como rota.
        if !t.starts_with('{') {
            continue;
        }
        match serde_json::from_str::<PasoDeDespliegue>(t) {
            Ok(p) => pasos.push(p),
            Err(_) => rotas += 1,
        }
    }
    (pasos, rotas)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLUJO: &str = r#"{"schema":1,"event":"meta","app":"mi-web","source":"nginx","unit":null,"since":null,"follow":false,"lines":80}
{"event":"line","ts":"2026-08-29T14:02:11+02:00","stream":"access","text":"GET / 200"}
{"event":"line","ts":null,"stream":"access","text":"sin marca de tiempo"}
{"event":"line","ts":"2026-08-29T14:03:01","stream":"error","text":"open() failed"}
{"event":"end","lines":3,"truncated":false}"#;

    #[test]
    fn se_lee_el_flujo_entero() {
        let l = leer_log(FLUJO);
        assert_eq!(l.sucesos.len(), 5);
        assert_eq!(l.rotas, 0);
        assert!(matches!(l.sucesos[0], SucesoDeLog::Meta(_)));
        assert!(matches!(l.sucesos[4], SucesoDeLog::Fin(_)));
    }

    #[test]
    fn la_primera_linea_trae_el_schema() {
        // Es lo que permite no adivinar si viene un objeto o un flujo.
        let l = leer_log(FLUJO);
        match &l.sucesos[0] {
            SucesoDeLog::Meta(m) => {
                assert_eq!(m.schema, 1);
                assert!(!m.follow);
            }
            _ => panic!("la primera línea tiene que ser el meta"),
        }
    }

    #[test]
    fn el_acceso_y_el_error_se_distinguen() {
        // Es lo que la salida en prosa pierde, porque `tail` los mezcla.
        let l = leer_log(FLUJO);
        let canales: Vec<Canal> = l
            .sucesos
            .iter()
            .filter_map(|s| match s {
                SucesoDeLog::Linea(x) => Some(x.stream),
                _ => None,
            })
            .collect();
        assert_eq!(canales, [Canal::Access, Canal::Access, Canal::Error]);
    }

    #[test]
    fn una_linea_sin_marca_de_tiempo_es_null_y_no_se_inventa() {
        let l = leer_log(FLUJO);
        let sin = l.sucesos.iter().find_map(|s| match s {
            SucesoDeLog::Linea(x) if x.ts.is_none() => Some(x),
            _ => None,
        });
        assert!(
            sin.is_some(),
            "hay una línea sin marca y tiene que llegar como null"
        );
        assert_eq!(sin.unwrap().text, "sin marca de tiempo");
    }

    #[test]
    fn una_linea_rota_no_tumba_el_flujo() {
        // Un byte mal puesto no puede convertir un log en una pantalla vacía, y
        // quien mira un log suele estar mirándolo porque algo va mal.
        let roto = FLUJO.replace(
            r#"{"event":"line","ts":null,"stream":"access","text":"sin marca de tiempo"}"#,
            r#"{"event":"line","ts":null,"stream":"acce"#,
        );
        let l = leer_log(&roto);
        assert_eq!(l.rotas, 1);
        assert_eq!(l.sucesos.len(), 4, "el resto tiene que seguir leyéndose");
    }

    #[test]
    fn las_lineas_rotas_se_cuentan_y_no_se_callan() {
        // Si se callaran, un log a medias se leería como un log completo.
        let l = leer_log("no soy json\n{roto\n");
        assert_eq!(l.rotas, 2);
        assert!(l.sucesos.is_empty());
    }

    #[test]
    fn un_suceso_que_no_conocemos_se_ignora_sin_ruido() {
        // Los sucesos se añaden, igual que los campos.
        let l = leer_log(r#"{"event":"delfuturo","algo":1}"#);
        assert_eq!(l.rotas, 0);
        assert!(l.sucesos.is_empty());
    }

    #[test]
    fn siguiendo_en_vivo_no_hay_fin_y_eso_es_correcto() {
        let vivo = FLUJO
            .replace(r#""follow":false"#, r#""follow":true"#)
            .replace("\n{\"event\":\"end\",\"lines\":3,\"truncated\":false}", "");
        let l = leer_log(&vivo);
        assert!(!l.sucesos.iter().any(|s| matches!(s, SucesoDeLog::Fin(_))));
        match &l.sucesos[0] {
            SucesoDeLog::Meta(m) => assert!(m.follow, "el meta ya avisó de que no habría fin"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn el_tope_se_anuncia() {
        let l = leer_log(&FLUJO.replace(r#""truncated":false"#, r#""truncated":true"#));
        match l.sucesos.last().unwrap() {
            SucesoDeLog::Fin(f) => assert!(f.truncated),
            _ => panic!(),
        }
    }

    #[test]
    fn el_progreso_ignora_la_prosa_sin_contarla_como_rota() {
        // Por stderr va también todo lo que Orbit le cuenta a una persona.
        let e = "  ✔ Compilando\n\
                 {\"event\":\"step\",\"app\":\"mi-web\",\"step\":\"build\",\"status\":\"start\"}\n\
                 Ctrl-C para salir\n\
                 {\"event\":\"step\",\"app\":\"mi-web\",\"step\":\"build\",\"status\":\"ok\"}\n";
        let (pasos, rotas) = leer_progreso(e);
        assert_eq!(pasos.len(), 2);
        assert_eq!(rotas, 0, "la prosa no es una línea rota");
    }

    #[test]
    fn cada_paso_de_un_lote_lleva_su_app() {
        // Sin ese campo, un paso de `deploy --all` no se puede atribuir.
        let e = "{\"event\":\"step\",\"app\":\"una\",\"step\":\"build\",\"status\":\"start\"}\n\
                 {\"event\":\"step\",\"app\":\"otra\",\"step\":\"build\",\"status\":\"start\"}\n";
        let (pasos, _) = leer_progreso(e);
        assert_eq!(pasos[0].app.as_deref(), Some("una"));
        assert_eq!(pasos[1].app.as_deref(), Some("otra"));
    }

    #[test]
    fn los_pesos_por_defecto_suman_uno_y_el_build_manda() {
        let p = pesos(None, None);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 0.001);
        // El build es el paso largo: una barra que le diera un sexto se
        // quedaría clavada dos minutos y luego correría hasta el final.
        assert!(p[2] > 0.5);
    }

    #[test]
    fn con_historico_los_pesos_salen_de_lo_medido() {
        // 90 s de build sobre 120 s totales: el build pesa tres cuartos.
        let p = pesos(Some(90), Some(120));
        assert!((p[2] - 0.75).abs() < 0.01);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 0.001);
    }

    #[test]
    fn un_historico_absurdo_no_desbarata_la_barra() {
        // Un build más largo que el total no puede pasar, y si pasa no se
        // convierte en una barra con pesos negativos.
        let p = pesos(Some(500), Some(10));
        assert_eq!(p, pesos(None, None), "se vuelve a los pesos por defecto");
        let q = pesos(Some(0), Some(0));
        assert_eq!(q, pesos(None, None));
    }

    #[test]
    fn la_barra_nunca_retrocede() {
        // Una barra que retrocede destruye más confianza que cualquier error.
        let p = pesos(None, None);
        let hechos = vec!["code".to_string(), "release".to_string()];
        let a = avance(&hechos, &p, 0.0);
        // Un cálculo que diera menos que lo ya pintado no puede ganar.
        assert_eq!(avance(&[], &p, a), a);
        assert!(avance(&hechos, &p, 0.9) >= 0.9);
    }

    #[test]
    fn la_barra_no_llega_al_final_antes_de_tiempo() {
        let p = pesos(None, None);
        let casi: Vec<String> = PASOS[..5].iter().map(|(n, _, _)| n.to_string()).collect();
        assert!(avance(&casi, &p, 0.0) < 1.0);
        let todos: Vec<String> = PASOS.iter().map(|(n, _, _)| n.to_string()).collect();
        assert!((avance(&todos, &p, 0.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn una_linea_de_progreso_rota_no_tumba_el_despliegue() {
        // Es la diferencia entre un cliente utilizable y uno que se cae a los
        // dos minutos y medio de un build de tres.
        let e = "{\"event\":\"step\",\"app\":\"x\",\"step\":\"build\",\"status\":\"start\"}\n\
                 {\"event\":\"step\",\"app\":\n\
                 {\"event\":\"step\",\"app\":\"x\",\"step\":\"build\",\"status\":\"ok\"}\n";
        let (pasos, rotas) = leer_progreso(e);
        assert_eq!(pasos.len(), 2);
        assert_eq!(rotas, 1);
    }
}
