# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
El cliente sigue [versionado semántico](https://semver.org/lang/es/) con la regla
de [docs/CLIENT.md](docs/CLIENT.md) §8.5: **el minor se reserva para los saltos
de compatibilidad de contrato**. Lo de abajo no cambia el contrato —Orbit sigue
en el 1, porque todo se ha *añadido*—, así que es un parche.

## [0.1.2-rc.1] - 2026-11-10

**Candidata a versión** de la 0.1.2. Puesta al día con **Orbit 1.5.1**, una
auditoría del cliente entero y, encima, **dos rondas de usuarios adversos**: seis
perfiles que intentaron romperlo —un servidor hostil, alguien que controla los
argumentos, un administrador con prisa y doce servidores, un auditor del
contrato con la red mala, una usuaria nueva con lector de pantalla y sólo
teclado, y un obseso del rendimiento—, cada hallazgo con su prueba, y una
segunda ronda contra los arreglos de la primera. El contrato sigue en el 1
—Orbit no ha cambiado ni un emisor entre la 1.4.9 y la 1.5.1—, así que es un
parche, aunque traiga pantallas nuevas. Ver
[docs/CONTRACT.md](docs/CONTRACT.md) §1.17.

Lo que salió de las rondas va primero, por gravedad. Lo de la auditoría va
después, como estaba.

### Corregido — en la ventana de verdad no se podía crear una web, instalar Orbit ni desplegar todo

El envoltorio **no declaraba ninguna capacidad de Tauri**. Con la lista vacía,
Tauri 2 deja pasar las órdenes propias pero rechaza todo lo de sus módulos, y
escuchar el progreso es uno de ellos: el `listen` fallaba antes del `try`, y
crear, instalar y la pasada no llegaban a empezar; un despliegue suelto corría
sin enseñar nada. Ninguna prueba lo veía, porque todas simulan el puente. Ahora
`capabilities/default.json` concede a la ventana `main` **sólo** escuchar y dejar
de escuchar eventos, una prueba falla si concede algo más, y si escuchar falla
la orden se lanza igual, sin salida en vivo.

### Seguridad

- **Con fish como shell de login se salía de las comillas.** fish lee `\'` y
  `\\` como escapes dentro de comillas simples, y el escapador dejaba las barras
  dentro: `\'; id #` ejecutaba `id`. Ahora toda barra y toda comilla van fuera,
  donde los cinco shells las leen igual, y la integración continua instala fish
  y lo prueba.
- **Un correo con un salto de línea acababa ejecutándose como root**:
  `orbit new --email` lo escribe en `/etc/orbit/orbit.conf`, que root lee con
  `source`. Correo, repositorio, rama, motivo de congelación y los ajustes de
  build (`--build`, `--start`, `--outdir`, `--appdir`, `--type`, que no se
  validaban: un salto en `--start` metía una línea en la unidad de systemd)
  rechazan ahora blancos de control y saltos.
- **Una app llamada como una palabra de Orbit hacía otra cosa.** Con
  `autodeploy checks`, una app llamada `no`, `on`, `si`… no se reconocía como
  app y Orbit elegía **la primera por orden alfabético**; `lock`, `unlock` y
  `remove` de una app `help` enseñaban la ayuda y salían con 0; `metrics prom`
  devolvía Prometheus. El cliente se niega, dice por qué y cómo hacerlo en el
  servidor.
- **`exec` en modo «argumentos» pasaba por `bash -lc`** cuando era un solo
  argumento con un metacarácter (`ls;id`): Orbit lo decide así. Ahora va como
  `env -- …`, que lo ejecuta tal cual o falla, y si además lleva un `=` se
  rehúsa, porque `env` enseñaría el entorno de la app.
- **El cliente de terminal pintaba las secuencias de escape del servidor**: un
  servidor podía redibujar las filas de los demás, cambiar el título de la
  ventana o escribir en el portapapeles (OSC 52). Todo lo que viene del
  servidor pasa ahora por un filtro de caracteres de control y de bidi.
- **El registro de servidores se creía lo que tenía dentro.** Una clave en una
  ruta UNC (`\\otro\recurso\clave`) hacía que Windows mandara las credenciales
  NTLM a otra máquina; un binario relativo saltaba la regla de la ruta
  absoluta. Se valida al guardar y al leer, y la interfaz ya no puede cambiar el
  binario de un servidor guardado ni apuntar `ssh` a un alias que no sea tuyo.
- **En Windows se lanzaba el primer `ssh` del `PATH`**, que con Git instalado
  suele ser el de MSYS: su arranque se come las barras invertidas y un
  argumento escapado llegaba cambiado. Ahora el de OpenSSH del sistema si está
  (`ORBIT_DESKTOP_SSH` lo cambia).
- La comprobación previa de la instalación va dentro de `sh -c`: con fish o
  csh, todas las comprobaciones salían «no».

### Corregido — el contrato

- **El asistente de web nueva daba por rota toda app de Node, Python o Go
  recién creada**: comparaba con `active`, la palabra de systemd, y Orbit dice
  `running`. Era el mismo fallo que la lista ya tenía corregido.
- **Nada comprobaba la forma de lo que contestaba el servidor.** Un `ok:"false"`
  salía «Desplegada»; una CPU que faltaba, «undefined %»; un nombre numérico o
  un estado ausente dejaban la vista en «Cargando…» para siempre. Ahora cada
  respuesta pasa por un validador, contrastado con los emisores reales de la
  1.3.6 a la 1.5.1, y lo que falta o viene con otro tipo es un fallo que dice
  qué orden y qué campo (`status: apps[3].name debería ser texto y es un
  número`). El núcleo exige además `schema: 1` y que `ok` y el código de salida
  digan lo mismo.
- **Un fallo al pintar ya no deja la ventana a medias**: cada vista y cada
  pestaña llevan una frontera que dice qué no se ha podido pintar y deja
  reintentarlo.
- **Las órdenes que cambian algo se cortaban a los 30 s**, y cortar `ssh` no
  para la orden en el servidor: un `rollback` que esperaba al cerrojo de un
  despliegue en curso revertía después la release recién puesta mientras la
  ventana decía «no ha contestado». Ahora su plazo es más largo que el cerrojo,
  la pasada tiene el suyo (6 h), `orbit new` ya no se corta en mitad del primer
  build, y si un plazo vence el mensaje dice que **no se sabe** cómo ha quedado.
- **Un error de PHP en `exec` salía como «no he llegado al servidor»** (PHP
  sale con 255), y un `git pull` sin la clave de GitHub como «el servidor puede
  ser un impostor».
- **Pasados 4 MB de salida se cerraba la tubería**, y eso mataba lo que corría
  al otro lado. Ahora se sigue leyendo y se descarta con una marca.
- El monitor ya no se inventa que una app tiene vhost y no está en
  mantenimiento; las copias no se esconden con una versión desconocida; dos apps
  o dos variables con el mismo nombre ya no hacen desaparecer filas; un `.bashrc`
  ruidoso o un Orbit sin `sudo` ya no se confunden con «no está instalado».

### Corregido — carreras y estado

- **Con una hoja de confirmación abierta, la paleta podía cambiar de servidor
  por debajo**: la hoja pasaba a decir «en bravo» y la orden corría en alfa.
  Ahora la hoja lleva el servidor con el que se abrió, y con una abierta no se
  abre la paleta ni se recarga.
- Con una pasada, una web a medio crear o una instalación en marcha, el rail no
  cambia de servidor —«Parar» iba al servidor que se estuviera mirando—, y de la
  instalación no se puede salir sin poder volver.
- Ctrl+R mientras cargaba la portada dejaba el esqueleto para siempre; un
  refresco fallido dejaba la pantalla de fallo aunque el siguiente funcionara;
  la comparación pintaba las apps de un servidor abandonado; el monitor y el
  diagnóstico enseñaban la lista de apps mientras esperaban.
- Recargar el entorno con un valor revelado lo dejaba **al lado del nombre de
  otra variable**; ahora se oculta.
- Mantener pulsada `j` lanzaba un `info` y un `cron` por cada fila; terminar el
  despliegue de otra app desmontaba una confirmación de borrado a medio
  escribir; cancelar un despliegue dejaba muerto el «Cancelar» del siguiente.
- Una app llamada `log` chocaba con la acción «Log de …» en la paleta, y un
  alias que estaba a la vez en el `~/.ssh/config` y guardado a mano tumbaba el
  rail.

### Rendimiento

Medido en un build de producción en Chrome, antes y después:

| | Antes | Después |
|---|---|---|
| Consola de un despliegue, por línea | 30 ms (0–5k líneas), 92 ms (5–10k); 10k líneas, 612 s | 0,024 ms, plana hasta 30k |
| «Desplegar todo», 30.560 líneas | 18,6 s de CPU | 8,6 ms |
| Historial del monitor, 300 apps, por vuelta | 20,6 ms | 0,1 ms |
| Búsqueda en el log, 50k líneas, por tecla | 82 ms | 0,3–0,7 ms (y una pasada tras 100 ms quieta) |
| Eventos de progreso, build de 30.000 líneas | 30.000 | 352 |
| Validar un `status` de 1.000 apps | — | 1,1 ms |

Un despliegue con variables ya no paga un saludo SSH más para saber la versión.
El JavaScript de la ventana pasa de 84,1 a 97,5 KB comprimido, por los
validadores y las pantallas nuevas; las respuestas de ejemplo ya no van en él.

### Accesibilidad y primera vez

- **El foco ya no se cae al `<body>`** al cerrar una hoja, la paleta o una
  vista, al confirmar un despliegue, ni al terminar uno con el foco en su
  salida: va al sitio de antes, al despliegue o al título. Escape cierra una
  capa, no dos.
- Contraste AA en los dos temas: el texto tenue (4,20:1 en el rail) y los chips
  neutros (4,03–4,10:1) estaban por debajo; ahora todo pasa de 4,5:1, con una
  prueba que lo mide sobre cada superficie.
- Los atajos de una tecla (`j`, `k`, `/`, flechas) sólo actúan en la lista: una
  flecha en una pestaña cambiaba de app.
- Una región viva para lo que termina mientras se mira otra cosa, y texto para
  lo que antes era sólo un glifo o un color (pasos del despliegue, requisitos de
  la instalación, chips, la gráfica del tráfico).
- Con la ventana estrecha o al 200 %, el rail pasa arriba y nada empuja la
  página hacia un lado.
- Los formularios de alta y de web nueva dicen qué falta al enviarlos, en vez de
  un botón desactivado que el teclado no encuentra.
- **Un servidor que no contesta dice qué hacer**, en castellano y según el
  fallo de `ssh`: instalar el Cliente OpenSSH en Windows, arrancar el
  `ssh-agent`, la clave rechazada, el nombre que no resuelve…, con lo que dijo
  `ssh` plegado debajo.

### Corregido — contra un servidor real, toda app con proceso salía «no se sabe»

`svc_state` de Orbit escribe **`running`** o `stopped`, y así lo dice el
contrato. Las muestras de las pruebas decían `active` —la palabra de systemd—
y el cliente sólo conocía ésa, en la interfaz y en el núcleo: en la lista, en
el monitor y en el cliente de terminal, una app en marcha era un punto gris.
Llevaba ahí desde la 0.1.0 y ninguna prueba podía verlo, porque todas las
muestras decían lo mismo que el código. `active` se sigue aceptando.

### Añadido — `csp` en el `orbit.json` (Orbit 1.5.1)

El generador del `orbit.json` pide la Content-Security-Policy (no está en el
descriptor: `info` no la devuelve) y la valida con **las reglas de
`_csp_check`, en el mismo orden y con el mismo motivo**. Una que Orbit
rechazaría —y que abortaría el despliegue— no se emite, y se dice por qué.
Con un servidor anterior a la 1.5.1 avisa de que la clave se ignoraría sin
decir nada. Y el fichero se copia con un botón.

### Añadido — la vista del servidor

`status --json`, que ya era la llamada de la portada, trae **el host** —nombre,
IP, tiempo encendido, carga, memoria, disco— **y los servicios del sistema**
(nginx, PostgreSQL, php-fpm, fail2ban, ufw), y el cliente lo tiraba. Ahora se
enseña sin pagar ni un saludo SSH más. Debajo, bajo demanda y cada una con su
fallo aparte: la **vigilancia** (`watch status`), las **copias** con un botón
para **verificarlas** (`backup list|verify`, 1.3.7), las **bases de datos**,
las **redirecciones** y las **colas**. Una carga vacía no se pinta como cero, un
`0` de época es «no se sabe» y no 1970, y con una redirección de dominio no se
enseña el nombre de la app como si fuera el origen.

### Añadido — la paleta de órdenes (Ctrl+K / ⌘K)

Servidores, apps, vistas y acciones, con búsqueda por subsecuencia. No hay nada
que sólo se pueda hacer desde la paleta: lo que cambia algo abre la misma hoja
con la orden literal que el botón.

### Añadido — moverse por la lista sin ratón

Un filtro por nombre, dominio o tipo (`/` para ir a él), un resumen por estados
que también filtra, y flechas o `j`/`k` para elegir la app siguiente. Lo que no
se ve se cuenta: «2 de 40», con un botón para verlas todas.

### Añadido — el monitor dibuja la tendencia, se puede pausar y se ordena

Una línea por app con la CPU y la memoria **de esta sesión**: vive en memoria y
se pierde al cerrar, porque guardar datos del servidor es la decisión que el
roadmap deja fuera. Un `null` corta la línea en vez de unir lo que nadie ha
medido. Se ordena por CPU o memoria (lo que no se sabe, al final), y se pausa.

### Añadido — el log se busca

Búsqueda con el texto resaltado —como elementos, nunca como HTML del
servidor—, recuentos por canal en una sola pasada, y sólo las últimas 1.500
líneas pintadas, con las de arriba a un clic. Una línea con una forma que no es
la del contrato se cuenta como rota en vez de pintarse a medias.

### Añadido — tema claro, oscuro o el del sistema

Desde el rail o la paleta. Dura la sesión y no se guarda: esta interfaz no
escribe nada en el navegador (T-12), y un tema no justifica empezar.

### Corregido — la pantalla del despliegue

- **«Ver el log», «Ir al diagnóstico» y «Volver a…» no hacían nada**: se
  pintaban sin `onclick`. Ahora llevan a su sitio, y revertir sigue pidiendo su
  confirmación en su pantalla. Un botón sin a dónde ir no se pinta.
- **Mientras corría no se veía la salida**: el interruptor estaba dentro del
  resultado. Ahora la consola se ve desde el primer segundo y sigue el final
  mientras nadie suba a leer.
- **No se podía cancelar** desde la ventana, aunque el puente lo sabía hacer.
- **El reloj se paraba** mientras el servidor callaba en mitad de un build, que
  es exactamente lo que parece un cuelgue. Ahora cuenta cada segundo.
- **Se podía lanzar un segundo despliegue encima del primero** de la misma app,
  y los dos mezclaban su progreso. El botón pasa a «Desplegando…» y lleva al que
  corre.
- **El progreso era cuadrático**: cada línea volvía a leer la salida entera, y
  un build de miles de líneas se iba frenando mientras alguien lo miraba. Ahora
  se lee línea a línea, con las mismas reglas (prueba de equivalencia incluida).

### Corregido — un fallo de una pestaña tapaba la lista entera

El log, el entorno, el tráfico, el monitor y el diagnóstico escribían su fallo
en el error **de la portada**: la lista se cambiaba por una pantalla de fallo
hasta cambiar de servidor, y la pestaña que había fallado se quedaba con su
esqueleto para siempre. Ahora cada pestaña y cada vista tiene el suyo, en su
sitio, y el de la portada es sólo de la portada. Y los fallos de crear una web,
de la pasada y de instalar Orbit, que se guardaban y **no se pintaban en
ningún sitio**, se enseñan en su pantalla.

### Corregido — y más

- **Tras «arreglar» en el diagnóstico la lista se quedaba en blanco**: se ponía
  a `null` y no se volvía a pedir.
- **El tráfico de una app sin despliegues medidos no se veía**: con
  `Promise.all`, unas métricas vacías se llevaban por delante el tráfico que sí
  había llegado. Y una app sin visitas es un estado vacío, no un error.
- **Los errores del servidor se pintaban `0`** en el tráfico cuando el campo no
  venía: ahora «·», como todo lo que no se sabe.
- **Comparar y la pasada con la portada sin cargar** decían que el otro
  servidor tenía todas sus apps «sólo allí» y que la pasada iba a tocar cero.
- **Revelar un valor del entorno o ejecutar algo que fallaba** no decía nada.
- **Retirar o revertir con un fallo de transporte** recargaba como si hubiera
  ido bien.
- **El monitor** se despertaba con la ventana oculta y, al volver, hacía
  esperar un periodo entero con datos viejos; ahora pregunta en el acto, espera
  más con cada fallo (hasta un minuto) y dice cuándo lo que se ve es la foto
  anterior.
- **Sin poder leer el `~/.ssh/config` la ventana se quedaba en blanco** en vez
  de abrir el alta.
- **Las hojas de confirmación no eran modales de verdad**: el foco se escapaba
  detrás y no volvía al botón que las abrió. Ahora lo de detrás es `inert` y el
  foco vuelve.
- Recargar lo que se mira, con ↻ o Ctrl+R, sin recargar la ventana (que perdería
  de vista los despliegues en curso).

### Seguridad — el alias de un servidor llegaba a `ssh` como una opción posible

El destino iba sin `--` delante, aunque CLIENT.md §3.2 lo pedía: un alias como
`-oProxyCommand=…` lo leía `ssh` como una opción, y eso es ejecutar algo en el
portátil. Ahora toda orden lleva `--` antes del destino (también `ssh -G`), y el
envoltorio rechaza un alias que empiece por guion o lleve blancos o caracteres
de control, y un binario que no sea una ruta absoluta.

### Seguridad — el instalador clonaba en una ruta fija de `/tmp`

En un servidor con más usuarios, cualquiera podía crear `/tmp/orbit-instalacion`
antes del clon y cambiar `install.sh` justo antes del `sudo bash`: una escalada
a root. Ahora clona en una carpeta de `mktemp -d` (nueva, `0700`, con un nombre
que no se adivina), dentro de `sh -c` para que valga con cualquier shell de
login, y la borra salga bien o mal devolviendo el código del instalador.

### Corregido — el núcleo

- **En Windows, cada orden abría una consola** un instante: ningún proceso
  `ssh` se lanzaba con `CREATE_NO_WINDOW`.
- **Un solo byte que no fuera UTF-8 congelaba el progreso** del despliegue para
  el resto del build, y cortaba la salida final. Ahora se lee con
  `from_utf8_lossy`.
- **Un despliegue fallido podía salir como «Orbit no está instalado»**: se
  miraba la prosa de stderr antes que el objeto, y un build con «command not
  found» en su salida perdía su `failed_step`. Ahora un objeto válido manda, y
  un 127 sólo es «no está» si el shell habla del binario de Orbit. En `exec`,
  una errata o un `sudo` del propio comando llegan como su salida.
- **Dos despliegues de la misma app a la vez** —un doble clic— se pisaban el
  mando de cancelar en el envoltorio. Ahora el segundo se rechaza (`en-curso`),
  y cada uno sólo borra el suyo. También para crear, la pasada y la instalación.
- **Listar y guardar servidores bloqueaba la ventana**, con un `ssh -G` por
  alias y en fila. Ahora fuera del hilo de la ventana y en paralelo.
- **En Windows se buscaba el `~/.ssh/config` en `HOME`**, que normalmente no
  existe: ahora `USERPROFILE` primero, que es donde lo lee Win32-OpenSSH.
- **Un `servidores.json` ilegible se vaciaba al guardar el siguiente**: ahora
  guardar se niega y el fichero queda intacto.
- **`resolver` preguntaba a `ssh -G` por servidores añadidos a mano**, que no
  están en el `~/.ssh/config`: ahora usa su dirección guardada.
- **Opciones de `ssh`** que pedía CLIENT.md y faltaban: `ConnectTimeout`,
  `ServerAliveInterval`/`CountMax`, `ForwardX11=no` y `RequestTTY=no`. Sin
  ellas, una instalación con la red caída esperaba para siempre.
- **Esperar a `ssh` ya no sondea** cada 10–20 ms: se bloquea hasta que contesta
  o vence el plazo. Una orden multiplexada de 13 ms podía pagar 10 más.
- Fuera `saludo::ORDEN_DE_INSTALACION`, la orden con tubería que el propio
  instalador dice que no funciona.

### Cambiado — lo que salió en la 1.3.7 ya no se da por sentado

`logs --json`, `backup … --json` y `doctor --fix --yes` llegaron en la 1.3.7, no
en la línea base. Con un servidor que se sabe anterior, el log y «arreglar» lo
dicen en vez de fallar; con una versión desconocida no se esconden.

### Cambiado — el paquete

La galería del sistema de diseño **ya no viaja en la aplicación**: el
comentario de `vite.config.ts` decía que no entraba y Tauri empaqueta todo
`ui/dist`. Ahora sólo con `npm run build:galeria`. Las respuestas de ejemplo de
las vistas nuevas se cargan bajo demanda y no van en el paquete principal.
Medido con `vite build`: la galería eran 33 KB que ya no van; el JavaScript de
la ventana pasa de 71,7 KB comprimido (0.1.1) a 97,5, por las pantallas nuevas,
los validadores y las consolas.

### Pruebas

- **Las dos rondas adversas dejan sus pruebas**: cada hallazgo vino con una
  sonda que lo reproducía, y las que se han quedado afirman el comportamiento
  bueno. En la interfaz, `ui/tests/rc-*.test.ts` y `rc2-*.test.ts` (carreras,
  formas de respuesta, consolas, contraste, foco, paleta); en el núcleo,
  `rc_ordenes_*`, `rc2_*` y `crates/orbit-app/tests/rc_capacidades.rs`, y la
  prueba de propiedad del escapado con fish.
- Al cerrar la candidata, en Windows: la interfaz, 686 en verde de 699 (las
  10 de `orbitjson-legible` necesitan `jq`, que la integración continua instala,
  y 3 de rendimiento sólo corren a petición); el núcleo, 194; el envoltorio, 17;
  el cliente de terminal, 5; y las de integración que no necesitan el servidor
  falso en bash. `svelte-check`, `clippy -D warnings` y `cargo fmt --check`,
  limpios. La prueba de propiedad, 1.235 casos en bash y dash; zsh, busybox y
  fish, en la integración continua.
- `ui/tests/orbit15.test.ts` (25) y `ui/tests/csp.test.ts` (15): `running`, las
  capacidades, la vista del servidor contra las muestras, la paleta, el filtro, la
  tendencia con huecos, el visor del log y la equivalencia del lector incremental.
- `ui/tests/despliegue.test.ts`: los botones del resultado, cancelar y el reloj.
- En el núcleo y el envoltorio, 17 pruebas nuevas: el `--` delante del destino, las opciones de
  `ssh`, el progreso sin UTF-8, el objeto antes que stderr, el 127, el `sudo` de
  `exec`, la espera bloqueante, el descubrimiento en paralelo, el registro ilegible,
  el despliegue duplicado, la validación del alias y la carpeta privada del
  instalador.

### Lo que no se ha podido hacer, y por qué

- **El selector de repositorios del asistente** sigue sin estar: `orbit github
  repos --json` existe en el script desde la 1.3.7 pero **no se puede invocar**
  —`main` llama a `cmd_github` sin sus argumentos—. El arreglo está hecho en
  Orbit (1.5.2, sin publicar); el selector llega cuando salga.
- **Cancelar no para el build en el servidor**: matar el `ssh` local no manda
  señal al proceso remoto. Hace falta un `deploy --cancel` en Orbit, o volver a
  preguntar con `info` tras cancelar.
- **Windows no multiplexa**: Win32-OpenSSH no tiene `ControlMaster`, y cada
  orden paga el saludo entero. La salida buena es un modo de sesión en Orbit, no
  un `ssh` de MSYS.

## [0.1.1] - 2026-11-05

Puesta al día con **Orbit 1.4.8**, y con lo que la 1.4.9 añadió para este
cliente. El cliente se escribió contra la 1.3.6, y
entre medias Orbit ganó once versiones: la congelación de despliegues, la puerta
de la CI de GitHub, las variables obligatorias que se piden antes del build, las
tareas programadas, los certificados de origen de Cloudflare y el grupo
`orbit-admin`. Todo con las formas sacadas **del Orbit de verdad ejecutado en el
banco**, no de su documentación. Ver [docs/CONTRACT.md](docs/CONTRACT.md) §1.16.

### Seguridad — con Orbit 1.4.9 las variables van por la entrada estándar, no en la orden

Orbit 1.4.9 añadió `deploy --env-stdin`, y con un servidor 1.4.9 o posterior el
cliente lo usa siempre: en la orden sólo va la bandera, y los valores se escriben
por la entrada del proceso `ssh` —una línea `CLAVE="valor"` por variable— y la
entrada se cierra al terminar. No tocan ni `ps` ni el registro de `sudo` del
servidor: cierra THREAT-MODEL T-13. Las comillas dobles son necesarias, porque
Orbit pasa cada valor por `_env_unquote`, que sin ellas corta un ` #` y los
espacios del final. Quien decide el camino es el envoltorio, que le pregunta la
versión al servidor justo antes; si no la puede leer, no manda nada. Con
1.4.3–1.4.8 se sigue usando `--env`, con el aviso de siempre en la hoja.

### Añadido — el detalle dice cuándo una app se despliega sola sin esperar a la CI

Desde Orbit 1.4.9, `autodeploy enable` en un repositorio de GitHub enciende la
espera a la CI por defecto (`--no-checks` para no esperarla). El cliente no
ofrece activar el automático, así que no cambia ninguna orden; pero una app que
ya estaba en automático sin la espera sigue así, y ahora se dice.

### Corregido — una pasada de un servidor 1.4.3 o posterior no se podía leer

El enumerado de los finales de `deploy --all` era cerrado, con los seis de la
1.3.6. Desde la 1.4.3 hay once —`locked`, `ignored`, `ci_pending`, `ci_failed`,
`ci_unknown`—, y **una sola app congelada convertía la pasada entera en «no he
entendido la respuesta»** en el núcleo y en el cliente de terminal. Ahora son
once, y un final que este cliente no conozca llega con su nombre en vez de tumbar
el lote: un valor nuevo de un campo que ya existía es la misma clase de añadido
que un campo nuevo.

### Corregido — la ventana se congelaba mientras esperaba al servidor

Casi todo el catálogo eran comandos de Tauri **sin `async`**, que corren en el
hilo de la ventana. Contra el servidor falso no se notaba —contesta en
milisegundos— y contra uno de verdad sí: `doctor` son 1,4 s con 40 apps, `top`
2,1 s en cada refresco del monitor, y todo ese tiempo la ventana no atendía un
clic. Y las peticiones iban en fila aunque la interfaz las lanzara a la vez: el
tráfico y las métricas, que se piden juntas, esperaban la suma de las dos. Ahora
todo lo que habla con un servidor corre fuera del hilo de la ventana, cada orden
en el suyo.

### Corregido — cambiar de servidor a mitad de una carga pintaba las apps del anterior

La portada de un servidor que tardaba más que la del siguiente llegaba después y
**se pintaba bajo el nombre del otro**, que es el accidente más caro de un cliente
multiservidor provocado por la red y no por nadie. Lo mismo con el log, el
entorno, el tráfico, el monitor y el diagnóstico. Cada respuesta se descarta
ahora si ya no es del servidor y de la app que se están mirando.

### Corregido — el monitor podía acumular peticiones

Iba con `setInterval`: un `top` que tardara más que el periodo dejaba otro
esperando detrás, y otro. Ahora hay una sola petición en vuelo y la siguiente se
programa cuando termina la anterior, con el periodo que ella misma midió.

### Corregido — el resultado de un despliegue desaparecía al llegar

Al terminar, la portada se recargaba con la misma función que se usa al cambiar
de servidor, y esa función **deselecciona la app**: el panel con el resultado se
cerraba justo cuando había algo que leer. Ahora se refresca sin soltar la app, y
lo mismo tras revertir.

### Corregido — `retirar` y `revertir` devolvían una salida vacía

Sin `--json`, Orbit cuenta lo que ha hecho por stdout (`_ui_route`), y el puente
devolvía sólo stderr. Ahora va entera.

### Corregido — en macOS no se multiplexaba nunca

El directorio de los sockets de `ControlMaster` salía de `XDG_RUNTIME_DIR`, que
macOS no define, así que **cada orden pagaba el saludo SSH entero** (246 ms en
vez de 13, medidos). Ahora cae a `~/.ssh/orbit-desktop`, con la carpeta
comprobada —que no sea un enlace, que sea del mismo dueño, `0700`— y no sólo
creada. No `$TMPDIR`: con el hash de `%C` y el sufijo temporal de `ssh` se pasa
de los 104 bytes de una ruta de socket.

### Corregido — los errores de un servidor sin UTF-8 empezaban por «x »

Orbit escribe `  ✗ mensaje` con UTF-8 y `  x mensaje` sin él (`LANG=C` por SSH,
un VPS recién creado). El cliente sólo quitaba el primero.

### Corregido — el `orbit.json` generado rompía el bloque `env` de un 1.3.8

Desde la 1.3.8 el `env_spec` del descriptor son siete campos separados por US
(`\x1f`), no cinco por tabuladores, y lleva `required` y `default`. El generador
lo partía por tabuladores: la línea entera acababa en el nombre de la variable,
y una obligatoria regenerada habría dejado de parar el despliegue. Lee los dos
formatos y conserva lo que trae el nuevo.

### Corregido — los servidores añadidos a mano no salían en el rail

Se podían usar desde el alta, pero no había forma de volver a ellos ni de ver su
contador de despliegues en curso. Tampoco contaban para «comparar».

### Añadido — congelar y descongelar (`orbit lock|unlock`, 1.4.3)

Desde el detalle, con el motivo, y con la orden literal antes de lanzarla. Una
congelada se marca en la lista como faceta y no como estado —atiende igual—, y
el detalle dice quién, cuándo y por qué; un `touch` a mano del testigo no dice
quién, y no se inventa. Desplegar una congelada es **otro botón**, «Desplegar
igualmente», que pasa `--force` y no la descongela. La pasada dice antes de
lanzarse cuáles se va a saltar.

### Añadido — las variables obligatorias se piden antes de desplegar (1.3.8)

Antes de lanzar un despliegue se pregunta `orbit env check --json` —sin un solo
valor— y, si falta una obligatoria, la hoja la pide y la manda con `--env`. Sin
esto, con `--json` no hay a quién preguntarle y el servidor se para antes del
build. Si aun así se para —el commit nuevo declara una que la release que sirve
no tenía—, el resultado lo dice con su nombre y ofrece darla y volver a
desplegar. **El valor viaja en el `argv`**: se ve en `ps` del servidor mientras
dura el despliegue y, si no se entra como root, en el registro de `sudo`. Orbit no
tiene hoy otra vía sin escribir un fichero en el servidor; la hoja lo dice y
[docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) T-13 lo cuenta.

### Añadido — la puerta de la CI (`orbit autodeploy checks on|off`, 1.4.3)

Desde el detalle, y la pasada pinta sus tres finales por separado: esperando,
en rojo y **sin respuesta**, que es el `unreachable` de la CI y pone la pasada en
rojo.

### Añadido — el certificado de origen de Cloudflare se ve (1.4.7)

`state.cert_type`: «HTTPS · origen CF» en la lista y la explicación en el
detalle, porque sólo vale detrás del proxy y no lo renueva certbot. El detalle
pide ahora `info` al elegir una app, que trae los días que le quedan —`list` no
los calcula—, la comprobación de salud y los límites.

### Añadido — las tareas programadas (`orbit cron list --json`, 1.3.9)

En el detalle, con la última y la próxima pasada. Un `null` es «nunca».

### Añadido — `orbit admin` en el mensaje de sudo (1.4.6)

Cuando el usuario SSH necesita contraseña para sudo, el mensaje dice cómo se
arregla: `orbit admin add <usuario>`, una vez y como root. Y que equivale a root,
porque Orbit también lo dice. El cliente no cambia cómo se eleva: invoca
`/usr/local/bin/orbit`, que se eleva solo con la ruta que nombra la regla.

### Cambiado — lo que un servidor viejo no dice no se lee como un «no»

Los campos que llegaron después de la 1.3.6 son opcionales en los tipos, con un
nivel más de `Option` que su tipo del contrato: que un 1.3.6 no diga `locked`
**no es «no está congelada»**, es que no sabe congelar. Lo que depende de una
orden nueva se ofrece según la versión de Orbit (`Capacidad`, `sabe()`), pedida
una vez por servidor y sesión en paralelo con la portada; lo que se lee de una
respuesta manda que el campo esté.

### Pruebas

- `crates/orbit-client/tests/orbit14.rs`, 16, contra respuestas del Orbit 1.4.8
  de verdad, y `tests/ordenes.rs`, que compara con la interfaz las órdenes que se
  enseñan —`tests/contrato/ordenes.json`, con los valores tapados—.
- `ui/tests/orbit14.test.ts`, 37, y `ui/tests/carreras.test.ts`, 2, con un
  servidor lento de verdad. Comprobadas mutando el código: enseñar el valor en la
  orden, quitar el final «negado», quitar el turno de la portada o volver al rail
  de antes las ponen en rojo.
- `ui/tests/orbitjson-legible.test.ts` ejecuta también la expresión de
  `_read_env_block` de la 1.4.8 contra el fichero generado.
- El servidor falso gana un caso `orbit-1.4` y cuatro patológicos; los de la 1.3.6
  se quedan y siguen en verde, a propósito. Y un caso `orbit-1.4.9`, que lee
  `--env-stdin` con el `_env_unquote` de Orbit copiado tal cual y sale con 99 si
  la entrada no se cierra en cinco segundos.
- `crates/orbit-client/tests/entrada.rs`, 5: en el `argv` del servidor no hay ni
  un valor ni un `--env`, lo leído por la entrada es lo escrito, la entrada se
  cierra, y un servidor anterior se niega sin que el valor toque el `argv`.
  Comprobadas mutando el código (sin comillas, o sin cerrar la entrada: en rojo),
  más una de punta a punta por `sshd`, con y sin multiplexar.
- `tools/bench/check-contract.py` comprueba contra el Orbit de `main` que la lista
  trae los cuatro campos nuevos y que la pasada cuenta los once finales.
