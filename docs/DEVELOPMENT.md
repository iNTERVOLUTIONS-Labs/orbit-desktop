# Cómo se trabaja en Orbit Desktop

Este documento es el de operaciones: cómo se monta el entorno, cómo se prueba
sin un VPS, y las trampas que ya conocemos. Las decisiones están en
[ARCHITECTURE.md](ARCHITECTURE.md); aquí está lo que hay que teclear.

---

## 1. Lo que hace falta

| | Para qué |
|---|---|
| `bash`, `dash`, `zsh`, `busybox` | La prueba de propiedad del escapado corre contra los cuatro |
| `python3` | El banco y las pruebas del contrato |
| Una copia del repositorio de **[orbit](https://github.com/iNTERVOLUTIONS-Labs/orbit)** | El banco parte de su script |
| `ssh` (OpenSSH) | El transporte. No embebemos un cliente SSH |

La cadena de compilación de la aplicación (Rust y Node) se documenta cuando
exista la aplicación. Hoy no hay ninguna, y decir qué versión de Node hace falta
para un código que no está escrito es la clase de dato que caduca antes de
usarse.

---

## 2. El banco: un Orbit de mentira con 40 apps

**Por qué existe.** `ARCHITECTURE §13.6d` de Orbit dice que *«la latencia del
contrato es la de la interfaz»* y da cifras para 40 apps. Un cliente que se fíe
de esas cifras sin reproducirlas está construyendo sobre una afirmación. Además,
un VPS real **no sabe devolver las respuestas patológicas**, que son justo las
que hay que probar.

```bash
tools/bench/make-bench.sh ../orbit 40
tools/bench/measure.sh
```

Lo que hace: copia `orbit`, parametriza `ETC_DIR` y `LOG_FILE` —que están fijos
en el script, al contrario que `APPS_DIR`— y apaga la auto-elevación de la línea
241. **No toca el repositorio de Orbit**, y eso se comprueba con `git status`
allí después de cada tanda.

Las apps se generan de cuatro tipos a propósito (`static`, `node`, `next`,
`php`). Una tanda de apps todas iguales mide una cosa que no existe: las
estáticas y las de PHP no tienen puerto ni servicio, así que son las únicas que
ejercitan los `null` del contrato — y los `null` son la mitad del trabajo del
cliente.

### Lo que el banco NO puede ver

Y conviene tenerlo delante, porque la tentación es creerse que una suite verde
aquí significa algo:

- **No hay systemd.** `svc_state` contesta lo que puede, así que los estados de
  servicio del banco son plausibles, no reales.
- **No hay nginx ni certificados.** `served` sale `false` para todo y `ssl`
  también. Está bien para probar cómo se pinta un `served:false` —que es un
  estado importante y difícil— y no vale para nada más.
- **No hay red.** Todas las cifras de `measure.sh` son del lado del servidor. La
  latencia que siente el usuario es ésa **más** el viaje, y sin multiplexado,
  más un saludo SSH completo por pantalla.
- **No hay un shell remoto.** El escapado de argumentos **no se prueba aquí**.
  Se prueba en `tests/escaping`, y de punta a punta hace falta un `sshd` de
  verdad.

Es la lección que Orbit se llevó con su propia suite: *lo que sólo existe dentro
del espacio de nombres de systemd no lo ve `make test`*. Aquí la frase
equivalente es: **lo que sólo existe dentro de un shell remoto no lo ve un doble
local.**

---

## 3. La prueba de propiedad del escapado

Es la pieza más importante del proyecto, porque de ella depende que el nombre de
una app no se convierta en un comando.

```bash
python3 tests/escaping/prop_test.py            # semilla por defecto
python3 tests/escaping/prop_test.py 31337 5000 # semilla y número de casos
```

La propiedad: para cualquier lista de cadenas,
`argv → escapar → shell remoto → argv` es **la identidad**. El «programa remoto»
imprime su `argv` separado por bytes nulos, que es el único separador que no
puede aparecer dentro de un argumento.

Corre contra **cuatro shells** —`bash`, `dash`, `zsh` y `busybox ash`— y el
motivo no es exhaustividad: **el shell de login del usuario remoto no lo elegimos
nosotros.**

**La semilla se imprime siempre, también cuando pasa.** Un fallo del fuzzer que
no se puede reproducir no es un fallo, es una anécdota.

### Lo que encontró la primera vez que se ejecutó

Falló. Cinco casos de 2.529, y **sólo en zsh**. El escapador tenía un conjunto de
caracteres «seguros» que se pasan sin comillas y `=` estaba dentro; zsh expande
las palabras que empiezan por `=` (opción `EQUALS`: `=ls` se sustituye por la
ruta de `ls`), así que el argumento `=Y` volvía como `zsh:1: Y not found`.
`bash`, `dash` y `busybox` pasaban los 2.529.

Es el modo de fallo exacto contra el que existe la prueba: **correcto en el shell
donde se desarrolla, roto en el que usa el usuario.** El arreglo no fue añadir
`=` a una lista de prohibidos —una lista negra crece cada vez que alguien
encuentra un carácter nuevo, que es la firma de que el diseño estaba mal— sino
**estrechar el conjunto seguro** a `[A-Za-z0-9_./-]` y entrecomillar todo lo
demás.

La regla que queda: *cada carácter que se añade al conjunto seguro es una regla
de expansión de cuatro shells que hay que conocer. Entrecomillar de más no cuesta
nada; entrecomillar de menos es una ejecución de comandos.*

Y un caso que no se escapa sino que se **rechaza**: un byte nulo. No puede viajar
en un `argv`, y fingir que sí es peor que fallar.

---

## 3b. El `sshd` del banco, y lo que sólo se ve ahí

```bash
tests/e2e/montar-sshd.sh /tmp/e2e-orbit
ORBIT_E2E=/tmp/e2e-orbit cargo test -p orbit-client --test e2e_sshd -- --ignored --nocapture
tests/e2e/parar-sshd.sh /tmp/e2e-orbit
```

Levanta un `sshd` de verdad en el 2222 con sus propias claves, su propio
`known_hosts` y el servidor falso instalado al otro lado. No toca el `sshd` de
la máquina ni el `~/.ssh` del usuario.

**Por qué no es opcional.** El doble local cubre el parser, los `null` y los
seis finales en milisegundos, y no cubre nada de lo que de verdad se rompe en el
camino: el escapado atravesando `sshd` y un shell de login, `known_hosts`, el
multiplexado, la separación de stdout y stderr sobre un canal, y los códigos de
salida de `ssh` frente a los de `orbit`.

Y hay una propiedad incómoda debajo que conviene tener escrita: **`sshd` siempre
entrega la petición al shell de login del usuario remoto, y OpenSSH concatena
con espacios los argumentos que le sobran.** O sea que «pasarle un `argv`
separado a `ssh`» es una creencia falsa, y muy extendida. Por eso la cadena la
construimos nosotros, escapada, y por eso esta prueba comprueba **lo que le
llegó** al otro lado y no lo que devolvió: el servidor falso apunta su `argv`
con los campos separados por bytes nulos, y ése es el único testigo fiable.

### Lo que ha pagado ya

Se escribió para cerrar un criterio pendiente y encontró dos cosas en su primera
ejecución. Ninguna se veía contra el doble local, y las dos son de las que se
descubren tarde:

- **Un cambio de clave de host le llegaba a la interfaz como «no he llegado al
  servidor».** Es la descripción de un problema de red, no la de un ataque de
  suplantación — que es lo que eso es. Ahora tiene su propio error
  (`ClaveDeHostCambiada`), su propio texto y conserva el detalle de OpenSSH, que
  es donde va la huella y la línea del `known_hosts` que hay que quitar. Un
  doble local no tiene claves de host, así que ahí no podía aparecer.
- **El mensaje de error se sacaba de la primera línea**, y cuando la clave de un
  host cambia OpenSSH abre con tres líneas de arroba. La interfaz iba a enseñar
  un muro de `@@@@@@@` justo en el único error del canal que hay que leer
  entero. Ahora se busca la primera línea **con palabras**.

Y una tercera, del propio banco: **el registro del `argv` se partía con un
argumento que llevara un salto de línea dentro**, porque usaba el salto como
separador de registro. El caso `a\nb` volvía como una lista vacía y la prueba
acusaba al escapado de un fallo que era del instrumento. Es la regla de siempre:
antes de creerte una medición, comprueba qué marca en reposo.

### Lo que este banco tampoco ve

- **Latencia de red.** Todo va contra `127.0.0.1`, así que el saludo no cruza
  nada. Medido aquí: 91 ms sin multiplexar frente a 20 ms con él. Contra un VPS
  real la diferencia es de otro orden —246 ms frente a 13 ms— y es la palanca de
  latencia más grande del producto. Por eso la prueba **no afirma un factor**:
  sólo afirma que multiplexar no sale más lento, que es lo único honesto que se
  puede medir en un bucle local.
- **`ProxyJump` con un bastión de verdad**, ni `ProxyCommand`, ni una llave en
  hardware, ni un `sshd` endurecido con `requiretty`.
- **Un `sudo` que pida contraseña.** El caso está en el servidor falso, pero
  aquí se entra como el propio usuario.

---

## 4. Las trampas que ya conocemos

Heredadas de Orbit o encontradas auditándolo. Están aquí para no volver a
pagarlas.

**La suite que se salta sola.** Sin `zsh` instalado, la prueba de propiedad
podría dar verde habiendo probado tres shells de cuatro. Orbit ya se comió esto:
sin `jq`, `rsync` y `nginx`, su `make test` salía verde habiendo hecho 1.662
comprobaciones de 2.447 —medido apartándolos del `PATH`, no estimado—, y por eso
existe `make test-strict`. Aquí: **el modo estricto se niega a dar verde si un
shell no estaba.**

**La suite que no ejecuta nadie.** `provision_test.sh` estuvo en el repositorio
de Orbit desde su propio commit sin estar en el `Makefile`: 36 comprobaciones que
no corría nadie, y no dejaba hueco porque el total sólo suma lo que sí se lanzó.
Aquí: el corredor **cruza los ficheros de prueba que hay contra los que ejecuta**
y falla si sobra alguno.

**La suite que hace daño de verdad.** La de Orbit, ejecutada como root en un
servidor que tenía una app llamada `tienda`, **borró el vhost de la app de
verdad**. 32 suites en verde, 2.512 comprobaciones, 0 fallos, y una web muerta.
De ahí sale una regla de este repositorio: **ninguna prueba apunta a un servidor
que no sea de pruebas**, y el nombre del servidor va en cada aserción destructiva.

**No leas el código para saber si toca el sistema.** Móntale un sistema falso
encima y mira qué queda tocado. Es como se auditó el banco y es como se audita
todo lo demás.

**Antes de creerte una medición, comprueba qué marca en reposo.** Una sonda que
se acusa a sí misma da un número perfectamente consistente y perfectamente falso.

---

## 4b. Las excepciones de la auditoría, y por qué caducan

`cargo audit` corre en cada tanda y **una vulnerabilidad falla el build, sin
excepciones**. Es código de terceros ejecutándose en el proceso que sostiene las
credenciales SSH del usuario, que es el argumento con el que se descartó
Electron: no vale relajarlo aquí.

Lo que sí admite excepción es un aviso de **«sin mantenimiento» para el que no
hay arreglo**. Hoy hay diecisiete, y ninguna es una vulnerabilidad: son las
ligaduras de **GTK3** que arrastra Tauri en Linux —`atk`, `gdk`, `gtk`,
`gdkwayland`, `gdkx11` y sus `-sys`— marcadas así porque el ecosistema de
`gtk-rs` se movió a GTK4. Tauri sigue en GTK3 porque es lo que usa `webkit2gtk`,
así que **no hay una versión mantenida a la que subir**: no es una actualización
que se nos haya olvidado hacer.

Van en `.cargo/audit.toml`, **una a una por su identificador** —nunca una regla
que silencie una categoría entera— y con una fecha:

```
# CADUCA: 2027-03-01
```

Y la fecha se comprueba, que es lo único que la hace valer:

```bash
tools/caducidad-excepciones.sh
```

Corre en CI **antes** de auditar y falla si la fecha pasó, o si hay excepciones
y no hay fecha. Sin eso, una excepción se vuelve permanente en tres meses y a
partir de ahí el escaneo sigue saliendo en verde **habiendo dejado de mirar**,
que es peor que no tenerlo: afirma algo que ya no comprueba.

Cuando caduque, lo que toca es mirar si Tauri ya ha migrado a GTK4 —o si el
envoltorio sigue justificándose— y no ampliar la fecha por inercia.

---

## 5. Cómo se audita el contrato cuando Orbit cambia

Orbit sube de versión sin que el contrato cambie —es lo normal— pero cuando
cambia hay que enterarse el mismo día, no cuando lo note un usuario.

- `orbit version --json` publica **dos** versiones, la de Orbit y la del
  contrato. Son ejes distintos y confundirlos hace que un cliente rechace un
  servidor perfectamente compatible.
- En CI, un script extrae del `orbit` real la lista de comandos capaces de JSON y
  la lista de campos de una app, y **falla si divergen de las del cliente**. Es
  la misma idea que `ORBIT_APP_FIELDS` resolvió dentro de Orbit: una lista, no
  tres sitios que hay que acordarse de tocar.
- Y lo que no se puede automatizar: **cuando la documentación de Orbit y su
  comportamiento discrepan, manda el comportamiento.** Se comprueba ejecutando.
