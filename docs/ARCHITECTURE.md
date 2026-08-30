# Arquitectura de Orbit Desktop

Este documento explica qué se construye, con qué, y por qué se descartó lo
demás. Es el punto de entrada: los otros cinco documentos desarrollan una parte
cada uno y todos se apoyan en las decisiones que se fijan aquí.

Está escrito **antes que el código**, igual que la §13 de la arquitectura de
Orbit se escribió antes que este repositorio, y por el mismo motivo: lo que no
está decidido antes de la primera pantalla se decide después a la carrera,
cuando ya hay usuarios y ya hay servidores de producción detrás.

| Documento | Qué contiene |
|---|---|
| **[CONTRACT.md](CONTRACT.md)** | El contrato `--json` tal y como **es**, auditado ejecutando |
| **[CLIENT.md](CLIENT.md)** | Stack, transporte SSH, modelo de datos, caché, distribución |
| **[DESIGN.md](DESIGN.md)** | Sistema de diseño, pantallas, estados difíciles, despliegue en vivo |
| **[THREAT-MODEL.md](THREAT-MODEL.md)** | Amenazas y las reglas duras del cliente |
| **[QA.md](QA.md)** | Plan de pruebas, respuestas patológicas, criterios de aceptación |
| **[DEVELOPMENT.md](DEVELOPMENT.md)** | Cómo se monta el entorno y cómo se prueba sin un VPS |

---

## 1. El mandato, y de dónde sale

Orbit se niega a tener un panel web, y el motivo está escrito en su propia
arquitectura antes de que existiera este repositorio:

> `orbit` se auto-eleva a root y `orbit exec` ejecuta comandos arbitrarios. Un
> panel web por encima de eso no es «un panel»: es **una shell de root expuesta
> a internet**.

No es una postura estética. Esa clase de producto es la más atacada del hosting,
y el ejemplo que Orbit cita es concreto y reciente: CVE-2025-48703 en CentOS Web
Panel, ejecución remota **sin autenticar** sobre unos 200.000 servidores,
explotada activamente. Orbit no tiene un equipo de seguridad a tiempo completo;
lo que tiene es una superficie de ataque de tres puertos, y esa es su mejor
característica.

Hay un segundo motivo, menos dramático y más definitivo: **un panel cambia de
producto.** Hoy Orbit compite siendo un script de Bash que cabe en una tarde de
lectura. Con panel competiría con Coolify, CapRover, Dokploy y Plesk, que tienen
equipos.

Orbit Desktop es la otra respuesta a la misma pregunta, y la clave está en dónde
corre:

> Una aplicación que corre **en tu portátil** y ejecuta `orbit` por SSH no rompe
> ninguno de los seis principios: el servidor no gana un proceso, ni un puerto,
> ni un byte de estado. Tiene exactamente el mismo estatus que tu terminal.

Por eso puede existir cuando un panel no puede. Y por eso el repositorio va
aparte: mantiene `orbit` en un solo fichero de Bash y convierte la interfaz en
*un cliente más*, no en *una parte de Orbit*. Si el cliente se queda sin
mantenimiento, Orbit no se entera.

---

## 2. Las cinco reglas heredadas

Vienen del repositorio de Orbit, están escritas allí antes que este código, y
aquí se dan por cerradas. El resto del documento las usa como axiomas.

**1. La interfaz nunca escribe en `/etc/nginx`, `/etc/orbit` ni systemd. Sólo
invoca `orbit`.** El día que genere un vhost por su cuenta habrá dos verdades
sobre cómo se despliega una web, y empieza el segundo producto dentro del
primero.

Y hay un motivo de seguridad además del de coherencia, que sale de auditar el
código: `load_app` carga el fichero de configuración de una app con `.`, o sea
que **un `.conf` de app es código Bash que se ejecuta como root**. Un cliente que
escriba ahí no está editando datos: está escribiendo un ejecutable privilegiado
desde una ventana.

**2. El servidor no gana ni un proceso, ni un puerto, ni un byte de estado.** Ni
un agente, ni una caché, ni un `tmux` para que un despliegue sobreviva a una
desconexión. Si Orbit Desktop desaparece mañana, el servidor no se entera.

**3. Habla SSH, no una API propia.** Se reutilizan las claves, el agente y
`~/.ssh/config`, incluido `ProxyJump`. No inventamos autenticación.

**4. Los secretos no cruzan el contrato.** `orbit env list --json` devuelve los
**nombres** de las variables y nada más — verificado en el código. Un valor se
pide con `orbit env get`, que es un acto deliberado por comando. La otra mitad de
esa promesa es del cliente, y está en [THREAT-MODEL.md](THREAT-MODEL.md) T-06.

**5. La latencia del contrato es la latencia de la interfaz.** Cada pantalla es
un `orbit … --json` por SSH, así que lo que tarde el servidor se suma al viaje y
no se esconde detrás de él.

---

## 3. Las tres reglas propias

Las heredadas dicen qué no se toca. Estas tres salen de auditar el contrato y de
medirlo, y dicen cómo se comporta el cliente.

**6. Lo que no se sabe no se pinta como un valor.** Un `null` no es un cero. «No
he podido preguntar» no es «no hay cambios». Orbit se tomó esto en serio —el
puerto de una web estática es `null` y su `service` también, porque *confundir «no
aplica» con «está caída» pinta una alarma roja donde no pasa nada, y eso enseña a
la gente a ignorar las alarmas»*— y el cliente cumple su mitad o la promesa no
vale nada.

Hay un precedente que lo justifica solo: confundir `unchanged` con `unreachable`
en Orbit hizo que un remoto caído se anunciase como «nada que hacer» cada cinco
minutos. El contrato tiene **seis** finales por app justamente para que un cliente
no pueda repetirlo, y agruparlos en «correctas / fallidas» está prohibido.

**7. La fuente de verdad es el binario, no la documentación.** Cuando el
comportamiento y los documentos de Orbit discrepan, manda el comportamiento. No
es una hipótesis: `orbit doctor --fix --json --yes` está documentado en dos
sitios y **no existe** — muere con «no sé qué es «--yes»», porque `--yes` no es
una bandera global. Se descubrió ejecutándolo, no leyéndolo.

**8. Toda orden lleva el nombre de la app explícito, siempre.** Sin terminal al
otro lado, un comando sin app **no aborta**: imprime un selector, elige la
primera por orden alfabético y sale con 0. Con `orbit restart` eso es reiniciar
la app equivocada sin que nada en la salida lo diga. Sólo `info --json`,
`deploy --json` y `rollback` se protegen. La regla la exige el tipo del
constructor de comandos, no una revisión.

---

## 4. Las decisiones, y lo que se descartó

Cada una está desarrollada en su documento; aquí queda el registro con el motivo
en una línea, que es lo que hace falta dentro de dos años para saber si algo se
puede cambiar.

### 4.1 Se delega en el binario `ssh` del sistema, no en una librería

**La subdecisión que manda del proyecto entero**, y no es «qué librería SSH» sino
**quién interpreta `~/.ssh/config`**. La respuesta honesta es que ninguna
librería lo hace del todo.

Se evaluaron `russh`, `ssh2-rs`, `thrussh`, el `ssh2` de Node y
`golang.org/x/crypto/ssh`. Todas comparten la misma trampa: **son el protocolo,
no el cliente.** No leen `~/.ssh/config` con sus `Match` y sus `Include`, no
hablan con el agente en las tres plataformas —el `SSH_AUTH_SOCK` de Windows es
una tubería con nombre—, y `ProxyJump` hay que implementarlo a mano. Cada una de
esas tres cosas son entre 200 y 600 líneas, y son exactamente las líneas donde un
cliente SSH se equivoca. `ProxyCommand` es directamente imposible sin lanzar un
proceso — y si vas a lanzar un proceso, ya estabas en la otra opción.

Delegando se gana `~/.ssh/config` entero, el agente con sus llaves en hardware,
`ProxyJump` y `ProxyCommand` —y con ellos `cloudflared`, `aws ssm` y el bastión
corporativo que nadie te va a contar—, `ControlMaster`, y la verificación de
`known_hosts` hecha por OpenSSH con su política y sus parches. **Cero superficie
criptográfica propia:** un CVE en libssh2 sería un CVE nuestro; uno en OpenSSH es
un `apt upgrade` del usuario.

Se pierde depender de un binario externo —presente siempre en macOS y Linux, y en
Windows desde 2018— y tener que reconocer errores en texto, que son cadenas
estables desde hace veinte años y que OpenSSH no traduce nunca.

**Es lo que hacen VS Code Remote-SSH y `git`.** Que las dos herramientas que más
SSH hacen en el escritorio de un desarrollador tomaran la misma decisión no es un
argumento de autoridad: es la señal de que reimplementar `~/.ssh/config` es un
pozo, y de que los dos lo comprobaron antes que nosotros.

### 4.2 Tauri v2, no Electron

Se descarta Electron por coherencia y por mantenimiento. Por coherencia: sería
raro responder a «un panel sobre `orbit` es una shell de root expuesta a
internet» metiendo Chromium y Node completos en el mismo proceso que sostiene las
credenciales SSH del usuario. En Tauri el renderer no tiene Node —no hay `fs`, no
hay `child_process`— y todo lo que la interfaz puede hacer pasa por comandos
declarados en Rust. Si una dependencia de npm se compromete, en Electron puede
leer `~/.ssh/id_ed25519`; en Tauri no puede, porque no hay API para eso.

Por mantenimiento: seguir a Chromium es una suscripción de trabajo indefinida
para un equipo pequeño.

Y Tauri es **el único candidato que trae actualizador con verificación de firma
de serie**, con la clave pública compilada dentro del binario. Para una
herramienta que tiene root en todos los servidores de sus usuarios, ésa es
exactamente la propiedad que hace falta. Wails, Avalonia y Qt no traen
actualizador, y escribir un actualizador es escribir el vector de ataque más
goloso del producto.

Lo que se paga: **tres WebView distintos**. Se compensa con disciplina de CSS y
con capturas en CI en las tres plataformas.

### 4.3 Svelte 5 y CSS con tokens — la contradicción, arbitrada

Es la única decisión que los dos informes especializados resolvieron **en
sentidos opuestos y a la vez**: el de diseño retiró Svelte y aceptó React; el de
arquitectura retiró React y aceptó Svelte. Los dos cedieron al otro con
argumentos honestos, así que hubo que arbitrar.

**Lo primero: el argumento original a favor de Svelte era falso, y conviene que
conste.** Decía que «el monitor reconcilia cuarenta filas cada dos segundos». Con
el banco delante: cuarenta filas son 12 KB, el servidor tarda 2,1 s en producir
una muestra de `top` —o sea que el ciclo real es de más de 3 s, no de 2—, el
monitor sólo sondea con la pantalla visible, y no hay ningún otro sondeo en la
aplicación. La diferencia entre los dos marcos ahí vale unos 2 ms cada 3,2 s. Una
decisión defendida con un número equivocado se reabre sola, así que el número
equivocado se escribe.

**Y el peso tampoco decide del todo**, porque bajo Tauri el WebView es del
sistema y no se empaqueta un navegador.

Cuando los dos diferenciadores objetivos se anulan, no queda un empate: queda el
criterio con el que este proyecto ya ha decidido **dos veces**, pagando una
incomodidad a cambio de una superficie más pequeña. Tauri en vez de Electron,
pagando dos lenguajes. El `ssh` del sistema en vez de una librería, pagando
parsear texto de error. **Elegir React aquí sería la primera vez que el proyecto
rompe su propio patrón, y lo rompería justo en la dimensión que el análisis de
seguridad puntúa peor de todas: la cadena de suministro.** 45 KB comprimidos y su
árbol de dependencias frente a unos 200 KB no cambian el arranque de forma
perceptible, pero sí cambian cuántos paquetes de terceros se ejecutan en el
proceso que sostiene las credenciales SSH del usuario. Es el mismo argumento con
el que se descartó Electron, aplicado un nivel más abajo.

**El coste se acepta y se escribe:** el mercado de gente que sabe React es mucho
mayor, y eso es un riesgo real de mantenimiento a cinco años para un equipo
pequeño. Lo que lo hace pagable es que **la lógica del cliente vive en un crate
de Rust independiente de la interfaz**: el transporte, el contrato y la caché no
se enteran de un cambio de framework. Esta decisión es reversible; las de §4.1 y
§4.2 no.

**Sin Tailwind**, y el argumento es del informe de diseño: la distinción entre
`served:false`, `service:null` y `service:"stopped"` es el activo más valioso del
producto, y es exactamente lo que se disuelve cuando el estado pasa a ser
`bg-red-500/10 text-red-400 border-red-500/30` repetido en cuatro componentes. El
día que alguien añada una quinta copia con un matiz distinto, la lista y el
detalle dejan de decir lo mismo y nadie lo notará hasta que un usuario vea un
incidente donde no lo hay. El estado semántico vive en un solo fichero, con
clases que se llaman como el concepto, y una regla de lint que falla el build.

### 4.4 El multiservidor sale de la arquitectura, no de una función

Un cliente que habla SSH con varios servidores **es** el `orbit remote add` de la
v2.0 de Orbit, sin plano de control y sin demonio. Es un regalo de la §4.1.

Con dos noes razonados, que están en [DESIGN.md](DESIGN.md): **ni despliegue en
grupo ni lista global de apps de todos los servidores.** Las dos son un plano de
control con otro nombre, y desplegar en tres servidores tiene catorce
combinaciones de éxito parcial que ninguna interfaz sencilla cuenta bien.

Lo que sí trae el multiservidor es **el accidente**: `orbit remove tienda --purge`
contra el servidor de pruebas y contra el de producción son la misma pantalla con
un desplegable distinto. Hay precedente en el propio repositorio de Orbit, y es
de los que enseñan: su suite de pruebas, ejecutada como root en un servidor que
tenía una app llamada `tienda`, **borró el vhost de la app de verdad**. 32 suites
en verde, 2.512 comprobaciones, 0 fallos, y una web muerta. De ahí sale la regla
de que **el nombre de una app no identifica nada por sí solo**: todo lo que se
confirme, se registre o se anuncie va como `servidor : app`.

---

## 5. Lo que se ha medido

Tres cosas separan este documento de una lista de buenas intenciones, y las tres
se pueden reproducir desde el repositorio. El detalle está en
[DEVELOPMENT.md](DEVELOPMENT.md).

**El banco de 40 apps.** `tools/bench/` monta un Orbit de mentira y mide. Sirvió
para corregir tres decisiones que estaban mal:

| | Medido | Qué cambió |
|---|---|---|
| `orbit version --json` | **72 ms** | Hay un **suelo por llamada**: 13.720 líneas de Bash parseadas cada vez, antes de la red. No estaba en ningún documento |
| `orbit list --json` | 306 ms (`72 ms + 5,9 ms/app`) | Escala limpio: no hace falta paginar hasta pasadas las 100 apps |
| `orbit status --json` | 389 ms | Trae el array de apps **idéntico** al de `list` (comprobado con `==`). La portada es **una** llamada de 389 ms y no dos de 695: un 44 % menos, sin escribir código |
| `orbit top --json` | **2.116 ms** (`≈1.053 + 26,6 ms/app`) | El plan de sondear cada 2 s era **físicamente imposible**. Venía de copiar un número del servidor sin ver que ése es del panel en vivo, que reutiliza la muestra anterior |
| Saludo SSH | **246 ms** sin multiplexar, **13 ms** con `ControlMaster` | Es la palanca de latencia más grande del producto |

**La prueba de propiedad del escapado.** `argv → escapar → shell remoto → argv`
es la identidad, comprobado contra **`bash`, `dash`, `zsh` y `busybox ash`**:
cuatro shells × 7.116 casos = **28.464 viajes**, con las semillas registradas.

Y vale porque **falló la primera vez que se ejecutó**. Cinco casos de 2.529, y
sólo en zsh: el escapador tenía un conjunto de caracteres «seguros» que pasan sin
comillas y `=` estaba dentro; zsh expande las palabras que empiezan por `=`, así
que el argumento `=Y` volvía como `zsh:1: Y not found`. `bash`, `dash` y
`busybox` pasaban los 2.529. Es el modo de fallo exacto contra el que existe la
prueba: **correcto en el shell donde se desarrolla, roto en el que usa el
usuario.**

El arreglo no fue prohibir un carácter —una lista negra crece cada vez que
alguien encuentra uno nuevo, que es la firma de que el diseño estaba mal— sino
**estrechar el conjunto seguro** y entrecomillar todo lo demás. Y de paso cayó
otra creencia: **`printf %q` de Bash no sirve** como escapador portable, porque
produce `$'\n'`, que `dash` no entiende.

**La superficie del socket de `ControlMaster`, verificada.** Es
`srw------- usuario:usuario`, pero con el agente desactivado y
`IdentityFile=/dev/null` se abrió un canal y se ejecutó `id`. No añade una clase
de amenaza nueva —el agente tiene la misma propiedad— pero **salta la
confirmación**: una YubiKey que pide un toque por conexión lo pediría una vez por
`ControlPersist`. De ahí que el multiplexado vaya apagado por defecto con claves
`sk-` y que el `persist` baje a 45 segundos.

---

## 6. Lo que Orbit Desktop no es

- **No es un panel web, ni lo va a ser.** Ponerlo en un puerto reconstruye
  exactamente lo que este proyecto existe para evitar.
- **No es una aplicación móvil.** El escenario «me escriben a las once y no tengo
  el portátil» es real y **ya tiene respuesta en Orbit**: su panel HTML estático
  de solo lectura, regenerado por el temporizador que ya corre y protegido con
  Cloudflare Access. Esto no lo sustituye. Una shell de root en un teléfono es
  otra cosa.
- **No es una terminal.** `orbit exec` se ofrece como lo que es —una shell del
  usuario de la app, con el `.env` cargado y todos sus secretos en el entorno—
  con dos modos explícitos y sin shell interactiva embebida. Fingir un terminal
  es, en palabras de la propia documentación de Orbit, *la peor solución de
  todas*.
- **No saca datos con `orbit exec`.** Ni para leer un fichero, ni para contar
  releases, ni para tapar un hueco del contrato. El día que lo haga, deja de
  hablar el contrato y pasa a hablar Bash contra un servidor cuyo layout puede
  cambiar.
- **No es una bóveda de secretos.** Los `.env` siguen en texto plano en el
  servidor. Lo único que aportamos es no empeorarlo.
- **No audita el código que despliegas.** Igual que Orbit.

---

## 7. La deuda que no es nuestra, y que ya está pagada

El trabajo más rentable de todo el proyecto **no estaba en este repositorio**, y
por eso conviene que quede escrito aquí: un PR a `orbit` que cerrara los huecos
del contrato en vez de rodearlos desde el cliente.

**Está abierto:
[iNTERVOLUTIONS-Labs/orbit#1](https://github.com/iNTERVOLUTIONS-Labs/orbit/pull/1)**,
con los cinco cambios que [CLIENT.md](CLIENT.md) §9b especificaba:

| | Qué cambia | Qué se lleva por delante |
|---|---|---|
| 9b-1 | `doctor --fix --json --yes` **existe** | La pantalla de diagnóstico recupera el botón donde `fixable` lo permite |
| 9b-2 | `backup list/verify --json` | Bytes en bruto e ISO-8601 en vez de una tabla de anchura fija y un `du -h` con el separador decimal del servidor |
| 9b-3 | `logs --json` como NDJSON | El parser de texto con heurística de glifos. Y **`stream` distingue el log de acceso del de error**, que la prosa perdía |
| 9b-4 | `tests/cli_test.sh` | La laguna que dejó pasar 9b-1: `main()` no la ejecutaba ninguna prueba |
| 9b-5 | La promesa de estabilidad, escrita | La política de contrato 2 deja de ser una apuesta |

**El efecto sobre este repositorio es que `contract/text.rs` no llega a existir.**
Era la cuarentena de [CLIENT.md](CLIENT.md) §7.1 y sus dos únicos habitantes iban
a ser `backup list` y `logs`. El cliente nace con **cero líneas que parseen
texto**, que es un estado que se puede comprobar con una regla de lint en vez de
perseguirlo como objetivo.

Dos cosas del PR importan aquí aunque no sean del contrato, porque son reglas
que este cliente tiene que cumplir y que ahora están **fijadas con una prueba**
en el otro lado:

- **Sin terminal, un comando sin app no aborta: elige la primera por orden
  alfabético y sale con 0.** Ya era la regla 8 de §3; ahora hay una prueba en
  `orbit` que se rompe el día que eso cambie.
- **`--json` detrás de un comando que no lo habla se ignora en silencio.** De ahí
  la regla práctica del constructor de comandos: **`--json` siempre delante**, que
  es la única posición con un comportamiento definido.

Mientras el PR no esté fusionado, el cliente tiene que hablar con servidores que
no lo tengan. Eso no es un caso raro ni temporal —un parque de servidores tarda
meses en actualizarse— así que la degradación es parte del diseño y no un apaño:
`orbit version --json` publica `version` y `contract` por separado, y de ahí sale
qué pantallas pueden ofrecer qué. La política está en [CLIENT.md](CLIENT.md) §4.4.
