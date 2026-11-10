<script lang="ts">
  import type { ErrorDelPuente } from '../lib/puente'
  import { consejoSsh } from '../lib/consejos'
  let { error, alias }: { error: ErrorDelPuente; alias: string } = $props()

  // La frase de arriba es NUESTRA y dice qué ha pasado. El mensaje del núcleo
  // va debajo, como cita: empieza en minúscula, a veces lleva pegado el texto
  // del sistema en inglés, y como frase principal de una pantalla se leía como
  // un error de programación. Como cita es lo que es: el detalle exacto, el
  // que alguien copiará en un buscador.
  const TITULO: Record<string, string> = {
    'orbit-no-esta': `No encuentro Orbit en ${alias}.`,
    'sudo-pide-clave': `El usuario de ${alias} necesita contraseña para sudo.`,
    orbit: `Orbit ha contestado con un error en ${alias}.`,
    tarde: `${alias} no ha contestado a tiempo.`,
    demasiado: 'La respuesta era demasiado grande y la he cortado.',
    'respuesta-sucia': 'La respuesta del servidor trae algo más que el objeto que esperaba.',
    json: 'No he entendido la respuesta del servidor.',
    forma: `La respuesta de ${alias} no tiene la forma que esta versión de Orbit Desktop entiende, así que no he enseñado nada de ella.`,
    'al-pintar': 'Esta vista no se ha podido pintar.',
    'sin-puente': 'Esto sólo funciona en la aplicación de escritorio.',
  }

  // El mensaje dice QUÉ ha pasado; esto dice qué HACER, y nada más. La primera
  // versión repetía el diagnóstico con otras palabras y quedaba a la vista en
  // la galería: dos frases seguidas diciendo lo mismo hacen que se lea la
  // primera y se ignore la segunda, que es justo donde está la acción.
  const QUE_HACER: Record<string, string> = {
    'orbit-no-esta': `Compruébalo con «ssh ${alias} which orbit».`,
    // Desde Orbit 1.4.6 hay una orden para esto, y la regla que pone es
    // exactamente la que hace falta: sin contraseña y sólo para el binario.
    // Se dice lo que es —equivale a root— porque Orbit también lo dice.
    'sudo-pide-clave':
      'Entra como root, o —con Orbit 1.4.6 o posterior— ejecuta una vez como root en el servidor «orbit admin add <usuario>». Equivale a darle root, igual que entrar como root.',
    // Neutro a propósito: `tarde` también sale de órdenes que CAMBIAN algo, y
    // ahí no se sabe si llegó a hacerse. Repetir a ciegas un despliegue o un
    // borrado porque «no contestó» es la forma de hacerlo dos veces.
    tarde:
      'Si era una orden que cambia algo, mira cómo ha quedado antes de repetirla; si era una consulta, vuelve a intentarlo o mira si el servidor está cargado.',
    demasiado: 'No es normal: una respuesta del contrato no llega a 20 KB.',
    forma:
      'Compara «orbit version» en el servidor con las versiones que admite esta aplicación (de la 1.3.6 a la 1.5.1) y actualiza la que se haya quedado atrás.',
    'al-pintar': 'Vuelve a intentarlo; si se repite, es un fallo de Orbit Desktop: copia el detalle en un issue.',
    'respuesta-sucia':
      'Suele ser un «echo» en el ~/.bashrc del servidor: haz que sólo salga en sesiones interactivas. No recorto lo que sobra a propósito: quedarse con el primer trozo que parezca válido es cómo se cuela una respuesta que no es la del servidor.',
  }

  // Los dos fallos de llegar. El consejo sale de leer lo que dijo ssh: antes
  // era uno solo para todos —«si eso también falla, no es cosa de Orbit
  // Desktop»—, que mandaba a la persona fuera justo cuando más ayuda
  // necesitaba, y no decía nada a quien no tiene ssh instalado.
  const ssh = $derived(
    error.clase === 'no-llego' || error.clase === 'no-se-pudo-lanzar'
      ? consejoSsh(`${error.mensaje}\n${error.detalle ?? ''}`, alias)
      : null,
  )
  // Un alias que no es ninguno de tus servidores tiene su clase: ahí no hay
  // respuesta del servidor ni versión que comparar. Se acepta también la frase
  // con `forma`, la de un núcleo anterior.
  const aliasDesconocido = $derived(
    error.clase === 'alias-desconocido'
      || (error.clase === 'forma' && /no es ninguno de tus servidores/.test(error.mensaje)),
  )
  const titulo = $derived(
    aliasDesconocido
      ? `«${alias}» no está entre tus servidores.`
      : (ssh?.que ?? TITULO[error.clase] ?? 'Algo ha fallado.'),
  )
  const hacer = $derived(
    aliasDesconocido
      ? 'Añádelo en «+ servidores», o elige uno de la lista.'
      : (ssh?.hacer ?? QUE_HACER[error.clase]),
  )
  // Cuando el título y el consejo ya lo dicen todo, la cita repetiría lo mismo
  // con otras palabras justo debajo.
  const citaRedundante = $derived(error.clase === 'sudo-pide-clave')
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
    <p class="mensaje">{titulo}</p>
    {#if hacer}
      <p class="que-hacer">{hacer}</p>
    {/if}
    {#if ssh}
      <!-- Lo que dijo ssh, plegado: en inglés y con su jerga, pero exacto. -->
      <details>
        <summary>Lo que dijo ssh</summary>
        <pre>{[error.mensaje, error.detalle].filter((x) => x && x.trim() !== '').join('\n\n')}</pre>
      </details>
    {:else}
      {#if !citaRedundante && error.mensaje.trim() !== ''}
        <p class="cita"><span class="et">El detalle:</span> {error.mensaje}</p>
      {/if}
      {#if error.detalle}
        <details>
          <summary>{error.clase === 'al-pintar' ? 'El detalle' : 'Lo que dijo el servidor'}</summary>
          <pre>{error.detalle}</pre>
        </details>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .fallo {
    border-radius: var(--r-3);
    /* El relleno lateral baja en estrecho: 24 px por lado se comían un
       séptimo de los 320 px que tiene que caber la pantalla. */
    padding: var(--e-4) clamp(var(--e-3), 4vw, var(--e-5));
    border: 1px solid var(--border);
    background: var(--surface);
  }
  h2 { margin: 0 0 var(--e-3); font-size: 16px; color: var(--fg); }
  .mensaje { margin: 0; color: var(--fg); font-size: 14px; }
  .que-hacer { margin: var(--e-3) 0 0; color: var(--fg-muted); font-size: 13px; max-width: 68ch; }
  .cita {
    margin: var(--e-3) 0 0; font-family: var(--mono); font-size: 12px;
    color: var(--fg-muted); overflow-wrap: anywhere; max-width: 80ch;
  }
  .cita .et { font-family: var(--fuente); }
  code, pre { font-family: var(--mono); font-size: 12px; }
  pre {
    margin: var(--e-3) 0 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    overflow-x: auto; color: var(--fg-muted); white-space: pre-wrap;
  }
  summary { cursor: pointer; color: var(--fg-muted); font-size: 13px; margin-top: var(--e-3); }
</style>
