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

/** Los cuatro finales visuales de un despliegue.
 *
 *  El objeto no tiene un campo «resultado» con cuatro valores: tiene `ok`,
 *  `rolled_back` y `recovered`, y su combinación da cuatro finales que **se ven
 *  distintos porque son distintos**. */
export type FinalVisual = 'bien' | 'recuperado' | 'revertido' | 'roto'

export function finalDe(d: Despliegue): FinalVisual {
  if (d.ok) return d.recovered ? 'recuperado' : 'bien'
  return d.rolled_back ? 'revertido' : 'roto'
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

/** Los seis finales de un lote, con su glifo y su palabra.
 *
 *  Son seis y **no se agrupan nunca**: confundir «no hay cambios» con «no he
 *  podido preguntar» costó un fallo real —un remoto caído anunciado como «nada
 *  que hacer» cada cinco minutos— y el contrato los separa para que un cliente
 *  no pueda repetirlo. */
export const FINALES_DEL_LOTE = [
  { id: 'deployed',    glifo: '●', texto: 'desplegadas',   frase: 'Se han desplegado.' },
  { id: 'failed',      glifo: '✕', texto: 'fallidas',      frase: 'Se han intentado y han fallado.' },
  { id: 'unchanged',   glifo: '—', texto: 'al día',        frase: 'El remoto no ha avanzado.' },
  { id: 'unreachable', glifo: '▲', texto: 'sin contacto',  frase: 'No he podido preguntarle al remoto. NO es lo mismo que «al día».' },
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
  { id: 'gone',        glifo: '↯', texto: 'rama perdida',  frase: 'La rama configurada ya no existe en el remoto.' },
  { id: 'skipped',     glifo: '·', texto: 'saltadas',      frase: 'Este commit ya rompió el build; se espera al siguiente.' },
] as const

export function recuentos(l: Lote): Array<{ id: string; glifo: string; texto: string; frase: string; n: number }> {
  return FINALES_DEL_LOTE.map((f) => ({
    ...f,
    n: (l as unknown as Record<string, number>)[f.id] ?? 0,
  }))
}
