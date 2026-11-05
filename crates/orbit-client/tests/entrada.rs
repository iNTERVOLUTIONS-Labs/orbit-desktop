//! `deploy --env-stdin` (Orbit 1.4.9): los secretos por la entrada estándar.
//!
//! Lo que se comprueba es lo que le llega al servidor, no lo que contesta:
//!
//! * **En el `argv` no va ni un valor**, y tampoco un `--env`. El servidor falso
//!   apunta su `argv` entero, y ésa es la aserción.
//! * **Lo que llega por la entrada es exactamente lo que se escribió**, leído
//!   con el `_env_unquote` de Orbit 1.4.9 copiado tal cual en el falso: sin
//!   comillas, Orbit corta un ` #` y quita los espacios del final.
//! * **La entrada se cierra.** `--env-stdin` lee hasta el final; con la tubería
//!   abierta Orbit esperaría para siempre. El falso espera cinco segundos por
//!   línea y sale con 99: si el transporte no la cerrara, estas pruebas lo
//!   dirían en vez de colgarse con el tope de una hora de un despliegue.
//! * **Un servidor anterior se niega, y no se filtra nada**: `--env-stdin` no
//!   existe ahí, y el despliegue muere antes de empezar con los valores fuera
//!   del `argv`.

use std::time::{Duration, Instant};

use orbit_client::comando::{Comando, Secreto, VariableNueva};
use orbit_client::contrato::Despliegue;
use orbit_client::transporte::{
    ejecutar_en_vivo_local, ejecutar_local, EnCurso, ErrorTransporte, Respuesta,
};

const HOSTILES: [(&str, &str); 6] = [
    ("DB_PASSWORD", "hunter2"),
    ("COMILLAS", r#"p@ss w"ord"#),
    ("COMENTARIO", "abc #no es un comentario  "),
    ("BARRA", r"back\slash\"),
    ("SHELL", "$(id) `id` ${HOME} ; | &&"),
    ("VACIA", ""),
];

fn binario() -> String {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    raiz.join("tests/fakeserver/orbit")
        .to_string_lossy()
        .into_owned()
}

fn desplegar(por_entrada: bool) -> Comando {
    Comando::Desplegar {
        app: "tienda".into(),
        progreso: true,
        entorno: HOSTILES
            .iter()
            .map(|(k, v)| VariableNueva {
                clave: (*k).into(),
                valor: Secreto::nuevo(*v),
            })
            .collect(),
        forzar: false,
        por_entrada,
    }
}

struct Registro {
    dir: std::path::PathBuf,
}

impl Registro {
    fn nuevo(nombre: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("entrada-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }
    fn ruta(&self, f: &str) -> String {
        self.dir.join(f).to_string_lossy().into_owned()
    }
    fn campos(&self, f: &str) -> Vec<String> {
        let bruto = std::fs::read(self.dir.join(f)).unwrap_or_default();
        let mut v: Vec<String> = String::from_utf8_lossy(&bruto)
            .split('\0')
            .map(String::from)
            .collect();
        v.pop();
        v
    }
}

impl Drop for Registro {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn comprobar(r: &Registro, resp: Result<Respuesta, ErrorTransporte>, t: Duration) {
    let resp = resp.expect("el despliegue del falso contesta");
    // Si la entrada no se hubiera cerrado, el falso habría salido con 99 a los
    // cinco segundos.
    assert_eq!(resp.codigo, 0, "stderr: {}", resp.stderr);
    assert!(
        t < Duration::from_secs(4),
        "tardó {t:?}: ¿se cerró la entrada?"
    );
    let d: Despliegue = resp.leer().unwrap();
    assert!(d.ok);

    let argv = r.campos("argv.log");
    assert!(argv.contains(&"--env-stdin".to_string()));
    assert!(!argv.iter().any(|a| a == "--env"), "{argv:?}");
    for (_, v) in HOSTILES.iter().filter(|(_, v)| !v.is_empty()) {
        assert!(
            !argv.iter().any(|a| a.contains(v)),
            "un valor ha llegado al argv: {argv:?}"
        );
    }

    let llegado = r.campos("env.log");
    let esperado: Vec<String> = HOSTILES.iter().map(|(k, v)| format!("{k}={v}")).collect();
    assert_eq!(
        llegado, esperado,
        "lo leído por la entrada no es lo escrito"
    );
}

#[test]
fn por_la_entrada_los_valores_llegan_enteros_y_el_argv_va_limpio() {
    let r = Registro::nuevo("una");
    let (log, env) = (r.ruta("argv.log"), r.ruta("env.log"));
    let t0 = Instant::now();
    let resp = ejecutar_local(
        &binario(),
        &desplegar(true),
        &[
            ("ORBIT_FAKE_CASE", "orbit-1.4.9"),
            ("ORBIT_FAKE_LOG", &log),
            ("ORBIT_FAKE_ENV", &env),
        ],
    );
    comprobar(&r, resp, t0.elapsed());
}

#[test]
fn en_vivo_tambien_se_escribe_y_se_cierra() {
    // El camino de la interfaz: el despliegue con su progreso servido.
    let r = Registro::nuevo("vivo");
    let (log, env) = (r.ruta("argv.log"), r.ruta("env.log"));
    let t0 = Instant::now();
    let resp = ejecutar_en_vivo_local(
        &binario(),
        &desplegar(true),
        &[
            ("ORBIT_FAKE_CASE", "orbit-1.4.9"),
            ("ORBIT_FAKE_LOG", &log),
            ("ORBIT_FAKE_ENV", &env),
        ],
        EnCurso::nuevo(),
        |_| {},
    );
    comprobar(&r, resp, t0.elapsed());
}

#[test]
fn un_servidor_anterior_se_niega_sin_que_el_valor_toque_el_argv() {
    let r = Registro::nuevo("vieja");
    let log = r.ruta("argv.log");
    let e = ejecutar_local(
        &binario(),
        &desplegar(true),
        &[("ORBIT_FAKE_CASE", "orbit-1.4"), ("ORBIT_FAKE_LOG", &log)],
    )
    .unwrap_err();
    assert!(e.to_string().contains("--env-stdin"), "{e}");
    let argv = r.campos("argv.log");
    assert!(!argv.join(" ").contains("hunter2"));
}

#[test]
fn sin_la_entrada_las_variables_siguen_yendo_por_el_argv() {
    // El camino de 1.4.3–1.4.8, con su aviso en la pantalla: no ha cambiado.
    let r = Registro::nuevo("argv");
    let log = r.ruta("argv.log");
    ejecutar_local(
        &binario(),
        &desplegar(false),
        &[("ORBIT_FAKE_CASE", "orbit-1.4"), ("ORBIT_FAKE_LOG", &log)],
    )
    .unwrap();
    let argv = r.campos("argv.log");
    assert!(argv.contains(&"DB_PASSWORD=hunter2".to_string()));
    assert!(!argv.contains(&"--env-stdin".to_string()));
}

#[test]
fn una_orden_sin_entrada_no_deja_la_entrada_abierta() {
    // El resto del catálogo sigue con la entrada a `null`: nadie va a
    // contestar a un selector, y un `read` del otro lado ve un EOF al momento.
    let t0 = Instant::now();
    ejecutar_local(
        &binario(),
        &Comando::Lista,
        &[("ORBIT_FAKE_CASE", "orbit-1.4.9")],
    )
    .unwrap();
    assert!(t0.elapsed() < Duration::from_secs(4));
}
