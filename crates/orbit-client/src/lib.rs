//! # orbit-client
//!
//! El núcleo de Orbit Desktop: el transporte SSH, el contrato `--json` y su
//! tipado. Vive aparte de la interfaz a propósito —se prueba con `cargo test`
//! sin levantar una ventana, un cambio de framework no lo toca, y una TUI podría
//! reutilizarlo sin duplicar nada.
//!
//! ## Las reglas que este crate hace cumplir
//!
//! No son estilo; son la frontera del producto, y están en
//! `docs/ARCHITECTURE.md`:
//!
//! 1. **Sólo se invoca `orbit`.** Nunca se escribe en `/etc/nginx`,
//!    `/etc/orbit` ni systemd. El catálogo de órdenes que este crate puede
//!    generar es finito y está en [`comando`], que es lo que hace la regla
//!    auditable leyendo un fichero.
//! 2. **Ninguna orden se construye concatenando cadenas.** Todo pasa por
//!    [`shquote::build`], y sólo [`transporte`] invoca `ssh`.
//! 3. **Lo que no se sabe no se pinta como un valor.** El tipado lo fuerza:
//!    `port` es `Option<u16>`, no `u16`. No hay forma de leer un `null` como
//!    cero sin escribir `unwrap_or(0)`, que es visible en una revisión.
//! 4. **Toda orden lleva el nombre de la app explícito.** Lo exige el tipo: sin
//!    terminal, un `orbit info` sin app no aborta — elige la primera por orden
//!    alfabético y sale con 0.

pub mod comando;
pub mod contrato;
pub mod descubrir;
pub mod flujo;
pub mod instalar;
pub mod registro;
pub mod saludo;
pub mod shquote;
pub mod transporte;

pub use comando::{Comando, ErrorForma};
pub use contrato::{App, Estado, Version};
pub use descubrir::{descubrir, AliasSsh};
pub use flujo::{leer_log, leer_progreso, SucesoDeLog};
pub use saludo::{Saludo, ORDEN_DE_INSTALACION};
pub use shquote::{build, shquote, ErrorEscapado};
pub use transporte::{ErrorTransporte, Respuesta};
