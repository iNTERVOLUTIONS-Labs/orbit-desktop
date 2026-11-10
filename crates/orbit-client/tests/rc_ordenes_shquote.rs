//! Regresión del escapador frente a fish (ronda adversaria, hallazgo 1).
//!
//! fish lee `\'` y `\\` como escapes **dentro** de comillas simples; con la
//! barra dentro, `\'; id #` ejecutaba `id`. Esto prueba el escapador que se
//! envía (no el gemelo de Python) contra los shells que haya instalados, fish
//! incluido. La prueba estricta —los cuatro POSIX obligatorios— sigue siendo
//! `tests/escapado.rs`; aquí un shell que falta se omite y se dice.

use std::process::Command;

use orbit_client::shquote::{build, shquote};

const SHELLS: &[(&str, &[&str])] = &[
    ("bash", &["bash", "-c"]),
    ("dash", &["dash", "-c"]),
    ("zsh", &["zsh", "-c"]),
    ("busybox-ash", &["busybox", "sh", "-c"]),
    ("fish", &["fish", "--no-config", "-c"]),
];

/// Lo que rompía fish, y sus vecinos.
const CASOS: &[&str] = &[
    r"\'; id #",
    r"a\\b",
    r"abc\",
    r"\",
    r"\'",
    r"'\'",
    r"\\'\\",
    "a'b",
    "'",
    "$(id) `id` ; | & > <",
    "\\\n\\",
    "",
    "x y",
    "~",
    "=Y",
];

fn hay(shell: &[&str]) -> bool {
    Command::new(shell[0])
        .args(&shell[1..])
        .arg("exit 0")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn impresor() -> String {
    let d = std::env::temp_dir().join("orbit-desktop-rc-impresor.sh");
    std::fs::write(
        &d,
        "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\0' \"$a\"; done\n",
    )
    .unwrap();
    d.to_string_lossy().replace('\\', "/")
}

fn ida_y_vuelta(shell: &[&str], argv: &[&str]) -> Result<Vec<String>, String> {
    let imp = impresor();
    let mut completo = vec!["sh", imp.as_str()];
    completo.extend_from_slice(argv);
    let linea = build(&completo).map_err(|e| e.to_string())?;
    let salida = if cfg!(windows) {
        // En Windows, MSYS rehace el argv de la línea de órdenes con sus reglas
        // y se come barras invertidas: eso mide a MSYS, no al shell. La línea
        // va en un fichero, que el shell lee con el mismo analizador que `-c`.
        let f = std::env::temp_dir().join("orbit-desktop-rc-linea.sh");
        std::fs::write(&f, &linea).unwrap();
        Command::new(shell[0])
            .args(&shell[1..shell.len() - 1])
            .arg(f.to_string_lossy().replace('\\', "/"))
            .output()
    } else {
        Command::new(shell[0])
            .args(&shell[1..])
            .arg(&linea)
            .output()
    }
    .map_err(|e| e.to_string())?;
    if !salida.status.success() {
        return Err(String::from_utf8_lossy(&salida.stderr).into_owned());
    }
    let s = String::from_utf8_lossy(&salida.stdout).into_owned();
    let mut v: Vec<String> = s.split('\0').map(String::from).collect();
    v.pop();
    Ok(v)
}

#[test]
fn lo_que_rompia_fish_vuelve_entero_en_cada_shell_instalado() {
    let mut probados = Vec::new();
    let mut fallos = Vec::new();
    for (nombre, shell) in SHELLS {
        if !hay(shell) {
            eprintln!("  {nombre:<12} OMITIDO: no está instalado");
            continue;
        }
        for caso in CASOS {
            match ida_y_vuelta(shell, &[caso]) {
                Ok(v) if v == [*caso] => {}
                Ok(v) => fallos.push(format!("{nombre}: {caso:?} volvió como {v:?}")),
                Err(e) => fallos.push(format!("{nombre}: {caso:?} → {e}")),
            }
        }
        probados.push(*nombre);
    }
    eprintln!("shells probados: {probados:?}");
    assert!(!probados.is_empty(), "ningún shell instalado");
    assert!(fallos.is_empty(), "{}", fallos.join("\n"));
}

#[test]
fn la_forma_exacta_no_deja_barras_dentro_de_comillas() {
    assert_eq!(shquote(r"\'; id #").unwrap(), r"\\\''; id #'");
    assert_eq!(shquote(r"a\\b").unwrap(), r"'a'\\\\'b'");
    // Y lo inocuo sigue saliendo sin comillas.
    assert_eq!(shquote("mi-web").unwrap(), "mi-web");
    assert_eq!(shquote("a'b").unwrap(), r"'a'\''b'");
}
