# Arquitectura del cliente

> Cómo se construye Orbit Desktop: stack, transporte SSH, modelo de datos,
> caché, multiservidor, estructura del repositorio, distribución y pruebas.
>
> Se apoya en **[CONTRACT.md](CONTRACT.md)**, que es la auditoría de lo que el
> servidor ofrece de verdad. Las decisiones de producto que enmarcan todo esto
> están en **[ARCHITECTURE.md](ARCHITECTURE.md)**; el sistema de diseño, en
> **[DESIGN.md](DESIGN.md)**; las reglas de seguridad que este documento tiene
> que satisfacer, en **[THREAT-MODEL.md](THREAT-MODEL.md)**.
>
> **Método.** Las cifras están medidas contra un banco de 40 apps reproducible
> con `tools/bench/`, sobre 8 vCPU. Las que no se han medido se dicen como
> estimaciones. Cuando se cita una línea, es la del fichero `orbit` de la
> v1.3.6.

> **Cómo leer las referencias.** Este documento y [CONTRACT.md](CONTRACT.md) eran uno solo y comparten numeración: allí vive la §1 —la auditoría del contrato— y aquí de la §2 en adelante. Las 26 citas a «§1.x» de este texto apuntan a CONTRACT.md.

---

## 2. Elección de stack

### 2.0 Los criterios, y cuáles de ellos deciden de verdad

Se evalúan seis candidatos contra nueve criterios. Pero antes conviene decir cuáles pesan, porque
una tabla con nueve columnas iguales no es una evaluación: es una lista.

**Los tres que deciden**, por orden:

1. **Madurez del soporte SSH con `~/.ssh/config`, ssh-agent y `ProxyJump`.** Es la regla dura nº 3
   del BRIEF y no es negociable. Si un stack no puede hacer esto bien, no entra, valga lo que valga
   en todo lo demás. Y hay una subdecisión dentro que resulta ser la más importante del documento
   entero: **librería nativa o el binario `ssh` del sistema** (§2.2).
2. **Coste de mantenimiento para un equipo pequeño.** Intervolutions no tiene un equipo de
   escritorio; tiene gente que además hace otras cosas. Un stack que exige mantener tres cadenas de
   compilación distintas y depurar fallos que sólo pasan en un runner de macOS consume el proyecto.
3. **Firma y notarización.** Es el criterio que la gente descubre tarde. Una app de escritorio sin
   firmar en macOS 15 no se abre con doble clic: Gatekeeper la manda a la papelera con un mensaje
   que dice que está dañada. En Windows, SmartScreen la marca en rojo hasta que acumule reputación.
   No es un detalle de empaquetado: es si el producto existe o no.

**Los que casi no deciden**, y conviene decirlo para no fingir rigor: el tamaño del binario y la
RAM. Orbit Desktop es una herramienta que un administrador abre y deja abierta; 60 MB frente a
150 MB de RSS no cambia la vida de nadie en 2026. Pesan un poco por decencia —y porque un binario
de 8 MB se descarga y se prueba, y uno de 120 MB se piensa dos veces— pero no son los que ordenan
la tabla.

### 2.1 La tabla

| | Tauri v2 | Electron | Wails v2 | .NET MAUI / Avalonia | Qt (C++/PySide) | TUI (Rust+ratatui) |
|---|---|---|---|---|---|---|
| **Binario, app vacía** | 3-8 MB | 85-150 MB | 8-12 MB | 30-70 MB (self-contained) | 15-40 MB | 2-5 MB |
| **RSS en reposo** | 60-120 MB | 130-250 MB | 60-120 MB | 80-150 MB | 40-90 MB | 5-15 MB |
| **Arranque en frío** | 0,3-0,8 s | 1,2-2,5 s | 0,3-0,8 s | 0,8-2 s | 0,3-1 s | <0,1 s |
| **Superficie de ataque** | WebView del SO + IPC tipada; sin Node | Chromium + Node completos en el renderer | WebView del SO + Go | CLR; sin WebView si Avalonia | Sin WebView | Ninguna gráfica |
| **Madurez SSH** | `russh` / `ssh2-rs` / delegar en `ssh` | `ssh2` (napi) / `node-ssh` | `golang.org/x/crypto/ssh` | `SSH.NET` | `libssh`/`libssh2` | igual que Tauri |
| **Firma macOS** | Nativa en `tauri build`, notarización con `notarytool` integrada | `electron-builder`, muy rodado | Manual (`codesign` + `notarytool` a mano) | `dotnet` + herramientas de Xcode | Manual | Igual que Tauri |
| **Firma Windows** | `.msi` (WiX) y `.exe` (NSIS) firmables; soporta Azure Trusted Signing | `.msi`/`.nsis`, rodadísimo | Manual | `.msix`/`.msi` nativo | Manual | Igual |
| **Actualizador con verificación de firma** | `tauri-plugin-updater`, firma **minisign** propia, verificada antes de aplicar | `electron-updater`, verifica la firma del instalador | No hay; hay que escribirlo | No hay estándar | No hay | Se distribuye por gestor de paquetes |
| **Coste de mantenimiento** | Medio: dos lenguajes, pero la superficie de Rust es pequeña | **Bajo** si el equipo ya es JS; alto en CVEs de Chromium | Medio-alto: Go+JS, y todo lo de empaquetado a mano | Alto en Linux; MAUI no soporta Linux oficialmente | Alto: C++ o el lío de empaquetar PySide | **Muy bajo** |
| **Riesgo de WebView divergente** | Sí: WebKitGTK / WKWebView / WebView2 | No: Chromium empotrado | Sí, el mismo | No (Avalonia pinta ella) | No | No |

### 2.2 La subdecisión que manda: qué librería SSH, y la trampa de cada una

Aquí es donde se decide de verdad, así que va con detalle. La pregunta no es «¿qué librería SSH?»
sino **«¿quién interpreta `~/.ssh/config`?»**. Y la respuesta honesta es que ninguna librería lo
hace del todo.

**`russh` (Rust, puro).** Implementación moderna, async, mantenida, sin dependencia de OpenSSL.
Es la que elegiría un ingeniero por gusto. **La trampa:** `russh` es el protocolo, no el cliente.
No lee `~/.ssh/config` (hay `ssh_config` como crate aparte, incompleto: `Match` y muchas
directivas no están), no habla con el ssh-agent (hay `russh-keys` con soporte de agente, pero el
`SSH_AUTH_SOCK` de Windows es un named pipe y ahí hay que escribir código propio), y **`ProxyJump`
hay que implementarlo a mano** abriendo una conexión al salto y luego un canal `direct-tcpip` sobre
ella. Cada una de esas tres cosas es entre 200 y 600 líneas y son exactamente las 200-600 líneas
donde un cliente SSH se equivoca. Además: `ProxyCommand` (que mucha gente usa para `cloudflared
access ssh`, para AWS SSM, para `sshuttle`) es imposible de soportar bien sin lanzar un proceso, y
si vas a lanzar un proceso, ya estabas en la otra opción.

**`ssh2-rs` (Rust sobre libssh2).** Envoltorio de una librería C con veinte años. **La trampa:** es
bloqueante, arrastra una dependencia nativa que hay que compilar en las tres plataformas (y en
Windows es donde duele), su soporte de algoritmos modernos ha ido siempre por detrás —durante años
no habló `ssh-ed25519` con certificados, y el soporte de `rsa-sha2-256/512` llegó tarde, que es lo
que rompe contra un `sshd` moderno con `PubkeyAcceptedAlgorithms` restringido— y tampoco lee
`~/.ssh/config` ni hace `ProxyJump`.

**`thrussh`.** Es el antecesor de `russh`; el proyecto está efectivamente archivado y `russh` es su
continuación mantenida. **La trampa es que ya no es una opción**: elegirla en 2026 es elegir código
sin mantenimiento. Se menciona porque aparece en las búsquedas, no porque compita.

**`ssh2` de Node (napi sobre libssh2) y `node-ssh`.** Mismas trampas que `ssh2-rs` más una propia:
binarios precompilados por plataforma y por versión de Node/Electron, con `prebuild-install` y sus
fallos silenciosos. Y `ssh2-streams`/`ssh2` de Node ha tenido su historial de CVEs de parseo.
`node-ssh` es azúcar encima, no resuelve nada de `~/.ssh/config`.

**`golang.org/x/crypto/ssh` (para Wails).** La mejor de las librerías puras en calidad de código y
mantenimiento. **La misma trampa que todas:** es el protocolo. `~/.ssh/config` no. `ProxyJump` se
hace a mano (aunque en Go es notablemente más limpio: `Dial` sobre la conexión del salto). El
ecosistema tiene `kevinburke/ssh_config`, que parsea el fichero pero no aplica `Match`, no expande
`%h`/`%p`/`%r` en todos los sitios y no soporta `Include` recursivo bien.

**`SSH.NET` (para .NET).** Puro C#. **La trampa:** ha sido históricamente el más flojo de los
cinco en algoritmos modernos y en mantenimiento, y su modelo de hilos es de 2012. No se recomienda
para producto.

**Delegar en el binario `ssh` del sistema.** No es una librería, y por eso gana.

**Lo que se gana**, y es exactamente lo que pide el BRIEF:

- `~/.ssh/config` **entero**, con `Match`, `Include`, `%h`, `Host` con comodines y todo lo que el
  usuario ya tenga escrito y funcionando. Sin reimplementar nada.
- **ssh-agent** en las tres plataformas, incluido el named pipe de Windows y el agente de 1Password,
  Secretive, YubiKey/PKCS#11 y las llaves en Secure Enclave. Ninguna librería habla con todos esos.
- **`ProxyJump` y `ProxyCommand`** gratis. Y con ellos, `cloudflared access ssh`, `aws ssm
  start-session`, `tailscale ssh`, y el bastión corporativo del cliente que nadie te va a contar.
- **`ControlMaster`** — multiplexado real (§3.2).
- **La verificación de `known_hosts` la hace OpenSSH**, con su política, sus algoritmos y sus
  actualizaciones de seguridad. Que un cliente de escritorio implemente su propia verificación de
  clave de host es una de las formas más fáciles de escribir un agujero.
- **Cero superficie criptográfica propia.** Un CVE en libssh2 es un CVE en tu producto; un CVE en
  OpenSSH es un `apt upgrade` del usuario.

**Lo que se pierde, y hay que decirlo:**

- **Dependencia de un binario externo.** En macOS y Linux está siempre. En Windows, `ssh.exe` viene
  con Windows 10 1809+ como característica opcional **instalada por defecto** desde 2018
  (`C:\Windows\System32\OpenSSH\ssh.exe`), pero puede faltar en imágenes empresariales recortadas.
  Mitigación: detectarlo al arrancar y, si falta, ofrecer el `winget install
  Microsoft.OpenSSH.Beta` o empaquetar un `ssh.exe` de repuesto en el instalador.
- **Parseo de errores en texto.** `ssh` no tiene salida máquina. Hay que reconocer
  `Permission denied`, `Host key verification failed`, `Connection timed out`, `Could not resolve
  hostname`. Son cadenas estables desde hace veinte años, y además localizadas **nunca** (OpenSSH
  no traduce). Es menos frágil que parsear la tabla de `orbit backup list`.
- **La contraseña.** Si la llave tiene passphrase y no está en el agente, `ssh` la pide por el TTY.
  Sin TTY falla. Solución estándar: `SSH_ASKPASS` + `SSH_ASKPASS_REQUIRE=force`, apuntando a un
  binario auxiliar del propio cliente que abre un diálogo nativo. Es lo que hacen VS Code Remote-SSH
  y Git Credential Manager. Son unas 80 líneas y resuelven también la contraseña de `sudo`.

**Veredicto de la subdecisión:** se delega en el binario `ssh` del sistema. **Es lo que hace VS Code
Remote-SSH, y es lo que hace `git` cuando clona por SSH.** Que las dos herramientas que más SSH
hacen en el escritorio de un desarrollador hayan tomado la misma decisión no es un argumento de
autoridad: es la señal de que reimplementar `~/.ssh/config` es un pozo, y de que los dos lo
comprobaron antes que nosotros.

Y esta subdecisión **desactiva la columna «madurez SSH» de la tabla de §2.1**, porque los seis
candidatos pueden lanzar un proceso. Lo que queda decidiendo son mantenimiento, firma y
distribución.

### 2.3 Recomendación: **Tauri v2 + Svelte 5**

**Se recomienda Tauri v2 con el frontend en TypeScript + Svelte 5, y el transporte SSH implementado
en Rust lanzando el binario `ssh` del sistema con `tokio::process`.**

(En la ronda 1 este documento decía React. La discrepancia con el informe de UX se resuelve en
§2.5, con medidas, y **cede este documento**.)

Los cuatro argumentos del envoltorio, en orden de peso:

**1. Es el único que trae actualizador con verificación de firma de serie.**
`tauri-plugin-updater` publica un `latest.json` firmado con **minisign** (Ed25519), y el cliente
verifica la firma **antes** de aplicar la actualización, con la clave pública compilada dentro del
binario. Comprometer el CDN de descargas no basta para empujar una actualización maliciosa: haría
falta además la clave privada, que vive en el secreto de CI. Para una herramienta que **tiene root
en todos los servidores de sus usuarios**, es exactamente la propiedad que hace falta. En Electron,
`electron-updater` verifica la firma del instalador (Authenticode / notarización), que también
sirve pero depende de que el certificado esté bien en las dos plataformas. En Wails, en MAUI y en
Qt **no hay actualizador**: hay que escribirlo, y escribir un actualizador es escribir el vector de
ataque más goloso del producto.

**2. La superficie de ataque encaja con lo que el propio Orbit predica.**
`ARCHITECTURE §13.3` descarta el panel web porque un panel sobre `orbit` es «una shell de root
expuesta a internet», y cita CVE-2025-48703. Sería incoherente responder a eso con un cliente que
mete Chromium y Node completos en el proceso que sostiene las credenciales SSH del usuario. En
Tauri el renderer es el WebView del sistema y **no tiene Node**: no hay `require`, no hay `fs`, no
hay `child_process`. Todo lo que el frontend puede hacer pasa por comandos declarados
explícitamente en Rust, con la CSP y las capacidades de Tauri v2 encima. Si una dependencia de npm
se compromete —y pasa cada pocos meses—, en Electron esa dependencia puede leer
`~/.ssh/id_ed25519`; en Tauri no puede, porque no hay API para tocar el disco desde el renderer si
no se la das. **Y con Svelte esa superficie es todavía menor**, porque el árbol de dependencias de
`ui/` baja de 33 paquetes a 21 (§7.4).

**3. El coste de mantenimiento es medio, no alto, porque la superficie de Rust es pequeña.**
El backend son cuatro cosas: lanzar procesos, leer dos tuberías, un mapa de conexiones y el almacén
de credenciales. Unas 1.200-1.800 líneas, sin `unsafe`, sin genéricos acrobáticos y sin async más
allá de `tokio::process` y `tokio::sync`. Todo lo demás es TypeScript. El coste real de Tauri no es
Rust: es que hay **tres WebView distintos** (WebKitGTK en Linux, WKWebView en macOS, WebView2 en
Windows) y de vez en cuando algo se ve mal en uno. Se paga con disciplina de CSS y con capturas en
CI en las tres plataformas (§9.4).

**4. Tamaño y arranque, que sí importan un poco.** 5-8 MB de instalador frente a 90-150 MB. Un
administrador que evalúa una herramienta descarga la de 6 MB. Y arranca en menos de un segundo.

### 2.4 Por qué NO cada uno de los otros envoltorios

**Electron.** Es la respuesta correcta para muchos productos y sería defendible aquí. Se descarta
por el punto 2: un runtime con Node completo en el mismo proceso que las credenciales SSH, para una
herramienta cuya tesis es que la superficie de ataque pequeña es la característica. Y por el
mantenimiento real, que no es escribir la app: es seguir a Chromium. Versión mayor cada ocho
semanas y CVEs que hay que atender aunque no uses la parte afectada, porque el escáner del cliente
sí la ve. Para un equipo pequeño es una suscripción de trabajo indefinida.

**Wails v2.** Técnicamente muy cercano a Tauri —WebView del sistema, binario pequeño— y con la
mejor librería SSH pura de las seis, aunque eso ya no importa tras §2.2. Se descarta por
distribución: **no tiene actualizador**, y la firma y la notarización son enteramente manuales. Es
exactamente el trabajo que Tauri ya ha hecho.

**.NET MAUI / Avalonia.** MAUI **no soporta Linux oficialmente**, y la mitad de los usuarios de una
herramienta para administrar servidores Linux trabajan en Linux. Eso lo elimina. Avalonia sí es
multiplataforma de verdad y pinta ella misma (nada de WebView), pero no hay actualizador estándar,
el empaquetado en Linux es artesanal, y el ecosistema de UI está lejos de lo que se consigue en un
día con CSS y unas primitivas accesibles.

**Qt (C++ o PySide).** El mejor rendimiento y el menor consumo de los seis. Se descarta por coste:
C++ para una herramienta de administración se paga cada día; y PySide arrastra el empaquetado de
Python, que es su propio proyecto. Ninguna de las dos tiene actualizador. Y la licencia de Qt es
una conversación que un proyecto MIT no necesita tener.

**Una TUI (Rust + ratatui).** Gana en todas las columnas técnicas y encaja culturalmente con Orbit
como un guante. **Se descarta porque el hueco que llena ya está lleno**: el BRIEF pide una app «con
ratón» (§13.3) y el terminal ya tiene `orbit top` y el menú de `orbit`. Una TUI sería una tercera
interfaz de terminal para el mismo servidor: los «dos productos dentro del primero» contra los que
avisa §1 de ARCHITECTURE. **Pero se recupera una pieza**: la lógica del cliente vive en
`crates/orbit-client`, independiente de Tauri, así que un `orbit-desktop-tui` de fin de semana no
duplicaría nada — y, más importante, esa lógica se prueba con `cargo test` sin levantar ventana.

### 2.5 La discrepancia con el informe de UX, resuelta con medidas

El informe de UX recomienda **Svelte 5** y **CSS puro con tokens**; la ronda 1 de este documento
recomendaba **React** y **Tailwind**. Es la única contradicción entre los tres documentos y no
puede quedar abierta, así que se resuelve aquí, y se resuelve **midiendo**, no repartiendo.

**El primer argumento de UX no sobrevive a la medición, y hay que decirlo.** UX escribe: «una
pantalla —el monitor— que reconcilia cuarenta filas cada dos segundos». Esa premisa es falsa, y lo
demuestra el banco de 40 apps: **`orbit top --json` tarda 2.116 ms** (§5.0), o sea que el monitor
no puede refrescarse cada dos segundos ni queriendo. Con el intervalo adaptativo de §5.4, el
repintado real ocurre **cada 3,2 s**. Y lo que se repinta son 40 filas × 8 celdas = 320 nodos.

Reconciliar 320 nodos en React 19 cuesta del orden de **1-3 ms**; en Svelte 5, con actualización
granular, del orden de **0,3 ms**. La diferencia es de **~2 ms una vez cada 3,2 segundos**: un
0,06 % de un núcleo. **El argumento de rendimiento del monitor, que UX pone el primero, vale dos
milisegundos.** Y es importante dejarlo escrito: *una decisión defendida con un número equivocado
es una decisión que se reabre*, y ésta no debe reabrirse por el motivo malo.

**El segundo argumento de UX sí sobrevive, y además yo lo subestimé.** El arranque en frío. Con las
dependencias que proponía la ronda 1 —React + react-dom (45 KB gz) + TanStack Query (13) + router
(2) + cinco primitivas de Radix (~45) + Recharts (~100)— el paquete de partida rondaba **200 KB
comprimidos**, o sea ~700 KB de JavaScript que hay que analizar y ejecutar **antes de pintar nada**.
En un portátil moderno eso son 40-70 ms; en el WebKitGTK de un portátil de hace cinco años, 120-180.
Con Svelte 5 + `@tanstack/svelte-query` + Melt UI + SVG propio, el equivalente son ~45 KB
comprimidos: **entre 30 y 130 ms menos de ventana en blanco, cada vez que se abre la app**. Contra
un suelo de contrato de 76 ms y una portada de 388 ms, eso no es ruido: es del mismo orden que la
llamada más barata del contrato.

**Y el argumento que yo esgrimía —el ecosistema— se disuelve al mirarlo.** Todas las dependencias
que quería tienen equivalente de primera parte en Svelte, del mismo mantenedor:

| Ronda 1 (React) | Svelte 5 | Mismo mantenedor |
|---|---|---|
| `@tanstack/react-query` | `@tanstack/svelte-query` | sí |
| `@tanstack/react-virtual` | `@tanstack/svelte-virtual` | sí |
| Radix primitives | Melt UI / Bits UI | no, pero equivalentes y accesibles |
| `recharts` | SVG propio + `d3-scale`/`d3-shape` | UX ya lo pedía, y con razón (§8.4 de UX) |
| `zustand` | runas `$state` | no hace falta librería |
| `wouter` | `svelte-routing` o rutas propias | trivial |

O sea: **el coste de ceder es cero en capacidad y negativo en peso**. No hay nada que la ronda 1
quisiera hacer que Svelte no haga.

**Un tercer argumento, que ninguno de los dos puso y que es mío:** de los tres WebView, el más
viejo y el que más problemas da es **WebKitGTK**. Svelte emite DOM y CSS más cercanos a lo llano;
React 19 usa características más recientes del navegador para su hidratación y su planificador.
No es un argumento decisivo —React funciona en WebKitGTK— pero apunta en la misma dirección y no
en la contraria.

**Lo que se pierde al ceder, dicho para que conste:** el mercado de gente que sabe React es mucho
mayor que el de Svelte, y eso es un riesgo real de mantenimiento a cinco años para un equipo
pequeño. Es el único argumento a favor de React que sigue en pie, y **no es suficiente**: Svelte 5
lleva desde 2024 en estable, está en el top-5 de frameworks por uso, y —lo que de verdad decide—
**la parte difícil de este producto no es la interfaz: es el contrato, el transporte y los `null`**,
y todo eso está en Rust, donde el framework no llega. Si dentro de tres años hubiera que cambiar de
framework, cambiaría `ui/` y **`crates/` no se enteraría**. Ése es el argumento que hace barata la
decisión, y es el que la cierra.

**Decisión: Svelte 5.** Y **CSS puro con tokens, sin Tailwind**, por el argumento de UX que este
documento no tenía y que es específico de este producto: `class="app-row is-unserved"` dice algo
cuando alguien inspecciona por qué una fila está en rojo; una ristra de utilidades no. En un
producto cuyo valor entero es **no perder los matices del contrato** —`served:false` no es
«parada», `null` no es cero—, disolver el estado semántico en clases de presentación va en la
dirección contraria. Lo que se pierde con Tailwind (velocidad en pantallas irregulares) no aplica
aquí: son doce pantallas muy regulares y unos sesenta tokens.

**Lo que NO cede este documento**, porque UX no lo discute y las medidas lo sostienen: el envoltorio
es Tauri v2 (§2.3), el transporte delega en el `ssh` del sistema (§2.2), y la lógica del cliente
vive en Rust y no en el frontend (§7.0).

### 2.6 Resumen de la elección

```
Tauri v2  +  TypeScript/Svelte 5  +  CSS con tokens  +  transporte por el `ssh` del sistema
```

- **Rust** (`crates/orbit-client`, `crates/orbit-app`): transporte, sesión, tipos del contrato,
  tareas, caché de salud. Se prueba sin ventana.
- **TS/Svelte 5** (`ui/`): toda la interfaz. Sin acceso al sistema de ficheros ni a procesos.
- **SSH**: `ssh -o ControlMaster=auto`, nunca una librería. §3.
- **Actualización**: `tauri-plugin-updater` con minisign. §8.

## 3. La capa de transporte SSH: `OrbitClient`

### 3.1 El modelo mental

Un `OrbitClient` es **un servidor**, no una conexión. Vive mientras el usuario tenga ese servidor
dado de alta, sabe si está sano o caído, y por debajo mantiene (o no) un socket de control
compartido. Encima de él, cada llamada al contrato es una función tipada.

```rust
pub struct OrbitClient {
    id: ServerId,
    target: SshTarget,          // el alias de ~/.ssh/config, o host+user+port
    control: ControlSocket,     // ruta del socket de multiplexado y su estado
    health: watch::Sender<Health>,
    gate: Arc<Semaphore>,       // como mucho N comandos a la vez en este servidor
    contract: OnceCell<ContractInfo>,
}

pub enum Health {
    Unknown,
    Probing,
    Ready { contract: u32, version: String, latency_ms: u32 },
    Degraded { reason: DegradedReason },   // llega, pero no puede ejecutar orbit
    Unreachable { error: SshError, since: Instant, retry_at: Instant },
}
```

`Health` es un `watch::Sender` y no un campo porque la interfaz se suscribe: cuando un servidor cae,
todas las pantallas que lo miran se enteran sin hacer polling.

### 3.2 Cómo se abre una conexión, y por qué se reutiliza con `ControlMaster`

**La decisión:** se usa el binario `ssh` del sistema con multiplexado explícito. No una librería
(§2.2), y no una invocación suelta por comando.

**Ronda 2: el ahorro está medido, no estimado.** Contra un `sshd` local (o sea, RTT ≈ 0: sólo el
coste de criptografía y autenticación, sin red), con clave Ed25519 y `BatchMode`:

| | sin multiplexado | con `ControlMaster` | ahorro |
|---|---|---|---|
| `ssh … true` (canal vacío) | **246 ms** (min 153 / max 342) | **13 ms** (9 / 30) | **233 ms** |
| `ssh … orbit version --json` | **300 ms** | **77 ms** | 223 ms |
| `ssh … orbit list --json` (40 apps) | **532 ms** | **313 ms** | 219 ms |

Las tres filas dan el mismo número, que es la señal de que se está midiendo lo que se cree:
**el apretón de manos cuesta ~230 ms, y es un coste fijo por llamada.** Y esto es **en localhost**:
el saludo de SSH son cuatro o cinco idas y vueltas, así que contra un VPS europeo a 30 ms de RTT
hay que sumarle ~120-150 ms más, y contra uno en Singapur desde Madrid, medio segundo.

Puesto al lado de las cifras del contrato (§5.0): sin multiplexado, **la llamada más barata del
contrato —`version --json`, 76 ms de trabajo real— costaría 300 ms**, o sea que **el 75 % del
tiempo se iría en volver a presentarse a un servidor con el que acabas de hablar**. Con
`ControlMaster`, 13 ms de canal sobre 76 de trabajo.

La invocación:

```
ssh -o ControlMaster=auto
    -o ControlPath=<dir_0700>/o-%C            # ver §3.11: el socket es una credencial viva
    -o ControlPersist=45                      # ver §3.11: es la ventana de exposición
    -o BatchMode=yes
    -o ConnectTimeout=10
    -o ServerAliveInterval=15
    -o ServerAliveCountMax=3
    -o ForwardAgent=no                        # SEC-10, explícito y sin interruptor
    -o ForwardX11=no
    -o ClearAllForwardings=yes                # nada de -L/-R/-D heredados de ~/.ssh/config
    -o RequestTTY=no
    -o StrictHostKeyChecking=accept-new       # sólo tras el alta; en el alta, =yes
    -T
    <alias>
    -- /usr/local/bin/orbit --lang es --json list      # SEC-03: ruta absoluta, nunca por PATH
```

Tres de esas opciones son de la ronda 2 y vienen de las reglas duras del informe de QA:

- **`ForwardAgent=no` y `ForwardX11=no`, explícitos.** SEC-10. No basta con «no lo activamos»:
  `~/.ssh/config` del usuario puede tener `Host * / ForwardAgent yes`, y entonces **el agente del
  usuario viajaría al servidor**, donde root puede usarlo para autenticarse en cualquier otro sitio
  al que el usuario tenga acceso. Se apaga en la línea de órdenes, que gana sobre el fichero.
- **`ClearAllForwardings=yes`.** Por lo mismo, para los reenvíos de puertos: un `LocalForward` en
  un `Host *` haría que Orbit Desktop abriera puertos que nadie ha pedido, y que fallara el comando
  si el puerto está ocupado.
- **`/usr/local/bin/orbit`, ruta absoluta y configurable por servidor.** SEC-03. Un `PATH`
  manipulado en el `.bashrc` del usuario remoto —o por un atacante con escritura sólo en su
  `$HOME`— redirigiría todos los comandos a otro binario, y ese binario recibiría la elevación a
  root de la línea 241. La ruta se comprueba en el alta (`command -v orbit` **sólo para sugerirla**)
  y a partir de ahí se fija.

Justificación opción por opción, porque cada una tapa un modo de fallo concreto:

- **`ControlPath=<dir_0700>/o-%C`**. `%C` es el hash de (host, puerto, usuario, proxy): la ruta no
  revela el nombre del servidor y **nunca supera los 104 bytes del límite de `sun_path`**, que es
  el error clásico de quien usa `%h_%p_%r` con un hostname largo (`o-` más 64 hex = 66 bytes, deja
  38 para el directorio). El directorio **lo crea el cliente con 0700 y lo verifica antes de cada
  uso** —dueño y modo— por lo que se explica en §3.11; nunca `/tmp` a pelo. Linux:
  `$XDG_RUNTIME_DIR/orbit-desktop/`. macOS: `$TMPDIR` (que ya es privado por usuario y corto).
  Windows: **no hay**, OpenSSH para Windows no implementa `ControlMaster`. Ver §3.7.
- **`ControlPersist=45`**. Bajado de los 120 s de la ronda 1 tras medir qué es exactamente ese
  socket (§3.11: **una conexión root autenticada que cualquier proceso del mismo usuario puede
  reutilizar sin credencial**, verificado). 45 s cubre de sobra una ráfaga de pantallas —la portada
  y el detalle de una app suman 500 ms— y reduce la ventana de exposición a menos de la mitad. No se
  usa `ControlPersist=yes` (indefinido) bajo ningún concepto.
- **`BatchMode=yes`**. Nunca pregunta nada por el terminal: ni contraseña, ni confirmación de clave
  de host. Falla rápido y con un mensaje reconocible en vez de colgarse. **Es la opción que
  convierte «se ha quedado pillado» en «error 255 con este texto»**, y por eso está.
- **`ConnectTimeout=10`** y **`ServerAliveInterval=15` / `CountMax=3`**. La primera acota el
  establecimiento; las otras dos detectan una conexión muerta (portátil que cambia de wifi, VPN que
  cae) en ~45 s en vez de en los ~15 minutos del keepalive de TCP. Es lo que hace que un despliegue
  de tres minutos sobre una red inestable falle con un error en vez de quedarse eternamente.
- **`-T`**. Nunca se pide TTY. Es lo que garantiza que `orbit` vea `[[ -t 1 ]]` falso y no pinte
  colores, animaciones ni fotogramas. La única excepción es la terminal embebida de `exec` (§3.6).
- **`--`**. Fin de opciones antes de la orden.

**El alta de un servidor** es el único momento donde la política es distinta: se ejecuta
`ssh -o BatchMode=yes -o StrictHostKeyChecking=yes <alias> true` y, si falla con
`Host key verification failed`, se lanza `ssh-keyscan` **en un proceso separado**, se le enseña al
usuario la huella (SHA256, formato de OpenSSH) y sólo si la acepta se escribe en `~/.ssh/known_hosts`
con la API de OpenSSH (`ssh-keygen -F` para comprobar, y añadir la línea). Nunca
`StrictHostKeyChecking=no`: eso es aceptar cualquier clave para siempre, y para una herramienta que
tiene root al otro lado es indefendible. `accept-new` sólo se usa después de que el usuario haya
aceptado el modelo, y se hace visible en la interfaz.

**El passphrase.** Si `ssh` falla con `Permission denied (publickey)` y el agente no tiene la clave,
el cliente ofrece cargarla: lanza `ssh-add` con `SSH_ASKPASS` apuntando a su propio binario auxiliar
(`orbit-askpass`) y `SSH_ASKPASS_REQUIRE=force`, que abre un diálogo nativo. La passphrase **no la
guarda el cliente**: la recibe el agente del sistema, que es donde tiene que estar. El único secreto
que el cliente guarda es el que no tiene otro sitio: la contraseña de `sudo`, si el usuario elige
guardarla, y va al llavero del SO (`keyring` de Rust → Keychain / Secret Service / Credential
Manager), nunca a un fichero propio.

### 3.3 Cómo se ejecuta un comando, y la separación de stdout y stderr

Es el corazón del cliente, así que va con la firma entera.

```rust
pub struct Invocation {
    pub argv: Vec<String>,          // ["--lang","es","--json","list"]
    pub stdin: StdinPolicy,         // Closed | Bytes(Vec<u8>) | Interactive
    pub timeout: Duration,
    pub progress: Option<mpsc::Sender<ProgressEvent>>,   // parsea stderr como NDJSON
    pub cancel: CancellationToken,
}

pub struct Completed {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration: Duration,
}

impl OrbitClient {
    pub async fn run(&self, inv: Invocation) -> Result<Completed, TransportError>;

    /// La capa de arriba: ejecuta, comprueba el código, y deserializa stdout.
    pub async fn json<T: DeserializeOwned>(&self, argv: &[&str]) -> Result<T, OrbitError>;
}
```

**La separación de canales es el punto crítico y hay que ser explícito sobre por qué.**

`orbit --json` garantiza que **stdout lleva un único objeto JSON y nada más** (§1.1, `_ui_route`).
Todo lo demás —prosa, spinner, avisos, el NDJSON de `--progress`— va por stderr. Si el cliente
combinara los dos canales, cada respuesta sería JSON con basura delante. Por eso:

1. **`stdout` y `stderr` son tuberías separadas** (`Stdio::piped()` en las dos), nunca `2>&1`.
2. **`stdout` se acumula entero** en un `Vec<u8>` acotado (16 MB por defecto; `status --json` con
   200 apps ronda los 300 KB, así que hay tres órdenes de magnitud de margen). No se procesa por
   líneas: es un documento, no un flujo.
3. **`stderr` se lee por líneas, en paralelo, según llega.** Cada línea se clasifica:
   - Empieza por `{`, parsea como JSON y tiene `event` → es un `ProgressEvent`. Se emite.
   - Cualquier otra cosa → es prosa para humanos. Se guarda en un anillo acotado (últimas 200
     líneas) para el mensaje de error y para el panel de «detalle técnico».
4. **Las dos tuberías se leen concurrentemente**, con `tokio::join!`. No secuencialmente. Si se
   leyera stdout hasta el final y sólo después stderr, un `deploy` que escribe mucho por stderr
   llenaría el búfer de la tubería (64 KB en Linux) y **se bloquearía para siempre**. Es un
   *deadlock* clásico y es el fallo nº 1 de quien implementa esto la primera vez.

```rust
let (out, err, status) = tokio::join!(
    read_to_end_capped(child.stdout.take().unwrap(), MAX_STDOUT),
    read_stderr_lines(child.stderr.take().unwrap(), progress_tx, ring.clone()),
    child.wait(),
);
```

**El orden de comprobación en `json::<T>()`**, que es donde se evitan los errores incomprensibles:

```
1. ¿El proceso `ssh` salió con 255?            → TransportError (no llegué). §3.4
2. ¿El código es 127?                          → OrbitError::NotInstalled
3. ¿stdout está vacío?                         → OrbitError::Command { code, stderr }
4. ¿stdout parsea como JSON?                   → si no: OrbitError::NotJson { head, stderr }
5. ¿tiene `schema` y vale lo que esperamos?    → si no: OrbitError::UnknownSchema
6. ¿el código de salida es 0?                  → si no, pero hay JSON válido: es un fallo
                                                  CON datos (deploy fallido). Se devuelven
                                                  los datos Y el código.
```

El paso 6 es el que la gente se salta y es el que hace falta: `orbit deploy --json` de un despliegue
fallido devuelve **1 y un objeto completo**. Tratar el código de salida como «no hay datos» tiraría
justo la información que el usuario necesita ver.

El paso 4 tampoco es teórico: es lo que pasa contra un servidor con `orbit` copiado pero sin
instalar, que escribe dos líneas de texto en español **por stdout** (§1.9).

### 3.4 `TransportError` frente a `OrbitError`, y por qué son dos tipos

Ésta es una decisión de modelado que ahorra media docena de bugs de interfaz.

- **`TransportError`** = no llegué a ejecutar `orbit`. `ssh` salió con 255, o el binario `ssh` no
  existe, o venció el `ConnectTimeout`. **El servidor se marca `Unreachable` y se aplica reintento
  con retroceso.**
- **`OrbitError`** = llegué, `orbit` contestó, y contestó que no. **El servidor sigue sano.** Se
  enseña el error en la pantalla que lo pidió, y nada más.

Confundirlos produce el peor bug de interfaz posible: marcar un servidor entero como caído porque
el usuario pidió `orbit info app-que-no-existe`.

`ssh` sale con 255 para todos sus errores y con el código del comando remoto en el resto. Pero
`orbit` también puede salir con 255 (no lo hace nunca hoy, pero podría), así que 255 solo no
distingue. La desambiguación es por stderr, y las cadenas de OpenSSH son estables y nunca están
traducidas:

```
ssh: connect to host X port 22: Connection refused / timed out
ssh: Could not resolve hostname X
Permission denied (publickey,...)
Host key verification failed.
kex_exchange_identification: ...
Bad configuration option: ...
```

Si stderr contiene una de éstas **y** stdout está vacío **y** el código es 255 → transporte.
En cualquier otro caso, 255 se trata como código de la aplicación. Es heurística, y por eso el
`TransportError` guarda siempre stderr entero para que el usuario pueda mirarlo.

### 3.5 Comandos largos, streaming, y no bloquear la interfaz

**Nada bloquea la interfaz nunca, porque el frontend no espera respuestas: se suscribe a eventos.**

Un despliegue de tres minutos se modela como una **tarea**, no como una llamada:

```rust
pub struct Task {
    pub id: TaskId,
    pub server: ServerId,
    pub kind: TaskKind,          // Deploy { app } | DeployAll | Ssl { app } | New { .. } | ...
    pub started: DateTime<Utc>,
    pub state: TaskState,        // Queued | Running { step, elapsed } | Done(Outcome) | Cancelled
}
```

El flujo:

1. La interfaz llama al comando de Tauri `task_start(server, kind)` y **recibe un `TaskId` de
   inmediato**. La pantalla no espera nada.
2. En Rust, la tarea corre en un `tokio::spawn`. Cada línea de progreso de stderr se convierte en un
   `ProgressEvent` y se emite al frontend con `app.emit("task:progress", …)`.
3. Al terminar, se emite `task:done` con el objeto completo y el código.
4. **Las tareas viven en un registro global**, no en la pantalla. Si el usuario navega a otra
   sección o cierra la ventana de detalle, el despliegue sigue. Al volver, se re-renderiza desde el
   registro. Esto es lo que separa una herramienta de un juguete: un deploy no se cancela porque
   cambies de pestaña.
5. Si la app se cierra, el proceso `ssh` muere y el despliegue **se queda a medias en el servidor**.
   Eso hay que decírselo al usuario: un diálogo «hay 1 despliegue en marcha; si sales ahora se
   interrumpirá y la app puede quedar en un estado intermedio». Y hay que decir la verdad completa:
   `orbit` tiene un `trap EXIT` (`_deploy_on_exit`, 5002) que quita el testigo de mantenimiento y
   limpia el puente del reinicio sin corte — pero ese trap **no se ejecuta si el proceso muere por
   SIGKILL**, y matar la sesión SSH manda SIGHUP al proceso remoto, que sí lo dispara. En la
   práctica, cerrar la app deja el servidor razonablemente limpio. En la interfaz se dice «se
   interrumpirá», no «puede corromperse», porque lo segundo sería alarmismo.

**El mapa de progreso a barra.** `deploy` tiene seis pasos conocidos (§1.5), así que la barra es
determinista y no una animación indefinida:

```ts
const DEPLOY_STEPS = ['code','release','build','activate','service','nginx'] as const;
// El paso N que ha emitido 'start' y no 'ok' es el actual.
// Si el objeto final trae failed_step, ése es el que se pinta en rojo.
```

Y los pesos no son uniformes: `build` es el 70 % del tiempo en un proyecto real. Se puede hacer
mejor: **`orbit metrics <app> --json` da `build_median_s`**, así que la barra puede estimar de
verdad, «unos 84 s» en vez de «trabajando…». Es la mejor pieza de interfaz que el contrato regala y
casi nadie la vería si no se lee `_metrics_app_json`.

**Cancelación.** `CancellationToken` → se manda `SIGTERM` al proceso `ssh` local. Eso cierra el
canal, el `sshd` remoto manda `SIGHUP` a `orbit`, y `orbit` ejecuta su trap. **Pero la interfaz no
puede prometer que el despliegue se ha cancelado**, porque entre el `SIGHUP` y el momento en que
`orbit` lo atiende puede haber terminado un paso. El botón se llama «Interrumpir», y lo que se
enseña después es un refresco de `info --json`, no una afirmación. Es la misma disciplina que
`orbit` aplica en `_new_undeployed`: mirar el disco en vez de creerse lo que dijeron los comandos.

**Timeouts, por clase de comando y con las cifras de §5.0 detrás.** En la ronda 1 eran números
redondos elegidos a ojo; ahora cada uno es **la mediana medida con 40 apps × un factor**, y el
factor dice qué se está tolerando.

| Clase | Medido (40 apps) | Timeout | Factor | Qué tolera |
|---|---|---|---|---|
| `version` | 76 ms | 15 s | ×200 | Una red mala, no un servidor roto |
| `info`, `env list` | 85-90 ms | 15 s | ×170 | Idem |
| `list`, `status`, `redirect list` | 314-388 ms | 20 s | ×50 | Un servidor con 200 apps (proyectado 1,3 s) y aun así red mala |
| `metrics` | 936 ms | 30 s | ×32 | Un histórico de 2.000 líneas y disco lento |
| `doctor` | 1,42 s | **120 s** | ×85 | **40 `dig` contra un DNS que no contesta.** El primero de los tres intentos tardó 4,1 s con NXDOMAIN rápido; con un resolutor que agota su timeout son 5 s × 40 = 200 s en el peor caso, así que 120 s es un compromiso y el cliente lo dice al cancelar |
| `traffic` | 2,0 s **con logs vacíos** | **300 s** | — | Descomprime hasta 15 ficheros rotados. El coste **no** depende de las apps sino de los bytes de log, y no se puede acotar por medida: 14 días de una web con tráfico son cientos de MB |
| `top` | 2,1 s | 30 s | ×14 | Y el muestreo adaptativo de §5.4 lo protege por debajo |
| Mutación corta (`restart`, `port`, `maintenance`, `env set`, `rollback`) | — | 60 s | — | `systemctl restart` + `health_wait` (hasta 30 s en `cmd_port`, línea 10927) |
| `deploy`, `deploy --all`, `new`, `ssl`, `restore`, `backup` | — | **sin total** | — | Sólo *watchdog* de inactividad |

**El *watchdog* de inactividad para lo largo, y por qué es la forma correcta.** No acota cuánto
puede tardar: acota **cuánto puede estar callado**. 10 minutos sin un solo byte por ninguna de las
dos tuberías. Un build de un monorepo grande pasa de 10 minutos con facilidad, así que un timeout
total mataría despliegues buenos; pero `--progress` emite en cada paso (§1.5), y el paso más largo
—`build`— arranca con su `{"event":"step","step":"build","status":"start"}` y luego puede callar
minutos, de ahí que sean 10 y no 2.

**Y una salvaguarda que no es un timeout:** `doctor` y `traffic` son los dos únicos comandos de
lectura que pueden tardar más de 3 s, y los dos son **cancelables desde la interfaz** con el botón
que pide P-19. El resto no necesita botón porque no da tiempo a pulsarlo.

### 3.6 Los tres modos de canal

No todo pasa por el mismo transporte, y forzarlo sería o inseguro o inútil.

**Modo A · petición-respuesta** (el 95 %). Lo descrito en §3.3. `-T`, stdin cerrado, stdout
acumulado. Todas las lecturas del contrato y todas las mutaciones cortas.

**Modo B · flujo largo con progreso.** Igual, pero con `progress` conectado y sin timeout total.
`deploy`, `deploy --all`, `new`, `ssl`, `restore`. **Y `logs --follow`**, que es el único donde lo
que interesa es stdout como flujo y no como documento: ahí `stdout` se lee por líneas y se emite al
frontend con `logs:line`, con un anillo de las últimas 5.000 líneas en Rust para poder re-renderizar
al volver a la pantalla. Se cierra cerrando el proceso; no hay otra forma (§1.11).

**Modo C · terminal interactiva** (`orbit exec <app>`). Aquí **sí** se pide `-t`, porque el usuario
quiere una shell de verdad: `ssh -tt <alias> -- orbit exec <app>`. Se conecta a un emulador de
terminal en el frontend (`xterm.js`), con el tamaño propagado por `SIGWINCH`. Es el único sitio de
toda la app donde hay un TTY y donde stdin está abierto, y va marcado en la interfaz con un aviso
de que eso es una shell con los permisos del usuario de la app. **No se usa para nada más.** §1.12.

### 3.7 Reconexión, salud, y el caso de Windows

**Salud.** Cada servidor tiene un `Health`. La sonda es `orbit --json version`, que es la llamada
más barata del contrato y además da el número de contrato. Se lanza:

- al dar de alta el servidor,
- al seleccionarlo en la interfaz,
- después de cualquier `TransportError`,
- y **nunca en un temporizador de fondo**. Un cliente que hace ping a 40 servidores cada 30 segundos
  es un cliente que abre 40 conexiones cada 30 segundos. Los servidores que no se están mirando no
  se sondean. §6.

**Reconexión con retroceso.** Tras un `TransportError`: 1 s, 2 s, 5 s, 15 s, 60 s, y a partir de ahí
cada 5 minutos, con *jitter* de ±20 %. El retroceso se **reinicia** cuando el usuario actúa
(selecciona el servidor, pulsa «reintentar») porque el usuario sabe algo que el cliente no: que
acaba de conectar la VPN.

**El socket de control muerto.** Si el máster muere (suspensión, cambio de red), la siguiente
llamada falla rápido. `ssh` lo detecta y reintenta sin máster automáticamente en algunos casos, pero
no siempre. El cliente lo hace explícito: ante un `TransportError` en la primera llamada tras un
periodo de inactividad, ejecuta `ssh -O exit <alias>` para limpiar el socket huérfano y reintenta
**una vez**, sin retroceso. Si vuelve a fallar, ya es el servidor.

**Windows: cuánto duele exactamente, medido.** OpenSSH para Windows **no implementa
`ControlMaster`**: los sockets Unix del multiplexado no están portados, y no es un bug que vaya a
arreglarse pronto. En la ronda 1 esto era «−3 puntos, hay que medirlo». Ya está medido, y el
cálculo se puede hacer entero.

El apretón de manos cuesta **246 ms en localhost** (§3.2) y son 4-5 idas y vueltas, así que contra
un VPS a 30 ms de RTT son **~370 ms**. Sumado a las cifras del contrato:

| Pantalla | Llamada | Linux/macOS (mux) | Windows (sin mux) | Penalización |
|---|---|---|---|---|
| Sondeo de salud | `version --json` | 76 + 13 = **89 ms** | 76 + 370 = **446 ms** | ×5,0 |
| Portada de servidor | `status --json` | 388 + 13 = **401 ms** | 388 + 370 = **758 ms** | ×1,9 |
| Detalle de app | `info --json` | 85 + 13 = **98 ms** | 85 + 370 = **455 ms** | ×4,6 |
| Monitor, por muestra | `top --json` | **2,13 s** | **2,49 s** | ×1,17 |
| Diagnóstico | `doctor --json` | **1,43 s** | **1,79 s** | ×1,25 |

**La lectura es que duele donde menos se esperaba.** La penalización **relativa** es peor en las
llamadas **baratas**: la portada sólo empeora un 90 %, pero el detalle de una app se multiplica por
4,6 y pasa de «instantáneo» (98 ms, por debajo del umbral de 150 ms de §5.0) a «hay que enseñar un
esqueleto» (455 ms). **En Windows, la regla de §5.0 cambia de lado para `info`, `version` y
`env list`**, y eso es una diferencia de sensación, no de segundos.

**La decisión: se acepta y se compensa, no se mitiga con trucos.** Tres cosas:

1. **`TTL_SCALE = 2` en Windows** (§5.3). La caché tapa la segunda visita, que es la mayoría.
2. **El umbral de esqueleto se lee de la latencia observada, no de una constante.** El cliente
   guarda la mediana móvil de las últimas 20 llamadas por servidor; si supera 150 ms, esa consulta
   pinta esqueleto. Así el ajuste es automático y también sirve para un VPS lejano en Linux, que
   tiene el mismo problema por otra causa.
3. **En Windows se prefiere `status` sobre `list` con más motivo todavía** (§5.2): el ahorro deja
   de ser 314 ms y pasa a ser 314 + 370 = **684 ms**, o sea que la jugada de §5.2 vale casi el
   doble ahí.

**Lo que se descarta, con su motivo, para que nadie lo reabra en la fase 4:** un **intérprete
persistente** — abrir una sola sesión `ssh -tt <alias> -- bash` y escribirle órdenes leyendo hasta
un delimitador único. Es factible, algunos clientes lo hacen, y resolvería la latencia. Se descarta
porque **un TTY mezcla stdout y stderr en el mismo flujo**, y toda la separación de canales de la
que depende el contrato (§1.1, §3.3) desaparece: el objeto JSON llegaría con el progreso y la prosa
intercalados. Se podría rodear redirigiendo stderr a un fichero temporal en el servidor y leyéndolo
después — pero eso **escribe en el servidor**, que es la regla dura nº 1, y además deja ficheros.
**Un truco que rompe dos invariantes para ahorrar 370 ms no es un truco: es un producto distinto.**

Si algún día la penalización de Windows se vuelve intolerable, la salida honesta es otra: pedir
`ssh` con `ControlMaster` funcionando —Microsoft tiene el issue abierto— o empaquetar un `ssh.exe`
que sí lo soporte. Las dos son de fuera, y las dos son mejores que romper el contrato.

### 3.8 Presupuesto de tamaño y de tiempo, con las cifras reales (SEC-21)

El informe de QA pide un presupuesto duro y propone 8 MB. Con las respuestas medidas, 8 MB es
**400 veces la mayor**, o sea que es un límite que no protege de nada útil: un servidor comprometido
que mande 7 MB de basura pasaría el filtro. Con las cifras de §5.0 se puede ser mucho más estrecho,
y por tanto mucho más útil.

| Respuesta | Medida (40 apps) | Proyección a 200 apps | **Presupuesto** |
|---|---|---|---|
| `version --json` | 44 B | 44 B | **4 KB** |
| `info --json` | 944 B | 944 B (una app) | **256 KB** (`releases[]` puede ser largo) |
| `list --json` | 12,4 KB | ~62 KB | **2 MB** |
| `status --json` | 12,9 KB | ~63 KB | **2 MB** |
| `doctor --json` | 19,8 KB · 109 checks | ~95 KB | **4 MB** |
| `top --json` | 7,3 KB | ~36 KB | **2 MB** |
| `metrics --json` | 3,8 KB | ~19 KB | **2 MB** |
| `traffic --json` | 10,0 KB | ~50 KB | **8 MB** (`paths[]` con `--top 200`) |
| `deploy --json` | ~600 B | ~600 B | **256 KB** |
| `deploy --all --json` | — | ~40 KB | **8 MB** |
| **stderr de cualquier comando** | — | — | **1 MB** en anillo, sin límite de lectura |
| `logs --follow` (stdout) | flujo | — | anillo de **5.000 líneas / 8 MB** |

Los presupuestos son **entre 30 y 150 veces** la proyección a 200 apps: dejan margen para el
crecimiento del contrato (los campos se añaden) sin dejar margen para un flujo malicioso. Se
implementan como un lector acotado que **corta y devuelve error**, no que trunque:

```rust
async fn read_to_end_capped<R: AsyncRead + Unpin>(mut r: R, cap: usize)
    -> Result<Vec<u8>, TransportError>
{
    let mut buf = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = r.read(&mut chunk).await?;
        if n == 0 { return Ok(buf); }
        if buf.len() + n > cap {
            // Truncar sería peor que fallar: un JSON truncado puede parsear
            // (P-01) y pintaría un subconjunto que parece completo.
            return Err(TransportError::ResponseTooLarge { cap, seen: buf.len() + n });
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}
```

**Truncar está prohibido**, y ése es el punto: P-01 del catálogo de QA dice que un JSON truncado no
se parsea a medias, y el modo de fallo real es peor de lo que parece — `{"schema":1,"apps":[…20 de
40…]` no parsea, pero `{"schema":1,"apps":[]}` cortado justo ahí **sí**, y pintaría «no hay apps».

**Y el `logs --follow`, que es la única lectura sin fin.** No tiene presupuesto de tamaño total —es
un flujo, por definición no termina— sino **presupuesto de anillo**: 5.000 líneas o 8 MB, lo que
llegue antes, y lo viejo se descarta. Más un cortacircuitos: si llegan más de **20.000 líneas por
segundo** durante 5 segundos seguidos, se corta el canal y se dice por qué («el log escribe más
rápido de lo que se puede leer»). Es P-20, y sin el cortacircuitos un bucle de reinicio de una app
llena la memoria del portátil en un minuto.

### 3.9 Basura antes del JSON: se rechaza, no se recorta (SEC-22, SEC-23)

Ésta es la regla que parece pedante hasta que se entiende el ataque, y por eso va escrita con él
delante.

**El fallo tentador:** el `.bashrc` del usuario remoto imprime un banner, o hay un `motd`, o
`orbit` escribió algo por stdout antes del objeto. La solución de una línea es buscar la primera
`{` y parsear desde ahí. **Y esa línea es exactamente cómo un servidor comprometido cuela un objeto
suyo delante del legítimo**: basta con que el `.bashrc` imprima
`{"schema":1,"apps":[]}` y la interfaz enseñará «no hay apps» sobre un servidor con cuarenta.

La regla, entonces:

```rust
fn parse_contract<T: DeserializeOwned>(stdout: &[u8]) -> Result<T, OrbitError> {
    // 1. UTF-8 estricto. Un byte inválido es un error, no un carácter de reemplazo (P-26).
    let text = std::str::from_utf8(stdout)
        .map_err(|e| OrbitError::NotUtf8 { at: e.valid_up_to() })?;

    // 2. Un ÚNICO valor JSON, con el flujo agotado. StreamDeserializer y no from_str:
    //    from_str acepta basura DETRÁS en algunas configuraciones, y aquí no.
    let mut de = serde_json::Deserializer::from_str(text).into_iter::<serde_json::Value>();
    let first = de.next().ok_or(OrbitError::EmptyStdout)??;      // P-06
    if de.next().is_some() { return Err(OrbitError::MultipleJsonValues); }  // P-13

    // 3. El resto del texto sólo puede ser espacio en blanco.
    //    (into_iter ya lo garantiza, pero se comprueba porque es barato y es la invariante.)

    // 4. schema antes de nada (SEC-19).
    let schema = first.get("schema").and_then(|v| v.as_u64())
        .ok_or(OrbitError::NoSchemaField)?;
    if schema != CONTRACT_SCHEMA { return Err(OrbitError::UnknownSchema { got: schema as u32 }); }

    serde_json::from_value(first).map_err(|e| OrbitError::ShapeMismatch(e.to_string()))
}
```

Cuatro consecuencias que se prueban una a una:

- **Basura delante → error** (P-12), con los primeros 200 bytes en el mensaje para que el usuario
  vea que su `.bashrc` está hablando. Y con el consejo correcto, que no es «desactiva tu banner»
  sino **«pon el banner dentro de `if [[ $- == *i* ]]`»**, que es la forma canónica de que un
  `.bashrc` no rompa las sesiones no interactivas.
- **Basura detrás → error** (P-12 por el otro lado). `serde_json::from_str` la ignoraría.
- **Dos objetos → error** (P-13). Ni el primero ni el último.
- **stdout vacío con código 0 → error, nunca «no hay»** (P-06). Es literalmente lo que §13.6bb del
  ARCHITECTURE de Orbit razona al emitir `total: 0` en vez de callar.

**Y la simétrica, que es la mitad que se olvida: stderr NO se parsea nunca como datos.** Sólo dos
cosas se hacen con stderr: (a) buscar líneas que sean NDJSON de progreso, y eso **sólo** cuando se
pidió `--progress`, y (b) guardarlo en un anillo para el mensaje de error. Ninguna decisión de la
interfaz se toma leyendo stderr, salvo la clasificación de errores de §4.5, que es texto para una
persona y se enseña como tal.

### 3.10 La respuesta tiene que corresponder a la pregunta (P-42, P-43)

Es la comprobación que casi nadie pone y es barata, así que se pone. Dos niveles:

**Nivel 1 · La forma encaja con el comando** (P-42). Cada respuesta declara sus claves obligatorias
de primer nivel, y se comprueban antes de deserializar:

```rust
const REQUIRED: &[(&str, &[&str])] = &[
    ("version",       &["schema", "version", "contract"]),
    ("list",          &["schema", "apps"]),
    ("status",        &["schema", "host", "services", "apps"]),
    ("info",          &["schema", "app"]),
    ("doctor",        &["schema", "checks", "summary"]),
    ("top",           &["schema", "apps"]),
    ("deploy",        &["schema", "app", "ok", "release", "commit"]),
    ("deploy-all",    &["schema", "apps", "total", "deployed", "failed",
                        "unchanged", "unreachable", "gone", "skipped", "ok"]),
    ("env-list",      &["schema", "app", "keys"]),
    ("db-list",       &["schema", "databases"]),
    ("redirect-list", &["schema", "redirects"]),
    ("watch-status",  &["schema", "timer_active", "subjects", "summary"]),
    ("queue-status",  &["schema", "timer_active", "apps"]),
    ("metrics",       &["schema", "apps"]),
    ("traffic",       &["schema", "apps"]),
];
```

Sin esto, `list` y `status` son indistinguibles para un deserializador tolerante: `status` tiene
`apps`, así que un `ListResponse` parsearía perfectamente la respuesta de `status`. Y `list`,
`top`, `metrics` y `traffic` **son todos `{"schema":1,"apps":[…]}`** — cuatro comandos con la misma
forma de primer nivel. Es exactamente el caso de P-42 y aquí no es hipotético: **es el contrato
real de Orbit**. La desambiguación se hace por las claves de dentro de `apps[0]`, que sí difieren
(`state` en `list`, `cpu_percent` en `top`, `deploys` en `metrics`, `since` en `traffic`).

**Nivel 2 · La identidad encaja con lo que se pidió** (P-43). Cinco respuestas llevan el nombre de
la app y se comprueba:

| Comando | Campo | Comprobación |
|---|---|---|
| `info <app>` | `app.name` | `== app` pedida |
| `env list <app>` | `app` | `== app` pedida |
| `deploy <app>` | `app` | `== app` pedida — **salvo `""`**, que es la respuesta legítima a `deploy --json` sin app (§1.5) |
| `metrics <app>` | `apps[*].app` | todos `== app` pedida, y `len == 1` |
| `traffic <app>` | `apps[*].app` | idem, con la salvedad de que **una app `redirect` se omite** (§1.5), así que `len` puede ser 0 |
| `deploy --all` | `apps[*].app` | conjunto ⊆ el de `list`, y sin repetidos |

Las que **no** llevan identidad (`list`, `status`, `doctor`, `top`, `db list`, `redirect list`,
`watch status`, `queue status`) se quedan con el nivel 1. Es lo que hay, y decirlo es mejor que
fingir una comprobación que no existe.

**Un tercer nivel que se descarta, con su motivo:** correlacionar petición y respuesta con un
identificador propio (mandar `ORBIT_REQ=<uuid>` y esperarlo de vuelta) sería la comprobación fuerte,
pero exigiría que `orbit` lo devolviera — o sea, **cambiar el contrato del servidor para beneficio
del cliente**, que es justo lo que este proyecto no puede pedir. Sobre un canal SSH multiplexado y
autenticado, con cada comando en su propio canal, el riesgo de que se crucen dos respuestas es
teórico. Las comprobaciones de nivel 1 y 2 son las que valen contra el caso realista: un servidor
que se comporta mal, no un canal que se confunde.

### 3.11 La superficie del socket de `ControlMaster`, medida y decidida

El informe de QA deja esto abierto: *«un socket de control en el sistema de ficheros que otro
proceso del mismo usuario puede reutilizar»*. Se ha comprobado, y la respuesta es peor de lo que
suena y a la vez menos grave de lo que parece. Las dos mitades importan.

**Lo medido.** Con un máster abierto:

```
$ stat -c '%A %U:%G %n' /tmp/orbcm-test
srw------- ubuntu:ubuntu /tmp/orbcm-test

$ env -u SSH_AUTH_SOCK ssh -o BatchMode=yes -o IdentitiesOnly=yes \
      -o IdentityFile=/dev/null -o ControlPath=/tmp/orbcm-test localhost 'id -un'
ubuntu
```

**Sin clave, sin agente, sin frase de paso.** El socket es 0600 —ningún otro usuario del sistema lo
puede abrir— pero **cualquier proceso del mismo usuario obtiene un canal sobre una sesión ya
autenticada**, y si esa sesión es la de un servidor donde `orbit` se auto-eleva a root, eso es root.

**Lo que NO cambia, y hay que decirlo antes de alarmar.** El `ssh-agent` tiene **exactamente la
misma propiedad**: cualquier proceso del usuario puede pedirle una firma y autenticarse en
cualquier sitio donde valga esa clave. Un cliente que no use `ControlMaster` pero sí el agente
—o sea, cualquier cliente SSH— está en la misma situación. **`ControlMaster` no introduce una clase
de amenaza nueva.** El modelo de amenazas de Orbit Desktop asume que un atacante con ejecución de
código como el usuario ya ha ganado, y eso es cierto con o sin socket.

**Lo que SÍ cambia, y es lo que decide.** Dos cosas concretas:

1. **Duración.** El agente se puede bloquear (`ssh-add -x`), se puede vaciar (`ssh-add -D`) y las
   claves se pueden añadir con caducidad (`ssh-add -t 1h`). El socket de control vive lo que diga
   `ControlPersist` y no hay nada que lo caduque antes.
2. **Y la que de verdad importa: el socket salta la confirmación.** Una clave añadida con
   `ssh-add -c` pide confirmación al usuario **en cada autenticación**; una clave FIDO
   (`sk-ssh-ed25519@openssh.com`, una YubiKey) exige **un toque físico por conexión**. Con
   `ControlMaster`, todo eso se paga **una vez** y luego cada canal es gratis. Para el usuario que
   eligió una llave con toque precisamente para que nada se autentique sin él, el multiplexado
   **degrada silenciosamente su modelo de seguridad**. Y ni lo sabe ni se lo hemos preguntado.

**Las decisiones que salen de ahí, y son tres:**

**a) `ControlPersist` baja de 120 s a 45 s.** La ventana de exposición es exactamente ese número, y
45 s cubre de sobra una ráfaga de pantallas: la portada (400 ms) más el detalle de dos apps (200 ms)
son menos de un segundo. Lo que se pierde es que volver a la app a los dos minutos pague otro
apretón de manos. Es un precio razonable por reducir la ventana a la tercera parte.

**b) El multiplexado se APAGA solo con claves de hardware o con confirmación.** Al dar de alta un
servidor, el cliente ejecuta `ssh-add -l` y mira los tipos:

```
sk-ssh-ed25519@openssh.com      → FIDO/YubiKey: toque por conexión
sk-ecdsa-sha2-nistp256@openssh.com → idem
```

Si la clave del servidor es una de ésas, **`ControlMaster` queda desactivado por defecto** para ese
servidor, con esta explicación en la interfaz:

> «Este servidor usa una llave de hardware. Orbit Desktop no reutiliza la conexión, así que cada
> pantalla te pedirá un toque. Puedes activar la reutilización (más rápido, un toque cada 45
> segundos), pero entonces cualquier programa que corra con tu usuario podrá usar esa conexión sin
> tocar la llave.»

Que es exactamente lo que pasa, dicho sin adornos. La confirmación por `ssh-add -c` **no se puede
detectar** desde fuera del agente —`ssh-add -l` no lo dice—, así que ahí se ofrece un interruptor
manual con el mismo texto.

**c) El directorio del socket lo crea y lo verifica el cliente.** No `/tmp` a pelo, porque en un
`/tmp` compartido y con *sticky bit* un atacante local puede pre-crear rutas y jugar con enlaces.
`$XDG_RUNTIME_DIR/orbit-desktop/` en Linux (ya es 0700 y propiedad del usuario, creado por systemd);
`$TMPDIR/orbit-desktop/` en macOS (que es `/var/folders/xx/…/T/`, privado por usuario). Y antes de
cada uso:

```rust
fn ensure_control_dir(p: &Path) -> Result<(), TransportError> {
    fs::create_dir_all(p)?;
    let md = fs::symlink_metadata(p)?;                       // symlink_metadata: no seguir enlaces
    if md.file_type().is_symlink() { return Err(TransportError::ControlDirIsSymlink); }
    if md.uid() != geteuid() { return Err(TransportError::ControlDirNotOwned); }
    if md.mode() & 0o077 != 0 { fs::set_permissions(p, Permissions::from_mode(0o700))?; }
    if p.as_os_str().len() + 66 > 100 { return Err(TransportError::ControlPathTooLong); }
    Ok(())
}
```

La última línea es la que evita el fallo más tonto y más frecuente: `sun_path` son 104 bytes en
macOS y 108 en Linux, y `ssh` falla con un mensaje incomprensible si se pasa.

**d) Y al cerrar la aplicación, se cierran los másters.** `ssh -O exit` por cada servidor con socket
abierto, en el manejador de cierre. No hay que esperar los 45 s de `ControlPersist`: si Orbit
Desktop ya no está, la conexión no tiene por qué seguir. Es la versión de transporte del principio
de que el servidor no gana nada permanente.

### 3.12 Las 25 reglas duras del informe de QA, y dónde se cumple cada una

Repaso completo, porque una regla que no se puede señalar en el diseño es una regla que no está.

| Regla | Dónde se cumple | Verificación |
|---|---|---|
| SEC-01 · ninguna concatenación | §7.2 `to_argv() -> Vec<String>` | R-02 (semgrep), §7.1 |
| SEC-02 · todo pasa por el escapador | §7.2, y **E3: 28.464 viajes en 4 shells** | prueba de propiedad |
| SEC-03 · nunca por `PATH` | §3.2, ruta absoluta en el `argv` | test unitario del argv |
| SEC-04 · dos transportes con nombres distintos | §3.6: modo A/B (`run`) y modo C (`exec`) | R-02 |
| SEC-05 · validación por forma | §7.2 `AppName`, `EnvKey`, `ReleaseId` | tests con corpus |
| SEC-06 · lo que viene del servidor es no confiable | §7.2: el tipo del contrato ≠ el tipo del constructor | lo fuerza el compilador |
| SEC-07 · ningún secreto guardado | §3.2 (`SSH_ASKPASS`), §6.1 | `grep -r` sobre el disco |
| SEC-08 · no implementamos autenticación SSH | §2.2 · la decisión entera | lectura + `ProxyJump` real |
| SEC-09 · `accept-new`, nunca `no` | §3.2, y `=yes` durante el alta | `sshd` con clave cambiada |
| SEC-10 · `ForwardAgent` apagado | §3.2, **explícito en el argv** | test del argv |
| SEC-11 · el valor de `env get` sólo en memoria | §5.5: `envKeys` se cachea, el **valor no** | §5.5 de QA |
| SEC-12 · el `.env` nunca entero | §1.3: el contrato **no lo permite** | el servidor ya lo impide |
| SEC-13 · los secretos se ocultan solos | interfaz | test de interfaz |
| SEC-14 · nada del contrato como marcado | R-03 (`no-at-html-tags`) | lint + caso `</script><img>` |
| SEC-15 · telemetría sin cadenas del servidor | no hay telemetría en la v1 | trivial |
| SEC-16 · informes de fallo revisables | interfaz | test de interfaz |
| SEC-17 · el log registra comando y objetivo | §3.3: el anillo de stderr **no** se persiste | inspección |
| SEC-18 · lo desconocido no es un valor | §4.3 (los siete `null`) + R-05 (lint) | fuzzing G1 |
| SEC-19 · `schema` y `contract` antes de nada | §3.9 paso 4, §4.4 | P-04, P-05 |
| SEC-20 · campo nuevo se ignora; campo con otro tipo es error | §4.1 reglas 2 y 3 | P-02, P-03, P-28 |
| SEC-21 · presupuesto de tamaño y tiempo | **§3.8, con cifras medidas** | flujo infinito |
| SEC-22 · un solo objeto por stdout; stderr nunca son datos | §3.9 | P-11, P-13 |
| SEC-23 · basura → fallo, no recorte | **§3.9, con el ataque explicado** | P-12 |
| SEC-24 · sólo se invoca `orbit` | §7.2: el catálogo de `argv` es finito y está en un fichero | auditoría por lectura + e2e |
| SEC-25 · no se instala nada | §4.4: el instalador **se copia, no se ejecuta** | e2e con inventario |

**Dos de las 25 no se cumplen del todo y hay que decirlo:**

- **SEC-11** tiene un agujero que no es del cliente: `orbit env get` deja el valor en el **historial
  de comandos del servidor** si alguien lo teclea a mano, y en nuestro caso en el `auth.log` del
  `sshd` sólo aparece el hecho de la conexión, no el comando. Pero **el comando sí puede acabar en
  el journal** si el `sshd` tiene `LogLevel VERBOSE`. No hay nada que el cliente pueda hacer:
  se documenta en el modelo de amenazas y se dice al usuario que `env get` deja rastro en el
  servidor.
- **SEC-17**: `orbit` mismo escribe `logline "exec $name"` (línea 9024) — sólo el nombre, nunca el
  comando, y el comentario del código lo justifica («un comando puede llevar una contraseña
  delante»). El cliente hereda eso gratis. Pero **`orbit env set` sí registra la clave** (línea
  11248: `logline "env set $name $key"`), no el valor. Correcto, y conviene saber que el nombre de
  la variable sí queda escrito en `/var/log/orbit/orbit.log`.

## 4. Modelo de datos y tipado

### 4.0 Dónde viven los tipos y por qué se generan, no se escriben dos veces

Los tipos se definen **una vez, en Rust**, con `serde`, dentro de `crates/orbit-client/src/contract/`.
El TypeScript se **genera** desde ahí con `ts-rs` (macro `#[derive(TS)]`) en un test de `cargo`, que
escribe `src/contract/generated.ts` y falla si el fichero no está al día.

El motivo no es elegancia: es que **el contrato tiene 37 campos de configuración, 11 de estado, 6
finales de despliegue, 4 niveles de diagnóstico y 15 formas de respuesta**, y mantenerlo a mano en
dos lenguajes es garantizar que se desincronizan. Es exactamente el problema que `ORBIT_APP_FIELDS`
resolvió del lado del servidor (§13.1 de ARCHITECTURE: «antes cada sitio tenía la suya y añadir un
campo era acordarse de tres»). El mismo remedio, del lado del cliente.

### 4.1 Los primitivos, y la disciplina de los `null`

**Regla nº 1: un campo que puede ser `null` se tipa `Option<T>` en Rust y `T | null` en TypeScript.
Nunca `T` con un valor por defecto.** Un `port: number` con `0` cuando no hay puerto es la mentira
que §13.1 prohíbe explícitamente. Y no basta con tipar: hay que **impedir que la interfaz lo
aplaste**. Por eso ningún componente recibe `port` y hace `port ?? 0`; recibe `port` y decide qué
pintar cuando es `null`. Se comprueba con un lint (§9.2).

**Regla nº 2: `serde` con `deny_unknown_fields` NUNCA.** El contrato promete que los campos se
añaden. Un cliente que rechace un campo nuevo se rompe con la siguiente versión de `orbit`, que es
justo lo contrario de lo que la promesa persigue. Los campos desconocidos se ignoran en silencio.

**Regla nº 3: los enumerados del contrato se tipan con un caso `Unknown(String)`.** Si `orbit`
añade un séptimo final a `deploy --all`, el cliente lo enseña como «desconocido: <valor>» en vez de
fallar al deserializar. `#[serde(other)]` para el caso unitario, o un `enum` con variante de captura.

```rust
#[derive(Deserialize, Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState { Running, Stopped, #[serde(other)] Unknown }

#[derive(Deserialize, Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {                       // deploy --all
    Deployed, Failed, Unchanged, Unreachable, Gone, Skipped,
    #[serde(other)] Unknown,
}

#[derive(Deserialize, Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel { Ok, Info, Warn, Error, #[serde(other)] Unknown }

#[derive(Deserialize, Serialize, TS, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeployStep { Code, Release, Build, Activate, Service, Nginx, #[serde(other)] Unknown }
```

### 4.2 Los tipos, completos

En TypeScript, que es como se van a leer más. El Rust es la traducción directa.

```ts
// ─── envoltura común ────────────────────────────────────────────────────────
/** Todas las respuestas del contrato lo llevan. Hoy vale 1. */
export interface Schema { schema: number }

// ─── version ────────────────────────────────────────────────────────────────
export interface VersionResponse extends Schema {
  version: string;      // "1.3.6"
  contract: number;     // hoy == schema. Se comparan por separado a propósito.
}

// ─── el estado observado de una app ─────────────────────────────────────────
export interface AppState {
  service: 'running' | 'stopped' | null;   // null en static, php, laravel y redirect
  port: number | null;                     // null en las que no tienen proceso
  ssl: boolean;
  /** OJO: null en `list` y en `status` SIEMPRE — ahí no se calcula.
   *  Sólo `info --json` lo trae. null NO significa "sin certificado": eso es `ssl`. */
  cert_days: number | null;
  maintenance: boolean;
  /** Lo primero que hay que mirar. false = nginx no tiene vhost: la conexión se cierra. */
  served: boolean;
  autodeploy: boolean;
  queue: boolean;
  releases: number;
  last_deploy: string | null;              // parte antes del primer espacio de A_LASTDEPLOY
  last_deploy_sha: string | null;          // parte tras el último espacio
}

// ─── list ───────────────────────────────────────────────────────────────────
export type AppType =
  | 'static' | 'next' | 'node' | 'go' | 'bun' | 'deno'
  | 'php' | 'laravel' | 'python' | 'redirect'
  | (string & {});                          // el contrato puede añadir tipos

export interface AppSummary {
  name: string;
  type: AppType;
  domain: string;
  aliases: string[];                        // puede ser []
  state: AppState;
}
export interface ListResponse extends Schema { apps: AppSummary[] }

// ─── info ───────────────────────────────────────────────────────────────────
/** El fichero /etc/orbit/apps/<app>.conf, clave a clave.
 *  TODOS los valores son cadenas, y una clave sin valor es "" (nunca null).
 *  37 campos: ORBIT_APP_FIELDS, líneas 1308-1315. */
export interface AppConfig {
  name: string; repo: string; branch: string; domain: string; aliases: string;
  type: string; pkg: string; build: string; start: string; outdir: string;
  spa: string; port: string; user: string; docroot: string; pyapp: string;
  appdir: string; pymgr: string; pyfw: string; migrate: string;
  static_url: string; static_root: string; media_url: string; media_root: string;
  redirect: string; redirect_code: string; autodeploy: string; autofail: string;
  maint_auto: string; queue: string; php: string; pnpm_allow: string;
  node_heap: string; shared: string; env_file: string; env_spec: string;
  created: string; lastdeploy: string;
  // El contrato puede añadir campos: se aceptan y se ignoran.
  [k: string]: string;
}

export interface InfoResponse extends Schema {
  app: {
    name: string;
    path: string;                 // /srv/apps/<name>
    config: AppConfig;
    state: AppState;              // aquí cert_days SÍ viene calculado
    releases: string[];           // "20260805-041230", de la más nueva a la más vieja
  };
}

// ─── status ─────────────────────────────────────────────────────────────────
export interface StatusResponse extends Schema {
  host: {
    hostname: string;
    ip: string;
    uptime_seconds: number | null;
    load: [number, number, number];
    memory_kb: { total: number|null; used: number|null; available: number|null };
    disk_kb:   { mount: string; total: number|null; used: number|null; use_percent: number|null };
  };
  services: { name: string; active: boolean }[];   // nginx, postgresql, php<v>-fpm, fail2ban, ufw
  apps: AppSummary[];                              // el MISMO array de list --json, entero
}

// ─── doctor ─────────────────────────────────────────────────────────────────
export interface Check {
  id: string;                     // único. "nginx", "vhost-<app>", "dns:<dominio>"
  level: 'ok' | 'info' | 'warn' | 'error';
  message: string;                // TRADUCIDO
  fix: string | null;             // TRADUCIDO. Un consejo, no siempre una orden ejecutable
  fixable: boolean;               // true sólo si hay acción Y level es warn|error
}
export interface DoctorResponse extends Schema {
  checks: Check[];
  /** OJO: summary.ok cuenta los `ok` Y los `info`. No cuadra con
   *  checks.filter(c => c.level === 'ok').length. */
  summary: { ok: number; warn: number; error: number };
}

// ─── top ────────────────────────────────────────────────────────────────────
export interface TopApp {
  name: string; type: AppType; domain: string;
  port: number | null;
  service: 'running' | 'stopped' | null;
  cpu_percent: number | null;             // null la primera muestra: "no se sabe"
  memory_bytes: number | null;            // null si no hay unidad
  requests_last_minute: number | null;    // null si el log no tiene marca de tiempo
  requests_capped: boolean;               // true = el minuto llenó las 5000 líneas
}
export interface TopResponse extends Schema { apps: TopApp[] }

// ─── deploy ─────────────────────────────────────────────────────────────────
export interface DeployResult extends Schema {
  app: string;                    // "" si se invocó sin nombre (sí, contesta igual)
  ok: boolean;
  release: string | null;
  previous: string | null;        // para ofrecer el rollback sin otra llamada
  commit: { sha: string|null; subject: string|null; ref: string|null };
  rolled_back: boolean;
  recovered: boolean;             // orbit arregló el build solo y reintentó
  duration_s: number;
  failed_step: 'code'|'release'|'build'|'activate'|'service'|'nginx'|null;
  error: string | null;           // TRADUCIDO. 22 cadenas fijas. No parsear.
}

export interface DeployAllEntry {
  app: string;
  status: 'deployed'|'failed'|'unchanged'|'unreachable'|'gone'|'skipped';
  error: string | null;           // TRADUCIDO (o el texto de git en unreachable)
  result: DeployResult | null;    // null en unchanged/unreachable/gone/skipped
}
export interface DeployAllResponse extends Schema {
  apps: DeployAllEntry[];
  total: number;
  deployed: number; failed: number; unchanged: number;
  unreachable: number; gone: number; skipped: number;
  ok: boolean;                    // f==0 && unreachable==0 && gone==0
  duration_s: number;
}

// ─── progreso (NDJSON por stderr) ───────────────────────────────────────────
export type ProgressEvent =
  | { event: 'step'; app: string; step: DeployStep|string; status: 'start'|'ok'; elapsed_s: number }
  | { event: 'app';  app: string; status: 'start'|DeployAllEntry['status']; elapsed_s: number };
// NO existe status:'error'. Un paso que falla se reconoce por el `start` sin `ok`.

// ─── env / db / redirect / watch / queue ────────────────────────────────────
export interface EnvListResponse extends Schema { app: string; keys: string[] }  // SÓLO nombres
export interface DbListResponse  extends Schema {
  databases: { name: string; owner: string; size_bytes: number|null }[];
}
export interface RedirectListResponse extends Schema {
  redirects: { app: string; kind: 'domain'|'path'; from: string; to: string;
               code: number|null; query: boolean }[];
}
export interface WatchStatusResponse extends Schema {
  timer_active: boolean;
  /** `state` es una cadena EN ESPAÑOL escrita en el código, no traducida:
   *  "ok" | "rendido" | "sin-contacto" | "sin-rama" | … Mapear con un default. */
  subjects: { subject: string; state: string; since: number|null;
              tries: number|null; last_try: number|null }[];
  summary: { total: number; down: number; warning: number };
}
export interface QueueStatusResponse extends Schema {
  timer_active: boolean;
  every_minutes: number | null;
  max_seconds: number | null;
  apps: { app: string; type: AppType; connection: string | null }[];
}

// ─── metrics ────────────────────────────────────────────────────────────────
/** SIEMPRE envuelto en apps[], también con una sola app. Y trae `kept`. */
export interface MetricsResponse extends Schema {
  apps: {
    app: string;
    deploys: number | null;
    failed: number | null;
    build_median_s: number | null;
    build_trend_s: number | null;   // null con menos de 6 builds: NO es "cero tendencia"
    last: string | null;            // fecha ISO del último despliegue apuntado
  }[];
  kept: number | null;              // METRICS_KEEP. No documentado en USAGE
}

// ─── traffic ────────────────────────────────────────────────────────────────
export interface TrafficApp {
  app: string;
  since: string;                    // la VENTANA pedida ("24h"), no un instante
  from: string;                     // "AAAAMMDDHHMMSS" — NO es ISO. Hay que parsearlo a mano
  complete: boolean;                // false = la ventana excede lo que cubre el log
  requests: number | null;
  ips: number | null;
  ips_capped: boolean;
  bytes: number | null;
  automated: number | null;         // escáneres. Va APARTE de requests, no restado
  status: Record<string, number>;   // DISPERSO: sólo las clases con tráfico. Puede ser {}
  latency_ms: { p50: number|null; p95: number|null; max: number|null; lines: number|null };
  paths: { path: string; requests: number }[];
  referrers: { referrer: string; requests: number }[];
  hours: { hour: string; requests: number }[];
}
/** SIEMPRE envuelto, también con una sola app. Las apps `redirect` se OMITEN. */
export interface TrafficResponse extends Schema { apps: TrafficApp[] }
```

### 4.3 Los siete `null` que hay que tratar con nombre propio

No basta con tipar `| null`: cada uno significa algo distinto y la interfaz tiene que decir cosas
distintas. Ésta es la tabla que va pegada al componente:

| Campo | `null` significa | Qué pinta la interfaz |
|---|---|---|
| `state.service` | «esta clase de app no tiene proceso» | Nada. Ni verde ni rojo. Un guion, o la palabra del tipo |
| `state.port` | «no usa puerto interno» | Un guion. Nunca `0` |
| `state.cert_days` | en `list`/`status`: «no lo he mirado». En `info`: «no hay certificado» | En la lista, nada. En el detalle, «sin certificado» si `ssl` es `false` |
| `top.cpu_percent` | «primera muestra, no hay con qué comparar» | «—», con un tooltip. **Nunca 0 %** |
| `top.memory_bytes` / `requests_last_minute` | «no lo sé» | «—». Y si `requests` es null, ofrecer `nginx-rebuild` |
| `metrics.build_trend_s` | «menos de 6 builds: dos datos no son una tendencia» | Nada, y explicarlo. Un `0` se leería como «estable» |
| `traffic.latency_ms.*` | «no hay líneas con tiempo de respuesta» | «—» |

Y dos booleanos que valen tanto como los `null`:

- **`state.served === false`** es el estado más fuerte de una app y **gana sobre todos los demás**,
  incluido `maintenance`. Sin vhost, nginx cierra la conexión: ni 404, ni 502, ni la página de 503.
  La interfaz lo pinta como el estado, no como un detalle: la fila entera en rojo, con la palabra
  «sin vhost», y las demás columnas atenuadas. Es exactamente lo que hace `cmd_list` (8546-8556) y
  el comentario del código lo justifica mejor de lo que se podría reescribir aquí.
- **`traffic.complete === false`** obliga a decir «la ventana sale recortada desde <from>» antes de
  enseñar el número. Un total que no cubre lo que se pidió y no lo avisa es lo que §13.8 llama el
  peor final posible.

### 4.4 Versionado del contrato y política de compatibilidad

El cliente lee `orbit --json version` en el alta y en cada transición a `Ready`. Compara **dos**
números, que son dos cosas distintas y por eso `orbit` los publica separados (`cmd_version`, 12020):

```rust
pub const CONTRACT_MIN: u32 = 1;   // lo más viejo que sabemos leer
pub const CONTRACT_MAX: u32 = 1;   // lo más nuevo que hemos probado

pub enum Compatibility {
    Ok,
    NewerServer { contract: u32 },   // contract > CONTRACT_MAX
    OlderServer { contract: u32 },   // contract < CONTRACT_MIN
    NoContract,                      // orbit responde pero no habla --json
    NotInstalled,
}
```

**Contrato 1 (hoy): `Ok`.** Todo funciona.

**Contrato 2 o superior: `NewerServer`. Se sigue hablando, con aviso.** Y esto merece defensa,
porque la reacción instintiva es negarse.

La promesa de `orbit` es que **los campos se añaden y nunca se renombran**, y que subir `schema` es
el mecanismo de última instancia para romper. Un cliente que se niegue a hablar con contrato 2
convierte cada subida del servidor en una interfaz muerta hasta que se publique una versión del
cliente — y como el cliente se actualiza solo (§8) pero el usuario puede tenerlo cerrado una semana,
eso es una semana sin herramienta. Al mismo tiempo, contrato 2 significa que **algo se rompió a
propósito** y no podemos saber qué.

La política que equilibra las dos cosas:

1. **Modo seguro.** Con `contract > CONTRACT_MAX`, el cliente entra en modo lectura degradada:
   se permiten `version`, `list`, `info` y `status`, que son las cuatro formas más estables y las
   que un cliente puede tolerar que ganen campos. **Se bloquean todas las mutaciones**: nada de
   `deploy`, `remove`, `rollback`, `ssl`. La razón es asimétrica y esa asimetría es el argumento:
   leer un campo que ya no significa lo mismo pinta un dato erróneo; **ejecutar** una mutación cuyo
   contrato cambió puede borrar algo.
2. **Banda persistente y visible.** «Este servidor habla el contrato 2 y esta versión de Orbit
   Desktop conoce hasta el 1. Se muestra sólo lectura. Actualiza Orbit Desktop.» Con un botón que
   comprueba actualizaciones.
3. **Y si el parseo falla**, se cae a `NoContract` (abajo).

**Ronda 2 · esta política deja de ser una apuesta si se acepta 9b-5.** La debilidad que la ronda 1
nombró aquí es real: «lectura degradada» suena razonable, pero como el contrato 2 no existe, no se
sabe qué se romperá — podría romperse justo el campo que la interfaz usa para clasificar. Lo que
hace falta no es código del cliente: es que `orbit` **escriba qué sobrevive a una subida de
`schema`**. Ese texto está redactado y listo para PR en **§9b-5**, y garantiza tres cosas para
siempre: la forma de `version --json`, la separación de canales, y que **lo que no existe siga
siendo `null`**.

Con esas tres escritas, la política de arriba se puede justificar campo a campo en vez de
defenderse por prudencia: el paso 1 (saludar) se apoya en la garantía 1; el paso 3 (parsear stdout
sin mezclarlo con stderr) en la 2; y la única razón por la que la lectura degradada es segura —que
un `null` no se habrá convertido en un 0 que la interfaz pinte como dato— en la 3. **Mientras 9b-5
no esté aceptado, esta sección es lo mejor que se puede hacer sin él, y hay que leerla sabiéndolo.**

**Contrato 0 o inexistente: `NoContract`.** Dos subcasos que hay que distinguir porque el mensaje
es distinto:

- **`orbit --json version` falla con `'orbit version' no tiene salida JSON`** → es un `orbit` muy
  anterior a la v1.1, cuando `version` aún no hablaba JSON. Se ejecuta `orbit version` a secas, que
  imprime `orbit 1.0.4`, se parsea la versión y se le dice al usuario: «Este servidor tiene Orbit
  1.0.4. Orbit Desktop necesita 1.1 o superior. Actualízalo en el servidor con `install.sh`.»
- **`orbit --json version` devuelve JSON pero sin `contract`** → contrato implícito 0. Mismo
  tratamiento.

En los dos casos, **modo informativo, cero funcionalidad**. No se intenta adivinar. Un cliente que
intente hablar un contrato que no existe acaba parseando tablas, y ése es el camino que §13.1 dice
que no se toma.

**`orbit` no instalado: `NotInstalled`.** Se reconoce por código 127 (o por las dos líneas
`Orbit no está instalado todavía (falta /etc/orbit/orbit.conf)` por stdout, §1.9). La interfaz
enseña **la instrucción exacta**, que está en la salida del propio `orbit`:

```
curl -fsSL https://raw.githubusercontent.com/iNTERVOLUTIONS-Labs/orbit/main/install.sh | sudo bash
```

…con un botón «Copiar» y **no** con un botón «Instalar». Instalar Orbit desde el cliente sería la
primera vez que Orbit Desktop escribe en el servidor algo que no es una invocación de `orbit`, y
la regla dura nº 1 no admite un «pero es el instalador». Se copia el comando; lo ejecuta la persona.

**La matriz completa**, que es lo que se implementa:

| `version --json` | Clasificación | Qué se permite |
|---|---|---|
| JSON, `contract` en [1,1] | `Ok` | Todo |
| JSON, `contract` > 1 | `NewerServer` | Lectura de `version`/`list`/`info`/`status`. Banda de aviso |
| JSON, sin `contract` o 0 | `NoContract` | Nada. Mensaje de «actualiza Orbit» |
| rc=1, stderr «no tiene salida JSON» | `NoContract` (pre-1.1) | Nada. Se lee `orbit version` para dar el número |
| rc=127 | `NotInstalled` | Nada. Comando de instalación para copiar |
| stdout con «Orbit no está instalado» | `NotInstalled` | Idem |
| stderr con «necesita privilegios de root» / mensaje de sudo | `Degraded` | Nada. Asistente de privilegios (§1.9) |
| rc=255 + stderr de OpenSSH | `Unreachable` | Nada. Reintento con retroceso |

### 4.5 `OrbitError`, y el arte de no enseñar un error de Bash

```rust
pub enum OrbitError {
    /// orbit contestó JSON válido pero con ok:false (deploy fallido, típicamente)
    Failed { data: serde_json::Value, code: i32, stderr_tail: Vec<String> },
    /// orbit murió con die(). Código 1, stdout vacío.
    Command { code: i32, message: String, stderr_tail: Vec<String> },
    /// El comando no habla JSON (rechazo del contrato)
    NoJsonSupport { command: String, capable: Vec<String> },
    /// stdout no parsea
    NotJson { head: String, stderr_tail: Vec<String> },
    UnknownSchema { got: u32 },
    NotInstalled,
    NeedsRoot { hint: String },
}
```

**`message` sale de stderr y hay que limpiarlo.** `err()` (471) escribe
`"  ${RED}✗${R} <mensaje>\n"`. Sin TTY no hay códigos de color, así que la línea es literalmente
`"  ✗ La app 'x' no existe. Mira 'orbit list'."`. La limpieza es: tomar la **primera** línea de
stderr que empiece por dos espacios y el glifo de error (`✗` o `x` en modo ASCII), quitar el
prefijo, y si hay líneas siguientes que empiezan por dos espacios y `·` o `!`, son el contexto
(`info`/`warn`) y se guardan aparte como «detalle».

Y hay que ser honesto sobre el límite: **el mensaje viene traducido y no tiene código**. Ni el
cliente ni nadie puede distinguir programáticamente «la app no existe» de «nginx rechazó la
configuración». Lo único que se puede hacer —y se hace— es reconocer un puñado de casos por su
forma **antes** de que ocurran, no después: si el cliente ya tiene la lista de apps en caché, sabe
que la app no existe sin preguntar; si el usuario no es root, ya lo detectó el asistente de alta.
La clasificación por texto se limita a tres patrones estables y con un `default` honesto:

| Patrón en stderr | Clasificación |
|---|---|
| `necesita root` / `needs root` / `sudo:` | `NeedsRoot` |
| `no tiene salida JSON` / `has no JSON output` | `NoJsonSupport` — **es un bug del cliente**, se reporta como tal |
| `no está instalado` / `is not installed yet` | `NotInstalled` |
| cualquier otro | `Command`, y se enseña el texto de `orbit` tal cual |

Ese último caso es la decisión importante: **cuando no se sabe clasificar, se enseña el mensaje de
`orbit` literal**, en el idioma del usuario, con el «detalle técnico» plegado debajo. Los mensajes
de `orbit` están escritos para una persona y son buenos: «La app 'x' no existe. Mira 'orbit list'»
es mejor mensaje del que escribiríamos nosotros. Inventar una capa de mensajes propios encima sería
traducir dos veces y perder información.

---

## 5. Estado, caché e invalidación

### 5.0 El presupuesto, medido

**Ronda 2: esta sección se ha rehecho entera con medidas propias.** Banco de 40 apps
(`tmp/bench`, cuatro tipos: 10 `static`, 10 `node`, 10 `next`, 10 `php`), máquina de 8 vCPU,
Ubuntu 26.04, sin systemd corriendo — o sea, un **suelo**: en un servidor con systemd de verdad,
todo lo que consulte unidades cuesta más.

| Comando | Mediana | min / max | n | Tamaño de la respuesta |
|---|---|---|---|---|
| `orbit version --json` | **76 ms** | 71 / 78 | 7 | 44 B |
| `orbit info app01 --json` | **85 ms** | 85 / 95 | 7 | 944 B |
| `orbit list --json` | **314 ms** | 303 / 324 | 7 | 12,4 KB |
| `orbit status --json` | **388 ms** | 385 / 391 | 5 | 12,9 KB |
| `orbit metrics --json` | **936 ms** | 925 / 958 | 5 | 3,8 KB |
| `orbit doctor --json` | **1.420 ms** | 1.420 / 4.119 | 3 | 19,8 KB · 109 comprobaciones |
| `orbit traffic --json` | **2.017 ms** | 2.007 / 2.121 | 3 | 10,0 KB · **con los logs vacíos** |
| `orbit top --json` | **2.116 ms** | 1.791 / 2.284 | 3 | 7,3 KB |

Cinco lecturas, y cuatro de ellas cambian decisiones que en la ronda 1 estaban tomadas a ojo.

**1 · Hay un suelo de 76 ms por llamada, y no está en ninguna documentación de Orbit.**
`version --json` no lee ni una app: emite 44 bytes. Esos 76 ms son **13.720 líneas de Bash que
bash parsea cada vez**, más `_i18n_load` construyendo un array asociativo de ~700 mensajes. Es un
coste fijo que se paga en **cada** pantalla, antes de la red y antes de que `orbit` mire nada.
Con `ControlMaster`, el canal SSH cuesta 13 ms (§3.2, medido): o sea que **el 85 % del coste de la
llamada más barata del contrato es arrancar el intérprete**, no la red ni el trabajo.

**2 · `list --json` escala limpiamente y la recta es útil.** Midiendo con 1, 5, 10, 20 y 40 apps:

```
N=1   76 ms      N=5   99 ms      N=10  135 ms      N=20  189 ms      N=40  309 ms
```

Regresión: **`list --json` ≈ 72 ms + 5,9 ms por app**. Con 100 apps serían ~660 ms; con 200,
~1,25 s. Eso permite decidir a partir de qué tamaño la lista necesita paginarse o pedirse en dos
tandas — y la respuesta es que **no hace falta hasta bien pasadas las 100 apps**, que es más de lo
que Orbit está pensado para llevar en un VPS.

**3 · `top --json` cuesta 2,1 s con 40 apps, no 1 s.** Ésta es la que rompe un plan de la ronda 1.
Midiendo el escalado: N=10 → 1.319 ms, N=40 → 2.116 ms, o sea **≈ 1.053 ms + 26,6 ms por app**.
El intercepto es el `sleep 1` de la línea 8760 más el arranque; la pendiente son **dos** mediciones
por app (una antes del `sleep` y otra en `_top_json`, líneas 8756-8761). **Con 40 apps, un
`refetchInterval` de 2 s es físicamente imposible.** Ver §5.4, reescrita.

**4 · `metrics --json` cuesta 936 ms con el histórico VACÍO.** Sorprende hasta que se lee
`_metrics_app_json` (6193-6209): por cada app hace `_metrics_lines | wc -l`, `| awk | wc -l`,
un `mapfile` con otro `awk`, `_med`, `_metrics_trend` y `| tail -n 1 | cut`. Son ~7 procesos por
app, 280 con 40 apps. Es el único comando del contrato cuyo coste **no** depende de los datos sino
del número de apps. No es un problema —se pide poco— pero desmiente la intuición de que «métricas
es barato porque sólo lee un TSV».

**5 · Ninguna respuesta pasa de 20 KB.** La mayor es `doctor --json` con 19,8 KB y 109
comprobaciones. Eso fija el presupuesto de tamaño de §3.9 en algo mucho más estrecho —y por tanto
más útil— que los 8 MB que pedía SEC-21 del informe de QA.

**El presupuesto de pantalla, con estos números:**

| Pantalla | Llamada | Coste (mux, VPS a 30 ms) | Veredicto |
|---|---|---|---|
| Portada de servidor | `status --json` | ~420 ms | Esqueleto, luego datos |
| Detalle de app | `info --json` | ~120 ms | **Directo, sin esqueleto** |
| Diagnóstico | `doctor --json` | ~1,5 s | Bajo demanda, con progreso |
| Monitor | `top --json` | ~2,2 s por muestra | Ver §5.4 |
| Tráfico | `traffic --json` | 2 s a decenas | Bajo demanda, cancelable |
| Métricas | `metrics --json` | ~1 s | Bajo demanda |

**La regla, ya con número:** por debajo de 150 ms se pinta el resultado directamente (nadie ve un
esqueleto de 120 ms, y enseñarlo produce un parpadeo que es peor que la espera). Entre 150 ms y
1 s, esqueleto. Por encima de 1 s, esqueleto **más** un indicador de progreso con botón de cancelar
—que es lo que pide P-19 del catálogo de QA—.

### 5.1 La arquitectura: caché por servidor con TanStack Query, y los datos en Rust

El estado del servidor remoto vive en **`@tanstack/svelte-query` v5** en el frontend, con la clave siempre
prefijada por el servidor:

```ts
const qk = {
  version: (s: ServerId) => ['srv', s, 'version'] as const,
  list:    (s: ServerId) => ['srv', s, 'list'] as const,
  status:  (s: ServerId) => ['srv', s, 'status'] as const,
  info:    (s: ServerId, a: string) => ['srv', s, 'info', a] as const,
  top:     (s: ServerId) => ['srv', s, 'top'] as const,
  doctor:  (s: ServerId) => ['srv', s, 'doctor'] as const,
  metrics: (s: ServerId, a?: string) => ['srv', s, 'metrics', a ?? '*'] as const,
  traffic: (s: ServerId, a: string|undefined, since: string) =>
             ['srv', s, 'traffic', a ?? '*', since] as const,
  envKeys: (s: ServerId, a: string) => ['srv', s, 'env', a] as const,
  // …
};
```

Por qué TanStack Query (en su adaptador de Svelte, §2.5) y no un store propio: porque `staleTime`, `gcTime`, el deduplicado de
peticiones en vuelo, el reintento con retroceso, el `refetchOnWindowFocus` y la invalidación por
prefijo ya están escritos, probados y son exactamente lo que hace falta. Escribir eso a mano es un
mes y acaba peor.

**El estado que NO va en Query** y vive en Rust, expuesto por eventos: la salud de cada servidor,
el registro de tareas en marcha, y el anillo de logs. Son cosas que sobreviven a la pantalla y que
no son «una respuesta a una petición».

### 5.2 La jugada que ahorra la mitad de las llamadas, ahora verificada

En la ronda 1 esto era una lectura del emisor. Ahora está **comprobado ejecutando los dos comandos
sobre el mismo banco de 40 apps y comparando los objetos con `==`**:

```
claves de status --json : ['schema', 'host', 'services', 'apps']
status.apps: 40 elementos     list.apps: 40 elementos
¿arrays idénticos?      : True
```

No es «parecido» ni «un superconjunto»: es **el mismo array, elemento a elemento**. Y en el código
se ve por qué no puede ser otra cosa: `_status_json` (11508) termina con
`printf '],"apps":%s}\n' "$(_apps_json)"`, y `_list_json` (8489) es
`printf '{"schema":%s,"apps":%s}\n' "$JSON_SCHEMA" "$(_apps_json)"`. **La misma función.**

Con las cifras de §5.0, la aritmética:

```
dos llamadas   list(314) + status(388) = 702 ms   +2 canales
una llamada    status(388)             = 388 ms   +1 canal
                                        ────────
                                        45 % menos
```

Por lo tanto: **la portada pide `status --json` y de ahí alimenta las dos cachés**. `list --json`
se pide sólo cuando se quiere refrescar la lista **sin** volver a leer `free`, `df` y los cinco
`systemctl` del host — que son los 74 ms de diferencia.

```ts
const status = createQuery(() => ({ queryKey: qk.status(s), queryFn: () => api.status(s) }));
// y al llegar:
queryClient.setQueryData(qk.list(s), { schema: data.schema, apps: data.apps });
```

Hay una condición que hay que vigilar y que se prueba: **si algún día `status` dejara de traer
`apps`, esta optimización pintaría una lista vacía.** Por eso el alimentador comprueba que
`data.apps` existe y es una lista antes de escribir la caché de `list`; si no lo es, no escribe
nada y `list` se pide aparte. Es P-42 aplicado a nuestro propio atajo.

### 5.3 TTL por naturaleza del dato, y ahora también por coste

`staleTime` se elige por **cuánto tarda el dato en dejar de ser cierto sin que yo lo haya
provocado**. Pero con las cifras de §5.0 hay un segundo criterio que en la ronda 1 faltaba:
**cuánto cuesta volver a pedirlo**. Un dato que cuesta 2 s no puede tener el mismo `staleTime`
que uno que cuesta 85 ms aunque envejezcan igual de rápido.

| Consulta | Coste medido | `staleTime` | `gcTime` | Razón |
|---|---|---|---|---|
| `version` | 76 ms | ∞ por sesión de servidor | ∞ | Sólo cambia si alguien actualiza `orbit` |
| `status` | 388 ms | 20 s | 5 min | Alguien puede desplegar desde su terminal |
| `list` | 314 ms | 20 s | 5 min | Se alimenta de `status` (§5.2); rara vez se pide sola |
| `info` | 85 ms | 30 s | 5 min | Barata: se puede refrescar sin pensarlo |
| `env list` | ~90 ms | 5 min | 30 min | Casi nunca cambia sin que lo hagas tú |
| `db list` | ~90 ms + psql | 5 min | 30 min | Idem |
| `redirect list` | ~300 ms | 5 min | 30 min | Recorre las 40 apps |
| `queue status` / `watch status` | ~100 ms | 60 s | 10 min | Los mueve un timer de systemd |
| `metrics` | **936 ms** | **5 min** | 30 min | Sube de 60 s a 5 min: cuesta casi un segundo y sólo cambia al desplegar — y **un despliegue la invalida explícitamente** (§5.5), así que el TTL sólo cubre los despliegues hechos desde fuera |
| `doctor` | **1,4 s** | **∞, sólo bajo demanda** | 30 min | Nunca se dispara sola |
| `traffic` | **2 s+** | **10 min** por ventana | 30 min | La ventana forma parte de la clave |
| `top` | **2,1 s** | 0 | 0 | Es una medición instantánea. Cachearla es mentir |

Dos cambios respecto a la ronda 1, los dos por las medidas:

- **`metrics` sube de 60 s a 5 minutos.** A 936 ms por llamada, un `staleTime` de un minuto
  significa que abrir la pestaña de métricas seis veces en una sesión cuesta seis segundos de
  espera para ver el mismo número. Y el dato sólo cambia cuando hay un despliegue, que ya invalida.
- **`traffic` sube de 5 a 10 minutos.** Por lo mismo, multiplicado por dos.

**Windows** (sin multiplexado, §3.7): el coste de cada llamada sube en el apretón de manos completo
—**246 ms medidos en localhost**, y más con red—. Todos los `staleTime` se multiplican por 2 salvo
`top`. Es **una constante** en la configuración del cliente (`TTL_SCALE`), no un `if` repartido.

### 5.4 Polling: sólo donde el usuario está mirando, y a la velocidad que el servidor puede

**Ronda 2: esta sección estaba mal y la medición lo demuestra.** La ronda 1 decía «`top` cada 2 s»,
copiando el `TOP_INTERVAL=2` del servidor (línea 318). Pero `TOP_INTERVAL` es el intervalo del
panel **en vivo** de `orbit top`, que reutiliza la muestra del fotograma anterior y por eso no paga
el `sleep 1`. **`top --json` sí lo paga, siempre** (líneas 8756-8761: toma una muestra, duerme un
segundo, mide otra vez). Medido: **2.116 ms con 40 apps**, ≈ 1.053 ms + 26,6 ms por app.

Un `refetchInterval` de 2 s contra un comando que tarda 2,1 s no es «apretado»: es imposible.
TanStack Query no encola —no lanza otra petición hasta que vuelve la anterior— así que el resultado
no sería una avalancha, sino algo peor de diagnosticar: **un cliente que cree refrescar cada 2 s y
refresca cada 2,2, con un servidor ejecutando `orbit top` sin parar y un `orbit` corriendo el
100 % del tiempo en el VPS del usuario**. Eso es exactamente lo que el principio 2 de Orbit dice
que el cliente no puede hacer.

**El intervalo se mide, no se fija.** El cliente cronometra cada muestra y calcula el siguiente
periodo:

```ts
// periodo = max(suelo, 1,5 × duración de la última muestra), acotado.
const FLOOR_MS = 2_000, CEIL_MS = 15_000;
let period = FLOOR_MS;
function onSample(durationMs: number) {
  period = Math.min(CEIL_MS, Math.max(FLOOR_MS, Math.round(durationMs * 1.5)));
}
```

Con 5 apps (~1,2 s) el periodo es el suelo de 2 s, que es lo que la gente espera de un monitor.
Con 40 apps (~2,1 s) sube solo a ~3,2 s. Con 100 apps (~3,7 s proyectado) a ~5,5 s. **El factor
1,5 deja al servidor un 33 % del tiempo sin `orbit` corriendo**, que es la diferencia entre
«monitorizar» y «ocupar».

Y se **enseña**: el rótulo del monitor dice «cada 3,2 s · la medición de 40 apps tarda 2,1 s en el
servidor». Un intervalo que cambia solo y no se explica parece un fallo. Y ese rótulo hace visible
algo verdadero y útil: mirar el monitor cuesta trabajo al servidor, y cuantas más apps, más.

**Las tres condiciones para que haya polling, y las tres son necesarias:**

```ts
refetchInterval: visible && focused && healthy ? period : false
```

- **`visible`**: la pantalla del monitor está montada. Con `IntersectionObserver` sobre el
  contenedor, no con la ruta: una pestaña oculta detrás de otra no cuenta.
- **`focused`**: la ventana tiene el foco. Un cliente que mide 40 apps mientras el usuario está en
  otra aplicación calienta un VPS por nada. Al recuperar el foco se hace una muestra inmediata.
- **`healthy`**: el servidor está en `Ready`. Un servidor caído no se sondea cada 3 s.

**No hay ningún otro polling en toda la aplicación.** Ni un `list` de fondo, ni un `doctor`
periódico, ni un sondeo de salud por temporizador. El refresco se dispara por tres cosas y sólo
tres: el usuario abre una pantalla, el usuario pulsa «refrescar», o una mutación invalida algo
(§5.5). Es la traducción al cliente del principio de que el servidor no gana un solo proceso: **con
Orbit Desktop abierto y el usuario sin hacer nada, el servidor no se entera de que existe.**

Una concesión, y sólo una: **`refetchOnWindowFocus`** activo para `status`/`list`/`info`, con sus
`staleTime` de 20-30 s. Volver a la aplicación después de un rato y ver datos de hace media hora es
peor que una llamada de 388 ms.

### 5.5 Invalidación tras mutación: de disciplina a mecanismo

En la ronda 1 esto era una tabla y una promesa de mantenerla. **Ésa era la debilidad nº 4 y aquí se
cierra**: la tabla se convierte en algo que **el compilador exige**, con el mismo remedio que
`orbit` aplicó a `ORBIT_APP_FIELDS` — una lista, no tres sitios.

**Paso 1 · `MutationKind` es una unión cerrada, derivada del constructor de comandos.**

```ts
// ui/src/api/mutations.ts
import type { OrbitCommand } from './generated';        // ← generado desde Rust por ts-rs

/** Las variantes de OrbitCommand que MUTAN. Se extrae del tipo generado, no se
 *  escribe a mano: si Rust añade una variante mutante, aparece aquí sola. */
export type MutationKind = Extract<
  OrbitCommand['kind'],
  | 'deploy' | 'deployAll' | 'rollback' | 'restart' | 'start' | 'stop'
  | 'maintenanceOn' | 'maintenanceOff' | 'envSet' | 'envUnset' | 'ssl' | 'port'
  | 'redirectAdd' | 'redirectRm' | 'newApp' | 'removeApp' | 'isolate'
  | 'dbCreate' | 'doctorFix' | 'queueEnable' | 'queueDisable'
  | 'autodeployEnable' | 'autodeployDisable' | 'nginxRebuild' | 'migrate' | 'clone'
  | 'restore' | 'backup'
>;
```

El `Extract` es la mitad importante y es lo que la ronda 1 no tenía: **si alguien añade una variante
mutante a `OrbitCommand` en Rust y no la lista aquí, `Extract` la deja fuera y el `Record` de abajo
sigue siendo exhaustivo sobre un conjunto incompleto** — o sea, el mecanismo no se dispararía. Se
cierra con una aserción de tipo que compara los dos conjuntos:

```ts
// Falla en compilación si OrbitCommand tiene una variante mutante que MutationKind no cubre.
type MutatingCommandKind = Extract<OrbitCommand, { mutates: true }>['kind'];
type _Exhaustive = MutatingCommandKind extends MutationKind ? true
  : ['FALTA en MutationKind:', Exclude<MutatingCommandKind, MutationKind>];
const _assert: _Exhaustive = true;   // ← el error del compilador nombra lo que falta
```

`mutates: true` no es un campo que haya que recordar poner: **lo pone `ts-rs` desde Rust**, porque
en Rust está declarado en el propio enum:

```rust
impl OrbitCommand {
    /// ¿Cambia algo en el servidor? Un `match` exhaustivo: añadir una variante
    /// sin contestar aquí no compila. Es la misma disciplina que ORBIT_APP_FIELDS.
    pub const fn mutates(&self) -> bool {
        use OrbitCommand::*;
        match self {
            Version | List | Info{..} | Status | Doctor | Top
            | Metrics{..} | Traffic{..} | EnvList{..} | DbList
            | RedirectList{..} | WatchStatus | QueueStatus | Logs{..} => false,
            Deploy{..} | DeployAll{..} | Rollback{..} | Restart{..} | Start{..}
            | Stop{..} | MaintenanceOn{..} | MaintenanceOff{..} | EnvSet{..}
            | EnvUnset{..} | Ssl{..} | Port{..} | RedirectAdd{..} | RedirectRm{..}
            | New{..} | Remove{..} | Isolate{..} | DbCreate{..} | DoctorFix
            | QueueEnable{..} | QueueDisable{..} | AutodeployEnable{..}
            | AutodeployDisable{..} | NginxRebuild | Migrate{..} | Clone{..}
            | Restore{..} | Backup{..} => true,
            Exec{..} => true,          // no se sabe qué hace: se asume lo peor
        }
    }
}
```

**Paso 2 · El `Record` exhaustivo.** `Record<MutationKind, InvalidationSet>` en TypeScript **obliga
a que estén todas las claves**. No es una convención: es la definición de `Record` sobre una unión
de literales.

```ts
type InvalidationSet = (s: ServerId, app?: AppName) => QueryKey[];

export const AFTER: Record<MutationKind, InvalidationSet> = {
  deploy:        (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!),
                           qk.metrics(s,a!), qk.metrics(s), qk.top(s)],
  deployAll:     (s)   => [['srv', s]],
  rollback:      (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.top(s)],
  restart:       (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.top(s)],
  start:         (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.top(s)],
  stop:          (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.top(s)],
  maintenanceOn: (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!)],
  maintenanceOff:(s,a) => [qk.list(s), qk.status(s), qk.info(s,a!)],
  envSet:        (s,a) => [qk.envKeys(s,a!)],
  envUnset:      (s,a) => [qk.envKeys(s,a!)],
  ssl:           (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.doctor(s)],
  port:          (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!), qk.doctor(s)],
  redirectAdd:   (s)   => [qk.redirects(s), qk.list(s), qk.status(s)],
  redirectRm:    (s)   => [qk.redirects(s), qk.list(s), qk.status(s)],
  newApp:        (s)   => [['srv', s]],
  removeApp:     (s)   => [['srv', s]],
  isolate:       (s,a) => [qk.info(s,a!), qk.list(s), qk.status(s), qk.doctor(s)],
  dbCreate:      (s,a) => [qk.databases(s), qk.envKeys(s,a!)],
  doctorFix:     (s)   => [['srv', s]],
  queueEnable:   (s,a) => [qk.queue(s), qk.list(s), qk.status(s), qk.info(s,a!)],
  queueDisable:  (s,a) => [qk.queue(s), qk.list(s), qk.status(s), qk.info(s,a!)],
  autodeployEnable:  (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!)],
  autodeployDisable: (s,a) => [qk.list(s), qk.status(s), qk.info(s,a!)],
  nginxRebuild:  (s)   => [['srv', s]],
  migrate:       (s,a) => [qk.info(s,a!)],
  clone:         (s)   => [['srv', s]],
  restore:       (s)   => [['srv', s]],
  backup:        (s)   => [qk.backups(s)],
};
```

**Paso 3 · La demostración de que funciona.** Se añade a Rust una variante nueva:

```rust
    /// Nueva en la v0.7: pone o quita el permiso de autodespliegue de golpe.
    AutodeployEvery { minutes: u16 },
```

…se contesta `true` en `mutates()` (si no, ese `match` ya no compila y el error es
`error[E0004]: non-exhaustive patterns: 'AutodeployEvery { .. }' not covered`), se regenera
`generated.ts`, y **sin tocar nada más `pnpm exec tsc` falla con esto**:

```
ui/src/api/mutations.ts:41:14 - error TS2739: Type '{ deploy: …; deployAll: …; … }'
  is missing the following properties from type
  'Record<MutationKind, InvalidationSet>': autodeployEvery

41 export const AFTER: Record<MutationKind, InvalidationSet> = {
                ~~~~~
```

Y si alguien intentara «arreglarlo» quitándola de `MutationKind` en lugar de declarar su
invalidación, salta la otra aserción:

```
ui/src/api/mutations.ts:23:7 - error TS2322: Type 'boolean' is not assignable to type
  '["FALTA en MutationKind:", "autodeployEvery"]'.

23 const _assert: _Exhaustive = true;
         ~~~~~~~
```

**El error nombra la mutación que falta.** Eso es lo que separa un mecanismo de una convención: no
hay que saber que la tabla existe, ni acordarse de mirarla; el compilador la nombra y dice qué
falta.

**Paso 4 · Nadie invalida fuera de aquí.** Un solo punto de entrada, y una regla de lint que impide
el resto (§7.1):

```ts
export function orbitMutation<K extends MutationKind>(kind: K, server: ServerId) {
  const qc = useQueryClient();          // @tanstack/svelte-query
  return createMutation(() => ({
    mutationFn: (args: ArgsFor<K>) => api.run(server, buildCommand(kind, args)),
    onSettled: (_d, _e, args) =>
      Promise.all(AFTER[kind](server, args.app).map(k => qc.invalidateQueries({ queryKey: k }))),
  }));
}
```

`onSettled` y no `onSuccess`: **una mutación que falla también puede haber cambiado el estado**.
`orbit deploy` que falla en el paso `service` ha creado una release, ha movido el symlink y puede
haber hecho rollback (§1.5). Invalidar sólo en el éxito dejaría la interfaz enseñando el estado de
antes de un fallo que sí tocó cosas. Es exactamente lo que `orbit` hace en `cmd_doctor` (12157):
volver a diagnosticar pase lo que pase, porque «lo que cuenta es cómo queda el servidor, no lo que
dijeron los comandos».

**Tres decisiones dentro de la tabla que no son obvias, y siguen valiendo:**

- **`deploy` invalida `metrics` dos veces**: la de la app y la global (`orbit metrics --json` sin
  app son dos entradas de caché distintas). Y ahora que sabemos que `metrics` cuesta 936 ms, la
  invalidación importa más: sin ella, el `staleTime` de 5 minutos dejaría la pantalla de métricas
  mostrando el despliegue anterior durante cinco minutos.
- **`envSet` NO invalida `list` ni `info`.** Escribir en el `.env` no cambia el estado observado:
  no reinicia nada salvo que se pase `--restart`, y entonces la mutación es otra.
- **`redirectAdd` invalida `list`.** Una redirección de dominio entero **registra una app** de tipo
  `redirect` (`_redirect_add_domain`), y aparece en `orbit list`. Sin esto, el usuario añade una
  redirección y no la ve.

### 5.6 Optimistic updates: dónde sí, dónde no, y la línea

**La línea es esta: se permite pintar por adelantado un cambio que el usuario puede deshacer
mirando, y nunca uno que le haría creer que algo destructivo ya ocurrió.**

**SÍ**, con reversión en el `onError`:

| Acción | Qué se pinta antes de tiempo | Por qué es seguro |
|---|---|---|
| `maintenance on/off` | `state.maintenance` cambia al instante | Es un fichero testigo. Reversible, visible, y el error se ve en 1 s |
| `autodeploy enable/disable` | `state.autodeploy` | Cambia una clave del `.conf`. Sin efecto inmediato |
| `queue enable/disable` | `state.queue` | Idem |
| Filtros, orden, pestaña, ventana de `traffic` | — | Estado local, no toca el servidor |

**NO, nunca, bajo ninguna circunstancia:**

| Acción | Por qué no |
|---|---|
| `deploy` | Tarda minutos y el resultado tiene siete formas (`ok`, `rolled_back`, `recovered`, `failed_step`…). Pintar «desplegado» y luego corregir a «revertido» es peor que esperar. **Se pinta la tarea con su progreso real**, que es información de verdad |
| `rollback` | Cambia qué está sirviendo. Pintar la release nueva antes de que el `ln -sfn` haya ocurrido es afirmar sobre producción |
| `remove` / `remove --purge` | Destructivo. La fila **no** desaparece hasta que el comando vuelve con 0. Y `--purge` además pide confirmación escribiendo el nombre, igual que hace `orbit` (11324) |
| `start` / `stop` / `restart` | Es un `systemctl`. Puede fallar y la app quedarse parada. Pintar «activa» sobre una app caída es la mentira que más caro sale |
| `ssl` | Llama a Let's Encrypt, que tiene límites de frecuencia y falla con relativa facilidad. Pintar el candado antes es prometer HTTPS |
| `port` | Reinicia el servicio y puede revertirse solo si nginx rechaza (10920-10923). El estado final no es predecible |
| `db create`, `migrate`, `restore`, `isolate`, `doctor --fix` | Todos tocan cosas que no se deshacen mirando |

**El patrón para todo lo que dice NO**: botón → estado `pending` en el propio botón (spinner, no
diálogo modal) → al volver, invalidación y refresco. Entre 250 ms y 3 s de espera con el botón
ocupado es aceptable y es honesto. Para lo que tarda minutos, el sistema de tareas de §3.5.

**Y una regla de interfaz que se deriva de todo esto:** cuando una mutación vuelve, **no se
actualiza la fila con lo que el cliente cree que pasó: se invalida y se relee**. `orbit` mismo hace
esto en `doctor --fix` (12157-12158, «se vuelve a diagnosticar: lo que cuenta es cómo queda el
servidor, no lo que dijeron los comandos») y en `_new_undeployed` (mira el disco, no el código de
salida). Es la disciplina de la casa y el cliente la hereda.

---

## 6. Multi-servidor

### 6.0 Lo que ya está resuelto y lo que no

El BRIEF lo dice: «un cliente que habla SSH con varios servidores **es** el `orbit remote add` de la
v2.0, sin plano de control». Es cierto, y por eso la parte difícil del multiservidor —el estado
compartido— no existe: **no hay estado compartido**. Cada servidor es independiente, no se sincroniza
nada, y una app de un servidor no sabe de las de otro.

Lo que sí hay que resolver es más humilde y más concreto: guardar la lista, importarla del sitio
donde el usuario ya la tiene, y no matar el portátil ni los servidores cuando se enseñan todos a la
vez.

### 6.1 Cómo se guardan los servidores

**Lo que se guarda es un puntero a `~/.ssh/config`, no una copia de la configuración SSH.**

```rust
#[derive(Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: ServerId,                  // uuid v7, estable, no depende del nombre
    pub label: String,                 // lo que ve el usuario. Editable.
    pub target: SshTarget,
    pub color: Option<String>,         // para distinguir producción de staging de un vistazo
    pub tags: Vec<String>,
    pub added: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
    pub last_contract: Option<u32>,
    pub last_version: Option<String>,
    pub pinned_apps: Vec<String>,      // favoritas, para la portada
}

#[derive(Serialize, Deserialize)]
pub enum SshTarget {
    /// Lo normal y lo preferido: un alias de ~/.ssh/config. Todo lo demás lo pone OpenSSH.
    Alias { host: String },
    /// Para quien no quiere tocar su config. Se traduce a -o en la línea de órdenes.
    Explicit { host: String, user: Option<String>, port: Option<u16>,
               identity_file: Option<PathBuf>, proxy_jump: Option<String> },
}
```

Fichero: `<config_dir>/orbit-desktop/servers.json` (`~/.config/…` en Linux,
`~/Library/Application Support/…` en macOS, `%APPDATA%\…` en Windows), permisos `0600` en Unix.

**Lo que NO se guarda ahí, y esto es una regla de seguridad, no de estilo:** ninguna clave privada,
ninguna passphrase, ningún `known_hosts` propio. Todo eso es de OpenSSH y se queda en OpenSSH. Lo
único que puede llegar al llavero del sistema es la contraseña de `sudo`, si el usuario elige
guardarla, y con un aviso que dice lo que es: la contraseña que abre root en ese servidor.

**Por qué `Alias` es lo preferido y `Explicit` la excepción:** porque `Alias` hereda `ProxyJump`,
`IdentityAgent`, `Match`, `Include` y todo lo que el usuario ya tenga. `Explicit` sólo cubre lo que
hayamos previsto. La interfaz de alta empuja hacia `Alias` y ofrece, cuando el usuario mete un host
suelto, un botón «Añadir a ~/.ssh/config» que **escribe un bloque bien formado y comentado** en el
fichero del usuario. Escribir en `~/.ssh/config` sí es legítimo: es el fichero del usuario, en su
portátil, y es donde esa información pertenece.

### 6.2 Importar de `~/.ssh/config`

Se parsea el fichero **sólo para sugerir**, nunca para ejecutar. La ejecución siempre la hace `ssh`
con el alias, así que un parser incompleto no puede producir una conexión incorrecta: como mucho,
produce una sugerencia de menos.

El parser (crate `ssh_config` o unas 150 líneas propias) hace:

- Resolver `Include` (relativo a `~/.ssh/`), con un tope de profundidad.
- Listar los bloques `Host` cuyo patrón **no** tenga comodines (`*`, `?`, `!`): un `Host *` no es un
  servidor, es una política.
- Extraer `HostName`, `User`, `Port`, `ProxyJump` para enseñarlos en la lista.
- Ignorar `Match` por completo, y decirlo: los bloques con `Match` no se sugieren.

La interfaz enseña la lista con casillas y un botón «Comprobar los seleccionados», que lanza
`orbit --json version` **con concurrencia acotada** (§6.3) y marca cada uno con: Orbit 1.3.6 ✓,
sin Orbit, sin permisos, o no responde. Sólo los que tienen Orbit se pueden añadir; los demás se
enseñan con su motivo, porque «no aparece en la lista» es peor que «aparece y dice por qué no».

**Lo que no se hace:** escanear la red, probar puertos, o leer `~/.ssh/known_hosts` para sugerir
hosts. Un cliente que abre conexiones que el usuario no ha pedido es un cliente que acaba en el
registro de fail2ban de alguien.

### 6.3 Cuarenta servidores sin abrir cuarenta conexiones

**Tres límites, y los tres hacen falta:**

1. **Un semáforo global de conexiones nuevas: 6.** No de comandos: de **handshakes**. Con
   multiplexado, la primera llamada a un servidor abre la conexión y las siguientes la reutilizan,
   así que lo que hay que acotar es el arranque. Seis simultáneos es lo que aguanta un portátil y una
   línea doméstica sin que el `ssh-agent` se convierta en el cuello de botella (cada handshake le
   pide una firma).
2. **Un semáforo por servidor: 2.** Un `deploy` de tres minutos no debe impedir que el usuario mire
   `info` de otra app del mismo servidor, pero cuatro comandos a la vez sobre el mismo `orbit` es
   pedirle a un script de Bash que se pise a sí mismo (y `orbit` tiene `flock` sólo en watch y en
   queue, no en `deploy`). Dos: uno «pesado» y uno «ligero».
3. **Sólo se sondea lo que se ve.** La portada multiservidor no consulta cuarenta servidores: usa
   `IntersectionObserver` y consulta **las tarjetas visibles**, en una cola con el semáforo de 6.
   Las que están fuera de la ventana se quedan en «—» hasta que se llega a ellas.

**Y la decisión de fondo, que es la que evita el problema entero: la portada multiservidor es una
lista de servidores, no un panel agregado.** Cada tarjeta enseña: nombre, color, estado (verde /
ámbar / rojo / gris), versión de Orbit, número de apps, y **cuántas tienen `served: false`** —el dato
más accionable del contrato—. Eso es **una** llamada por servidor (`status --json`, que trae host y
apps), y sólo de los visibles.

Lo que **no** hace la portada: sumar CPU de cuarenta servidores, ni pintar una gráfica agregada, ni
un «total de apps caídas» global. Eso exigiría consultarlos todos, todo el rato, y sería un plano de
control con otro nombre. Si el usuario quiere el detalle de un servidor, entra en él.

**El refresco de la portada** es manual (botón «Actualizar todos»), con el mismo semáforo, y con
un aviso honesto: «Comprobando 12 de 40». Nada de refresco automático.

### 6.4 Un servidor caído no degrada nada

Éste es el requisito explícito del BRIEF y se cumple con tres mecanismos:

**a) Aislamiento de fallos por `Health`.** Un `TransportError` en el servidor A no toca las cachés
de B. Las claves de Query están prefijadas por `ServerId` desde el primer día justo para esto.

**b) El retroceso, y un límite duro al ruido.** Tras cinco fallos consecutivos, el servidor pasa a
`Unreachable` con `retry_at` cada 5 minutos y **la interfaz deja de intentarlo sola**. La tarjeta se
pone gris con «No responde desde las 10:32» y un botón «Reintentar». Un cliente que sigue golpeando
un servidor caído cada dos segundos consume batería y llena el `auth.log` de alguien.

**c) La interfaz nunca espera a nadie.** Cada tarjeta tiene su propio estado de carga. La portada se
pinta entera de inmediato con esqueletos y se va rellenando. **No hay ningún `Promise.all` que espere
a todos**: si un servidor tarda 30 s en dar timeout, los otros 39 ya están pintados hace 29.

**d) Los datos viejos se enseñan, marcados como viejos.** Si el servidor A tenía datos en caché y
ahora no responde, se siguen enseñando **atenuados**, con una banda: «Última lectura hace 6 minutos.
El servidor no responde.» Es más útil que una pantalla vacía y es honesto porque lo dice. Lo que no
se permite es actuar sobre datos viejos: **con el servidor en `Unreachable`, todos los botones de
mutación están deshabilitados**, no porque fueran a fallar (fallarían, y eso no es grave) sino
porque el estado sobre el que el usuario decidiría puede ser falso.

---

## 7. Estructura del repositorio

### 7.0 La separación, y por qué es ésa

Tres capas, y el criterio para separarlas es **qué puede probarse sin qué**:

- `crates/orbit-client` — el contrato y el transporte. Se prueba con `cargo test` **sin ventana y
  sin servidor**, contra el `orbit` falso de §9.3. Es el 30 % del código y el 80 % de los bugs
  posibles, así que es donde va el 80 % de las pruebas.
- `crates/orbit-app` — la aplicación Tauri: comandos, eventos, tareas, almacén de servidores,
  llavero. Depende de `orbit-client`, y no al revés. Es delgada a propósito.
- `ui/` — el frontend. No sabe qué es SSH. Sólo conoce los comandos de Tauri y los tipos generados.

Esa dependencia en una sola dirección es lo que permite el `orbit-desktop-tui` de §2.4 sin duplicar
nada, y —más importante— lo que permite que el parser del contrato se pruebe a fondo sin arrancar
una interfaz.

```
orbit-desktop/
├─ Cargo.toml                       # workspace
├─ package.json                     # workspace de pnpm
├─ pnpm-workspace.yaml
├─ rust-toolchain.toml              # versión de Rust fijada
├─ .github/workflows/
│  ├─ ci.yml                        # lint + test, las 3 plataformas
│  └─ release.yml                   # build + firma + notarización + publicación
├─ crates/
│  ├─ orbit-client/
│  │  ├─ src/
│  │  │  ├─ lib.rs
│  │  │  ├─ contract/               # ← el contrato, y nada más
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ version.rs           # VersionResponse, Compatibility
│  │  │  │  ├─ app.rs               # AppState, AppSummary, AppConfig, InfoResponse
│  │  │  │  ├─ status.rs
│  │  │  │  ├─ doctor.rs
│  │  │  │  ├─ top.rs
│  │  │  │  ├─ deploy.rs            # DeployResult, DeployAll*, ProgressEvent
│  │  │  │  ├─ metrics.rs
│  │  │  │  ├─ traffic.rs
│  │  │  │  ├─ misc.rs              # env, db, redirect, watch, queue
│  │  │  │  └─ text.rs              # los parsers de TEXTO. Aislados a propósito. §7.1
│  │  │  ├─ transport/
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ ssh.rs               # construcción de argv, ControlMaster, plataforma
│  │  │  │  ├─ exec.rs              # run(): dos tuberías, join, watchdog
│  │  │  │  ├─ error.rs             # TransportError vs OrbitError, clasificación
│  │  │  │  └─ askpass.rs
│  │  │  ├─ client.rs               # OrbitClient: salud, semáforos, reconexión
│  │  │  └─ command.rs              # el constructor de órdenes tipado. §7.2
│  │  └─ tests/
│  │     ├─ fixtures/               # salidas REALES de orbit, capturadas
│  │     ├─ contract_parse.rs
│  │     ├─ argv.rs
│  │     └─ fake_orbit.rs
│  └─ orbit-app/
│     ├─ src/
│     │  ├─ main.rs
│     │  ├─ commands/               # #[tauri::command], uno por área
│     │  ├─ tasks.rs                # el registro de tareas largas
│     │  ├─ servers.rs              # servers.json, import de ~/.ssh/config
│     │  ├─ secrets.rs              # keyring
│     │  └─ events.rs
│     ├─ tauri.conf.json
│     ├─ capabilities/default.json  # permisos de Tauri v2, mínimos
│     └─ icons/
├─ ui/                              # Svelte 5 + TS. Ver §2.5.
│  ├─ src/
│  │  ├─ main.ts
│  │  ├─ App.svelte                 # shell: barra de servidores + ruta actual ($state)
│  │  ├─ api/                       # ← ÚNICO sitio que llama a invoke(). Regla R-04.
│  │  │  ├─ generated.ts            # ← ts-rs. NO se edita a mano (R-06)
│  │  │  ├─ invoke.ts               # envoltura tipada de invoke()
│  │  │  ├─ queries.ts              # createQuery por consulta, con su staleTime de §5.3
│  │  │  ├─ mutations.ts            # MutationKind + Record AFTER exhaustivo. §5.5
│  │  │  └─ keys.ts
│  │  ├─ features/                  # una carpeta por pantalla, con sus .svelte
│  │  │  ├─ servers/  apps/  app-detail/  deploy/  logs/  doctor/
│  │  │  ├─ top/  traffic/  metrics/  env/  redirects/  backups/  terminal/
│  │  ├─ components/                # tontos, sin lógica de datos, sin invoke()
│  │  ├─ charts/                    # SVG a mano. d3-scale + d3-shape. §7.4
│  │  ├─ styles/
│  │  │  ├─ tokens.css              # los ~60 tokens, los dos temas. §3.3 de UX
│  │  │  └─ base.css
│  │  ├─ lib/
│  │  │  ├─ format.ts               # bytes, duraciones, el "AAAAMMDDHHMMSS" de traffic
│  │  │  ├─ appState.ts             # served > maintenance > service. Una sola vez. §7.3
│  │  │  └─ i18n.ts
│  │  └─ locales/{es,en}.json
│  ├─ index.html
│  └─ vite.config.ts
├─ docs/                            # la biblia. En español.
│  ├─ ARCHITECTURE.md  CONTRACT.md  DESIGN.md  THREAT-MODEL.md  QA.md  ROADMAP.md
└─ tools/
   └─ fake-orbit/                   # el orbit de mentira. §9.3
```

### 7.1 Las reglas, escritas como reglas ejecutables

**Ronda 2: en la ronda 1 esto eran «convenciones». Una convención es una intención, y las
intenciones se erosionan en el commit 400** — es literalmente lo que dice §2 del informe de QA.
Aquí van como ficheros que fallan el build. Las tres primeras cubren SEC-01, SEC-04 y SEC-14 del
informe de QA, que también las pide verificables.

**R-01 · Ningún parser de texto fuera de `contract/text.rs`.** (Es la cuarentena de §7.1 de la
ronda 1, ahora con mecanismo.)

```yaml
# .semgrep/orbit.yml
rules:
  - id: orbit-text-parsing-outside-quarantine
    languages: [rust]
    severity: ERROR
    message: >-
      Parsear texto de la salida de `orbit` sólo se permite en
      crates/orbit-client/src/contract/text.rs, que es la cuarentena: ahí cada
      función lleva su fixture y el comando de `orbit` que la haría innecesaria
      si tuviera --json. Ver REPORT-CODE §1.4 y §9b.
    paths:
      include: ["crates/**/*.rs"]
      exclude: ["crates/orbit-client/src/contract/text.rs"]
    patterns:
      - pattern-either:
          - pattern: $S.lines()
          - pattern: $S.split_whitespace()
          - pattern: $S.split($SEP)
          - pattern: $S.splitn($N, $SEP)
          - pattern: regex::Regex::new($RE)
      - pattern-not-inside: |
          fn $F(...) { ... }   // marcado con #[allow(orbit_text_parsing)]
```

La versión que de verdad se ejecuta usa además un `clippy.toml` con
`disallowed-methods` para `str::split_whitespace` y `str::lines`, y un `#[allow]` con nombre propio
que sólo existe en `text.rs`. El módulo empieza así, y ese comentario es parte de la regla:

```rust
//! CUARENTENA. Todo lo que hay aquí existe porque un comando de `orbit` no tiene `--json`.
//! Cada función nombra el comando y qué JSON lo haría innecesario (§9b).
//! Cuando ese `--json` exista, la función se BORRA. No se mantiene "por compatibilidad".
//!
//!   parse_backup_list  ← `orbit backup list`   · sustituible por §9b-2
//!   parse_backup_verify ← `orbit backup verify` · sustituible por §9b-2
//!
//! Hoy son dos. Si algún día son cinco, el problema no es este fichero.
```

**R-02 · Ninguna cadena de comando fuera de `transport/`.** (SEC-01 y SEC-04.)

```yaml
  - id: orbit-command-string-outside-transport
    languages: [rust]
    severity: ERROR
    message: >-
      El binario `ssh` y el nombre `orbit` sólo se mencionan en
      crates/orbit-client/src/transport/. Toda orden se expresa como Vec<String>
      y se serializa en un único punto (SEC-01). Ver §7.2.
    paths:
      include: ["crates/**/*.rs", "ui/**/*.ts", "ui/**/*.svelte"]
      exclude:
        - "crates/orbit-client/src/transport/**"
        - "crates/orbit-client/src/command.rs"
    patterns:
      - pattern-either:
          - pattern-regex: '"[^"]*\bssh\b[^"]*"'
          - pattern-regex: '"[^"]*/usr/local/bin/orbit[^"]*"'
          - pattern-regex: 'format!\("[^"]*orbit '
          - pattern: Command::new($X)
          - pattern: std::process::Command::new($X)
```

**R-03 · Ningún dato del contrato se inserta como marcado.** (SEC-14.)

```js
// eslint.config.js — fragmento
{
  files: ['ui/**/*.{ts,svelte}'],
  rules: {
    'no-restricted-syntax': ['error',
      { selector: "MemberExpression[property.name='innerHTML']",
        message: 'SEC-14: prohibido innerHTML. Un nombre de app con </script><img onerror> tiene que verse literalmente.' },
      { selector: "MemberExpression[property.name='outerHTML']", message: 'SEC-14.' },
      { selector: "CallExpression[callee.property.name='insertAdjacentHTML']", message: 'SEC-14.' },
      { selector: "CallExpression[callee.object.name='document'][callee.property.name='write']", message: 'SEC-14.' },
    ],
    'svelte/no-at-html-tags': 'error',        // {@html …} prohibido, sin excepciones
  },
}
```

`svelte/no-at-html-tags` es la que cierra el agujero real en Svelte: `{@html x}` es el equivalente
exacto de `dangerouslySetInnerHTML`, y a diferencia de éste **no lleva la palabra «peligroso» en el
nombre**, así que la regla de lint importa más, no menos.

**R-04 · Ningún componente llama a `invoke()`.** Sólo `ui/src/api/`.

```js
{ files: ['ui/src/{features,components}/**'],
  rules: { 'no-restricted-imports': ['error', { paths: [
    { name: '@tauri-apps/api/core', message: 'Sólo ui/src/api/ habla con el backend. Ver §7.1 R-04.' },
    { name: '@tauri-apps/api/event', message: 'Idem.' } ] }] } }
```

Es lo que hace que la tabla `AFTER` de §5.5 sea completa: si una mutación pudiera lanzarse desde
cualquier sitio, la tabla sería una sugerencia.

**R-05 · Prohibido aplastar un `null` del contrato.** (SEC-18, y el hallazgo de §4.3.)

```js
{ files: ['ui/**/*.{ts,svelte}'],
  rules: { '@typescript-eslint/no-unnecessary-condition': 'off',
           'local/no-nullish-default-on-contract': 'error' } }
```

La regla propia (unas 40 líneas) recorre el AST buscando `??` y `||` cuyo lado izquierdo tenga un
tipo procedente de `api/generated.ts` y cuyo lado derecho sea `0`, `''`, `false` o `'—'`. Es la
única de las cinco que hay que escribir; las otras cuatro son configuración. Y es la que más vale,
porque **el fallo que persigue no es una caída: es una mentira** (P2 del fuzzing de QA, §5b).

**R-06 · Los tipos generados no se editan.**

```
# .github/workflows/ci.yml
- run: cargo test -p orbit-client --test generate_ts
- run: git diff --exit-code ui/src/api/generated.ts
```

`generated.ts` lleva cabecera `// @generated`, está en `.prettierignore` y en `CODEOWNERS`
apuntando a nadie, de modo que cualquier cambio pide revisión explícita.

**Y una que no es de lint sino de proceso, porque no hay forma de automatizarla:** cada vez que se
toca `contract/`, el PR tiene que decir contra qué versión de `orbit` se ha comprobado. Lo pide la
plantilla de PR y lo verifica `check-contract.sh` (§9.1), que es la única de las siete que compara
con el servidor de verdad.

### 7.2 El constructor de órdenes tipado

Es la pieza que hace cumplir las reglas duras de §1.10 con el compilador en vez de con disciplina.

```rust
pub enum OrbitCommand {
    Version,
    List,
    Info { app: AppName },                     // AppName, no String, y no Option
    Status,
    Doctor,
    Top,
    Metrics { app: Option<AppName> },          // aquí Option SÍ: el contrato lo permite
    Traffic { app: Option<AppName>, since: Window, top: Option<u8> },
    Deploy { app: AppName, r#ref: Option<GitRef>, progress: bool },
    DeployAll { if_changed: bool, progress: bool },
    EnvList { app: AppName },
    EnvGet { app: AppName, key: EnvKey },      // EnvKey valida ^[A-Za-z_][A-Za-z0-9_]*$
    EnvSet { app: AppName, key: EnvKey, value: String, restart: bool },
    Logs { app: AppName, since: Option<Window>, lines: u32, mode: LogMode, follow: Follow },
    Rollback { app: AppName, release: ReleaseId },   // ReleaseId obligatorio: §13.5
    Remove { app: AppName, purge: bool },            // siempre lleva -y implícito
    New(NewSpec),
    Exec { app: AppName, argv: Vec<String> },
    // …
}

impl OrbitCommand {
    /// Construye argv. --json y --lang SIEMPRE delante del comando (§1.3).
    pub fn to_argv(&self, lang: Lang) -> Vec<String>;
    /// ¿Habla JSON? Espeja _json_capable + las guardas de subcomando.
    pub fn speaks_json(&self) -> bool;
}
```

`AppName` es un *newtype* que valida `^[a-z0-9][a-z0-9._-]{0,39}$` y rechaza `..`, la misma regla
que `_app_name_ok` (línea 1176). No es paranoia: **es lo que garantiza que ningún nombre de app
pueda ser una opción**. Sin él, una app llamada `--all` haría que `deploy <app>` se convirtiera en
`deploy --all`. Hoy `orbit` no permite crear esa app, pero un `.conf` escrito a mano sí, y el
cliente lee los nombres del servidor (SEC-06).

**Y la regla que el tipo hace cumplir, que es la que sale del hallazgo E5.** Verificado ejecutando:
sin TTY, `orbit info` sin app **elige `app01` en silencio y sale con 0** (§1.10). Lo mismo haría
`orbit restart`, que reiniciaría la primera app por orden alfabético sin que nada en la salida
diga que hubo una elección. Por eso:

> **Ninguna variante de `OrbitCommand` que actúe sobre una app tiene el nombre opcional.** Es
> `Info { app: AppName }`, no `Info { app: Option<AppName> }`. **No hay forma de expresar la orden
> peligrosa**, así que no hay revisión de código que pueda dejarla pasar.

Las tres excepciones donde `Option<AppName>` sí es correcto, y se ven de un vistazo porque son las
únicas: `Metrics`, `Traffic` y `RedirectList`, los tres comandos donde «sin app» significa «todas»
y el contrato lo dice explícitamente. `Logs` **no** está entre ellos, aunque `orbit logs` sin app
sea sintácticamente válido: elegiría la primera app y seguiría en vivo para siempre (§1.11).

**El escapado, y la lección que costó una prueba.** `to_argv` nunca concatena una cadena de shell:
devuelve un vector que se le pasa a `ssh` como argumentos separados. Pero `ssh` los une con espacios
para la shell remota, **así que sí hay una shell remota y hay que citar**.

La prueba de propiedad (E3, 28.464 viajes de ida y vuelta: `argv → escapar → shell → argv` es la
identidad) se ejecutó contra **cuatro** shells y **falló en la primera pasada, sólo en zsh**, 5
casos de 2.529. El escapador tenía un conjunto de caracteres «seguros» que se pasan sin comillas y
`=` estaba dentro; zsh expande las palabras que empiezan por `=` (opción `EQUALS`: `=ls` se
sustituye por la ruta de `ls`), así que `=Y` volvía como `zsh:1: Y not found`. **bash, dash y
busybox pasaban los 2.529.**

El arreglo no fue añadir `=` a una lista de prohibidos —eso deja la siguiente sorpresa esperando—
sino **estrechar el conjunto seguro a `[A-Za-z0-9_./-]` y entrecomillar todo lo demás**. La regla
que queda escrita en el fichero, encima de la función:

```rust
/// Conjunto seguro: [A-Za-z0-9_./-]. Todo lo demás va entre comillas simples,
/// con la comilla simple escapada como '\'' (igual que `_q()` de orbit, línea 1338).
///
/// NO SE AMPLÍA. Cada carácter que se añade aquí es una regla de expansión de
/// cuatro shells que hay que conocer: `=` costó 5 fallos en zsh y sólo en zsh.
/// Entrecomillar de más no cuesta nada; entrecomillar de menos cuesta un CVE.
const SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_./-";
```

Y el byte nulo **se rechaza, no se escapa**: no puede viajar dentro de un `argv` y fingir que sí es
peor que fallar.

Es el modo de fallo característico de esta clase de código —correcto en el shell donde se
desarrolla, roto en el que usa el usuario— y es el argumento entero a favor de que la prueba sea de
propiedad y contra varios shells, no una tabla de casos escritos a mano.

### 7.3 `appState.ts`: la precedencia de estados, escrita una vez

```ts
export type Display = 'no-vhost' | 'maintenance' | 'running' | 'stopped' | 'served' | 'redirect';

/** El orden es LEY y sale de cmd_list (orbit:8523-8556). No se reordena. */
export function displayState(a: AppSummary): Display {
  if (!a.state.served)        return 'no-vhost';    // gana sobre todo: nginx cierra la conexión
  if (a.state.maintenance)    return 'maintenance'; // gana sobre el servicio: nginx da 503 antes
  if (a.type === 'redirect')  return 'redirect';
  if (a.state.service === 'running') return 'running';
  if (a.state.service === 'stopped') return 'stopped';
  return 'served';            // static / php / laravel sanas: no tienen proceso
}
```

Ese orden es literalmente el de `cmd_list`, y el comentario del código (8534-8556) explica que
faltaba y que producía una contradicción entre la tabla humana y el `--json` del mismo comando.
Copiarlo aquí, con la cita, es la forma de que no se pierda.

### 7.4 Dependencias exactas

**`Cargo.toml` del workspace** (versiones de agosto de 2026; se fijan y las sube Dependabot):

```toml
[workspace]
members = ["crates/orbit-client", "crates/orbit-app"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.82"
license = "MIT"

[workspace.dependencies]
tokio        = { version = "1.47", features = ["rt-multi-thread","process","io-util","sync","time","macros"] }
serde        = { version = "1.0",  features = ["derive"] }
serde_json   = "1.0"
ts-rs        = { version = "10.1", features = ["serde-compat","chrono-impl"] }
thiserror    = "2.0"
anyhow       = "1.0"
tracing      = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter","json"] }
chrono       = { version = "0.4", features = ["serde"] }
uuid         = { version = "1.11", features = ["v7","serde"] }
regex        = "1.11"
dirs         = "5.0"
rustix       = { version = "0.38", features = ["fs","process"] }   # uid/modo del dir de control, §3.11
```

`crates/orbit-client/Cargo.toml`: lo de arriba y nada más. **Ninguna dependencia de SSH**, que es el
punto de §2.2. `dev-dependencies`: `insta = "1.41"` (snapshots), `proptest = "1.6"` (la propiedad
del escapador, E3), `assert_matches = "1.5"`, `tempfile = "3.14"`.

`crates/orbit-app/Cargo.toml`:

```toml
[dependencies]
orbit-client = { path = "../orbit-client" }
tauri  = { version = "2.9", features = ["tray-icon"] }
tauri-plugin-updater  = "2.9"
tauri-plugin-dialog   = "2.3"
tauri-plugin-opener   = "2.5"     # abrir una URL en el navegador. NO ejecuta comandos.
tauri-plugin-single-instance = "2.3"
keyring = "3.6"
```

Nótese que **`tauri-plugin-shell` ya no está**. En la ronda 1 se incluía «sólo para abrir en el
navegador»; `tauri-plugin-opener` hace eso mismo sin traer la capacidad de ejecutar procesos desde
el frontend. Quitar una API que no se usa es más barato que confiar en no usarla, y encaja con
SEC-24.

**`package.json`** — reescrito tras la decisión de §2.5. De 33 paquetes a 21, y el paquete de
partida de ~200 KB comprimidos a ~45:

```json
{
  "name": "orbit-desktop-ui",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "svelte-check --tsconfig ./tsconfig.json && vite build",
    "lint": "eslint . && prettier --check .",
    "test": "vitest run",
    "test:e2e": "playwright test",
    "tauri": "tauri"
  },
  "dependencies": {
    "svelte": "5.19.0",
    "@tanstack/svelte-query": "5.66.0",
    "@tanstack/svelte-virtual": "3.13.0",
    "@tauri-apps/api": "2.9.0",
    "@tauri-apps/plugin-updater": "2.9.0",
    "@tauri-apps/plugin-dialog": "2.3.0",
    "@tauri-apps/plugin-opener": "2.5.0",
    "melt": "0.20.0",
    "@xterm/xterm": "5.5.0",
    "@xterm/addon-fit": "0.10.0",
    "d3-scale": "4.0.2",
    "d3-shape": "3.2.0",
    "date-fns": "4.1.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "2.9.0",
    "@sveltejs/vite-plugin-svelte": "5.0.3",
    "svelte-check": "4.1.4",
    "typescript": "5.7.3",
    "vite": "6.1.0",
    "vitest": "3.0.5",
    "@testing-library/svelte": "5.2.6",
    "@playwright/test": "1.50.1",
    "eslint": "9.20.0",
    "eslint-plugin-svelte": "2.46.1",
    "typescript-eslint": "8.24.0",
    "prettier": "3.5.0",
    "prettier-plugin-svelte": "3.3.3"
  }
}
```

Seis elecciones que merecen una línea, y cuatro de ellas son **ausencias**:

- **No hay librería de estilos.** CSS puro con tokens y `<style>` con ámbito por componente, que es
  lo que Svelte da de serie. Sesenta tokens y veinticinco componentes no necesitan un motor de
  utilidades, y el DOM se lee: `class="app-row is-unserved"` explica por qué una fila está en rojo,
  una ristra de utilidades no. §2.5.
- **No hay librería de estado.** Las runas (`$state`, `$derived`) cubren lo local; el estado remoto
  es de `@tanstack/svelte-query`. Meter una tercera fuente de verdad es cómo se acaba con dos cachés.
- **No hay librería de gráficas.** `d3-scale` y `d3-shape` (7 KB entre las dos) y SVG a mano. Las
  cuatro gráficas del producto tienen que saber pintar **lo que no se sabe** —`cpu_percent: null`
  es un hueco, no un cero; `build_trend_s: null` es «no dibujes tendencia», no una línea plana— y
  ninguna librería genérica hace eso por defecto. Es el argumento de §8.4 del informe de UX y es
  correcto.
- **No hay router.** Doce pantallas y tres niveles (§2.4 de UX): un `$state` con la ruta actual y
  un `{#if}`. Un router es 20 KB para resolver un `switch`.
- **`melt`** para las cinco piezas donde la accesibilidad es difícil y equivocarse es caro: menú,
  diálogo, tooltip, popover y combobox. Gestión de foco, `aria-expanded`, cierre por `Esc`,
  colisiones de posicionamiento. Reescribirlas es perder dos semanas para acabar peor.
- **`@tanstack/svelte-virtual`**: la lista con 200 apps y el log con 5.000 líneas lo necesitan, y
  WebKitGTK es el que peor lleva un DOM grande.

## 8. Build, empaquetado y distribución

### 8.1 La matriz

| Plataforma | Objetivo | Artefacto | Firma |
|---|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | `.deb` + `.AppImage` | GPG del `.deb`; el `.AppImage` con la firma del actualizador |
| Linux arm64 | `aarch64-unknown-linux-gnu` | `.deb` + `.AppImage` | Idem |
| macOS universal | `universal-apple-darwin` | `.dmg` + `.app.tar.gz` | Developer ID Application + notarización + *staple* |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `.msi` (WiX) + `.exe` (NSIS) | Authenticode (Azure Trusted Signing) |

**macOS universal y no dos binarios**: `lipo` los une, el `.dmg` es uno solo y no hay que explicarle
a nadie si tiene Intel o Apple Silicon. Cuesta ~40 MB más de artefacto y ahorra una pregunta de
soporte por cada usuario de Mac.

**Linux `.deb` y `.AppImage`, no Flatpak ni Snap**: Orbit Desktop necesita `~/.ssh`, el
`SSH_AUTH_SOCK` y el binario `ssh` del sistema. En Flatpak eso es una batalla de permisos
(`--filesystem=~/.ssh`, `--socket=ssh-auth`) que además rompe `ProxyCommand`, porque el comando del
proxy se ejecutaría dentro del sandbox. En Snap, peor. **Un sandbox que hay que abrir de par en par
no es un sandbox: es un obstáculo con marketing.** `.deb` para quien tiene Debian/Ubuntu (que es el
público exacto de Orbit) y `.AppImage` para el resto.

### 8.2 La CI

**`ci.yml`** (en cada PR, ~8 min):

```yaml
jobs:
  rust:      # ubuntu-24.04, macos-15, windows-2025
    - cargo fmt --check
    - cargo clippy --all-targets -- -D warnings
    - cargo test --workspace          # incluye la generación de tipos y el orbit falso
  ui:        # ubuntu-24.04
    - pnpm lint && pnpm exec tsc -b --noEmit
    - pnpm test -- --coverage
  contract:  # ubuntu-24.04
    - ./tools/check-contract.sh       # §9.1: compara con el orbit real
  build:     # las 3, sin firmar, sólo para saber que compila
    - pnpm tauri build --no-bundle
```

**`release.yml`** (por etiqueta `v*`, ~25 min): matriz de 4, `tauri-action`, firma, notarización, y
un `job` final que compone el `latest.json` del actualizador y publica la release de GitHub.

### 8.3 Firma y notarización, con los secretos concretos

**macOS.** El certificado *Developer ID Application* se guarda como `.p12` en base64
(`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`), se importa a un llavero temporal en el runner,
y `tauri build` firma con *hardened runtime*. La notarización usa **API key de App Store Connect**
(`APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_API_KEY`) y **no** usuario/contraseña específica de
app: la clave no caduca cada pocos meses y no está atada a una cuenta personal. Después,
`xcrun stapler staple` sobre el `.app` y sobre el `.dmg`, para que la primera apertura funcione sin
red.

El `entitlements.plist` es **mínimo**. Sin `com.apple.security.app-sandbox` —una app que lanza
`ssh` y lee `~/.ssh/config` no puede vivir en el sandbox de la App Store, y por eso Orbit Desktop
**no se distribuye por la App Store**, decisión tomada aquí y no descubierta después. Sí lleva
`com.apple.security.cs.allow-unsigned-executable-memory` sólo si el WebView lo exige (JIT de
JavaScriptCore); se comprueba y, si no hace falta, se quita.

**Windows.** **Azure Trusted Signing** y no un certificado EV en un token físico. Motivos: (a) el
token USB no se puede usar en un runner de GitHub sin un agente autoalojado con el token enchufado a
una máquina, que es un punto de fallo físico y una llave que alguien tiene en un cajón; (b) Trusted
Signing cuesta unos 10 $/mes frente a 300-500 $/año de un EV; (c) la reputación de SmartScreen se
acumula igual. Secretos: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, más el
endpoint y el perfil.

**Linux.** El `.deb` se firma con GPG (`dpkg-sig`) y se publica la clave pública. No hay
repositorio APT propio en la fase 1: es infraestructura que hay que mantener y el `.deb` suelto
resuelve el 100 % del caso. Se plantea para la fase 5 si alguien lo pide.

### 8.4 El actualizador, y su verificación de firma

`tauri-plugin-updater`. Lo que hay que entender es que **hay dos firmas y son independientes**:

1. **La del sistema operativo** (Authenticode, notarización). Protege la instalación inicial y la
   confianza del SO.
2. **La de minisign del actualizador**. Es una clave Ed25519 propia del proyecto. La pública va
   **compilada dentro del binario** (`tauri.conf.json` → `plugins.updater.pubkey`); la privada vive
   sólo en `TAURI_SIGNING_PRIVATE_KEY` de GitHub Actions, protegida con contraseña
   (`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).

El flujo: el cliente descarga `latest.json` de una URL fija, comprueba la versión, descarga el
artefacto, **verifica la firma minisign contra la clave que lleva dentro** y sólo entonces aplica.
Un atacante que controle el CDN o la release de GitHub puede servir un binario, pero no puede
firmarlo, y el cliente lo rechaza.

**Política de actualización:** se comprueba al arrancar y cada 6 horas; **nunca se instala sola**.
Se enseña una banda «Hay una versión nueva (0.4.0). Ver los cambios · Instalar y reiniciar». La
razón es concreta y no es filosófica: **si hay un despliegue en marcha, reiniciar la app lo corta**
(§3.5). Con un despliegue activo, el botón de instalar está deshabilitado y lo dice.

Y una regla que se escribe antes que el código: **Orbit Desktop nunca actualiza `orbit` en el
servidor.** Ni lo ofrece con un botón que lo haga. Si detecta una versión vieja, enseña el comando
para copiar. Actualizar el servidor desde el cliente sería escribir en el servidor algo que no es
una invocación del contrato, y es exactamente la regla dura nº 1.

### 8.5 Versionado

**SemVer para el cliente**, con el minor reservado para los saltos de compatibilidad de contrato.
El binario publica tres cosas en «Acerca de» y en el registro: su versión, `CONTRACT_MIN` y
`CONTRACT_MAX`. Cuando `orbit` publique contrato 2 y el cliente lo soporte, eso es un minor del
cliente, no un patch, aunque el diff sea pequeño: es un cambio de con quién puede hablar.

Y una tabla en `docs/CONTRACT.md` —mantenida a mano, con fecha— que dice qué versión de Orbit
Desktop habla con qué versiones de Orbit. Es la primera pregunta que hará cualquiera que abra un
issue.

---

## 9. Plan de pruebas del lado del código

### 9.1 La prueba que ningún otro proyecto puede tener: contra el `orbit` real

`tools/check-contract.sh` corre en CI y hace algo que sólo se puede hacer porque `orbit` es un
fichero de texto: **extrae la verdad del script y la compara con la del cliente**.

```bash
# 1. La lista de comandos con --json, sacada de _json_capable, no de la documentación.
sed -n '/^_json_capable()/,/^}/p' "$ORBIT" \
  | grep -oE '^\s+[a-z|áéíóú-]+\)' | tr -d ' )' | tr '|' '\n' | sort -u > /tmp/server-json.txt

# 2. La misma lista, desde el cliente.
cargo run -q -p orbit-client --bin dump-json-capable | sort -u > /tmp/client-json.txt

diff -u /tmp/server-json.txt /tmp/client-json.txt

# 3. Los 37 campos de configuración.
sed -n '/^ORBIT_APP_FIELDS=(/,/^)/p' "$ORBIT" | grep -oE 'A_[A-Z_]+' \
  | sed 's/^A_//' | tr 'A-Z' 'a-z' | sort > /tmp/server-fields.txt
cargo run -q -p orbit-client --bin dump-config-fields | sort > /tmp/client-fields.txt
diff -u /tmp/server-fields.txt /tmp/client-fields.txt

# 4. Los pasos de deploy.
grep -oE '_dstep [a-z]+' "$ORBIT" | awk '{print $2}' | sort -u > /tmp/server-steps.txt
```

Corre contra el `orbit` de la última etiqueta publicada, traído con `git clone --depth 1`. **Cuando
Orbit añada un campo, esta prueba falla y alguien lo ve el mismo día.** Sin ella, la divergencia se
descubre cuando un usuario ve un hueco en una pantalla. Es la prueba que justifica, ella sola, que
el cliente sea un repositorio aparte del mismo autor: se puede permitir esta clase de acoplamiento
de verificación sin acoplar el código.

### 9.2 Unitarias del parser del contrato

**Fixtures reales, no inventadas.** `crates/orbit-client/tests/fixtures/` contiene salidas
capturadas de un `orbit` de verdad, incluyendo las raras. La lista mínima, que sale directamente de
esta auditoría:

```
version.ok.json                 list.empty.json             list.4apps.json
info.node.json                  info.static.json            info.redirect.json
status.json                     doctor.mixed.json           doctor.allok.json
top.firstsample.json            top.nulls.json              # cpu/mem/requests null
deploy.ok.json                  deploy.failed-build.json    deploy.rolledback.json
deploy.recovered.json           deploy.noapp.json           # app:"" — sí, contesta
deployall.sixfinals.json                                    # los seis a la vez
metrics.empty.json              metrics.oneapp.json         # envuelto en apps[] + kept
traffic.empty.json              traffic.incomplete.json     # complete:false, status:{}
traffic.sparse-status.json                                  # sólo 2xx y 4xx
envlist.empty.json              watch.spanish-states.json   queue.json  redirect.json
```

Y las de error, que son la mitad del trabajo:

```
err.noapp.txt                   err.nojson.txt              err.needroot.txt
err.notinstalled.stdout.txt     # las dos líneas por STDOUT de la línea 265
err.sudo-no-tty.txt             err.ssh-refused.txt         err.ssh-hostkey.txt
```

Las pruebas, con `insta` para snapshots:

- **Cada fixture parsea sin error** y produce el tipo esperado.
- **Los `null` sobreviven**: `info.static.json` → `port == None`, `service == None`. Un test que
  falla si alguien mete un `unwrap_or(0)`.
- **Campos desconocidos se ignoran**: se toma `list.4apps.json`, se le añade
  `"state":{"cosa_nueva":true,…}` y `"apps":[{"campo_futuro":1,…}]`, y debe seguir parseando.
- **Enumerados desconocidos no rompen**: `"status":"septimofinal"` → `AppStatus::Unknown`.
- **`summary.ok` incluye los `info`**: la prueba lo afirma explícitamente, con el comentario que
  explica por qué no cuadra con el filtro.
- **`traffic.status` disperso**: `{}` y `{"2xx":10,"4xx":3}` parsean los dos.
- **`from` de traffic no es ISO**: la función `parseTrafficStamp("20260829035045")` tiene su tabla.
- **Argv**: una tabla de `OrbitCommand` → `Vec<String>` esperado, que comprueba que `--json` y
  `--lang` van **delante**, que `deploy` lleva siempre app, que `rollback` lleva siempre release y
  que `remove` lleva siempre `-y`.
- **Citado de shell**: nombres y valores con `'`, `"`, `$`, `` ` ``, espacios, `;`, salto de línea y
  UTF-8. Comparado contra lo que produce `printf %q` de bash para los mismos casos.
- **Clasificación de errores**: cada `err.*.txt` → la variante esperada de `TransportError`/
  `OrbitError`.

Un lint propio de ESLint (`no-null-coalesce-on-contract`) prohíbe `?? 0`, `?? ''` y `|| 0` sobre
cualquier campo de los tipos generados. Es la forma de hacer cumplir la regla nº 1 de §4.1 sin
depender de que alguien lo recuerde en la revisión.

### 9.3 El `orbit` de mentira

`tools/fake-orbit/` es un script de Bash de unas 200 líneas —Bash, para que no haya que instalar
nada en el runner— que acepta el mismo argv que `orbit` y responde con fixtures.

```bash
#!/usr/bin/env bash
# Un orbit de mentira. Responde el contrato desde ficheros y sabe fallar a propósito.
#   FAKE_ORBIT_SCENARIO=happy|no-json|not-installed|needs-root|slow|deploy-fails|contract2
set -euo pipefail
S="${FAKE_ORBIT_SCENARIO:-happy}"
F="$(dirname "$0")/fixtures"

case "$S" in
  not-installed) echo "Orbit no está instalado todavía (falta /etc/orbit/orbit.conf)."
                 echo "Ejecuta primero:  sudo bash install.sh"; exit 1 ;;   # ¡por stdout!
  needs-root)    echo "  ✗ Este comando necesita root. Usa:  sudo orbit list" >&2; exit 1 ;;
esac

json=no; lang=es; args=()
for a in "$@"; do
  case "$a" in --json) json=yes ;; --lang) ;; es|en) lang="$a" ;; *) args+=("$a") ;; esac
done
cmd="${args[0]:-menu}"

[[ "$S" == slow ]] && sleep "${FAKE_ORBIT_DELAY:-3}"

case "$cmd" in
  version) [[ "$json" == yes ]] \
      && { [[ "$S" == contract2 ]] \
             && echo '{"schema":2,"version":"2.0.0","contract":2}' \
             || cat "$F/version.ok.json"; } \
      || echo "orbit 1.3.6" ;;
  list)    cat "$F/list.4apps.json" ;;
  status)  cat "$F/status.json" ;;
  info)    cat "$F/info.${args[1]:-node}.json" ;;
  top)     sleep 1; cat "$F/top.nulls.json" ;;
  doctor)  cat "$F/doctor.mixed.json" ;;
  deploy)
    # El progreso por STDERR, mezclado con prosa, tal y como lo hace orbit de verdad.
    printf '\nDesplegando %s\n──────────────\n' "${args[1]}" >&2
    for s in code release build activate service nginx; do
      printf '{"event":"step","app":"%s","step":"%s","status":"start","elapsed_s":0}\n' "${args[1]}" "$s" >&2
      printf '  ✓ %s\n' "$s" >&2
      [[ "$S" == "deploy-fails" && "$s" == "build" ]] && { cat "$F/deploy.failed-build.json"; exit 1; }
      printf '{"event":"step","app":"%s","step":"%s","status":"ok","elapsed_s":1}\n' "${args[1]}" "$s" >&2
    done
    cat "$F/deploy.ok.json" ;;
  logs)    [[ "$S" == no-json ]] && { echo "  ✗ Opción desconocida: --json" >&2; exit 1; }
           cat "$F/logs.txt" ;;
  *)       echo "  ✗ 'orbit $cmd' no tiene salida JSON. La tienen: …" >&2; exit 1 ;;
esac
```

Se usa de dos formas:

- **Sin SSH**, sustituyendo el «binario ssh» del cliente por un envoltorio que ejecuta el falso en
  local. Es lo que permite probar `OrbitClient` entero —tuberías, progreso, timeouts, cancelación—
  en `cargo test`, en los tres sistemas operativos, sin red.
- **Con SSH real contra `localhost`**, copiando el falso a un contenedor con `sshd` y una clave.
  Es lo que prueba de verdad `ControlMaster`, `ProxyJump` (con dos contenedores), el fallo de
  `BatchMode` y el timeout. Un `docker compose` de tres servicios en `tools/ssh-lab/`.

Los escenarios `slow`, `deploy-fails`, `contract2`, `not-installed` y `needs-root` son las cinco
pantallas de error que hay que poder ver a demanda. Sin ellas, la interfaz de error se escribe a
ciegas y se descubre rota en producción.

### 9.4 Integración y end-to-end

**Integración (Rust, `cargo test`)**, contra el falso:

- El objeto de `deploy` llega entero aunque el proceso salga con 1.
- El progreso se emite en orden y **se filtra la prosa** intercalada.
- Dos tuberías: un `deploy` que escribe 1 MB por stderr no bloquea. (La prueba del *deadlock*: sin
  el `join!` concurrente, falla.)
- La cancelación mata el proceso y emite `Cancelled` en menos de 500 ms.
- El *watchdog* de inactividad dispara con `FAKE_ORBIT_DELAY` alto.
- La matriz de compatibilidad de §4.4: los ocho casos, uno por escenario.
- `not-installed` produce `NotInstalled` **aunque el texto venga por stdout**.

**End-to-end (Playwright sobre `tauri driver`)**, seis flujos y no más, porque los e2e caros que
nadie mantiene son peor que ninguno:

1. Alta de servidor → sondeo → aparece con su versión.
2. Lista de apps → una con `served:false` se pinta como «sin vhost» y gana sobre `maintenance`.
3. Desplegar → barra de seis pasos → objeto final → la lista se invalida y se refresca.
4. Desplegar y que falle → el paso `build` en rojo, `error` visible, y **la app sigue en la lista**.
5. Servidor caído → tarjeta gris, datos viejos atenuados, botones de mutación deshabilitados.
6. Cambio de idioma → la interfaz y el `--lang` del comando cambian juntos.

**Capturas en CI en las tres plataformas** para las pantallas principales, comparadas con
tolerancia. No para detectar regresiones de píxel —eso es una fuente de falsos positivos— sino
para detectar el caso real de Tauri: **algo que se ve bien en WebView2 y se rompe en WebKitGTK**.

### 9.4b Las 44 respuestas patológicas del informe de QA, y dónde se cubre cada una

El catálogo de §5.2b del informe de QA es la mejor pieza de ese documento y hay que decir dónde se
satisface, no dar por hecho que sí. Agrupadas por mecanismo, porque una tabla de 44 filas no se
lee:

| Grupo | P-nn | Mecanismo de este documento |
|---|---|---|
| **Forma de stdout** | P-01, P-06, P-11, P-12, P-13 | §3.9 · `StreamDeserializer` con flujo agotado, UTF-8 estricto, error ante basura delante o detrás y ante dos objetos |
| **Presupuesto** | P-19, P-20, P-21, P-27 | §3.8 · lectores acotados por comando, cortacircuitos de 20.000 líneas/s en `logs`, `deploy` con *watchdog* de inactividad |
| **Contrato** | P-02, P-03, P-04, P-05, P-28, P-29 | §4.1 (sin `deny_unknown_fields`, enumerados con `Unknown`), §4.4 (matriz de compatibilidad), §3.9 paso 4 |
| **Códigos de salida** | P-08, P-09, P-10, P-14, P-15 | §3.3 (el orden de comprobación de seis pasos), §3.4 (`TransportError` vs `OrbitError`), §4.5 |
| **Identidad de la respuesta** | P-42, P-43 | §3.10 · claves obligatorias por comando + comprobación del campo `app` en los cinco que lo llevan |
| **Transporte** | P-16, P-17, P-18, P-44 | §3.2 (`BatchMode`, `ConnectTimeout`, `ServerAlive*`), §3.5 (idempotencia: sólo se reintenta lo que se puede), §3.7 (retroceso, y latencia ≠ caída) |
| **Texto hostil** | P-22, P-23, P-24, P-25, P-26, P-30 | §7.1 R-03 (`no-at-html-tags`), §7.2 (`AppName` valida y **el escapado es independiente**, E3), §3.9 (UTF-8 estricto) |
| **Progreso** | P-31, P-32 | §1.5 (el filtro de stderr descarta lo que no parsea; el campo `app` permite atribuir) |
| **Los seis finales** | P-33, P-34 | §4.2 `DeployAllResponse` con los seis recuentos separados; §5.6 y §10 fase 2 |
| **Los `null` con nombre** | P-35, P-36, P-37, P-38, P-39, P-40, P-41 | **§4.3, la tabla de los siete `null`**, más R-05 (el lint que prohíbe `?? 0`) |
| **Vacío legítimo** | P-07 | §4.1 + el emisor: `_dall_json` devuelve `total:0`, no silencio |

**Cuatro de las 44 merecen una nota porque este documento aporta algo que el catálogo no podía
saber:**

- **P-41 (`load: []`)** — el catálogo la deduce de la línea 11497. Correcto: `_status_json` emite
  `"load":[%s]` con `cut -d' ' -f1-3 /proc/loadavg | tr ' ' ','`, así que si `/proc/loadavg` no se
  puede leer, el array sale vacío y **el tipo `[number,number,number]` de §4.2 no encaja**. Se
  tipa como `number[]` con una guarda de longitud, no como tupla.
- **P-29 (`cert_days: -5`)** — el catálogo dice, con razón, que un negativo **es real**: un
  certificado caducado. `cert_days_left` (3221) resta fechas y no acota por abajo. La interfaz lo
  pinta como «caducado hace 5 días», no como un número raro.
- **P-30 (`env list` con una clave que parece un valor)** — imposible por construcción: el `grep`
  de la línea 11274 corta en el `=`, así que lo que sale nunca lleva valor. Pero **la comprobación
  se hace igual**, porque el cliente no debe depender de cómo esté escrito hoy el emisor.
- **P-35 (`fixable:false` con `fix` no nulo)** — no es patológico: **es lo normal**. Medido en el
  banco de 40 apps, `doctor --json` da 109 comprobaciones y la mayoría de las que traen `fix` no
  son arreglables (§1.5). Enseñar el texto sin botón es el comportamiento correcto, no el caso
  raro.

**Y el fuzzing de §5b del informe de QA** encaja en este plan sin cambios: G1 (estructurado, el que
busca mentiras y no caídas) usa los mismos tipos generados de §4.2 como esquema; G2 (mutador) parte
de los fixtures de §9.2; G3 (adversario) reutiliza el corpus del escapador de E3, que ya tiene el
`</script><img src=x onerror=…>`, el `U+202E`, el `$(rm -rf /)` y los 64 KB de una letra. **La
propiedad P2 —«el cliente nunca pinta datos inventados»— se comprueba contra la tabla de los siete
`null` de §4.3**, que es lo que le da al fuzzer algo que afirmar en vez de sólo algo que no romper.

### 9.5 Lo que NO se prueba, dicho a propósito

- No se prueba `orbit` — tiene sus 33 suites. El cliente prueba **su lectura** de `orbit`.
- No hay pruebas contra un VPS real en CI. Sí una lista de comprobación manual de 20 minutos antes
  de cada release, en `docs/QA.md`, contra un servidor de verdad con Orbit instalado: es donde
  aparecen las cosas que ningún falso reproduce (una latencia real, un `sudo` con contraseña, un
  `ProxyJump` a través de un bastión que exige `PasswordAuthentication`).
- No se prueba el actualizador en CI más allá de que el `latest.json` se genera y valida. Probarlo
  de verdad es publicar una versión y actualizar desde la anterior, y eso se hace a mano una vez por
  minor.

---

## 9b. El PR a `orbit`: el trabajo más rentable que hay

Esta sección no es una lista de deseos. Es **el entregable de la ronda 2 para el punto que la
ronda 1 identificó como el de mayor rendimiento y dejó sin escribir**: cuatro cambios en `orbit`,
listos para abrirse, que borran tres huecos del contrato, arreglan un comando documentado y roto, y
cierran la laguna de pruebas que dejó pasar ese comando.

**Y no es una conversación hipotética.** Orbit y Orbit Desktop son de la misma organización
(iNTERVOLUTIONS-Labs), así que esto se abre como PR y se discute como cualquier otro. Va escrito
con la disciplina que el repositorio exige: cada cambio con su porqué, con lo descartado, y con lo
que se sabe que no cubre.

El orden es por rendimiento decreciente para el cliente.

### 9b-1 · `orbit doctor --fix --json --yes` (arreglo de un comando documentado y roto)

**El problema.** Verificado ejecutando (§1.6b, y confirmado independientemente en E4):

```
$ orbit doctor --fix --json --yes
  ✗ orbit doctor: no sé qué es «--yes». Sólo acepta --fix y --json.
$ orbit --yes version --json
  ✗ Comando desconocido: --yes
```

`--yes` **no es una bandera global** —el bucle de `main()` (13627-13637) sólo conoce `--json`,
`--eva` y `--lang`— y `ASSUME_YES` sólo se asigna en la línea 6915, dentro de `cmd_new`, sobre una
**copia local** (`local ASSUME_YES="$ASSUME_YES"`, 6889). La guarda de la línea 12118 exige
`ASSUME_YES=yes` para dejar pasar `doctor --fix --json`, y no hay forma de dársela. **El camino está
muerto por las dos ramas**, y está documentado en `USAGE.md:1674` y razonado en `ARCHITECTURE
§19.5`.

**El arreglo, y la elección entre dos formas.**

*Forma A — `--yes` como bandera global*, junto a `--json`, en el bucle de `main()`:

```bash
      --json) JSON="yes"; shift ;;
      -y|--yes|--si) ASSUME_YES="yes"; shift ;;      # ← nueva
      --eva|--jedimaster|--jedi) EVA="yes"; shift ;;
```

*Forma B — `--yes` reconocido en `cmd_doctor`*, y en ningún sitio más:

```bash
  for a in "$@"; do
    case "$a" in
      --fix) fix="yes" ;;
      -y|--yes|--si) ASSUME_YES="yes" ;;             # ← nueva
      *) die "orbit doctor: no sé qué es «%s». Sólo acepta --fix, --yes y --json." "$a" ;;
    esac
  done
```

**Se propone la B, y el motivo es de diseño, no de tamaño.** La forma A convierte `--yes` en una
bandera que vale en **todos** los comandos, y hoy hay ocho que la reconocen cada uno por su cuenta
con su propio significado local: `cmd_new` (6915), `cmd_remove` (11307), `cmd_restore` (8302),
`cmd_migrate` (10847). Hacerla global sin revisar los ocho es cambiar el comportamiento de
`orbit remove --yes` y de `orbit migrate --yes` de formas que nadie ha pensado — y `remove` es
destructivo. La forma B arregla lo que está roto y no toca nada más. Si algún día se quiere la A,
es su propio PR, con su propia revisión de los ocho.

Un detalle que hay que cambiar además: **el mensaje de la línea 12088 nombra las opciones que
acepta**, y si se añade una hay que nombrarla ahí. Es la misma trampa que el comentario de la línea
1099 cuenta sobre `_json_cmds_help`: «son la misma verdad contada dos veces, y cuando se separan la
que se queda corta le dice a un cliente que algo no existe».

**Impacto en el cliente:** la pantalla de diagnóstico pasa de «enseñar el texto del arreglo» a
«enseñar el botón donde `fixable` es cierto». Es la diferencia entre un informe y una herramienta,
y es lo que `fixable` existe para permitir (comentario de la línea 12006).

### 9b-2 · `orbit backup list --json` y `orbit backup verify --json`

**El problema.** `orbit backup list` (8033-8046) imprime una tabla:

```bash
        printf "  %-44s %8s  %s\n" "$(basename "$f")" \
          "$(du -h "$f" | cut -f1)" "$(date -r "$f" '+%Y-%m-%d %H:%M')"
```

Un cliente que quiera enseñar las copias tiene que parsear columnas de anchura fija, que es
exactamente lo que `ARCHITECTURE §13.1` dice que no se debe hacer: *«a partir de ese momento
alinear una columna es un cambio incompatible»*. Y el tamaño viene de `du -h`, o sea `1,2G`, que un
cliente tiene que reinterpretar — con el separador decimal de la configuración regional del
servidor, para más gracia.

**La forma propuesta**, siguiendo las convenciones de la casa (lo que no existe es `null`, los
bytes en bruto, un solo objeto por stdout):

```json
{"schema":1,
 "dir":"/var/backups/orbit",
 "backups":[
   {"file":"/var/backups/orbit/mi-web-20260829-031500.tar.gz",
    "app":"mi-web",
    "kind":"app",
    "created":"2026-08-29T03:15:00+00:00",
    "size_bytes":128374912,
    "verified":null},
   {"file":"/var/backups/orbit/_orbit-conf-20260829-031500.tar.gz",
    "app":null,
    "kind":"config",
    "created":"2026-08-29T03:15:00+00:00",
    "size_bytes":8192,
    "verified":null}
 ],
 "total":2,
 "bytes":128383104,
 "keep_days":14}
```

Cuatro decisiones dentro, y las cuatro se derivan de reglas que Orbit ya aplica:

- **`app: null` para la copia de la configuración global.** `_verify_one` ya la trata aparte
  («La configuración global no lleva app ni manifiesto», línea 8014), así que el JSON tiene que
  poder decir lo mismo. `kind` distingue `"app"` de `"config"` sin obligar a mirar el nombre del
  fichero.
- **`size_bytes`, no `size`.** En bruto. `du -h` es presentación.
- **`created` desde el mtime, en ISO-8601 con huso**, como `A_CREATED` (línea 7170:
  `date -Iseconds`). No `%Y-%m-%d %H:%M`, que pierde el huso y no se puede comparar.
- **`verified: null`** = «no lo he comprobado en esta llamada». Es el mismo `null` de `cert_days`:
  «no lo he mirado» no es «está mal». `orbit backup verify --json` sí lo rellena.

Y para `verify`:

```json
{"schema":1,
 "backups":[{"file":"…/mi-web-20260829-031500.tar.gz","ok":true,"message":"3 apps · 12 MB · con base de datos"},
            {"file":"…/otra-20260828-031500.tar.gz","ok":false,"message":"el volcado de la base de datos está vacío"}],
 "total":2,"ok":1,"bad":1}
```

`message` traducido (es `$VERIFY_MSG`, prosa para una persona), `ok` para automatizar. Es la misma
separación que `deploy --json` hace entre `error` y `failed_step`.

**El código de salida no cambia**: `verify` ya devuelve 1 si hay alguna copia rota (8027), y esa
regla es buena. **`list` sigue saliendo con 0 con cero copias**, porque una colección vacía es una
respuesta (§13.6bb).

**Impacto en el cliente:** desaparece `contract/text.rs` **entero** — es la cuarentena de §7.1, y
sus dos únicos habitantes son estos dos comandos. El cliente pasa a tener **cero líneas que
parseen texto**, que es un estado que se puede afirmar y comprobar con una regla de lint, en vez de
un objetivo.

### 9b-3 · `orbit logs --json`

Es el hueco más grande del contrato y también el más discutible, así que va con la alternativa
descartada por delante.

**El problema.** `orbit logs` (8854-8927) no habla JSON, y además **sigue en vivo por defecto**
(8884: sin `--since` y sin `--no-follow`, `follow="yes"`). Un cliente que quiera una ventana de log
recibe texto crudo de dos fuentes distintas —`journalctl -u orbit-<app>` para las apps con proceso,
`tail` de los dos ficheros de nginx para el resto— con una línea de cortesía traducida delante
(`Ctrl-C para salir`, 8894 y 8919) y a veces un `info` sobre qué logs se están viendo (8907).

**Lo que NO se propone, y por qué.** *Un `--json` que devuelva un objeto con todas las líneas
dentro.* Sería coherente con «por stdout un solo objeto» (§13.6b), pero rompe el caso principal:
`--follow`. Un objeto no se puede emitir hasta que termina, y un log en vivo no termina. Y con
`--since 7d` sobre una web con tráfico, ese objeto son cientos de MB en memoria de bash antes de
imprimirse. **Un contrato que sólo funciona para el caso pequeño es un contrato con una excepción,
y §13.6b dice que un contrato con una excepción son dos contratos.**

**Lo que se propone: NDJSON por stdout, con la excepción declarada.** Una línea de JSON por línea
de log, legible según llega:

```
$ orbit logs mi-web --json --since 1h --no-follow
{"schema":1,"event":"meta","app":"mi-web","source":"journal","unit":"orbit-mi-web","since":"2026-08-29T14:00:00+00:00","follow":false}
{"event":"line","ts":"2026-08-29T14:02:11+00:00","stream":"journal","text":"Listening on 127.0.0.1:3001"}
{"event":"line","ts":"2026-08-29T14:02:44+00:00","stream":"journal","text":"GET / 200"}
{"event":"end","lines":2,"truncated":false}
```

Y para las apps sin proceso, `source: "nginx"` y `stream: "access"` o `"error"`, que es la
distinción que hoy se pierde porque `tail` mezcla los dos ficheros sin decir cuál es cuál.

**Las cinco decisiones, con su motivo:**

1. **NDJSON y no un objeto, sólo aquí, y dicho en voz alta.** `logs` es el único comando del
   contrato cuya salida es **inherentemente un flujo sin final**. La excepción se declara en la
   documentación en vez de fingir que no existe. Y para que un cliente no tenga que adivinar, la
   **primera** línea es un `event:"meta"` que lleva `"schema":1` y dice qué va a venir: si el
   contrato de `logs` cambiara, ahí se vería.
2. **`--follow` sigue siendo el defecto en modo humano y pasa a ser `false` con `--json`.** Es la
   misma regla que `orbit top`, que «en modo máquina siempre es una foto» (USAGE). Quien quiera
   seguir en vivo lo pide con `--follow` explícito, que es lo que hará el cliente para su pantalla
   de log en vivo. **Esto invierte un defecto**, y por eso es la parte del PR que más discusión
   merece: cambia el comportamiento de `orbit logs --json`, que hoy es un error, así que no rompe a
   nadie — pero conviene decirlo.
3. **`ts` en ISO-8601 y `null` si no se sabe.** Un log de nginx del formato antiguo no lleva marca
   de tiempo (`_log_has_time`, 8846) y hoy eso se avisa con un `warn`. En JSON es `"ts": null`, y el
   cliente puede ofrecer el `orbit nginx-rebuild` sin parsear un mensaje traducido.
4. **`truncated`**, para decir que se llegó al tope de `--lines`. Es la misma honestidad que
   `requests_capped` en `top` y `complete` en `traffic`: *«un número corto sin avisar se lee como
   "hay poco tráfico", que es justo lo contrario de lo que está pasando»* (§13.6).
5. **Nada de estructurar el contenido de la línea.** `text` es la línea tal cual. Intentar sacarle
   el nivel, el módulo o el código HTTP sería inventar un formato que la app del usuario no ha
   prometido, y sería el primer paso hacia un parser de logs dentro de `orbit`.

**Coste de implementación estimado:** unas 60 líneas. Las dos ramas de `cmd_logs` ya tienen la
información; lo que falta es un envoltorio que convierta cada línea, y `_j_str` ya existe. La parte
delicada es el `ts`: para `journalctl` sale gratis con `-o short-iso`; para nginx hay que
reutilizar el `awk` de `_LOG_SINCE_AWK` (8811), que ya sabe leer las dos marcas de tiempo posibles.

**Impacto en el cliente:** la pantalla de logs deja de tener un parser de texto con heurística de
glifos (§1.11) y pasa a leer un flujo tipado. Y gana algo que hoy no se puede: **distinguir el log
de acceso del de error**, que es la primera pregunta de cualquiera que mira un log de nginx.

### 9b-4 · Una prueba que ejecute `main()`

Éste es el cambio pequeño y es el que evita el siguiente 9b-1.

**El problema.** `tests/lib.sh:34-38` carga el script **sin su última línea**:

```bash
sed '$ d' "$ORBIT_ROOT/orbit" \
  | sed "s|^ETC_DIR=.*|ETC_DIR=\"$TMP/etc\"|" \
  | sed 's|^if \[\[ \$EUID -ne 0 \]\]; then|if false; then|' \
  > "$TMP/orbitlib.sh"
source "$TMP/orbitlib.sh"
```

Esa última línea es `main "$@"`. Quitarla es lo correcto para probar funciones una a una, y las 33
suites del repositorio hacen eso muy bien. **Pero significa que `main()` —las 110 líneas donde
viven el bucle de banderas globales, `_json_strip`, `_lang_strip`, la criba de `_json_capable` y el
árbol de despacho entero— no la ejecuta ninguna prueba.**

Y ése es exactamente el motivo por el que 9b-1 lleva meses documentado y roto:
`tests/doctorfix_test.sh:362` llama a `cmd_doctor --fix` **como función**, saltándose `main()`, que
es donde vive `ASSUME_YES`. La prueba pasa y el comando no funciona. Es la lección que el propio
`ARCHITECTURE §13.6c` ya escribió sobre otro fallo: *«la lección no es "revisar mejor": es que la
prueba tiene que ejercer el camino»*.

**Lo que se propone: `tests/cli_test.sh`.** Unas 60 líneas, sin dobles, invocando el script **como
binario**, que es como lo invoca un usuario y como lo invocará Orbit Desktop:

```bash
#!/usr/bin/env bash
# El router: main() de punta a punta. Las demás suites cargan las funciones sin
# main (tests/lib.sh corta la última línea a propósito); ésta ejercita justo eso.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Una copia ejecutable con ETC_DIR redirigido y la auto-elevación fuera.
sed "s|^ETC_DIR=.*|ETC_DIR=\"$TMP/etc\"|; s|^if \[\[ \\\$EUID -ne 0 \]\]; then|if false; then|" \
    "$ORBIT_ROOT/orbit" > "$TMP/orbit"; chmod +x "$TMP/orbit"
O="$TMP/orbit"

section "Banderas globales"
check "--json delante"        0 "$("$O" --json version >/dev/null 2>&1; echo $?)"
check "--json detrás"         0 "$("$O" version --json >/dev/null 2>&1; echo $?)"
check "--json en un comando que no lo habla"  1 "$("$O" --json logs >/dev/null 2>&1; echo $?)"
check "--lang sin valor"      1 "$("$O" --lang >/dev/null 2>&1; echo $?)"
check "--lang inexistente"    1 "$("$O" --lang klingon list >/dev/null 2>&1; echo $?)"
check "comando desconocido"   1 "$("$O" noexiste >/dev/null 2>&1; echo $?)"

section "Los comandos que dicen hablar JSON, lo hablan"
# La lista se saca del propio script, no se escribe a mano: si _json_capable
# gana una entrada y esta prueba no, la que se queda corta miente (línea 1099).
while read -r c; do
  out="$("$O" --json "$c" 2>/dev/null || true)"
  check "orbit --json $c emite un objeto" "yes" \
        "$([[ "${out:0:1}" == "{" ]] && echo yes || echo no)"
done < <(sed -n '/^_json_capable()/,/^}/p' "$ORBIT_ROOT/orbit" \
         | grep -oE '^\s+[a-z-]+\)' | tr -d ' )' | grep -vE '^(env|db|redirect|watch|queue|deploy)$')

section "Lo documentado se puede ejecutar"
# Regresión de este PR. Antes de 9b-1 esto salía con 1 y el mensaje
# «no sé qué es «--yes»».
check "doctor --fix --json --yes no protesta por --yes" "no" \
      "$("$O" doctor --fix --json --yes 2>&1 | grep -qF -- '--yes' && echo si || echo no)"

section "Sin terminal no se elige por el usuario"
# La otra mitad del mismo problema (E5): 'orbit info' sin app y sin TTY
# elige la primera. Aquí sólo se DOCUMENTA el comportamiento actual con una
# prueba, para que un cambio futuro sea deliberado y no un accidente.
check "info sin app y sin tty elige la primera" "app-a" \
      "$("$O" info </dev/null 2>/dev/null | sed -n '2p' | tr -d ' ')"
```

Nótese la última sección, que es la parte interesante y la que hace que este PR valga más que sus
60 líneas: **no arregla el comportamiento de E5, lo fija con una prueba.** Cambiar `pick_app` para
que aborte sin TTY sería un cambio de comportamiento en once comandos, y eso es su propio PR con su
propia discusión. Lo que sí hace falta hoy es que ese comportamiento **esté escrito en una prueba**,
para que el día que alguien lo cambie sea porque quiso, y no porque se le pasó.

### 9b-5 · La promesa de estabilidad del contrato, escrita

Éste es el cambio de **cero líneas de código** y el más valioso de los cinco. Es lo que la ronda 1
identificó como la debilidad nº 2 («la política de contrato 2 es una apuesta») y la conclusión fue
que lo que hace falta no es código del cliente: es que **`orbit` diga qué garantiza que sobrevive a
una subida de `schema`**. Hoy la promesa es media promesa.

Lo que hoy dice `ARCHITECTURE §13.1`:

> **Los campos se añaden, nunca se renombran.** Es una promesa explícita, escrita también en
> `USAGE.md`, y es lo único que hace que alguien pueda depender de esta salida. Si algún día hubiera
> que romperla, sube `schema`, que viaja en todas las respuestas precisamente para eso.

Es buena y es honesta, pero **le falta la otra mitad**: qué pasa *después* de subir `schema`. Un
cliente que lee «si hay que romper, sube schema» sólo puede concluir que schema 2 puede ser
cualquier cosa, y de ahí sale la política defensiva de §4.4 (lectura degradada, mutaciones
bloqueadas). Con dos párrafos más, esa política se puede afinar y el cliente puede seguir siendo
útil contra un servidor más nuevo.

**Texto propuesto, para añadir al final de §13.1 de `ARCHITECTURE.md`:**

> **Y qué se garantiza cuando `schema` sube.** Subir `schema` es la salida de emergencia y por eso
> conviene decir hasta dónde llega, porque un cliente que no lo sepa tiene que suponer lo peor y
> negarse a hablar — que es la peor forma de romper algo que todavía funcionaba.
>
> Tres cosas se garantizan **para siempre**, en cualquier `schema`:
>
> 1. **`orbit version --json` no cambia de forma.** Sigue siendo un objeto con `schema`, `version`
>    y `contract`, y los tres siguen significando lo mismo. Es el saludo, y un saludo que cambia
>    no se puede usar para negociar nada.
> 2. **Por stdout va un solo objeto JSON, y todo lo dirigido a una persona sale por stderr.** La
>    regla de §13.6b es del contrato, no de un comando.
> 3. **Lo que no existe sigue siendo `null`.** Ningún `schema` futuro sustituirá un `null` por un 0,
>    por una cadena vacía o por un valor centinela. Es la regla que separa «no aplica» de «está
>    caído», y romperla haría que un cliente antiguo pintara alarmas falsas en vez de fallar.
>
> Y una que se garantiza **dentro de un `schema`**: un campo, una vez publicado, no cambia de tipo
> ni de significado. Puede quedarse sin usar; no puede querer decir otra cosa.
>
> Lo que **no** se garantiza al subir `schema`: que un campo siga existiendo, que se llame igual,
> que una colección conserve su nombre, o que un enumerado no gane valores. Un cliente que se
> encuentre un `schema` mayor del que conoce puede fiarse de las cuatro reglas de arriba y de nada
> más — que es suficiente para saludar, avisar al usuario y no inventarse datos.

**Y el añadido a `USAGE.md`**, en la sección del contrato, más corto porque su público es otro:

> **Si algún día `schema` sube**, cambia el nombre o el significado de algún campo. Lo que **no**
> cambia nunca, ni entre versiones de `schema`: la forma de `orbit version --json`, que por stdout
> vaya un solo objeto y lo demás por stderr, y que **lo que no existe sea `null`** y no un cero.
> Un script que sólo dependa de esas tres cosas sigue funcionando pase lo que pase.

**Impacto en el cliente:** con esa promesa escrita, la política de §4.4 se puede relajar de forma
justificada: un servidor con `contract` mayor sigue permitiendo **saludar** (garantía 1), sigue
permitiendo **separar canales** (garantía 2), y todo lo que se lea se puede pintar sin miedo a que
un `null` se haya convertido en un 0 (garantía 3). El modo de lectura degradada deja de ser una
apuesta y pasa a apoyarse en algo escrito. Sube la nota de §4.4 y —más importante— **es lo único de
todo este documento que no puede hacer el cliente por su cuenta**.

### 9b-6 · Lo que este PR NO propone, dicho a propósito

Para que nadie lo pida en la revisión, y porque decidir qué no se pide es la mitad del trabajo:

- **No se propone `--json` en las mutaciones** (`new`, `remove`, `ssl`, `rollback`, `restart`).
  Sería cómodo, pero el cliente **ya tiene la respuesta correcta**: preguntar por el estado después
  (§1.14). Y añadir quince formas nuevas al contrato para ahorrar una llamada de 85 ms es engordar
  la superficie que hay que mantener durante años a cambio de casi nada.
- **No se propone un identificador de correlación** (`ORBIT_REQ` de ida y vuelta). Sería la
  comprobación fuerte de P-43, pero es cambiar el contrato del servidor para beneficio del cliente,
  y sobre un canal SSH autenticado no resuelve una amenaza real. §3.10.
- **No se propone cambiar `pick_app` para que aborte sin TTY.** Es el arreglo «correcto» de E5, y
  toca once comandos. Merece su propio PR y su propia discusión sobre compatibilidad; aquí sólo se
  fija con una prueba (9b-4).
- **No se propone tocar el código de salida de nada.** `doctor` sale con 0 aunque haya errores
  (12102) porque hay scripts encadenados con `&&` desde antes, y ése razonamiento sigue valiendo.
  El cliente lee `.summary.error`.
- **No se propone que `orbit` gane un modo demonio, una caché, ni nada que sobreviva al proceso.**
  El suelo de 76 ms por llamada (§5.0) es el precio de un script de Bash sin estado, y es el precio
  correcto: es lo que hace que Orbit Desktop pueda existir sin que el servidor gane un proceso.

---

## 10. Roadmap de implementación por fases

El criterio: **cada fase termina en algo que se puede enseñar y usar**. Nada de «la fase 1 es la
arquitectura».

**Ronda 2 · el PR a `orbit` sube de la fase 3 a la fase 0.** En la ronda 1 estaba en la fase 3, con
el argumento de que «es el momento correcto: ya se sabe qué falta porque se ha intentado usar». Ese
argumento ya no vale: **ya se sabe qué falta**, está auditado y ejecutado (§1, §9b), y esperar tres
meses significa escribir en las fases 1-3 el código que el PR haría innecesario — la cuarentena de
parsers de texto, el apaño de `doctor --fix`, y el parser de logs con heurística de glifos. Se
adelanta.

### Fase 0 · El PR a `orbit` (1 semana, en paralelo con la 1)

**Entra:** los cinco cambios de §9b, en dos PR separados porque tienen públicos distintos.

- **PR-A (código):** 9b-1 (`doctor --fix --json --yes`), 9b-2 (`backup list/verify --json`) y
  9b-4 (`tests/cli_test.sh`). Los tres son pequeños, los tres son defendibles por sí solos y
  **9b-4 es el que impide que vuelva a pasar lo de 9b-1**.
- **PR-B (documentación):** 9b-5, la promesa de estabilidad del contrato. Cero líneas de código y
  es el de mayor valor a largo plazo, porque es lo único que el cliente no puede hacer solo.
- **9b-3 (`logs --json`) va aparte y después**, porque es el único que invierte un defecto
  (`--follow` con `--json`) y merece su propia discusión sin arrastrar a los otros cuatro.

**No entra:** cambiar `pick_app`, tocar códigos de salida, `--json` en las mutaciones. §9b-6.

**Por qué en paralelo con la fase 1 y no antes:** la fase 1 no depende de ninguno de los cinco.
Si el PR se acepta, la fase 3 se simplifica; si se retrasa, la fase 3 lleva sus apaños y se quitan
después. **Ninguna de las dos ramas bloquea a la otra**, que es la propiedad que hace que se pueda
adelantar sin riesgo.

### Fase 1 · «Ver» (2-3 semanas) — lo pequeño y demostrable

**Entra:** un servidor (leído de `~/.ssh/config` por su alias), `OrbitClient` con `ControlMaster`,
`version` + `status` + `list` + `info`, dos pantallas (lista y detalle), la clasificación de estado
de §7.3, los tipos generados, el `orbit` falso, y las unitarias del parser.

**No entra:** ninguna mutación. Ni un botón que cambie nada. Multi-servidor. Logs. Despliegue.
Actualizador. Firma.

**Por qué así:** con `status --json` (una llamada) ya se puede pintar la pantalla que da valor desde
el minuto uno: la lista de apps con quién está caída, quién está en mantenimiento y —lo que ninguna
otra herramienta enseña— **quién está registrada pero sin vhost**. Eso ya es útil, ya se puede
enseñar, y valida la decisión de transporte con datos reales antes de haber escrito diez pantallas
encima.

**Criterio de terminado, ahora con número.** Con las medidas de §5.0 y §3.2 se puede escribir sin
ambigüedad, que es lo que hace que un criterio sirva:

- Contra un VPS de 40 apps con la conexión caliente, la portada (`status --json`) carga en
  **≤ 600 ms** de punta a punta. Presupuesto: 388 ms de servidor + 13 ms de canal + RTT + pintado.
- El detalle de una app (`info --json`) en **≤ 250 ms**, o sea **sin esqueleto** (§5.0).
- El sondeo de salud (`version --json`) en **≤ 150 ms**.
- Contra el `orbit` falso, las **ocho** clasificaciones de §4.4 se ven en pantalla, cada una con su
  mensaje propio.
- Y `check-contract.sh` (§9.1) pasa contra el `orbit` de la última etiqueta.

### Fase 2 · «Desplegar» (2-3 semanas)

**Entra:** el sistema de tareas (§3.5), `deploy <app> --json --progress` con barra de seis pasos y
estimación por `build_median_s`, `deploy --all --json` con los seis finales pintados **separados**,
`rollback <app> <release>` (con la lista de releases de `info`), `restart/start/stop`,
`maintenance on/off`, la tabla de invalidación de §5.5 y los *optimistic updates* de los dos únicos
sitios donde valen.

**No entra:** `new`, `remove`, `ssl`, logs, terminal.

**Por qué así:** es el núcleo del producto. Y `deploy --all` con los seis finales bien separados es
la pantalla que justifica el proyecto: el propio ARCHITECTURE cuenta que confundir `unchanged` con
`unreachable` costó un bug real. Un cliente que los enseñe distintos hace algo que la salida en
prosa no hacía.

### Fase 3 · «Operar» (3 semanas)

**Entra:** logs (ventana y vivo, §1.11), `doctor --json` con botones sólo donde `fixable`,
`env list` + `env get` (con el valor tras un clic deliberado y un aviso), `env set/unset`, `top`
con el **muestreo adaptativo** de §5.4 —no un intervalo fijo de 2 s, que con 40 apps es
imposible—, `metrics`, `traffic`, la terminal embebida de `exec`.

**No entra:** `new`, `remove --purge`, backups.

**Depende de la fase 0.** Si PR-A se aceptó, el botón de «arreglar» del diagnóstico es un
`doctor --fix --json --yes` limpio; si no, es el apaño de §1.6b (ejecutar sin `--json`, con stdin
conectado, responder `s\n`) **con un aviso en la interfaz** que dice qué versión de Orbit hace
falta para la vía limpia. Las dos rutas se implementan detrás de la misma función, y la elección se
hace mirando `version --json`. Que haya que escribir las dos es exactamente el coste de que la
fase 0 no se hubiera adelantado.

### Fase 4 · «Administrar» (3 semanas)

**Entra:** multi-servidor completo (§6), importación de `~/.ssh/config`, `new` con su asistente de
dos pasos (§1.14), `remove` con la doble confirmación, `ssl`, `redirect`, `db list`, `queue`,
`watch`, `autodeploy`, y las copias de seguridad — **con `backup list --json` si PR-A se aceptó, y
si no con el parser en cuarentena de §7.1 R-01, que se borra el día que exista**.

**No entra:** nada de empaquetado todavía. Se sigue distribuyendo como `pnpm tauri dev` para el
equipo.

### Fase 5 · «Distribuir» (2 semanas)

**Entra:** la matriz de CI, firma en las tres plataformas, notarización, el actualizador con
minisign, los iconos, el instalador, la primera release pública `0.5.0`.

**No entra:** repositorio APT. Homebrew cask. Winget.

**Por qué al final y no al principio:** porque firmar y notarizar es un trabajo de dos semanas que
no se puede paralelizar bien y que no aporta nada mientras el producto no esté. Y porque hacerlo con
el producto terminado permite probar la actualización de verdad (0.5.0 → 0.5.1) en vez de con una
app vacía. **El riesgo es conocido**: si la cuenta de desarrollador de Apple o Azure Trusted Signing
tardan, la fase 5 se alarga. Por eso las cuentas se piden **en la fase 1**, aunque no se usen hasta
la 5. Es lo único de la fase 5 que se adelanta.

### Fase 6+ · lo que queda, sin fecha

Diferencias entre servidores (comparar dos), acciones en lote sobre varias apps, gráficas
históricas (que exigirían guardar algo en el cliente, lo cual es una decisión aparte), integración
con el `orbit.json` de `orbit init`, y el `orbit-desktop-tui` del §2.4.

---

## 11. Puntuación

**Ronda 1: 73 / 100. Ronda 2: 86 / 100.** Se evalúa la **propuesta**, no un producto que todavía no
existe. Lo que ha subido la nota no es escribir más: es **medir** lo que estaba supuesto y
**convertir en mecanismo** lo que estaba en disciplina.

**Y una corrección antes de la tabla, porque si no la nota no vale nada: la tabla de la ronda 1 no
sumaba.** Sus nueve celdas daban 84 y el titular decía 73; el hueco eran las cuatro debilidades
(−14) restadas por fuera, o sea contadas dos veces. Es exactamente el fallo que
`ARCHITECTURE §13.6d` cuenta sobre sus propias cifras —*«ninguna cambiaba la decisión, las dos
habrían pasado tres revisiones»*— y por eso se dice en vez de arreglarse en silencio. Aquí las
celdas **son** la nota: cada una ya lleva descontado lo que le falta, y las columnas suman lo que
dice el titular. Las debilidades de abajo **explican** los puntos perdidos; no vuelven a restarlos.

| Dimensión | Peso | R1 | R2 | Qué ha cambiado |
|---|---|---|---|---|
| Fidelidad al contrato real | 20 | 16 | **19** | Banco de 40 apps ejecutado: latencias, escalado, tamaños. Los cinco hallazgos de la ronda 1 confirmados de forma independiente (E1-E5). Pierde 1: el banco no tiene systemd, así que todas las cifras son un suelo |
| Elección de stack | 15 | 11 | **12** | La contradicción con UX resuelta midiendo (§2.5), con el argumento del monitor refutado y el del arranque aceptado. Pierde 3 por los tres WebView, que se mitiga y no se elimina |
| Transporte SSH | 15 | 11 | **13** | Apretón de manos medido (246 ms sin multiplexado, 13 ms con él), Windows calculado, las 25 reglas de QA mapeadas, presupuestos con cifras, y la superficie del socket de control **verificada** (§3.11). Pierde 2 por Windows, que se acepta y no se resuelve |
| Modelo de datos | 15 | 11 | **12** | §4.4 deja de ser una apuesta **si** 9b-5 se acepta, y el texto está escrito. Pierde 3 mientras no lo esté |
| Estado y caché | 10 | 7 | **10** | Rehecha entera con medidas: `top` no puede ir a 2 s, `metrics` cuesta 936 ms, el ahorro de §5.2 es del 45 % verificado, e invalidación exhaustiva exigida por el compilador |
| Multiservidor | 5 | 3 | **4** | Sin cambios de diseño, pero los semáforos ahora tienen detrás el coste real de un apretón de manos (§3.2), que es lo que los justifica |
| Estructura y dependencias | 10 | 6 | **9** | Las reglas son ficheros de `semgrep`/`clippy`/`eslint`/CI, no intenciones. Y de 33 dependencias de npm a 21 tras §2.5 |
| Build y distribución | 5 | 4 | **4** | Sin cambios |
| Pruebas | 5 | 4 | **3** | `check-contract.sh` sigue siendo la mejor pieza y se añade el mapa de las 44 respuestas patológicas (§9.4b). **Baja un punto**: la ronda 2 ha demostrado que las cifras del banco sin systemd son un suelo, o sea que la ausencia de CI contra un servidor real es más grave de lo que la ronda 1 creía |
| | **100** | **73** | **86** | |

**La casilla de Pruebas baja y eso es deliberado.** Medir mejor no sólo sube notas: también enseña
dónde la propuesta era optimista. La ronda 1 daba 4 sobre 5 a un plan de pruebas que no toca un
servidor real; la ronda 2 ha comprobado que sin systemd corriendo, `top`, `list` y `doctor` miden
**menos** de lo que costarán, así que ese hueco vale más de lo que se le puso.

### Qué cerró cada una de las cuatro debilidades

**1 · «Nada probado con 40 apps» (−4 → 0).** Cerrada. Y no sólo confirmó lo que se suponía: cambió
tres decisiones.

- **Apareció un suelo de 76 ms por llamada** que no está en ninguna documentación de Orbit. Son
  13.720 líneas de Bash parseadas cada vez, más el catálogo de i18n. Con `ControlMaster` costando
  13 ms de canal, **el 85 % del coste de la llamada más barata del contrato es arrancar el
  intérprete**. Cambia el presupuesto de §5.0 entero.
- **`top --json` cuesta 2.116 ms con 40 apps, no 1.000.** El plan de la ronda 1 —polling cada 2 s,
  copiado del `TOP_INTERVAL` del servidor— era **físicamente imposible**. Sustituido por muestreo
  adaptativo con suelo de 2 s y factor 1,5 sobre la duración observada (§5.4).
- **`metrics --json` cuesta 936 ms con el histórico vacío**, porque hace ~7 procesos por app. Su
  `staleTime` sube de 60 s a 5 minutos.
- Y la recta de `list --json` (**72 ms + 5,9 ms por app**) permite decir con número que no hace
  falta paginar hasta bien pasadas las 100 apps.

**2 · «La política de contrato 2 es una apuesta» (−3 → −2).** **Medio cerrada, y es honesto decir
por qué no del todo.** La ronda 1 concluyó que lo valioso no es código del cliente sino que `orbit`
escriba qué sobrevive a una subida de `schema`. Ese texto está redactado, con sus tres garantías
permanentes, listo para PR (§9b-5). **Pero un texto propuesto no es un texto aceptado**, y hasta
que lo esté, §4.4 sigue apoyándose en prudencia y no en una promesa. Se recupera un punto por
tenerlo escrito y accionable; los otros dos los da la aceptación del PR, no este documento.

**3 · «Windows es ciudadano de segunda» (−3 → −1).** Cerrada como se pedía: midiendo. El apretón de
manos son **246 ms en localhost** y ~370 contra un VPS a 30 ms. La lectura útil es que **duele donde
menos se esperaba**: la portada sólo empeora un 90 %, pero el detalle de una app se multiplica por
4,6 y cruza el umbral de los 150 ms, o sea que pasa de instantáneo a necesitar esqueleto. Se acepta
y se compensa con tres cosas concretas (§3.7), y el truco del intérprete persistente se descarta con
su motivo escrito para que nadie lo reabra: rompe la separación de canales o escribe en el servidor,
y las dos son invariantes.

**4 · «Disciplina, no mecanismo» (−4 → 0).** Cerrada, y era la más barata como se anticipó.
`MutationKind` sale de `OrbitCommand` con `Extract`, `mutates()` es un `match` exhaustivo en Rust
que no compila si falta una variante, `Record<MutationKind, InvalidationSet>` obliga a declarar la
invalidación, y **el error del compilador nombra la mutación que falta** (§5.5, con los dos errores
de `tsc` escritos). Y las convenciones de §7.1 son ahora seis reglas ejecutables —`semgrep`,
`clippy.toml`, `eslint`, `git diff --exit-code`— de las cuales sólo una hay que escribir a mano.

### Los 14 puntos que faltan, y de dónde sale cada uno

Suman exactamente lo que le falta a la tabla, para que no haya un descuento escondido en ninguna
parte.

**a · La aceptación del PR a `orbit` no depende de este documento (−4: datos −3, estructura −1).**
§9b es la palanca más grande que hay. Con PR-A y PR-B aceptados: desaparecen los tres huecos del
contrato, se borra la cuarentena entera de parsers de texto (§7.1 R-01 se queda sin habitantes) y
§4.4 pasa de apuesta a política justificada por una promesa escrita. **Este documento puede
escribir el PR; no puede aceptarlo.** Con los cinco cambios dentro, la nota sube a **90**: datos
12→15, estructura 9→10.

**b · Los tres WebView (−3: stack).** Es el precio de Tauri y está aceptado con los ojos abiertos.
Las capturas en CI en las tres plataformas (§9.4) detectan la divergencia, pero la detectan
**después** de escribirla. La única forma de eliminarlo sería Electron —un solo motor— y eso cuesta
el punto 2 de §2.3, que es el que hace coherente al producto con lo que Orbit predica. **Se paga el
riesgo a cambio de la coherencia**, y se dice en vez de esconderlo. Svelte lo reduce un poco (§2.5,
tercer argumento) pero no lo quita.

**c · Windows sin `ControlMaster` (−2: transporte).** Medido, no estimado: **246 ms de apretón de
manos por llamada**, que multiplica por 4,6 el detalle de una app y lo saca del umbral de los
150 ms (§3.7). Se acepta y se compensa con tres cosas concretas, y el truco que lo resolvería —el
intérprete persistente— se descarta con su motivo escrito, porque rompe la separación de canales o
escribe en el servidor. **No es una debilidad que se pueda cerrar desde aquí**: la cierra
Microsoft, o un `ssh.exe` empaquetado.

**d · Sin CI contra un servidor real (−2: pruebas).** El banco de 40 apps es una copia del script
con `ETC_DIR` redirigido y **sin systemd corriendo**, así que todas las cifras de §5.0 son un
**suelo**: en un servidor de verdad, cada `systemctl show` de `top` y cada `systemctl is-active` de
`list` cuestan más. La lista de comprobación manual de `docs/QA.md` lo tapa, pero una lista manual
es una lista que alguien tiene que acordarse de ejecutar. **Qué haría falta:** un VPS con Orbit
instalado de verdad y 40 apps, y un trabajo nocturno que ejecute las siete consultas y publique las
medianas. Medio día de trabajo, y convierte §5.0 en un panel en vez de en una foto.

**e · Las cifras son un suelo (−1: fidelidad).** Consecuencia de la d, contada aparte porque afecta
a otra dimensión: la auditoría del contrato es completa y verificada, pero **las latencias que la
acompañan se han medido en condiciones más favorables que las reales**. La forma de las respuestas
no cambiará; los milisegundos, sí.

**f · Multiservidor sin ejercitar (−1: multiservidor).** Los semáforos (6 globales, 2 por servidor)
y el sondeo por visibilidad son la respuesta correcta, pero **están razonados, no probados con 40
servidores**. El número 6 sale de cuántas firmas puede atender un `ssh-agent` sin ser el cuello de
botella, y eso es una estimación. **Qué haría falta:** 40 contenedores con `sshd` y medir dónde se
degrada. Media jornada.

**g · Distribución sin ensayar (−1: build).** La matriz, la firma, la notarización y el actualizador
con minisign están completos sobre el papel y con los secretos nombrados, pero **nada de eso se ha
ejecutado**. La firma de macOS y Azure Trusted Signing son los dos sitios donde el papel y la
realidad más divergen. Se mitiga pidiendo las cuentas en la fase 1 aunque no se usen hasta la 5
(§10), que es lo único que se puede hacer por adelantado.

### Y una nota de método, porque es lo que ha movido la nota

De los trece puntos que ha subido este documento entre la ronda 1 y la 2, **once salen de ejecutar
cosas** y sólo dos de pensarlas mejor. El suelo de 76 ms, los 2,1 s de `top`, los 936 ms de
`metrics`, los 246 ms del apretón de manos, la identidad de los dos arrays de `status` y `list`, y
el socket de control reutilizable sin credencial: ninguno de los seis se puede deducir leyendo, y
los seis cambian una decisión.

Es la misma lección que el propio `ARCHITECTURE §13.6d` dejó escrita después de medir sus forks:
*«las dos primeras cifras que escribí eran falsas… ninguna cambiaba la decisión, las dos habrían
pasado tres revisiones —una cifra dentro de un comentario no la comprueba nadie— y las dos se
cayeron al medir de verdad»*. Aquí ha pasado exactamente lo mismo con el intervalo del monitor: los
2 s venían de copiar el `TOP_INTERVAL` del servidor sin comprobar que ese número es del panel en
vivo y no del `--json`. Habría pasado tres revisiones. Se cayó al primer cronómetro.
