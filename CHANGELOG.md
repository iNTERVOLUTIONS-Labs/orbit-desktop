# Changelog

Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).
El cliente sigue [versionado semántico](https://semver.org/lang/es/) con la regla
de [docs/CLIENT.md](docs/CLIENT.md) §8.5: **el minor se reserva para los saltos
de compatibilidad de contrato**. Lo de abajo no cambia el contrato —Orbit sigue
en el 1, porque todo se ha *añadido*—, así que es un parche.

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
