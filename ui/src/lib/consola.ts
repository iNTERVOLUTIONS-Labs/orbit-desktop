// La salida cruda de una orden larga: lo que se enseña en las consolas en vivo
// (un despliegue, «desplegar todo», `orbit new`, la instalación).
//
// **No es estado reactivo, a propósito.** La primera versión guardaba la salida
// como un `string` en `$state` y le sumaba cada línea: cada línea copiaba el
// texto entero, lo volvía a pintar en un `<pre>` y forzaba un reflujo para
// bajar al final. Medido en Chrome: 30 ms por línea al principio y 92 ms a las
// cinco mil — diez mil líneas de un build tardaban diez minutos en pintarse.
//
// Aquí se guardan las líneas en un anillo con tope, y quien pinta (`Consola`)
// se entera por una suscripción y junta todo lo que llegó en un fotograma.

/** Cuántas líneas se guardan. Un build ruidoso pasa de las diez mil; más allá
 *  de veinte mil, lo que se busca —el error— está al final. */
export const TOPE_DE_LINEAS = 20_000

/** Y cuántos caracteres en total, para que veinte mil líneas enormes no se
 *  coman la memoria (≈8 MB en UTF-16). */
export const TOPE_DE_CARACTERES = 4_000_000

/** Una línea suelta más larga que esto se recorta, **y se dice**: un
 *  JavaScript minificado volcado a stderr es una sola línea de megas. */
export const TOPE_DE_LINEA = 64_000

export class Salida {
  readonly tope: number
  readonly topeDeCaracteres: number
  /** El anillo: las líneas vivas son `#l[#ini..]`. Se compacta de vez en
   *  cuando en vez de quitar por delante en cada línea, que sería O(n). */
  #l: string[] = []
  #ini = 0
  #caracteres = 0
  #total = 0
  #vez = 0
  #oyentes = new Set<() => void>()

  constructor(tope = TOPE_DE_LINEAS, topeDeCaracteres = TOPE_DE_CARACTERES) {
    this.tope = Math.max(1, tope)
    this.topeDeCaracteres = Math.max(1, topeDeCaracteres)
  }

  /** Cuántas líneas han llegado desde el principio (o desde `vaciar`). */
  get total(): number {
    return this.#total
  }

  /** Cuántas se guardan ahora mismo. */
  get guardadas(): number {
    return this.#l.length - this.#ini
  }

  /** Las que llegaron y ya no se guardan, por el tope. */
  get perdidas(): number {
    return this.#total - this.guardadas
  }

  /** Cambia en cada `vaciar`: quien pinta sabe así que tiene que empezar de
   *  cero en vez de seguir añadiendo. */
  get vez(): number {
    return this.#vez
  }

  /** Una línea, sin el salto final. */
  anotar(linea: string): void {
    this.#meter(linea)
    this.#avisar()
  }

  /** Varias de golpe, con un solo aviso: el núcleo las manda en tandas. */
  anotarVarias(lineas: readonly string[]): void {
    if (lineas.length === 0) return
    for (const l of lineas) this.#meter(l)
    this.#avisar()
  }

  vaciar(): void {
    this.#l = []
    this.#ini = 0
    this.#caracteres = 0
    this.#total = 0
    this.#vez += 1
    this.#avisar()
  }

  /** Las últimas `n` guardadas, de la más vieja a la más nueva. */
  ultimas(n: number): string[] {
    const desde = Math.max(this.#ini, this.#l.length - Math.max(0, n))
    return this.#l.slice(desde)
  }

  /** Las guardadas entre `desde` y `hasta` (sin incluir), contando desde la
   *  más vieja que se guarda. */
  entre(desde: number, hasta: number): string[] {
    const g = this.guardadas
    const a = Math.max(0, Math.min(g, desde))
    const b = Math.max(a, Math.min(g, hasta))
    return this.#l.slice(this.#ini + a, this.#ini + b)
  }

  /** Todo lo guardado, como texto. Es lo que se pega en un issue. */
  texto(): string {
    const g = this.guardadas
    return g === 0 ? '' : this.#l.slice(this.#ini).join('\n') + '\n'
  }

  /** Avisa de cada cambio. Devuelve con qué darse de baja. El aviso es
   *  síncrono y barato a propósito: quien escucha decide cuándo pinta. */
  alCambiar(f: () => void): () => void {
    this.#oyentes.add(f)
    return () => this.#oyentes.delete(f)
  }

  #meter(cruda: string): void {
    let linea = typeof cruda === 'string' ? cruda : String(cruda)
    if (linea.length > TOPE_DE_LINEA) {
      linea = `${linea.slice(0, TOPE_DE_LINEA)} … [línea recortada: tenía ${linea.length} caracteres]`
    }
    this.#l.push(linea)
    this.#caracteres += linea.length
    this.#total += 1
    while (this.guardadas > 1 && (this.guardadas > this.tope || this.#caracteres > this.topeDeCaracteres)) {
      this.#caracteres -= this.#l[this.#ini]!.length
      this.#ini += 1
    }
    // Compactar cuando la parte muerta pasa de la viva: amortizado O(1).
    if (this.#ini > 1024 && this.#ini > this.guardadas) {
      this.#l = this.#l.slice(this.#ini)
      this.#ini = 0
    }
  }

  #avisar(): void {
    for (const f of this.#oyentes) f()
  }
}

/** Una salida ya terminada, a partir de un texto: para la galería y las
 *  pruebas, que pasan la salida entera de una vez. */
export function salidaDeTexto(texto: string): Salida {
  const s = new Salida()
  if (texto === '') return s
  const lineas = texto.split('\n')
  // El salto final no es una línea vacía más.
  if (lineas[lineas.length - 1] === '') lineas.pop()
  s.anotarVarias(lineas)
  return s
}
