# Plan de pruebas

> Cómo se demuestra que el cliente cumple sus reglas y —lo que más importa— que
> **no miente**. El fallo característico de este producto no es que la
> aplicación explote: es que pinte algo plausible.
>
> Las reglas que aquí se verifican están en **[THREAT-MODEL.md](THREAT-MODEL.md)**.
> Las pruebas del lado del código, en **[CLIENT.md](CLIENT.md) §9**. Cómo correr
> lo que ya existe, en **[DEVELOPMENT.md](DEVELOPMENT.md)**.

> **Cómo leer las referencias.** Este documento y [THREAT-MODEL.md](THREAT-MODEL.md) eran uno solo y comparten numeración: allí viven las §0 a §4 y aquí de la §5 en adelante. Las **T-nn** (amenazas) y las **SEC-nn** (reglas duras) que se citan aquí están definidas allí; las **P-nn** (respuestas patológicas) y las **B-nn** (criterios de aceptación), aquí.

---

## 5. Plan de QA

### 5.0 El estándar que se hereda

Orbit tiene 32 suites y unas 2.500 comprobaciones, y lo que hace que ese número
signifique algo no es el número: es la lista de formas de mentir que ya han
catalogado. `docs/DEVELOPMENT.md` §«Lo que las pruebas unitarias no ven» enumera
**cinco estados** de una prueba que no es verde-de-verdad, y los cinco se aplican
igual a un cliente de escritorio:

1. **La suite que se salta sola** cuando falta una herramienta, y lo dice en una línea
   que se pierde. Sin `jq`, `rsync` y `nginx`, `make test` sale verde habiendo probado
   dos tercios: 1.662 comprobaciones contra 2.447 —**medido apartándolos del `PATH`,
   no estimado**—. Por eso existe `make test-strict`, que se niega a dar verde si algo
   se saltó.
2. **La suite que se pone en roja** por falta de una herramienta opcional, acusando a
   quien no era. Pasó con `jq` y 16 fallos repartidos por tres funciones sanas.
3. **La guarda que mide lo que no es.** `python3 -m venv --help` contesta que sí aunque
   falte `ensurepip`. Regla: *preguntarle a una herramienta si existe no es preguntarle
   si funciona* — y si la guarda es barata, **ejecútala de verdad**.
4. **La suite que no ejecuta nadie.** `provision_test.sh` estuvo desde su propio commit
   sin estar en el `Makefile`: 36 comprobaciones de travesía de rutas y enlaces
   simbólicos que no corría nadie, y no dejaba hueco porque el total sólo suma lo que
   sí se lanzó. Desde entonces `make check` **cruza los ficheros que hay contra los que
   el target invoca** y falla si sobra alguno.
5. **La suite que hace daño de verdad.** El vhost de producción borrado.

Y una regla de método que vale para todo lo que sigue: **para saber si algo toca el
sistema, no lo leas — móntale un sistema falso encima y mira qué queda tocado.**

Cuatro cosas se heredan literalmente:

- **`make check` cruza la clase entera contra lo que se ejecuta.** En Orbit Desktop:
  ningún fichero de pruebas puede existir sin que el corredor lo ejecute, y CI falla
  si aparece uno huérfano. Aplicable también a los ficheros del catálogo de respuestas
  patológicas (§5.2b): uno nuevo que nadie consuma es exactamente el estado 4.
- **Un modo estricto que se niega a dar verde si algo se saltó.** Es el que corre CI.
- **`run()` con las mismas opciones que en producción.** La lección de `tests/lib.sh`
  —`run cmd_x && r=0 || r=$?` desactiva `errexit` y la prueba pasa sin haber probado
  nada— tiene su equivalente en cualquier lenguaje: **una aserción que envuelve la
  llamada en un manejador de errores cambia el comportamiento que está midiendo.**
- **Antes de creerte una medición, comprueba qué marca en reposo.** La sonda de tráfico
  de §5.2 de Orbit iba a 100 req/s contra un `limit_req` de 40 y se acusaba a sí misma.

### 5.1 Unitarias

La base de la pirámide. Rápidas, sin red, sin proceso, sin `sshd`. Objetivo: que
**ningún fallo de esta capa pueda llegar a una capa de arriba**, porque arriba es más
caro depurarlo.

**A · El escapador de argumentos. Escrito, ejecutado y con un fallo cazado.**
Es el test más importante de todo el proyecto, porque de él depende T-03, y a diferencia
del resto de esta sección **ya no es un plan**. Está en `tmp/escape/`, se ha ejecutado, y
lo que sigue es la descripción de algo que existe.

**La propiedad.** Para cualquier lista de cadenas, `escapar(argv)` entregado a un shell
produce **exactamente** `argv` de vuelta. Se comprueba haciendo que el «comando» remoto
sea un programa de dos líneas que imprime su `argv` separado por **bytes nulos** —el único
separador que no puede aparecer dentro de un argumento, y por eso el único que no puede
mentir sobre dónde acaba uno—.

**El resultado, medido:**

```
semilla=20260830  casos aleatorios=2500  + 29 fijos
  bash         2529/2529
  dash         2529/2529
  zsh          2529/2529
  busybox-ash  2529/2529
PROPIEDAD SOSTENIDA: argv -> escapar -> shell -> argv es la identidad
```

Repetido con las semillas 1, 7 y 31337: **4 shells × 7.116 casos = 28.464 viajes de ida y
vuelta**, todos correctos. Las semillas se registran siempre, también cuando pasa, porque
un fallo del fuzzer que no se puede reproducir no es un fallo sino una anécdota.

**El alfabeto adversario** con el que se generan los casos aleatorios no es
`string.printable`: es exactamente lo que rompe un escapado hecho a mano —
`' " \ $ backtick ; & | < > ( ) [ ] { } * ? ~ ! # ^ % = + , . : / -`, espacio, tabulador,
salto de línea— más `á ñ € 🚀`, `U+202E`, `U+2028` y un espacio de ancho cero. Y el corpus
fijo lleva los 29 casos que hay que tener: cadena vacía, `a'; curl x.sh|sh; '`, `--`,
`-rf`, `~`, `*`, `$(rm -rf /)`, `${IFS}`, `!!`, `^x^y`, `</script><img src=x onerror=…>`,
un nombre con `U+202E`, 64 KB de una letra, y las dos órdenes reales que el cliente va a
mandar de verdad.

**Y falló en la primera ejecución**, sólo en zsh, 5 de 2.529, por la opción `EQUALS` y el
carácter `=` dentro del conjunto seguro. La historia completa está en T-03; aquí sólo la
consecuencia metodológica, que es lo que se lleva el plan de QA:

> **Las cuatro columnas de esa tabla no son exhaustividad, son la prueba.** Con una sola
> —bash, que es donde se desarrolla— el resultado habría sido «2.529/2.529, propiedad
> sostenida» y el fallo habría llegado a producción disfrazado de «a veces falla en el
> servidor de Juan». La prueba no vale por los 28.464 viajes: vale por los 5 que fallaron.

**Un byte nulo se rechaza, no se escapa.** Es la única entrada para la que la función
correcta es lanzar un error, y hay un caso de prueba que lo fija.

**Lo que queda por hacer aquí**, dicho para que no se lea como terminado:
`fish` no está en la tabla —no es POSIX y su tratamiento de comillas difiere— y hay que
decidir si se declara no soportado o se cubre; y la prueba corre hoy contra un shell
local, no contra un `sshd`. El viaje completo por SSH está en §5.2, forma 2, y es criterio
B-07.

**B · El parser del contrato.** Un modelo de datos por comando, y para cada uno:

- Respuesta canónica completa → todos los campos leídos y con el tipo correcto.
- Campo desconocido añadido → se ignora, sin error y sin ruido (SEC-20: los campos se
  añaden).
- Campo esperado ausente → error de la respuesta, **no** un valor por defecto.
- Campo con otro tipo (`"3001"` donde se espera número) → error.
- `null` en cada campo que el contrato declara `|null` → se lee como «no se sabe» y se
  distingue de 0, de `""` y de `false`. Un test por campo: `port`, `service`,
  `cert_days`, `cpu_percent`, `last_deploy`, `last_deploy_sha`, `build_trend_s`,
  `result` de `deploy --all`, `previous`, `failed_step`, `error`.
- `schema` distinto de 1 → rechazo explicado.
- `contract` mayor del conocido → aviso y degradación, nunca invención.
- **`load` con menos de tres elementos** (la línea 11497 puede emitir `[]`) → no
  revienta.
- **Los seis finales de `deploy --all`** se modelan como seis, no como un booleano.
  Test que falla si alguien añade una función `esOk(status)` que agrupe `unchanged` con
  `unreachable`: es el bug real de §13.6bb y el contrato existe para que no se repita.

**C · Comparación de versiones.** Dos ejes distintos y no se pueden mezclar, porque
§13.1 lo dice: *«Orbit puede subir de versión sin que el contrato cambie —de hecho es
lo normal— y un cliente que las confundiera se negaría a hablar con un servidor
perfectamente compatible.»*

- `version` es semver del servidor: `1.3.6` < `1.10.0` —el error clásico, comparar como
  cadenas da lo contrario—, sufijos de prelanzamiento, versiones con más de tres
  componentes, una versión vacía, una versión que no es semver.
- `contract` es un entero y se compara como entero.
- Matriz de decisión probada entera: contrato del servidor menor / igual / mayor que el
  del cliente, cruzado con cada comando. **La celda que importa es «servidor más nuevo
  que el cliente»**: ahí el cliente degrada y avisa, y no se niega a funcionar.

**D · Validadores de forma** (SEC-05): la tabla de T-03, con los casos límite de cada
regla. Nombre de 40 caracteres (vale) y de 41 (no); `a..b` (no); `-web` (no); `Web`
(no, `_app_name_ok` es minúsculas); `viejo.com` (sí, y hay que probarlo porque las
redirecciones de dominio registran apps así, comentario de la línea 1174).

**E · Redacción y telemetría** (SEC-15, SEC-17): dado un estado lleno de datos
sensibles conocidos, ningún evento de telemetría ni ninguna línea de log los contiene.

**F · Formateo y presentación de lo desconocido** (SEC-18): `null` no se pinta como 0,
`unreachable` no se pinta como `unchanged`, `complete:false` sale marcado,
`requests_capped:true` sale con el `+`, `cpu_percent:null` sale como «—» y no como
«0 %».

**G · Concurrencia.** Es la capa que la ronda 1 dejó fuera, y no es un lujo: el cliente es
multiservidor por diseño (principio 5) y cada pantalla lanza llamadas en paralelo.

- **Dos llamadas simultáneas al mismo servidor** no se pisan: cada una tiene su par de
  tuberías, su presupuesto y su cancelación. La aserción concreta es que la respuesta de
  una no puede aparecer en la otra —se comprueba con un servidor falso que devuelve un
  identificador distinto por invocación—.
- **Cancelar una llamada no afecta a las demás**, ni cierra el máster compartido.
- **Cuarenta servidores a la vez** no abren cuarenta conexiones: hay un límite de
  concurrencia y una cola, y la prueba comprueba que el límite se respeta contando
  procesos `ssh` vivos, no leyendo el código.
- **La lectura de stdout y stderr es concurrente, no secuencial.** Ésta es la prueba que
  atrapa el *deadlock* clásico: un servidor falso que escribe 1 MB por stderr y luego un
  objeto por stdout **bloquea para siempre** a un cliente que lea stdout hasta el final
  antes de tocar stderr, porque el búfer de tubería son 64 KB en Linux. Es el fallo número
  uno de quien implementa esto por primera vez, y sin esta prueba no se descubre hasta que
  alguien despliega una app que compila con mucho ruido.
- **Una mutación y una lectura en vuelo a la vez**: un `deploy` corriendo y un `list`
  pedido por otra pantalla. La lectura no se bloquea, y su resultado —que puede ser previo
  al despliegue— **no pisa el estado del despliegue en curso**.
- **Dos mutaciones destructivas sobre la misma app** desde dos pantallas del mismo cliente:
  la segunda se rechaza en el cliente. P-46 cubre el caso de dos clientes distintos, que no
  se puede rechazar y sólo se puede detectar.

**H · Ciclo de vida de la conexión.** Todo con un `sshd` real (§5.2, forma 2), porque nada
de esto se puede simular con un doble local.

- **Primera llamada**: se crea el máster; el socket existe, con `0600`, en el directorio
  esperado. Se comprueba el modo del fichero, no que la ruta esté escrita en el código.
- **Segunda llamada**: reutiliza el máster. Se verifica contando procesos `ssh`, y midiendo:
  la segunda tiene que ser al menos un orden de magnitud más barata en el establecimiento.
- **`ControlPersist` vence**: pasado el plazo, el máster se va solo y la llamada siguiente
  vuelve a pagar el handshake, sin error visible para el usuario.
- **Socket huérfano**: se mata el proceso máster dejando el fichero, y la llamada siguiente
  hace `ssh -O exit` y **un** reintento. Se comprueba que reintenta una vez y no dos, y que
  no entra en bucle.
- **Bloqueo de la aplicación**: los másters se cierran… **salvo el del despliegue en
  curso**. Las dos mitades tienen su prueba, y la segunda es la que se olvida.
- **Cierre de la aplicación**: no queda ningún socket ni ningún proceso `ssh` vivo. Se
  comprueba enumerando el directorio y la tabla de procesos, no confiando en un `Drop`.
- **Suspensión y cambio de red**: se corta la conexión por debajo (se tira la interfaz de
  red del contenedor) y se comprueba que `ServerAliveInterval` la detecta en el orden de
  los 45 segundos, y no en los 15 minutos del keepalive de TCP.
- **Windows sin `ControlMaster`**: la misma batería, comprobando que no se emiten las
  opciones de multiplexado y que todo funciona igual, más despacio. Es la plataforma donde
  el código es distinto y por tanto donde hay que probar.

### 5.2 El servidor Orbit falso

**El problema:** las pruebas de integración no pueden depender de un VPS. Ni por coste,
ni por velocidad, ni por reproducibilidad, ni —sobre todo— porque **un VPS real no
sabe devolver las respuestas patológicas**, que son justo las que hay que probar.

**La solución, y por qué ésta y no otra.** Un script —bash, por coherencia con lo que
prueba, y porque el contrato se escribe a mano con `printf` y no con `jq` a propósito
(comentario de las líneas 1063-1068)— que se comporta como el binario `orbit` en el
otro extremo:

```
tests/fakeserver/orbit          # el binario falso
tests/fakeserver/fixtures/      # un fichero por respuesta
tests/fakeserver/scenarios/     # guiones: qué contesta a qué, en qué orden
```

Y **dos formas de conectarlo**, porque prueban cosas distintas y hacen falta las dos:

1. **Sustituyendo el transporte.** El cliente se configura con una ruta de binario
   alternativa y se ejecuta el falso localmente. Rápido —milisegundos—, cubre el
   parser, el modelo, la interfaz y el manejo de errores. Ahí va el 90 % de los casos.
2. **Con un `sshd` de verdad en un contenedor**, con el falso instalado como
   `/usr/local/bin/orbit`. Lento —segundos— y cubre lo que el primero **no puede
   ver**: el escapado de argumentos de punta a punta, `known_hosts`, `ProxyJump`, el
   agente, los cortes de conexión, los códigos de salida de `ssh` frente a los de
   `orbit`, y la diferencia entre stdout y stderr de verdad.

**La segunda no es opcional**, y el motivo está en la lección de `ARCHITECTURE.md`
§5.1: *«lo que sólo existe dentro del espacio de nombres de systemd no lo ve
`make test`»*. Aquí la frase equivalente es: **lo que sólo existe dentro de un shell
remoto no lo ve un doble local.** El escapado se prueba contra un shell de verdad o no
se ha probado.

**Cómo se construye el falso, en concreto.** Lee `$SSH_ORIGINAL_COMMAND` o su `argv`,
mira un fichero de guion —variable de entorno o fichero en el contenedor— y contesta lo
que diga: JSON por stdout, ruido por stderr, un código de salida y, si el guion lo
pide, una pausa o un cierre abrupto. Debe poder **registrar el `argv` exacto que
recibió**, porque ese registro es la aserción de las pruebas de escapado: no se
comprueba lo que devuelve, se comprueba lo que le llegó.

### 5.2b Catálogo de respuestas patológicas

Esto es el corazón del plan de QA. Cada entrada tiene: qué devuelve el falso, qué debe
hacer el cliente, y qué **no** debe hacer. La tercera columna es la que atrapa los
bugs, porque el fallo típico no es que el cliente explote: es que **pinta algo
plausible**.

| # | Respuesta del falso | El cliente debe | El cliente NO debe |
|---|---|---|---|
| P-01 | **JSON truncado** a mitad de un objeto | Error de respuesta incompleta, con el comando y el servidor | Parsear lo que llegó; pintar las apps que sí se leyeron |
| P-02 | **JSON con campos nuevos** que no conoce | Ignorarlos en silencio y funcionar igual | Fallar; avisar; pintar «campo desconocido» |
| P-03 | **`null` donde el contrato dice número** (`"releases": null`) | Tratarlo como desconocido y pintarlo como tal | Pintar 0; ordenar la lista poniéndolo como si fuera 0 |
| P-04 | **`"schema": 2`** | Rechazar la respuesta y decir que el servidor habla un contrato que este cliente no entiende, con la versión de cada uno | Intentar leerla «por si acaso» |
| P-05 | **`"contract": 9`** en `version --json` con `schema:1` | Avisar de que el servidor es más nuevo; seguir usando lo que entiende | Negarse a funcionar (§13.1 lo dice explícitamente) |
| P-06 | **Salida vacía**, código de salida 0 | Error: «el servidor no ha contestado nada» | Interpretarlo como «no hay apps». §13.6bb: *«con stdout en blanco, un cliente no puede distinguir "no había nada" de "algo se rompió"»* — por eso el contrato devuelve `total: 0` |
| P-07 | **Colección vacía legítima**: `{"schema":1,"apps":[],"total":0}` | Pintar «no hay apps» | Confundirlo con P-06 |
| P-08 | **Código de salida distinto de 0 con stdout vacío** | Enseñar el stderr, que es donde va el mensaje para la persona, y el código | Inventar un mensaje; decir «error desconocido» a secas si hay stderr |
| P-09 | **Código distinto de 0 con un JSON válido en stdout** — el caso real de un despliegue que falla | Leer el objeto: `ok:false` y `failed_step` traen el motivo (§13.6b, el `trap '_deploy_on_exit' EXIT` de la línea 5228) | Descartar el stdout por el código de salida |
| P-10 | **Código de salida 0 con `"ok": false`** | Creerle al objeto y anotar la incoherencia en el log | Fiarse sólo del código |
| P-11 | **stderr con basura antes del JSON** (avisos, banner del `.bashrc`, `motd`) | Ignorar stderr para los datos; enseñarlo si hay error | Buscar la primera llave en la salida combinada |
| P-12 | **Basura en stdout antes del JSON** | Error (SEC-23) | Recortar hasta la primera llave — es cómo se cuela un objeto falso |
| P-13 | **Dos objetos JSON en stdout** | Error | Quedarse con el primero o con el último |
| P-14 | **`orbit` no existe** (`command not found`, código 127) | Mensaje específico: Orbit no está instalado o la ruta es otra, con la ruta que se intentó | «Error al conectar» |
| P-15 | **`orbit` existe pero no es ejecutable** o permiso denegado (126) | Mensaje específico | Confundirlo con P-14 |
| P-16 | **`sudo` pide contraseña** y se queda esperando | Detectar el bloqueo, abortar y explicar que ese usuario necesita sudo sin contraseña o ser root | Colgarse hasta el timeout sin decir por qué |
| P-17 | **Conexión cortada a mitad de un `deploy`** | «Estado desconocido» más cómo averiguarlo (T-10) | Decir que falló; reintentar solo |
| P-18 | **Conexión cortada a mitad de un `list`** | Reintentar es legítimo aquí: es idempotente | Reintentar un comando que no lo es |
| P-19 | **Respuesta lentísima** (60 s) | Progreso indeterminado, botón de cancelar, y cancelar de verdad | Congelar la interfaz; abortar antes de tiempo un `deploy` legítimo |
| P-20 | **Respuesta infinita** (goteo eterno) | Cortar por presupuesto (SEC-21) y decirlo | Consumir memoria hasta morir |
| P-21 | **Respuesta enorme** (200 MB en un solo objeto) | Cortar por presupuesto | Cargarla entera |
| P-22 | **Nombre de app con `</script><img src=x onerror=…>`** | Pintarlo **literalmente**, y marcar la app como no operable (SEC-05) | Ejecutarlo; «limpiarlo» y operar sobre el nombre limpio |
| P-23 | **Nombre de app con `'; rm -rf / #`** | Igual que P-22; y si por algún camino llegara a un comando, el escapado lo neutraliza | Ejecutarlo |
| P-24 | **Nombre con un RIGHT-TO-LEFT OVERRIDE (U+202E)** y con homoglifos cirílicos | Pintarlo marcado (T-11) | Pintarlo como si fuera el nombre latino que aparenta |
| P-25 | **Cadena con `U+2028`** en un campo de texto | Pintarla; el parser no se cae | Romper si el JSON se incrusta en una plantilla |
| P-26 | **UTF-8 malformado** en un valor | Error de decodificación explicado, o sustitución marcada | Pintar el carácter de reemplazo en silencio y seguir como si nada |
| P-27 | **Cadena de 5 MB** en un campo `error` | Truncar en pantalla, guardar entera si el usuario la pide | Pintar 5 MB y congelar el hilo de la interfaz |
| P-28 | **`"apps"` es un objeto, no una lista** | Error de tipo | Iterar y pintar basura |
| P-29 | **Números fuera de rango**: `port: 99999999999999999999`, `cert_days: -5` | Rechazar o marcar; `cert_days` negativo **es real** (certificado caducado) y hay que pintarlo como caducado | Desbordar en silencio; pintar «-5 días» como si fuera normal |
| P-30 | **`env list` devuelve una clave con pinta de valor** (`DB_PASSWORD=hunter2`) | Pintar el nombre tal cual | Partirlo por `=` y creerse la mitad |
| P-31 | **NDJSON de `--progress` con una línea rota** por medio | Saltar la línea, seguir con el resto, y no perder el objeto final de stdout | Abortar el despliegue por una línea de progreso |
| P-32 | **`--progress` que anuncia pasos de una app que no se pidió** | Ignorar o marcar (§13.6bb: el suceso lleva el nombre de la app precisamente para poder atribuirlo) | Atribuírselos a la app actual |
| P-33 | **`deploy --all` con los seis finales presentes** | Seis recuentos distintos en pantalla | Agrupar en «ok / fallidas» |
| P-34 | **`unreachable` en todas las apps** | «No he podido preguntarle al remoto» | «Nada que hacer» — es el bug real de §13.6bb |
| P-35 | **`doctor` con `fixable:false` y `fix` no nulo** | Enseñar el texto del arreglo **sin** botón | Ofrecer un botón que no hace nada. `fixable` existe exactamente para eso |
| P-36 | **`served:false`** en una app | Pintarlo como el fallo grave que es: hay app y no hay vhost, nginx cierra la conexión y `curl` dice `000` | Pintarlo como «parada» o dejarlo pasar |
| P-37 | **`service: null` en una app estática** | «No aplica» | «Parada» (§13.1: *«pinta una alarma roja donde no pasa nada, y eso enseña a la gente a ignorar las alarmas»*) |
| P-38 | **`cpu_percent: null` la primera vez** | «—» | 0 % |
| P-39 | **`traffic` con `complete:false`** | Decir que la ventana está recortada | Dar el número como si fuera la respuesta |
| P-40 | **`metrics` sin `build_trend_s`** (menos de 6 builds) | No pintar tendencia | Pintar una flecha plana |
| P-41 | **`load: []`** (la línea 11497 lo emite si `/proc/loadavg` no se puede leer) | No pintar carga | Reventar al leer el primer elemento |
| P-42 | **El servidor devuelve la respuesta de otro comando** (`list` contesta lo de `status`) | Error: la forma no encaja | Leer los campos que casualmente coincidan |
| P-43 | **Respuesta con `"app": "otra-app"`** cuando se pidió `mi-app` | Error: la respuesta no es de lo que se preguntó | Pintarla como si fuera de la app pedida |
| P-44 | **El servidor tarda 3 s en el primer byte y luego va rápido** | No confundir latencia con caída | Marcar el servidor como caído |

**P-42 y P-43 merecen una nota**, porque son las que casi nadie pone y son las que
cierran T-02: un cliente que no comprueba que la respuesta corresponde a la pregunta es
un cliente al que un servidor comprometido le puede pintar el estado de otra cosa. La
comprobación es barata —el campo `app` está en `info`, `env list`, `deploy`, `metrics`
y `traffic`— y no hacerla sale gratis hasta el día que no lo es.

### 5.2c Los ocho casos de la ronda 2

Cinco salen de lo que la ronda 1 reconoció que faltaba y **tres están verificados
ejecutando** contra `orbit` v1.3.6 (`EVIDENCIA.md`). Los tres verificados abren una clase
de fallo que el catálogo original no tenía, y es la que más me importa de esta ronda.

| # | Situación | El cliente debe | El cliente NO debe |
|---|---|---|---|
| P-45 | **Goteo lento con datos parciales**: el servidor emite 200 bytes del objeto, espera 20 s, emite otros 200, y así hasta completarlo en 4 minutos | Aplicar el presupuesto de **tiempo total**, no sólo el de «tiempo hasta el primer byte»; y si termina dentro del presupuesto, aceptarlo | Dar por buena la conexión porque «sigue llegando algo»; pintar los campos que ya se han leído |
| P-46 | **Dos comandos concurrentes contra el mismo servidor**: un `deploy` en curso y un `remove --purge` lanzado desde otra pantalla o desde el cliente de un compañero | Releer el estado inmediatamente antes de la acción destructiva y **rechazarla si ha cambiado** desde que se pintó la pantalla («esta app se ha desplegado hace 12 segundos; vuelve a mirar») | Ejecutar sobre un estado que ya era mentira cuando se pintó el diálogo |
| P-47 | **Dos llamadas simultáneas por el mismo `ControlMaster`**, una de ellas larga | Multiplexar: son dos canales sobre una conexión, y eso es lo que `ControlMaster` hace | Serializarlas con un cerrojo propio —anula la ventaja— ni mezclar sus salidas |
| P-48 | **El máster muere a mitad de la segunda llamada** (suspensión, cambio de red) | `ssh -O exit`, **un** reintento sin retroceso, y si vuelve a fallar es el servidor | Reintentar en bucle; dejar el socket huérfano; culpar al servidor a la primera |
| P-49 | **`orbit doctor --fix --json --yes`**: la respuesta que la documentación promete y el binario no da. Verificado: `✗ orbit doctor: no sé qué es «--yes». Sólo acepta --fix y --json.`, rc=1. Y `orbit --yes version --json` responde `✗ Comando desconocido: --yes`, porque **`--yes` no es una bandera global** | **No ofrecer el botón de arreglar.** Enseñar el texto de `fix` como texto copiable, que es exactamente para lo que existe `fixable` | Pintar el botón porque `USAGE.md:1674` y `ARCHITECTURE §19.5` lo documentan |
| P-50 | **Sin TTY, un comando sin app no aborta: elige la primera.** Verificado: `orbit info </dev/null` imprime el menú, **elige `app01` en silencio y sale con rc=0**. Sólo se protegen `info --json`, `deploy --json` y `rollback` | Que el constructor de órdenes **exija el nombre de la app en el tipo**, siempre, aunque parezca redundante | Confiar en que el servidor abortará. Para `restart`/`stop` eso es operar sobre la app equivocada con rc 0 |
| P-51 | **Un servidor que contesta `version --json` en 5 ms** | Sospechar. El suelo medido de `orbit` es **72 ms** por llamada, y ese suelo es estructural: son 13.720 líneas de Bash que el intérprete parsea cada vez, incluso para un comando que no lee ninguna app | Tomarlo como un servidor rápido |
| P-52 | **Respuesta correcta y coherente pero de una versión de `orbit` que no coincide con la que `version --json` declaró hace un minuto** | Revalidar `contract` y avisar: alguien ha actualizado el servidor por debajo, o no es el mismo servidor | Seguir usando el modelo de datos cacheado |

**P-49 es una clase nueva y merece nombre: la respuesta que la documentación promete y el
binario no da.** El catálogo original asumía que la documentación y el comportamiento
coinciden y que lo que hay que probar es lo raro. Aquí lo raro es lo normal: un camino
documentado en dos ficheros, razonado en `ARCHITECTURE.md`, y **muerto por las dos ramas**
—`main()` no reconoce `--yes` como bandera global, y `cmd_doctor` (líneas 12084-12089)
rechaza todo lo que no sea `--fix`—. No lo cazó ninguna prueba de Orbit porque
`tests/doctorfix_test.sh` llama a `cmd_doctor --fix` **como función**, saltándose `main()`,
que es donde vive `ASSUME_YES`; y `tests/lib.sh` carga el script con `sed '$ d'`, o sea
deliberadamente sin `main`. Es el propio §13.6c de `ARCHITECTURE.md` otra vez: *«la prueba
tiene que ejercer el camino»*.

La regla que queda, y es de las que se aplican a todo el producto:

> **La fuente de verdad del cliente es el comportamiento del binario, no la
> documentación.** Cada capacidad que el cliente asume se comprueba ejecutándola contra el
> `orbit` real del contenedor de nivel 1 (§5.4b), **en el arranque de CI**, no leyendo un
> `.md`. Una capacidad que se cae se convierte en un botón que no se pinta, y en un issue
> contra `orbit`.

Y su corolario para el catálogo: **hay que probar el camino feliz documentado igual que se
prueban los patológicos**, porque el camino feliz puede no existir.

**P-50 enlaza directamente con T-07.** El accidente de «servidor equivocado» tiene un
hermano peor y más silencioso: **app equivocada, en el servidor correcto, con código de
salida 0**. `orbit restart` sin app reinicia la primera por orden alfabético y no hay nada
en la salida que diga que hubo una elección. La mitigación no es una comprobación: es que
el tipo del constructor de órdenes **no permita construir la llamada sin app**. Es SEC-26 y
es criterio B-55.

### 5.3 Accesibilidad

Va en el plan de QA y no en un documento aparte por la misma razón por la que Orbit
tiene `UI_ANIM` y un camino ASCII —líneas 345 y 480, con el comentario *«quien
trabaja por una SSH lenta o con un lector de pantalla la quiere quieta»*—: **si no se
prueba, se rompe en el commit siguiente.**

**Automáticas, en CI, fallando el build:**

- `axe-core` o el equivalente de la plataforma sobre cada pantalla, con las reglas de
  WCAG 2.2 AA. Cero violaciones de nivel *serious* y *critical*.
- Contraste comprobado por cálculo sobre los testigos del sistema de diseño, en tema
  claro y oscuro. **Y con especial atención a los colores de estado**: el verde de «ok»
  y el rojo de «error» son los que más veces se eligen por gusto y no por contraste.
- Nombre accesible en todo control interactivo. Un botón que sólo tiene un icono y no
  tiene nombre no pasa.
- Orden de foco recorrido programáticamente en cada pantalla: sin trampas, sin saltos.
- Ninguna información transmitida **sólo** por color. Es especialmente crítico aquí: la
  distinción entre los seis finales de `deploy --all`, y la que hay entre
  `served:false` y `service:null`, no puede depender de un matiz.

**Manuales, en una lista de comprobación por release:**

- Recorrido completo del flujo principal **sólo con teclado**: añadir servidor, listar
  apps, ver una app, desplegar, ver el resultado.
- Lector de pantalla en las tres plataformas: VoiceOver, NVDA y Orca. Lo que hay que
  verificar de verdad: **que el resultado de un despliegue se anuncie** —una región
  viva o su equivalente nativo—, porque un despliegue tarda minutos y quien no ve la
  pantalla no tiene forma de enterarse de que terminó.
- Zoom al 200 % sin pérdida de contenido ni desplazamiento horizontal.
- Movimiento reducido respetado: sin animaciones si el sistema lo pide.
- Modo de alto contraste del sistema operativo.
- **Y una específica de este producto:** con el lector de pantalla, la confirmación de
  `--purge` tiene que anunciar el nombre del servidor y el de la app **antes** que el
  botón. Un diálogo destructivo cuyo primer anuncio es «Botón: Eliminar» es una trampa.

### 5.4 End-to-end contra un VPS real

`DEVELOPMENT.md` es tajante sobre esto y con datos: *«Probar de punta a punta encontró
bugs en los cuatro modos, con 1.258 comprobaciones en verde. EVA: 4. Sencillo: 1.
Menú: 3. JSON: 3.»* Y el primer contacto con un servidor de verdad *«costó cinco
minutos y dos bugs»*. No es opcional.

**Qué se prueba, y sólo se puede probar aquí:**

- El ciclo real: añadir servidor por primera vez (TOFU, huella, `known_hosts`), listar,
  crear una app, desplegar, ver el resultado, revertir, mirar logs, `exec`, retirar.
- **La latencia de verdad.** §13.6d da la cifra del servidor —`list --json` con 40 apps,
  250 ms— y dice que la latencia del contrato **es** la de la interfaz. Lo que falta
  medir es la red: cuánto tarda cada pantalla desde un portátil, con y sin
  `ControlMaster`. Y si se decide usar `ControlMaster` para reutilizar la conexión
  —recomendable: es la diferencia entre 250 ms y más de un segundo por pantalla— hay
  que probar qué pasa cuando el socket de control se queda huérfano.
- `ProxyJump` real, con un bastión de verdad, **y la comprobación de la clave del host de
  salto**, que es el caso que se olvida (T-04).
- Un `deploy` que tarda minutos, con la ventana minimizada, con el equipo suspendido a
  mitad y con un cambio de red.
- El comportamiento con un servidor **grande**: 40 apps, logs de cientos de megas, un
  `orbit traffic` con el tope de `TRAFFIC_MAX_KEYS` alcanzado.
- El primer despliegue de una app **con proceso**, que es donde `health_wait` entra en
  juego, y que §13.6c señala como el camino que las pruebas con una app estática **no
  ejercitan**: *«la lección no es "revisar mejor": es que la prueba tiene que ejercer el
  camino»*.
- Los seis finales de `deploy --all` provocados de verdad: un remoto caído da
  `unreachable`, una rama borrada da `gone`, un commit que ya rompió da `skipped`.
- Que `orbit doctor` sale igual antes y después de usar el cliente (SEC-24 y SEC-25).

**Qué NO se puede probar sin un VPS, y por tanto qué no se puede afirmar hasta que haya
uno:**

- Que el escapado aguanta contra un `/etc/profile` corporativo que imprime un banner antes
  de cada comando. El contenedor de nivel 1 (§5.4b) cubre el `sshd` real y los cuatro
  shells de login, que es la mayor parte; lo que no puede traer es el entorno raro de la
  empresa de alguien.
- Nada relacionado con Let's Encrypt, DNS-01 ni Cloudflare.
- El comportamiento bajo tráfico real: `limit_req`, el 502 del reinicio.
- systemd de verdad: las unidades, el `ProtectHome` que costó un bug (§5.1), el
  arranque en frío.
- La interacción con fail2ban. Y un aviso concreto: **una prueba automática que
  reconecte muchas veces puede hacer que el propio CI se autobloquee** —4 intentos, 4
  horas, según `SECURITY.md` de Orbit—. Se escribe aquí para que no se descubra
  perdiendo el acceso a la máquina de pruebas.

### 5.4b La infraestructura, decidida

La ronda 1 dejó esto como «sin decidir», y sin decidir es como se queda para siempre. La
respuesta es **las dos cosas, y cada una prueba algo que la otra no puede**. Con su coste,
para que la decisión sea real y no una intención.

**Nivel 1 · Contenedor con `sshd` real y el `orbit` real dentro.** Es la pieza que más
valor da por euro y **la única de las tres que puede correr en cada PR**.

- Qué es: una imagen Ubuntu 24.04 con `openssh-server`, un usuario, una clave de pruebas
  generada al vuelo, y **el `orbit` de verdad instalado** —no el falso—, con el banco de 40
  apps del `EVIDENCIA.md` ya sembrado en `/etc/orbit/apps`. Encima, el servidor falso como
  binario alternativo para los casos patológicos que `orbit` no sabe producir.
- Qué prueba, y sólo aquí se puede: **el escapado de punta a punta a través de un shell de
  login real** (B-07), `known_hosts` y el cambio de clave de host (B-22), el ciclo de vida
  del `ControlMaster` entero (§5.1 H), la separación real de stdout y stderr, los códigos
  de salida de `ssh` frente a los de `orbit`, `ProxyJump` con un **segundo** contenedor
  haciendo de bastión (B-24), y los cortes de conexión provocados tirando la interfaz de
  red del contenedor.
- **Y la variante de shells**: la misma imagen con el shell de login del usuario cambiado
  a `dash`, `zsh` y `busybox ash`. Es lo que convierte la prueba de propiedad de §5.1 A en
  una prueba de sistema: no que el escapador sea correcto, sino que **la cadena entera** lo
  es. Cuatro contenedores, o uno con cuatro usuarios, que es más barato.
- Coste: unos minutos de runner por ejecución y cero euros de infraestructura. Se levanta
  con `docker compose` y se puede correr en el portátil, que es la mitad de su valor: una
  prueba que sólo corre en CI no la usa nadie para depurar.
- **Lo que NO prueba:** systemd de verdad, certificados, tráfico real, latencia de red
  real, y nada que dependa del tiempo de un build de tres minutos.

**Nivel 2 · VPS de pruebas efímero, creado y destruido por el pipeline.** Un VPS pequeño
de un proveedor con API —del orden de 5 €/mes si estuviera siempre encendido, y **céntimos
por ejecución** si se crea y se destruye, que es como se usa—, provisionado con el
`install.sh` de Orbit de verdad.

- Qué prueba, y sólo aquí: `install.sh` sobre una imagen limpia (el `apt` que no resuelve
  `build-essential`, que costó dos bugs reales según `DEVELOPMENT.md`), systemd de verdad
  con `ProtectHome`, un despliegue real de tres minutos con la red cayéndose a mitad, el
  arranque en frío, y la latencia contra un servidor en otro continente.
- **Efímero y no permanente**, por tres motivos: un VPS permanente acumula estado y las
  pruebas empiezan a depender de él; se olvida encendido y se paga; y —el importante— **un
  VPS permanente con una app llamada `tienda` es exactamente el incidente de
  `ARCHITECTURE.md` §11.1 esperando a ocurrir otra vez**, ahora con nuestro cliente en vez
  de con su suite.
- **Nunca en un PR.** Corre en cada release, en cada cambio del transporte, y a mano cuando
  alguien lo pide. Motivo: un CI que depende de un proveedor externo acaba siendo un CI
  rojo por motivos ajenos, y un CI rojo por motivos ajenos se ignora — que es el estado 1
  de §5.0 con otra ropa.
- **Con dos guardas de seguridad escritas antes de la primera ejecución:** el `sshd` del
  VPS de pruebas lleva `MaxAuthTries` alto y fail2ban desactivado, porque una batería que
  reconecta cien veces **autobloquea al propio CI** (4 intentos, 4 horas); y el token del
  proveedor es de un proyecto separado, con permiso sólo sobre esa etiqueta de recursos,
  para que un CI comprometido no pueda tocar nada más.

**Nivel 3 · Un VPS de verdad, a mano, una vez antes de la v1.** No es infraestructura: es
una tarde de una persona con el cliente, un dominio real, Cloudflare y Let's Encrypt, y una
lista de comprobación. `DEVELOPMENT.md` documenta que el primer contacto con un servidor de
verdad *«costó cinco minutos y dos bugs»*, y que probar de punta a punta encontró fallos en
los cuatro modos con 1.258 comprobaciones en verde. Es criterio B-48 y su entregable es un
informe escrito, no un «funciona».

**Qué corre dónde, resumido:**

| | Cada PR | Cada release | A mano |
|---|---|---|---|
| Unitarias, parser, fuzzing 60 s | sí | sí | |
| Servidor falso, doble local (90 % del catálogo) | sí | sí | |
| Nivel 1 · contenedor `sshd` + `orbit` real | **sí** | sí | |
| Nivel 1 · variante de cuatro shells | no | **sí** | sí |
| Fuzzing nocturno con corpus | | | nocturno |
| Nivel 2 · VPS efímero | no | **sí** | sí |
| Nivel 3 · VPS real con dominio | no | no | **antes de v1 y por trimestre** |
| Accesibilidad manual, tres lectores | no | **sí** | |

### 5.5 Las pruebas que no se pueden quitar

Orbit tiene una sección con ese nombre: el `curl` con `X-Forwarded-Proto` que evita que
vuelva el bucle de redirecciones, y el build fallido que no puede mover `current`. Aquí
hay cuatro candidatas, y las cuatro son de seguridad:

1. **La propiedad del escapado** (§5.1 A). Si esta cae, T-03 está abierto. Es la única de
   las cuatro que ya existe, y la única que ya ha encontrado algo.
2. **El barrido de secretos en disco.** Flujo completo con secretos marcados
   —`ORBIT_TEST_SECRET_a1b2c3…`— en el `.env` del servidor falso, revelados en la
   pantalla, copiados al portapapeles, con un fallo provocado y un informe generado. Al
   terminar: `grep -r` sobre **todo** lo que la aplicación ha escrito —config, estado,
   caché, temporales, logs, informe— y sobre la telemetría capturada. Cero
   coincidencias. Es SEC-07, SEC-11, SEC-12, SEC-15, SEC-17 y T-12 en una sola prueba, y
   es la traducción directa del método de §11.1: **no leas el código, monta un sistema
   falso encima y mira qué queda escrito.**
3. **El nombre de app hostil de punta a punta** (P-22): entra por el servidor falso,
   pasa por el parser, se pinta, y el árbol resultante no contiene ningún elemento que
   no hayamos creado nosotros.
4. **Los canales de salida de un secreto, medidos en las tres plataformas.** Es la que la
   ronda 2 añade, y es de las que sólo se pueden hacer a mano: copiar un secreto conocido,
   y comprobar **en la máquina de verdad** que (a) desaparece del portapapeles a los 45 s,
   (b) no aparece en el historial de portapapeles del sistema —Win+V en Windows, el gestor
   instalado en macOS—, y (c) el panel del valor revelado sale en negro o no sale en una
   grabación de pantalla hecha con la herramienta del sistema. En Linux **se comprueba que
   falla**, que es igual de importante: el resultado esperado ahí es que el valor sí
   aparezca en la captura, y lo que se verifica es que la interfaz no había prometido lo
   contrario. Una vez por release, con capturas adjuntas al informe. Cierra el −1 que T-06
   se deja marcado en §8.1.

### 5.6 Cobertura: qué se mide y qué no

**Objetivos**, distintos por capa porque el valor de la cobertura es distinto en cada
una:

| Capa | Objetivo | Por qué |
|---|---|---|
| Transporte, escapado, validadores | **100 % de líneas y de ramas**, sin excepciones | Es poco código y es todo el riesgo. Una rama sin cubrir aquí es T-03. |
| Parser del contrato y modelo | ≥ 95 % | Idem, con menos consecuencia. |
| Lógica de presentación (formateo, estados) | ≥ 85 % | |
| Interfaz | Sin objetivo numérico | Ver abajo. |
| **Global** | ≥ 80 %, y **no puede bajar** entre releases | El umbral que no baja vale más que el número. |

**Y lo que NO se mide con cobertura, dicho claro porque es donde la cobertura miente:**

- **Que el escapado sea correcto.** Un escapador roto tiene el 100 % de cobertura si lo
  llamas una vez. Lo que da la garantía es la prueba de propiedad, y la cobertura no la
  ve.
- **Que el catálogo de respuestas patológicas esté completo.** P-45 no existe hasta que
  alguien lo piensa. La cobertura mide el código que hay, no los casos que faltan.
- **Que la interfaz sea usable, legible o accesible.** Un test que monta un componente y
  no afirma nada cubre el 100 % de sus líneas.
- **Que la aplicación no filtre secretos.** Es una propiedad del sistema entero, la mide
  §5.5, y la cobertura no tiene nada que decir sobre ella.
- **Que las pruebas se ejecuten.** El estado 4 de §5.0. La cobertura de una suite
  huérfana es 0, sí, pero **diluida entre las demás no se nota**. Lo que lo detecta es
  el cruce de la clase entera, no el porcentaje.
- **Lo que sólo existe al otro lado del SSH.** El equivalente aquí del *«lo que sólo
  existe dentro del espacio de nombres de systemd no lo ve `make test`»*.

**Y una regla de higiene sacada del incidente del vhost:** ninguna prueba puede tocar
`~/.ssh`, `~/.config` ni el `known_hosts` de quien las ejecuta. Todo va a un `HOME`
temporal, y se **audita montando un sistema falso encima** y mirando qué queda tocado,
no leyendo las pruebas. Una prueba que use un nombre de servidor real es una prueba
esperando a que alguien la lance con su configuración de verdad delante.

---

## 5b. Fuzzing del contrato

**Qué se busca.** Dos propiedades, y son distintas:

- **P1 · El cliente nunca revienta.** Ninguna entrada, válida o no, produce un cierre
  inesperado, un cuelgue, un consumo de memoria sin límite ni una excepción sin
  manejar. El resultado aceptable es siempre uno de dos: se lee correctamente, o se
  rechaza con un error explicado.
- **P2 · El cliente nunca pinta datos inventados.** Es la propiedad importante y la que
  casi nadie fuzzea. Si un campo no llegó, o llegó `null`, o llegó con otro tipo, en
  pantalla no puede aparecer un valor. **Un 0 es una afirmación** (§13.6).

**«Dato inventado», definido de forma que un test lo pueda evaluar solo.** La ronda 1 dejó
esto como el punto flojo del fuzzing: la comparación campo a campo funciona para un
escalar —«llegó `null`, se pintó `—`»— y se vuelve difusa en cuanto la pantalla compone,
que es casi siempre. «Vas a volver 3 despliegues atrás, al de hace 6 días» no se compara
con ningún campo.

**La propiedad no es sobre el valor: es sobre la procedencia.** Cada trozo de dato que
aparece en pantalla tiene que poder señalar de dónde salió, y el conjunto de orígenes
lícitos es cerrado. Operativamente:

> Todo carácter que la interfaz pinte pertenece a exactamente una de estas cinco clases,
> y el test puede decidir a cuál:
>
> 1. **Literal del producto**: texto de la interfaz, de un catálogo de traducción con
>    identificador. Está en el catálogo o no vale.
> 2. **Derivado de campos presentes**, por una función pura y declarada. La función se
>    registra: `fecha_relativa(last_deploy)`, `cuenta_atras(releases, destino)`. El test la
>    vuelve a ejecutar sobre los mismos campos y compara.
> 3. **Marcador de desconocido**: el glifo único —`—`— reservado. **No lo puede producir
>    ninguna otra clase**, así que su presencia es una afirmación comprobable de que el
>    dato faltaba.
> 4. **Entrada del propio usuario**, en un campo que el usuario acaba de escribir.
> 5. **Nada más.**

Lo que hace que esto sea automatizable es la **instrumentación de procedencia**: en las
compilaciones de prueba, cada nodo de texto lleva un atributo con su clase y, si es de la
clase 2, con la lista de campos de los que depende y el nombre de la función. No es una
anotación manual: sale de la capa de formateo, porque **la capa de formateo es el único
sitio del cliente donde un campo se convierte en texto** —eso ya lo exige SEC-18 y §5.1 F—.

Con eso, el aserto del fuzzer es mecánico y vale igual en una vista compuesta:

```
para cada nodo de texto de la pantalla:
    clase 1 -> su cadena está en el catálogo
    clase 2 -> f(campos_declarados) == texto  Y  ningún campo declarado es null/ausente
    clase 3 -> el campo que representa es null o ausente en la respuesta
    clase 4 -> coincide con lo que el guion de prueba escribió
    sin clase -> FALLO: dato de procedencia desconocida
```

Y la comprobación que cierra P2 de verdad, que es la contrapositiva:

```
para cada campo null o ausente en la respuesta:
    ningún nodo de clase 2 lo declara entre sus dependencias
```

Un `cpu_percent: null` que aparezca como dependencia de un nodo que pinta «0 %» falla,
aunque la cadena «0 %» sea perfectamente plausible. **Eso es exactamente lo que no se podía
comprobar en la ronda 1.**

Tres consecuencias que conviene aceptar antes de implementarlo:

- **Obliga a que la capa de formateo sea única y declarativa.** Es una restricción de
  arquitectura que sale de una necesidad de prueba, y está bien que salga de ahí: es la
  misma dirección que la de §5.1 A —la prueba dicta la forma del código— y la contraria a
  añadir una prueba encima de lo que ya está escrito.
- **La instrumentación sólo existe en compilaciones de prueba**, y hay un test que
  comprueba que no aparece en la de release. Un atributo de procedencia en producción es
  información sobre la estructura de datos en el DOM, y no aporta nada.
- **No cubre los gráficos.** Una barra de tráfico no es un nodo de texto. Para los gráficos
  la propiedad se comprueba sobre la serie que se le pasa al renderizador, no sobre los
  píxeles, y se declara así: es la única parte de P2 que se prueba una capa más abajo.

**Arquitectura del fuzzer.** Tres generadores, porque una sola estrategia no cubre las
tres clases de fallo.

**G1 · Generador estructurado (el más valioso).** Parte del esquema de cada respuesta y
genera objetos **válidos** con valores en los extremos: enteros en los límites, cadenas
vacías, cadenas larguísimas, `null` en cada campo que lo permite, colecciones vacías y
de 10.000 elementos, todas las combinaciones de banderas booleanas, los seis finales de
`deploy --all` en todas las proporciones. La aserción no es «no revienta» sino:
**para cada campo, lo que se pinta es función de lo que llegó, y `null` nunca se pinta
como un valor**. Se comprueba con una tabla de correspondencia: el generador sabe qué
metió, y el test comprueba qué salió en la vista.

Que este generador sea el primero no es casual. Fuzzear con basura encuentra caídas;
fuzzear con datos **válidos y extremos** encuentra mentiras, y las mentiras son el
fallo característico de este producto.

**G2 · Mutador.** Toma las respuestas canónicas y les aplica mutaciones: truncar en un
byte al azar, cambiar un tipo, borrar un campo, duplicar una clave —`{"a":1,"a":2}` es
JSON legal y las librerías no se ponen de acuerdo en cuál gana—, anidar diez mil
niveles, meter bytes que no son UTF-8, inyectar caracteres de control escapados,
cambiar `1` por `1.0`, por `1e400`, por `-0` y por un entero de 300 dígitos. Aserción:
P1. Y una segunda, específica: **una mutación que cambia el significado tiene que
cambiar lo que se pinta o producir un error, nunca pasar desapercibida.**

**G3 · Generador adversario.** No genera JSON: genera **lo que un atacante mandaría**.
Cargas de XSS en cada campo de texto, cargas de inyección de comandos, bidi,
homoglifos, rutas con `../`, `file://`, `javascript:`, cadenas que parecen otro JSON,
cadenas que contienen el separador que use nuestro registro interno. Aserción: P1, P2,
y **ninguna de esas cadenas produce un efecto distinto de aparecer como texto**.

**Cómo se ejecuta.**

- En cada PR: 60 segundos por generador, con semilla aleatoria. Barato, y encuentra las
  regresiones.
- Nocturno: una hora, con corpus persistente. El corpus crece y **se guarda en el
  repositorio**: es la memoria de lo que ya se ha probado.
- Cada fallo encontrado se convierte en un caso fijo del catálogo de §5.2b, con su
  entrada P-nn. **El fuzzer descubre; la suite recuerda.** Un fallo que sólo vive en el
  fuzzer se vuelve a encontrar dentro de seis meses, y esa segunda vez cuesta lo mismo
  que la primera.
- Semilla registrada en la salida **siempre**, incluso cuando pasa. Un fallo del fuzzer
  que no se puede reproducir no es un fallo: es una anécdota.

**Y el fuzzing del otro lado, que es la mitad que se olvida.** No sólo se fuzzea lo que
llega: se fuzzea **lo que sale**. Se generan nombres de app, dominios, refs y comandos
de `exec` al azar, se construye el comando, se manda al servidor falso a través del
`sshd` real, y se comprueba que el `argv` que registra el falso es **exactamente** el
que se quiso mandar. Es la prueba de propiedad de §5.1 A ejecutada de punta a punta, y
es lo único que demuestra T-03 resuelto en vez de afirmarlo.

---

## 6. Criterios de aceptación

Numerados y verificables. Cada uno se puede marcar o no marcar sin discusión, que es la
única forma de que una lista así sirva para algo.

### 6.1 Bloqueantes para la v1

**Transporte y ejecución**

1. `B-01` Existe un único módulo que construye órdenes remotas, y es el único que
   invoca `ssh`. Un test de arquitectura falla si otro fichero lo hace.
2. `B-02` **✔ CUMPLIDO (ronda 2).** Todo argumento pasa por el escapador; la prueba de
   propiedad (`argv → escapar → shell → argv` es la identidad) pasa contra `bash`, `dash`,
   `zsh` y `busybox ash`. **Medido: 4 shells × 7.116 casos = 28.464 viajes**, semillas
   20260830 / 1 / 7 / 31337 registradas. Código en `tmp/escape/`. Encontró un fallo real
   (`=` en el conjunto seguro contra la opción `EQUALS` de zsh) y el arreglo fue estrechar
   el conjunto seguro a `[A-Za-z0-9_./-]`, no añadir una prohibición. Queda por decidir si
   `fish` se declara no soportado o se cubre, y el viaje por un `sshd` real es B-07.
3. `B-03` No existe ninguna concatenación de cadenas para formar un comando; verificado
   por lint y por revisión del módulo de transporte.
4. `B-04` `orbit` se invoca por ruta absoluta configurable, nunca por `PATH`.
5. `B-05` Existen dos transportes con nombres distintos, y `remote_shell` se usa desde
   un único fichero.
6. `B-06` Los validadores de forma de la tabla de T-03 existen y se aplican antes de
   construir el comando, **incluidos los datos que vienen del servidor**.
7. `B-07` El fuzzing de salida (§5b) pasa: para cualquier entrada generada, el `argv`
   que llega al servidor falso es el pretendido.

**Contrato**

8. `B-08` Los 44 casos del catálogo §5.2b tienen prueba automática y pasan (los ocho de
   §5.2c van aparte, en B-61: **52 en total**).
9. `B-09` `schema` y `contract` se comprueban antes de interpretar la respuesta.
10. `B-10` Los seis finales de `deploy --all` se modelan y se pintan por separado; hay
    un test que falla si se agrupan.
11. `B-11` `null` nunca se pinta como 0, `""` ni `false`. Test por cada campo nullable
    del contrato.
12. `B-12` `served:false`, `service:null`, `cpu_percent:null`, `complete:false`,
    `requests_capped:true` y `fixable:false` tienen representación propia y probada.
13. `B-13` La respuesta se comprueba contra la pregunta (P-42, P-43).
14. `B-14` Presupuesto de tamaño y de tiempo implementado y probado.

**Secretos**

15. `B-15` La prueba de barrido de secretos en disco (§5.5.2) pasa con cero
    coincidencias.
16. `B-16` El cliente no guarda contraseñas, frases de paso, tokens ni claves privadas.
    Verificado por B-15 y por revisión.
17. `B-17` El `.env` no se puede ver entero; cada valor exige un `env get` explícito y
    se oculta solo.
18. `B-18` La telemetría está apagada por defecto y, encendida, no contiene ninguna
    cadena procedente del servidor o del usuario. Test con estado sensible.
19. `B-19` Los informes de fallo se enseñan completos antes de enviarse y no se envían
    solos.
20. `B-20` El log local no contiene argumentos sensibles.

**SSH**

21. `B-21` `StrictHostKeyChecking=accept-new`, y `~/.ssh/known_hosts` como único
    almacén.
22. `B-22` Un cambio de clave de un host conocido bloquea y no tiene botón de
    continuar. Probado con un `sshd` al que se le cambia la clave entre dos conexiones.
23. `B-23` `ForwardAgent` desactivado, sin interruptor global.
24. `B-24` `~/.ssh/config` se respeta, incluido `ProxyJump`, probado contra un bastión
    real.
25. `B-25` La huella del host se enseña completa en el primer contacto, y «Confiar» no
    es el botón por defecto.

**Comandos peligrosos**

26. `B-26` `--purge` exige escribir el nombre de la app, enseña el inventario de lo que
    se pierde, nombra el servidor, y no tiene «recordar mi elección».
27. `B-27` `--purge` y la retirada simple son dos acciones distintas en la interfaz, no
    una casilla.
28. `B-28` `rollback` elige de una lista, con la release activa desactivada, y avisa
    siempre de las migraciones y —cuando `state.autodeploy` es `true`— del
    autodespliegue, con la acción de desactivarlo al lado.
29. `B-29` `exec` enseña el comando exacto ya escapado antes de ejecutarlo, y tiene los
    dos modos (comando / shell) explícitos.
30. `B-30` `exec` pinta la salida como texto plano, con límite en pantalla.

**Presentación**

31. `B-31` Cero usos de `innerHTML` o equivalentes sobre datos del contrato; regla de
    lint activa. P-22 pasa de punta a punta.
32. `B-32` Cero violaciones de accesibilidad *serious* y *critical* en CI.
33. `B-33` El flujo principal se completa sólo con teclado, verificado a mano.
34. `B-34` El resultado de un despliegue se anuncia a los lectores de pantalla.
35. `B-35` Ninguna información depende sólo del color.

**Almacenamiento y sesión**

36. `B-36` Rutas XDG en Linux, Application Support en macOS, `%APPDATA%` en Windows;
    `0700`/`0600` y la ACL equivalente en Windows, comprobado en una prueba.
37. `B-37` Bloqueo por inactividad, por defecto 15 minutos, que oculta secretos, limpia
    el portapapeles y **no** aborta las operaciones en curso.
38. `B-38` La lista de lo que se persiste está declarada en código, y hay un test que la
    hace cumplir.

**Cadena de suministro**

39. `B-39` Lockfile obligatorio; `cargo audit` / `npm audit` en CI, fallando en
    severidad alta o crítica.
40. `B-40` `--ignore-scripts` por defecto, con lista blanca corta y justificada.
41. `B-41` Los artefactos de release van firmados; el actualizador verifica con una
    clave empotrada, rechaza degradaciones y nunca aplica nada sin avisar.
42. `B-42` SBOM publicado con cada release.
43. `B-43` `SECURITY.md` propio, con dirección de contacto y plazo de respuesta.

**Higiene del proyecto**

44. `B-44` CI falla si existe un fichero de pruebas que el corredor no ejecuta.
45. `B-45` Existe un modo estricto que se niega a dar verde si alguna prueba se saltó, y
    es el que corre CI.
46. `B-46` Ninguna prueba toca el `HOME` real, ni `~/.ssh`, ni `known_hosts`; auditado
    montando un sistema falso encima, no leyendo las pruebas.
47. `B-47` Cobertura ≥ 80 % global, 100 % en transporte y escapado, y no puede bajar
    entre releases.
48. `B-48` El e2e contra un VPS real se ha ejecutado entero al menos una vez antes de la
    v1, con su informe escrito.

**Cadena de suministro del entorno de desarrollo y de la firma** (ronda 2)

49. `B-49` Toda acción de CI se referencia por SHA de commit, nunca por etiqueta.
    Verificado con un `grep` que falla el build.
50. `B-50` La firma ocurre en un trabajo de CI distinto del que compila, sin secretos en el
    de compilación.
51. `B-51` Existe una **clave de reserva del actualizador**, con su pública ya empotrada en
    el binario desde la primera release, y su privada guardada fuera de cualquier máquina
    conectada, en dos sitios físicos.
52. `B-52` El procedimiento de rotación de §7.3b se ha **ensayado en frío** una vez, sin
    incidente, antes de la 1.0.
53. `B-53` Cuarentena de 7 días para adoptar una versión nueva de una dependencia, salvo
    CVE que nos afecte. Verificado por el bot de actualizaciones.
54. `B-54` Se publica, junto al artefacto firmado, el hash del **artefacto sin firmar**
    reproducible desde fuentes en la misma plataforma. La promesa escrita en la
    documentación es ésa y no «releases reproducibles».

**Canal multiplexado** (ronda 2)

55. `B-55` El socket de `ControlMaster` está en un directorio `0700` propio del usuario,
    con modo `0600`, nunca en `/tmp`, y usa `%C`. Comprobado sobre el fichero real.
56. `B-56` `ControlPersist` finito; los másters se cierran al bloquear y al cerrar la
    aplicación, **salvo el de un despliegue en curso**. Las dos mitades probadas.
57. `B-57` El socket huérfano se limpia con `ssh -O exit` y se reintenta **una** vez, sin
    bucle. Probado matando el proceso máster y dejando el fichero.
58. `B-58` La lectura de stdout y stderr es concurrente. Probada con un servidor falso que
    escribe 1 MB por stderr antes del objeto: un cliente secuencial se bloquea para siempre.

**Corrección frente al binario real** (ronda 2)

59. `B-59` El constructor de órdenes **no puede representar** una llamada sin nombre de app
    (SEC-26). Verificado por el sistema de tipos y por un test.
60. `B-60` Ninguna capacidad del servidor se asume a partir de la documentación: cada una
    tiene una prueba que la ejerce contra el `orbit` real del contenedor de nivel 1
    (SEC-27). En particular, **el botón de arreglo automático de la pantalla de diagnóstico
    no se pinta** mientras `doctor --fix --json --yes` no exista.
61. `B-61` Los ocho casos de §5.2c (P-45…P-52) tienen prueba automática y pasan.
62. `B-62` El fuzzer evalúa P2 por procedencia (§5b): ningún nodo de texto sin clase, y
    ningún campo `null` o ausente declarado como dependencia de un nodo pintado.
63. `B-63` La infraestructura de §5.4b existe: contenedor de nivel 1 corriendo en cada PR,
    con la variante de cuatro shells en cada release, y VPS efímero en cada release.

### 6.2 Deseables

1. `D-01` `ControlMaster` para reutilizar la conexión, con la gestión del socket
   huérfano resuelta y **medida**, no supuesta.
2. `D-02` Detección y aviso de claves SSH sin frase de paso (T-08), una sola vez.
3. `D-03` Modo presentación que oculta valores, dominios y hostnames de golpe.
   **Deseable en macOS y Windows; bloqueante en Linux**, donde es la única mitigación
   disponible contra la captura de pantalla (T-06, canal 2).
4. `D-04` Registro local de acciones destructivas, exportable.
5. `D-05` Oferta de copia de seguridad antes de un `--purge`, apoyada en `orbit backup`.
6. `D-06` Exclusión de la captura de pantalla para el panel del valor revelado, con
   `NSWindow.sharingType = .none` en macOS y `SetWindowDisplayAffinity` en Windows. **En
   Linux no existe API y no se promete.** Requiere que el valor revelado viva en su propia
   ventana, no en un `div`.
7. `D-07` Borrado del portapapeles a los 45 segundos con cuenta atrás visible, más marca de
   contenido sensible (`org.nspasteboard.ConcealedType` en macOS,
   `ExcludeClipboardContentFromMonitorProcessing` en Windows). En X11, aviso una vez por
   sesión de que cualquier aplicación puede leer el portapapeles.
8. `D-08` Builds reproducibles.
9. `D-09` Fuzzing nocturno con corpus persistente en el repositorio.
10. `D-10` Marcado de homoglifos y de caracteres bidi en los nombres.
11. `D-11` Comprobación, al añadir un servidor, de que el usuario remoto tiene sudo sin
    contraseña, con un mensaje específico. Evita que P-16 aparezca por primera vez en
    producción.
12. `D-12` Auditoría externa del módulo de transporte y del actualizador antes de la 1.0
    pública.
13. `D-13` Programa de divulgación con reconocimiento público en el changelog, como ya
    hace Orbit.
14. `D-14` Interfaz en español e inglés con el mismo rigor de pruebas que Orbit:
    `i18n_test.sh` comprueba el catálogo entero y detecta las frases que se quedan sin
    traducir; el cliente necesita su equivalente, porque una interfaz medio traducida en
    un diálogo destructivo es un riesgo, no una molestia.
15. `D-15` Recomendación activa, en el alta de un servidor, de usar **un usuario SSH por
    persona** en vez de que todo el equipo entre como `root`. Es la única atribución que se
    consigue sin tocar el servidor (T-11b).
16. `D-16` Detección de operación concurrente: releer el estado justo antes de una acción
    destructiva y rechazarla si ha cambiado (P-46).
17. `D-17` Decidir `fish`: cubrirlo en la prueba de propiedad, o declararlo no soportado en
    la documentación. Lo que no vale es callarse.
18. `D-18` Aviso, en el diálogo de `--purge` de un servidor de producción, de cuándo fue el
    último despliegue de esa app. «Se desplegó hace 40 minutos» es la frase que hace que
    alguien pregunte en el chat antes de borrar (T-11b).

---

## 7. Divulgación y CI de seguridad

### 7.1 `SECURITY.md` propio

**Propio, no un enlace al de Orbit.** Son dos superficies distintas —el servidor no
tiene actualizador ni interfaz gráfica; el cliente no tiene nginx ni certificados— y
compartir el documento haría que ninguno de los dos fuera exacto. Lo que sí se comparte
es la estructura, que en el de Orbit es buena: versiones con soporte, cómo reportar,
**modelo de amenazas con un apartado explícito de «qué NO protege»**, y
recomendaciones para producción.

Contenido mínimo:

- **Versiones con soporte**, con una política honesta: la última menor, y arreglos de
  seguridad para la anterior durante un plazo declarado.
- **Cómo reportar.** `security@intervolutions.com`, sin issue público, con la misma
  promesa de 72 horas que ya da Orbit. Y **una clave pública para quien la quiera**: un
  reporte de una vulnerabilidad crítica enviado por correo en claro es una
  vulnerabilidad más.
- **El modelo de amenazas resumido**, con el apartado de límites conocidos: que un
  equipo comprometido es el fin de la partida, que no auditamos el código que
  despliegas, y que **el cliente tiene privilegios equivalentes a root en tus
  servidores por diseño y eso no es un fallo sino la naturaleza del producto**. Esa
  frase tiene que estar escrita, porque es lo que un usuario necesita saber antes de
  instalarlo en un portátil que se lleva a todas partes.
- **Recomendaciones para el usuario**: clave con frase de paso, agente con caducidad
  (`ssh-add -t`), no reenviar el agente, marcar los servidores de producción, bloqueo
  por inactividad, y telemetría apagada si administra servidores de terceros.
- **Crédito** a quien reporte, salvo que prefiera lo contrario. Es lo que hace que la
  gente reporte en vez de vender el hallazgo.

### 7.2 CI de seguridad

Cinco cosas que fallan el build:

1. **Auditoría de dependencias** en cada PR: `cargo audit` y/o `npm audit --omit=dev`,
   fallando en alta y crítica. Las excepciones —hay CVEs sin arreglo en dependencias
   transitivas y fingir lo contrario bloquea el proyecto— van en un fichero **con fecha
   de caducidad**: pasada la fecha, vuelve a fallar. Una excepción sin caducidad se
   convierte en permanente en tres meses, y entonces el escaneo ya no dice nada.
2. **Análisis estático de seguridad.** Los lints del lenguaje, más reglas propias que
   codifican nuestras reglas duras: la que prohíbe `innerHTML` sobre datos del contrato
   y la que prohíbe construir comandos fuera del módulo de transporte son **reglas de
   `semgrep`, no revisiones humanas**. Una regla que depende de que un revisor se
   acuerde es la misma clase de error que §13.6c le achaca a `deploy`: *«una regla que
   se cumple acordándose no sobrevive a un comando largo»*.
3. **Escaneo de secretos**: `gitleaks` sobre cada PR y sobre el histórico, más un hook
   local. Un secreto que llega al PR ya está en el historial de alguien.
3b. **Fijación de las acciones de CI por SHA de commit** (B-49), comprobada con un `grep`
   que falla el build. Una etiqueta —`@v4`— se puede mover; un SHA no. Es el vector del
   incidente de `tj-actions` de 2025 y es la mitigación de esta lista con mejor relación
   entre coste y daño evitado: quince minutos de trabajo y un bot que mantiene los pines.
4. **SBOM** en CycloneDX o SPDX, generado en el build y **publicado como artefacto de la
   release**. No es burocracia: es lo que permite contestar en diez minutos a
   «¿nos afecta esta CVE?», que es la pregunta que llega siempre un viernes por la
   tarde.
5. **Comprobación de licencias** de las dependencias, para no acabar distribuyendo algo
   incompatible con la MIT del proyecto.

Y dos que no fallan el build, pero se publican:

6. **Un informe de superficie**: número de dependencias de producción, líneas del módulo
   de transporte, y **la lista de comandos `orbit` que el cliente puede generar**. Ese
   último es el que hay que mirar en cada revisión: si la lista crece, la superficie de
   T-03 crece, y crece en silencio.
7. **El resultado del fuzzing nocturno**, con las semillas.

### 7.3 Firma de releases

- **Todos los artefactos firmados**, en las tres plataformas: firma de código de Apple
  con notarización, Authenticode en Windows, y firma separada (minisign o cosign) para
  los paquetes de Linux y para el `.tar.gz`.
- **Checksums publicados y firmados**, no sólo checksums. Un `SHA256SUMS` sin firmar
  junto al binario en el mismo servidor no protege de nada: quien puede cambiar uno
  puede cambiar el otro.
- **La clave de firma no vive en el CI.** Almacén con acceso auditado, o firma con
  identidad efímera y registro de transparencia. Motivo: un CI comprometido con la
  clave dentro firma la puerta trasera, y el actualizador la verifica correctamente.
  Ése es exactamente el ataque que ha ocurrido varias veces en los últimos años y el
  que peor se detecta.
- **La clave pública del actualizador va empotrada en el binario** (T-05c), y con ella la
  de la clave de reserva de §7.3b, desde la **primera** release. Añadirla después no sirve
  de nada: los clientes que ya están instalados no la tendrían, que es justo cuando hace
  falta.
- **Notas de release que dicen qué arregla**, incluidas las de seguridad, con su CVE si lo
  tiene. Un usuario que administra producción necesita poder decidir si actualiza hoy o el
  lunes, y para eso necesita saber qué se arregló.

### 7.3b Compromiso y rotación de la clave de firma

La ronda 1 decía «rotación documentada» y no la documentaba, que es exactamente la forma de
no tenerla. Va escrita, porque **el día que haga falta no habrá tiempo de pensarla** y
porque hay una asimetría cruel en este producto: la clave del actualizador es más peligrosa
que cualquier servidor de cualquier usuario.

**Qué claves hay, y cuál duele.** Son dos y son independientes:

| Clave | Qué protege | Si se compromete |
|---|---|---|
| **Firma del SO** (Developer ID de Apple, Authenticode) | La instalación inicial y la confianza del sistema | Malo, y **revocable por un tercero**: Apple y las CA revocan, y los clientes ya instalados dejan de confiar en el artefacto firmado con ella. Hay a quién llamar. |
| **Clave del actualizador** (Ed25519, minisign; la pública compilada dentro del binario) | Toda actualización futura de todos los clientes instalados | **Catastrófico, y no hay a quién llamar.** No existe revocación: el binario instalado confía en esa clave pública y sólo en ésa, porque va empotrada. |

Esa última línea es la que hay que tener clara antes de escribir el procedimiento:
**la propiedad que hace segura la clave empotrada —que nadie puede sustituirla por el
canal— es la misma que hace imposible revocarla por el canal.** No es un defecto del diseño
de Tauri: es la consecuencia inevitable de anclar la confianza en el binario, y la
alternativa (descargar la clave) es peor.

**Preparación, antes de que pase nada.** Cinco cosas, y las cinco son baratas hoy:

1. **La clave privada no vive en el entorno de desarrollo de nadie.** Está en el secreto de
   CI, cifrada con contraseña, y sólo la usa un trabajo de firma que **no compila** (T-05e,
   mitigación 2). Sólo dos personas tienen acceso a leerla, y ese acceso está auditado.
2. **Una clave de reserva generada hoy, con su pública ya compilada dentro del binario.**
   El actualizador acepta artefactos firmados por **cualquiera de las dos**. La privada de
   reserva vive **fuera de CI**, en frío —una copia en papel o en un módulo de hardware, en
   dos sitios físicos—, y nunca ha tocado una máquina conectada. Es la pieza que convierte
   un compromiso en un incidente de 48 horas en vez de en el final del canal de
   actualización. Sin ella, el único camino es que cada usuario reinstale a mano.
3. **La clave pública, las dos, publicadas** en el repositorio, en el sitio web y en
   `SECURITY.md`, con su huella. Para que alguien pueda verificar por su cuenta.
4. **Un procedimiento de rotación *programada*, ensayado una vez.** Rotar en frío, sin
   incidente, es la única forma de saber que el procedimiento funciona. Se ensaya antes de
   la 1.0 y se repite cada dos años.
5. **Una lista de cómo se avisa**: notas de release, `SECURITY.md`, la banda dentro de la
   propia aplicación, y los canales donde esté el proyecto. Escrita, con quién publica en
   cada uno.

**Si la clave del actualizador se compromete.** En orden, y el orden importa:

1. **Hora 0 · Cortar el canal.** Se retira el `latest.json` o se sirve uno que no ofrece
   ninguna actualización. Un atacante con la clave necesita además el canal para llegar a
   los clientes; quitarle el canal es lo primero y lo más rápido.
2. **Hora 0 · Revocar el acceso al secreto de CI** y rotar todos los demás secretos del
   pipeline, porque lo más probable es que se hayan ido juntos.
3. **Hora 1 · Publicar el aviso, antes de tener el arreglo.** Qué versiones son seguras,
   qué hay que hacer, y **cómo verificar a mano** lo que ya se tiene instalado (huella del
   artefacto). Publicar tarde y completo es peor que publicar pronto e incompleto: la gente
   necesita saber si desactivar las actualizaciones automáticas ahora mismo.
4. **Horas 1-24 · Firmar con la clave de reserva.** Se saca una versión nueva —aunque no
   cambie nada funcional— firmada **sólo** con la de reserva, y esa versión trae empotrada
   una **tercera** clave pública nueva y deja de aceptar la comprometida. Los clientes
   instalados aceptan esta actualización porque ya llevaban la pública de reserva dentro
   desde el primer día. **Éste es el único paso que no se puede improvisar**, y es la razón
   entera de la preparación nº 2.
5. **Horas 1-24 · Auditar qué se publicó con la clave comprometida.** Se compara el hash de
   cada artefacto publicado con el del build reproducible sin firmar (§T-05d). Si algo no
   cuadra, el aviso cambia de tono: ya no es «rotamos por precaución», es «hubo un binario
   que no era nuestro», y entonces hay que decirle a los usuarios afectados que **roten sus
   claves SSH**, porque un cliente malicioso tuvo acceso a su agente.
6. **Día 2-7 · Los que no actualizaron.** Un parque de clientes nunca actualiza entero. Los
   que se quedaron en la versión vieja siguen confiando en la clave comprometida y siguen
   apuntando a la URL del canal, así que **la defensa de esos usuarios es que el canal esté
   controlado por nosotros**, no la criptografía. Por eso el paso 1 va el primero y por eso
   el `latest.json` se sirve desde un sitio del que se puede recuperar el control (dominio
   propio, no una release de un tercero que alguien más pueda editar).
7. **Semana 2 · Post mortem escrito y publicado**, y regeneración de la reserva: después de
   usarla, ya no es una reserva.

**Si sólo se compromete la firma del SO**, el procedimiento es más corto: revocar el
certificado con Apple o con la CA, sacar una versión firmada con el nuevo, y avisar. El
canal de actualización sigue siendo de fiar porque su clave es otra, que es precisamente
por qué son dos claves.

**Y la pregunta desagradable, contestada:** ¿y si la clave de reserva también estaba
comprometida? Entonces no hay canal de actualización, y el único camino es que cada usuario
reinstale a mano desde un artefacto verificado por otra vía. Se dice aquí para que el coste
de mantener la reserva **fuera** de cualquier máquina conectada se entienda como lo que es:
la diferencia entre un mal día y el final del producto.

---

## 8. Puntuación, ronda 2

La ronda 1 se puntuó 74 en seguridad y 78 en QA, y dijo cuál era el techo: *«este documento
describe un cliente que no existe; todo lo que afirma es verificable, y nada está
verificado»*. La ronda 2 no ha añadido páginas: ha convertido tres de esas afirmaciones en
mediciones y ha escrito las dos políticas que estaban mencionadas y no escritas. El
desglose vuelve a ir por apartado, con lo que cambió y lo que sigue sin medir.

### 8.1 Seguridad: **86 / 100** (era 74)

| Apartado | Peso | R1 | R2 | Qué cambió |
|---|---|---|---|---|
| Modelo de amenazas | 20 | 17 | **19** | Entra T-11b (equipo compartido sin control de acceso, con la explicación de por qué no lo puede haber y las cinco cosas que sí se pueden hacer), T-10b (la superficie que el propio cliente crea con `ControlMaster`), T-05e (la cadena de suministro del entorno de desarrollo) y C6 desarrollado. Los tres huecos que la ronda 1 se marcó a sí misma están cerrados. **Falta** el escenario de un usuario que administra servidores de un cliente final —quién responde ante quién— y no hay análisis de qué pasa cuando el usuario deja la empresa. |
| Inyección de comandos (T-03) | 25 | 21 | **24** | Deja de ser una afirmación. 28.464 viajes de ida y vuelta en cuatro shells, semillas registradas, y **un fallo real cazado** —`=` contra la opción `EQUALS` de zsh, 5 casos de 2.529, invisible en bash— con el arreglo correcto: estrechar el conjunto seguro, no prohibir un carácter. Además se corrigió una imprecisión de la ronda 1: `printf %q` de bash **no** vale, porque produce `$'\n'`, que `dash` no entiende. **Falta** el viaje por un `sshd` real (B-07) y decidir `fish`. Ese −1 es honesto: la propiedad está probada contra un shell local, no contra la cadena entera. |
| Gestión de secretos | 15 | 13 | **14** | El canal 2 de T-06 pasa de «lo que debería pasar» a lo que hace cada plataforma, con la conclusión incómoda escrita: en X11 no se puede prometer nada, la exclusión de captura no existe en Linux, y ninguna plataforma protege de una foto. De ahí sale una decisión de arquitectura —el valor revelado vive en su propia ventana, porque `sharingType` es por ventana— y una reordenación: el modo presentación pasa a bloqueante en Linux. **Falta** ejecutarlo: sigue sin haber una medición propia de si Maccy o Raycast respetan de verdad la convención en la versión actual. |
| Canal SSH | 10 | 8 | **9** | T-10b analiza la superficie que la ronda 1 sólo mencionaba: qué es un socket de control (una sesión root guardada en un fichero), dónde vive, con qué permisos, cuánto dura, qué pasa al bloquear y qué pasa en una máquina compartida. Cuatro reglas nuevas (SEC-28 a SEC-31) y cuatro criterios. **Falta** medir el coste real de `ControlPersist=120` frente a `600` en una red mala: la elección de 120 está razonada, no medida. |
| Cadena de suministro | 15 | 9 | **13** | Era el apartado más débil y era lo que más pesaba. Ahora tiene: T-05e con cinco mitigaciones concretas —acciones por SHA, firma en un trabajo sin código de terceros, `--ignore-scripts` también en desarrollo—; la tabla de qué es reproducible con Tauri y qué **no puede serlo** (el instalador firmado, nunca, por la marca de tiempo y el ticket de notarización), con la promesa acotada al artefacto sin firmar; el procedimiento escrito para el día que caiga la cuenta de un mantenedor, con la cuarentena de 7 días como la medida que convierte la mayoría de esos incidentes en un no-evento; y §7.3b entera. **Falta** ejecutar el ensayo de rotación en frío (B-52), que es lo que separa un procedimiento escrito de uno que funciona. |
| Comandos peligrosos | 10 | 9 | **9** | Sin cambios de fondo. Se refuerza con P-50: el hermano silencioso del «servidor equivocado» es **app equivocada con rc 0**, verificado ejecutando, y la mitigación es de tipos (SEC-26), no de comprobación. Sigue perdiendo un punto por la lista negra de patrones de `exec`. |
| Divulgación y CI | 5 | 4 | **5** | §7.3b escribe lo que faltaba: dos claves con propiedades distintas, la asimetría de que la clave empotrada no se puede revocar por el canal, la clave de reserva empotrada desde el día uno, el procedimiento por horas, y la pregunta desagradable contestada (si la reserva también cayó, no hay canal). |
| **Total** | 100 | 74 | **86** | |

**Lo que haría falta para 92**, y ninguna se compra escribiendo: ejecutar B-07 (el viaje por
un `sshd` real con los cuatro shells de login), ensayar la rotación de clave en frío, medir
el comportamiento real de los gestores de portapapeles, y una auditoría externa del
transporte y del actualizador por alguien que no seamos nosotros.

### 8.2 Calidad de QA: **87 / 100** (era 78)

| Apartado | Peso | R1 | R2 | Qué cambió |
|---|---|---|---|---|
| Pirámide y estructura | 15 | 13 | **14** | Entra la capa que faltaba: concurrencia (§5.1 G) y ciclo de vida de la conexión (§5.1 H), las dos con aserciones que se comprueban sobre el sistema —contar procesos `ssh`, mirar el modo del socket— y no leyendo el código. La prueba del *deadlock* de tuberías es la que más bugs va a evitar por línea escrita. |
| Servidor falso y catálogo | 25 | 23 | **24** | 52 casos. Los ocho nuevos incluyen los dos que la ronda 1 echó en falta (goteo lento, concurrentes) y **tres verificados ejecutando**. Y abre una clase que no estaba: **la respuesta que la documentación promete y el binario no da** (P-49, `doctor --fix --json --yes`), con su regla —la fuente de verdad es el binario— y su corolario: hay que probar el camino feliz documentado igual que los patológicos, porque puede no existir. |
| Fuzzing | 15 | 12 | **14** | «Dato inventado» deja de ser difuso. La propiedad se traslada del valor a la **procedencia**: cinco clases cerradas por nodo de texto, instrumentación que sale de la capa de formateo, y la contrapositiva que es la que cierra P2 —ningún campo `null` puede figurar entre las dependencias declaradas de un nodo pintado—. Se acepta explícitamente lo que no cubre: los gráficos, que se prueban una capa más abajo, sobre la serie. |
| Unitarias | 10 | 8 | **9** | §5.1 A pasa de plan a hecho, con el resultado y el fallo encontrado. Entran G y H. **Falta** el corpus persistente del fuzzer. |
| Accesibilidad | 10 | 8 | **8** | Sin cambios. Sigue faltando el detalle de cómo se anuncian los cambios de estado en vivo sin volverse ruidosos, que es el problema real de una pantalla que se refresca cada dos segundos. |
| End-to-end | 10 | 8 | **9** | §5.4b decide la infraestructura en tres niveles, con qué prueba cada uno, qué **no** puede probar, coste, y una tabla de qué corre en cada PR, en cada release y a mano. Con dos guardas escritas antes de la primera ejecución: fail2ban desactivado en el VPS de pruebas —una batería que reconecta cien veces autobloquea al CI— y token de proveedor con permiso sólo sobre esa etiqueta. **Falta** ejecutarlo. |
| Cobertura y su honestidad | 10 | 9 | **9** | Sin cambios; sigue siendo el apartado que mejor envejece porque lo que dice es qué **no** mide. |
| Criterios de aceptación | 5 | 5 | **5** | 63 bloqueantes y 18 deseables. B-02 marcado como cumplido con su cifra, que es la primera casilla que se marca de verdad. |
| **Total** | 100 | 78 | **87** | |

**Lo que haría falta para 92:** ejecutar la infraestructura de §5.4b —no diseñarla—; el
corpus persistente del fuzzer con tres meses de historia; resolver el anuncio de estados en
vivo para lectores de pantalla; y la única que no depende de nosotros, que es que la suite
haya corrido el tiempo suficiente para que detrás de una parte de sus comprobaciones haya un
incidente con nombre, como pasa en Orbit.

### 8.3 Lo que sigue sin medir, dicho igual de claro que en la ronda 1

La ronda 1 cerraba diciendo que describía un cliente que no existe. Sigue sin existir, y esa
frase sigue siendo el techo de las dos notas. Lo que ha cambiado es **qué parte del documento
está apoyada en una ejecución**:

**Medido, con su artefacto reproducible:**

- El escapador y su propiedad: 28.464 viajes, cuatro shells, semillas registradas, un fallo
  encontrado y arreglado (`tmp/escape/`).
- La latencia del contrato con 40 apps: suelo de **72 ms**, `list --json` 306 ms,
  `status --json` 389 ms, `info` 86 ms (`tmp/bench/`). La cifra de 250 ms de
  `ARCHITECTURE.md` §13.6d se sostiene: mismo orden de magnitud en otra máquina.
- `doctor --fix --json --yes` no existe, por las dos ramas.
- Sin TTY, un comando sin app elige la primera y sale con 0.

**Escrito y no ejecutado, que es la mayoría:**

- Los 52 casos del catálogo. Están razonados y varios salen de bugs reales de Orbit, pero
  ninguno tiene todavía un servidor falso que los produzca.
- Las tres capas de infraestructura de §5.4b.
- El procedimiento de rotación de clave. Es el que más me preocupa de esta lista, porque un
  procedimiento de incidente que no se ha ensayado **es una lista de buenas intenciones con
  formato de lista numerada**, y se descubre el peor día.
- Todo lo de portapapeles y captura: las APIs son las correctas y el comportamiento está
  documentado por sus fabricantes, pero no lo hemos visto con nuestros ojos en las tres
  plataformas.

**Y la lección de método de esta ronda, que es la misma que la de la anterior y por eso
merece quedar escrita dos veces.** `ARCHITECTURE.md` §13.6d dice que dos cifras suyas eran
falsas y que *«ninguna cambiaba la decisión, las dos habrían pasado tres revisiones… y las
dos se cayeron al medir de verdad»*. Esta ronda tiene su versión: la ronda 1 afirmaba que la
prueba de propiedad daría la garantía, y era verdad; lo que no sabía es que **la garantía
sólo aparece con la cuarta columna de la tabla**. Con bash, la salida habría sido
«2.529/2.529, propiedad sostenida» y el documento habría dicho lo mismo, con la misma
seguridad, siendo falso. La prueba no vale por los 28.464 viajes que salieron bien: vale por
los cinco que fallaron.

---

