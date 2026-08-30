<script lang="ts">
  import ListaApps from './componentes/ListaApps.svelte'
  import DetalleApp from './componentes/DetalleApp.svelte'
  import Esqueleto from './componentes/Esqueleto.svelte'
  import Fallo from './componentes/Fallo.svelte'
  import type { App } from './lib/contrato'
  import {
    hayPuente, portada, servidoresDelConfig,
    type AliasSsh, type ErrorDelPuente,
  } from './lib/puente'

  let servidores = $state<AliasSsh[]>([])
  let alias = $state('')
  let apps = $state<App[] | null>(null)
  let error = $state<ErrorDelPuente | null>(null)
  let cargando = $state(false)
  let elegida = $state<App | null>(null)

  async function cargar(a: string) {
    alias = a
    // La selección NO sobrevive a un cambio de servidor. `tienda` existe en
    // tres servidores y son apps distintas: conservar el nombre al cambiar
    // sería enseñar los datos de una bajo el nombre de otra, que es el
    // accidente más caro de un cliente multiservidor.
    elegida = null
    apps = null
    error = null
    cargando = true
    try {
      apps = (await portada(a)).apps
    } catch (e) {
      // Un fallo se pinta como un fallo. Lo que NO se hace es dejar los datos
      // viejos en pantalla sin decir que son viejos: un panel que dice «todo
      // verde» cuando no ha podido preguntar oculta un incidente en curso.
      error = e as ErrorDelPuente
    } finally {
      cargando = false
    }
  }

  $effect(() => {
    servidoresDelConfig().then((s) => {
      servidores = s
      if (s.length > 0 && !alias) cargar(s[0]!.alias)
    })
  })
</script>

<div class="marco">
  <!-- El servidor activo está SIEMPRE visible, no escondido en un desplegable
       que se lee al entrar. El accidente más caro de un cliente multiservidor
       no es un ataque: es ejecutar lo correcto contra el servidor equivocado.
       Hay precedente — la suite de pruebas de Orbit borró el vhost de una app
       de producción llamada `tienda` por eso mismo. -->
  <nav class="rail" aria-label="Servidores">
    <p class="marca">Orbit</p>
    <ul>
      {#each servidores as s (s.alias)}
        <li>
          <button
            type="button"
            class="servidor"
            class:servidor--activo={alias === s.alias}
            aria-current={alias === s.alias ? 'true' : undefined}
            title={s.hostname ? `${s.usuario ?? ''}@${s.hostname}:${s.puerto ?? 22}` : undefined}
            onclick={() => cargar(s.alias)}
          >
            {s.alias}
            {#if s.salto}
              <!-- Un salto se anuncia porque cambia lo que se puede prometer
                   sobre la latencia: el saludo se paga dos veces. -->
              <span class="salto" title="Por {s.salto}">↪</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if !hayPuente()}
      <p class="aviso">
        Sin envoltorio de escritorio: estás viendo las respuestas de ejemplo del
        servidor de pruebas, no un servidor de verdad.
      </p>
    {/if}
  </nav>

  <main>
    <header class="cabecera">
      <h1>{alias || '—'}</h1>
      {#if apps}
        <p class="cuenta">{apps.length} {apps.length === 1 ? 'app' : 'apps'}</p>
      {/if}
    </header>

    <section class="panel">
      {#if cargando}
        <Esqueleto />
      {:else if error}
        <Fallo {error} {alias} />
      {:else if apps}
        <ListaApps
          {apps}
          servidor={alias}
          seleccionada={elegida?.name ?? null}
          alElegir={(a) => (elegida = a)}
        />
      {/if}
    </section>

    {#if elegida}
      <section class="panel panel--detalle">
        <DetalleApp app={elegida} servidor={alias} />
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
    display: flex; flex-direction: column;
  }
  .marca {
    margin: 0 0 var(--e-5); padding-left: var(--e-2);
    font-weight: 700; letter-spacing: .12em; text-transform: uppercase;
    font-size: 12px; color: var(--accent-text);
  }
  .rail ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .servidor {
    width: 100%; text-align: left;
    display: flex; align-items: center; justify-content: space-between; gap: var(--e-2);
    background: none; border: 0; border-radius: var(--r-2);
    padding: var(--e-2);
    font: inherit; font-size: 13px; color: var(--fg-muted); cursor: pointer;
  }
  .servidor:hover { background: var(--surface-2); color: var(--fg); }
  .servidor--activo { background: var(--surface-2); color: var(--fg); font-weight: 600; }
  .servidor:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .salto { font-size: 11px; opacity: .7; }
  .aviso {
    margin-top: auto; padding: var(--e-3) var(--e-2) 0;
    font-size: 11px; line-height: 1.5; color: var(--fg-faint);
  }
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
