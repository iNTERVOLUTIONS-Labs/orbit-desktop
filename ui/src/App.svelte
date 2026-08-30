<script lang="ts">
  import ListaApps from './componentes/ListaApps.svelte'
  import DetalleApp from './componentes/DetalleApp.svelte'
  import type { App, Lista } from './lib/contrato'

  import listaSana from './lib/muestras/list.json'
  import listaEstados from './lib/muestras/list-estados.json'
  import listaHostil from './lib/muestras/list-nombre-hostil.json'
  import listaVacia from './lib/muestras/list-vacia.json'

  // Fase 1 es «Ver» y todavía no hay transporte enganchado: estas muestras son
  // las MISMAS respuestas del servidor falso, no un JSON inventado a mano. Así
  // la interfaz y las pruebas del núcleo no pueden divergir del contrato.
  const escenarios: Record<string, Lista> = {
    'vps-ovh': listaSana as Lista,
    'pruebas': listaEstados as Lista,
    'comprometido': listaHostil as Lista,
    'recien-instalado': listaVacia as Lista,
  }

  let servidor = $state('vps-ovh')
  let elegida = $state<App | null>(null)

  const apps = $derived(escenarios[servidor]?.apps ?? [])

  function cambiarServidor(s: string) {
    servidor = s
    // La selección NO sobrevive a un cambio de servidor. `tienda` existe en
    // tres servidores y son apps distintas: conservar el nombre al cambiar
    // sería enseñar los datos de una bajo el nombre de otra.
    elegida = null
  }
</script>

<div class="marco">
  <!-- El servidor activo está SIEMPRE visible, no escondido en un desplegable
       que se lee al entrar. El accidente más caro de un cliente multiservidor
       no es un ataque: es ejecutar lo correcto contra el servidor equivocado. -->
  <nav class="rail" aria-label="Servidores">
    <p class="marca">Orbit</p>
    <ul>
      {#each Object.keys(escenarios) as s (s)}
        <li>
          <button
            type="button"
            class="servidor"
            class:servidor--activo={servidor === s}
            aria-current={servidor === s ? 'true' : undefined}
            onclick={() => cambiarServidor(s)}
          >{s}</button>
        </li>
      {/each}
    </ul>
  </nav>

  <main>
    <header class="cabecera">
      <h1>{servidor}</h1>
      <p class="cuenta">{apps.length} {apps.length === 1 ? 'app' : 'apps'}</p>
    </header>

    <section class="panel">
      <ListaApps
        {apps}
        {servidor}
        seleccionada={elegida?.name ?? null}
        alElegir={(a) => (elegida = a)}
      />
    </section>

    {#if elegida}
      <section class="panel panel--detalle">
        <DetalleApp app={elegida} {servidor} />
      </section>
    {/if}
  </main>
</div>

<style>
  .marco { display: grid; grid-template-columns: 200px 1fr; min-height: 100vh; }
  .rail {
    background: var(--surface-sunken);
    border-right: 1px solid var(--border);
    padding: var(--e-4) var(--e-3);
  }
  .marca {
    margin: 0 0 var(--e-5); padding-left: var(--e-2);
    font-weight: 700; letter-spacing: .12em; text-transform: uppercase;
    font-size: 12px; color: var(--accent-text);
  }
  .rail ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .servidor {
    width: 100%; text-align: left;
    background: none; border: 0; border-radius: var(--r-2);
    padding: var(--e-2) var(--e-2);
    font: inherit; font-size: 13px; color: var(--fg-muted); cursor: pointer;
  }
  .servidor:hover { background: var(--surface-2); color: var(--fg); }
  .servidor--activo { background: var(--surface-2); color: var(--fg); font-weight: 600; }
  .servidor:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  main { padding: var(--e-5) var(--e-6); background: var(--bg); }
  .cabecera { display: flex; align-items: baseline; gap: var(--e-3); margin-bottom: var(--e-5); }
  h1 { margin: 0; font-size: 20px; color: var(--fg); }
  .cuenta { margin: 0; color: var(--fg-faint); font-size: 13px; }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-1);
    padding: var(--e-4);
  }
  .panel--detalle { margin-top: var(--e-5); padding: 0; }
</style>
