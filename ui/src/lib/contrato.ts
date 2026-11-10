// El contrato --json, tipado. Es el gemelo en TypeScript de
// `crates/orbit-client/src/contrato.rs`, y la regla es la misma:
//
//   **lo que no existe es null, no cero ni cadena vacía.**
//
// El puerto de una web estática es `null` porque no tiene puerto; el 0 sería un
// puerto. Su `service` también es `null` y no `"stopped"`, porque no hay ningún
// proceso que arrancar — confundir «no aplica» con «está caída» pinta una
// alarma roja donde no pasa nada, y eso enseña a la gente a ignorar las
// alarmas.
//
// Aquí eso se traduce en `| null` en todos los sitios donde el contrato lo dice,
// y en que NO hay valores por defecto en ninguna parte.

export const CONTRATO_CONOCIDO = 1

export interface Version {
  schema: number
  /** La de Orbit: semver. */
  version: string
  /** La del contrato: un entero. Son ejes distintos, y confundirlos hace que un
   *  cliente rechace un servidor perfectamente compatible. */
  contract: number
}

export interface Estado {
  /** `null` en una web estática: no hay proceso. NO es "stopped". */
  service: string | null
  /** `null` en una web estática. El 0 sería un puerto. */
  port: number | null
  ssl: boolean
  /** `null` = «no lo he mirado en esta llamada», que no es «no hay
   *  certificado». Sólo `info` lo calcula. Puede ser **negativo**, y eso es
   *  real: un certificado caducado. */
  cert_days: number | null
  maintenance: boolean
  /** **Lo primero que hay que mirar.** Con `false` hay app registrada y no hay
   *  vhost: nginx cierra la conexión, ni 404 ni 502. */
  served: boolean
  autodeploy: boolean
  queue: boolean
  releases: number | null
  last_deploy: string | null
  last_deploy_sha: string | null

  // ── Lo que llegó después de la línea base (1.3.6) ────────────────────────
  //
  // Opcionales, y **opcional no quiere decir «por defecto falso»**: que un
  // 1.3.6 no diga `locked` no significa que la app no esté congelada, significa
  // que ese servidor no sabe congelar. `undefined` es «este servidor no lo
  // dice», `null` es «lo dice y no hay». Son dos respuestas distintas y se
  // pintan distinto — la misma regla de siempre, en el eje de la versión.

  /** De quién es el certificado. `'origin'` es el de Cloudflare, que **sólo
   *  vale detrás de su proxy**: quien entre a la IP del servidor sin pasar por
   *  Cloudflare ve un error de certificado. 1.4.7. */
  cert_type?: 'letsencrypt' | 'origin' | string | null
  /** El autodespliegue y `deploy --all` esperan a la CI de GitHub. 1.4.3. */
  checks?: boolean
  /** Los despliegues están congelados (`orbit lock`). **No es un estado de lo
   *  que se sirve**: una congelada atiende igual. 1.4.3. */
  locked?: boolean
  /** Quién, cuándo y por qué. `null` sin congelación. Cada campo de dentro
   *  puede ser `null`: un `touch` a mano del testigo no dice quién. */
  lock?: Congelacion | null
}

export interface Congelacion {
  at: string | null
  by: string | null
  reason: string | null
}

export interface App {
  name: string
  type: string
  domain: string
  aliases: string[]
  state: Estado
}

export interface Lista {
  schema: number
  apps: App[]
}

/** Los estados visibles. La resolución tiene que ser **la misma** en la lista,
 *  en el detalle y en el monitor: si dos pantallas resuelven distinto, el
 *  producto ha perdido. Por eso vive en una función y no en cada componente. */
export type Salud =
  | 'sin-vhost'
  | 'mantenimiento'
  | 'no-aplica'
  | 'activo'
  | 'parado'
  | 'desconocido'

/** El orden de precedencia, escrito. El primero que se cumple, gana. */
export function salud(e: Estado): Salud {
  // 1 · Sin vhost gana sobre TODO, incluido el mantenimiento: sin vhost tampoco
  //     se sirve la página de 503, así que ningún otro campo describe lo que
  //     recibe un visitante.
  if (e.served === false) return 'sin-vhost'
  // 2 · Alguien la bajó a propósito. No es una avería.
  if (e.maintenance === true) return 'mantenimiento'
  // 3 · El proceso, si lo hay.
  return proceso(e.service)
}

/** Lo que dice `service` por sí solo, sin el vhost ni el mantenimiento.
 *
 *  Aparte de `salud()` porque hay quien sólo pregunta por el proceso —el
 *  asistente, al terminar `orbit new`— y la primera versión de esa pregunta se
 *  escribió a mano comparando con `'active'`: toda app sana salía «el proceso
 *  no se mantiene en pie». Una sola tabla, y todos la leen. */
export function proceso(service: string | null): 'no-aplica' | 'activo' | 'parado' | 'desconocido' {
  // No hay proceso que arrancar. **Esto no es un fallo.**
  if (service === null) return 'no-aplica'
  // `svc_state` de Orbit emite **`running`** o `stopped`, y así lo dice el
  // contrato. Las muestras de las pruebas decían `active` —la palabra de
  // systemd— y el cliente sólo conocía ésa: contra un servidor de verdad, toda
  // app con proceso salía «no se sabe». `active` se sigue aceptando porque es
  // inequívoca, pero la que llega es `running`.
  if (service === 'running' || service === 'active') return 'activo'
  if (service === 'inactive' || service === 'failed' || service === 'stopped') return 'parado'
  // systemd contestó algo que no esperábamos. Se dice, no se traduce a
  // «parado»: un estado desconocido pintado como conocido es una mentira.
  return 'desconocido'
}

/** Glifo, texto y frase. El color nunca va solo, así que los tres viajan
 *  juntos y no hay forma de pintar uno sin los otros. */
export const PRESENTACION: Record<Salud, { glifo: string; texto: string; frase: string }> = {
  activo:        { glifo: '●', texto: 'activo',        frase: 'El servicio responde.' },
  parado:        { glifo: '✕', texto: 'parado',        frase: 'El servicio existe y no está corriendo.' },
  'no-aplica':   { glifo: '—', texto: '—',             frase: 'Web estática: no hay ningún proceso que arrancar. Esto no es un fallo.' },
  mantenimiento: { glifo: '▲', texto: 'mantenimiento', frase: 'nginx devuelve 503 con tu página de «volvemos enseguida».' },
  'sin-vhost':   { glifo: '⊘', texto: 'sin vhost',     frase: 'nginx no tiene el vhost. La conexión se cierra: ni 404 ni 502.' },
  desconocido:   { glifo: '·', texto: '·',             frase: 'No se sabe todavía.' },
}

/** La regla de forma del servidor, copiada de `_app_name_ok`:
 *  `^[a-z0-9][a-z0-9._-]{0,39}$`, sin `..`.
 *
 *  Se aplica también —y sobre todo— a los nombres que llegan **del servidor**:
 *  `app_names()` en Orbit enumera los `.conf` sin filtrar, así que un servidor
 *  comprometido puede meter ahí lo que quiera. **Un dato que ha dado la vuelta
 *  por el servidor no es de más confianza que uno tecleado: es de menos.** */
export function nombreOperable(s: string): boolean {
  if (s.length === 0 || s.length > 40 || s.includes('..')) return false
  return /^[a-z0-9][a-z0-9._-]*$/.test(s)
}

/** Un número que no se sabe se pinta como que no se sabe. Nunca como 0.
 *  Un cero es una afirmación. */
export function num(n: number | null | undefined): string {
  // También lo que no es un número: lo validado nunca lo es, pero esto se pinta
  // y un «NaN» en una celda es peor que un «·».
  return typeof n === 'number' && Number.isFinite(n) ? String(n) : '·'
}

/**
 * Hace visible lo que engaña al ojo.
 *
 * Un nombre de app llega del servidor y `app_names()` en Orbit enumera los
 * `.conf` **sin filtrar**, así que puede traer lo que sea. Tres cosas concretas
 * pasan desapercibidas y las tres sirven para que alguien confirme una acción
 * sobre una app creyendo que es otra:
 *
 *  · **Bidi overrides** (`U+202E` y familia). Es el ataque «Trojan Source»
 *    aplicado a una lista: el nombre `produccion‮gnitset-` se pinta en pantalla
 *    como `produccion-testing`. Lo comprobé mirando una captura, no razonando:
 *    la fila se veía perfectamente normal.
 *  · **Homoglifos.** `оrbit` con o cirílica es indistinguible de `orbit`.
 *  · **Caracteres de ancho cero.** `con​cero` lleva un `U+200B` dentro y no se
 *    ve nada.
 *
 * La regla de forma del servidor ya deja fuera a los tres —es ASCII y
 * minúsculas— así que **cualquier carácter fuera del ASCII imprimible en un
 * nombre es, por definición, algo sobre lo que no se puede operar**. Se marca
 * con su punto de código y no se borra: borrarlo sería «arreglar» el nombre, y
 * un nombre arreglado ya no identifica a nadie.
 */
export function marcarInvisibles(s: string): string {
  let fuera = ''
  for (const c of s) {
    const p = c.codePointAt(0)!
    fuera += p >= 0x20 && p <= 0x7e ? c : `‹U+${p.toString(16).toUpperCase().padStart(4, '0')}›`
  }
  return fuera
}

// ── el diagnóstico ─────────────────────────────────────────────────────────

export interface Comprobacion {
  id: string
  level: 'ok' | 'info' | 'warn' | 'error'
  message: string
  fix: string | null
  /** Si `orbit doctor --fix` se encargaría de esto.
   *
   *  **El botón sólo se enseña aquí.** Uno que no hace nada es peor que
   *  ninguno: invita a averiguar por qué no hace nada, y la respuesta es una
   *  frase que se podría haber leído sin el botón. Este campo existe
   *  exactamente para poder distinguirlo. */
  fixable: boolean
}

export interface Doctor {
  schema: number
  checks: Comprobacion[]
  summary: { ok: number; warn: number; error: number }
}

/** El orden en que se leen los problemas: primero lo que está roto, después lo
 *  que avisa, y al final lo que está bien —que se puede plegar—. Ordenar por el
 *  orden en que Orbit los comprueba pondría un `ok` entre dos errores. */
export const PESO_NIVEL: Record<Comprobacion['level'], number> = {
  error: 0, warn: 1, info: 2, ok: 3,
}

// ── el log ─────────────────────────────────────────────────────────────────

export type Canal = 'journal' | 'access' | 'error'

export interface LineaDeLog {
  /** Sale del propio log y no se inventa. `null` cuando el formato viejo de
   *  nginx no la lleva — y ése es el momento de ofrecer `orbit nginx-rebuild`,
   *  no de poner la hora de ahora. */
  ts: string | null
  stream: Canal
  /** La línea tal cual. **No se estructura**: sacarle el nivel o el código HTTP
   *  sería inventar un formato que la aplicación del usuario no ha prometido. */
  text: string
}

export interface MetaDeLog {
  schema: number
  app: string
  source: 'journal' | 'nginx'
  unit: string | null
  since: string | null
  follow: boolean
  lines: number | null
}

export interface Log {
  meta: MetaDeLog | null
  lineas: LineaDeLog[]
  /** Que se llegó al tope de `--lines`, por fuente. */
  truncado: boolean
  /** Cuántas líneas no se entendieron. **Se enseña**: si se callara, un log a
   *  medias se leería como un log completo. */
  rotas: number
}

/** Lee el NDJSON de `orbit logs --json`.
 *
 *  Una línea rota se salta y se cuenta. Abortar convertiría un byte mal puesto
 *  en una pantalla vacía, y quien mira un log suele estar mirándolo justamente
 *  porque algo va mal. */
export function leerLog(texto: string): Log {
  const out: Log = { meta: null, lineas: [], truncado: false, rotas: 0 }
  for (const cruda of texto.split('\n')) {
    const t = cruda.trim()
    if (!t) continue
    let crudo: unknown
    try {
      crudo = JSON.parse(t)
    } catch {
      out.rotas += 1
      continue
    }
    // `null` o `42` son JSON válido y no son un suceso: leerles `.event`
    // reventaba el visor entero por una línea.
    if (typeof crudo !== 'object' || crudo === null || Array.isArray(crudo)) {
      out.rotas += 1
      continue
    }
    const v = crudo as Record<string, unknown>
    // Por su campo `event` y no por su forma: adivinar por los campos que trae
    // haría que un campo nuevo cambiara el tipo de un suceso, y los campos se
    // añaden.
    switch (v.event) {
      case 'meta': {
        // La cabecera se mira como cualquier otra respuesta. Si no tiene su
        // forma se cuenta como rota y no se pinta: un `follow` que no es
        // booleano no puede decidir si el log es una foto o un flujo.
        try {
          out.meta = validar('logs', leerMetaDeLog, v)
        } catch {
          out.rotas += 1
        }
        break
      }
      case 'line': {
        // Se mira la forma antes de creérsela: una línea sin texto o con un
        // canal que no existe se cuenta como rota, no se pinta a medias.
        const canalOk = v.stream === 'journal' || v.stream === 'access' || v.stream === 'error'
        if (typeof v.text !== 'string' || !canalOk || (v.ts !== null && v.ts !== undefined && typeof v.ts !== 'string')) {
          out.rotas += 1
          break
        }
        out.lineas.push({ ts: (v.ts as string | null | undefined) ?? null, stream: v.stream as Canal, text: v.text })
        break
      }
      case 'end':
        if (typeof v.truncated === 'boolean') out.truncado = v.truncated
        else out.rotas += 1
        break
      // Un suceso que no conocemos se ignora sin ruido: los sucesos se añaden,
      // igual que los campos.
      default: break
    }
  }
  return out
}

// ── el despliegue ──────────────────────────────────────────────────────────

export interface Commit {
  sha: string | null
  subject: string | null
  ref: string | null
}

export interface Despliegue {
  schema: number
  app: string
  ok: boolean
  release: string | null
  /** La release anterior. Viaja en el objeto **precisamente** para poder
   *  ofrecer la vuelta atrás sin una segunda llamada. */
  previous: string | null
  commit: Commit
  /** Salió mal y se volvió atrás. */
  rolled_back: boolean
  /** Orbit arregló el build por su cuenta y reintentó. Es **distinto** de
   *  `rolled_back` y se enseña distinto: los dos campos existen para que un
   *  panel pueda distinguir lo que es distinto. */
  recovered: boolean
  duration_s: number
  failed_step: string | null
  error: string | null
  /** Las variables que siguen sin valor tras el despliegue. 1.3.8; `undefined`
   *  en un servidor anterior, que **no es «no falta ninguna»**. Cuando falta
   *  una obligatoria el despliegue se para antes del build, y la forma de
   *  seguir es volver a lanzarlo con `--env CLAVE=valor`. */
  env_missing?: string[]
}

/** Los finales de una pasada. Eran seis; la 1.4.3 añadió los cinco de las
 *  puertas. Y se admite cualquier otra cadena: un final que no conozcamos
 *  tiene que poder llegar a la pantalla en vez de tumbar el lote. */
export type FinalDeLote =
  | 'deployed' | 'failed' | 'unchanged' | 'unreachable' | 'gone' | 'skipped'
  | 'locked' | 'ignored' | 'ci_pending' | 'ci_failed' | 'ci_unknown'
  | (string & {})

export interface AppDelLote {
  app: string
  status: FinalDeLote
  /** El objeto de `deploy <app> --json` **sin recortar**. `null` en las que no
   *  se han desplegado, y entonces el motivo va en `error`: un null es una
   *  respuesta, un objeto a medias no. */
  result: Despliegue | null
  error: string | null
}

export interface Lote {
  schema: number
  apps: AppDelLote[]
  total: number
  deployed: number
  failed: number
  unchanged: number
  unreachable: number
  gone: number
  skipped: number
  // Los cinco de las puertas, 1.4.3. Ausentes en un servidor anterior: ése no
  // puede producirlos, y un cero diría que ha mirado.
  locked?: number
  ignored?: number
  ci_pending?: number
  ci_failed?: number
  ci_unknown?: number
  /** La **misma regla que el código de salida**: ni fallidas, ni sin contacto,
   *  ni ramas desaparecidas, ni CI a la que no se ha podido preguntar. Existe
   *  para que quien mire el objeto y quien mire el rc no puedan discrepar
   *  nunca. */
  ok: boolean
  duration_s: number
}

// ── lo que declara el repositorio ──────────────────────────────────────────

/** `orbit env check <app> --json`. 1.3.8. **Sin un solo valor.**
 *
 *  Mira la release que **está sirviendo**: si el commit nuevo declara una
 *  variable más, esto no la ve todavía. Es una comprobación previa, y la última
 *  palabra la tiene el despliegue con `env_missing`. */
export interface ComprobacionDeEntorno {
  schema: number
  app: string
  file: string
  ok: boolean
  vars: VariableDeclarada[]
}

export interface VariableDeclarada {
  key: string
  /** Si el `.env` la tiene. **Nunca el valor.** */
  set: boolean
  required: boolean
  /** `orbit.json` o `example`: las del `.env.example` nunca son obligatorias. */
  source: 'orbit.json' | 'example' | string
}

/** Las que pararían el próximo despliegue antes del build. */
export function obligatoriasSinValor(c: ComprobacionDeEntorno): string[] {
  return c.vars.filter((v) => v.required && !v.set).map((v) => v.key)
}

/** `orbit cron list <app> --json`. 1.3.9. */
export interface Tareas {
  schema: number
  app: string
  tasks: Tarea[]
}

export interface Tarea {
  name: string
  /** Como lo da systemd (`Thu 2026-11-05 03:00:00 UTC`), no ISO-8601. `null`
   *  es «no ha corrido nunca», que no es una fecha vacía. */
  last_run: string | null
  next_run: string | null
  /** El `Result` de systemd de la última pasada: `success`, `exit-code`… */
  result: string | null
}

// ── qué sabe hacer un servidor ─────────────────────────────────────────────

/** Lo que no estaba en la línea base, y desde qué versión de Orbit. Es el
 *  gemelo de `Capacidad` en `contrato.rs`, y los números salen del CHANGELOG
 *  de Orbit, comprobados contra el código. */
export const CAPACIDADES = {
  /** `env check --json`, `deploy --env` y `env_missing`. */
  entornoDeclarado: [1, 3, 8],
  /** `cron list --json`. */
  tareas: [1, 3, 9],
  /** `lock`/`unlock`, `deploy --force` y `state.locked`. */
  congelar: [1, 4, 3],
  /** `autodeploy checks on|off` y `state.checks`. */
  esperarCi: [1, 4, 3],
  /** `state.cert_type`. */
  tipoDeCertificado: [1, 4, 7],
  /** `deploy --env-stdin`: las variables por la entrada, fuera del `argv`. */
  entornoPorEntrada: [1, 4, 9],
  /** `"csp"` y `"csp_report_only"` en el `orbit.json`. Antes se ignoran. */
  csp: [1, 5, 1],
  /** `backup list|verify --json`, `logs --json` y `doctor --fix --yes`. La
   *  sección «No publicado» del CHANGELOG de Orbit los cuenta como si fueran de
   *  siempre, y no lo son: no están en la etiqueta 1.3.6, salieron en la 1.3.7. */
  contratoCompleto: [1, 3, 7],
} as const

export type Capacidad = keyof typeof CAPACIDADES

/** La versión por componentes. `null` si no es semver: entonces no se sabe.
 *
 *  Acepta `unknown` a propósito: es lo primero que se hace con una respuesta, y
 *  un `version: 1.5` (un número) reventaba aquí y se llevaba por delante todo lo
 *  que se pide detrás. Una `v` delante (`v1.5.1`, la etiqueta de git) se lee. */
export function semver(v: unknown): [number, number, number] | null {
  if (typeof v !== 'string' || v === '') return null
  const base = v.replace(/^v/, '').split(/[-+]/)[0]!
  const p = base.split('.')
  if (p.length < 2 || p.length > 3) return null
  const n = p.map((x) => (/^\d+$/.test(x) ? Number(x) : NaN))
  if (n.some((x) => Number.isNaN(x))) return null
  return [n[0]!, n[1]!, n[2] ?? 0]
}

/** Si un servidor sabe hacer algo, por su versión de Orbit.
 *
 *  Por versión y no por contrato: el contrato sigue en 1, porque todo esto se
 *  ha **añadido** sin romper nada, que es justamente lo que promete. Con una
 *  versión que no se entiende la respuesta es **no**: ofrecer un botón que el
 *  servidor no entiende es peor que no ofrecerlo. Lo que se lee de una
 *  respuesta —`locked`, `cert_type`— no pasa por aquí: manda que el campo
 *  esté, que es más fiable que cualquier tabla. */
/** Lo contrario de `sabe`, pero **sólo cuando se sabe**: la versión se ha
 *  leído y es anterior. Con una versión desconocida no se esconde lo que ya se
 *  ofrecía antes de que existiera esta tabla — esconderlo por no haber podido
 *  preguntar sería quitarle a alguien el log porque falló un saludo. */
export function seQuedaCorto(version: unknown, c: Capacidad): boolean {
  return semver(version) !== null && !sabe(version, c)
}

export function sabe(version: unknown, c: Capacidad): boolean {
  const v = semver(version)
  if (!v) return false
  const d = CAPACIDADES[c]
  for (let i = 0; i < 3; i++) {
    if (v[i]! !== d[i]!) return v[i]! > d[i]!
  }
  return true
}

// ── el entorno ─────────────────────────────────────────────────────────────

export interface Entorno {
  schema: number
  app: string
  /** **Sólo los nombres.** Los secretos no cruzan el contrato: un panel que
   *  enseñe el `.env` entero filtra la contraseña de la base de datos en la
   *  primera captura que alguien pegue en un issue. */
  keys: string[]
}

// ── el monitor ─────────────────────────────────────────────────────────────

export interface AppDelMonitor {
  name: string
  type: string
  domain: string
  port: number | null
  service: string | null
  /** `null` la primera vez, y eso es «no se sabe». La CPU es la **diferencia
   *  entre dos lecturas** del cgroup, así que la primera no da porcentaje.
   *  Inventar un cero sería mentir: un cero es una afirmación. */
  cpu_percent: number | null
  memory_bytes: number | null
  requests_last_minute: number | null
  /** Que el minuto llenó el tope de líneas de log que se miran. Un número corto
   *  sin avisar se lee como «hay poco tráfico», que es justo lo contrario de lo
   *  que está pasando. */
  requests_capped: boolean
}

export interface Monitor {
  schema: number
  apps: AppDelMonitor[]
}

/** Bytes en algo legible. El contrato los da en bruto **a propósito** —`du -h`
 *  es presentación, y su separador decimal depende de la configuración regional
 *  del servidor— así que la presentación se hace aquí. */
export function bytes(n: number | null | undefined): string {
  if (typeof n !== 'number' || !Number.isFinite(n)) return '·'
  const u = ['B', 'KB', 'MB', 'GB']
  let v = n
  let i = 0
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i += 1 }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${u[i]}`
}

// ── el tráfico ─────────────────────────────────────────────────────────────

export interface Latencia {
  p50: number | null
  p95: number | null
  max: number | null
  /** Sobre cuántas líneas se calcularon. Con 0, los tres percentiles son `null`
   *  y **no se pintan**: un percentil sin muestras no es un cero, es nada. */
  lines: number
}

export interface Trafico {
  app: string
  since: string
  from: string
  /** **`false` = la ventana pedida excede lo que el log cubre.** Se dice, no se
   *  devuelve un número más pequeño y se calla: un total recortado sin avisar
   *  se lee como el total. */
  complete: boolean
  requests: number | null
  /** Son **IPs, no personas**. Una misma persona con datos móviles son tres. */
  ips: number | null
  ips_capped: boolean
  bytes: number | null
  /** Lo automático va **aparte**. En un VPS con IP pública buena parte del
   *  tráfico son escáneres buscando `/.git/config`, y sumarlo al de las
   *  personas convierte la analítica en un número que no describe a nadie. */
  automated: number | null
  status: Record<string, number>
  latency_ms: Latencia
  paths: Array<{ path: string; requests: number }>
  referrers?: Array<{ referrer: string; requests: number }>
  hours?: Array<{ hour: string; requests: number }>
}

/** Las respuestas de `traffic` y `metrics` vienen envueltas: `{schema, apps:[…]}`
 *  también cuando se pide una sola app. Lo comprobamos ejecutando, porque la
 *  documentación las daba planas. */
export interface Envoltorio<T> {
  schema: number
  apps: T[]
}

// ── las métricas ───────────────────────────────────────────────────────────

export interface Metricas {
  app: string
  deploys: number | null
  failed: number | null
  /** La **mediana**, no la media. Un build que una vez tardó 400 s no describe
   *  ningún despliegue real. */
  build_median_s: number | null
  /** **`null` con menos de seis builds**, a propósito: dos datos no son una
   *  tendencia y fingirla es peor que no tenerla. Cuando es null, **no se pinta
   *  ninguna flecha** — ni siquiera una plana. */
  build_trend_s: number | null
  last: string | null
}

// ── exec ───────────────────────────────────────────────────────────────────

export interface SalidaDeExec {
  /** La orden literal, ya escapada. Se enseña **antes** de ejecutar: es lo que
   *  convierte «confío en la interfaz» en «he leído lo que va a pasar». */
  orden: string
  stdout: string
  stderr: string
  codigo: number
}

/** Los patrones que hacen que la pantalla pida una confirmación reforzada.
 *
 *  **Es una lista negra, y por eso su valor es pedagógico y no defensivo.** No
 *  impide nada —hay mil formas de escribir un `rm`— y quien quiera saltársela lo
 *  hará sin proponérselo. Lo que sí para es el error de dedos a las tres de la
 *  mañana, que es el caso real y el único contra el que sirve.
 *
 *  Se documenta así para que nadie la confunda con una protección y construya
 *  encima suponiendo que lo es. */
export function parecePeligroso(texto: string): string | null {
  const t = texto.toLowerCase()
  const patrones: Array<[string, string]> = [
    ['rm -rf /', 'borra recursivamente desde una ruta absoluta'],
    ['drop database', 'borra una base de datos entera'],
    ['truncate', 'vacía una tabla'],
    ['mkfs', 'formatea un sistema de ficheros'],
    ['dd of=/dev/', 'escribe directamente sobre un dispositivo'],
    ['chmod -r 777 /', 'abre los permisos de todo'],
    ['> /dev/sd', 'escribe sobre un disco'],
  ]
  return patrones.find(([p]) => t.includes(p))?.[1] ?? null
}

// ── el detalle de una app ──────────────────────────────────────────────────

export interface AppInfo {
  name: string
  path: string
  /** El fichero clave a clave, y **por eso todos sus valores son cadenas**: en
   *  el fichero lo son. Los datos con tipo están en `state`. */
  config: Record<string, string>
  state: Estado
  releases: string[]
}

export interface Info {
  schema: number
  app: AppInfo
}

// ── el saludo ──────────────────────────────────────────────────────────────

export type ClaseDeSaludo =
  | 'ok' | 'mas-nuevo' | 'sin-contrato' | 'no-instalado'
  | 'sin-privilegios' | 'no-se-llega' | 'clave-de-host-cambiada'

export interface Saludo {
  clase: ClaseDeSaludo
  version: string | null
  contrato: number | null
  motivo: string | null
  puede_operar: boolean
  puede_leer: boolean
  orden_de_instalacion: string
}

// ── el servidor ────────────────────────────────────────────────────────────
//
// Llega en la MISMA llamada que la portada (`status --json`), y hasta ahora se
// tiraba. Enseñarlo no cuesta ni un saludo SSH más.

export interface Host {
  hostname: string
  ip: string
  uptime_seconds: number | null
  /** Las tres cargas. **Puede venir vacía** —un `/proc/loadavg` que no se
   *  pudo leer—, y vacía no es «carga cero». */
  load: number[]
  memory_kb: { total: number | null; used: number | null; available: number | null }
  disk_kb: { mount: string; total: number | null; used: number | null; use_percent: number | null }
}

export interface ServicioDelHost {
  name: string
  active: boolean
}

export interface Portada {
  schema?: number
  /** Ausente en las muestras sin envoltorio y en un servidor que no lo diga:
   *  entonces la vista del servidor dice que no lo sabe. */
  host?: Host
  services?: ServicioDelHost[]
  apps: App[]
}

// ── las copias (`backup list|verify --json`, 1.3.7) ────────────────────────

export interface Copia {
  /** Ruta absoluta en el servidor. Se enseña tal cual: es lo que se pasa a
   *  `orbit restore`. */
  file: string
  /** `null` en la copia de la configuración global, que no es de ninguna app. */
  app: string | null
  kind: 'app' | 'config' | string
  /** ISO-8601 con huso. */
  created: string
  size_bytes: number | null
  /** **Siempre `null` en `list`**: «no lo he comprobado en esta llamada» no es
   *  «está mal». Abrir cada fichero cuesta, y es la orden de al lado. */
  verified: boolean | null
}

export interface Copias {
  schema: number
  dir: string
  backups: Copia[]
  total: number
  bytes: number
  keep_days: number | null
}

export interface CopiaVerificada {
  file: string
  kind: string
  app: string | null
  ok: boolean
  /** En el idioma del servidor. Se enseña, no se interpreta. */
  message: string
}

export interface Verificacion {
  schema: number
  backups: CopiaVerificada[]
  total: number
  good: number
  bad: number
  /** La misma regla que el código de salida: quien mire esto y quien mire el
   *  `exit` no pueden discrepar. */
  ok: boolean
}

// ── lo demás del servidor ──────────────────────────────────────────────────

export interface Redireccion {
  app: string
  /** `domain` redirige un dominio entero, y entonces `from` es el **nombre de
   *  la app**, no un dominio: así lo da Orbit. `path`, una ruta. */
  kind: 'domain' | 'path' | string
  from: string
  to: string
  /** `null` cuando el descriptor trae un código que no es un número: Orbit lo
   *  pasa por `_j_num`, que no inventa. */
  code: number | null
  query: boolean
}

export interface Redirecciones {
  schema: number
  redirects: Redireccion[]
}

export interface BaseDeDatos {
  name: string
  owner: string
  size_bytes: number | null
}

export interface BasesDeDatos {
  schema: number
  databases: BaseDeDatos[]
}

export interface Vigilado {
  /** `nginx`, `app:<n>`, `ci:<n>`, `remoto:<n>`… Uno desconocido se enseña
   *  con su nombre. */
  subject: string
  state: 'ok' | 'fallo' | 'rendido' | string
  /** Época en segundos. **`0` es «no se sabe»** —así lo escribe Orbit—, no el
   *  1 de enero de 1970. `null` si el fichero de estado trae basura: pasa por
   *  `_j_num`. */
  since: number | null
  tries: number | null
  last_try: number | null
}

export interface Vigilancia {
  schema: number
  timer_active: boolean
  subjects: Vigilado[]
  summary: { total: number; down: number; warning: number }
}

export interface Colas {
  schema: number
  timer_active: boolean
  every_minutes: number | null
  max_seconds: number | null
  apps: Array<{ app: string; type: string; connection: string | null }>
}

/** Una época de Orbit, donde el 0 significa «no se sabe». */
export function epoca(s: number | null | undefined): Date | null {
  return typeof s === 'number' && s > 0 ? new Date(s * 1000) : null
}

/** Segundos en algo legible: «3 d 4 h», «12 min». `null` es «·». */
export function duracion(s: number | null | undefined): string {
  if (typeof s !== 'number' || !Number.isFinite(s) || s < 0) return '·'
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60)
  if (d > 0) return `${d} d ${h} h`
  if (h > 0) return `${h} h ${m} min`
  if (m > 0) return `${m} min`
  return `${Math.floor(s)} s`
}

// ── la forma de lo que llega ───────────────────────────────────────────────
//
// El puente devuelve lo que el servidor escribió **sin mirarlo**: en Rust es un
// `serde_json::Value`, y un tipo de TypeScript es una promesa que nadie
// comprueba en ejecución. Así que `ok:"false"` se pintaba «Desplegada», un
// `name:42` dejaba la lista en «Cargando…» para siempre y un `cpu_percent` que
// faltaba salía como «undefined %».
//
// Aquí se mira la forma **una vez, al entrar**, y lo de dentro de la ventana ya
// se puede creer. Las reglas son las del contrato:
//
//  · un campo que no conocemos se ignora sin ruido: los campos se añaden;
//  · uno esperado que falta o trae otro tipo es un **error de la respuesta**,
//    nunca un valor por defecto — un default es inventarse lo que el servidor
//    no ha dicho;
//  · lo que llegó después de la 1.3.6 es opcional, y opcional no es «falso»:
//    si no está, no está.
//
// Sin dependencias a propósito: son unas pocas funciones, y cada respuesta se
// lee con ellas en un bloque que se compara a ojo con su emisor en Orbit.

/** Lo que se lanza cuando una respuesta no tiene la forma del contrato. Es la
 *  forma de los errores del puente, así que se pinta como cualquier otro. */
export interface ErrorDeForma {
  clase: 'forma'
  mensaje: string
  detalle: string | null
}

/** El fallo por dentro, con la ruta. Sólo vive hasta `validar()`, que le pone
 *  el nombre del comando y lo convierte en un error del puente. */
class FormaRota {
  constructor(
    readonly ruta: string,
    readonly esperado: string,
    readonly llego: unknown,
  ) {}
}

/** Lee un valor de la forma `T`, o lanza diciendo dónde no la tiene. */
export type Lector<T> = (v: unknown, ruta: string) => T

/** Un campo que un servidor anterior no emite. */
export interface Opcional<T> {
  readonly opcional: Lector<T>
}

type Campos = Record<string, Lector<unknown> | Opcional<unknown>>
type Obligatorios<F> = {
  [K in keyof F as F[K] extends Opcional<unknown> ? never : K]: F[K] extends Lector<infer T> ? T : never
}
type Opcionales<F> = {
  [K in keyof F as F[K] extends Opcional<unknown> ? K : never]?: F[K] extends Opcional<infer T> ? T : never
}
type Plano<T> = { [K in keyof T]: T[K] }
export type Leido<F> = Plano<Obligatorios<F> & Opcionales<F>>

function esObjeto(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/** Qué es lo que llegó, dicho para una persona. */
function queEs(v: unknown): string {
  if (v === null) return 'null'
  if (Array.isArray(v)) return 'una lista'
  switch (typeof v) {
    case 'string': return 'texto'
    case 'number': return 'un número'
    case 'boolean': return 'verdadero o falso'
    case 'object': return 'un objeto'
    default: return typeof v
  }
}

export const texto: Lector<string> = (v, ruta) => {
  if (typeof v !== 'string') throw new FormaRota(ruta, 'texto', v)
  return v
}

export const numero: Lector<number> = (v, ruta) => {
  if (typeof v !== 'number' || !Number.isFinite(v)) throw new FormaRota(ruta, 'un número', v)
  return v
}

/** **Un booleano de verdad.** `"false"` es una cadena que no está vacía, y un
 *  `if` la lee como cierta: así se pintaba bien un despliegue que falló. */
export const booleano: Lector<boolean> = (v, ruta) => {
  if (typeof v !== 'boolean') throw new FormaRota(ruta, 'verdadero o falso', v)
  return v
}

/** `null` o `T`. **No admite que falte**: Orbit escribe el `null`, y un campo
 *  ausente no es un `null`, es otra respuesta. */
export function nulo<T>(l: Lector<T>): Lector<T | null> {
  return (v, ruta) => {
    if (v === null) return null
    try {
      return l(v, ruta)
    } catch (e) {
      if (e instanceof FormaRota && e.ruta === ruta) throw new FormaRota(ruta, `${e.esperado} o null`, v)
      throw e
    }
  }
}

export function opcional<T>(l: Lector<T>): Opcional<T> {
  return { opcional: l }
}

export function lista<T>(l: Lector<T>): Lector<T[]> {
  return (v, ruta) => {
    if (!Array.isArray(v)) throw new FormaRota(ruta, 'una lista', v)
    return v.map((x, i) => l(x, `${ruta}[${i}]`))
  }
}

export function enumerado<const T extends string>(valores: readonly T[]): Lector<T> {
  return (v, ruta) => {
    if (typeof v === 'string' && (valores as readonly string[]).includes(v)) return v as T
    throw new FormaRota(ruta, `uno de ${valores.map((x) => `«${x}»`).join(', ')}`, v)
  }
}

/** Un objeto con los campos que se le dicen. Lo que traiga de más se queda
 *  fuera del valor que sale: lo que no se conoce no se usa. */
export function objeto<F extends Campos>(campos: F): Lector<Leido<F>> {
  return (v, ruta) => {
    if (!esObjeto(v)) throw new FormaRota(ruta, 'un objeto', v)
    const fuera: Record<string, unknown> = {}
    for (const k of Object.keys(campos)) {
      const c = campos[k]!
      const sub = ruta === '' ? k : `${ruta}.${k}`
      // `hasOwn` y no `v[k]` a secas: un campo que no viene no puede salir del
      // prototipo de Object y pasar por uno que sí.
      const valor = Object.hasOwn(v, k) ? v[k] : undefined
      if ('opcional' in c) {
        if (valor !== undefined) fuera[k] = c.opcional(valor, sub)
      } else {
        fuera[k] = c(valor, sub)
      }
    }
    return fuera as Leido<F>
  }
}

/** Un objeto de claves que pone el servidor —el `config` de `info`, las clases
 *  del `status` del tráfico—. Sale **sin prototipo**: en un `{}` normal una
 *  clave `__proto__` no se guarda, cambia el prototipo, y la clave desaparece. */
export function registro<T>(l: Lector<T>): Lector<Record<string, T>> {
  return (v, ruta) => {
    if (!esObjeto(v)) throw new FormaRota(ruta, 'un objeto', v)
    const fuera = Object.create(null) as Record<string, T>
    for (const k of Object.keys(v)) fuera[k] = l(v[k], ruta === '' ? k : `${ruta}.${k}`)
    return fuera
  }
}

/** Un trozo de lo que llegó, para el detalle del error. Corto: es para
 *  reconocerlo, no para leerlo entero. */
function muestra(v: unknown): string | null {
  if (v === undefined) return null
  let s: string
  try {
    s = JSON.stringify(v) ?? String(v)
  } catch {
    s = String(v)
  }
  return s.length > 200 ? `${s.slice(0, 200)}…` : s
}

/** Lee una respuesta entera. `comando` es el nombre con el que se reconoce
 *  —el de Orbit cuando lo es: `status`, `backup list`…—, porque el mensaje
 *  tiene que decir **qué** respuesta y **dónde**, no «algo ha ido mal». */
export function validar<T>(comando: string, l: Lector<T>, v: unknown): T {
  try {
    return l(v, '')
  } catch (e) {
    if (!(e instanceof FormaRota)) throw e
    const donde = e.ruta === '' ? 'la respuesta' : e.ruta
    const mensaje =
      e.llego === undefined
        ? e.ruta === ''
          ? `«${comando}»: no ha llegado ninguna respuesta, y debería ser ${e.esperado}`
          : `«${comando}»: falta ${donde}, que debería ser ${e.esperado}`
        : `«${comando}»: ${donde} debería ser ${e.esperado} y es ${queEs(e.llego)}`
    const fallo: ErrorDeForma = { clase: 'forma', mensaje, detalle: muestra(e.llego) }
    throw fallo
  }
}

// ── las respuestas, una a una ──────────────────────────────────────────────
//
// Cada una es el gemelo de su emisor en `orbit` (`_status_json`,
// `_app_state_json`, `_deploy_json`…), mirado en la 1.3.6 y en la 1.5.1. Los
// `nulo` están donde el emisor escribe `null` —o pasa por `_j_num`, que escribe
// `null` con lo que no es un número—, y en ningún otro sitio.

const textoONulo = nulo(texto)
const numeroONulo = nulo(numero)

export const leerVersion: Lector<Version> = objeto({ schema: numero, version: texto, contract: numero })

const leerCongelacion: Lector<Congelacion> = objeto({ at: textoONulo, by: textoONulo, reason: textoONulo })

export const leerEstado: Lector<Estado> = objeto({
  service: textoONulo,
  port: numeroONulo,
  ssl: booleano,
  cert_days: numeroONulo,
  maintenance: booleano,
  served: booleano,
  autodeploy: booleano,
  queue: booleano,
  releases: numeroONulo,
  last_deploy: textoONulo,
  last_deploy_sha: textoONulo,
  // 1.4.7
  cert_type: opcional(textoONulo),
  // 1.4.3
  checks: opcional(booleano),
  locked: opcional(booleano),
  lock: opcional(nulo(leerCongelacion)),
})

export const leerApp: Lector<App> = objeto({
  name: texto,
  type: texto,
  domain: texto,
  aliases: lista(texto),
  state: leerEstado,
})

export const leerLista: Lector<Lista> = objeto({ schema: numero, apps: lista(leerApp) })

export const leerHost: Lector<Host> = objeto({
  hostname: texto,
  ip: texto,
  uptime_seconds: numeroONulo,
  // Vacía si `/proc/loadavg` no se pudo leer; nunca con `null` dentro.
  load: lista(numero),
  memory_kb: objeto({ total: numeroONulo, used: numeroONulo, available: numeroONulo }),
  disk_kb: objeto({ mount: texto, total: numeroONulo, used: numeroONulo, use_percent: numeroONulo }),
})

/** `status --json`. El host y los servicios están desde la línea base: si
 *  faltan, la respuesta está mal, no es un servidor que «no lo dice». */
export const leerPortada: Lector<Portada & { schema: number; host: Host; services: ServicioDelHost[] }> = objeto({
  schema: numero,
  host: leerHost,
  services: lista(objeto({ name: texto, active: booleano })),
  apps: lista(leerApp),
})

export const leerInfo: Lector<Info> = objeto({
  schema: numero,
  app: objeto({
    name: texto,
    path: texto,
    // El fichero clave a clave: todo cadenas, y las claves las pone el fichero.
    config: registro(texto),
    state: leerEstado,
    releases: lista(texto),
  }),
})

export const leerDoctor: Lector<Doctor> = objeto({
  schema: numero,
  checks: lista(objeto({
    id: texto,
    // Cerrado a propósito: la pantalla ordena y pinta por nivel, y un nivel que
    // no conoce no tiene sitio en ninguna de las dos cosas.
    level: enumerado(['ok', 'info', 'warn', 'error']),
    message: texto,
    fix: textoONulo,
    fixable: booleano,
  })),
  summary: objeto({ ok: numero, warn: numero, error: numero }),
})

export const leerMetaDeLog: Lector<MetaDeLog> = objeto({
  schema: numero,
  app: texto,
  source: enumerado(['journal', 'nginx']),
  unit: textoONulo,
  since: textoONulo,
  follow: booleano,
  lines: numeroONulo,
})

export const leerDespliegue: Lector<Despliegue> = objeto({
  schema: numero,
  app: texto,
  ok: booleano,
  release: textoONulo,
  previous: textoONulo,
  commit: objeto({ sha: textoONulo, subject: textoONulo, ref: textoONulo }),
  rolled_back: booleano,
  recovered: booleano,
  duration_s: numero,
  failed_step: textoONulo,
  error: textoONulo,
  // 1.3.8
  env_missing: opcional(lista(texto)),
})

export const leerLote: Lector<Lote> = objeto({
  schema: numero,
  apps: lista(objeto({
    app: texto,
    // Abierto: un final que no conozcamos tiene que llegar a la pantalla.
    status: texto,
    result: nulo(leerDespliegue),
    error: textoONulo,
  })),
  total: numero,
  deployed: numero,
  failed: numero,
  unchanged: numero,
  unreachable: numero,
  gone: numero,
  skipped: numero,
  // 1.4.3
  locked: opcional(numero),
  ignored: opcional(numero),
  ci_pending: opcional(numero),
  ci_failed: opcional(numero),
  ci_unknown: opcional(numero),
  ok: booleano,
  duration_s: numero,
})

export const leerComprobacionDeEntorno: Lector<ComprobacionDeEntorno> = objeto({
  schema: numero,
  app: texto,
  file: texto,
  ok: booleano,
  vars: lista(objeto({ key: texto, set: booleano, required: booleano, source: texto })),
})

export const leerTareas: Lector<Tareas> = objeto({
  schema: numero,
  app: texto,
  tasks: lista(objeto({ name: texto, last_run: textoONulo, next_run: textoONulo, result: textoONulo })),
})

export const leerEntorno: Lector<Entorno> = objeto({ schema: numero, app: texto, keys: lista(texto) })

export const leerMonitor: Lector<Monitor> = objeto({
  schema: numero,
  apps: lista(objeto({
    name: texto,
    type: texto,
    domain: texto,
    port: numeroONulo,
    service: textoONulo,
    cpu_percent: numeroONulo,
    memory_bytes: numeroONulo,
    requests_last_minute: numeroONulo,
    requests_capped: booleano,
  })),
})

const leerTrafico: Lector<Trafico> = objeto({
  app: texto,
  since: texto,
  from: texto,
  complete: booleano,
  requests: numeroONulo,
  ips: numeroONulo,
  ips_capped: booleano,
  bytes: numeroONulo,
  automated: numeroONulo,
  // Disperso: sólo las clases con tráfico, y puede ser `{}`.
  status: registro(numero),
  latency_ms: objeto({ p50: numeroONulo, p95: numeroONulo, max: numeroONulo, lines: numero }),
  paths: lista(objeto({ path: texto, requests: numero })),
  referrers: opcional(lista(objeto({ referrer: texto, requests: numero }))),
  hours: opcional(lista(objeto({ hour: texto, requests: numero }))),
})

const leerMetricas: Lector<Metricas> = objeto({
  app: texto,
  deploys: numeroONulo,
  failed: numeroONulo,
  build_median_s: numeroONulo,
  build_trend_s: numeroONulo,
  last: textoONulo,
})

function envuelto<T>(l: Lector<T>): Lector<Envoltorio<T>> {
  return objeto({ schema: numero, apps: lista(l) })
}

/** `traffic --json` y `metrics --json` vienen envueltos también con una app. */
export const leerTraficos: Lector<Envoltorio<Trafico>> = envuelto(leerTrafico)
export const leerMetricasEnvueltas: Lector<Envoltorio<Metricas>> = envuelto(leerMetricas)

export const leerSalidaDeExec: Lector<SalidaDeExec> = objeto({
  orden: texto, stdout: texto, stderr: texto, codigo: numero,
})

/** El saludo lo compone el núcleo, no Orbit, pero cruza el mismo puente sin
 *  tipos: se mira igual. */
export const leerSaludo: Lector<Saludo> = objeto({
  clase: enumerado([
    'ok', 'mas-nuevo', 'sin-contrato', 'no-instalado', 'sin-privilegios', 'no-se-llega', 'clave-de-host-cambiada',
  ]),
  version: textoONulo,
  contrato: numeroONulo,
  motivo: textoONulo,
  puede_operar: booleano,
  puede_leer: booleano,
  orden_de_instalacion: texto,
})

export const leerCopias: Lector<Copias> = objeto({
  schema: numero,
  dir: texto,
  backups: lista(objeto({
    file: texto,
    app: textoONulo,
    kind: texto,
    created: texto,
    size_bytes: numeroONulo,
    // Siempre `null` en `list`, y así tiene que llegar: no se ha comprobado.
    verified: nulo(booleano),
  })),
  total: numero,
  bytes: numero,
  keep_days: numeroONulo,
})

export const leerVerificacion: Lector<Verificacion> = objeto({
  schema: numero,
  backups: lista(objeto({ file: texto, kind: texto, app: textoONulo, ok: booleano, message: texto })),
  total: numero,
  good: numero,
  bad: numero,
  ok: booleano,
})

export const leerRedirecciones: Lector<Redirecciones> = objeto({
  schema: numero,
  redirects: lista(objeto({
    app: texto, kind: texto, from: texto, to: texto, code: numeroONulo, query: booleano,
  })),
})

export const leerBasesDeDatos: Lector<BasesDeDatos> = objeto({
  schema: numero,
  databases: lista(objeto({ name: texto, owner: texto, size_bytes: numeroONulo })),
})

export const leerVigilancia: Lector<Vigilancia> = objeto({
  schema: numero,
  timer_active: booleano,
  subjects: lista(objeto({
    subject: texto, state: texto, since: numeroONulo, tries: numeroONulo, last_try: numeroONulo,
  })),
  summary: objeto({ total: numero, down: numero, warning: numero }),
})

export const leerColas: Lector<Colas> = objeto({
  schema: numero,
  timer_active: booleano,
  every_minutes: numeroONulo,
  max_seconds: numeroONulo,
  apps: lista(objeto({ app: texto, type: texto, connection: textoONulo })),
})
