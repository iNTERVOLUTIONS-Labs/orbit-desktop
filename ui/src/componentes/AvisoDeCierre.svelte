<script lang="ts">
  import { enCurso } from '../lib/vivos.svelte'

  // Cerrar la ventana **no cancela el despliegue**: el proceso remoto sigue.
  // Eso se dice al cerrar si hay uno vivo, porque lo contrario —dejar que
  // alguien crea que lo ha parado— es la clase de suposición que se descubre
  // media hora después.
  const cuantos = $derived(enCurso().length)
</script>

<!-- La región existe siempre y lo que aparece es su contenido: un nodo que
     nace ya con el texto dentro no se anuncia, y el aviso salía en silencio. -->
<div role="status">
  {#if cuantos > 0}
    <p class="aviso">
      {cuantos === 1 ? 'Hay un despliegue en curso' : `Hay ${cuantos} despliegues en curso`}.
      Cerrar esta ventana <strong>no {cuantos === 1 ? 'lo cancela' : 'los cancela'}</strong>:
      {cuantos === 1 ? 'sigue' : 'siguen'} en el servidor.
    </p>
  {/if}
</div>

<style>
  .aviso {
    margin: 0; padding: var(--e-2) var(--e-4);
    font-size: 12px; color: var(--fg-muted);
    background: var(--surface-2); border-bottom: 1px solid var(--border);
  }
</style>
