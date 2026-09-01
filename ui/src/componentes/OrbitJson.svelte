<script lang="ts">
  import { generar } from '../lib/orbitjson'
  import type { AppInfo, Entorno } from '../lib/contrato'

  let {
    app,
    info,
    entorno,
  }: {
    app: string
    /** `null` mientras no ha llegado. El descriptor es lo único imprescindible:
     *  sin él no hay nada que generar. */
    info: AppInfo | null
    /** Los **nombres** de las variables, que es todo lo que el contrato deja
     *  pasar. `null` es «no los he pedido», y no una lista vacía. */
    entorno: Entorno | null
  } = $props()

  const g = $derived(info === null ? null : generar(info, entorno?.keys ?? null))
</script>

<section class="oj">
  <p class="intro">
    El <code>orbit.json</code> que reproduce cómo está configurada «{app}»
    <strong>ahora mismo en este servidor</strong>. Súbelo al repositorio y a
    partir de entonces manda el fichero: el descriptor pisa a la detección.
  </p>

  <!--
    Por qué esto existe habiendo `orbit init`, dicho donde se decide usarlo.
    `orbit init` corre la detección sobre el repositorio, o sea exactamente lo
    mismo que se equivocó la primera vez. Esto lee una app que ya funciona.
  -->
  <p class="intro">
    <code>orbit init</code> hace algo parecido, pero <strong>volviendo a
    detectar</strong> sobre tu copia del repositorio: si la detección se
    equivocó al crear la app, se equivoca otra vez igual. Esto no detecta nada —
    copia lo que ya está funcionando, incluidos los campos que arreglaste a mano.
  </p>

  {#if g === null}
    <p class="tenue">pidiendo el descriptor…</p>
  {:else}
    <pre class="fichero">{g.texto}</pre>

    <p class="claves">
      {g.claves.length}
      {g.claves.length === 1 ? 'clave' : 'claves'}: {g.claves.join(', ')}.
    </p>

    {#if g.huecos.length > 0}
      <!--
        Un fichero incompleto que no dice qué le falta es peor que ninguno: se
        sube tal cual y el hueco aparece tres despliegues más tarde.
      -->
      <ul class="huecos">
        {#each g.huecos as h (h)}<li>{h}</li>{/each}
      </ul>
    {/if}

    <!--
      Las tres advertencias que no salen del fichero sino del servidor que va a
      leerlo, y que sólo se pueden dar aquí: quien lo pegue en su repositorio ya
      no está mirando esta pantalla.
    -->
    <div class="avisos">
      <p>
        <strong>Sin <code>type</code>, Orbit ignora el fichero entero</strong> y
        vuelve a detectar como si no existiera. No es un aviso teórico: es la
        última línea de <code>_read_descriptor</code>.
      </p>
      <p>
        El servidor necesita <code>jq</code> para leerlo. Si no lo tiene, avisa y
        lo ignora — no falla, sigue con la detección.
      </p>
      <p>
        Una ruta que se salga del repositorio <strong>no se corrige, se
        ignora</strong>, y el despliegue lo dice. Aceptarla «como se pueda» sería
        publicar una carpeta que nadie ha elegido.
      </p>
    </div>

    <p class="sin-secretos">
      <!--
        La regla del contrato, dicha donde alguien podría esperar lo contrario:
        está mirando un fichero que habla de sus variables de entorno.
      -->
      El bloque <code>env</code> lleva <strong>nombres, no valores</strong>. Es
      una especificación —qué variables hacen falta y cómo obtenerlas— y no un
      almacén: los valores no cruzan el contrato, así que este fichero se puede
      subir a un repositorio público sin pensárselo.
    </p>
  {/if}
</section>

<style>
  .oj { display: grid; gap: var(--e-4); max-width: 84ch; }
  .intro { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 76ch; }
  .tenue { margin: 0; font-size: 12px; color: var(--fg-faint); }
  .fichero {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    white-space: pre; overflow-x: auto; user-select: all;
    max-height: 32rem; overflow-y: auto;
  }
  .claves { margin: 0; font-size: 12px; color: var(--fg-faint); }
  .huecos {
    margin: 0; padding-left: var(--e-4);
    font-size: 12px; color: var(--fg-muted); max-width: 76ch;
  }
  .avisos { display: grid; gap: var(--e-2); }
  .avisos p { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 76ch; }
  .sin-secretos {
    margin: 0; padding-left: var(--e-3);
    font-size: 12px; color: var(--fg-muted); max-width: 76ch;
  }
  code { font-family: var(--mono); font-size: 12px; }
</style>
