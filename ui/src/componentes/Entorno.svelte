<script lang="ts">
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
  let revelada = $state<string | null>(null)
  let valor = $state<string | null>(null)
  let pidiendo = $state(false)
  let quedan = $state(0)

  /** Treinta segundos. Un secreto en pantalla es un secreto en cualquier
   *  captura, videollamada o mirada por encima del hombro que ocurra mientras
   *  está ahí. */
  const SEGUNDOS = 30

  function ocultar() {
    revelada = null
    // Se borra de la memoria, no sólo de la vista: dejarlo en una variable
    // «oculta» es dejarlo en el volcado del día que la aplicación se caiga.
    valor = null
    quedan = 0
  }

  async function revelar(clave: string) {
    if (revelada === clave) return ocultar()
    ocultar()
    pidiendo = true
    try {
      valor = await pedirValor(clave)
      revelada = clave
      quedan = SEGUNDOS
    } finally {
      pidiendo = false
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
  $effect(() => {
    const f = () => ocultar()
    window.addEventListener('blur', f)
    return () => window.removeEventListener('blur', f)
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
    {#each entorno.keys as k (k)}
      <li class="fila">
        <code class="clave">{k}</code>
        {#if revelada === k}
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
          onclick={() => revelar(k)}
          disabled={pidiendo}
          aria-label={revelada === k ? `Ocultar ${k}` : `Revelar el valor de ${k} en ${servidor}`}
        >{revelada === k ? 'ocultar' : 'revelar'}</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .intro { margin: 0 0 var(--e-4); font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .vacio { color: var(--fg-muted); font-size: 13px; }
  .claves { list-style: none; margin: 0; padding: 0; }
  .fila {
    display: grid; grid-template-columns: 220px 1fr 48px auto;
    gap: var(--e-3); align-items: center;
    padding: var(--e-2) 0; border-top: 1px solid var(--border);
  }
  .clave { font-family: var(--mono); font-size: 12px; color: var(--fg); }
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
