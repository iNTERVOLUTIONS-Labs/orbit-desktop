# Roadmap de Orbit Desktop

Leyenda: ✅ hecho · 🚧 en curso · 📋 planificado · 💭 en debate

El criterio que ordena todo lo de abajo: **cada fase termina en algo que se puede
enseñar y usar.** Ninguna fase es «la arquitectura». Esa ya está escrita, y está
en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Fase 0 · La biblia 🚧

Escribir las decisiones antes que el código, que es lo que hizo Orbit con su
§13 y por lo que este repositorio existe con esa forma.

- ✅ **Auditoría del contrato `--json` ejecutando el script**, no leyéndolo. De
  ahí salieron cosas que llevaban documentadas y rotas: `orbit doctor --fix
  --json --yes` no existe, y sin TTY un comando sin app no aborta sino que elige
  la primera por orden alfabético y sale con 0
- ✅ **Elección de stack** con las cinco alternativas descartadas por su motivo, y
  la subdecisión que de verdad manda: quién interpreta `~/.ssh/config`
- ✅ **Sistema de diseño** con la paleta real de la marca sacada de los SVG de
  Orbit y los contrastes **calculados**, no estimados
- ✅ **Modelo de amenazas** con un actor que casi nadie modela: el servidor
  gestionado que devuelve lo que quiera al cliente que tiene las llaves de los
  otros cuatro
- ✅ **Plan de pruebas** con el catálogo de respuestas patológicas, que es la
  parte que caza los bugs de verdad
- ✅ **El banco de 40 apps**, para que las cifras de latencia sean medidas
- ✅ **La prueba de propiedad del escapado** contra cuatro shells. Encontró un
  fallo real en su primera ejecución, y sólo en zsh
- ✅ **El PR a `orbit`** con los cinco cambios que el contrato necesitaba
  ([orbit#1](https://github.com/iNTERVOLUTIONS-Labs/orbit/pull/1)): `make
  test-strict` en verde con 2.524 comprobaciones y la salida en prosa idéntica
  byte a byte

## Fase 1 · Ver 🚧

El núcleo está: `crates/orbit-client` con el transporte, el contrato tipado, el
catálogo de órdenes y el escapado, con 90 pruebas rápidas y 11 de punta a punta
contra un `sshd` de verdad. Y la interfaz tiene sus dos pantallas —lista y
detalle— con 33 pruebas y capturas en los dos temas.

Y las dos mitades ya están enganchadas: `crates/orbit-app` es el envoltorio de
Tauri —delgado a propósito, sólo declara el catálogo de comandos— y la interfaz
descubre servidores leyendo los alias de `~/.ssh/config` y preguntándole a
`ssh -G` qué significan.

Falta el **camino de alta**: hoy se listan todos los alias del fichero, y hace
falta la pantalla que comprueba cuáles tienen un Orbit detrás y con qué versión
de contrato hablan.

Se empezó por aquí y no por una pantalla porque es lo que el plan de pruebas
manda: sin el escapador demostrado, sin el servidor falso y sin el barrido de
secretos, todo lo demás se construye sobre una afirmación.


Un servidor, leído de `~/.ssh/config` por su alias. El transporte con
`ControlMaster`. `version`, `status`, `list`, `info`. Dos pantallas.

**No entra ni un botón que cambie nada.** Ninguna mutación, ni multiservidor, ni
logs, ni despliegue, ni actualizador.

Se puede empezar por aquí porque `status --json` trae el array de apps
**completo e idéntico** al de `list --json` —comprobado comparando los dos
objetos—, así que una sola llamada pinta la pantalla que ya da valor el primer
día: quién está caída, quién está en mantenimiento y, lo que ninguna otra
herramienta enseña, **quién está registrada pero sin vhost**, que es el fallo que
no ve ni `nginx -t` ni `orbit list`.

**Terminado cuando:** contra un VPS de verdad la lista carga en menos de 500 ms
con la conexión caliente, y contra el servidor falso se ven en pantalla las ocho
clasificaciones de estado.

## Fase 2 · Desplegar ✅

La barra ponderada con los seis pasos y su cronómetro, los cuatro finales de un
despliegue, y **el lote con sus seis finales separados**, que es lo que
justifica el proyecto — la salida en prosa no los distinguía y confundir dos de
ellos costó un fallo real.

Y la **hoja de comando**, que enseña la orden literal antes de ejecutarla. Es la
prueba visible de que esto sólo invoca `orbit`, que es la promesa que lo deja
existir.

Y el canal en vivo ya está: el transporte sirve cada línea de `--progress`
**según llega**, no al terminar, con su mando para cancelar. Se pueden lanzar
varios a la vez, en apps y servidores distintos — cada uno es un proceso SSH
independiente y nada en el servidor los coordina, ni falta.

Un despliegue **sobrevive a que alguien se vaya a otra pantalla**: el registro
vive fuera del componente, el servidor lleva su contador en el rail mientras
corre, y al cerrar se avisa de que cerrar **no cancela** — el proceso sigue en
el servidor.

### Lo que había aquí

`deploy <app> --json --progress` con la barra de pasos y la estimación sacada de
`build_median_s`. `deploy --all` con los **seis** finales pintados separados.
`rollback`, `restart/start/stop`, `maintenance`.

Es el núcleo. Y `deploy --all` es la pantalla que justifica el proyecto entero:
confundir «no hay cambios» con «no he podido preguntar» costó un fallo real en
la versión en prosa de Orbit —un remoto caído se anunciaba como «nada que hacer»
cada cinco minutos— y el contrato tiene seis finales justamente para que un
cliente no pueda repetirlo. **Agruparlos en «correctas / fallidas» está
prohibido.**

## Fase 3 · Operar ✅

Adelantada en parte, porque el PR a Orbit la desbloqueó antes de tiempo: ya está
el **log** —con el filtro que separa el de acceso del de error, que es lo que la
salida en prosa pierde porque `tail` los mezcla— y el **diagnóstico**, con el
botón de arreglar que sólo aparece donde `fixable` lo permite y, contra un
servidor que no pueda aplicarlo sin terminal, la orden para copiar en su lugar.

Y también las **variables de entorno** —con la regla que las gobierna: sólo
nombres, cada valor de uno en uno con su `orbit env get` de verdad, y se oculta
solo a los 30 segundos, al perder el foco la ventana y al cambiar de pantalla—
y el **monitor**, con el periodo adaptado a lo que de verdad tarda.

Y el **tráfico** con las **métricas**, que son las dos pantallas cuyo valor está
en lo que **no** afirman: son IPs y no personas, lo automático va aparte —en un
VPS con IP pública es la mayoría—, una ventana que el log ya no cubre se anuncia
recortada, un percentil sin muestras no se pinta como cero, y la tendencia no se
dibuja plana cuando el servidor se la calla.

Y la pantalla de **`exec`**, que es la puerta trasera y se trata como tal: los
cuatro datos siempre a la vista, los dos modos explícitos en vez de la
heurística invisible del servidor, la orden exacta antes de ejecutarla, el
histórico que no toca el disco, y **sin shell interactiva** — se ofrece la orden
para pegarla en un terminal de verdad, que es más honesto que emular medio.

### Lo que había aquí, ya hecho

Logs en ventana y en vivo. `doctor` con botón **sólo** donde `fixable` lo
permita. Variables de entorno, con los nombres a la vista y cada valor tras un
gesto deliberado. `top` refrescándose. Métricas, tráfico, y la terminal de
`exec`.

El PR contra `orbit` que esta fase iba a necesitar **ya está fusionado**
([orbit#1](https://github.com/iNTERVOLUTIONS-Labs/orbit/pull/1)), y se adelantó a
propósito: era el trabajo más rentable del proyecto y no es trabajo de este
repositorio. `--json` en `logs` y en `backup list`, el arreglo de `--yes` en
`doctor --fix --json`, la promesa de estabilidad del contrato escrita, y la
prueba que ejecuta `main()` — la laguna que había dejado un comando documentado
y roto sin que nadie lo notara.

El efecto: esta fase se escribe contra un contrato completo, y el cliente nace
con **cero líneas que parseen texto**.

## Fase 4 · Administrar ✅

Hechas las dos operaciones que cambian algo de verdad.

**Retirar** son dos entradas y no una casilla —una casilla junto a un botón se
marca sin leerla— con la que borra datos en un submenú. Y con el inventario de
lo que se pierde pedido **en ese momento**: «5 releases, el último despliegue y
5 variables de entorno», no un texto genérico. Escribir el nombre se pide sólo
ahí, porque **`orbit remove -y --purge` no pregunta nada** y toda la protección
se traslada a la pantalla.

**Revertir** no pide escribir nada, porque es reversible y la fricción es un
recurso escaso: gastarla aquí enseña a teclear nombres sin leer, y entonces se
teclea también en el borrado. La release se elige de una lista, la activa se
marca y no se ofrece, y los dos avisos van con ella — el de las migraciones
siempre, y el del autodespliegue cuando está puesto.

Y los **siete finales** de una web nueva, que es la parte que el informe de
diseño señaló como la más frágil. `orbit new` no tiene `--json` —sólo prosa y un
código de salida— así que la interfaz **no la interpreta: le vuelve a preguntar
al servidor** con `orbit info --json`, que sí tiene contrato. Cinco de los siete
son parciales, y cada uno dice qué existe, qué falta y qué se deshace.

Y el **alta de servidores**: la lista sale de `~/.ssh/config` **sin hablar con
ninguno** —enumerar no es visitar, y abrir una pantalla no puede significar
abrir cuarenta sesiones SSH— y preguntar por uno es un gesto aparte, con siete
respuestas posibles y cada una diciendo qué hacer. Sin esa clasificación, quien
añade un servidor ve «error» y no sabe si es su clave, su red o su servidor.

Y la **pasada por todas las apps**, que es la orden más cara que este cliente
puede lanzar. Estaba en el catálogo y tenía componente desde la fase 3, pero no
había forma de llegar a ella: los seis finales sólo existían en la galería.

`deploy --all` **significa dos cosas** según lleve `--if-changed` o no —preguntar
a cada remoto y desplegar lo que se ha movido, o recompilarlo todo— así que son
dos entradas y nunca una casilla, con la cara en un submenú. Es la misma
decisión que en la pantalla de retirar y por el mismo motivo: la opción que está
al lado se elige sin leerla, y aquí la de al lado son cuarenta builds.

Leerlo entero destapó que **los seis finales no son seis en las dos ramas**:
`unchanged`, `unreachable` y `gone` salen de preguntarle al remoto, y sin
`--if-changed` no se le pregunta a nadie. Sus recuentos son cero *por
construcción*, que no es lo mismo que «he mirado y no había nada», así que la
pantalla lo dice — y sólo de los que de hecho valen cero, porque una predicción
mía no puede pisar un dato del servidor. `skipped` no puede salir en ninguna de
las dos: sólo lo produce `--auto`, la bandera del autodespliegue, que este
cliente no pasa nunca.

Y el **formulario** del asistente, que cierra la fase: cinco pasos, con la
orden literal en el último — construida por el mismo código que una prueba
compara, contra un fichero compartido, con el catálogo que la ejecuta. Enseñar
una orden y ejecutar otra sería el único fallo que anula por completo a una
pantalla cuyo argumento entero es «mira lo que va a pasar antes de que pase».

Construirlo obligó a **corregir el diseño en tres sitios**, y las tres
correcciones son del mismo tipo: cosas que el informe daba por hechas y que el
servidor no hace.

- El paso 2 iba a enseñar **lo detectado, con su prueba, antes de crear**. No se
  puede: `detect_stack` recibe un directorio y el clon ocurre dentro de
  `orbit new`. Antes de ejecutar no hay conclusión, sólo una promesa — y una
  promesa junto a un botón «cambiar» se lee como un hecho. Se ha partido en dos
  pantallas honestas: adelantarse a la detección antes, y enseñar lo detectado
  después, cuando ya sale del descriptor.
- El **autodespliegue** no es una bandera de `new`, es una orden aparte. Se ha
  quitado del paso 4, y en su sitio hay una línea que dice dónde está.
- `orbit new` **habla por stdout**, no por stderr como el resto: `_ui_route`
  deja `UI_FD=1` sin `--json`. El transporte servía stderr siempre, así que la
  única orden que de verdad tarda habría pasado tres minutos muda.

Otras dos las cazaron las pruebas, no la lectura. Los **alias** van separados por
espacios y se estaban mandando con comas, lo que habría llegado hasta el `-d` de
certbot como un solo dominio con una coma dentro. Y el **tri-estado** de un campo
de detección —«detéctalo» / «déjalo vacío» / «este valor»— cruzaba el puente como
`undefined` / `null` / cadena dando por hecho que serde distinguiría el campo
ausente del `null`: no lo distingue, los colapsa, y `--build ''` no habría
llegado nunca al servidor.

### Lo que había aquí

Multiservidor completo con importación de `~/.ssh/config`. El asistente de web
nueva. Retirar una app, con la confirmación reforzada que el servidor ya no da:
`orbit remove -y --purge` **no pregunta nada**, porque la pregunta vivía en el
terminal que este cliente sustituye. Certificados, redirecciones, bases de datos,
copias, colas y vigilancia.

## Fase 5 · Distribuir 🚧

Firma en las tres plataformas, notarización, y el actualizador con verificación
de firma y la clave pública empotrada en el binario.

**Partida en dos, y sólo una mitad se puede hacer hoy.** Compilar en las tres
plataformas sí: está en `.github/workflows/paquetes.yml`, con sus paquetes y sus
dependencias declaradas —un `.deb` sin `libwebkit2gtk` instala y deja la ventana
en blanco, que parece un defecto de la aplicación y es una dependencia que falta.

Firmar, notarizar y actualizar **no**, y no está escrito a medias a propósito. Un
workflow con tres secretos vacíos y un `continue-on-error` no es media solución:
es una que sale en verde sin haber hecho nada, y el día que existan las
credenciales nadie comprobaría que funciona porque «ya estaba puesto». Lo mismo
con el actualizador: enchufar el plugin hoy sería añadir una dependencia **al
proceso que sostiene las claves SSH del usuario** —el argumento con el que se
descartó Electron— configurada con una clave pública que no existe y apuntando a
un endpoint que tampoco. Se añade cuando haya clave.

La matriz corre en verde en las tres, y produce paquetes de verdad: 1,8 MB de
`.deb`, 1,9 MB de `.dmg`, 1,4 MB de `.exe` y 78 MB de AppImage. **Ése es el
argumento con el que se eligió Tauri sobre Electron, medido por fin**: los tres
primeros pesan eso porque usan el motor web del sistema, y un Electron
equivalente ronda los 85 MB por plataforma.

Ponerla en marcha destapó dos cosas que nadie sabía porque **nunca se había
empaquetado**: el `beforeBuildCommand` estaba mal desde el primer día, y no había
`icon.ico` — sin el cual `tauri-build` aborta, o sea que este proyecto no podía
compilarse para Windows.

Qué hace falta para firmar, secreto a secreto, y en qué orden conviene pedirlo,
está en [docs/DISTRIBUCION.md](docs/DISTRIBUCION.md).

**Va al final, con una excepción.** Firmar y notarizar es un trabajo que no se
paraleliza y que no aporta nada mientras no haya producto; hacerlo con el
producto terminado permite además probar una actualización de verdad en vez de
con una app vacía. La excepción: **las cuentas de firma se piden en la fase 1**,
porque si Apple o Azure tardan, tardan.

## Fase 6 · Mirar dos servidores a la vez ✅

**Comparar dos servidores**: qué hay en uno que no esté en el otro, y en qué se
diferencian las que están en los dos. Dos lecturas, no escribe nada — y aun así
es la pantalla más peligrosa del cliente, porque es la única en la que dos
servidores están a la misma altura.

De ahí salen sus dos reglas, que son las dos que el producto ya tenía por
separado y que se rompen justo aquí:

**La clave es `servidor:app` y nunca la app sola.** «tienda» existe en tres
servidores y son tres apps distintas; el accidente más caro de un cliente
multiservidor no es un ataque, es confundir dos. Así que el alias va escrito en
la cabecera de cada columna y en cada lista, siempre, también cuando parece
obvio. El color es el refuerzo; la señal es el nombre.

**Media comparación no es una comparación.** Si el otro servidor no contesta, no
se compara nada — no se enseña la lista del que sí con los huecos del otro en
blanco. Eso sacaría todas sus apps como «sólo en produccion», que invita a
crearlas otra vez en un servidor donde puede que ya existan. Es confundir «no he
podido preguntar» con «no lo tiene», el mismo error que costó que un remoto
caído se anunciara como «nada que hacer» durante días, y aquí con peores
consecuencias.

Y dos cosas que la pantalla dice de sí misma, porque callarlas haría creer de
más. **La rama y el repositorio no salen en `list --json`**, así que dos apps sin
diferencias podrían venir de repositorios distintos. Y **que dos apps se llamen
igual no las hace la misma app**: si el nombre coincide y el dominio no, se
enseña la duda y no una conclusión, porque quien mira sabe cuál de las dos cosas
es y esta pantalla no.

Lo que cambia solo —el proceso, el puerto, el número de releases, la fecha del
último despliegue— **no se compara a propósito**: difiere casi siempre entre dos
servidores y no dice nada, y una lista de diferencias con ruido es una lista que
se deja de leer. `cert_days` tampoco se puede comparar aunque se quisiera: es
`null` en `list` y en `status` siempre, y sólo lo calcula `info`.

## Fase 6b · El `orbit.json`, escrito desde lo que ya funciona ✅

`orbit init` escribe ese fichero **volviendo a detectar** sobre el repositorio,
en el portátil de quien lo ejecuta. Y ahí está su límite: la detección se
equivoca en un monorepo, con un adaptador o con un arranque propio, y `orbit
init` se equivoca **otra vez igual**, porque hace exactamente lo mismo.

El cliente puede hacerlo al revés, y es la única ventaja que tiene sobre esa
orden: leer el descriptor de una app **que está desplegada y sirviendo**, o sea
la configuración que de verdad funciona, incluidos los campos que alguien
arregló a mano después de que la detección fallara. Eso no lo puede saber un
comando que corre sobre un directorio.

No escribe nada: genera el texto y se copia. Ni en el servidor ni en el
repositorio de nadie — la misma regla que la orden de instalación de la pantalla
de servidores.

**El bloque `env` lleva nombres y no valores**, y no por prudencia: es que el
bloque `env` de un `orbit.json` es una *especificación* —qué variables hacen
falta y cómo obtenerlas— y no un almacén. Es el único sitio del producto donde
un cliente puede ayudar con las variables de entorno sin romper la regla de los
secretos, porque los nombres sí cruzan el contrato y los valores no. El fichero
generado se puede subir a un repositorio público sin pensárselo.

Lo que costó encontrar, y lo que la pantalla dice por eso:

**El lector tiene tres formas de ignorar el fichero en silencio.** Sin `type` lo
descarta entero; sin `jq` no lo abre; y una ruta que no pasa `_safe_relpath` la
salta con un aviso. Las tres se avisan donde se decide subirlo, porque quien lo
pegue en su repositorio ya no está mirando esta pantalla.

**El descriptor puede contener rutas que el `orbit.json` no admite.** `--appdir`
se valida al crear la app, pero `--outdir` y `--docroot` no. O sea que una app
desplegada y sirviendo puede tener un `outdir` que, copiado al fichero, el
despliegue descartaría. Así que esas claves se validan con la regla **del
fichero** y, si no pasan, no se emiten y se dice cuál era: emitirla sería lo peor
de las tres opciones, porque el fichero se ve bien, se sube, y la carpeta que
publica no es la que pone ahí.

**Una especificación que ya existía no se aplana.** Si `env_spec` trae
`generate`, poner `prompt` en su lugar cambia el significado en silencio.

Y la prueba que sostiene todo lo anterior **no comprueba que el objeto tenga las
claves que yo creo**: ejecuta las expresiones literales de `_read_descriptor` y
`_read_env_block` —con `jq`, contra el fichero generado— y mira qué valores
salen al otro lado. Si no hay `jq`, falla en vez de saltarse: es la lección del
`make test-strict` de Orbit, que ya se comió que una suite se saltara `jq`,
`rsync` y `nginx` y saliera en verde.

## Fase 6c · El mismo núcleo en un terminal, y lo que eso destapó ✅

El `Cargo.toml` del núcleo llevaba desde el primer día una afirmación **sin
comprobar**: *«una TUI podría reutilizarlo sin duplicar nada»*. Este programa es
lo que había que escribir para saber si era verdad.

**No es una interfaz de pantalla completa**, y no por falta de tiempo: no hay
modo crudo, ni teclas, ni redibujado. Imprime y termina. Eso es lo que la hace
utilizable donde de verdad hace falta un cliente de terminal, que es dentro de
otro `ssh`, en un `watch` o en la salida de un script.

Y **sólo hace lo que `orbit` no puede hacer**. La pregunta que decidió su forma
fue: ¿qué da esto que no dé `ssh servidor orbit list`? Si la respuesta hubiera
sido «nada», lo honesto era no escribirlo. La respuesta es **el abanico**:
`orbit list` corre en un servidor, y esto le pregunta a los diez a la vez —con
las conexiones multiplexadas por el mismo `ControlMaster` que ya tiene abierto la
ventana— y pone las respuestas en una tabla. Un servidor que no contesta sale
diciendo que no contesta, **nunca con cero apps**. Todo lo demás —una app suelta,
un log, un `exec`— se hace mejor con `orbit` por `ssh`, y por eso no está.

### La afirmación era verdad a medias

**Cero dependencias nuevas.** Ni librería de terminal, ni analizador de
argumentos, ni runtime asíncrono: el color se apaga solo con `IsTerminal`, que es
de la biblioteca estándar, y el abanico son N hilos esperando a un `ssh`, que es
justo para lo que existe `std::thread::scope`. El transporte, el contrato, el
escapado, el descubrimiento de servidores y **la precedencia de estados** se
reutilizaron tal cual. Por ahí la afirmación se sostiene entera.

**Y era falsa en dos sitios**, los dos por el mismo motivo: con una sola interfaz
no se notaba dónde estaba la frontera.

- **El vocabulario de estados vivía sólo en la interfaz.** La precedencia estaba
  en el núcleo desde el principio —`Estado::salud()`, con su orden escrito— pero
  las palabras, los glifos y las frases estaban en `contrato.ts`. Con dos
  interfaces eso es la forma exacta en que se disuelve el activo más valioso del
  producto: basta con que una diga «parado» donde la otra dice «no aplica» para
  que la distinción entre «no hay proceso» y «el proceso se ha caído» deje de
  existir. Ahora está en el núcleo y las dos se comparan contra
  `tests/contrato/vocabulario.json`.
- **El directorio de los sockets de `ControlMaster`** estaba en el envoltorio de
  escritorio, y es política del transporte. Compartirlo es además lo que se
  quiere: el terminal reutiliza la conexión que ya abrió la ventana, que es la
  diferencia medida entre 246 ms y 13 ms de saludo.

### Y un defecto cometido dos veces

La primera versión de la tabla pintaba **`— —`** en las filas de «no aplica»:
en los dos estados neutros el texto *es* el glifo, así que pintar los dos deja
una fila absurda. Es **exactamente** el defecto que la ventana ya se había comido
—y que allí se cazó mirando una captura, con el DOM en verde— cometido otra vez,
por su cuenta y a la primera, en la segunda interfaz.

Eso no es mala suerte: es la prueba de que la regla no era de una interfaz sino
del vocabulario. Ahora es `Salud::rotulo()`, en el núcleo, con su prueba.

La suite de punta a punta corre contra el mismo sshd del banco y **con tres
servidores a propósito**: con uno el abanico no existe y la prueba pasaría sin
comprobar nada.

## Fase 7 y más allá 💭

- 💭 **Gráficas históricas**, que exigirían que el cliente guarde algo. Es una
  decisión aparte y no pequeña: hoy el cliente no persiste ningún dato del
  servidor, y eso es media hoja del modelo de amenazas

---

## Fuera de alcance

Para que nadie pierda el tiempo proponiéndolo.

- **Una versión web.** Es la razón de ser del proyecto. Orbit descartó el panel
  web porque `orbit` se auto-eleva a root y `orbit exec` ejecuta cualquier cosa;
  poner esta aplicación en un puerto reconstruye exactamente lo que se evitó.
- **Una versión móvil.** Lo mismo, en un aparato que se pierde. El escenario «me
  escriben a las once y no tengo el portátil» es real y **ya tiene respuesta en
  Orbit**: el panel HTML estático de solo lectura, regenerado por el temporizador
  de `watch` y protegido con Cloudflare Access. Esta aplicación no lo sustituye.
- **Escribir en `/etc/nginx`, `/etc/orbit` o systemd.** Nunca, por nada. Si falta
  un comando, el PR va al repositorio de Orbit.
- **Sacar datos con `orbit exec`** para tapar un hueco del contrato. El día que
  se haga, el cliente deja de hablar el contrato y pasa a hablar Bash contra un
  servidor cuyo layout puede cambiar.
- **Guardar contraseñas, frases de paso o claves privadas.** En ningún soporte,
  ni cifrado. Cifrar con una clave que también está en el disco es una ofuscación
  con nombre de cifrado.
- **Telemetría con nombres de dominio, de app, hostnames, IPs o rutas.** Los
  dominios de un cliente de agencia son su cartera de clientes.
- **Gestionar servidores que no tengan Orbit.** Hay herramientas mejores.
- **Un agente en el servidor.** Ni para acelerar, ni para cachear, ni para nada.
  El servidor no gana un byte de estado, y esa es la característica.

---

## Cómo influir en esto

Abre un issue describiendo **el problema que tienes**, no la solución que
imaginas. Y lee [CONTRIBUTING.md](CONTRIBUTING.md) antes de escribir código: hay
cinco reglas que no se discuten en la revisión de un PR.
