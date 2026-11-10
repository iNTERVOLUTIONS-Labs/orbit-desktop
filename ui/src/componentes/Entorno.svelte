<script lang="ts">
  import { untrack } from 'svelte'
  import type { Entorno } from '../lib/contrato'

  let {
    entorno,
    app,
    servidor,
    pedirValor,
  }: {
    entorno: Entorno
    app: string
    servidor: string
    /** Cada revelación es **un `orbit env get` de verdad**, con su latencia.
     *  Es el mismo acto deliberado que el contrato exige por comando. */
    pedirValor: (clave: string) => Promise<string>
  } = $props()

  // UNO. No hay «revelar todo», y no es una función pendiente: enseñar el
  // `.env` entero es lo que convierte una captura de pantalla en una fuga.
  //
  // Por POSICIÓN y no por nombre: `orbit env list` no quita los repetidos, y
  // con la misma clave dos veces revelar una enseñaba el valor en las dos filas.
  let revelada = $state<number | null>(null)
  let valor = $state<string | null>(null)
  let pidiendo = $state(false)
  // Que no se pueda leer un valor es un dato, y se dice al lado de su clave:
  // antes el fallo se tragaba y el botón volvía como si nada.
  let fallo = $state<{ fila: number; mensaje: string } | null>(null)
  let quedan = $state(0)

  /** Treinta segundos. Un secreto en pantalla es un secreto en cualquier
   *  captura, videollamada o mirada por encima del hombro que ocurra mientras
   *  está ahí. */
  const SEGUNDOS = 30

  // Cada petición lleva un número, y ocultar lo cambia. La respuesta que llega
  // con un número viejo se TIRA: antes, perder el foco mientras `orbit env get`
  // estaba en camino ocultaba lo que había —nada— y la respuesta llegaba
  // después y se pintaba igual, con la ventana en segundo plano, durante treinta
  // segundos. Es justo el caso de la videollamada que esto existe para cubrir.
  let peticion = 0

  function ocultar() {
    peticion++
    pidiendo = false
    revelada = null
    // Se borra de la memoria, no sólo de la vista: dejarlo en una variable
    // «oculta» es dejarlo en el volcado del día que la aplicación se caiga.
    valor = null
    quedan = 0
  }

  async function revelar(fila: number, clave: string) {
    if (revelada === fila) return ocultar()
    ocultar()
    const mia = peticion
    pidiendo = true
    fallo = null
    try {
      const v = await pedirValor(clave)
      // Entre pedirlo y recibirlo ha pasado algo —la ventana perdió el foco,
      // se cambió de pantalla, se pidió otro— o la ventana está escondida:
      // la respuesta no se pinta, y tampoco se guarda.
      if (mia !== peticion || document.visibilityState === 'hidden') return
      // Y la fila tiene que seguir siendo esa clave.
      if (entorno.keys[fila] !== clave) return
      valor = v
      revelada = fila
      quedan = SEGUNDOS
    } catch (e) {
      if (mia !== peticion) return
      fallo = { fila, mensaje: (e as { mensaje?: string })?.mensaje ?? String(e) }
    } finally {
      if (mia === peticion) pidiendo = false
    }
  }

  $effect(() => {
    if (revelada === null) return
    const t = setInterval(() => {
      quedan -= 1
      if (quedan <= 0) ocultar()
    }, 1000)
    return () => clearInterval(t)
  })

  // Al perder el foco la ventana. Es el caso de la videollamada compartida y el
  // del portátil abierto en una mesa, que son los reales.
  // Y al esconderse —minimizar, cambiar de escritorio—, que no siempre trae un
  // `blur` consigo.
  $effect(() => {
    const f = () => ocultar()
    const v = () => document.visibilityState === 'hidden' && ocultar()
    window.addEventListener('blur', f)
    document.addEventListener('visibilitychange', v)
    return () => {
      window.removeEventListener('blur', f)
      document.removeEventListener('visibilitychange', v)
    }
  })

  // Las claves que salen más de una vez. Se enseñan todas —esconder una sería
  // decidir por el servidor cuál vale— y se marcan.
  const repetidas = $derived.by(() => {
    const n = new Map<string, number>()
    for (const k of entorno.keys) n.set(k, (n.get(k) ?? 0) + 1)
    return new Set([...n].filter(([, c]) => c > 1).map(([k]) => k))
  })

  // Si llega otro entorno —↻, o alguien ha tocado el .env—, las filas pueden
  // ser otras: lo revelado se oculta. Se guardaba la posición, y tras recargar
  // con una variable nueva arriba el valor de B salía al lado del nombre de A.
  $effect(() => {
    void entorno
    untrack(() => ocultar())
  })

  // Y al desmontarse: cambiar de pantalla también oculta.
  $effect(() => () => ocultar())
</script>

<p class="intro">
  {entorno.keys.length} {entorno.keys.length === 1 ? 'variable' : 'variables'} en
  <code>{app}</code>. Orbit devuelve <strong>sólo los nombres</strong>; cada valor
  se pide por separado, y esa llamada se ve en el servidor.
</p>

{#if entorno.keys.length === 0}
  <p class="vacio">El <code>.env</code> de «{app}» está vacío.</p>
{:else}
  <ul class="claves">
    {#each entorno.keys as k, i (i)}
      <li class="fila">
        <code class="clave">{k}{#if repetidas.has(k)}<span class="repetida" title="Esta clave sale más de una vez en el .env de «{app}».">repetida</span>{/if}</code>
        {#if revelada === i}
          <!-- Seleccionable para copiar, y con el reloj a la vista: quien lo
               revela tiene que ver cuánto le queda, no descubrir que se ha ido. -->
          <code class="valor">{valor}</code>
          <span class="reloj" aria-live="off">{quedan} s</span>
        {:else}
          <span class="oculto" aria-hidden="true">••••••••</span>
          <span></span>
        {/if}
        <button
          type="button"
          onclick={() => revelar(i, k)}
          disabled={pidiendo}
          aria-label={revelada === i ? `Ocultar ${k}` : `Revelar el valor de ${k} en ${servidor}`}
        >{revelada === i ? 'ocultar' : 'revelar'}</button>
        {#if fallo?.fila === i}
          <!-- Nuestra frase y, detrás, lo que dijo el núcleo como cita: empieza
               en minúscula y no es una frase de esta pantalla. -->
          <p class="fallo dicho--mal" role="alert">
            No he podido leer {k}. <span class="cita">{fallo.mensaje}</span>
          </p>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .intro { margin: 0 0 var(--e-4); font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .vacio { color: var(--fg-muted); font-size: 13px; }
  .claves { list-style: none; margin: 0; padding: 0; container-type: inline-size; }
  .fila {
    display: grid; grid-template-columns: minmax(0, 220px) 1fr 48px auto;
    gap: var(--e-3); align-items: center;
    padding: var(--e-2) 0; border-top: 1px solid var(--border);
  }
  /* En estrecho, la clave ocupa su propia línea y el valor, el reloj y el botón
     van debajo: cuatro columnas fijas no caben en 320 px. */
  @container (max-width: 26rem) {
    .fila { grid-template-columns: 1fr auto auto; row-gap: var(--e-1); }
    .clave { grid-column: 1 / -1; }
  }
  .repetida {
    margin-left: var(--e-2); font-family: var(--fuente); font-size: 11px;
    color: var(--fg-muted);
  }
  .cita { font-family: var(--mono); overflow-wrap: anywhere; }
  .fallo { grid-column: 1 / -1; margin: 0; font-size: 12px; }
  .clave { font-family: var(--mono); font-size: 12px; color: var(--fg); overflow-wrap: anywhere; }
  .valor {
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    user-select: all; word-break: break-all;
  }
  .oculto { font-family: var(--mono); color: var(--fg-faint); letter-spacing: .1em; }
  .reloj { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); text-align: right; }
  button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  button:hover:not(:disabled) { color: var(--fg); }
  button:disabled { opacity: .5; cursor: progress; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  code { font-family: var(--mono); }
</style>
