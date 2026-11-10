// La paleta de órdenes: lo que se puede buscar y cómo se puntúa.
export interface Accion {
  id: string
  /** Lo que se lee. */
  texto: string
  /** El grupo: «servidor», «app», «ver», «hacer». */
  grupo: string
  /** Lo que se enseña a la derecha: el servidor de una app, un atajo. */
  pista?: string
  /** Más palabras con las que se puede encontrar. */
  claves?: string
  hacer: () => void
}

/** Una coincidencia por subsecuencia, con premio a lo contiguo y al
 *  principio de palabra. `null` si no casa. Sin dependencias: son veinte
 *  líneas y una librería serían veinte mil. */
export function puntuar(consulta: string, texto: string): number | null {
  const q = consulta.trim().toLowerCase()
  if (q === '') return 0
  const t = texto.toLowerCase()
  let i = 0, punt = 0, seguidas = 0, previo = -2
  for (let j = 0; j < t.length && i < q.length; j++) {
    if (t[j] !== q[i]) continue
    seguidas = j === previo + 1 ? seguidas + 1 : 0
    punt += 1 + seguidas * 2 + (j === 0 || /[\s:/._-]/.test(t[j - 1]!) ? 3 : 0)
    previo = j
    i++
  }
  return i === q.length ? punt - t.length * 0.01 : null
}
