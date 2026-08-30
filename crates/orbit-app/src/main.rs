// En Windows, sin esto la aplicación abre una consola detrás de la ventana.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    orbit_app_lib::ejecutar()
}
