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
  // 3 · No hay proceso que arrancar. **Esto no es un fallo.**
  if (e.service === null) return 'no-aplica'
  if (e.service === 'active') return 'activo'
  if (e.service === 'inactive' || e.service === 'failed' || e.service === 'stopped') {
    return 'parado'
  }
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
  return n === null || n === undefined ? '·' : String(n)
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
    let v: Record<string, unknown>
    try {
      v = JSON.parse(t)
    } catch {
      out.rotas += 1
      continue
    }
    // Por su campo `event` y no por su forma: adivinar por los campos que trae
    // haría que un campo nuevo cambiara el tipo de un suceso, y los campos se
    // añaden.
    switch (v.event) {
      case 'meta': out.meta = v as unknown as MetaDeLog; break
      case 'line': out.lineas.push(v as unknown as LineaDeLog); break
      case 'end': out.truncado = v.truncated === true; break
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
}

export type FinalDeLote =
  | 'deployed' | 'failed' | 'unchanged' | 'unreachable' | 'gone' | 'skipped'

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
  /** La **misma regla que el código de salida**: ni fallidas, ni sin contacto,
   *  ni ramas desaparecidas. Existe para que quien mire el objeto y quien mire
   *  el rc no puedan discrepar nunca. */
  ok: boolean
  duration_s: number
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
export function bytes(n: number | null): string {
  if (n === null) return '·'
  const u = ['B', 'KB', 'MB', 'GB']
  let v = n
  let i = 0
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i += 1 }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${u[i]}`
}
