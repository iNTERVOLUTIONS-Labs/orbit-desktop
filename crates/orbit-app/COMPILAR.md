# Compilar el envoltorio

```bash
npm --prefix ui ci          # una vez
npm --prefix ui run build   # SIEMPRE antes de tauri
cd crates/orbit-app && cargo tauri build
```

Y para desarrollar, dos terminales:

```bash
npm --prefix ui run dev
cd crates/orbit-app && cargo tauri dev
```

## Por qué no hay `beforeBuildCommand`

Lo había, y **estaba mal desde el primer día sin que se notara**: nadie había
ejecutado nunca `tauri build`, porque el CI compilaba con `cargo build`, que no
pasa por ahí.

Al arreglarlo apareció el motivo de fondo, que es peor que la errata. Tauri
ejecuta ese comando desde **el directorio donde encuentra un `package.json`**, y
eso lo deduce. En este repositorio el `package.json` está en `ui/` y no en la
raíz, así que la deducción no tiene un ancla clara — y salió distinta en dos
sitios:

| Dónde | Directorio que eligió | Qué pasó con `../../ui` |
|---|---|---|
| Esta máquina | `ui/` | apuntaba fuera del repositorio |
| Un runner limpio de CI | `crates/` | apuntaba a `crates/package.json` |

Un paso implícito cuyo directorio de trabajo cambia según el estado de la
máquina no es una comodidad: es una trampa que sólo salta el día que se
construye un paquete de verdad. Así que se quita y la interfaz se compila a
mano, que es una línea y siempre hace lo mismo.

Las otras dos rutas del fichero **sí** están ancladas y se quedan:
`frontendDist` es relativa a este fichero, y `devUrl` es una URL.
