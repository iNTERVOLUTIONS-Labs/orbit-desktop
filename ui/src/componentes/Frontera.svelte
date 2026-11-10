<script lang="ts">
  // Una frontera para los fallos al pintar.
  //
  // Sin ella, una excepción en una vista —un dato con una forma que nadie
  // esperaba, un descuido en un componente— no se queda en esa vista: tumba el
  // lote de actualizaciones entero, y la ventana se queda con media pantalla
  // vieja y el contador del rail congelado. Las respuestas ya se miran al
  // entrar (`validar` en el puente); esto es para lo que quede, y lo que haga
  // es **decir qué ha fallado y dejar el resto en pie**.
  import type { Snippet } from 'svelte'
  import Fallo from './Fallo.svelte'
  import type { ErrorDelPuente } from '../lib/puente'

  let {
    que,
    alReintentar = null,
    children,
  }: {
    /** Lo que hay dentro, dicho para una persona: «la lista de apps», «el
     *  tráfico de tienda». Es lo que se nombra en el fallo. */
    que: string
    /** Lo que hay que hacer antes de volver a pintar —volver a pedir el dato,
     *  normalmente—. Sin esto se vuelve a pintar lo mismo, que sirve cuando el
     *  fallo dependía de algo que ya ha cambiado. */
    alReintentar?: (() => void) | null
    children: Snippet
  } = $props()

  function anotar(e: unknown) {
    // A la consola entero, con su pila: en pantalla va una frase, y quien tenga
    // que arreglarlo necesita el resto.
    console.error(`[Frontera] al pintar ${que}:`, e)
  }

  /** El fallo con la forma de los del puente, para pintarlo con `Fallo`. Si ya
   *  la tiene —un error del puente que se ha colado hasta aquí— se respeta. */
  function comoFallo(e: unknown): ErrorDelPuente {
    if (typeof e === 'object' && e !== null && typeof (e as ErrorDelPuente).clase === 'string'
        && typeof (e as ErrorDelPuente).mensaje === 'string') {
      return e as ErrorDelPuente
    }
    const detalle = e instanceof Error ? e.message : typeof e === 'string' ? e : null
    return { clase: 'al-pintar', mensaje: `No se ha podido pintar ${que}. El resto de la ventana sigue funcionando.`, detalle }
  }
</script>

<svelte:boundary onerror={anotar}>
  {@render children()}

  {#snippet failed(error, reset)}
    <div class="frontera">
      <Fallo error={comoFallo(error)} alias="" />
      <button
        type="button"
        class="reintentar"
        onclick={() => {
          alReintentar?.()
          reset()
        }}
      >Volver a intentarlo</button>
    </div>
  {/snippet}
</svelte:boundary>

<style>
  .frontera { display: grid; gap: var(--e-3); justify-items: start; }
  .reintentar {
    font: inherit; font-size: 13px;
    padding: var(--e-2) var(--e-4);
    border-radius: var(--r-2);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--fg);
    cursor: pointer;
  }
</style>
