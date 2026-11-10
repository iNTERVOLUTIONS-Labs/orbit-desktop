<script lang="ts">
  // Una línea pequeña con lo que se ha visto en esta sesión.
  //
  // Dos reglas, las dos de la casa: un `null` es un HUECO —unirlo con sus
  // vecinos dibujaría una medida que nadie ha tomado—, y la figura nunca va
  // sola: lleva al lado la última cifra en texto, porque una línea sin escala
  // no le dice nada a un lector de pantalla ni a quien no distingue el color.
  let {
    valores,
    max = null,
    etiqueta,
    ancho = 96,
    alto = 22,
  }: {
    valores: readonly (number | null)[]
    /** El techo de la escala. `null` es «el mayor que se haya visto». */
    max?: number | null
    etiqueta: string
    ancho?: number
    alto?: number
  } = $props()

  const techo = $derived.by(() => {
    if (max !== null) return max
    let m = 0
    for (const v of valores) if (v !== null && v > m) m = v
    return m > 0 ? m : 1
  })

  /** Los tramos, cortados en cada `null`. */
  const tramos = $derived.by(() => {
    const n = valores.length
    if (n < 2) return [] as string[]
    const paso = ancho / (n - 1)
    const fuera: string[] = []
    let actual: string[] = []
    valores.forEach((v, i) => {
      if (v === null) {
        if (actual.length > 1) fuera.push(actual.join(' '))
        actual = []
        return
      }
      const x = (i * paso).toFixed(1)
      const y = (alto - 2 - (Math.min(v, techo) / techo) * (alto - 4)).toFixed(1)
      actual.push(`${actual.length === 0 ? 'M' : 'L'}${x},${y}`)
    })
    if (actual.length > 1) fuera.push(actual.join(' '))
    return fuera
  })

  const ultimo = $derived.by(() => {
    for (let i = valores.length - 1; i >= 0; i--) if (valores[i] !== null) return { i, v: valores[i]! }
    return null
  })
</script>

<svg
  class="tendencia"
  width={ancho}
  height={alto}
  viewBox="0 0 {ancho} {alto}"
  role="img"
  aria-label={etiqueta}
>
  {#if tramos.length === 0}
    <line x1="0" y1={alto - 2} x2={ancho} y2={alto - 2} class="base" />
  {/if}
  {#each tramos as d, i (i)}
    <path {d} class="linea" />
  {/each}
  {#if ultimo && valores.length > 1}
    <circle
      cx={(ultimo.i * ancho) / (valores.length - 1)}
      cy={alto - 2 - (Math.min(ultimo.v, techo) / techo) * (alto - 4)}
      r="2"
      class="punto"
    />
  {/if}
</svg>

<style>
  .tendencia { display: block; overflow: visible; }
  .base { stroke: var(--border); stroke-width: 1; stroke-dasharray: 2 3; }
  .linea {
    fill: none; stroke: var(--accent); stroke-width: 1.5;
    stroke-linejoin: round; stroke-linecap: round;
  }
  .punto { fill: var(--accent); }
</style>
