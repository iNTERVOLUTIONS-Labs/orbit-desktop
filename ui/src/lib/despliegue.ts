// El despliegue en vivo: el modelo de la pantalla estrella.
//
// Es la única pantalla que tiene a alguien mirándola fijamente durante minutos,
// y donde el contrato da más de lo habitual: NDJSON por stderr con `--progress`
// y un objeto final que **también contesta cuando falla**.

import type { Despliegue, Lote } from './contrato'

/** Los seis pasos, en orden, con lo que pesa cada uno por defecto.
 *
 *  **Seis pasos no valen un sexto cada uno**: el build es típicamente el 70-85 %
 *  del tiempo, así que una barra lineal se quedaría clavada en el 33 % durante
 *  dos minutos y luego correría hasta el final. */
export const PASOS = [
  { id: 'code', texto: 'actualizar el clon de git', peso: 0.05 },
  { id: 'release', texto: 'copiar a la release nueva', peso: 0.05 },
  { id: 'build', texto: 'compilar', peso: 0.70 },
  { id: 'activate', texto: 'mover el symlink current', peso: 0.03 },
  { id: 'service', texto: 'reiniciar y esperar al health check', peso: 0.12 },
  { id: 'nginx', texto: 'recargar nginx', peso: 0.05 },
] as const

export interface SucesoDeProgreso {
  event: string
  app?: string | null
  step?: string | null
  status?: string | null
  elapsed_s?: number | null
}

export interface EstadoDePaso {
  id: string
  texto: string
  estado: 'pendiente' | 'haciendo' | 'hecho'
  /** El cronómetro se conserva al terminar: en tres despliegues alguien aprende
   *  cuánto tarda su build, y al cuarto sabe si algo va raro sin que nadie se
   *  lo diga. Es observabilidad regalada. */
  segundos: number | null
}

export interface Progreso {
  pasos: EstadoDePaso[]
  /** De 0 a 1. **Monótona creciente, nunca retrocede.** */
  avance: number
  transcurrido: number
  /** Que los pesos son los de por defecto porque el servidor se calló la
   *  tendencia. Se dice en la pantalla en vez de rellenarlo. */
  sinHistorico: boolean
  /** Las líneas que no se entendieron. Se cuentan y se dicen. */
  rotas: number
}

/** Los pesos, afinados con el histórico si lo hay.
 *
 *  Orbit **se calla la tendencia con menos de seis builds** —dos datos no son
 *  una tendencia y fingirla es peor que no tenerla— y la interfaz respeta ese
 *  silencio en vez de rellenarlo. */
export function pesos(buildMediana: number | null, totalMediana: number | null): number[] {
  const def = PASOS.map((p) => p.peso)
  if (buildMediana === null || totalMediana === null) return def
  if (totalMediana <= 0 || buildMediana > totalMediana) return def
  const build = Math.min(0.95, Math.max(0.05, buildMediana / totalMediana))
  const restoDef = def[0]! + def[1]! + def[3]! + def[4]! + def[5]!
  const escala = (1 - build) / restoDef
  return [def[0]! * escala, def[1]! * escala, build, def[3]! * escala, def[4]! * escala, def[5]! * escala]
}

/**
 * Lee el flujo de progreso y lo convierte en el estado de la pantalla.
 *
 * Tres reglas de robustez, todas derivadas de que los campos se añaden y nunca
 * se renombran:
 *
 *  1. Un `event` desconocido **no rompe nada**. Un cliente que se caiga porque
 *     el servidor añadió un evento es un cliente que rompe cada vez que Orbit
 *     mejora.
 *  2. Un `step` desconocido **se añade al final** en vez de descartarse, y la
 *     barra se recalcula. Si algún día hay un séptimo paso, la pantalla lo
 *     enseña sin actualizar el cliente.
 *  3. `app` se usa siempre para atribuir, también con una sola app: es el campo
 *     que existe para que un lote mezcle niveles por el mismo canal, y usarlo
 *     desde el principio evita tener dos analizadores.
 */
export function leerProgreso(
  texto: string,
  opciones: { app?: string; pesos?: number[]; anterior?: number } = {},
): Progreso {
  const w = opciones.pesos ?? PASOS.map((p) => p.peso)
  const pasos: EstadoDePaso[] = PASOS.map((p) => ({
    id: p.id, texto: p.texto, estado: 'pendiente', segundos: null,
  }))
  const inicio = new Map<string, number>()
  let transcurrido = 0
  let rotas = 0

  for (const cruda of texto.split('\n')) {
    const t = cruda.trim()
    // Por stderr va también todo lo que Orbit le cuenta a una persona. Lo que
    // no empieza por llave es prosa, no una línea rota.
    if (!t.startsWith('{')) continue
    let s: SucesoDeProgreso
    try {
      s = JSON.parse(t)
    } catch {
      rotas += 1
      continue
    }
    if (opciones.app && s.app && s.app !== opciones.app) continue
    if (typeof s.elapsed_s === 'number') transcurrido = Math.max(transcurrido, s.elapsed_s)
    if (s.event !== 'step' || !s.step) continue

    let p = pasos.find((x) => x.id === s.step)
    if (!p) {
      // Un paso que no conocemos se AÑADE, no se descarta: el día que Orbit
      // tenga un séptimo, esta pantalla lo enseña sin actualizarse.
      p = { id: s.step, texto: s.step, estado: 'pendiente', segundos: null }
      pasos.push(p)
    }
    if (s.status === 'start') {
      p.estado = 'haciendo'
      inicio.set(p.id, s.elapsed_s ?? 0)
    } else if (s.status === 'ok') {
      p.estado = 'hecho'
      const i = inicio.get(p.id)
      if (i !== undefined && typeof s.elapsed_s === 'number') p.segundos = s.elapsed_s - i
    }
  }

  let avance = 0
  pasos.forEach((p, i) => {
    if (p.estado === 'hecho') avance += w[i] ?? 0
  })
  // **Nunca retrocede.** Una barra que retrocede destruye más confianza que
  // cualquier error.
  avance = Math.min(1, Math.max(avance, opciones.anterior ?? 0))

  return {
    pasos, avance, transcurrido, rotas,
    sinHistorico: opciones.pesos === undefined,
  }
}

/** Los finales visuales de un despliegue.
 *
 *  El objeto no tiene un campo «resultado»: tiene `ok`, `rolled_back` y
 *  `recovered`, y su combinación da cuatro finales que **se ven distintos
 *  porque son distintos**. Desde la 1.3.8 hay dos más que no son fallos de
 *  nada, y pintarlos como «roto» mandaría a buscar el problema donde no está:
 *
 *  · **`negado`**: el servidor se negó antes de empezar —la app está
 *    congelada, u otro despliegue de la misma app no ha terminado—. No hay
 *    release ni paso fallido, porque no se ha tocado nada. Es el único final
 *    en el que la web es exactamente la de antes **por diseño**.
 *  · **`faltan`**: se paró antes del build porque falta una variable
 *    obligatoria. Lo que hay que hacer no es mirar un log, es dar el valor.
 *    Se reconoce por el paso (`release`, que es donde se comprueba) y porque
 *    el error **nombra** una de las de `env_missing`: los nombres de las
 *    variables no se traducen, la frase sí, y decidir leyendo la frase se
 *    rompería en el primer servidor en inglés. */
export type FinalVisual = 'bien' | 'recuperado' | 'revertido' | 'roto' | 'negado' | 'faltan'

export function finalDe(d: Despliegue): FinalVisual {
  if (d.ok) return d.recovered ? 'recuperado' : 'bien'
  if (d.rolled_back) return 'revertido'
  if (d.release === null && d.failed_step === null) return 'negado'
  if (faltanVariables(d).length > 0) return 'faltan'
  return 'roto'
}

/** Las variables **obligatorias** por las que se paró el despliegue, si se
 *  paró por eso.
 *
 *  `env_missing` trae todas las que siguen sin valor, obligatorias o no, y el
 *  objeto no dice cuáles son cuáles. Lo dice el `error`, que nombra las
 *  obligatorias (`faltan variables obligatorias: A B`): se buscan **como
 *  palabras enteras**, porque con `includes` una `DB` declarada daría por
 *  nombrada a `DB_PASSWORD`.
 *
 *  Vacío si no fue por eso, **aunque `env_missing` traiga algo**: una opcional
 *  sin valor no para nada, y un despliegue que falla en el build con una
 *  opcional pendiente sigue siendo un fallo del build. */
export function faltanVariables(d: Despliegue): string[] {
  const faltan = d.env_missing ?? []
  if (d.ok || d.failed_step !== 'release' || faltan.length === 0) return []
  const nombradas = new Set((d.error ?? '').split(/[^A-Za-z0-9_]+/))
  return faltan.filter((k) => nombradas.has(k))
}

/** Cuándo se ofrece volver a la release anterior.
 *
 *  Si `rolled_back` ya es cierto, **la vuelta atrás ya ocurrió**: ofrecerla otra
 *  vez sería ofrecer lo que acaba de pasar, y alguien haría clic pensando que
 *  hace falta. */
export function ofreceRollback(d: Despliegue): boolean {
  return d.previous !== null && d.rolled_back === false
}

/** Qué se enseña abierto según dónde se rompió.
 *
 *  El caso de `build` es el que más cambia la sensación de la pantalla: se abre
 *  **el final** del log y no el principio. Un error de compilación está en las
 *  últimas veinte líneas, y una consola que se abre por arriba obliga a
 *  desplazarse mil líneas hasta donde está la respuesta. */
export const QUE_MIRAR: Record<string, { mira: string; accion: string }> = {
  code:     { mira: 'El primer «fatal:» de git.',            accion: 'Revisar el repositorio' },
  release:  { mira: 'Espacio en disco y permisos.',          accion: 'Ir al diagnóstico' },
  build:    { mira: 'El FINAL del log del build.',           accion: 'Ver el log' },
  activate: { mira: 'El estado del symlink current.',        accion: 'Ir al diagnóstico' },
  service:  { mira: 'Las últimas líneas del journal.',       accion: 'Volver a la anterior' },
  nginx:    { mira: 'La salida de nginx -t.',                accion: 'Ir al diagnóstico' },
}

/** Los finales de un lote, con su glifo y su palabra.
 *
 *  **No se agrupan nunca**: confundir «no hay cambios» con «no he podido
 *  preguntar» costó un fallo real —un remoto caído anunciado como «nada que
 *  hacer» cada cinco minutos— y el contrato los separa para que un cliente no
 *  pueda repetirlo.
 *
 *  Los seis primeros son los de siempre y se enseñan siempre. Los cinco de las
 *  puertas llegaron en la 1.4.3 y **sólo se enseñan si el servidor los
 *  cuenta**: en un servidor anterior su recuento no es cero, no existe, y una
 *  celda con un cero diría que ha mirado. */
export const FINALES_DEL_LOTE = [
  { id: 'deployed',    glifo: '●', texto: 'desplegadas',   frase: 'Se han desplegado.', base: true },
  { id: 'failed',      glifo: '✕', texto: 'fallidas',      frase: 'Se han intentado y han fallado.', base: true },
  { id: 'unchanged',   glifo: '—', texto: 'al día',        frase: 'El remoto no ha avanzado.', base: true },
  { id: 'unreachable', glifo: '▲', texto: 'sin contacto',  frase: 'No he podido preguntarle al remoto. NO es lo mismo que «al día».', base: true },
  // Glifo propio, y no el `✕` de `failed`. La primera versión compartía los dos
  // —color y glifo— y se vio mirando una captura: dos celdas idénticas salvo
  // por la palabra, que es justo lo que la regla de «el color nunca va solo»
  // existe para evitar.
  //
  // El color sí lo comparte con `failed`, y es deliberado: la paleta tiene
  // cinco tokens de estado, los cinco con su contraste comprobado, e inventar
  // un sexto para esto sería añadir un color que nadie sabría nombrar. Las dos
  // cosas quieren decir «algo va mal y hay que mirarlo»; lo que las separa es
  // el glifo y la palabra, que es donde tiene que estar.
  { id: 'gone',        glifo: '↯', texto: 'rama perdida',  frase: 'La rama configurada ya no existe en el remoto.', base: true },
  { id: 'skipped',     glifo: '·', texto: 'saltadas',      frase: 'Este commit ya rompió el build; se espera al siguiente.', base: true },
  // Las cinco puertas de la 1.4.3. Cada una con su glifo, por la misma regla:
  // ninguno es un emoji en las fuentes de color —`✗` y no `✖`, que Orbit ya
  // aprendió que se lo queda la fuente de emoji— y todos están en DejaVu Sans
  // Mono.
  { id: 'locked',      glifo: '■', texto: 'congeladas',    frase: 'Alguien las congeló con «orbit lock»: ni se le pregunta al remoto. El rollback sigue permitido.', base: false },
  { id: 'ignored',     glifo: '○', texto: 'sin cambios vigilados', frase: 'Hay commits nuevos, pero ninguno toca las rutas que vigila su orbit.json.', base: false },
  { id: 'ci_pending',  glifo: '…', texto: 'esperando a la CI', frase: 'La CI de GitHub de ese commit sigue corriendo. Se desplegará en una pasada siguiente.', base: false },
  { id: 'ci_failed',   glifo: '✗', texto: 'CI en rojo',    frase: 'La CI de ese commit ha fallado y no se despliega. Se desplegará el siguiente que pase.', base: false },
  // El `unreachable` de la CI, con su color: no es «pendiente» ni «en rojo»,
  // es no saber. Por eso pone `ok` a `false`.
  { id: 'ci_unknown',  glifo: '?', texto: 'CI sin respuesta', frase: 'No he podido preguntarle a GitHub por la CI. NO es lo mismo que «esperando».', base: false },
] as const

export type IdDeFinal = (typeof FINALES_DEL_LOTE)[number]['id']

/** La palabra de un final para una fila. Uno que no conozcamos se enseña tal
 *  cual lo manda el servidor: es mejor una palabra en inglés que una fila que
 *  dice otra cosa. */
export function textoDeFinal(id: string): string {
  return FINALES_DEL_LOTE.find((f) => f.id === id)?.texto ?? id
}

/** Los recuentos, con `n: null` cuando el servidor no lo cuenta. */
export function recuentos(
  l: Lote,
): Array<{ id: IdDeFinal; glifo: string; texto: string; frase: string; base: boolean; n: number | null }> {
  const r = l as unknown as Record<string, number | undefined>
  return FINALES_DEL_LOTE.map((f) => ({
    ...f,
    n: typeof r[f.id] === 'number' ? r[f.id]! : f.base ? 0 : null,
  }))
}

/** Las que necesitan una mirada: la misma regla que `ok` y el código de
 *  salida —fallidas, sin contacto, rama perdida y CI sin respuesta—. */
export function necesitanMirada(l: Lote): number {
  return l.failed + l.unreachable + l.gone + (l.ci_unknown ?? 0)
}

/** Las que no se han desplegado **a propósito**: congeladas o retenidas por
 *  una CI que todavía corre o está en rojo. No ponen `ok` a `false` —el
 *  servidor ha hecho lo que se le pidió— pero tampoco son «al día», y un
 *  veredicto que dijera «ninguna falló» sin más se leería como «todo está
 *  publicado». */
export function retenidas(l: Lote): number {
  return (l.locked ?? 0) + (l.ci_pending ?? 0) + (l.ci_failed ?? 0)
}

// ── La pasada por todas las apps ────────────────────────────────────────────

/** Una app dentro de una pasada, mientras la pasada ocurre. */
export interface AppEnPasada {
  app: string
  /** El final, cuando lo tiene. `null` mientras está en curso.
   *
   *  Es la cadena tal cual la manda el servidor y **no se traduce a un
   *  enumerado**: un final que no conozcamos tiene que poder llegar a la
   *  pantalla, y un analizador que sólo admita seis valores es un cliente que
   *  se rompe el día que Orbit añada el séptimo. */
  final: string | null
  /** En qué paso va, si está desplegando. Sale de los sucesos `step`, que
   *  llegan **mezclados** con los de la pasada por el mismo canal. */
  paso: string | null
}

export interface Pasada {
  apps: AppEnPasada[]
  /** Cuál se está desplegando ahora. `null` entre una y otra, y al terminar. */
  enCurso: string | null
  terminadas: number
  /** Segundos desde que arrancó la pasada.
   *
   *  Sale **sólo** de los sucesos `app`. Los sucesos `step` traen su propio
   *  `elapsed_s`, que cuenta desde que empezó **esa app** (`DEP_T0`) y no desde
   *  que empezó la pasada (`DALL_T0`): mezclarlos haría que el reloj saltara
   *  hacia atrás en cada app nueva. */
  transcurrido: number
  rotas: number
}

/**
 * Lee el progreso de `deploy --all --progress`.
 *
 * Por el mismo canal llegan **dos niveles de suceso** —`{"event":"app"}` para
 * el principio y el final de cada app, `{"event":"step"}` para los seis pasos
 * de dentro— y los dos traen `app`, que es el campo que Orbit añadió justo para
 * que se puedan atribuir. Eso permite decir a la vez cuántas van y en qué paso
 * está la que corre, que es bastante más útil que una ruedecita.
 *
 * Misma regla de siempre: **una línea rota no tumba una pasada**, y un `event`
 * que no conozcamos se ignora sin contarse como rota.
 */
export function leerPasada(texto: string): Pasada {
  const apps: AppEnPasada[] = []
  const indice = new Map<string, AppEnPasada>()
  let enCurso: string | null = null
  let transcurrido = 0
  let rotas = 0

  const dame = (nombre: string): AppEnPasada => {
    let a = indice.get(nombre)
    if (!a) {
      a = { app: nombre, final: null, paso: null }
      indice.set(nombre, a)
      // En el orden en que el servidor las va tocando, que es el orden en que
      // ocurren. Ordenarlas alfabéticamente escondería cuál va ahora.
      apps.push(a)
    }
    return a
  }

  for (const cruda of texto.split('\n')) {
    const t = cruda.trim()
    // Por stderr va también todo lo que Orbit le cuenta a una persona. Lo que
    // no empieza por llave es prosa, no una línea rota.
    if (!t.startsWith('{')) continue
    let s: SucesoDeProgreso
    try {
      s = JSON.parse(t)
    } catch {
      rotas += 1
      continue
    }
    if (!s.app) continue

    if (s.event === 'app') {
      if (typeof s.elapsed_s === 'number') transcurrido = Math.max(transcurrido, s.elapsed_s)
      const a = dame(s.app)
      if (s.status === 'start') {
        enCurso = s.app
        a.final = null
      } else if (s.status) {
        a.final = s.status
        a.paso = null
        if (enCurso === s.app) enCurso = null
      }
    } else if (s.event === 'step' && s.step) {
      // Un paso de dentro. No mueve el reloj de la pasada — ver `transcurrido`.
      const a = dame(s.app)
      if (a.final === null) a.paso = s.step
      // Y un paso implica que esa app está corriendo, aunque su «start» de nivel
      // de app se hubiera perdido: el canal puede llegar cortado por arriba si
      // alguien abre la pantalla a mitad.
      if (a.final === null) enCurso = s.app
    }
  }

  return {
    apps,
    enCurso,
    terminadas: apps.filter((a) => a.final !== null).length,
    transcurrido,
    rotas,
  }
}

/**
 * Qué finales puede dar una pasada, según cómo se lance.
 *
 * **No es una curiosidad: cambia lo que la pantalla puede prometer.** Los
 * cuatro finales baratos salen de preguntarle al remoto de cada app, y sin
 * `--if-changed` no se le pregunta a nadie — así que una pasada completa sólo
 * puede terminar en `deployed` o en `failed`, y sus otros cuatro recuentos
 * serán cero por construcción y no porque no haya pasado nada.
 *
 * `skipped` no aparece en ninguna de las dos: sólo lo produce `--auto`, que es
 * la bandera del autodespliegue y que este cliente no pasa nunca.
 */
export function finalesPosibles(soloSiCambia: boolean, conPuertas = false): string[] {
  if (!conPuertas) {
    return soloSiCambia
      ? ['deployed', 'failed', 'unchanged', 'unreachable', 'gone']
      : ['deployed', 'failed']
  }
  // Desde la 1.4.3 la rama sin `--if-changed` ya no es sólo «desplegada o
  // fallida». Una congelada se salta en las dos, antes de nada. Y una app con
  // la puerta de la CI **sí le pregunta al remoto** aunque no se haya pedido
  // `--if-changed` —la CI necesita saber qué commit mirar—, así que en esa
  // rama también pueden salir «sin contacto», «rama perdida» y los tres de la
  // CI. Lo que sigue sin poder salir sin `--if-changed` es «al día» y «sin
  // cambios vigilados»: son las dos respuestas a «¿ha cambiado?», y esa
  // pregunta no se hace.
  //
  // `conPuertas` sale de la respuesta y no de una versión: que el lote traiga
  // `locked`, o que la lista traiga `state.locked`. Un servidor anterior sigue
  // con la regla de arriba, que para él sigue siendo verdad.
  return soloSiCambia
    ? ['deployed', 'failed', 'unchanged', 'unreachable', 'gone',
       'locked', 'ignored', 'ci_pending', 'ci_failed', 'ci_unknown']
    : ['deployed', 'failed', 'unreachable', 'gone',
       'locked', 'ci_pending', 'ci_failed', 'ci_unknown']
}

// ── Las órdenes que se enseñan antes de lanzarlas ──────────────────────────

/** Lo que se enseña en lugar de un valor. El mismo en el núcleo (`OCULTO`), y
 *  siempre igual: la longitud de una contraseña también es un dato sobre ella. */
export const OCULTO = '••••••'

export interface VariableNueva {
  clave: string
  valor: string
}

/** Los argumentos de un despliegue **tal como se enseñan**: los mismos que
 *  manda el núcleo (`Comando::argv_visible`), con cada valor de `--env`
 *  tapado. Se comparan los dos contra `tests/contrato/ordenes.json`. */
export function argvDeDespliegue(app: string, entorno: VariableNueva[] = [], forzar = false): string[] {
  const v = ['--json', 'deploy', app, '--progress']
  if (forzar) v.push('--force')
  for (const e of entorno) v.push('--env', `${e.clave}=${OCULTO}`)
  return v
}

/** `orbit lock`. Sin `--json`, que `lock` no habla. Un motivo en blanco es no
 *  decir motivo, igual que en el núcleo. */
export function argvDeCongelar(app: string, motivo: string | null): string[] {
  const v = ['lock', app]
  if (motivo !== null && motivo.trim() !== '') v.push('-m', motivo)
  return v
}

export function argvDeDescongelar(app: string): string[] {
  return ['unlock', app]
}

export function argvDeEsperarCi(app: string, esperar: boolean): string[] {
  return ['autodeploy', 'checks', esperar ? 'on' : 'off', app]
}

/** La orden para leerla, entrecomillada lo justo. Sólo para enseñar: lo que
 *  viaja son los argumentos, y el escapado de verdad lo hace el núcleo. */
export function ordenLegible(argv: string[], binario = 'orbit'): string {
  return [binario, ...argv]
    .map((t) => (t === '' || /[^A-Za-z0-9_./:@=•-]/.test(t) ? `'${t.split("'").join(`'\\''`)}'` : t))
    .join(' ')
}

/** Las mismas reglas que el núcleo y que `_env_value_problem` en el servidor:
 *  lo que no se puede guardar se dice aquí, junto al campo, y no a mitad de un
 *  despliegue. Devuelve el motivo, o `null`. **Nunca repite el valor.** */
export function problemaDelValor(v: string): string | null {
  if (/[\r\n]/.test(v)) return 'lleva saltos de línea, y en un .env cada variable es una línea'
  if (v.includes("'")) return 'lleva una comilla simple, y systemd y bash no la escapan igual'
  if (v.includes('\0')) return 'lleva un byte nulo'
  return null
}

/** Lo mismo para el motivo de una congelación: una línea y 200 caracteres. */
export function problemaDelMotivo(m: string): string | null {
  if ([...m].length > 200) return 'pasa de 200 caracteres'
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(m)) return 'tiene que caber en una línea'
  return null
}

/**
 * Cada cuánto se vuelve a pedir el monitor.
 *
 * **Medido: `orbit top --json` cuesta ~2,1 s con 40 apps** (`≈1.053 ms +
 * 26,6 ms/app`), y no es lentitud — la CPU es la diferencia entre dos lecturas
 * del cgroup, así que una foto suelta tiene que esperar a la segunda a
 * propósito.
 *
 * O sea que el plan de refrescar cada dos segundos era **físicamente
 * imposible**: venía de copiar el intervalo del panel en vivo de Orbit sin ver
 * que ése reutiliza la muestra anterior y no paga esa espera.
 *
 * Así que el periodo se adapta a lo que de verdad tardó la última vez, con un
 * margen. Encadenar peticiones más rápido de lo que contestan no da más
 * frescura: da una cola.
 */
export function periodoDelMonitor(ultimaDuracionMs: number | null): number {
  if (ultimaDuracionMs === null) return 3
  const s = (ultimaDuracionMs / 1000) * 1.5
  return Math.max(3, Math.min(30, Math.ceil(s)))
}
