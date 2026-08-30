<script lang="ts">
  import ChipEstado from './ChipEstado.svelte'
  import FacetaSsl from './FacetaSsl.svelte'
  import { PRESENTACION, salud, num, type App } from '../lib/contrato'

  let { app, servidor }: { app: App; servidor: string } = $props()
  const s = $derived(salud(app.state))
</script>

<article class="detalle">
  <header>
    <!-- Siempre `servidor : app`, nunca la app sola. El nombre de una app no
         identifica nada por sí solo: `tienda` existe en tres servidores, y ese
         accidente tiene precedente — la suite de pruebas de Orbit borró el
         vhost de una app de producción llamada así. -->
    <p class="ruta"><span class="servidor">{servidor}</span> : <strong>{app.name}</strong></p>
    <ChipEstado estado={app.state} />
  </header>

  <!-- La frase, no sólo el color. El color dice que pasa algo; la frase dice
       qué, y es lo único accionable. -->
  <p class="frase">{PRESENTACION[s].frase}</p>

  <dl>
    <div><dt>Dominio</dt><dd>{app.domain}</dd></div>
    {#if app.aliases.length > 0}
      <div><dt>Alias</dt><dd>{app.aliases.join(' · ')}</dd></div>
    {/if}
    <div><dt>Tipo</dt><dd>{app.type}</dd></div>
    <div><dt>Puerto</dt><dd class="mono">{num(app.state.port)}</dd></div>
    <div><dt>HTTPS</dt><dd><FacetaSsl estado={app.state} /></dd></div>
    <div><dt>Releases</dt><dd class="mono">{num(app.state.releases)}</dd></div>
    <div>
      <dt>Último despliegue</dt>
      <!-- `null` es «nunca», y decirlo con la palabra es mejor que con un
           guion: un guion aquí se lee como «no lo sé». -->
      <dd class="mono">{app.state.last_deploy ?? 'nunca'}</dd>
    </div>
    <div>
      <dt>Autodespliegue</dt>
      <dd>{app.state.autodeploy ? 'activado' : 'desactivado'}</dd>
    </div>
  </dl>
</article>

<style>
  .detalle { padding: var(--e-5); }
  header { display: flex; align-items: center; gap: var(--e-3); margin-bottom: var(--e-2); }
  .ruta { margin: 0; font-size: 18px; color: var(--fg); }
  .servidor { color: var(--fg-muted); }
  .frase { margin: 0 0 var(--e-5); color: var(--fg-muted); font-size: 13px; max-width: 62ch; }
  dl { display: grid; grid-template-columns: 1fr; gap: var(--e-3); margin: 0; }
  dl > div { display: grid; grid-template-columns: 160px 1fr; gap: var(--e-3); align-items: baseline; }
  dt { color: var(--fg-faint); font-size: 12px; }
  dd { margin: 0; color: var(--fg); font-size: 13px; }
  .mono { font-family: var(--mono); }
</style>
