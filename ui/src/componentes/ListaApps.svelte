<script lang="ts">
  import ChipEstado from './ChipEstado.svelte'
  import FacetaSsl from './FacetaSsl.svelte'
  import { marcarInvisibles, nombreOperable, salud, num, type App } from '../lib/contrato'

  let {
    apps,
    servidor,
    seleccionada = null,
    alElegir = (_: App) => {},
  }: {
    apps: App[]
    servidor: string
    seleccionada?: string | null
    alElegir?: (a: App) => void
  } = $props()

  // La banda de resumen sale de mirar el banco de verdad: con 40 apps recién
  // creadas, TODAS salen `served:false`, y cuarenta chips rojos no dicen
  // «cuarenta problemas» — no dicen nada. Cuando algo le pasa a la mayoría, deja
  // de ser una excepción y hay que contarlo como un hecho del servidor.
  const sinVhost = $derived(apps.filter((a) => salud(a.state) === 'sin-vhost').length)
  const mayoria = $derived(apps.length > 0 && sinVhost / apps.length >= 0.7)

  // El nombre se recorta por el MEDIO y no por el final: `tienda-produccion` y
  // `tienda-staging` comparten prefijo, y truncar por el final las hace
  // idénticas en pantalla. El título lleva el nombre entero.
  function recorta(s: string, max = 28): string {
    if (s.length <= max) return s
    const izq = Math.ceil((max - 1) / 2)
    return s.slice(0, izq) + '…' + s.slice(s.length - (max - 1 - izq))
  }
</script>

{#if mayoria}
  <p class="banda" role="status">
    <strong>{sinVhost} de {apps.length} apps no tienen vhost.</strong>
    nginx no atiende sus dominios: la conexión se cierra, ni 404 ni 502.
    Suele arreglarlo <code>orbit doctor --fix</code>.
  </p>
{/if}

{#if apps.length === 0}
  <!-- Una colección vacía es una respuesta. Que no haya apps NO es un error, y
       decirlo así evita que se confunda con «el servidor no ha contestado», que
       sí lo es y se pinta en otro sitio. -->
  <p class="vacio">Este servidor no tiene ninguna app todavía.</p>
{:else}
  <table class="tabla">
    <caption class="sr">Apps de {servidor}</caption>
    <thead>
      <tr>
        <th scope="col">App</th>
        <th scope="col">Estado</th>
        <th scope="col">Dominio</th>
        <th scope="col">Tipo</th>
        <th scope="col" class="num">Puerto</th>
        <th scope="col">HTTPS</th>
        <th scope="col" class="num">Releases</th>
      </tr>
    </thead>
    <tbody>
      {#each apps as app (app.name)}
        {@const operable = nombreOperable(app.name)}
        <tr
          class:fila--no-operable={!operable}
          class:fila--activa={seleccionada === app.name}
          aria-selected={seleccionada === app.name}
        >
          <td class="celda-nombre">
            {#if operable}
              <button type="button" class="enlace" onclick={() => alElegir(app)}>
                <span title={app.name}>{recorta(app.name)}</span>
              </button>
            {:else}
              <!-- Un nombre que no pasa la regla de forma llega del servidor y
                   NO se puede operar. No se «arregla»: un nombre arreglado ya no
                   identifica a nadie. Y se pinta como texto, nunca como marcado:
                   `_j_str` no escapa `<` ni `>`, y hace bien — su trabajo es
                   producir JSON válido, no HTML seguro. -->
              <span
                title="Este nombre no tiene la forma que Orbit admite, así que no se puede operar sobre él desde aquí. Tal cual llegó: {marcarInvisibles(app.name)}"
                >{recorta(marcarInvisibles(app.name))}</span
              >
            {/if}
          </td>
          <td><ChipEstado estado={app.state} /></td>
          <td class="tenue" title={app.domain}>{app.domain}</td>
          <td class="tenue">{app.type}</td>
          <!-- `null` no es 0. Una web estática no tiene puerto, y el 0 sería un
               puerto. -->
          <td class="num mono">{num(app.state.port)}</td>
          <td><FacetaSsl estado={app.state} /></td>
          <td class="num mono">{num(app.state.releases)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabla th {
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: .04em;
    text-transform: uppercase;
    color: var(--fg-faint);
    padding: var(--e-2) var(--e-3);
    border-bottom: 1px solid var(--border);
  }
  .tabla td {
    padding: var(--e-2) var(--e-3);
    border-bottom: 1px solid var(--border);
    color: var(--fg);
  }
  .tabla tr:hover td { background: var(--surface-2); }
  .fila--activa td { background: var(--surface-2); }
  .num { text-align: right; }
  .mono { font-family: var(--mono); }
  .tenue { color: var(--fg-muted); }
  .enlace {
    background: none; border: 0; padding: 0;
    font: inherit; color: var(--accent-text);
    cursor: pointer; text-align: left;
  }
  .enlace:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; border-radius: var(--r-1); }
  .banda {
    margin: 0 0 var(--e-4);
    padding: var(--e-3) var(--e-4);
    border-radius: var(--r-2);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--fg);
    font-size: 13px;
  }
  .banda code { font-family: var(--mono); font-size: 12px; }
  .vacio { color: var(--fg-muted); font-size: 13px; }
  .sr {
    position: absolute; width: 1px; height: 1px;
    overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap;
  }
</style>
