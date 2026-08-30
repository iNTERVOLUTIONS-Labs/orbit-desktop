<script lang="ts">
  // La pieza más característica del producto, y existe por tres motivos que no
  // son estéticos:
  //
  //  1. Es la **prueba visible** de que la aplicación sólo invoca `orbit`, que
  //     es la promesa que la deja existir. Un panel web de los que Orbit
  //     rechaza no podría enseñar esto nunca.
  //  2. Enseña la CLI mientras se usa el ratón. El usuario objetivo ya sabe
  //     terminal: el día que tenga que hacerlo por SSH, sabrá cómo.
  //  3. Convierte «¿qué va a pasar?» en algo que se lee, no que se supone.

  let {
    titulo,
    servidor,
    orden,
    consecuencia,
    verbo,
    peligrosa = false,
    /** Cuando es peligrosa, hay que escribir esto para poder seguir. La
     *  fricción es un recurso escaso: se gasta donde el daño es irreversible y
     *  en ningún otro sitio. Si se pide en todo, se teclea sin leer. */
    confirmarEscribiendo = null,
    alConfirmar,
    alCancelar,
  }: {
    titulo: string
    servidor: string
    orden: string
    consecuencia: string
    verbo: string
    peligrosa?: boolean
    confirmarEscribiendo?: string | null
    alConfirmar: () => void
    alCancelar: () => void
  } = $props()

  let escrito = $state('')
  // Se compara literalmente: sin recortar espacios ni normalizar mayúsculas.
  // «Arreglar» lo que alguien escribe sería aceptar algo que no escribió.
  const listo = $derived(confirmarEscribiendo === null || escrito === confirmarEscribiendo)

  let dialogo = $state<HTMLDivElement | null>(null)
  $effect(() => {
    // El foco entra en el diálogo, pero NO en el botón de confirmar: un
    // destructivo cuyo primer anuncio es «Botón: Eliminar» es una trampa.
    dialogo?.focus()
  })

  function teclado(e: KeyboardEvent) {
    if (e.key === 'Escape') alCancelar()
  }
</script>

<svelte:window onkeydown={teclado} />

<div class="velo">
  <div
    class="hoja"
    class:hoja--peligrosa={peligrosa}
    role="dialog"
    aria-modal="true"
    aria-labelledby="hoja-titulo"
    tabindex="-1"
    bind:this={dialogo}
  >
    <!-- El servidor va en el TÍTULO, no en la letra pequeña. `tienda` existe en
         tres servidores, y el accidente más caro de un cliente multiservidor no
         es un ataque: es ejecutar lo correcto contra el equivocado. -->
    <h2 id="hoja-titulo">{titulo} en <strong>{servidor}</strong></h2>

    <p class="intro">Se va a ejecutar:</p>
    <!-- La orden literal, monoespaciada y seleccionable. -->
    <pre class="orden">{orden}</pre>

    <p class="consecuencia">{consecuencia}</p>

    {#if confirmarEscribiendo !== null}
      <label class="escribir">
        Escribe <code>{confirmarEscribiendo}</code> para confirmar:
        <input type="text" bind:value={escrito} autocomplete="off" spellcheck="false" />
      </label>
    {/if}

    <div class="acciones">
      <button type="button" class="secundaria" onclick={alCancelar}>Cancelar</button>
      <!-- Nunca es el botón por defecto y nunca tiene el foco al abrir. -->
      <button type="button" class="confirmar" disabled={!listo} onclick={alConfirmar}>
        {verbo}
      </button>
    </div>
  </div>
</div>

<style>
  .velo {
    position: fixed; inset: 0; display: grid; place-items: center;
    background: var(--velo); z-index: 10; padding: var(--e-5);
  }
  .hoja {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--r-4); box-shadow: var(--shadow-3);
    padding: var(--e-5); max-width: 620px; width: 100%;
  }
  .hoja:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  h2 { margin: 0 0 var(--e-4); font-size: 16px; color: var(--fg); }
  .intro { margin: 0 0 var(--e-2); font-size: 13px; color: var(--fg-muted); }
  .orden {
    margin: 0 0 var(--e-4); padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 13px; color: var(--fg);
    white-space: pre-wrap; word-break: break-all; user-select: all;
  }
  .consecuencia { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 62ch; }
  .escribir { display: block; margin-top: var(--e-4); font-size: 13px; color: var(--fg); }
  .escribir code { font-family: var(--mono); }
  .escribir input {
    display: block; width: 100%; margin-top: var(--e-2);
    padding: var(--e-2); border-radius: var(--r-2);
    border: 1px solid var(--border-strong); background: var(--surface-2);
    font-family: var(--mono); font-size: 13px; color: var(--fg);
  }
  .acciones { display: flex; justify-content: flex-end; gap: var(--e-2); margin-top: var(--e-5); }
  .secundaria, .confirmar {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-4);
    font: inherit; font-size: 13px; cursor: pointer;
  }
  .secundaria { background: none; color: var(--fg); border: 1px solid var(--border-strong); }
  .confirmar { background: var(--accent-fill); color: var(--on-accent); border: 0; font-weight: 600; }
  .confirmar:disabled { opacity: .45; cursor: not-allowed; }
  .secundaria:focus-visible, .confirmar:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
