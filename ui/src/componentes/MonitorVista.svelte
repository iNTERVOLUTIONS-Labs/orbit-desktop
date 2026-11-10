<script lang="ts">
  import { bytes, marcarInvisibles, num, salud, PRESENTACION, type App, type Monitor, type Salud } from '../lib/contrato'
  import { serie } from '../lib/historial.svelte'
  import Tendencia from './Tendencia.svelte'

  let {
    monitor,
    servidor,
    periodo,
    pausado = false,
    alPausar = null,
    falloUltimo = null,
    portada = null,
  }: {
    monitor: Monitor
    servidor: string
    periodo: number
    pausado?: boolean
    /** Parar y seguir el sondeo. `null` donde no hay sondeo (la galería). */
    alPausar?: ((pausar: boolean) => void) | null
    /** Que la última vuelta NO contestó. Lo de la tabla es entonces la foto
     *  anterior, y se dice: un monitor que enseña datos viejos sin avisar es
     *  peor que uno vacío. */
    falloUltimo?: string | null
    /** Las apps de la portada (`status --json`), para saber si cada una tiene
     *  vhost o está en mantenimiento: `top --json` no lo dice. Sin ellas —o
     *  sin la app— el estado es «no se sabe», nunca uno inventado. */
    portada?: App[] | null
  } = $props()

  // Por nombre, y sólo los que no se repiten: con dos apps que se llaman igual
  // no se sabe de cuál es cada fila, y elegir una sería inventar.
  const porNombre = $derived.by(() => {
    const m = new Map<string, App | null>()
    for (const a of portada ?? []) m.set(a.name, m.has(a.name) ? null : a)
    return m
  })

  /** El porcentaje, o `null` si no es un número. Lo del puente ya viene
   *  validado; esto es para que ningún camino pinte «undefined %». */
  function pct(n: number | null | undefined): string | null {
    return typeof n === 'number' && Number.isFinite(n) ? `${n} %` : null
  }

  /** El estado de una fila: el proceso lo dice `top`, que es lo más fresco; el
   *  vhost y el mantenimiento, la portada. */
  function estadoDe(a: Monitor['apps'][number]): Salud {
    const p = porNombre.get(a.name)
    if (!p) return 'desconocido'
    return salud({ ...p.state, service: a.service })
  }

  // El orden se elige; por defecto, el del servidor, que es alfabético.
  let orden = $state<'nombre' | 'cpu' | 'memoria'>('nombre')
  const filas = $derived.by(() => {
    const a = [...monitor.apps]
    // Lo que no se sabe va al final, nunca como un cero.
    const clave = (x: (typeof a)[number]) =>
      orden === 'cpu' ? x.cpu_percent : orden === 'memoria' ? x.memory_bytes : null
    if (orden !== 'nombre') {
      a.sort((x, y) => {
        const p = clave(x), q = clave(y)
        if (p === null && q === null) return x.name.localeCompare(y.name)
        if (p === null) return 1
        if (q === null) return -1
        return q - p
      })
    }
    return a
  })
</script>

<div class="barra">
  <p class="intro">
    {monitor.apps.length} apps en <strong>{servidor}</strong>.
  </p>
  <!--
    El periodo se ENSEÑA, y no es un adorno.

    Medido: `orbit top --json` cuesta ~2,1 s con 40 apps, y no es lentitud — la
    CPU es la diferencia entre dos lecturas del cgroup, así que una foto suelta
    tiene que esperar a la segunda a propósito. El plan de refrescar cada dos
    segundos era físicamente imposible, y venía de copiar el intervalo del panel
    en vivo de Orbit sin ver que ése reutiliza la muestra anterior.

    Así que el periodo se adapta a lo que de verdad tarda, y se dice: una
    pantalla que promete «en vivo» y refresca cada cinco segundos está mintiendo
    por omisión.
  -->
  <div class="mandos">
    <p class="periodo">{pausado ? 'en pausa' : `cada ${periodo} s`}</p>
    {#if alPausar}
      <button type="button" class="mando" aria-pressed={pausado} onclick={() => alPausar!(!pausado)}>
        {pausado ? '▶ seguir' : '❚❚ pausar'}
      </button>
    {/if}
  </div>
</div>

{#if falloUltimo}
  <p class="viejo foto--vieja" role="status">
    La última vuelta no ha contestado ({falloUltimo}). Lo de abajo es la foto
    anterior; vuelvo a preguntar con menos prisa.
  </p>
{/if}

<table class="tabla">
  <caption class="sr">Consumo de las apps de {servidor}</caption>
  <thead>
    <tr>
      <th scope="col">App</th>
      <th scope="col">Estado</th>
      <th scope="col" class="num" aria-sort={orden === 'cpu' ? 'descending' : 'none'}>
        <button type="button" class="orden" onclick={() => (orden = orden === 'cpu' ? 'nombre' : 'cpu')}>CPU</button>
      </th>
      <th scope="col" class="tend"><span class="sr">Tendencia de la CPU en esta sesión</span></th>
      <th scope="col" class="num" aria-sort={orden === 'memoria' ? 'descending' : 'none'}>
        <button type="button" class="orden" onclick={() => (orden = orden === 'memoria' ? 'nombre' : 'memoria')}>Memoria</button>
      </th>
      <th scope="col" class="tend"><span class="sr">Tendencia de la memoria en esta sesión</span></th>
      <th scope="col" class="num">Peticiones / min</th>
    </tr>
  </thead>
  <tbody>
    <!-- Por posición y no por nombre: el nombre lo pone el servidor y se puede
         repetir, y una clave repetida revienta en desarrollo y se come filas
         en producción sin decir nada. -->
    {#each filas as a, i (i)}
      {@const h = serie(servidor, a.name)}
      {@const nombre = marcarInvisibles(a.name)}
      {@const e = estadoDe(a)}
      <tr>
        <td class="nombre">{nombre}</td>
        <td class="tenue" title={PRESENTACION[e].frase}>{PRESENTACION[e].texto}</td>
        <!--
          `cpu_percent` es `null` la primera vez, y eso es «no se sabe».
          Pintarlo como 0 % sería inventar una afirmación: un cero dice que la
          app no está consumiendo, y eso no se sabe todavía.
        -->
        <td class="num mono">{pct(a.cpu_percent) ?? '·'}</td>
        <td class="tend">
          {#if h}<Tendencia valores={h.cpu} max={100} etiqueta="CPU de {nombre}: {pct(a.cpu_percent) ?? 'no se sabe todavía'}" />{/if}
        </td>
        <td class="num mono">{bytes(a.memory_bytes)}</td>
        <td class="tend">
          {#if h}<Tendencia valores={h.memoria} etiqueta="Memoria de {nombre}: {bytes(a.memory_bytes)}" />{/if}
        </td>
        <td class="num mono">
          {num(a.requests_last_minute)}{#if a.requests_capped}<span
            class="tope"
            title="El minuto llenó el tope de líneas de log que se miran: hay más peticiones de las que se ven."
          >+</span>{/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .barra { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  .intro { margin: 0 0 var(--e-3); font-size: 13px; color: var(--fg-muted); }
  .periodo { margin: 0; font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .mandos { display: flex; align-items: center; gap: var(--e-3); }
  .mando, .orden {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 1px var(--e-2); font: inherit; font-size: 11px; color: var(--fg-muted); cursor: pointer;
  }
  .orden { border-color: transparent; padding: 0; font-weight: 600; letter-spacing: .04em; text-transform: uppercase; color: inherit; }
  .mando:hover, .orden:hover { color: var(--fg); }
  .mando:focus-visible, .orden:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .viejo {
    margin: 0 0 var(--e-3); padding: var(--e-2) var(--e-3);
    border-width: 1px; border-style: solid; border-radius: var(--r-2);
    font-size: 12px; color: var(--fg);
  }
  .tend { width: 104px; padding-left: var(--e-2) !important; }
  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabla th {
    text-align: left; font-size: 11px; font-weight: 600; letter-spacing: .04em;
    text-transform: uppercase; color: var(--fg-faint);
    padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border);
  }
  .tabla td { padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border); }
  .num { text-align: right; }
  .mono { font-family: var(--mono); }
  .nombre { font-family: var(--mono); font-size: 12px; }
  .tenue { color: var(--fg-muted); }
  .tope { color: var(--fg-faint); }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
