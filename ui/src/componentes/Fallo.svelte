<script lang="ts">
  import type { ErrorDelPuente } from '../lib/puente'
  let { error, alias }: { error: ErrorDelPuente; alias: string } = $props()

  // El mensaje dice QUÉ ha pasado; esto dice qué HACER, y nada más. La primera
  // versión repetía el diagnóstico con otras palabras y quedaba a la vista en
  // la galería: dos frases seguidas diciendo lo mismo hacen que se lea la
  // primera y se ignore la segunda, que es justo donde está la acción.
  const QUE_HACER: Record<string, string> = {
    'orbit-no-esta': `Compruébalo con «ssh ${alias} which orbit».`,
    'sudo-pide-clave':
      'Entra como root, o dale a ese usuario sudo sin contraseña para /usr/local/bin/orbit.',
    'no-llego': `Prueba «ssh ${alias}» desde tu terminal: si eso también falla, no es cosa de Orbit Desktop.`,
    tarde: 'Vuelve a intentarlo, o mira si el servidor está cargado.',
    demasiado: 'No es normal: una respuesta del contrato no llega a 20 KB.',
    'respuesta-sucia':
      'No recorto lo que sobra a propósito: quedarse con el primer trozo que parezca válido es cómo se cuela una respuesta que no es la del servidor.',
  }
</script>

{#if error.clase === 'clave-de-host-cambiada'}
  <!--
    Su propia pantalla, sin botón de continuar. No es un problema de red: es
    exactamente lo que se ve en un ataque de suplantación. Cambiar la clave de
    un host es raro —una reinstalación, una migración— y siempre lo sabe quien
    lo hizo; un ataque, no. La única salida es editar `~/.ssh/known_hosts` a
    mano, fuera de esta ventana, y es deliberadamente incómodo.
  -->
  <section class="fallo aviso-alto" role="alert">
    <h2>La clave de <strong>{alias}</strong> ha cambiado</h2>
    <p>
      Puede ser una reinstalación o una migración. También puede ser que estés
      hablando con <strong>otra máquina</strong>.
    </p>
    <p class="que-hacer">
      Orbit Desktop no va a seguir por su cuenta. Comprueba la huella por otro
      canal —la consola de tu proveedor— y, si es legítima, quita la línea vieja
      de <code>~/.ssh/known_hosts</code> desde tu terminal.
    </p>
    {#if error.detalle}
      <pre>{error.detalle}</pre>
    {/if}
  </section>
{:else}
  <section class="fallo" role="alert">
    <p class="mensaje">{error.mensaje}</p>
    {#if QUE_HACER[error.clase]}
      <p class="que-hacer">{QUE_HACER[error.clase]}</p>
    {/if}
    {#if error.detalle}
      <details>
        <summary>Lo que dijo el servidor</summary>
        <pre>{error.detalle}</pre>
      </details>
    {/if}
  </section>
{/if}

<style>
  .fallo {
    border-radius: var(--r-3);
    padding: var(--e-4) var(--e-5);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  h2 { margin: 0 0 var(--e-3); font-size: 16px; color: var(--fg); }
  .mensaje { margin: 0; color: var(--fg); font-size: 14px; }
  .que-hacer { margin: var(--e-3) 0 0; color: var(--fg-muted); font-size: 13px; max-width: 68ch; }
  code, pre { font-family: var(--mono); font-size: 12px; }
  pre {
    margin: var(--e-3) 0 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    overflow-x: auto; color: var(--fg-muted); white-space: pre-wrap;
  }
  summary { cursor: pointer; color: var(--fg-muted); font-size: 13px; margin-top: var(--e-3); }
</style>
