<script lang="ts">
  import { tick } from 'svelte'
  import { aliasValido, hostValido, usuarioValido, type Borrador } from '../lib/servidores'
  import type { ErrorDelPuente } from '../lib/puente'

  let {
    yaUsados,
    guardando = false,
    fallo = null,
    enfocarAlAbrir = true,
    alGuardar,
    alCerrar,
  }: {
    /** Los alias que ya existen, de las dos fuentes. Se comprueba **mientras se
     *  escribe** porque el choque con un `Host` del `~/.ssh/config` no es un
     *  detalle: `ssh` resolvería el del fichero y la aplicación enseñaría un
     *  servidor mientras habla con otro. */
    yaUsados: string[]
    guardando?: boolean
    fallo?: ErrorDelPuente | null
    /** La galería pinta varias pantallas a la vez y no puede dejar que cada
     *  una se lleve el foco. En la aplicación, siempre. */
    enfocarAlAbrir?: boolean
    alGuardar: (b: Borrador) => void
    alCerrar: () => void
  } = $props()

  let b = $state<Borrador>({ alias: '', host: '', usuario: 'root', puerto: 22, clave: '' })

  // El alias se propone a partir del host y deja de proponerse en cuanto
  // alguien lo toca. Misma regla que el nombre en el asistente de web nueva.
  let aliasTocado = $state(false)

  // Qué campos ha visitado ya alguien.
  //
  // **Un campo vacío que nadie ha tocado no está mal: está vacío.** Quejarse
  // antes de que hayan escrito nada es ruido, y además enseña un formulario en
  // rojo nada más abrirlo — que se lee como «esto está roto» justo en la
  // pantalla donde alguien empieza. Es la misma regla que `nombreValido('')`,
  // que devuelve null a propósito.
  //
  // Al intentar enviar se marcan todos: ahí sí hay que decir qué falta.
  let tocados = $state<Record<string, boolean>>({})
  const tocar = (campo: string) => (tocados[campo] = true)

  function alEscribirHost(v: string) {
    b.host = v
    if (!aliasTocado) {
      const propuesto = v.split('.')[0]?.toLowerCase().replace(/[^a-z0-9._-]/g, '-') ?? ''
      b.alias = aliasValido(propuesto) && !yaUsados.includes(propuesto) ? propuesto : ''
    }
  }

  const problemas = $derived.by(() => {
    const p: { campo: string; que: string }[] = []
    const a = b.alias.trim()
    if (a === '') p.push({ campo: 'alias', que: 'Ponle un nombre.' })
    else if (!aliasValido(a))
      p.push({ campo: 'alias', que: 'Letras, números, punto, guion y guion bajo.' })
    else if (yaUsados.includes(a))
      p.push({
        campo: 'alias',
        que: 'Ya tienes uno con ese nombre. Si es el de tu ~/.ssh/config, ssh usaría ése y verías un servidor mientras hablas con otro.',
      })

    if (b.host.trim() === '') p.push({ campo: 'host', que: 'Falta la dirección.' })
    else if (!hostValido(b.host.trim()))
      p.push({ campo: 'host', que: 'Una IP o un dominio, sin espacios.' })

    if (b.usuario.trim() === '') p.push({ campo: 'usuario', que: 'Falta el usuario.' })
    else if (!usuarioValido(b.usuario.trim()))
      p.push({ campo: 'usuario', que: 'Ese nombre de usuario no vale.' })

    if (!Number.isInteger(b.puerto) || b.puerto < 1 || b.puerto > 65535)
      p.push({ campo: 'puerto', que: 'Entre 1 y 65535.' })
    return p
  })

  const de = (campo: string) =>
    tocados[campo] ? problemas.find((p) => p.campo === campo)?.que : undefined
  const listo = $derived(problemas.length === 0)

  /** Los ids de lo que describe a un campo: su error si lo hay y su pista. Un
   *  error que no está enlazado al campo no se oye al llegar a él: el lector
   *  dice «no válido» y no dice por qué. */
  const describe = (...ids: (string | false | undefined | null)[]) =>
    ids.filter(Boolean).join(' ') || undefined

  // Lo que se anuncia al intentar enviar con algo mal. El foco ya va al primer
  // campo que falla —y ese campo dice su error—, pero sin esto quien no ve la
  // pantalla no sabe que hay MÁS de uno.
  let aviso = $state('')
  let formulario = $state<HTMLFormElement | null>(null)

  async function enviar(e: SubmitEvent) {
    e.preventDefault()
    if (guardando) return
    for (const c of ['alias', 'host', 'usuario', 'puerto']) tocar(c)
    if (listo) {
      aviso = ''
      alGuardar(b)
      return
    }
    // El botón está SIEMPRE habilitado: uno deshabilitado no recibe el foco con
    // el tabulador, así que con el teclado no se encontraba, e Intro en un
    // campo no hacía nada sin decir por qué. Se valida al pulsar y se lleva el
    // foco a lo primero que hay que arreglar.
    const n = problemas.length
    aviso = n === 1 ? 'Hay un campo por corregir.' : `Hay ${n} campos por corregir.`
    await tick()
    formulario?.querySelector<HTMLInputElement>('[aria-invalid="true"]')?.focus()
  }

  // Al abrirse, el foco va al primer campo. La vista la abre App y el foco se
  // quedaba en el botón que la abrió, que ya no existe: acababa en el <body> y
  // había que volver a recorrer la página entera con el tabulador.
  let primero = $state<HTMLInputElement | null>(null)
  $effect(() => {
    if (enfocarAlAbrir) primero?.focus()
  })

  const ID_TITULO = 'ns-titulo'

  /** El choque con otro alias se dice **mientras se escribe**, sin esperar a
   *  salir del campo: no es «te falta algo», es «esto no puede ser», y callarlo
   *  hasta el final deja teclear un nombre entero para tirarlo después. */
  const choca = $derived(
    b.alias.trim() !== '' && yaUsados.includes(b.alias.trim())
      ? problemas.find((p) => p.campo === 'alias')?.que
      : undefined,
  )
</script>

<form
  class="alta"
  aria-labelledby={ID_TITULO}
  novalidate
  bind:this={formulario}
  onsubmit={enviar}
>
  <h3 id={ID_TITULO}>Añadir un servidor</h3>

  <div class="campo campo--ancho">
    <label for="ns-host">Dirección del servidor</label>
    <input
      id="ns-host"
      bind:this={primero}
      value={b.host}
      oninput={(e) => alEscribirHost(e.currentTarget.value)}
      onblur={() => tocar('host')}
      placeholder="203.0.113.10  ·  srv.ejemplo.com"
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      aria-invalid={de('host') ? 'true' : undefined}
      aria-describedby={describe(de('host') && 'ns-host-mal')}
    />
    {#if de('host')}<p class="mal" id="ns-host-mal">{de('host')}</p>{/if}
  </div>

  <div class="fila">
    <div class="campo">
      <label for="ns-usuario">Usuario</label>
      <input
        id="ns-usuario"
        bind:value={b.usuario}
        onblur={() => tocar('usuario')}
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
        aria-invalid={de('usuario') ? 'true' : undefined}
        aria-describedby={describe(de('usuario') && 'ns-usuario-mal')}
      />
      {#if de('usuario')}<p class="mal" id="ns-usuario-mal">{de('usuario')}</p>{/if}
    </div>
    <div class="campo campo--corto">
      <label for="ns-puerto">Puerto</label>
      <input
        id="ns-puerto"
        type="number"
        min="1"
        max="65535"
        bind:value={b.puerto}
        onblur={() => tocar('puerto')}
        aria-invalid={de('puerto') ? 'true' : undefined}
        aria-describedby={describe(de('puerto') && 'ns-puerto-mal')}
      />
      <!-- Faltaba: el campo se marcaba en rojo y nadie decía por qué. -->
      {#if de('puerto')}<p class="mal" id="ns-puerto-mal">{de('puerto')}</p>{/if}
    </div>
  </div>

  <div class="campo campo--ancho">
    <label for="ns-alias">Cómo lo llamas aquí</label>
    <input
      id="ns-alias"
      value={b.alias}
      oninput={(e) => {
        aliasTocado = true
        b.alias = e.currentTarget.value
      }}
      onblur={() => tocar('alias')}
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      aria-invalid={de('alias') || choca ? 'true' : undefined}
      aria-describedby={describe(
        (choca || de('alias')) && 'ns-alias-mal',
        !choca && !de('alias') && !aliasTocado && b.alias !== '' && 'ns-alias-pista',
      )}
    />
    {#if choca}
      <p class="mal" id="ns-alias-mal">{choca}</p>
    {:else if de('alias')}
      <p class="mal" id="ns-alias-mal">{de('alias')}</p>
    {:else if !aliasTocado && b.alias !== ''}
      <p class="pista" id="ns-alias-pista">Propuesto a partir de la dirección. Cámbialo si quieres.</p>
    {/if}
  </div>

  <!--
    La clave, plegada, porque el caso normal es no tocarla: si hay un agente
    cargado —y lo hay casi siempre— `ssh` la encuentra solo.
  -->
  <details class="avanzado">
    <summary>Usar una clave concreta</summary>
    <div class="campo campo--ancho">
      <label for="ns-clave">Ruta del fichero de la clave</label>
      <input
        id="ns-clave"
        aria-describedby="ns-clave-pista"
        bind:value={b.clave}
        placeholder="~/.ssh/id_ed25519"
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
      />
      <!--
        Dicho donde alguien podría temer lo contrario: está a punto de escribir
        algo sobre una clave privada en un formulario.
      -->
      <p class="pista" id="ns-clave-pista">
        Se guarda <strong>la ruta</strong>, nunca la clave ni su frase de paso.
        Lo que hay dentro se lo pide <code>ssh</code> a tu agente, igual que en
        un terminal.
      </p>
    </div>
  </details>

  {#if fallo}
    <!-- Nuestra frase arriba y la del núcleo debajo, como detalle: la del
         núcleo empieza en minúscula y no dice qué hacer. -->
    <div class="fallo" role="alert">
      <p>No he podido guardarlo. Revisa los datos y vuelve a intentarlo.</p>
      {#if fallo.mensaje.trim() !== ''}<p class="cita">{fallo.mensaje}</p>{/if}
    </div>
  {/if}

  <p class="sr" role="status">{aviso}</p>

  <footer>
    <button type="button" class="secundario" onclick={alCerrar}>Cancelar</button>
    <!-- Habilitado siempre, también mientras guarda: un botón que se
         deshabilita con el foco encima tira el foco al <body>. Mientras guarda
         lo dice y no hace nada. -->
    <button type="submit" class="primario" aria-disabled={guardando ? 'true' : undefined}>
      {#if guardando}
        <span class="giro" aria-hidden="true"></span> Guardando…
      {:else}
        Añadir servidor
      {/if}
    </button>
  </footer>

  <p class="nota">
    Se guarda en tu ordenador: el nombre, la dirección, el usuario y el puerto.
    <strong>Nada de lo que diga el servidor</strong>, y ninguna contraseña.
  </p>
</form>

<style>
  .alta { display: grid; gap: var(--e-4); max-width: 56ch; container-type: inline-size; }
  h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }

  .campo { display: grid; gap: var(--e-1); min-width: 0; }
  .campo--ancho { grid-column: 1 / -1; }
  .fila { display: grid; grid-template-columns: 1fr 8rem; gap: var(--e-3); }
  /* En estrecho, una columna: usuario y puerto uno debajo del otro. */
  @container (max-width: 22rem) {
    .fila { grid-template-columns: 1fr; }
  }
  .campo--corto input { width: 100%; }

  label { font-size: 12px; color: var(--fg-muted); }
  input {
    background: var(--surface); border: 1px solid var(--border-strong);
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 14px; color: var(--fg);
    transition: border-color var(--t-rapido) var(--e-suave),
                box-shadow var(--t-rapido) var(--e-suave);
  }
  input:hover { border-color: var(--accent); }
  input:focus {
    outline: none;
    border-color: var(--accent);
    /* El anillo crece desde el borde en vez de aparecer: el foco se sigue con
       la vista, y algo que aparece de golpe hay que volver a buscarlo. */
    box-shadow: 0 0 0 3px var(--velo);
  }
  input:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  /* El campo que está mal se marca por borde Y por texto: el color no es nunca
     el único portador. */
  input[aria-invalid='true'] { border-color: var(--aviso-campo, var(--border-strong)); }

  .mal, .pista { margin: 0; font-size: 12px; max-width: 52ch; }
  .mal { color: var(--aviso-campo, var(--fg-muted)); }
  .pista { color: var(--fg-faint); }

  .avanzado { display: grid; gap: var(--e-3); }
  summary {
    font-size: 13px; color: var(--fg-muted); cursor: pointer;
    padding: var(--e-1) 0; list-style-position: outside;
  }
  summary:hover { color: var(--fg); }

  .fallo p { margin: 0; }
  .fallo .cita { margin-top: var(--e-1); font-family: var(--mono); font-size: 12px; overflow-wrap: anywhere; }
  .fallo {
    margin: 0; font-size: 13px; padding: var(--e-3);
    border-radius: var(--r-2);
    color: var(--aviso-campo, var(--fg));
    border: 1px solid var(--aviso-campo, var(--border-strong));
    animation: entra var(--t-normal) var(--e-entrada);
  }

  footer { display: flex; flex-wrap: wrap; gap: var(--e-2); justify-content: flex-end; align-items: center; }
  button {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-4);
    font: inherit; font-size: 14px; font-weight: 600; cursor: pointer;
    display: inline-flex; align-items: center; gap: var(--e-2);
    transition: transform var(--t-rapido) var(--e-suave),
                opacity var(--t-rapido) var(--e-suave),
                filter var(--t-rapido) var(--e-suave);
  }
  .primario { background: var(--accent-fill); color: var(--on-accent); border: 0; }
  .secundario { background: none; border: 1px solid var(--border-strong); color: var(--fg-muted); }
  button:hover:not(:disabled) { filter: brightness(1.08); }
  /* Se hunde al pulsar. Es el único movimiento de esta pantalla que no informa
     de nada: informa de que se ha pulsado, que en un botón que tarda es justo
     lo que hace falta. */
  button:active:not(:disabled) { transform: translateY(1px); }
  button:disabled, button[aria-disabled='true'] { opacity: .45; cursor: default; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }

  .giro {
    width: 12px; height: 12px; border-radius: 50%;
    border: 2px solid currentColor; border-top-color: transparent;
    animation: gira 700ms linear infinite;
  }
  @keyframes gira { to { transform: rotate(360deg); } }
  @keyframes entra {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: none; }
  }

  .nota { margin: 0; font-size: 12px; color: var(--fg-faint); max-width: 52ch; }
  code { font-family: var(--mono); font-size: 12px; }
  input { min-width: 0; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
</style>
