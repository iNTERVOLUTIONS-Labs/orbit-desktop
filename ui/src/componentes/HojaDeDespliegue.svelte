<script lang="ts">
  import { devolverFoco } from '../lib/foco'
  // La hoja de comando de un despliegue. Es la de siempre —la orden literal
  // antes de lanzarla— con las dos cosas que Orbit 1.3.8 y 1.4.3 añadieron al
  // despliegue y que sin terminal no tienen otra salida:
  //
  //  · **Las variables obligatorias que faltan.** Sin nadie al teclado, Orbit
  //    no puede preguntarlas: se para antes del build y la web se queda como
  //    estaba. Aquí se preguntan ANTES, y viajan con `--env`.
  //  · **La congelación.** `orbit deploy` se niega en una app congelada. Lo que
  //    se ofrece es «desplegar igualmente» (`--force`), que **no la
  //    descongela**: es un botón con su propio nombre, nunca una casilla al
  //    lado del de siempre.
  import {
    argvDeDespliegue, ordenLegible, problemaDelValor, type VariableNueva,
  } from '../lib/despliegue'
  import type { App, ComprobacionDeEntorno } from '../lib/contrato'

  let {
    app,
    servidor,
    comprobacion = null,
    estadoComprobacion = 'no-sabe',
    faltanDeAntes = [],
    porEntrada = false,
    alConfirmar,
    alCancelar,
  }: {
    app: App
    servidor: string
    /** `env check` de la release que sirve. `null` mientras no se sabe. */
    comprobacion?: ComprobacionDeEntorno | null
    /** Qué pasó al preguntar: el servidor es anterior a la 1.3.8 y no lo sabe
     *  decir (`no-sabe`), se está preguntando, se ha podido o no se ha podido.
     *  Las cuatro se dicen distinto, porque «no he podido mirarlo» no es «no
     *  falta nada». */
    estadoComprobacion?: 'no-sabe' | 'mirando' | 'hecho' | 'no-se-pudo'
    /** Las que pidió un despliegue anterior que se paró por ellas. Mandan sobre
     *  la comprobación: ésa mira la release que sirve, y ésta la del commit
     *  nuevo, que puede declarar una más. */
    faltanDeAntes?: string[]
    /** El servidor tiene Orbit 1.4.9 o posterior: los valores irán por la
     *  entrada estándar (`--env-stdin`) y no en la orden. Lo decide de nuevo
     *  el envoltorio, preguntándole la versión al servidor justo antes; esto
     *  sólo cambia lo que se enseña y se avisa. */
    porEntrada?: boolean
    alConfirmar: (entorno: VariableNueva[], forzar: boolean) => void
    alCancelar: () => void
  } = $props()

  // Las obligatorias sin valor, de las dos fuentes y sin repetir.
  const obligatorias = $derived([
    ...new Set([
      ...faltanDeAntes,
      ...(comprobacion?.vars.filter((v) => v.required && !v.set).map((v) => v.key) ?? []),
    ]),
  ])
  // Las opcionales sin valor se ofrecen, pero no paran nada.
  const opcionales = $derived(
    (comprobacion?.vars.filter((v) => !v.required && !v.set).map((v) => v.key) ?? [])
      .filter((k) => !obligatorias.includes(k)),
  )

  // Los valores viven aquí y sólo aquí, y se vacían en cuanto se usan o se
  // cierra la hoja: no se guardan en ningún sitio, ni se devuelven al padre más
  // que en la llamada que los manda.
  let valores = $state<Record<string, string>>({})
  let verOpcionales = $state(false)

  const congelada = $derived(app.state.locked === true)
  const lock = $derived(app.state.lock ?? null)

  function relleno(): VariableNueva[] {
    return [...obligatorias, ...opcionales]
      .filter((k) => (valores[k] ?? '') !== '')
      .map((k) => ({ clave: k, valor: valores[k]! }))
  }

  const problemas = $derived(
    Object.fromEntries(
      Object.entries(valores)
        .map(([k, v]) => [k, v === '' ? null : problemaDelValor(v)] as const)
        .filter(([, p]) => p !== null),
    ) as Record<string, string>,
  )
  const completas = $derived(obligatorias.every((k) => (valores[k] ?? '') !== ''))
  const listo = $derived(
    completas && Object.keys(problemas).length === 0 && estadoComprobacion !== 'mirando',
  )

  // La orden que se enseña sale del mismo sitio que la que se compara con el
  // núcleo, y con los valores tapados: se lee la clave, nunca el valor.
  const orden = $derived(ordenLegible(argvDeDespliegue(app.name, relleno(), congelada, porEntrada)))
  const porLaEntrada = $derived(porEntrada ? relleno().map((e) => e.clave) : [])

  function olvidar() {
    for (const k of Object.keys(valores)) valores[k] = ''
    valores = {}
  }

  function confirmar() {
    if (!listo) return
    const entorno = relleno()
    olvidar()
    alConfirmar(entorno, congelada)
  }

  function cancelar() {
    olvidar()
    alCancelar()
  }

  let dialogo = $state<HTMLDivElement | null>(null)
  $effect(() => {
    // Como en la hoja de siempre: el foco entra en el diálogo, no en el botón.
    dialogo?.focus()
  })
  // Y al cerrarse vuelve a donde estaba: sin esto, quien navega con teclado
  // acababa al principio de la página después de cada confirmación.
  const previo = typeof document !== 'undefined' ? (document.activeElement as HTMLElement | null) : null
  $effect(() => () => devolverFoco(previo))
  $effect(() => () => olvidar())

  function teclado(e: KeyboardEvent) {
    // Un Escape que ya ha atendido otra capa —la paleta— no cierra también ésta.
    if (e.key === 'Escape' && !e.defaultPrevented) cancelar()
  }
</script>

<svelte:window onkeydown={teclado} />

<div class="velo">
  <div
    class="hoja"
    role="dialog"
    aria-modal="true"
    aria-labelledby="hoja-despliegue-titulo"
    tabindex="-1"
    bind:this={dialogo}
  >
    <h2 id="hoja-despliegue-titulo">Desplegar {app.name} en <strong>{servidor}</strong></h2>

    {#if congelada}
      <!-- Lo primero, antes de la orden: es lo que cambia la decisión. -->
      <section class="congelada" role="note">
        <p class="aviso">
          <!-- Los espacios van como texto explícito: Svelte se come el blanco
               del principio de un bloque, y salía «congeladadesde …por». -->
          <strong>Está congelada</strong>{#if lock?.at}{' desde '}<span title={lock.at}>{lock.at.replace('T', ' ').slice(0, 16)}</span>{/if}{#if lock?.by}{' por '}<strong>{lock.by}</strong>{/if}{#if lock?.reason}{': '}«{lock.reason}»{/if}.
        </p>
        <p class="que">
          Desplegarla ahora usa <code>--force</code>: se despliega esta vez y
          <strong>sigue congelada</strong> después. Si lo que quieres es que vuelva a
          desplegarse con normalidad, descongélala desde su detalle.
        </p>
      </section>
    {/if}

    {#if estadoComprobacion === 'mirando'}
      <p class="que" role="status">Mirando si le falta alguna variable…</p>
    {:else if estadoComprobacion === 'no-se-pudo' && faltanDeAntes.length === 0}
      <p class="que">
        No he podido preguntar qué variables le faltan. Si falta una obligatoria,
        el servidor parará antes del build y la web seguirá como está.
      </p>
    {/if}

    {#if obligatorias.length > 0 || opcionales.length > 0}
      <section class="variables">
        {#if obligatorias.length > 0}
          <p class="que">
            {obligatorias.length === 1
              ? 'Le falta una variable obligatoria. Sin ella el servidor se para antes del build: aquí no hay nadie al teclado para que la pregunte.'
              : `Le faltan ${obligatorias.length} variables obligatorias. Sin ellas el servidor se para antes del build: aquí no hay nadie al teclado para que las pregunte.`}
          </p>
          {#each obligatorias as k (k)}
            <label class="campo">
              <span class="clave">{k}</span>
              <input
                type="password"
                autocomplete="off"
                spellcheck="false"
                aria-invalid={problemas[k] ? 'true' : undefined}
                bind:value={valores[k]}
              />
              {#if problemas[k]}<span class="mal">El valor {problemas[k]}.</span>{/if}
            </label>
          {/each}
        {/if}

        {#if opcionales.length > 0}
          <details bind:open={verOpcionales}>
            <summary>
              {opcionales.length === 1 ? 'Una opcional sin valor' : `${opcionales.length} opcionales sin valor`}:
              {opcionales.join(', ')}
            </summary>
            {#each opcionales as k (k)}
              <label class="campo">
                <span class="clave">{k}</span>
                <input
                  type="password"
                  autocomplete="off"
                  spellcheck="false"
                  aria-invalid={problemas[k] ? 'true' : undefined}
                  bind:value={valores[k]}
                />
                {#if problemas[k]}<span class="mal">El valor {problemas[k]}.</span>{/if}
              </label>
            {/each}
          </details>
        {/if}

        {#if porEntrada}
          <!-- Orbit 1.4.9: `--env-stdin`. El valor va por el canal cifrado de
               SSH y no toca la línea de órdenes del servidor. -->
          <p class="letra">
            Los valores viajan por la entrada del canal SSH, no dentro de la
            orden: no se ven en la lista de procesos del servidor ni quedan en
            el registro de <code>sudo</code>. Aquí no se guardan en ningún sitio
            y se olvidan al cerrar.
          </p>
        {:else}
          <!-- Lo incómodo, dicho donde se decide. Antes de Orbit 1.4.9 no hay
               otra forma de recibir un valor sin terminal: `--env-file` exige
               un fichero en el servidor, y escribirlo es justo lo que este
               cliente no hace. -->
          <p class="letra">
            Este servidor tiene un Orbit anterior a la 1.4.9, y los valores
            viajan dentro de la orden: mientras dura el despliegue se ven en la
            lista de procesos del servidor, y si no entras como root quedan en
            el registro de <code>sudo</code>. Con Orbit 1.4.9 irían por la
            entrada del canal SSH. Aquí no se guardan en ningún sitio y se
            olvidan al cerrar.
          </p>
        {/if}
      </section>
    {/if}

    <p class="intro">Se va a ejecutar:</p>
    <pre class="orden">{orden}</pre>
    {#if porLaEntrada.length > 0}
      <p class="letra entrada">Y por la entrada estándar: {porLaEntrada.map((k) => `${k}=••••••`).join(', ')}.</p>
    {/if}

    <p class="consecuencia">
      Actualiza el clon, compila en una release nueva y sólo al final mueve el
      symlink. Si el build falla, la versión que está publicada ahora ni se
      entera.
    </p>

    <div class="acciones">
      <button type="button" class="secundaria" onclick={cancelar}>Cancelar</button>
      <button type="button" class="confirmar" disabled={!listo} onclick={confirmar}>
        {congelada ? 'Desplegar igualmente' : 'Desplegar'}
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
    max-height: calc(100vh - 2 * var(--e-5)); overflow: auto;
  }
  .hoja:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  h2 { margin: 0 0 var(--e-4); font-size: 16px; color: var(--fg); }
  .congelada {
    border: 1px solid var(--border-strong); border-radius: var(--r-2);
    padding: var(--e-3); margin-bottom: var(--e-4);
  }
  .aviso { margin: 0 0 var(--e-2); font-size: 13px; color: var(--fg); }
  .que { margin: 0 0 var(--e-3); font-size: 13px; color: var(--fg-muted); max-width: 62ch; }
  .variables { margin-bottom: var(--e-4); }
  .campo { display: grid; gap: var(--e-1); margin-bottom: var(--e-3); font-size: 13px; }
  .clave { font-family: var(--mono); font-size: 12px; color: var(--fg); }
  .campo input {
    padding: var(--e-2); border-radius: var(--r-2);
    border: 1px solid var(--border-strong); background: var(--surface-2);
    font-family: var(--mono); font-size: 13px; color: var(--fg);
  }
  .mal { font-size: 12px; color: var(--fg); }
  summary { font-size: 12px; color: var(--fg-muted); cursor: pointer; margin-bottom: var(--e-2); }
  .letra { margin: 0; font-size: 11px; color: var(--fg-faint); max-width: 62ch; }
  .entrada { margin: calc(-1 * var(--e-3)) 0 var(--e-4); font-family: var(--mono); }
  .intro { margin: 0 0 var(--e-2); font-size: 13px; color: var(--fg-muted); }
  .orden {
    margin: 0 0 var(--e-4); padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 13px; color: var(--fg);
    white-space: pre-wrap; word-break: break-all; user-select: all;
  }
  .consecuencia { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 62ch; }
  code { font-family: var(--mono); font-size: 12px; }
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
