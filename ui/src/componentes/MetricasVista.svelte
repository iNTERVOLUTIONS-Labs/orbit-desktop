<script lang="ts">
  import { num, type Metricas } from '../lib/contrato'

  let { metricas }: { metricas: Metricas } = $props()

  function dur(s: number | null): string {
    if (s === null) return '·'
    return s < 60 ? `${s} s` : `${Math.floor(s / 60)} min ${s % 60} s`
  }
</script>

<div class="cifras">
  <div class="cifra">
    <span class="n">{num(metricas.deploys)}</span>
    <span class="et">despliegues</span>
  </div>
  <div class="cifra">
    <span class="n">{num(metricas.failed)}</span>
    <span class="et">fallidos</span>
  </div>
  <div class="cifra">
    <!-- La MEDIANA, no la media. Un build que una vez tardó 400 s no describe
         ningún despliegue real, y la media se lo lleva todo. -->
    <span class="n">{dur(metricas.build_median_s)}</span>
    <span class="et">build, mediana</span>
  </div>
</div>

<h3>Tendencia</h3>
{#if metricas.build_trend_s === null}
  <!--
    Orbit se calla la tendencia con menos de seis builds, porque dos datos no
    son una tendencia y fingirla es peor que no tenerla. Aquí NO se pinta una
    flecha plana para rellenar el hueco: una flecha plana afirma «no cambia», y
    eso es justo lo que no se sabe.
  -->
  <p class="sin-datos">
    Todavía no hay suficientes despliegues para decir si va a peor o a mejor.
    Orbit se calla con menos de seis, y aquí no se rellena el hueco.
  </p>
{:else}
  <p class="tendencia" class:tendencia--peor={metricas.build_trend_s > 0}>
    {#if metricas.build_trend_s > 0}
      El build tarda <strong>{dur(metricas.build_trend_s)}</strong> más que antes.
    {:else if metricas.build_trend_s < 0}
      El build tarda <strong>{dur(Math.abs(metricas.build_trend_s))}</strong> menos que antes.
    {:else}
      El build tarda lo mismo que antes.
    {/if}
  </p>
{/if}

{#if metricas.last}
  <p class="pie">Último despliegue: <code>{metricas.last}</code>.</p>
{/if}

<style>
  .cifras { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--e-3); }
  .cifra { padding: var(--e-3); background: var(--surface-2); border-radius: var(--r-2); }
  .n { display: block; font-size: 22px; font-weight: 700; color: var(--fg); font-family: var(--mono); }
  .et { display: block; font-size: 11px; color: var(--fg-muted); margin-top: 2px; }
  h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .04em;
       color: var(--fg-faint); margin: var(--e-5) 0 var(--e-2); }
  .tendencia { margin: 0; font-size: 13px; color: var(--fg); }
  .sin-datos { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .pie { margin: var(--e-4) 0 0; font-size: 12px; color: var(--fg-faint); }
  code { font-family: var(--mono); font-size: 12px; }
</style>
