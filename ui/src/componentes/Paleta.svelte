<script lang="ts">
  import { tick } from 'svelte'
  import { puntuar, type Accion } from '../lib/paleta'
  import { devolverFoco } from '../lib/foco'

  let {
    acciones,
    alCerrar,
  }: {
    acciones: Accion[]
    alCerrar: () => void
  } = $props()

  let consulta = $state('')
  let marcada = $state(0)
  let caja = $state<HTMLInputElement | null>(null)
  let lista = $state<HTMLUListElement | null>(null)
  // A dónde vuelve el foco al cerrar: al sitio desde el que se abrió.
  const previo = typeof document !== 'undefined' ? (document.activeElement as HTMLElement | null) : null

  const resultados = $derived.by(() => {
    const r: { a: Accion; p: number }[] = []
    for (const a of acciones) {
      const p = puntuar(consulta, `${a.texto} ${a.claves ?? ''} ${a.pista ?? ''}`)
      if (p !== null) r.push({ a, p })
    }
    if (consulta.trim() !== '') r.sort((x, y) => y.p - x.p)
    return r.slice(0, 50).map((x) => x.a)
  })

  $effect(() => {
    void consulta
    marcada = 0
  })

  $effect(() => {
    void tick().then(() => caja?.focus())
  })

  function cerrar() {
    alCerrar()
    devolverFoco(previo)
  }

  function elegir(a: Accion | undefined) {
    if (!a) return
    alCerrar()
    // Después de cerrar: la acción puede abrir otra hoja, y ésa se queda el
    // foco (`devolverFoco` no lo toca si hay un diálogo abierto).
    a.hacer()
    devolverFoco(previo)
  }

  async function teclas(e: KeyboardEvent) {
    // `stopPropagation` además de `preventDefault`: las hojas escuchan Escape
    // en la ventana, y uno solo cerraba las dos capas a la vez.
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); cerrar() }
    // Ctrl+K con la paleta abierta la cierra por el mismo camino, que es el que
    // devuelve el foco; sin esto lo cerraba la ventana y el foco se perdía.
    else if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) { e.preventDefault(); e.stopPropagation(); cerrar() }
    else if (e.key === 'ArrowDown') { e.preventDefault(); marcada = Math.min(resultados.length - 1, marcada + 1); await verMarcada() }
    else if (e.key === 'ArrowUp') { e.preventDefault(); marcada = Math.max(0, marcada - 1); await verMarcada() }
    else if (e.key === 'Enter') { e.preventDefault(); elegir(resultados[marcada]) }
    else if (e.key === 'Tab') { e.preventDefault() } // el foco no se escapa del diálogo
  }

  async function verMarcada() {
    await tick()
    lista?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: 'nearest' })
  }
</script>

<div class="velo" role="presentation" onclick={cerrar}></div>
<div class="paleta" role="dialog" aria-modal="true" aria-label="Paleta de órdenes">
  <input
    bind:this={caja}
    bind:value={consulta}
    class="consulta"
    type="text"
    placeholder="Busca un servidor, una app o qué hacer…"
    spellcheck="false"
    autocomplete="off"
    role="combobox"
    aria-expanded="true"
    aria-controls="paleta-lista"
    aria-activedescendant={resultados[marcada] ? `paleta-${resultados[marcada]!.id}` : undefined}
    onkeydown={teclas}
  />
  {#if resultados.length === 0}
    <p class="nada">Nada casa con «{consulta}».</p>
  {:else}
    <ul id="paleta-lista" class="lista" role="listbox" bind:this={lista} aria-label="Resultados">
      {#each resultados as a, i (a.id)}
        <li
          id="paleta-{a.id}"
          role="option"
          aria-selected={i === marcada}
          class="opcion"
          class:opcion--marcada={i === marcada}
          onmousemove={() => (marcada = i)}
          onmousedown={(e) => { e.preventDefault(); elegir(a) }}
        >
          <span class="grupo">{a.grupo}</span>
          <span class="texto">{a.texto}</span>
          {#if a.pista}<span class="pista mono">{a.pista}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  <p class="ayuda"><kbd>↑</kbd><kbd>↓</kbd> moverse · <kbd>↵</kbd> elegir · <kbd>esc</kbd> cerrar</p>
</div>

<style>
  .velo {
    position: fixed; inset: 0; z-index: 40;
    background: var(--velo);
    animation: aparecer var(--t-rapido) ease-out;
  }
  .paleta {
    position: fixed; z-index: 41; left: 50%; top: 14vh; transform: translateX(-50%);
    width: min(620px, calc(100vw - 32px));
    background: var(--surface); border: 1px solid var(--border-strong); border-radius: var(--r-3);
    box-shadow: var(--shadow-3); overflow: hidden;
    animation: entrar var(--t-normal) cubic-bezier(.2, .8, .2, 1);
  }
  .consulta {
    width: 100%; box-sizing: border-box; border: 0; border-bottom: 1px solid var(--border);
    padding: var(--e-4); font: inherit; font-size: 15px; background: transparent; color: var(--fg);
  }
  /* Un anillo hacia dentro: el borde de la caja es el de la paleta. */
  .consulta:focus { outline: none; }
  .consulta:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .lista { list-style: none; margin: 0; padding: var(--e-2); max-height: 50vh; overflow-y: auto; }
  .opcion {
    display: grid; grid-template-columns: 72px 1fr auto; gap: var(--e-3); align-items: baseline;
    padding: var(--e-2) var(--e-3); border-radius: var(--r-2); cursor: pointer; font-size: 13px; color: var(--fg);
  }
  .opcion--marcada { background: var(--surface-2); }
  .grupo { font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--fg-faint); }
  .pista { font-size: 11px; color: var(--fg-faint); }
  .mono { font-family: var(--mono); }
  .nada { margin: 0; padding: var(--e-4); font-size: 13px; color: var(--fg-muted); }
  .ayuda {
    margin: 0; padding: var(--e-2) var(--e-4); border-top: 1px solid var(--border);
    font-size: 11px; color: var(--fg-faint);
  }
  kbd {
    font-family: var(--mono); font-size: 10px; padding: 0 4px; margin-right: 2px;
    border: 1px solid var(--border-strong); border-radius: 3px;
  }
  @keyframes aparecer { from { opacity: 0 } to { opacity: 1 } }
  @keyframes entrar { from { opacity: 0; transform: translate(-50%, -6px) scale(.98) } to { opacity: 1; transform: translate(-50%, 0) scale(1) } }
  @media (prefers-reduced-motion: reduce) { .velo, .paleta { animation: none } }
</style>
