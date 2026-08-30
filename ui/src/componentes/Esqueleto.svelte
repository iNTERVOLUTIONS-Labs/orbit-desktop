<script lang="ts">
  // Un esqueleto y no un spinner. Un spinner dice «espera» y no dice cuánto;
  // el esqueleto dice **qué** va a aparecer, y eso hace que la espera se sienta
  // más corta aunque dure lo mismo.
  //
  // Los umbrales salen de lo medido, no de la intuición: la portada cuesta
  // ~389 ms en el servidor más el viaje —unos 400 ms en caliente con
  // multiplexado, 560-740 en frío— así que hay tiempo de sobra para que el
  // esqueleto se vea, y no aparece antes de 150 ms para no parpadear en las
  // cargas rápidas.
  let { filas = 6 }: { filas?: number } = $props()
</script>

<div class="esqueleto" aria-hidden="true">
  {#each Array(filas) as _, i (i)}
    <div class="fila"></div>
  {/each}
</div>
<p class="sr" role="status">Cargando…</p>

<style>
  .esqueleto { display: grid; gap: var(--e-2); animation: aparecer .1s .15s both; }
  /* `--surface-2` sobre `--surface` da casi el mismo color en oscuro —#16203C
     sobre #10182F— así que las filas estaban ahí y no se veían: un esqueleto
     invisible es una caja vacía, que es peor que un spinner. Se calcula desde
     el texto, que contrasta con el fondo por definición en los dos temas. */
  .fila {
    height: 32px;
    border-radius: var(--r-2);
    background: color-mix(in srgb, var(--fg) 9%, var(--surface));
  }
  @keyframes aparecer { from { opacity: 0 } to { opacity: 1 } }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
