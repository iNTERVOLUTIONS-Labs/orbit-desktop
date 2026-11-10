<script lang="ts">
  import { generar, problemaDeLaCsp } from '../lib/orbitjson'
  import { sabe, type AppInfo, type Entorno } from '../lib/contrato'

  let {
    app,
    info,
    entorno,
    version = null,
  }: {
    /** La versión de Orbit del servidor. Decide si se avisa de que la CSP no
     *  la leería: un servidor anterior a 1.5.1 ignora la clave sin decir nada. */
    version?: string | null
    app: string
    /** `null` mientras no ha llegado. El descriptor es lo único imprescindible:
     *  sin él no hay nada que generar. */
    info: AppInfo | null
    /** Los **nombres** de las variables, que es todo lo que el contrato deja
     *  pasar. `null` es «no los he pedido», y no una lista vacía. */
    entorno: Entorno | null
  } = $props()

  // La CSP no está en el descriptor —vive en el `orbit.json` de la release y
  // `info` no la devuelve—, así que es lo único que se escribe aquí a mano.
  let csp = $state('')
  let cspSoloInforme = $state(true)
  const motivoCsp = $derived(problemaDeLaCsp(csp))
  const conCsp = $derived(info !== null && info.config.type !== 'redirect')
  const cspSabida = $derived(sabe(version, 'csp'))

  const g = $derived(
    info === null ? null : generar(info, entorno?.keys ?? null, { csp, cspSoloInforme }),
  )

  let copiado = $state(false)
  let quitarCopiado: ReturnType<typeof setTimeout> | undefined
  async function copiar() {
    if (!g) return
    try {
      await navigator.clipboard.writeText(g.texto)
      copiado = true
      clearTimeout(quitarCopiado)
      quitarCopiado = setTimeout(() => (copiado = false), 1800)
    } catch {
      // Sin portapapeles el fichero sigue seleccionable con un clic.
    }
  }
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
    {#if conCsp}
      <fieldset class="csp">
        <legend>Content-Security-Policy <span class="desde">Orbit 1.5.1</span></legend>
        <label for="oj-csp">Política (opcional)</label>
        <input
          id="oj-csp"
          type="text"
          spellcheck="false"
          autocomplete="off"
          placeholder="default-src 'self'; img-src 'self' data:"
          bind:value={csp}
          aria-invalid={motivoCsp !== null}
          class:campo--mal={motivoCsp !== null}
          aria-describedby="oj-csp-ayuda"
        />
        <label class="casilla">
          <input type="checkbox" bind:checked={cspSoloInforme} />
          Sólo informar (<code>csp_report_only</code>): no bloquea nada, para probarla primero
        </label>
        <p id="oj-csp-ayuda" class="ayuda" class:dicho--mal={motivoCsp !== null}>
          {#if motivoCsp !== null}
            No vale: {motivoCsp}. Orbit la rechazaría y abortaría el despliegue.
          {:else if csp.trim() !== '' && !cspSabida}
            Este servidor {version ? `tiene Orbit ${version}` : 'no ha dicho su versión'}: hasta la
            1.5.1 la clave se ignora sin aviso.
          {:else}
            Sin comillas dobles, «$», llaves ni «;» al final. Se valida antes de tocar producción.
          {/if}
        </p>
      </fieldset>
    {/if}

    <div class="fichero-envoltorio">
      <pre class="fichero">{g.texto}</pre>
      <button type="button" class="copiar" onclick={copiar} aria-live="polite">
        {copiado ? '✓ copiado' : 'copiar'}
      </button>
    </div>

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
        {#each g.huecos as h, i (i)}<li>{h}</li>{/each}
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
  .fichero-envoltorio { position: relative; }
  .copiar {
    position: absolute; top: var(--e-2); right: var(--e-2);
    background: var(--surface); border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 11px; color: var(--fg-muted); cursor: pointer;
  }
  .copiar:hover { color: var(--fg); }
  .copiar:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .csp {
    margin: 0; padding: var(--e-3); display: grid; gap: var(--e-2);
    border: 1px solid var(--border); border-radius: var(--r-2);
  }
  .csp legend { padding: 0 var(--e-1); font-size: 12px; color: var(--fg); font-weight: 600; }
  .desde { font-weight: 400; color: var(--fg-faint); font-family: var(--mono); font-size: 11px; }
  .csp label { font-size: 12px; color: var(--fg-muted); }
  .csp input[type='text'] {
    font-family: var(--mono); font-size: 12px; padding: var(--e-2);
    background: var(--surface-sunken); color: var(--fg);
    border: 1px solid var(--border-strong); border-radius: var(--r-1);
  }
  .csp input[type='text']:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }

  .casilla { display: flex; gap: var(--e-2); align-items: center; }
  .ayuda { margin: 0; font-size: 12px; }
  .ayuda:not(.dicho--mal) { color: var(--fg-faint); }

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
