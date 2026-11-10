<script lang="ts">
  import type { Log, Canal } from '../lib/contrato'

  let { log, app }: { log: Log; app: string } = $props()

  // El filtro por canal es la razón de ser de esta pantalla: la salida en prosa
  // mezcla el log de acceso con el de error sin decir cuál es cuál, y
  // distinguirlos es la primera pregunta de cualquiera que mira un log de
  // nginx. Aquí no sólo se distinguen: se pueden separar.
  let solo = $state<Canal | 'todo'>('todo')
  let busca = $state('')
  // Lo buscado se aplica cuando se deja de teclear. Filtrar y volver a partir
  // 1.500 filas en cada tecla costaba 51 ms la primera (87 ms con 50.000
  // líneas), y escribir «products» eran ocho de ésas seguidas.
  const ESPERA = 100
  let consulta = $state('')
  $effect(() => {
    const q = busca
    const t = setTimeout(() => (consulta = q), ESPERA)
    return () => clearTimeout(t)
  })

  /** Los canales con su nombre en la pantalla. El id de Orbit va en el
   *  `title`: es lo que sale en `orbit logs` y en un issue. */
  const NOMBRE_DE_CANAL: Record<Canal, string> = { journal: 'servicio', access: 'accesos', error: 'errores' }

  // Los recuentos, en UNA pasada. La primera versión recorría el log entero
  // cuatro veces por cada pintada —una por botón— y con miles de líneas se
  // notaba al escribir en el filtro.
  const cuentas = $derived.by(() => {
    const c: Record<Canal, number> = { journal: 0, access: 0, error: 0 }
    // `hasOwn`: el canal lo dice el servidor, y no puede alcanzar el prototipo.
    for (const l of log.lineas) if (Object.hasOwn(c, l.stream)) c[l.stream] += 1
    return c
  })

  // Los ÍNDICES de las que pasan el filtro, y no copias: cada fila se identifica
  // por su posición en el log. Con la posición en la lista filtrada, cada tecla
  // reescribía todas las filas en vez de quitar las que ya no pasan.
  const filtradas = $derived.by(() => {
    const q = consulta.trim().toLowerCase()
    const ls = log.lineas
    const fuera: number[] = []
    for (let i = 0; i < ls.length; i++) {
      const l = ls[i]!
      if ((solo === 'todo' || l.stream === solo) && (q === '' || l.text.toLowerCase().includes(q))) fuera.push(i)
    }
    return fuera
  })

  // Se pintan las ÚLTIMAS, que es donde está lo que se busca en un log, y se
  // dice cuántas quedan arriba. Pintar diez mil filas para mirar las veinte
  // de abajo congelaba la pestaña.
  const TRAMO = 1500
  let cuantas = $state(TRAMO)
  $effect(() => {
    void solo
    void consulta
    cuantas = TRAMO
  })
  const visibles = $derived(filtradas.length > cuantas ? filtradas.slice(filtradas.length - cuantas) : filtradas)
  const arriba = $derived(filtradas.length - visibles.length)

  /** El texto partido por lo buscado, para marcarlo **sin marcado**: cada
   *  trozo es texto, y el resaltado es un elemento. Nada del servidor se
   *  inserta como HTML. */
  function trozos(t: string): { t: string; m: boolean }[] {
    const q = consulta.trim()
    if (q === '') return [{ t, m: false }]
    const fuera: { t: string; m: boolean }[] = []
    const bajo = t.toLowerCase(), ql = q.toLowerCase()
    let i = 0
    for (;;) {
      const j = bajo.indexOf(ql, i)
      if (j === -1) { fuera.push({ t: t.slice(i), m: false }); break }
      if (j > i) fuera.push({ t: t.slice(i, j), m: false })
      fuera.push({ t: t.slice(j, j + q.length), m: true })
      i = j + q.length
    }
    return fuera
  }

  // Al llegar, abajo del todo: lo último es lo que importa.
  let caja = $state<HTMLOListElement | null>(null)
  $effect(() => {
    void log
    if (caja) caja.scrollTop = caja.scrollHeight
  })

  // La marca de tiempo se pinta corta, pero el título lleva la original entera:
  // el huso importa cuando se compara con otro servidor, y recortarlo sin
  // dejarlo a mano sería tirar el dato.
  function hora(ts: string | null): string {
    if (ts === null) return '·'
    const m = ts.match(/T(\d{2}:\d{2}:\d{2})/)
    return m ? m[1]! : ts
  }
</script>

<div class="barra">
  <div class="filtros" role="group" aria-label="Filtrar por origen">
    <button type="button" class:activo={solo === 'todo'} aria-pressed={solo === 'todo'} onclick={() => (solo = 'todo')}>
      todo <span class="cuenta">{log.lineas.length}</span>
    </button>
    {#each (['journal', 'access', 'error'] as Canal[]).filter((c) => cuentas[c] > 0) as c (c)}
      <button type="button" class:activo={solo === c} aria-pressed={solo === c} title={c} onclick={() => (solo = c)}>
        {NOMBRE_DE_CANAL[c]} <span class="cuenta">{cuentas[c]}</span>
      </button>
    {/each}
  </div>

  <input
    class="busca"
    type="search"
    bind:value={busca}
    placeholder="Buscar en el log"
    aria-label="Buscar en el log de {app}"
    spellcheck="false"
    autocomplete="off"
  />

  {#if log.meta?.follow}
    <span class="vivo">en vivo</span>
  {/if}
</div>

{#if log.truncado}
  <!-- Un número corto sin avisar se lee como «hay poco log», que suele ser lo
       contrario de lo que pasa. -->
  <p class="aviso">
    Se ha llegado al tope de líneas. Hay más log del que se ve: pide más con
    <code>--lines</code> o acota con una ventana de tiempo.
  </p>
{/if}

{#if log.rotas > 0}
  <!-- Se enseña. Si se callara, un log a medias se leería como uno completo. -->
  <p class="aviso">
    {log.rotas} {log.rotas === 1 ? 'línea no se ha entendido' : 'líneas no se han entendido'}
    y no se muestran.
  </p>
{/if}

{#if log.lineas.length === 0}
  <p class="vacio">
    «{app}» no tiene líneas en esta ventana. No es un error: puede que aún no
    haya recibido ninguna visita.
  </p>
{:else}
  <!-- `role="log"` con `aria-live="off"`: un log a cuarenta líneas por segundo
       leído en voz alta inutiliza la pantalla. Lo que se anuncia son las
       transiciones, no cada línea, y de eso se encarga otro sitio. -->
  {#if consulta.trim() !== '' || solo !== 'todo'}
    <p class="aviso" role="status">{filtradas.length} de {log.lineas.length} líneas.</p>
  {/if}
  {#if arriba > 0}
    <button type="button" class="mas" onclick={() => (cuantas += TRAMO)}>
      {arriba} más arriba · ver {Math.min(TRAMO, arriba)} más
    </button>
  {/if}
  <ol class="log" role="log" aria-live="off" aria-label="Log de {app}" bind:this={caja}>
    {#each visibles as i (i)}
      {@const l = log.lineas[i]!}
      <li class="linea log--{l.stream}">
        <time class="ts" title={l.ts ?? 'esta línea no lleva marca de tiempo'}>{hora(l.ts)}</time>
        <!-- El aspa y el color no se oyen: un lector de pantalla dice «error». -->
        <span class="canal"><span aria-hidden="true">{l.stream === 'error' ? '✕' : '·'}</span>{#if l.stream === 'error'}<span class="sr">error: </span>{/if}</span>
        <span class="texto">{#each trozos(l.text) as p, k (k)}{#if p.m}<mark>{p.t}</mark>{:else}{p.t}{/if}{/each}</span>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .barra { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--e-3); margin-bottom: var(--e-3); }
  .filtros { display: flex; gap: var(--e-1); }
  .filtros button {
    background: none; border: 1px solid transparent; border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .filtros button:hover { color: var(--fg); }
  .filtros button.activo { border-color: var(--border-strong); color: var(--fg); }
  .filtros button:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  /* Sin `opacity`: atenuado se quedaba en 3,46:1, por debajo del AA. */
  .cuenta { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .vivo { font-size: 11px; color: var(--fg-faint); }
  .aviso { margin: 0 0 var(--e-3); font-size: 12px; color: var(--fg-muted); }
  .aviso code { font-family: var(--mono); }
  .vacio { color: var(--fg-muted); font-size: 13px; }
  .log {
    list-style: none; margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; line-height: 1.7;
    max-height: 52vh; overflow: auto;
  }
  .linea {
    display: grid; grid-template-columns: 68px 12px 1fr; gap: var(--e-2);
    /* El navegador no compone lo que no se ve: con miles de filas, desplazarse
       deja de costar lo que cuesta el log entero. */
    content-visibility: auto; contain-intrinsic-size: auto 20px;
  }
  .busca {
    flex: 0 1 260px; min-width: 0; font: inherit; font-size: 12px; padding: 2px var(--e-2);
    background: var(--surface-sunken); color: var(--fg);
    border: 1px solid var(--border); border-radius: var(--r-1);
  }
  .busca:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .mas {
    margin: 0 0 var(--e-2); background: none; border: 0; padding: 0;
    font: inherit; font-size: 12px; color: var(--accent-text); cursor: pointer;
  }
  .mas:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  mark { background: var(--accent-fill); color: var(--on-accent); border-radius: 2px; padding: 0 1px; }
  .ts { color: var(--fg-faint); }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .texto { white-space: pre-wrap; word-break: break-word; color: var(--fg); }
</style>
