//! Segunda ronda adversaria antes de la 0.1.2 — el núcleo.
//!
//! Empezaron siendo sondas escritas para fallar mientras cada fallo existía; se
//! quedan como pruebas de regresión. Cada bloque dice qué pasaba.

use orbit_client::comando::{Comando, ErrorForma, ModoDeExec};
use orbit_client::saludo::{self, Saludo};
use orbit_client::transporte::{self, ErrorTransporte, Respuesta};

const B: &str = "/usr/local/bin/orbit";

// ── 1. El aviso del .bashrc sólo se perdonaba con rc 0 ──────────────────────
//
// `saludo::clasificar` miraba «command not found» en stderr **antes** que el
// resto de señales cuando rc != 0. Un Orbit anterior a la 1.1 (rechaza
// `--json`, rc 1) detrás de un `.bashrc` con `rbenv: command not found` salía
// como «no hay ningún orbit en esa ruta» en vez de «sano y viejo».
#[test]
fn rc2_un_orbit_viejo_tras_un_bashrc_ruidoso_no_es_no_instalado() {
    let r = Respuesta {
        stdout: String::new(),
        stderr: "bash: rbenv: command not found\n  ✗ 'orbit version' no tiene salida JSON. La tienen: list, info\n"
            .into(),
        codigo: 1,
        cancelado: false,
    };
    let s = saludo::clasificar(&r);
    assert!(
        matches!(s, Saludo::SinContrato { .. }),
        "un Orbit pre-1.1 sale como {s:?}"
    );
}

#[test]
fn rc2_un_orbit_sin_instalar_tras_un_bashrc_ruidoso_dice_lo_que_falta() {
    // El binario está pero falta orbit.conf (lo dice por stdout); el .bashrc
    // añade su «command not found». El motivo que se enseña es el de orbit.conf.
    let r = Respuesta {
        stdout: "Orbit no está instalado: falta /etc/orbit/orbit.conf\n".into(),
        stderr: "bash: rbenv: command not found\n".into(),
        codigo: 1,
        cancelado: false,
    };
    match saludo::clasificar(&r) {
        Saludo::NoInstalado { motivo } => {
            assert!(motivo.contains("orbit.conf"), "motivo equivocado: {motivo}")
        }
        otro => panic!("{otro:?}"),
    }
}

// ── 2. `--lang` como valor se lo come el servidor ───────────────────────────
//
// `main()` del servidor pasa `_lang_strip` a todos los argumentos de todas las
// órdenes salvo `exec` (orbit ~19410). Un motivo de `lock` que sea `--lang`,
// `--idioma` o `--lang=xx` desaparece: `lock web -m` muere con «necesita un
// valor» (o «--lang necesita un idioma»), y `--lang=xx` además cambia el
// idioma de la respuesta. El cliente lo aceptaba y lo mandaba; ahora lo
// rechaza diciendo por qué.
#[test]
fn rc2_un_motivo_que_el_servidor_leeria_como_lang_no_se_manda() {
    for m in ["--lang", "--idioma", "--lang=en", "--idioma=fr"] {
        let c = Comando::Congelar {
            app: "web".into(),
            motivo: Some(m.into()),
        };
        assert_eq!(c.argv(B), Err(ErrorForma::ValorEsIdioma(m.into())));
    }
}

// ── 3. Un 255 de un `ssh` DENTRO de exec ────────────────────────────────────
//
// `clasificar` aceptaba un 255 de `exec` como «no he llegado» si stderr
// «hablaba el cliente ssh», y como suplantación si decía «host key
// verification failed». Pero un `ssh`/`scp`/`rsync` que el usuario lanza dentro
// de la app escribe exactamente eso y sale con 255: la interfaz tiraba su
// salida y decía que **nuestro** servidor no contestaba, o enseñaba la pantalla
// de impostor. Ahora sólo es nuestro si el mensaje nombra a nuestro destino.
//
// El «servidor» es un script local que escribe esas líneas por stderr, una por
// stdout y sale con 255.
fn falso(nombre: &str, lineas: &[&str]) -> String {
    let dir = std::env::temp_dir().join(format!("orbit-rc2-exec255-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(windows)]
    let p = {
        let p = dir.join(format!("{nombre}.bat"));
        let mut t = String::from("@echo off\r\n");
        for l in lineas {
            // `^!` porque cmd se come el `!` sin escapar.
            t.push_str(&format!("echo {} 1>&2\r\n", l.replace('!', "^!")));
        }
        t.push_str("echo salida-del-comando\r\nexit /b 255\r\n");
        std::fs::write(&p, t).unwrap();
        p
    };
    #[cfg(not(windows))]
    let p = {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(format!("{nombre}.sh"));
        let mut t = String::from("#!/bin/sh\n");
        for l in lineas {
            let l = l.replace('\'', r"'\''");
            t.push_str(&format!("printf '%s\\n' '{l}' >&2\n"));
        }
        t.push_str("echo salida-del-comando\nexit 255\n");
        std::fs::write(&p, t).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    };
    p.to_string_lossy().into_owned()
}

fn exec_ssh() -> Comando {
    Comando::Ejecutar {
        app: "web".into(),
        modo: ModoDeExec::Argumentos(vec!["ssh".into(), "db.interno".into()]),
    }
}

/// Lo que tiene que llegar: la salida del comando del usuario, con su 255.
fn es_la_salida_del_comando(r: Result<Respuesta, ErrorTransporte>) {
    match r {
        Err(ErrorTransporte::Orbit {
            codigo: 255,
            stdout,
            ..
        }) => assert!(stdout.contains("salida-del-comando"), "{stdout:?}"),
        otro => panic!("la salida del comando del usuario se ha perdido: {otro:?}"),
    }
}

#[test]
fn rc2_un_ssh_interno_que_no_conecta_no_es_que_el_servidor_no_conteste() {
    let bin = falso(
        "noconecta",
        &["ssh: connect to host db.interno port 22: Connection refused"],
    );
    es_la_salida_del_comando(transporte::ejecutar_local(&bin, &exec_ssh(), &[]));
}

#[test]
fn rc2_la_clave_de_host_de_un_ssh_interno_no_es_la_del_servidor() {
    let bin = falso(
        "hostkey",
        &[
            "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@",
            "@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @",
            "Host key for db.interno has changed and you have requested strict checking.",
            "Host key verification failed.",
        ],
    );
    es_la_salida_del_comando(transporte::ejecutar_local(&bin, &exec_ssh(), &[]));
}

// ── 4. El modo «shell» sólo era shell si el texto tenía un metacarácter ─────
//
// `ModoDeExec::Shell(t)` mandaba `t` como argumento único y confiaba en la
// regla del servidor (`[[:space:]|&;<>()$\`]`, orbit ~13355) para que fuera a
// `bash -lc`. Un texto de una palabra sin esos caracteres —`~/bin/backup.sh`,
// `*.sh`, `{a,b}`— no la cumple: el servidor lo ejecutaba tal cual, sin shell,
// y `~` no se expandía («No such file or directory»). Ahora sale siempre como
// `/bin/bash -lc <texto>`.
#[test]
fn rc2_el_modo_shell_pasa_por_un_shell_aunque_el_texto_sea_una_palabra() {
    for t in ["~/bin/backup.sh", "~/.local/bin/tarea", "*.sh", "ls -la"] {
        let v = Comando::Ejecutar {
            app: "web".into(),
            modo: ModoDeExec::Shell(t.into()),
        }
        .argv(B)
        .unwrap();
        assert_eq!(v, [B, "exec", "web", "/bin/bash", "-lc", t], "{t:?}");
    }
}
