// Lo que el monitor ha visto **en esta sesión**, para dibujar la tendencia.
//
// El roadmap deja las gráficas históricas fuera porque exigirían que el
// cliente guarde algo del servidor, y eso es media hoja del modelo de
// amenazas. Esto no guarda nada: vive en la memoria del proceso, se pierde al
// cerrar la ventana y no sale de ella. Es lo mismo que hace `top`, que también
// recuerda la lectura anterior para poder restar.
//
// Las claves son `servidor:app` y nunca la app sola, como en todo el cliente.
// Y un `null` se guarda como `null`: un hueco en la línea es «no se sabe», y
// unirlo con sus vecinos dibujaría una medida que nadie ha tomado.

import type { Monitor } from './contrato'

/** Cuántas muestras por serie. Con el periodo medido (2-5 s con un servidor
 *  normal) son unos cinco minutos, que es lo que cabe en una celda. */
export const MUESTRAS = 90

export interface Serie {
  readonly cpu: readonly (number | null)[]
  readonly memoria: readonly (number | null)[]
  readonly peticiones: readonly (number | null)[]
}

// Un solo `$state.raw` que se sustituye entero **una vez por pasada** del
// monitor. La primera versión eran arrays en `$state` profundo con `push` y
// `splice`: cada muestra era una escritura vigilada, y con 300 apps el
// apunte costaba ~20 ms por pasada (73 ms con el monitor pintado). Copiar 90
// números por serie es más barato que vigilar cada uno.
//
// Un `Map` y no un objeto: la clave lleva el nombre de una app que manda el
// servidor, y una app llamada `__proto__` no puede tocar el prototipo.
let series = $state.raw<ReadonlyMap<string, Serie>>(new Map())

function con<T>(a: readonly T[], v: T): T[] {
  const b = a.length >= MUESTRAS ? a.slice(a.length - MUESTRAS + 1) : a.slice()
  b.push(v)
  return b
}

const VACIA: Serie = { cpu: [], memoria: [], peticiones: [] }

export function anotar(servidor: string, m: Monitor): void {
  if (!Array.isArray(m?.apps) || m.apps.length === 0) return
  const nuevo = new Map(series)
  for (const a of m.apps) {
    const k = `${servidor}:${a.name}`
    const s = nuevo.get(k) ?? VACIA
    nuevo.set(k, {
      cpu: con(s.cpu, a.cpu_percent),
      memoria: con(s.memoria, a.memory_bytes),
      // Con el tope lleno el número es un suelo, no una medida: se guarda
      // igual, y la tabla ya marca el «+».
      peticiones: con(s.peticiones, a.requests_last_minute),
    })
  }
  series = nuevo
}

export function serie(servidor: string, app: string): Serie | undefined {
  return series.get(`${servidor}:${app}`)
}

/** Al cambiar de servidor no se borra nada: volver a uno tiene que enseñar lo
 *  que ya se había visto. Sólo se olvida a petición. */
export function olvidar(servidor: string): void {
  const nuevo = new Map(series)
  for (const k of nuevo.keys()) if (k.startsWith(`${servidor}:`)) nuevo.delete(k)
  series = nuevo
}
