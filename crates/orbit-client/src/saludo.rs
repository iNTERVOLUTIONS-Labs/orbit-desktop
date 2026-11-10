//! El saludo: qué hay al otro lado, antes de fiarse de nada.
//!
//! Es lo primero que se le pregunta a un servidor y lo único que se le puede
//! preguntar antes de saber si habla nuestro contrato. Y clasificarlo bien es
//! la diferencia entre un producto y una demo: sin esto, quien añade un
//! servidor ve «error» y no sabe si es **su clave, su red o su servidor**.
//!
//! Los casos salen de ejecutar el script, no de suponerlos:
//!
//! - `orbit` se auto-eleva a root con `exec sudo`, y **sin TTY eso falla** si
//!   sudo pide contraseña. El mensaje sale por stderr y el comando no llega a
//!   ejecutarse.
//! - Con el binario copiado pero sin instalar, `orbit` imprime dos líneas en
//!   castellano **por stdout** —no por stderr— e **ignora `--json`**. Un cliente
//!   que haga `JSON.parse(stdout)` sin mirar el código de salida explota con un
//!   error incomprensible. Por eso el código de salida se mira primero. Siempre.

use crate::contrato::{Version, CONTRATO_CONOCIDO};
use crate::transporte::{ErrorTransporte, Respuesta};

/// Qué hay al otro lado, y qué se puede hacer con ello.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saludo {
    /// Habla nuestro contrato. Todo disponible.
    Ok(Version),
    /// Habla uno más nuevo. **No es motivo para negarse**: se avisa y se
    /// degrada a lectura. Negarse sería la peor forma de romper algo que
    /// todavía funcionaba, y las garantías permanentes del contrato siguen en
    /// pie — la forma del saludo, la separación de canales y que `null` siga
    /// siendo `null`.
    MasNuevo(Version),
    /// Hay un Orbit, pero anterior al contrato. **Modo informativo, cero
    /// funcionalidad**: un cliente que intente hablar un contrato que no existe
    /// acaba parseando tablas, y ése es justo el camino que el contrato existe
    /// para no tomar.
    SinContrato { version: Option<String> },
    /// No hay `orbit` en esa ruta, o está copiado pero sin instalar.
    NoInstalado { motivo: &'static str },
    /// Hay `orbit`, pero ese usuario no puede elevarse sin escribir una
    /// contraseña, y aquí no hay dónde escribirla.
    SinPrivilegios,
    /// No se llegó al servidor.
    NoSeLlega { detalle: String },
    /// La clave del host ha cambiado. Su propio caso: no es un problema de red.
    ClaveDeHostCambiada { detalle: String },
}

impl Saludo {
    /// Si se puede operar, o sólo mirar, o nada.
    pub fn permite_operar(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
    pub fn permite_leer(&self) -> bool {
        matches!(self, Self::Ok(_) | Self::MasNuevo(_))
    }
}

/// Clasifica lo que contestó `orbit --json version`.
///
/// **El código de salida se mira antes que el contenido.** No es una
/// precaución teórica: un Orbit copiado pero sin instalar escribe prosa por
/// stdout ignorando `--json`, así que parsear primero da un error de sintaxis
/// donde la respuesta correcta es «este servidor tiene Orbit a medias».
///
/// Con una excepción: **rc 0 con un `version` válido por stdout manda** sobre lo
/// que diga stderr. stderr es también del `.bashrc` del usuario SSH, y un
/// `bash: rbenv: command not found` ahí convertía un Orbit sano en «no hay
/// orbit». Con rc 0 y el objeto, quien ha hablado es `orbit`.
///
/// Supone el binario de siempre; con otra ruta, [`clasificar_para`].
pub fn clasificar(r: &Respuesta) -> Saludo {
    clasificar_para(r, "/usr/local/bin/orbit")
}

/// [`clasificar`], sabiendo en qué ruta se ha llamado a `orbit`: un «command
/// not found» sólo es «no hay orbit» si **nombra a ese binario**.
pub fn clasificar_para(r: &Respuesta, binario: &str) -> Saludo {
    let e = r.stderr.to_lowercase();
    let o = r.stdout.to_lowercase();

    if r.codigo == 0 {
        if let Ok(texto) = r.objeto() {
            if let Ok(v) = serde_json::from_str::<Version>(texto) {
                return por_contrato(v);
            }
        }
    }

    // Antes que «command not found»: un `sudo` que pide contraseña con un aviso
    // del `.bashrc` delante es un problema de privilegios, no de instalación.
    //
    // Y por los dos canales: sin `sudo` en el servidor, Orbit lo dice con un
    // `echo` por **stdout** (orbit:341), no por stderr.
    if e.contains("a terminal is required to read the password")
        || e.contains("askpass")
        || e.contains("sudo: a password is required")
        || e.contains("necesita privilegios de root")
        || e.contains("needs root privileges")
        || o.contains("necesita privilegios de root")
        || o.contains("needs root privileges")
    {
        return Saludo::SinPrivilegios;
    }

    // Lo que dice el propio Orbit va antes que «command not found»: ese aviso
    // puede ser del `.bashrc` (`rbenv: command not found`), y antes ganaba con
    // cualquier rc distinto de 0. Un Orbit anterior a la 1.1 detrás de un
    // `.bashrc` ruidoso salía como «no hay orbit», y uno sin orbit.conf con el
    // motivo equivocado.
    //
    // Las dos líneas que imprime un Orbit copiado y sin instalar. Van por
    // **stdout**, que es lo que hace que un cliente descuidado explote aquí.
    if o.contains("no está instalado") || o.contains("is not installed") {
        return Saludo::NoInstalado {
            motivo:
                "el binario está, pero falta /etc/orbit/orbit.conf: no se ha ejecutado install.sh",
        };
    }
    // Un Orbit anterior a la v1.1: `version` todavía no hablaba JSON, así que
    // rechaza la bandera. Es un servidor perfectamente sano y viejo, no un
    // fallo, y merece un mensaje que lo diga.
    if e.contains("no tiene salida json") || e.contains("has no json output") {
        return Saludo::SinContrato { version: None };
    }
    // Y el «no está» del shell, sólo si habla **de este binario**: la misma
    // regla que el transporte (`nombra_al_binario`). El 127 a secas se queda:
    // es lo que sale de `orbit --json version` cuando no hay orbit, sea cual
    // sea el idioma del shell.
    if r.codigo == 127
        || ((e.contains("command not found") || e.contains("no such file"))
            && crate::transporte::nombra_al_binario(&e, binario))
    {
        return Saludo::NoInstalado {
            motivo: "no hay ningún «orbit» en esa ruta",
        };
    }

    match r.objeto() {
        Ok(texto) => match serde_json::from_str::<Version>(texto) {
            Ok(v) => por_contrato(v),
            Err(_) => Saludo::SinContrato { version: None },
        },
        Err(_) => Saludo::NoSeLlega {
            detalle: if r.stderr.is_empty() {
                r.stdout.clone()
            } else {
                r.stderr.clone()
            },
        },
    }
}

fn por_contrato(v: Version) -> Saludo {
    if v.contract == CONTRATO_CONOCIDO {
        Saludo::Ok(v)
    } else if v.contract > CONTRATO_CONOCIDO {
        Saludo::MasNuevo(v)
    } else {
        // Contrato 0 o ausente: mismo tratamiento que un Orbit pre-1.1.
        Saludo::SinContrato {
            version: Some(v.version),
        }
    }
}

/// Lo mismo, partiendo de un fallo del transporte.
pub fn de_error(e: &ErrorTransporte) -> Saludo {
    match e {
        ErrorTransporte::OrbitNoEsta { .. } => Saludo::NoInstalado {
            motivo: "no hay ningún «orbit» ejecutable en esa ruta",
        },
        ErrorTransporte::SudoPideClave => Saludo::SinPrivilegios,
        ErrorTransporte::ClaveDeHostCambiada { detalle } => Saludo::ClaveDeHostCambiada {
            detalle: detalle.clone(),
        },
        otro => Saludo::NoSeLlega {
            detalle: otro.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(codigo: i32, stdout: &str, stderr: &str) -> Respuesta {
        Respuesta {
            stdout: stdout.into(),
            stderr: stderr.into(),
            codigo,
            cancelado: false,
        }
    }

    #[test]
    fn un_servidor_sano() {
        let s = clasificar(&r(0, r#"{"schema":1,"version":"1.3.6","contract":1}"#, ""));
        assert!(matches!(s, Saludo::Ok(_)));
        assert!(s.permite_operar());
    }

    #[test]
    fn uno_mas_nuevo_no_nos_hace_negarnos() {
        let s = clasificar(&r(0, r#"{"schema":1,"version":"2.1.0","contract":9}"#, ""));
        assert!(matches!(s, Saludo::MasNuevo(_)));
        // Se degrada a lectura, no se apaga: negarse sería la peor forma de
        // romper algo que todavía funcionaba.
        assert!(s.permite_leer());
        assert!(!s.permite_operar());
    }

    #[test]
    fn sin_orbit_en_la_ruta() {
        let s = clasificar(&r(127, "", "bash: orbit: command not found"));
        assert!(matches!(s, Saludo::NoInstalado { .. }));
    }

    #[test]
    fn instalado_a_medias_se_reconoce_por_stdout() {
        // Es el caso que rompe a un cliente descuidado: las dos líneas van por
        // stdout, no por stderr, e ignoran --json. Parsear antes de mirar el
        // código de salida da un error de sintaxis donde la respuesta correcta
        // es «tienes Orbit a medias».
        let s = clasificar(&r(
            1,
            "Orbit no está instalado todavía (falta /etc/orbit/orbit.conf).\nEjecuta primero:  sudo bash install.sh\n",
            "",
        ));
        match s {
            Saludo::NoInstalado { motivo } => assert!(motivo.contains("install.sh")),
            otro => panic!("esperaba NoInstalado, no {otro:?}"),
        }
    }

    #[test]
    fn sudo_pidiendo_contrasena_es_su_propio_caso() {
        // No es «no llegué» ni «no está instalado»: es que ese usuario necesita
        // una contraseña y aquí no hay dónde escribirla. Confundirlo manda a
        // quien lo mire a revisar su red.
        let s = clasificar(&r(
            1,
            "",
            "sudo: a terminal is required to read the password; either use the -S option",
        ));
        assert_eq!(s, Saludo::SinPrivilegios);
    }

    #[test]
    fn un_orbit_anterior_al_contrato() {
        // Un servidor sano y viejo, no un fallo.
        let s = clasificar(&r(1, "", "  ✗ 'orbit version' no tiene salida JSON"));
        assert!(matches!(s, Saludo::SinContrato { .. }));
        assert!(
            !s.permite_leer(),
            "cero funcionalidad, no funcionalidad a medias"
        );
    }

    #[test]
    fn json_sin_contract_es_contrato_cero() {
        let s = clasificar(&r(0, r#"{"schema":1,"version":"1.0.4"}"#, ""));
        assert!(matches!(s, Saludo::SinContrato { .. }));
    }

    #[test]
    fn el_codigo_de_salida_se_mira_antes_que_el_contenido() {
        // Un stdout con prosa y rc distinto de 0 no puede acabar en el parser.
        let s = clasificar(&r(1, "Orbit no está instalado todavía", ""));
        assert!(!matches!(s, Saludo::SinContrato { .. }));
    }

    #[test]
    fn rc_un_aviso_del_bashrc_no_esconde_un_orbit_sano() {
        let s = clasificar(&r(
            0,
            r#"{"schema":1,"version":"1.5.1","contract":1}"#,
            "bash: rbenv: command not found\n",
        ));
        assert!(matches!(s, Saludo::Ok(_)), "{s:?}");
        // Pero sin el objeto, el aviso sigue sin ser un Orbit.
        let s = clasificar(&r(127, "", "bash: orbit: command not found"));
        assert!(matches!(s, Saludo::NoInstalado { .. }));
    }

    #[test]
    fn rc_sin_sudo_el_aviso_llega_por_stdout() {
        for o in [
            "Orbit necesita privilegios de root.\n",
            "Orbit needs root privileges.\n",
        ] {
            assert_eq!(clasificar(&r(1, o, "")), Saludo::SinPrivilegios, "{o}");
        }
        // Y un `sudo` que pide contraseña no se esconde detrás del `.bashrc`.
        let s = clasificar(&r(
            1,
            "",
            "bash: rbenv: command not found\nsudo: a terminal is required to read the password",
        ));
        assert_eq!(s, Saludo::SinPrivilegios);
    }

    #[test]
    fn rc2_el_ruido_del_bashrc_no_tapa_lo_que_dice_orbit() {
        let ruido = "bash: rbenv: command not found\n";
        let viejo = format!("{ruido}  ✗ 'orbit version' no tiene salida JSON");
        assert!(matches!(
            clasificar(&r(1, "", &viejo)),
            Saludo::SinContrato { .. }
        ));
        match clasificar(&r(1, "Orbit no está instalado todavía\n", ruido)) {
            Saludo::NoInstalado { motivo } => assert!(motivo.contains("orbit.conf"), "{motivo}"),
            otro => panic!("{otro:?}"),
        }
        // Un «command not found» que no nombra a orbit no es «no hay orbit».
        assert!(!matches!(
            clasificar(&r(1, "", ruido)),
            Saludo::NoInstalado { .. }
        ));
        // Y el que sí lo nombra, por la ruta que se ha usado.
        let e = "bash: /opt/o/bin/orbit: No such file or directory";
        assert!(matches!(
            clasificar_para(&r(1, "", e), "/opt/o/bin/orbit"),
            Saludo::NoInstalado { .. }
        ));
    }

    #[test]
    fn una_clave_de_host_cambiada_conserva_su_camino() {
        let s = de_error(&ErrorTransporte::ClaveDeHostCambiada {
            detalle: "SHA256:xxx".into(),
        });
        assert!(matches!(s, Saludo::ClaveDeHostCambiada { .. }));
        assert!(!s.permite_leer());
    }
}
