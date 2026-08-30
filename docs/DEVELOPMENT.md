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
