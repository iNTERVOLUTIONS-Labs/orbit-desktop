<script lang="ts">
  import { enCurso } from '../lib/vivos.svelte'

  // Cerrar la ventana **no cancela el despliegue**: el proceso remoto sigue.
  // Eso se dice al cerrar si hay uno vivo, porque lo contrario —dejar que
  // alguien crea que lo ha parado— es la clase de suposición que se descubre
  // media hora después.
  const cuantos = $derived(enCurso().length)
</script>

{#if cuantos > 0}
  <p class="aviso" role="status">
    {cuantos === 1 ? 'Hay un despliegue en curso' : `Hay ${cuantos} despliegues en curso`}.
    Cerrar esta ventana <strong>no los cancela</strong>: siguen en el servidor.
  </p>
{/if}

<style>
  .aviso {
    margin: 0; padding: var(--e-2) var(--e-4);
    font-size: 12px; color: var(--fg-muted);
    background: var(--surface-2); border-bottom: 1px solid var(--border);
  }
</style>
