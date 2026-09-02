//! Las reglas duras, comprobadas leyendo el código.
//!
//! Están aquí y no en una lista de revisión porque **una regla que depende de
//! que alguien se acuerde no sobrevive al commit 400**. Es la misma lección que
//! `ARCHITECTURE §13.6c` de Orbit le achaca a su propio `deploy`: *una regla que
//! se cumple acordándose no sobrevive a un comando largo*.
//!
//! Son toscas —leen ficheros y buscan cadenas— y esa tosquedad es deliberada:
//! una comprobación que se entiende en diez segundos se arregla cuando falla, y
//! una que no, se desactiva.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fuentes() -> Vec<(String, String)> {
    let src = crate_dir().join("src");
    let mut v = Vec::new();
    for e in fs::read_dir(&src).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            let nombre = p.file_name().unwrap().to_string_lossy().into_owned();
            v.push((nombre, fs::read_to_string(&p).unwrap()));
        }
    }
    v
}

/// El código que se envía: sin comentarios y **sin el módulo de pruebas**.
///
/// Las dos exclusiones costaron un falso positivo cada una la primera vez que se
/// ejecutó esto. Un comentario que menciona `ssh` es exactamente lo que queremos
/// que exista; y una prueba que comprueba que **no** aparece
/// `StrictHostKeyChecking=no` contiene esa cadena literalmente, así que
/// escanearla acusaba al fichero de justo lo que estaba impidiendo.
fn codigo_enviado(s: &str) -> String {
    let s = s.split("#[cfg(test)]").next().unwrap_or(s);
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("//") && !l.starts_with("/*") && !l.starts_with('*'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// **SEC-01 y SEC-04.** Sólo dos ficheros pueden lanzar un proceso, y hacen
/// cosas distintas.
///
/// Es la mitigación estructural de la inyección de comandos: si el sitio donde
/// una lista de argumentos se convierte en una cadena de shell es uno solo, se
/// prueba una vez y se audita una vez. Repartido por veinte pantallas, se
/// equivoca en la diecinueve.
///
/// `descubrir.rs` está en la lista y la regla **se afinó, no se relajó**, que
/// es una distinción que importa. Lo que hace es `ssh -G <alias>`: una consulta
/// **local** que no abre ninguna conexión y que no construye ninguna cadena de
/// shell —los argumentos van por `argv` a un `execve`, sin shell de por medio—.
/// La comprobación de abajo lo fija: ahí no puede aparecer nunca el binario de
/// Orbit ni una llamada al transporte.
///
/// Y afinarla salió barato: al mirar por qué este fichero la incumplía apareció
/// que un alias que empezara por guion se le habría pasado a `ssh` como una
/// **opción** en el sitio donde va un nombre. Ahora se descarta.
#[test]
fn solo_dos_ficheros_lanzan_procesos() {
    const PERMITIDOS: [&str; 2] = ["transporte.rs", "descubrir.rs"];
    for (nombre, texto) in fuentes() {
        if PERMITIDOS.contains(&nombre.as_str()) {
            continue;
        }
        let codigo = codigo_enviado(&texto);
        for prohibido in ["Command::new", "std::process", "\"ssh\""] {
            assert!(
                !codigo.contains(prohibido),
                "{nombre} contiene «{prohibido}»: sólo {PERMITIDOS:?} pueden lanzar procesos"
            );
        }
    }
}

/// Y descubrir servidores **no puede convertirse en hablar con ellos**.
///
/// Enumerar no es visitar. Si este fichero llegara a invocar el binario de
/// Orbit o al transporte, abrir una pantalla pasaría a significar abrir
/// cuarenta sesiones SSH — y la lista de comandos que el cliente puede generar
/// dejaría de estar en un solo sitio.
#[test]
fn descubrir_no_habla_con_ningun_servidor() {
    let c = codigo_enviado(&fs::read_to_string(crate_dir().join("src/descubrir.rs")).unwrap());
    assert!(
        c.contains("\"-G\""),
        "la consulta tiene que ser la local de ssh"
    );
    for prohibido in ["orbit", "transporte::", "ejecutar("] {
        assert!(
            !c.contains(prohibido),
            "descubrir.rs menciona «{prohibido}»: enumerar no es visitar"
        );
    }
}

/// **SEC-02.** Nadie construye una cadena de comando a mano.
#[test]
fn nadie_concatena_una_orden() {
    for (nombre, texto) in fuentes() {
        if nombre == "shquote.rs" {
            continue; // es quien la construye, y por eso tiene la prueba de propiedad
        }
        let codigo = codigo_enviado(&texto);
        // El patrón que delataría una concatenación: interpolar dentro de una
        // cadena que ya lleva el binario o un subcomando.
        for sospechoso in ["format!(\"orbit ", "format!(\"ssh ", "+ \" orbit"] {
            assert!(
                !codigo.contains(sospechoso),
                "{nombre} parece construir una orden concatenando: «{sospechoso}»"
            );
        }
    }
}

/// **SEC-03.** `orbit` nunca se invoca por `PATH`.
///
/// Un `PATH` manipulado en el `.bashrc` del usuario remoto —o por quien sólo
/// tenga escritura en su `$HOME`— redirige todos los comandos a otro binario.
#[test]
fn el_binario_va_por_ruta_absoluta() {
    let c = orbit_client::comando::Comando::Version;
    assert!(c.argv("/usr/local/bin/orbit").unwrap()[0].starts_with('/'));
    let t = fs::read_to_string(crate_dir().join("src/transporte.rs")).unwrap();
    assert!(
        t.contains("\"/usr/local/bin/orbit\""),
        "el valor por defecto tiene que ser una ruta absoluta"
    );
}

/// **SEC-09 y SEC-10.** La política del canal no se puede relajar sin que esto
/// falle.
#[test]
fn la_politica_del_canal_ssh_no_se_relaja() {
    let t = codigo_enviado(&fs::read_to_string(crate_dir().join("src/transporte.rs")).unwrap());
    // Un cambio de clave de un host conocido tiene que seguir siendo un error.
    assert!(!t.contains("StrictHostKeyChecking=no"));
    assert!(t.contains("StrictHostKeyChecking=accept-new"));
    // Reenviar el agente le da a un servidor la clave del usuario mientras la
    // sesión esté abierta: es T-02 escalando a la pérdida total.
    assert!(!t.contains("ForwardAgent=yes"));
    assert!(t.contains("ForwardAgent=no"));
    // Un master eterno es una sesión root abierta hasta el siguiente reinicio.
    assert!(!t.contains("ControlPersist=yes"));
}

/// **La regla nº 1**, hecha auditable: el catálogo de órdenes es finito y está
/// en un fichero. Esta prueba no puede comprobar que sea correcto —eso se hace
/// leyéndolo— pero sí que **siga siendo pequeño y que esté donde se dijo**.
///
/// Y publica el número, que es el dato que hay que mirar en cada revisión: si la
/// lista crece, la superficie crece, y crece en silencio.
#[test]
fn el_catalogo_de_ordenes_es_finito_y_vive_en_un_sitio() {
    let c = fs::read_to_string(crate_dir().join("src/comando.rs")).unwrap();
    // Se cuentan las llaves para encontrar el final del enumerado. La primera
    // versión partía por el primer '}' y daba **4** en vez de 20, porque
    // variantes como `Info { app: String }` llevan una llave dentro. Es la clase
    // de cifra que nadie comprueba —y este número es justo el que hay que mirar
    // en cada revisión, así que tiene que ser exacto o no vale nada.
    let dentro = c
        .split("pub enum Comando")
        .nth(1)
        .expect("falta el enumerado");
    let abre = dentro.find('{').expect("el enumerado no abre");
    let mut prof = 0i32;
    let mut fin = abre;
    for (i, ch) in dentro.char_indices().skip(abre) {
        match ch {
            '{' => prof += 1,
            '}' => {
                prof -= 1;
                if prof == 0 {
                    fin = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let cuerpo = &dentro[abre + 1..fin];
    // Una variante es una línea al primer nivel de indentación que empieza por
    // mayúscula. Los campos de dentro van más indentados o en la misma línea.
    let variantes = cuerpo
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            l.len() - t.len() == 4
                && !t.starts_with("//")
                && t.chars().next().is_some_and(char::is_uppercase)
        })
        .count();
    println!("el cliente puede generar {variantes} órdenes distintas");
    assert!(variantes > 0);
    // El tope no es mágico: es un aviso. Si hace falta pasarlo, se sube **en el
    // mismo commit** que añade la orden, y entonces alguien lo ve en el diff.
    assert!(
        variantes <= 30,
        "el catálogo ha llegado a {variantes} órdenes. Eso es más superficie de \
         la prevista: revisa si de verdad hace falta y sube el tope a propósito"
    );
    // Y ninguna otra parte del crate define órdenes por su cuenta.
    for (nombre, texto) in fuentes() {
        if nombre == "comando.rs" {
            continue;
        }
        assert!(
            !codigo_enviado(&texto).contains("\"deploy\""),
            "{nombre} nombra un subcomando: el catálogo vive en comando.rs"
        );
    }
}

/// **SEC-11 y T-12.** Nada del contrato se persiste.
///
/// La versión anterior de esta prueba decía «el núcleo no escribe en disco», a
/// secas, y cayó el día que hubo que guardar los servidores que alguien añade a
/// mano. Borrarla habría sido perder la regla; dejarla habría sido no poder
/// arreglar el fallo. Así que se afina, porque **la regla que importa nunca fue
/// «no escribir»**: era que lo que dice un servidor no acaba en el disco de
/// nadie.
///
/// Lo que se puede escribir: **cómo llegar** a un servidor —alias, host,
/// usuario, puerto y la ruta de una clave—, que es la misma clase de dato que
/// lleva un `~/.ssh/config` en claro desde siempre.
///
/// Lo que no: nada que haya contestado un servidor, y ninguna credencial.
#[test]
fn solo_el_registro_escribe_y_solo_escribe_como_llegar() {
    for (nombre, texto) in fuentes() {
        let codigo = codigo_enviado(&texto);
        for prohibido in ["fs::write", "File::create", "OpenOptions"] {
            if !codigo.contains(prohibido) {
                continue;
            }
            assert_eq!(
                nombre, "registro.rs",
                "{nombre} escribe en disco. El único que puede es el registro de \
                 servidores, y sólo con cómo llegar a ellos: si hace falta persistir \
                 algo que diga un servidor, primero hay que decidir qué y añadir el \
                 barrido de secretos de docs/QA.md §5.5"
            );
        }
    }
}

/// Y el registro no tiene dónde guardar lo que no debe.
///
/// Se comprueba sobre el **tipo**, no sobre lo que se escriba hoy: el día que
/// alguien añada un campo para una contraseña «sólo para no tener que teclearla»,
/// esta prueba se lo dice antes de que llegue a un fichero.
#[test]
fn el_registro_no_puede_guardar_una_credencial_ni_una_respuesta() {
    let c = fs::read_to_string(crate_dir().join("src/registro.rs")).unwrap();
    let tipo = c
        .split("pub struct ServidorGuardado {")
        .nth(1)
        .expect("falta el tipo")
        .split("\n}")
        .next()
        .unwrap();
    let campos = codigo_enviado(tipo);

    for prohibido in [
        "contrasena",
        "password",
        "passphrase",
        "frase",
        "secreto",
        "token",
        "contenido",
        "privada",
    ] {
        assert!(
            !campos.contains(prohibido),
            "el registro tiene un campo «{prohibido}»: ahí no va ninguna credencial, \
             sólo la RUTA de una clave"
        );
    }
    // Y nada que venga del contrato: una app, un estado, una release.
    for del_servidor in ["app", "estado", "state", "release", "releases", "log"] {
        assert!(
            !campos.contains(del_servidor),
            "el registro tiene un campo «{del_servidor}»: lo que dice un servidor no \
             se guarda"
        );
    }
}

/// El servidor falso tiene que existir y ser ejecutable, o las pruebas de
/// integración se saltan solas y la suite sale en verde habiendo probado nada.
#[test]
fn el_servidor_falso_esta_donde_se_dijo() {
    let raiz = crate_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let falso = raiz.join("tests/fakeserver/orbit");
    assert!(falso.exists(), "falta {}", falso.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = fs::metadata(&falso).unwrap().permissions().mode();
        assert!(m & 0o111 != 0, "el servidor falso no es ejecutable");
    }
    let fixtures = raiz.join("tests/fakeserver/fixtures");
    let n = fs::read_dir(&fixtures).unwrap().count();
    assert!(
        n >= 20,
        "sólo hay {n} respuestas: el catálogo se ha quedado corto"
    );
}

/// Ninguna respuesta del servidor falso puede estar rota: una respuesta que no
/// parsea prueba el caso equivocado y nadie se entera.
#[test]
fn las_respuestas_sanas_son_json_valido() {
    let raiz = crate_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let dir: &Path = &raiz.join("tests/fakeserver/fixtures");
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let nombre = p.file_name().unwrap().to_string_lossy().into_owned();
        if !nombre.ends_with(".json") {
            continue;
        }
        let t = fs::read_to_string(&p).unwrap();
        serde_json::from_str::<serde_json::Value>(&t)
            .unwrap_or_else(|e| panic!("{nombre} no es JSON válido: {e}"));
    }
}

/// La puerta trasera del transporte tiene **un solo** usuario.
///
/// `ejecutar_crudo` y `ejecutar_crudo_en_vivo` ejecutan una línea que no está en
/// el catálogo, y el catálogo es lo que hace revisable este proyecto: la
/// superficie de lo que el cliente le puede pedir a un servidor cabe en una
/// pantalla. Existen para una sola cosa —instalar Orbit donde todavía no
/// está— y el día que alguien las use para tapar un hueco del contrato, el
/// cliente deja de hablar el contrato y pasa a hablar Bash contra un servidor
/// cuyo layout puede cambiar.
#[test]
fn lo_crudo_solo_lo_usa_el_instalador() {
    for (nombre, texto) in fuentes() {
        // Donde se definen y donde se documentan, no cuenta.
        if nombre == "transporte.rs" || nombre == "instalar.rs" {
            continue;
        }
        let codigo = codigo_enviado(&texto);
        assert!(
            !codigo.contains("ejecutar_crudo"),
            "{nombre} usa la puerta trasera del transporte: la línea la construye \
             `instalar`, y nadie más"
        );
    }
}

/// Y la línea que se ejecuta por ahí **no la compone la interfaz**.
///
/// Es la condición que hace aceptable la excepción: no hay ninguna forma de que
/// texto escrito en un campo llegue a un shell remoto por esa vía.
#[test]
fn la_linea_cruda_sale_de_una_funcion_sin_argumentos() {
    let c = fs::read_to_string(crate_dir().join("src/instalar.rs")).unwrap();
    assert!(
        c.contains("pub fn orden() -> String"),
        "la orden se genera entera, sin recibir nada de fuera"
    );
    assert!(
        c.contains("pub const COMPROBACION: &str"),
        "la comprobación es una constante, no una plantilla"
    );
}
