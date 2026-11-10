<script lang="ts">
  import ChipEstado from './ChipEstado.svelte'
  import FacetaSsl from './FacetaSsl.svelte'
  import Faceta from './Faceta.svelte'
  import { marcarInvisibles, nombreOperable, salud, num, PRESENTACION, type App, type Salud } from '../lib/contrato'

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
  // ── filtrar y moverse ──────────────────────────────────────────────────
  //
  // Con cuarenta apps, buscar una con el ratón es leer cuarenta filas. El
  // filtro casa con el nombre, el dominio y el tipo, y el resumen por estados
  // filtra con un clic. Lo que NO hace es esconder nada sin decirlo: la cuenta
  // de las que no se ven está siempre a la vista.
  let filtro = $state('')
  let soloEstado = $state<Salud | null>(null)
  let caja = $state<HTMLInputElement | null>(null)

  // En la tabla, «—» al lado de un dominio se entiende; suelto en un botón de
  // filtro, no. Aquí cada estado lleva su nombre.
  const ROTULO: Record<Salud, string> = {
    activo: 'activas', parado: 'paradas', 'no-aplica': 'sin proceso',
    mantenimiento: 'en mantenimiento', 'sin-vhost': 'sin vhost', desconocido: 'no se sabe',
  }

  const recuento = $derived.by(() => {
    const m = new Map<Salud, number>()
    for (const a of apps) {
      const s = salud(a.state)
      m.set(s, (m.get(s) ?? 0) + 1)
    }
    return [...m.entries()].sort((x, y) => y[1] - x[1])
  })

  const visibles = $derived.by(() => {
    const q = filtro.trim().toLowerCase()
    return apps.filter((a) => {
      if (soloEstado && salud(a.state) !== soloEstado) return false
      if (q === '') return true
      return a.name.toLowerCase().includes(q) || a.domain.toLowerCase().includes(q) || a.type.toLowerCase().includes(q)
    })
  })

  // Lo que dice la región viva al moverse con el teclado. Con el foco en la
  // caja de filtrar, la flecha cambia la app elegida sin mover el foco, y sin
  // esto el cambio sólo se veía.
  let anuncio = $state('')

  /** Arriba y abajo por las que se ven y se pueden operar. */
  function mover(d: 1 | -1, enfocar: boolean) {
    const ops = visibles.filter((a) => nombreOperable(a.name))
    if (ops.length === 0) return
    const i = ops.findIndex((a) => a.name === seleccionada)
    const j = i === -1 ? (d === 1 ? 0 : ops.length - 1) : Math.min(ops.length - 1, Math.max(0, i + d))
    const a = ops[j]!
    alElegir(a)
    anuncio = `${a.name}, ${j + 1} de ${ops.length}`
    // La fila se busca por su posición y no por un `id` hecho con el nombre:
    // `CSS.escape` es para selectores, no para ids, y con un nombre como
    // `1web` o `mi.web` el `getElementById` no encontraba nada y la fila
    // elegida se quedaba fuera de la vista. Además dos apps con el mismo
    // nombre —un servidor comprometido puede mandarlas— daban dos ids iguales.
    const fila = raiz?.querySelectorAll('tbody tr')[visibles.indexOf(a)]
    fila?.scrollIntoView?.({ block: 'nearest' })
    if (enfocar) (fila?.querySelector('button') as HTMLButtonElement | null)?.focus()
  }

  // ── El teclado: de quién es cada tecla ─────────────────────────────────
  //
  // La regla, en tres casos:
  //
  //  1. El foco está DENTRO de esta lista —la caja de filtrar, los botones de
  //     estado o una fila—: las flechas y `j`/`k` mueven la app elegida (en la
  //     caja, `j` y `k` se escriben, claro). Si el foco estaba en una fila, el
  //     foco va con la elegida, para que lo que se oye y lo que está elegido no
  //     sean dos filas distintas.
  //  2. No lo tiene NADIE —está en el `<body>`—: `j`, `k` y `/` siguen
  //     funcionando, porque no le quitan nada a nadie. Las flechas NO: en el
  //     `<body>` desplazan la página, y con el zoom al 200 % es la única forma
  //     de llegar a lo que no cabe.
  //  3. Lo tiene cualquier otra cosa —una pestaña del detalle, un desplegable,
  //     un `<summary>`, un registro que se desplaza—: la tecla es suya. Antes
  //     una flecha en la pestaña «log» saltaba a la app siguiente y devolvía la
  //     pestaña a «detalle», y una `j` en un botón hacía lo mismo.
  let raiz = $state<HTMLElement | null>(null)

  function teclas(e: KeyboardEvent) {
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return
    if (document.querySelector('[role="dialog"]')) return
    const t = e.target
    // La ventana o el documento cuentan como «nadie»: es a quien le llega la
    // tecla cuando no hay nada enfocado.
    // Y el título de la página, que es adonde se devuelve el foco al cerrar
    // una vista o una hoja cuyo botón ya no existe: sin esto, j/k dejaban de
    // funcionar hasta hacer clic en algún sitio.
    const enNingunSitio =
      !(t instanceof Element) || t === document.body || t === document.documentElement
      || (t instanceof HTMLHeadingElement && t.matches('main h1'))
    const dentro = t instanceof Node && !!raiz?.contains(t)
    if (!dentro && !enNingunSitio) return

    const flecha = e.key === 'ArrowDown' || e.key === 'ArrowUp'
    const abajo = e.key === 'ArrowDown' || e.key === 'j'
    const arriba = e.key === 'ArrowUp' || e.key === 'k'

    if (t === caja) {
      if (flecha) {
        e.preventDefault()
        mover(e.key === 'ArrowDown' ? 1 : -1, false)
      } else if (e.key === 'Escape') {
        // Vaciar y quedarse: quitarle el foco a la caja lo dejaba en el <body>.
        filtro = ''
      }
      return
    }
    if (e.key === '/') {
      e.preventDefault()
      caja?.focus()
      return
    }
    if (flecha && !dentro) return
    if (abajo || arriba) {
      e.preventDefault()
      mover(abajo ? 1 : -1, dentro)
    }
  }

  function recorta(s: string, max = 28): string {
    if (s.length <= max) return s
    const izq = Math.ceil((max - 1) / 2)
    return s.slice(0, izq) + '…' + s.slice(s.length - (max - 1 - izq))
  }
</script>

<svelte:window onkeydown={teclas} />

<div class="lista-apps" bind:this={raiz}>
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
  <div class="filtros">
    <input
      bind:this={caja}
      bind:value={filtro}
      type="search"
      class="buscar"
      placeholder="Filtrar por nombre, dominio o tipo   /"
      aria-label="Filtrar las apps de {servidor}"
      spellcheck="false"
      autocomplete="off"
    />
    <div class="recuento" role="group" aria-label="Filtrar por estado">
      {#each recuento as [s, n] (s)}
        <button
          type="button"
          class="estado-boton"
          class:estado-boton--activo={soloEstado === s}
          aria-pressed={soloEstado === s}
          onclick={() => (soloEstado = soloEstado === s ? null : s)}
        >
          <span class="chip chip--{s}" title={PRESENTACION[s].frase}><span class="chip__glifo" aria-hidden="true">{PRESENTACION[s].glifo}</span> {ROTULO[s]}</span>
          <span class="mono">{n}</span>
        </button>
      {/each}
    </div>
  </div>
  <!-- La región viva existe SIEMPRE y lo que cambia es su texto: una región
       que nace ya con el texto dentro no se anuncia, y el primer «2 de 40» al
       filtrar se perdía. -->
  <p class="ocultas" class:ocultas--con={visibles.length < apps.length} role="status">
    {#if visibles.length < apps.length}
      Se ven {visibles.length} de {apps.length}.
      <button type="button" class="enlace" onclick={() => { filtro = ''; soloEstado = null }}>Ver todas</button>
    {/if}
  </p>
  <p class="sr" role="status">{anuncio}</p>
  {#if visibles.length === 0}
    <p class="vacio">Ninguna app casa con el filtro.</p>
  {:else}
  <!-- La tabla se desplaza dentro de su marco y no arrastra la página: a 320 px
       de ancho siete columnas no caben, y lo que no puede pasar es que el
       filtro y el resto de la pantalla se vayan con ella. -->
  <div class="marco-tabla">
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
      <!-- Por posición y no por nombre: dos apps con el mismo nombre —que un
           servidor puede mandar, porque enumera ficheros sin filtrar— rompían la
           lista en desarrollo y se comían una fila en producción. -->
      {#each visibles as app, i (i)}
        {@const operable = nombreOperable(app.name)}
        <!-- `aria-current` y no `aria-selected`: en una tabla normal no hay
             selección que valga; `aria-selected` en un `<tr>` sólo existe
             dentro de un `grid`. -->
        <tr
          class:fila--no-operable={!operable}
          class:fila--activa={seleccionada === app.name}
          aria-current={seleccionada === app.name ? 'true' : undefined}
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
          <td>
            <ChipEstado estado={app.state} />
            {#if app.state.locked === true}
              <!-- Detrás del chip y como faceta, no en su lugar: una congelada
                   atiende igual, lo congelado son sus despliegues. Es lo que
                   hace `orbit list`, que lo pone al final de la fila. -->
              <Faceta
                texto="congelada"
                tono="aviso"
                titulo={app.state.lock?.reason
                  ? `Despliegues congelados: «${app.state.lock.reason}»`
                  : 'Despliegues congelados con «orbit lock»'}
              />
            {/if}
          </td>
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
  </div>
  {/if}
{/if}
</div>

<style>
  .filtros { display: flex; flex-wrap: wrap; gap: var(--e-3); align-items: center; margin-bottom: var(--e-3); }
  /* `min-width: 0` y una base que cabe en 320 px: con 240 px fijos más el
     relleno del panel, la caja empujaba la página de lado al 400 %. */
  .buscar {
    flex: 1 1 14rem; min-width: 0; max-width: 360px;
    font: inherit; font-size: 13px; padding: var(--e-2) var(--e-3);
    background: var(--surface-sunken); color: var(--fg);
    border: 1px solid var(--border); border-radius: var(--r-2);
    transition: border-color var(--t-rapido) ease-out;
  }
  .buscar:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; border-color: var(--border-strong); }
  .recuento { display: flex; flex-wrap: wrap; gap: var(--e-1); }
  .estado-boton {
    display: inline-flex; align-items: center; gap: var(--e-2);
    background: none; border: 1px solid transparent; border-radius: 999px;
    padding: 1px var(--e-2) 1px 1px; font: inherit; font-size: 12px; color: var(--fg-muted); cursor: pointer;
  }
  .estado-boton:hover { color: var(--fg); }
  .estado-boton--activo { border-color: var(--border-strong); color: var(--fg); }
  .estado-boton:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .ocultas { margin: 0; font-size: 12px; color: var(--fg-faint); }
  .ocultas--con { margin-bottom: var(--e-3); }
  .marco-tabla { overflow-x: auto; max-width: 100%; }
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
