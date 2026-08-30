<script lang="ts">
  // Una gráfica de barras en SVG, sin librería.
  //
  // El argumento que decidió no delegarlo es concreto: **tiene que saber pintar
  // un hueco**. Un `null` en una serie no es un cero — «no se sabe» y «no hubo»
  // son cosas distintas— y las librerías genéricas o interpolan por encima o lo
  // tratan como cero. Las dos cosas son mentiras, y son justo la clase de
  // mentira que este producto existe para no contar.

  let {
    datos,
    etiqueta,
    alto = 96,
  }: {
    /** `null` es «no se sabe». `0` es «no hubo», y se pinta distinto. */
    datos: Array<{ x: string; y: number | null }>
    etiqueta: string
    alto?: number
  } = $props()

  const maximo = $derived(Math.max(1, ...datos.map((d) => d.y ?? 0)))
  const ancho = $derived(Math.max(1, datos.length))
</script>

<figure>
  <figcaption class="sr">{etiqueta}</figcaption>
  <svg viewBox="0 0 {ancho} {alto}" preserveAspectRatio="none" role="img" aria-label={etiqueta}>
    {#each datos as d, i (d.x)}
      {#if d.y === null}
        <!-- Un hueco: una marca tenue a media altura, ni barra ni vacío. Dejarlo
             en blanco lo haría indistinguible de un cero, que es la confusión
             que esto existe para evitar. -->
        <rect class="hueco" x={i + 0.15} y={alto / 2 - 1} width="0.7" height="2" />
      {:else}
        <rect
          class="barra"
          x={i + 0.15}
          width="0.7"
          y={alto - (d.y / maximo) * alto}
          height={Math.max(d.y === 0 ? 0 : 1, (d.y / maximo) * alto)}
        ><title>{d.x}: {d.y}</title></rect>
      {/if}
    {/each}
  </svg>
</figure>

<style>
  figure { margin: 0; }
  svg { width: 100%; height: 96px; display: block; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
