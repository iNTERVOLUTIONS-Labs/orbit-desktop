# El contrato `--json`, tal y como ES

> Auditoría de la superficie que Orbit Desktop consume: `orbit … --json` sobre
> Orbit v1.3.6. **Está ejecutada, no leída.** Cuando el comportamiento no se
> deduce de la lectura, se montó un banco y se llamó a `main()` — que es justo
> la parte que las pruebas del repositorio de Orbit no ejercitan, porque
> `tests/lib.sh` carga el script con `sed '$ d'`, sin su última línea.
>
> Esa laguna es la que dejó pasar el hallazgo de §1.6b: un comando documentado
> que no existe.
>
> El diseño del cliente que consume esto está en **[CLIENT.md](CLIENT.md)**.
> Las decisiones que lo enmarcan, en **[ARCHITECTURE.md](ARCHITECTURE.md)**.
> Cómo reproducir el banco, en **[DEVELOPMENT.md](DEVELOPMENT.md)**.

> **Cómo leer las referencias.** Este documento y [CLIENT.md](CLIENT.md) eran uno solo y comparten numeración: aquí vive la §1 y allí de la §2 en adelante. Un «§3.2» citado aquí está en CLIENT.md.

---

## 1. Auditoría del contrato tal y como ES

### 1.0 Por qué esta sección va primero y ocupa un tercio del documento

El BRIEF describe el contrato `--json` como «TODA la API disponible». Es exacto, y por eso el
riesgo del proyecto no está en elegir Tauri o Electron: está en construir seis pantallas sobre un
contrato que se conoce por lo que dicen `USAGE.md` y `ARCHITECTURE.md §13` en vez de por lo que
hace el script. Los documentos de Orbit son extraordinariamente buenos —mejor razonados que el 95 %
de lo que se publica—, pero llevan seis meses de commits encima y en cuatro puntos concretos ya no
describen el programa. Uno de esos cuatro es un comando documentado que **no se puede ejecutar**.

Empezar por el stack y descubrir esto en la fase 3 sería descubrirlo con la interfaz ya escrita
encima. Empezar por aquí cuesta un día.

### 1.1 Cómo se parsea `--json`, línea a línea

El reconocimiento de `--json` vive en dos sitios y los dos son necesarios.

**Primero, la lista de comandos capaces**, `_json_capable()`, líneas 1110-1121:

```bash
_json_capable() {
  case "$1" in
    list|ls|info|show|status|doctor|check|env|top|deploy|up) return 0 ;;
    metrics|metricas|métricas) return 0 ;;
    traffic|trafico|tráfico) return 0 ;;
    redirect|redir|db|database|watch) return 0 ;;
    queue|cola|colas) return 0 ;;
    version|-v|--version) return 0 ;;
    *) return 1 ;;
  esac
}
```

Obsérvese que **acepta el comando, no el subcomando**. `env`, `db`, `redirect`, `watch` y `queue`
entran enteros, y el filtro fino se hace después, dentro de cada `cmd_*` (§1.3).

**Segundo, el extractor**, `_json_strip()`, líneas 1125-1133:

```bash
declare -a JSON_ARGS=()
_json_strip() {
  JSON_ARGS=()
  local a
  for a in "$@"; do
    if [[ "$a" == "--json" ]]; then JSON="yes"; else JSON_ARGS+=("$a"); fi
  done
}
```

Es una criba plana: recorre todos los argumentos, marca `JSON=yes` si encuentra `--json` en
cualquier posición y devuelve el resto en `JSON_ARGS`. No tiene el `--` de fin de opciones que sí
tiene `_lang_strip` (línea 1145), así que **un `--json` literal detrás de un `--` también se lo
come**. En la práctica sólo importaría en `orbit env set app FLAGS -- --json`, y ahí `env set`
rechaza `--json` una línea antes por otro motivo, pero conviene saber que la asimetría existe.

**Y el pegamento**, en `main()`, líneas 13655-13659:

```bash
  if _json_capable "$cmd"; then
    _json_strip "$@"; set -- "${JSON_ARGS[@]}"
  elif [[ "$JSON" == "yes" ]]; then
    die "'orbit %s' no tiene salida JSON. La tienen: %s." "$cmd" "$(_json_cmds_help)"
  fi
```

`$JSON` sólo puede valer `yes` al llegar aquí si `--json` iba **delante** del comando, porque el
bucle de banderas globales de la línea 13627 lo consume. Ésa es la clave de una asimetría que un
cliente tiene que conocer y que se documenta en §1.4.

Inmediatamente después, línea 13663, se llama a `_ui_route()` (definida en 447):

```bash
UI_FD=1
_ui_route() { [[ "$JSON" == "yes" ]] && UI_FD=2 || UI_FD=1; }
```

Ése es el mecanismo entero del que depende la promesa «por stdout sólo el JSON». `title`, `ok`,
`info`, `warn`, `hr` y el spinner escriben en `>&$UI_FD` (líneas 464-470, 837-850). `err` y `hint`
escriben siempre en `2` (471, 468). No es una convención que se cumpla a mano comando a comando:
es estructural, y por eso se puede confiar en ella. Verificado ejecutando `deploy --json`, que es
el comando que más habla: stdout salió con un único objeto y nada más, incluso fallando.

**La latencia.** El comentario de `_app_state_json` (línea 1526) trae los números que Orbit midió
con `strace` sobre 40 apps: `list --json` pasó de 842 procesos y 630 ms a 242 y 250 ms al dejar de
capturar los ayudantes JSON en `"$( )"`. Es el dato de diseño más importante del cliente y está en
§13.6d: **la latencia del contrato es la latencia de la interfaz**.

**Ronda 2 · esa cifra se ha vuelto a medir y se sostiene, con un matiz que no estaba.** Sobre el
banco propio de 40 apps salen **314 ms** —mismo orden, máquina distinta y sin systemd— pero
midiendo también `version --json`, que no lee ninguna app, aparecen **76 ms de suelo por llamada**
que no están en ninguna documentación de Orbit: son las 13.720 líneas de Bash que bash parsea cada
vez, más el catálogo de i18n. O sea que **la recta no es `250 ms`, es `72 ms + 5,9 ms por app`**, y
el término independiente se paga en cada pantalla aunque el servidor tenga una sola web. Todo el
capítulo 5 sale de ahí; el detalle en §5.0.

### 1.2 Los comandos que SÍ hablan JSON, con su forma real

Quince entradas, contando alias. Todo lo que sigue está copiado de una ejecución real.

| Invocación | Forma de stdout | Notas verificadas |
|---|---|---|
| `orbit version --json` | `{"schema":1,"version":"1.3.6","contract":1}` | La llamada más barata. Pero **sí exige** que Orbit esté instalado: sin `/etc/orbit/orbit.conf` muere en la línea 265. Ver §1.9 |
| `orbit list --json` | `{"schema":1,"apps":[…]}` | `apps` puede ser `[]`. Verificado |
| `orbit info <app> --json` | `{"schema":1,"app":{name,path,config,state,releases}}` | `config` tiene **37** claves, no 31 (§1.6a) |
| `orbit status --json` | `{"schema":1,"host":{…},"services":[…],"apps":[…]}` | `apps` es el **mismo** array de `list --json`, entero. El BRIEF lo elide con «…» |
| `orbit doctor --json` | `{"schema":1,"checks":[…],"summary":{ok,warn,error}}` | Sale con **0 aunque haya errores** (línea 12102, deliberado) |
| `orbit top --json` | `{"schema":1,"apps":[…]}` | Tarda **~1 s fijo** (`sleep 1`, línea 8760) |
| `orbit metrics [app] --json` | `{"schema":1,"apps":[…],"kept":2000}` | **Siempre envuelto en `apps[]`**, incluso con una app. Y trae `kept`, que no está documentado |
| `orbit traffic [app] --json` | `{"schema":1,"apps":[…]}` | Igual: envuelto siempre. Omite las apps `redirect` |
| `orbit deploy <app> --json [--progress]` | un objeto de despliegue | Emite objeto **también al fallar**, con `exit 1` |
| `orbit deploy --all --json [--progress]` | `{"schema":1,"apps":[…],…,"ok":bool}` | `result` anidado lleva su propio `"schema":1` |
| `orbit env list <app> --json` | `{"schema":1,"app":…,"keys":[…]}` | Sólo nombres. §13.2 |
| `orbit db list --json` | `{"schema":1,"databases":[{name,owner,size_bytes}]}` | Si PostgreSQL no contesta, **aborta** en vez de devolver `[]` (línea 7615) |
| `orbit redirect list [app] --json` | `{"schema":1,"redirects":[…]}` | |
| `orbit watch status --json` | `{"schema":1,"timer_active",…,"summary":{…}}` | |
| `orbit queue status --json` | `{"schema":1,"timer_active","every_minutes","max_seconds","apps":[…]}` | |

Los alias también valen: `ls`, `show`, `check`, `up`, `métricas`, `tráfico`, `redir`, `database`,
`cola`, `-v`, `--version`. Un cliente no debería usarlos —escribir siempre el nombre canónico
elimina una clase entera de sorpresas—, pero conviene saber que existen porque el usuario puede
tenerlos en su `~/.ssh/config` o en un alias de shell.

### 1.3 Los rechazos, y por qué hay DOS mensajes distintos

Éste es el hallazgo más operativo de la sección, porque cambia cómo se escribe el manejador de
errores del cliente.

**Caso A — `--json` delante del comando.** Lo consume el bucle global (13627), llega a la criba de
13655, `_json_capable` dice que no, y muere con el mensaje del contrato:

```
$ orbit --json logs
  ✗ 'orbit logs' no tiene salida JSON. La tienen: list, info, status, doctor, top,
    metrics, traffic, deploy, version, 'env list', 'db list', 'redirect list',
    'watch status' y 'queue status'.
rc=1
```

**Caso B — `--json` detrás del comando.** `$JSON` sigue valiendo `no`, así que **ninguna de las dos
ramas de 13655 se ejecuta** y `--json` viaja intacto hasta el `cmd_*`, que lo trata como una opción
suya y desconocida:

```
$ orbit logs --json
  ✗ Opción desconocida: --json
  uso: orbit logs [app] [--since 1h] [--lines 200] [--nginx] [-f]
rc=1
```

Los dos salen con 1 y por stderr, así que ningún cliente se traga texto por JSON. Pero el mensaje
es distinto, y hay un tercer comportamiento peor: **los comandos cuyo parser posicional no rechaza
las opciones desconocidas se comerían el `--json` como si fuera un argumento**. `cmd_traffic`
(6604) sí lo rechaza (`-*)` die), `cmd_logs` también (8869), `cmd_remove` también (11314). Pero
`cmd_migrate` (10847) hace `-*) die` — bien— y `cmd_rollback` (6699) **no filtra nada**: sus
argumentos son `name="${1:-}" target="${2:-}"`, así que `orbit rollback --json` intentaría cargar
una app llamada `--json`. Se muere igual, pero con el mensaje equivocado («La app '--json' no
existe»).

**Regla para el cliente:** `--json` va SIEMPRE delante del comando. `orbit --json list`, no
`orbit list --json`. Es una línea de código en el constructor de la orden y elimina de golpe las
tres variantes de error y el caso de `rollback`.

**Y los rechazos por subcomando.** Los cinco comandos con subcomandos hacen su propia criba
inmediatamente después de `_subcmd`:

| Línea | Comando | Guarda |
|---|---|---|
| 11198 | `env` | `[[ "$JSON" == "yes" && "$sub" != "list" ]] && die "--json sólo está en 'orbit env list <app>'"` |
| 7591 | `db` | `… "$sub" != "list" … die "--json sólo está en 'orbit db list'"` |
| 9729 | `redirect` | `… "$sub" != "list" …` |
| 10073 | `watch` | `… "$sub" != "status" …` |
| 10521 | `queue` | `… "$sub" != "status" …` |

Todos mueren con `exit 1`. Verificado: `orbit --json env get app CLAVE` →
`✗ --json sólo está en 'orbit env list <app>'`, rc=1. Es el comportamiento correcto y es lo que
protege el §13.2: no hay forma de sacar un valor del `.env` por el contrato.

### 1.4 Lista definitiva: dónde el cliente se queda sin datos

Ésta es la tabla que hay que tener delante al diseñar cada pantalla. La columna «qué hace el
cliente» es la decisión de diseño, no una constatación.

| Necesidad de la interfaz | ¿JSON? | Qué hace el cliente |
|---|---|---|
| Lista de apps + estado | ✅ `list --json` | Directo |
| Detalle de una app | ✅ `info <app> --json` | Directo |
| Salud del servidor | ✅ `status --json` | Directo (y trae `apps` de regalo: **una llamada, dos pantallas**) |
| Diagnóstico + botones de arreglo | ✅ `doctor --json` | Directo. El botón sólo si `fixable` |
| Aplicar los arreglos | ❌ **roto** | Ver §1.6b. Hay que ejecutar `orbit doctor --fix` con `stdin` conectado y responder `s\n`, o parchear el servidor |
| CPU/memoria en vivo | ✅ `top --json` | Directo, pero 1 s de coste por muestra |
| Historial de despliegues | ✅ `metrics --json` | Directo |
| Analítica de tráfico | ✅ `traffic --json` | Directo |
| Desplegar | ✅ `deploy <app> --json --progress` | Directo. Progreso por stderr |
| Desplegar todo | ✅ `deploy --all --json --progress` | Directo |
| Nombres de variables de entorno | ✅ `env list <app> --json` | Directo |
| **Valor** de una variable | ❌ por diseño | `env get <app> CLAVE` → una línea pelada por stdout. Es texto, y así debe ser |
| Bases de datos | ✅ `db list --json` | Directo |
| Redirecciones | ✅ `redirect list --json` | Directo |
| Vigilancia | ✅ `watch status --json` | Directo |
| Colas | ✅ `queue status --json` | Directo |
| **Logs** | ❌ **no existe** | Streaming de texto crudo. Ver §1.11 |
| **Copias de seguridad** | ❌ **no existe** | `backup list` → tabla `%-44s %8s  %s`. Ver abajo |
| **Verificar copias** | ❌ | `backup verify` → líneas `✓ fichero · mensaje`. rc≠0 si hay alguna rota |
| **Restaurar** | ❌ | `restore <fichero> -y` → prosa. rc |
| **Mantenimiento (estado)** | ❌ | `maintenance status` → tabla. **Pero** `list --json` ya trae `state.maintenance` por app: **no hace falta** |
| **Autodespliegue (estado)** | ❌ | `autodeploy status` → prosa. **Pero** `list --json` trae `state.autodeploy`: **no hace falta** |
| **Notificaciones** | ❌ | `notify status` → prosa |
| **Cortafuegos** | ❌ | `firewall status` → `ufw status verbose` indentado |
| **Certificado (emitir)** | ❌ | `ssl <app>` → prosa + rc. `list --json` ya dice `state.ssl` y `info` da `cert_days` |
| **Crear app** | ❌ | `new --yes …` → prosa + rc |
| **Rollback** | ❌ | `rollback <app> <release>` → prosa + rc. `info --json` da la lista de releases |
| **Eliminar app** | ❌ | `remove <app> -y [--purge]` → prosa + rc |
| **Arrancar/parar/reiniciar** | ❌ | `restart|start|stop <app>` → una línea + rc |
| **Ejecutar algo** | ❌ | `exec <app> <cmd>` → la salida del comando, tal cual |
| **Puerto** | ❌ | `port <app> [n]` → prosa + rc |
| **Clonar** | ❌ | `clone <app> [nuevo]` → prosa + rc |
| **Migraciones** | ❌ | `migrate <app> --yes` → el plan y el resultado en prosa |
| **Aislar** | ❌ | `isolate <app>` → prosa + rc |
| **Idioma del servidor** | ❌ | `lang [código]` → prosa |

**Los tres huecos que de verdad duelen**, en orden:

1. **`orbit logs` no tiene JSON, y además es infinito por defecto.** Es el hueco caro porque los
   logs son la segunda pantalla más visitada de cualquier gestor de despliegues. Detalle en §1.11.
2. **`orbit backup list` no tiene JSON.** Verificado (líneas 8033-8046): imprime
   `printf "  %-44s %8s  %s\n"` con `basename`, `du -h` y `date -r`. Los nombres de fichero son
   `<app>-<AAAAMMDD-HHMMSS>.tar.gz` o `_orbit-conf-<sello>.tar.gz`, así que **no contienen espacios**
   y el parseo por columnas fijas es estable mientras el `printf` no cambie. Pero es exactamente lo
   que §13.1 dice que no hay que hacer: «alinear una columna es un cambio incompatible». La
   decisión del cliente en §1.12.
3. **`doctor --fix` no se puede automatizar.** Ver §1.6b. Es un fallo del servidor, no del cliente.

**Los cuatro huecos que NO duelen**, y merece la pena decirlo para que nadie invente trabajo:
`maintenance status`, `autodeploy status`, el estado de SSL y el número de releases ya están en
`list --json` / `info --json`. Cuatro comandos de texto que el cliente no necesita ejecutar jamás.

### 1.5 Los campos, con sus `null` de verdad

Lo que sigue está sacado de ejecuciones, no del BRIEF. Las diferencias con el BRIEF están marcadas.

**`state`** (`_app_state_json`, 1526-1557). Se emite en `list --json`, en `status --json` y en
`info --json`. La única diferencia entre los tres: `cert_days` **sólo** se calcula en `info`
(argumento `--cert-days`), porque cuesta un `openssl` por app. En `list` y en `status` es siempre
`null`, y `null` aquí significa «no lo he mirado», no «no hay certificado» — para eso está `ssl`.
Es el `null` más peligroso del contrato y no está documentado como tal.

```
service          string|null   "running" | "stopped" | null
port             number|null
ssl              boolean
cert_days        number|null   ← null en list y status SIEMPRE. Sólo info lo calcula
maintenance      boolean
served           boolean
autodeploy       boolean
queue            boolean
releases         number
last_deploy      string|null
last_deploy_sha  string|null
```

`service` es `null` para `static`, `php`, `laravel` y `redirect` — verificado —, porque
`needs_svc()` (línea 1520ish) sólo devuelve cierto para `node|next|python|go|bun|deno`. Nótese que
**`laravel` y `php` no tienen servicio**: los sirve php-fpm. Un cliente que pinte «parada» en rojo
sobre un Laravel estaría inventando una alarma; es exactamente el caso que §13.1 usa para explicar
por qué `null` no es `"stopped"`.

`last_deploy` y `last_deploy_sha` salen de partir `A_LASTDEPLOY` por el **primer** y el **último**
espacio (`${A_LASTDEPLOY%% *}` y `${A_LASTDEPLOY##* }`). Si el campo no llevara espacio, los dos
devolverían la misma cadena. Hoy siempre lo lleva (`fecha-ISO SHA`), pero el cliente no debe
asumir que `last_deploy` parsea como fecha sin comprobarlo.

**`top`** (`_top_json`, 8719-8736). Aquí el BRIEF se queda corto:

```
cpu_percent           number|null
memory_bytes          number|null   ← el BRIEF lo da como number. Es null si la unidad no existe
requests_last_minute  number|null   ← idem. null = "no lo sé" (log sin marca de tiempo, o sin log)
requests_capped       boolean
port                  number|null
service               string|null
```

Verificado con apps paradas: `"memory_bytes":null,"requests_last_minute":null`. Un cliente que
tipara `number` y pintara `0 B` estaría afirmando que la app usa cero memoria, que es justo lo que
§13.6 prohíbe hacer con la CPU.

**`traffic`** (`_traffic_json`, 6531-6557). Cinco cosas que el BRIEF no dice:

```
since        string    la VENTANA pedida ("24h"), no un instante
from         string    "AAAAMMDDHHMMSS", sin separadores, sin huso. NO es ISO-8601
complete     boolean
requests     number     (nunca null: se fuerza a 0)
ips          number|null
bytes        number|null
automated    number|null
status       object     SPARSE: sólo las clases con tráfico. Puede ser {}
latency_ms   {p50:number|null, p95:number|null, max:number|null, lines:number}
```

Salida real de una app sin tráfico:
`"ips":null,"bytes":null,"automated":null,"status":{},"latency_ms":{"p50":null,…,"lines":0}`.

`status` es un objeto disperso con claves `"1xx"`…`"5xx"` y sólo aparecen las que tienen cuenta
(bucle de 6543-6548, `[[ -n "$n" ]] || continue`). Un `Record<string, number>` en TypeScript, no un
tipo con cinco campos obligatorios.

Y lo más importante: **`traffic --json` SIEMPRE devuelve `{"schema":1,"apps":[…]}`**, aunque se pida
una sola app. El BRIEF lo describe como un objeto plano `{"app":…,"since":…}`. Lo mismo con
`metrics --json`, que además añade `"kept":2000`. Verificado los dos.

**`deploy`** (`_deploy_json`, 4915-4934). El objeto llega también cuando el despliegue muere,
gracias al `trap '_deploy_on_exit' EXIT` de la línea 5225. Verificado con un repo inexistente:

```json
{"schema":1,"app":"alpha","ok":false,"release":"20260830-035038","previous":null,
 "commit":{"sha":null,"subject":null,"ref":null},"rolled_back":false,"recovered":false,
 "duration_s":0,"failed_step":"code","error":"no pude clonar el repositorio"}
```

con `rc=1`. Y el caso raro: `orbit deploy --json` **sin nombre de app** emite el objeto igualmente,
con `"app":""` y `"error":"con --json hay que decir qué app desplegar"` (líneas 5233-5235). O sea
que **la ausencia de nombre no es un error de sintaxis del cliente: es una respuesta del contrato**.
Bien pensado, y hay que tratarlo como tal.

Los seis valores posibles de `failed_step` son exactamente los seis `_dstep` del script (5271,
5320, 5444, 5510, 5535, 5606): `code`, `release`, `build`, `activate`, `service`, `nginx`.
`error` es una de 22 cadenas fijas (`grep -c 'DEP_ERR='` → 22), **traducidas**: con
`orbit --lang en` salen en inglés. Para automatizar están `ok` y `failed_step`, como dice USAGE.

**`--progress`**. Hay dos clases de suceso y van los dos por **stderr**, mezclados con la prosa
para humanos. Verificado literalmente:

```
                                          ← línea en blanco (title)
Desplegando alpha · alpha.example.com     ← prosa
──────────────────────────────────────    ← hr
{"event":"step","app":"alpha","step":"code","status":"start","elapsed_s":0}
  ✗ Clonando https://github.com/x/alpha.git
      Cloning into '/tmp/orbtest/apps/alpha/cache'...
```

Es decir: **el NDJSON no viene solo por stderr; viene intercalado con texto ANSI**. El cliente tiene
que filtrar línea a línea y aceptar sólo las que empiezan por `{` y parsean. USAGE.md muestra los
sucesos **sin** el campo `"app"`; el código lo añadió al escribir `deploy --all --json` (comentario
de la línea 4904). El campo `app` está siempre, incluso en un `deploy` de una sola.

`_dprog` sólo emite `status: "start"` y `status: "ok"` (4913, y los seis `_dprog … ok`). **No hay
`status: "error"`.** Un paso que falla se reconoce porque tiene `start` y nunca llega su `ok`, y
porque el objeto final trae `failed_step`. Diseño consciente y correcto, pero hay que saberlo: una
barra de progreso que espere un evento de fallo se queda colgada para siempre.

`_dall_prog` (5879) emite `{"event":"app","app":…,"status":…,"elapsed_s":…}` con los siete valores
`start | deployed | failed | unchanged | unreachable | gone | skipped`.

**`doctor`** (`_doctor_json`, 11995-12015). Un detalle de tipado que muerde:

```bash
case "$lvl" in ok|info) n_ok=$((n_ok + 1)) ;; warn) … ;; error) … ;; esac
```

`summary.ok` cuenta los `ok` **y los `info`**. Así que `summary.ok + summary.warn + summary.error`
== `checks.length`, pero `checks.filter(c => c.level === 'ok').length` **≠** `summary.ok`. Si el
cliente pinta «3 correctos» a partir del resumen y luego pinta la lista, los números no cuadran.
Hay que decidir uno de los dos y ser consistente.

`fixable` es `true` sólo si hay `DOC_ACT` **y** el nivel es `warn` o `error` (12008). Los `id` son
únicos y namespaced: verificado con cuatro apps salen `nginx`, `postgresql`, `pnpm`, `github`,
`cloudflare-token`, `default-server`, `disk`, `ports`, `service-<app>`, `vhost-<app>`,
`dns:<dominio>`. Se puede usar `id` como clave de lista sin miedo.

### 1.6 Divergencias entre lo documentado y lo que hace el código

Cuatro. Las cuatro verificadas ejecutando.

**a) `info --json` devuelve 37 campos de `config`, no 31.** `ORBIT_APP_FIELDS` (1308-1315) tiene 37
entradas. Contadas sobre la salida real: 37. Los que faltan en la documentación son los que se
añadieron después de escribirla (`pnpm_allow`, `node_heap`, `shared`, `env_file`, `env_spec`, y
`maint_auto`). No rompe nada —los campos se añaden, que es la promesa— pero un tipo TypeScript
escrito a partir del BRIEF se quedaría corto. La lista completa está en §4.

**b) `orbit doctor --fix --json --yes` NO EXISTE. Es un error.** Éste es el hallazgo grave.

`USAGE.md:1674` documenta literalmente `orbit doctor --fix --json --yes`, y `ARCHITECTURE §19.5`
(línea 2459) explica por qué `--yes` es obligatorio ahí. Pero:

- `--yes` **no es una bandera global**. El bucle de banderas de `main()` (13627-13637) reconoce
  `--json`, `--eva`/`--jedimaster`/`--jedi` y `--lang`. Nada más.
- `ASSUME_YES` sólo se pone a `"yes"` en **un** sitio del script entero: la línea 6915, dentro de
  `cmd_new`, sobre una **copia local** (`local ASSUME_YES="$ASSUME_YES"`, línea 6889).
- `cmd_doctor` (12084-12089) rechaza cualquier argumento que no sea `--fix`:

```bash
  for a in "$@"; do
    case "$a" in
      --fix) fix="yes" ;;
      *) die "orbit doctor: no sé qué es «%s». Sólo acepta --fix y --json." "$a" ;;
    esac
  done
```

Ejecutado:

```
$ orbit doctor --fix --json --yes
  ✗ orbit doctor: no sé qué es «--yes». Sólo acepta --fix y --json.
rc=1
```

Y sin `--yes` no sirve, porque la guarda de 12118-12121 exige `ASSUME_YES=yes`:

```
'doctor --fix --json' necesita también --yes: sin terminal no puedo preguntar.
```

O sea: **`doctor --fix --json` es un camino muerto**. Las dos ramas cierran la puerta y no hay
llave. Está documentado en dos ficheros y no funciona en ninguna versión que tenga esta forma de
`cmd_doctor`. No lo cazó ninguna prueba porque `tests/doctorfix_test.sh:362` llama a
`cmd_doctor --fix` **como función**, saltándose `main()`, que es donde vive `ASSUME_YES`; y
`tests/lib.sh` carga el script con `sed '$ d'`, o sea deliberadamente sin `main`. Es el mismo
patrón que el propio ARCHITECTURE §13.6c describe: «una prueba tiene que ejercer el camino».

**Consecuencia para Orbit Desktop:** el botón «arreglar» de la pantalla de diagnóstico **no se puede
implementar contra un servidor 1.3.6 sin trucos**. Las opciones, en orden de preferencia:

1. **Abrir un issue y un PR contra `orbit`.** El arreglo es de tres líneas: reconocer
   `-y|--yes|--si) ASSUME_YES="yes"` en el bucle de `main()`, o —mejor, porque no toca el ámbito
   global— añadirlo al `case` de `cmd_doctor`. Es el camino correcto y es barato.
2. **Mientras tanto, ejecutar `orbit doctor --fix` sin `--json`, con stdin conectado, y escribir
   `s\n`** (la pregunta es `confirm "¿Los aplico?" y`, línea 12148). Funciona, pero devuelve prosa
   y hay que parsearla o simplemente ignorarla y refrescar `doctor --json` después. **Es lo que hará
   la fase 3 del cliente**, y con una nota en la interfaz: «este servidor necesita Orbit ≥ X para el
   arreglo automático limpio».
3. **No ofrecer el botón.** Enseñar `fix` como texto copiable. Es lo que hará la fase 1.

**c) `metrics` y `traffic` no tienen la forma que dice el BRIEF.** Los dos envuelven siempre en
`{"schema":1,"apps":[…]}`. `metrics` añade `"kept"`. Ya tratado en §1.5.

**d) La lista de comandos con `--json` de `USAGE.md:1541` está incompleta.** No menciona `metrics`,
`traffic`, `deploy` ni `queue status`. La lista buena es la de `_json_cmds_help()` (1105), que es la
que sale en el mensaje de error y que un comentario del propio script (1099-1104) advierte que ya
se quedó corta una vez: «durante un tiempo este mensaje nombraba cuatro de los nueve, así que quien
probaba `orbit db list --json` y leía la respuesta se lo creía». **El cliente debe tomar
`_json_capable` como fuente de verdad, no la documentación**, y por eso §9 propone una prueba que
extrae esa lista del script con `grep` y la compara con la del cliente.

### 1.7 El árbol de despacho completo

`main()` (13609) hace exactamente cinco cosas en este orden, y el orden importa:

1. **Bucle de banderas globales** (13627-13637). Sólo `--json`, `--eva|--jedimaster|--jedi`,
   `--lang|--idioma` y `--lang=|--idioma=`. Al primer argumento que no encaje, `break`.
2. **`cmd="${1:-menu}"; shift`** (13638). Sin argumentos, el comando es `menu`.
3. **`_lang_strip` sobre el resto** (13641-13644), **salvo en `exec|run`**, donde los argumentos son
   la orden de otro y un `--lang` es suyo.
4. **Resolución de idioma y carga del catálogo** (13649-13653), y aquí se validan `--lang` vacío y
   `--lang` desconocido, los dos con `die` → 1.
5. **Criba de `--json`** (13655-13659) y **`_ui_route`** (13663).

Y el `case` de despacho, 13664-13717, íntegro y con sus alias:

```
menu | ""                    → menu
new | create                 → cmd_new
deploy | up                  → cmd_deploy   (o cmd_deploy_all si $1 es --all|--todas)
autodeploy                   → cmd_autodeploy
queue | cola | colas         → cmd_queue
maintenance | mant           → cmd_maintenance
clone | copy                 → cmd_clone
rollback                     → cmd_rollback
list | ls                    → cmd_list
info | show                  → cmd_info
top                          → cmd_top
metrics | metricas | métricas→ cmd_metrics
traffic | trafico | tráfico  → cmd_traffic
logs | log                   → cmd_logs
restart                      → cmd_service restart
start                        → cmd_service start
stop                         → cmd_service stop
env                          → cmd_env
exec | run                   → cmd_exec
migrate                      → cmd_migrate
redirect | redir             → cmd_redirect
watch                        → cmd_watch
notify                       → cmd_notify
port                         → cmd_port
isolate | aislar             → cmd_isolate
init                         → cmd_init
domain | domains             → cmd_domain
remove | rm | del            → cmd_remove
ssl | cert                   → cmd_ssl
db | database                → cmd_db
backup | copia               → cmd_backup
restore                      → cmd_restore
lang | idioma                → cmd_lang
github | gh                  → cmd_github
cf-token                     → cmd_cf_token
cf-update                    → cmd_cf_update
firewall | fw                → cmd_firewall
nginx-rebuild                → cmd_self_update
status                       → cmd_status
doctor | check               → cmd_doctor
version | -v | --version     → cmd_version
help | -h | --help           → usage
*                            → err "Comando desconocido"; usage; exit 1
```

**39 comandos, 55 nombres contando alias.** Detalle contraintuitivo:
`nginx-rebuild` despacha a `cmd_self_update`, un nombre heredado.

**Los subcomandos**, que van por `_subcmd()` (1261-1301). Su firma es
`_subcmd <comando> <defecto> <app|noapp> <spec> [args…]` y el `spec` lista grupos separados por
espacios, con alias dentro del grupo separados por `|`. Los que hay:

| Comando | Defecto | Modo | Subcomandos |
|---|---|---|---|
| `env` | `edit` | app | `get set unset list edit` |
| `db` | `help` | noapp | `create list shell backup backup-all help` |
| `backup` | `create` | app | `create all|--all|--todas list|ls verify|check help` |
| `redirect` | `list` | app | `add rm|del|remove list|ls help` |
| `watch` | `once` | noapp | `enable disable status history|--history once|--once quiet|--quiet|-q` |
| `queue` | `status` | app | `enable|on disable|off every|cada run|once status` |
| `maintenance` | `status` | app | `on|activar off|quitar edit|page status` |
| `firewall` | `status` | noapp | `lock unlock status` |
| `autodeploy` | (ver 10199) | app | `enable/disable/status/every` |
| `notify` | (ver 9428) | noapp | `status setup test` |

Dos cosas de `_subcmd` que un cliente tiene que respetar:

- **El subcomando gana sobre el nombre de app.** Si alguien tiene una app llamada `status`,
  `orbit maintenance status` es el subcomando, no la app. El propio `usage` lo advierte. El cliente
  siempre escribe el subcomando explícito, así que no le afecta — pero si el usuario tiene una app
  llamada `list` y el cliente hace `orbit backup list`, obtendrá la lista, no la copia. Es correcto
  y es lo que quiere, pero conviene saber por qué.
- **`_subcmd` sale con `exit 0` en la ayuda (1293) y con `exit 1` en el subcomando desconocido
  (1300).** Salen del proceso entero, no de la función.

### 1.8 Códigos de salida: sólo hay dos, y un puñado de rarezas

El resultado de la auditoría es más simple de lo que parecía y a la vez menos útil de lo que un
cliente querría.

**`die()` es la única salida de error de todo el script** (línea 472):

```bash
err()  { _t "$@"; printf "  ${RED}%s${R} %s\n" "$G_ERR" "$I18N_MSG" >&2; }
die()  { err "$@"; exit 1; }
```

`exit 1`, siempre. No hay `exit 2` para «argumento malo», ni `exit 3` para «app no existe», ni
`exit 4` para «no soy root». **Todo error del contrato es 1.** Un cliente no puede distinguir «la
app no existe» de «nginx rechazó la configuración» por el código de salida: tiene que leer stderr.

El barrido de `grep -n "exit [0-9]"` da 17 apariciones y sólo estas seis no son `exit 1`:

| Línea | Código | Cuándo |
|---|---|---|
| 1291 | `exit 0` | `_subcmd`, al pedir ayuda de un subcomando |
| 8770 | `exit 0` | trap de `INT TERM` en `orbit top` en vivo |
| 9861-9862 | `exit 0` | `watch quiet`: no pudo tomar el `flock` → se va callado (lo llama un timer) |
| 10118 | `exit 0` | idem, otra ruta de watch |
| 10468 | `exit 96` | `queue`: no pudo abrir el fichero de bloqueo |
| 10469 | `exit 97` | `queue`: otra pasada de la cola sigue en marcha |
| 12409 | `exit 0` | opción `0`/`q`/`salir` del menú |

**96 y 97 son los dos únicos códigos con significado propio del script entero**, y sólo aparecen
en el camino del temporizador de colas, que el cliente no invoca. Anecdótico, pero hay que dejarlo
escrito para que nadie los descubra en producción.

Además, hay tres códigos que no vienen de `orbit` sino de debajo y que sí pueden llegar al cliente:

- **`130`** — SIGINT. El comentario de la línea 4095 lo menciona para una app que se muere en seco.
  Si el cliente cancela un comando mandando `SIGINT` por el canal SSH, esto es lo que verá.
- **`127`** — comando no encontrado. Es lo que sale si `orbit` **no está instalado** en el servidor:
  la shell remota contesta `bash: orbit: command not found` por stderr y 127. Es la señal que el
  cliente usa para el caso «servidor sin Orbit» (§4.5).
- **`255`** — el propio `ssh` cuando falla la conexión. Nunca lo produce `orbit`. Es la frontera
  entre «no pude llegar» y «llegué y me contestó», y **es la razón por la que el cliente nunca debe
  confundir el código de salida de `ssh` con el de `orbit`** sin más contexto. Detalle en §3.4.

**Los códigos de éxito con matiz**, que sí son útiles:

| Comando | rc | Significado |
|---|---|---|
| `doctor` / `doctor --json` | **0 siempre**, aunque haya errores | Deliberado (12102): hay scripts que lo encadenan con `&&`. Decidir con `.summary.error` |
| `deploy <app> --json` | 0 / 1 | 1 == `ok:false`. Verificado |
| `deploy --all --json` | 0 / 1 | 1 si `fail_n>0 || mute_n>0`. `mute_n` incluye `unreachable` **y** `gone` (5964, 5971). Coincide exactamente con `"ok"` del objeto, que es `f==0 && r==0 && g==0` (5911). Verificado: los dos discrepan **nunca** |
| `backup verify` | 0 / 1 | 1 si alguna copia está rota (8027) |
| `top --json` | 0 | |
| `ssl <app>` | 0 / 1 | 1 si certbot falló (7494) |

**Regla para el cliente:** el código de salida es un booleano. Toda la información está en el JSON
de stdout (cuando lo hay) y en stderr (siempre). Diseñar el `OrbitError` a partir de eso, no del
código. Detalle en §4.6.

### 1.9 Auto-elevación a root: la trampa que puede matar el proyecto en la demo

Está en las líneas 240-262, **antes de que exista `main`**, y es lo primero que hace el script
después de resolver el idioma:

```bash
if [[ $EUID -ne 0 ]]; then
  if [[ "$ORBIT_CMD0" != "init" ]]; then
  command -v sudo >/dev/null || { echo "$(t "Orbit necesita privilegios de root.")"; exit 1; }
  [[ -n "$ORBIT_LANG_ENV" ]] && _lang_supported "$(_lang_norm "$ORBIT_LANG_ENV")" \
    && exec sudo -- "$0" --lang "$ORBIT_LANG_ENV" "$@"
  exec sudo -- "$0" "$@"
  fi
fi
```

Tres consecuencias, y las tres importan:

**Uno: `need_root` no es lo que exige root.** Aparece 28 veces (5188, 6599, 6699, 6882, 7326, 7451,
7504, 7533, 7588, 7991, 8298, 8930, 8992, 9429, 9727, 10070, 10200, 10516, 10748, 10843, 10886,
11193, 11287, 11303, 11378, 12106, 12170, 12196, 12228) y está definida en 854:

```bash
need_root() { [[ $EUID -eq 0 ]] || die "Este comando necesita root. Usa:  sudo orbit %s" "$*"; }
```

Pero como la auto-elevación de la línea 241 ya se ha ejecutado, **`need_root` sólo puede fallar si
la elevación falló y aun así el script siguió** — que no puede pasar, porque `exec` sustituye el
proceso. Es un cinturón sobre unos tirantes, y está bien que esté. Lo que importa es lo otro: la
lista de `need_root` **no** es la lista de comandos que necesitan root. **Todo `orbit` necesita
root salvo `orbit init`.** Verificado: `orbit list --json` con la elevación neutralizada funciona,
pero `orbit redirect list --json` muere con `need_root`; en un servidor de verdad los dos se elevan
antes de llegar ahí.

Los que **no** tienen `need_root` explícito pero sí se elevan: `list`, `info`, `status`, `top`,
`metrics`, `logs`, `version`, `lang`, `menu`, `help`. Los que **sí** lo tienen: los 28 de arriba.
El único que **no se eleva**: `init` (245), porque escribe un `orbit.json` dentro del repositorio
de quien lo ejecuta y dejarlo de root sería un incordio para siempre.

**Dos: `orbit` exige que Orbit esté instalado, incluso para `version`.** Líneas 265-273:

```bash
if [[ -r "$CONF_FILE" ]]; then
  . "$CONF_FILE"
elif [[ "$ORBIT_CMD0" != "init" ]]; then
  echo "$(t "Orbit no está instalado todavía (falta %s)." "$CONF_FILE")"
  echo "$(t "Ejecuta primero:  sudo bash install.sh")"
  exit 1
fi
```

Y ojo: esos dos `echo` van por **stdout**, no por stderr, y no respetan `--json`. Así que
`ssh host 'orbit --json version'` contra un servidor con el binario copiado pero sin instalar
devuelve **dos líneas de texto en español por stdout** y `rc=1`. Un cliente que haga
`JSON.parse(stdout)` sin mirar el rc explota con un error incomprensible. Hay que mirar el rc
primero. Siempre.

**Tres, y es el grande: `exec sudo` sin TTY.** Éste es el modo de fallo que hundiría una demo.

`ssh usuario@host orbit list --json` abre una sesión **sin TTY** (no se pidió `-t`). Si `sudo` en
ese servidor pide contraseña, `sudo` no tiene dónde pedirla:

```
sudo: a terminal is required to read the password; either use the -S option
      to read from standard input or configure an askpass helper
```

por stderr, y `rc=1`. El comando no se ejecuta. Y si el sudoers tiene `Defaults requiretty` —raro
en Ubuntu 24.04, común en imágenes endurecidas—, falla incluso con NOPASSWD.

Las combinaciones, con lo que hace el cliente en cada una:

| Usuario SSH | sudoers | Qué pasa | Qué hace Orbit Desktop |
|---|---|---|---|
| `root` | — | Funciona. `$EUID -eq 0`, no se eleva | Camino feliz. Es el que hay que recomendar |
| usuario con `NOPASSWD: ALL` | | Funciona sin TTY | Camino feliz |
| usuario con `NOPASSWD: /usr/local/bin/orbit` | | Funciona. Es el grupo `orbit-admin` del ROADMAP | Camino feliz, y el que hay que documentar |
| usuario con sudo **con** contraseña | | **Falla** sin TTY | Detectar el mensaje y ofrecer: (a) pedir `-t` y una contraseña en un diálogo, (b) explicar cómo poner NOPASSWD |
| usuario sin sudo | | `Orbit necesita privilegios de root.` + rc 1 | Mensaje claro en la interfaz |

**Decisión de diseño para el cliente:** el asistente de alta de servidor ejecuta, como primer paso,
`orbit --json version` y clasifica el resultado en cinco casos: JSON válido (ok), 127
(no instalado), texto «Orbit no está instalado todavía» (instalado a medias), mensaje de sudo (falta
privilegio), 255 (no llegué). Sin esa clasificación, el usuario ve «error» y no sabe si es su clave,
su red o su servidor. Es la diferencia entre un producto y una demo.

**Sobre `orbit-admin` (💭 en el ROADMAP).** El propio ROADMAP dice la verdad incómoda: un sudoers
limitado a `/usr/local/bin/orbit` **sigue siendo equivalente a root**, porque `orbit exec <app>`
ejecuta cualquier cosa y `orbit exec` sin comando abre una shell (línea 9006:
`if (( $# == 0 )); then argv=(bash)`). Orbit Desktop no debe presentarlo como un modo «restringido»;
debe presentarlo como «no tienes que teclear la contraseña de root», que es lo que de verdad
resuelve. Prometer aislamiento donde no lo hay es el pecado que §13.3 le reprocha a los paneles web.

### 1.10 Detección de terminal: qué cambia sin TTY, y qué se rompe

Hay **cuatro** comprobaciones distintas y no significan lo mismo. Confundirlas es lo que causó uno
de los tres fallos que cuenta §13.6c.

| Línea | Comprobación | Qué decide |
|---|---|---|
| 346 | `UI_TTY="no"; [[ -t 1 ]] && UI_TTY="yes"` | Color y ancho. Se evalúa **al cargar el script**, antes de saber si hay `--json` |
| 453 | `_ui_tty() { [[ -t "$UI_FD" ]]; }` | Si se dibuja. Mira el descriptor **por donde se escribe**, que con `--json` es el 2 |
| 659 | `command -v fzf && [[ -t 0 ]]` | Si `choose` usa `fzf` o lista numerada |
| 8753 | `[[ -t 1 && -t 0 ]] \|\| once="yes"` | Si `orbit top` entra en bucle o da una foto |

Lo que cambia sin TTY, verificado:

- **Sin color** (347): `B`, `D`, `R`, `RED`… quedan vacíos. Perfecto para el cliente.
- **Ancho fijo a 66 columnas** (417-425): no se llama a `tput`. La salida redirigida es
  byte-a-byte estable entre ejecuciones, que es lo que permite hacer *snapshot tests* del texto.
- **Sin animaciones** (481, `_can_anim`): requiere `UI_TTY=yes` **y** color **y** `UI_ANIM=yes`.
  El spinner (837-850) sigue escribiendo su línea final por `$UI_FD`, pero sin `\r` ni fotogramas.
- **`orbit top` da una foto y sale** (8753). Con `--json` siempre es una foto, con o sin TTY (8756).
- **`choose` cae a la lista numerada** (663-670).

Y ahora lo que **se rompe**, que es lo que ninguna de las tres documentaciones dice con esta
claridad:

**`ask` y `confirm` no abortan sin terminal: toman el valor por defecto.**

`ask` (585-608) hace `read -r __ans || true` y `[[ -z "$__ans" ]] && __ans="$__def"`. Con stdin
cerrado, `read` falla, `__ans` queda vacío y se toma el defecto. `confirm` (610-651) lo hace
explícito y hasta lo imprime:

```bash
elif ! read -r a && [[ -z "$a" ]]; then
  a="$def"; printf '%s %s\n' "$a" "$(t "(sin nadie a quien preguntar)")"
fi
```

El comentario de las líneas 626-640 documenta el razonamiento y es correcto —una tubería es una
forma legítima de contestar—, pero la consecuencia para un cliente es brutal, y se ve mejor
ejecutándolo. `orbit info` sin nombre de app y sin TTY:

```
$ orbit info < /dev/null
    1) alpha
    2) beta
  App (número) [1]:
alpha
──────────────────────────────────────────────
  Repositorio   https://github.com/x/alpha.git (main)
  …
rc=0
```

**Ha elegido la primera app de la lista y ha seguido, con código 0.** No ha preguntado, no ha
avisado, y ha hecho algo. En `info` es inofensivo. En estos NO lo es, y todos usan `pick_app`
(676-687) sin guarda de TTY:

| Comando | Línea del `pick_app` | Qué haría sobre la app equivocada |
|---|---|---|
| `restart`/`start`/`stop` | 8933 | Reiniciar o **parar** una web que estaba bien |
| `deploy` (sin `--json`) | 5241 | Desplegar la app que no era |
| `env` (subcomando `edit`, el defecto) | 11203 | Abrir `$EDITOR` sobre el `.env` de otra app — y **colgarse** |
| `maintenance on/off` | 10756, 10778 | Poner en 503 una web sana |
| `port` | 10888 | Cambiar el puerto de otra app y reiniciarla |
| `domain` | 11289 | Y luego `ask A_DOMAIN`, que con stdin cerrado deja el dominio **vacío** |
| `isolate` | 11390 | Migrar el usuario de sistema de otra app |
| `migrate` | 10852 | Ejecutar migraciones sobre otra base de datos |
| `ssl` | 7453 | Pedir un certificado a Let's Encrypt para el dominio equivocado, con sus límites de frecuencia |
| `clone` | 7330ish | |
| `remove` (sin `-y`) | 11321 | Aquí sí hay red: pide teclear el nombre (`ask typed`), que sin stdin sale vacío y **no coincide** → «Cancelado». Único destructivo protegido por accidente |

**Los tres comandos que SÍ se protegen a propósito**, y merecen aplauso:

- `rollback` (6714): `[[ -t 0 ]] || die "uso: orbit rollback %s <release> · están: …"`. El
  razonamiento está en §13.5 y es impecable: la primera release de la lista es la que ya está
  activa, así que el «valor por defecto sensato» habría sido reiniciar el servicio para dejarlo
  todo igual.
- `deploy --json` (5233): `[[ -n "$name" ]] || die … "no puedo preguntar"`.
- `info --json` (8579): `die "uso: orbit info <app> --json"`.

**Regla dura para Orbit Desktop, que va escrita antes que el código.** Confirmada de forma
independiente en la ronda 2 (E5 de `EVIDENCIA.md`, sobre el banco de 40 apps: `orbit info` sin app
y sin TTY elige `app01`, lo imprime y sale con 0):

> **Toda invocación lleva el nombre de la app explícito y todas las banderas que evitan una
> pregunta. Ningún comando se ejecuta jamás sin su objeto.**

Y no como convención sino **como tipo**: en `OrbitCommand` (§7.2), ninguna variante que actúe sobre
una app tiene el nombre opcional. Es `Info { app: AppName }`, nunca `Info { app: Option<AppName> }`.
**La orden peligrosa no se puede expresar**, así que no hay revisión de código que pueda dejarla
pasar. Las tres únicas variantes con `Option<AppName>` son `Metrics`, `Traffic` y `RedirectList`,
los tres comandos donde «sin app» significa «todas» y el contrato lo dice.

Y una segunda, que es su corolario:

> **El canal de stdin del comando remoto se cierra siempre** (`ssh … < /dev/null`), salvo en las
> dos excepciones controladas: `exec` interactivo y el `doctor --fix` de la §1.6b. Cerrarlo no
> impide el desastre —lo acabamos de ver— pero impide el otro: que un comando se quede colgado
> esperando `nano`.

**Y el caso más raro de todos:** `orbit` **sin argumentos** y sin TTY pinta el menú entero,
incluyendo `\e[H\e[2J` (borrado de pantalla), lee «Opción» con defecto `"0"` y sale con 0 (12406,
12409). Inofensivo, pero si algún día alguien escribe `ssh host orbit` en el cliente por error,
recibirá 30 líneas de ANSI y un código 0. Otra razón para que el constructor de órdenes no permita
comandos vacíos.

### 1.11 `orbit logs`: streaming por defecto, y ninguna forma de decirle que pare

`cmd_logs` (8854-8927). El parser acepta `--since|--desde`, `--lines|-n`, `--follow|-f`,
`--no-follow`, `--nginx|--web` y un posicional (la app). Rechaza cualquier otra opción con `die`
(8869), incluido `--json`.

**La decisión que lo define** está en 8884-8886:

```bash
  if [[ -z "$follow" ]]; then
    if [[ -n "$since" ]]; then follow="no"; else follow="yes"; fi
  fi
```

**Sin `--since` y sin `--no-follow`, sigue en vivo. Para siempre.** Verificado: `orbit logs alpha`
con stdin cerrado y sin TTY se quedó corriendo hasta que lo mató un `timeout`.

Después bifurca en dos fuentes completamente distintas:

**a) Apps con proceso** (`needs_svc` cierto: node, next, python, go, bun, deno) y sin `--nginx`
(8891-8901): `journalctl -u orbit-<app> -n <lines> [--since 'AAAA-MM-DD HH:MM:SS'] [-f]`. La
salida es la de `journalctl`, cruda, sin tocar. Con `-f`, infinita.

**b) Todo lo demás** (8903-8925): `tail -n <lines> [-f] <access.log> <error.log>` sobre
`/var/log/nginx/<app>.access.log` y `.error.log`, o —si hay `--since`— un `awk` de filtrado por
fecha (`_LOG_SINCE_AWK`, 8811-8829) tubería `tail -n <lines>`, que **termina**.

Detalles que un cliente tiene que absorber:

- Con `-f` **imprime una línea de cortesía por `$UI_FD`** antes de arrancar:
  `  Ctrl-C para salir` (8894, 8919). Traducida. Hay que descartarla, y no basta con «descarta la
  primera línea»: en las apps sin proceso hay además un `info` sobre qué logs se están viendo
  (8907). El filtro correcto es descartar las líneas que empiezan por dos espacios y un glifo de
  la tabla de `orbit` (`·`, `!`, `✓`, `✗` o sus fallbacks ASCII), no contar líneas.
- **Con `--since`, si el log de acceso no lleva marca de tiempo**, emite un `warn` (8912) y sigue.
  El cliente puede detectarlo y ofrecer el botón de `orbit nginx-rebuild`.
- **Una app recién creada no tiene ficheros de log** y entonces `cmd_logs` dice
  `«x» todavía no tiene logs de nginx` y devuelve 0 (8909-8913). No es un error.
- El error de opción desconocida es un `die` con `\n` embebido en el mensaje (8869), o sea que
  ocupa **dos líneas** en stderr. Otra razón para no parsear stderr línea a línea buscando un
  patrón exacto.

**Cómo lo resuelve Orbit Desktop.** Dos modos, y son dos transportes distintos:

1. **Ventana** (el 90 % de los usos): `orbit logs <app> --since 30m --lines 500 --no-follow`.
   Termina, devuelve un bloque, el cliente lo trocea por `\n` y lo pinta. El `--no-follow` es
   redundante con `--since` pero se pone igual: hace la intención explícita y sobrevive a un
   cambio de la lógica de 8884.
2. **Vivo**: `orbit logs <app> --follow --lines 200` sobre un canal SSH dedicado que el cliente
   **cierra explícitamente** al salir de la pantalla. No hay forma de que `orbit` pare solo, así
   que parar es cerrar el canal. Detalle en §3.5.

**Nunca** se pide `orbit logs` sin app: elegiría la primera de la lista y seguiría eternamente,
verificado en §1.10.

### 1.12 `orbit exec`, y por qué el cliente lo trata como territorio ajeno

`cmd_exec` (8991-9028). Lo que hace:

- Exige app (o abre `pick_app` — misma trampa de §1.10).
- Exige release activa: `[[ -d "$dir" ]] || die "'%s' no tiene ninguna release activa"` (8998).
- Decide la forma del comando (9004-9013): sin argumentos, `bash` interactivo; **un** argumento con
  metacaracteres (`[[:space:]|&;<>()$\`]`), `bash -lc "$1"`; en cualquier otro caso, los argumentos
  tal cual.
- Avisa por **stderr** si el comando contiene `install` y el tipo no es php/python (9018-9022).
- Ejecuta vía `_run_in_dir` (8977) → `sudo -u <usuario de la app> -H bash -lc "$_exec_script"`,
  que reproduce el entorno de la unidad systemd: `.env` primero, luego `NODE_ENV=production`,
  `HOST=127.0.0.1`, `PORT`, `COREPACK_HOME`, y un `PATH` con `node_modules/.bin`, `.venv/bin`,
  `bin/` y `/usr/local/go/bin`.

**La salida es la del comando, sin tocar.** Es lo que dice `USAGE.md §"La salida es la del
comando"` y es exacto: `orbit` no envuelve, no prefija y no formatea. El código de salida también
es el del comando (no hay `|| die`), porque `_run_in_dir` es la última orden de la función.

Y una cosa que sí es de `orbit` y no del comando: **`--json` delante de `exec` se rechaza**
(verificado: `orbit --json exec web ls` → `'orbit exec' no tiene salida JSON`). Detrás, viaja al
comando: `main` no toca los argumentos de `exec|run` ni para `--json` (13655, porque
`_json_capable exec` es falso y `$JSON` es `no`) ni para `--lang` (13641-13644, `exec|run) : ;;`).
Bien pensado: `orbit exec app mi-script --json` le pasa `--json` a `mi-script`, que es lo que
quiere el usuario.

**Decisión para Orbit Desktop:** `exec` es la puerta trasera y hay que tratarla como tal. No se usa
para nada de la interfaz —ni para leer un fichero, ni para contar releases, ni para «rellenar» un
hueco del contrato—, porque el día que se use, el cliente deja de hablar el contrato y pasa a hablar
Bash contra un servidor cuyo layout puede cambiar. Es la regla dura nº 1 del BRIEF aplicada al
cliente: *la interfaz sólo invoca `orbit`, y sólo por su contrato*. `exec` se ofrece al usuario
como una terminal embebida, con su aviso de que eso es una shell del usuario de la app, y nada más.

Con una excepción que sí es legítima y que hay que escribir para que no se discuta después:
**cuando un hueco del contrato no tiene alternativa, se llama al comando de `orbit` que sí existe y
se parsea su texto, nunca se sortea `orbit` con `exec`.** `orbit backup list` produce una tabla; se
parsea la tabla. Lo que no se hace es `orbit exec … ls /var/backups/orbit`, que además fallaría
porque `exec` corre como el usuario de una app y no puede leer un directorio 0700 de root.

### 1.13 Idioma: cómo se fuerza inglés o español, y qué no cambia nunca

El núcleo está en las líneas 40-116 y se comparte con `install.sh` entre dos marcas
(`>>> núcleo de idiomas`).

`ORBIT_LANGS="es en"` (42). Sólo dos. `_lang_norm` (51) normaliza `es_ES.UTF-8`, `es-ES`,
`es_AR@valencia` y `ES` a `es`. `_lang_supported` (57) compara con espacios alrededor para que `e`
no encaje dentro de `es`.

La precedencia, de `_lang_resolve` (líneas 118-129) y del comentario de 105-116:

1. `--lang <código>` / `--idioma <código>` / `--lang=<código>` — sólo esa orden
2. `ORBIT_LANG` en el entorno — esa sesión
3. `ORBIT_LANG` en `/etc/orbit/orbit.conf` — ese servidor (lo escribe `orbit lang <código>`)
4. `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG` de la sesión — con `C` y `POSIX` **excluidos** (65)
5. `/etc/default/locale` o `/etc/locale.conf` (84-102)
6. `es`, que es el idioma fuente

Que el 3 vaya por delante del 4 es deliberado y está razonado: «un servidor que se puso en inglés
sigue avisando en inglés a todo el equipo aunque tú entres con el tuyo».

**`--lang` vale en cualquier posición**, igual que `--json`, gracias a `_lang_strip` (1139-1157),
que a diferencia de `_json_strip` **sí respeta el `--`**: después de un `--`, lo que venga es un
valor y no una bandera, para que `orbit env set web FLAGS -- --lang` pueda guardar esa cadena.
Y no se aplica a `exec|run` (13642).

Hay además un `_lang_early` (204-215) que corre **antes** de la auto-elevación, para que
«Orbit necesita privilegios de root» y «Orbit no está instalado todavía» salgan en el idioma
correcto. Y la elevación reinyecta el idioma como bandera (256-258) porque `sudo` no conserva
`ORBIT_LANG` —sólo `LANG` y `LC_*` están en el `env_keep` de fábrica— y una preferencia que se
evapora al elevarse es peor que no tenerla.

**Cómo lo fuerza Orbit Desktop.** Con `--lang`, siempre, delante del comando:

```
orbit --lang en --json list
orbit --lang es --json deploy mi-web --progress
```

Y no con `ORBIT_LANG=` en el entorno, por dos motivos concretos: (a) `ssh` no propaga variables de
entorno salvo que el `sshd` remoto lo permita con `AcceptEnv`, que por defecto sólo cubre `LANG` y
`LC_*`; (b) un `ORBIT_LANG` mal escrito **se ignora** en el entorno pero **aborta** como bandera
(comentario de 252-255), y ese comportamiento asimétrico es justo lo que un cliente no quiere.
Con `--lang` validado por el cliente contra `["es","en"]` antes de enviarlo, no hay caso raro.

**Lo que NO cambia de idioma** (documentado en USAGE y verificado):

- **Los nombres de campo del JSON.** Son el contrato.
- Los valores enumerados: `level` (`ok|info|warn|error`), `status` de `deploy --all`
  (`deployed|failed|…`), `failed_step` (`code|release|…`), `service` (`running|stopped`),
  `kind` de `redirect` (`domain|path`). Todos en inglés, todos estables.
- `/var/log/orbit/orbit.log`.

**Lo que SÍ cambia:** `error` de `deploy --json`, `message` y `fix` de `doctor --json`,
`state` de `watch status --json` (¡ojo: `ok`, `rendido`, `sin-contacto`, `sin-rama` — el estado de
watch está en **español** en el código, líneas 10012-10014 y 5966-5975, y no está traducido: es un
valor de datos, no un mensaje), y todo lo que sale por stderr.

**Consecuencia de tipado:** `watch.subjects[].state` es una cadena en español que el cliente debe
mapear él mismo. Es la única fuga de idioma dentro de un campo enumerable del contrato, y hay que
tratarla como un `string` con un mapa conocido y un caso `default`, no como una unión cerrada.

**Y para la interfaz:** el idioma de la interfaz y el idioma que se le pide a `orbit` son **la misma
preferencia**, y hay que atarlos. Si el usuario tiene el cliente en inglés y el servidor contesta
`error: "el build ha fallado"`, la pantalla queda mestiza. Un solo selector, y el cliente le pasa a
`orbit` lo que él mismo está hablando.

### 1.14 `orbit new` con `--yes` y todas sus banderas

`cmd_new` (6881-7222). Es el comando más largo del script y el que más cuidado necesita del lado
del cliente, porque es el único que hace trece preguntas.

**Las banderas**, del parser 6892-6918 y de la ayuda 6743-6786:

| Bandera | Valor | Efecto |
|---|---|---|
| `--repo <url>` | https o git@ | Sin ella y sin GitHub conectado, `ask` → sin respuesta → `die "Sin repositorio no hay despliegue"` |
| `--name <nombre>` | | Validado con `_app_name_ok` (1176): `^[a-z0-9][a-z0-9._-]{0,39}$` y sin `..` |
| `--domain <dominio>` | sin `https://` | Sin ella, `die "Necesito un dominio"` |
| `--branch <rama>` | | Con `--yes` no se abre el selector de ramas remotas (6975) |
| `--aliases "<a b>"` | separados por espacios | **`--aliases ''` es una respuesta**: significa «ninguno», distinto de omitirla (6993) |
| `--email <correo>` | | Escribe `LETSENCRYPT_EMAIL` en el conf **global** (6940) |
| `--type <t>` | `static next node go bun deno php laravel python` | Validado antes de clonar (6929). `redirect` **no** está: esas las crea `redirect add` |
| `--build <cmd>` | `""` para ninguno | `--build ''` también es una respuesta (`o_build_set`) |
| `--start <cmd>` | | |
| `--outdir <dir>` | | |
| `--appdir <dir>` | | **No es un campo más**: dirige la detección entera (7010-7021) |
| `--spa yes\|no` | | Validado (6933) |
| `--php yes\|no` | | Validado (6937). `--php no` **apaga** lo detectado (7057) |
| `--docroot <dir>` | | |
| `--db` | | Crea la base de datos PostgreSQL |
| `--no-ssl` | | No emite certificado al terminar |
| `-y \| --yes \| --si` | | `ASSUME_YES="yes"` **local** a esta invocación (6889, 6915) |
| `-h \| --help \| help \| ayuda` | | Imprime la ayuda y `return 0` |

**Qué hace exactamente `--yes`.** Pone `ASSUME_YES=yes`, que hacen dos cosas y sólo dos: `ask`
(596-601) imprime el valor por defecto y **no lee**; `confirm` (623-624) responde el defecto y
**no lee**. Nada más. `--yes` **no es «que sí a todo»**, y las consecuencias concretas:

- **No se crea la base de datos.** `confirm "¿Crear una base de datos…?" "$want_db"` (7174) y
  `want_db` vale `"n"` salvo que venga `--db`. Correcto.
- **No se abre el editor del `.env`.** La condición es
  `[[ "$ASSUME_YES" != "yes" ]] && confirm … n` (7178). Doble protección.
- **Sí se emite el certificado**, porque `want_ssl` vale `"y"` por defecto (7217). Salvo que no
  haya `LETSENCRYPT_EMAIL`, en cuyo caso avisa y sigue **sin fallar** (7215-7217) — el bug que
  cuenta el comentario de 7204-7214, donde la app quedaba creada, desplegada y sirviendo, y el
  comando salía con 1.

**El primer despliegue va dentro** (7194): `if ! ( cmd_deploy "$name" ); then _new_undeployed …;
return 1; fi`. O sea: **`orbit new` puede tardar tres minutos y devolver 1 dejando la app creada**.
El texto de `_new_undeployed` (7226-7290) explica en prosa qué existe y qué no, distinguiendo tres
casos según haya release activa y según responda el puerto. Es excelente para una persona y es
inutilizable para un cliente: no hay JSON.

**Cómo lo resuelve Orbit Desktop.** El asistente de alta se hace en **dos pasos separados**, y ésta
es una decisión de diseño, no una limitación:

```
paso 1: orbit new --repo … --name … --domain … --branch … --type … --no-ssl --yes
paso 2 (opcional, con su propia barra): orbit deploy <name> --json --progress
paso 3 (opcional): orbit ssl <name>
```

Con `--no-ssl` en el paso 1, `orbit new` sigue haciendo el primer `cmd_deploy` interno (no hay
bandera para saltárselo), así que el paso 2 sólo hace falta si el paso 1 devolvió 1. Y como no hay
JSON, la forma de saber cómo quedó es **volver a preguntar**: `orbit --json info <name>` justo
después. Si la app existe y `state.served` es `true` y `state.releases > 0`, salió bien, diga lo
que diga el código de salida. Es más fiable que parsear `_new_undeployed`, es una llamada más de
300 ms sobre un comando que ha tardado tres minutos, y **es la única forma que no depende del
idioma**.

---

