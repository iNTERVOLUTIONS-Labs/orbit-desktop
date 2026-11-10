<script lang="ts">
  // Una gráfica de barras en SVG, sin librería.
  //
  // El argumento que decidió no delegarlo es concreto: **tiene que saber pintar
  // un hueco**. Un `null` en una serie no es un cero — «no se sabe» y «no hubo»
  // son cosas distintas— y las librerías genéricas o interpolan por encima o lo
  // tratan como cero. Las dos cosas son mentiras, y son justo la clase de
  // mentira que este producto existe para no contar.

  const id = $props.id()

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

  // El resumen que se oye. Con `role="img"` los `<title>` de cada barra pasan a
  // ser decoración —el lector sólo dice el nombre de la imagen— y las cifras
  // sólo se veían pasando el ratón por encima. Lo que se dice aquí es lo que un
  // vistazo a la gráfica cuenta: dónde está el pico y cuántas horas no se saben.
  const resumen = $derived.by(() => {
    const conDato = datos.filter((d) => d.y !== null) as Array<{ x: string; y: number }>
    const huecos = datos.length - conDato.length
    if (conDato.length === 0) return `${etiqueta}: no se sabe ninguna hora.`
    const pico = conDato.reduce((a, d) => (d.y > a.y ? d : a))
    const sin = huecos === 0 ? '' : huecos === 1 ? ' Una hora no se sabe.' : ` ${huecos} horas no se saben.`
    return `${etiqueta}. Máximo: ${pico.y} a las ${pico.x}.${sin}`
  })
</script>

<!--
  Un solo nombre: antes iba en el `<figcaption>` oculto Y en el `aria-label`, y
  el lector lo decía dos veces seguidas. La imagen lleva el resumen; las cifras
  hora a hora van en una tabla plegada, que es la única forma de llegar a ellas
  con el teclado —el `<title>` de una barra sólo sale con el ratón encima.
-->
<!-- Un `<div>` y no un `<figure>`: un `<figure>` con su `<figcaption>` es otro
     nombre más, y el resumen se habría oído dos veces. -->
<div class="grafica">
  <svg viewBox="0 0 {ancho} {alto}" preserveAspectRatio="none" role="img" aria-label={etiqueta} aria-describedby="{id}-resumen">
    {#each datos as d, i (i)}
      {#if d.y === null}
        <!-- Un hueco: una marca tenue a media altura, ni barra ni vacío. Dejarlo
             en blanco lo haría indistinguible de un cero, que es la confusión
             que esto existe para evitar. -->
        <rect class="hueco" x={i + 0.15} y={alto / 2 - 1} width="0.7" height="2"><title>{d.x}: no se sabe</title></rect>
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
  <p id="{id}-resumen" class="sr">{resumen}</p>
  <details class="cifras">
    <summary>Ver las cifras</summary>
    <table>
      <caption class="sr">{etiqueta}</caption>
      <thead><tr><th scope="col">Hora</th><th scope="col" class="num">Valor</th></tr></thead>
      <tbody>
        {#each datos as d, i (i)}
          <!-- «no se sabe» y no un cero ni una celda vacía: es la misma
               distinción que el hueco hace en la gráfica. -->
          <tr><td>{d.x}</td><td class="num">{d.y === null ? 'no se sabe' : d.y}</td></tr>
        {/each}
      </tbody>
    </table>
  </details>
</div>

<style>
  svg { width: 100%; height: 96px; display: block; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .cifras { margin-top: var(--e-2); font-size: 12px; color: var(--fg-muted); }
  summary { cursor: pointer; width: fit-content; }
  summary:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; border-radius: var(--r-1); }
  table { border-collapse: collapse; margin-top: var(--e-2); font-variant-numeric: tabular-nums; }
  th, td { padding: 2px var(--e-3); text-align: left; border-bottom: 1px solid var(--border); }
  th { font-weight: 600; color: var(--fg-muted); }
  td { color: var(--fg); }
  .num { text-align: right; }
</style>
