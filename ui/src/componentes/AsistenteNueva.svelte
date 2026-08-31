<script lang="ts">
  import {
    argvDeNew,
    avisoDelCertificado,
    borradorNuevo,
    listaDeAlias,
    nombreDesdeRepo,
    nombreValido,
    ordenDeNew,
    PASOS,
    problemasDe,
    sinAjustes,
    type Anulacion,
    type Borrador,
    type Paso,
  } from '../lib/asistente'

  let {
    servidor,
    inicial,
    pasoInicial,
    /** Si el servidor ya tiene `LETSENCRYPT_EMAIL`. `null` es **no lo he
     *  mirado**, y no se pinta como «no lo tiene». */
    correoEnElServidor = null,
    resolucion = null,
    resolviendo = false,
    alResolver,
    alCrear,
    alCerrar,
  }: {
    servidor: string
    inicial?: Borrador
    pasoInicial?: Paso
    correoEnElServidor?: boolean | null
    resolucion?: { del_dominio: string[]; del_servidor: string[]; coinciden: boolean | null } | null
    resolviendo?: boolean
    alResolver: (dominio: string) => void
    alCrear: (b: Borrador) => void
    alCerrar: () => void
  } = $props()

  let b = $state<Borrador>(inicial ? { ...inicial } : borradorNuevo())
  let paso = $state<Paso>(pasoInicial ?? 'Origen')

  // El nombre se propone a partir del repositorio y deja de proponerse en cuanto
  // alguien lo toca. Un campo que se reescribe solo después de haberlo editado
  // es un campo que pelea contra quien lo usa.
  let nombreTocado = $state(false)

  // Plegado en el caso normal, que es no tocar nada. Abierto si el borrador ya
  // trae ajustes: volver a este paso no puede esconder lo que ya se rellenó.
  let ajustesAbiertos = $state(!sinAjustes(b.ajustes))

  const indice = $derived(PASOS.indexOf(paso))
  const problemas = $derived(problemasDe(paso, b))
  const puedeSeguir = $derived(problemas.length === 0)
  const avisoCert = $derived(avisoDelCertificado(b, correoEnElServidor))
  const orden = $derived(ordenDeNew(b))
  const banderas = $derived(argvDeNew(b).filter((t) => t.startsWith('--')).length)

  function alEscribirRepo(v: string) {
    b.repo = v
    if (!nombreTocado) b.nombre = nombreDesdeRepo(v)
  }

  function ir(n: number) {
    const destino = PASOS[Math.max(0, Math.min(PASOS.length - 1, indice + n))]!
    // Hacia atrás siempre se puede. Hacia delante, sólo si este paso está
    // resuelto: dejar avanzar y quejarse al final es la forma cara de decir lo
    // mismo tres pantallas más tarde.
    if (n > 0 && !puedeSeguir) return
    paso = destino
    if (destino === 'Dominio' && b.dominio.trim() !== '') alResolver(b.dominio.trim())
  }

  /** Los tres estados de un campo que el servidor sabe detectar. */
  function ciclar(a: Anulacion): Anulacion {
    if (a.modo === 'detectar') return { modo: 'valor', valor: '' }
    if (a.modo === 'valor') return { modo: 'vacia' }
    return { modo: 'detectar' }
  }

  const AJUSTABLES: { clave: 'build' | 'arranque' | 'outdir'; etiqueta: string; vacio: string }[] = [
    { clave: 'build', etiqueta: 'Build', vacio: 'sin build' },
    { clave: 'arranque', etiqueta: 'Arranque', vacio: 'no arranca nada' },
    { clave: 'outdir', etiqueta: 'Carpeta de salida', vacio: 'sin carpeta de salida' },
  ]
</script>

<section class="asistente">
  <header>
    <h3>Nueva web en {servidor}</h3>
    <button type="button" class="cerrar" onclick={alCerrar}>Cancelar</button>
  </header>

  <!-- El raíl no es decoración: dice cuántos pasos quedan, que es lo que
       distingue un formulario de un pozo. Los pasos ya pasados se pueden pulsar
       para volver; los de delante no, porque saltarse una validación no es
       navegar. -->
  <ol class="rail">
    {#each PASOS as p, i (p)}
      <li class="tramo" class:tramo--hecho={i < indice} class:tramo--aqui={i === indice}>
        <button type="button" disabled={i > indice} onclick={() => (paso = p)}>
          <span class="num">{i + 1}</span>
          <span class="nom">{p}</span>
        </button>
      </li>
    {/each}
  </ol>

  <div class="cuerpo">
    {#if paso === 'Origen'}
      <label class="campo">
        <span class="et">Repositorio</span>
        <input
          value={b.repo}
          oninput={(e) => alEscribirRepo(e.currentTarget.value)}
          placeholder="usuario/mi-web"
          spellcheck="false"
          autocapitalize="off"
        />
        <span class="ayuda">
          «usuario/repo», o una URL <code>https</code>. Un <code>git@…</code> no,
          porque autentica con la clave del servidor y desde aquí no se puede
          comprobar que exista.
        </span>
      </label>

      <label class="campo campo--corto">
        <span class="et">Rama</span>
        <input bind:value={b.rama} spellcheck="false" autocapitalize="off" />
      </label>

      <label class="campo campo--corto">
        <span class="et">Nombre</span>
        <input
          value={b.nombre}
          oninput={(e) => {
            nombreTocado = true
            b.nombre = e.currentTarget.value
          }}
          spellcheck="false"
          autocapitalize="off"
        />
        <span class="ayuda">
          {#if !nombreTocado && b.nombre !== ''}
            Salido del repositorio. Cámbialo si quieres.
          {:else}
            Minúsculas, números, punto, guion y guion bajo. Como mucho 40.
          {/if}
        </span>
      </label>

    {:else if paso === 'Detección'}
      <!--
        Este paso NO enseña lo que Orbit ha detectado, porque todavía no lo ha
        detectado: `detect_stack` lee un directorio, y el directorio no existe
        hasta que `orbit new` ha clonado. No hay ninguna orden que mire un
        repositorio remoto y diga qué stack es.

        Así que aquí no se promete una conclusión: se ofrece adelantarse a ella,
        que es una pregunta distinta y mucho más honesta. Lo detectado se enseña
        después, al terminar, leído del descriptor — que ya es un hecho.
      -->
      <p class="explica">
        Orbit detecta el tipo de proyecto <strong>al clonarlo</strong>, así que
        todavía no puedo enseñarte qué va a detectar. Casi siempre acierta y este
        paso se deja en blanco.
      </p>
      <p class="explica">
        Se equivoca en tres casos, y si estás en uno ya lo sabes: un monorepo
        donde la app vive en un subdirectorio, un Astro o un SvelteKit donde el
        adaptador decide si sale un sitio estático o un servidor, y un proyecto
        que arranca con un script propio.
      </p>

      <details class="adelantar" bind:open={ajustesAbiertos}>
        <summary>Ya sé que se va a equivocar</summary>

        <label class="campo">
          <span class="et">Carpeta de la app</span>
          <input bind:value={b.ajustes.carpeta} placeholder="apps/web" spellcheck="false" />
          <!-- No es un campo más y no se pinta como tal: cambiarla redirige la
               detección entera, así que los otros se leen contra otro
               directorio. Ponerla en la misma lista daría a entender que es un
               ajuste al lado de los demás, y es el que invalida a los demás. -->
          <span class="ayuda">
            Relativa a la raíz del repositorio. <strong>Cambia dónde mira todo lo
            demás</strong>: con esto puesto, el tipo y el build se detectan
            dentro de esa carpeta.
          </span>
        </label>

        <label class="campo campo--corto">
          <span class="et">Tipo</span>
          <input bind:value={b.ajustes.tipo} placeholder="next, laravel, static…" spellcheck="false" />
        </label>

        {#each AJUSTABLES as a (a.clave)}
          {@const v = b.ajustes[a.clave]}
          <div class="campo campo--tri">
            <span class="et">{a.etiqueta}</span>
            <div class="tri">
              {#if v.modo === 'valor'}
                <input
                  value={v.valor}
                  oninput={(e) => (b.ajustes[a.clave] = { modo: 'valor', valor: e.currentTarget.value })}
                  spellcheck="false"
                  aria-label={a.etiqueta}
                />
              {:else}
                <span class="estado-tri" class:anulacion--vacia={v.modo === 'vacia'}>
                  {v.modo === 'vacia' ? a.vacio : 'lo detecta Orbit'}
                </span>
              {/if}
              <button type="button" onclick={() => (b.ajustes[a.clave] = ciclar(v))}>
                cambiar
              </button>
            </div>
          </div>
        {/each}

        <!-- «Ninguno» es una respuesta y tiene que poder escribirse: `--build ''`
             significa «esta app no se compila», y es distinto de no decir nada.
             Un campo de texto en blanco no puede significar las dos cosas. -->
        <p class="ayuda ayuda--suelta">
          «lo detecta Orbit» y «sin build» no son lo mismo: lo segundo
          le dice al servidor que esta app no se compila, y viaja en la orden como
          <code>--build ''</code>.
        </p>
      </details>

      <p class="salida">
        Si la detección no encaja nunca —y en un monorepo raro no encaja nunca—,
        la respuesta no es pelearse con este formulario cada vez: ejecuta
        <code>orbit init</code> en tu repositorio y sube el <code>orbit.json</code>
        que escriba. A partir de entonces manda el fichero, no la detección.
      </p>

    {:else if paso === 'Dominio'}
      <label class="campo">
        <span class="et">Dominio</span>
        <input
          bind:value={b.dominio}
          onblur={() => b.dominio.trim() !== '' && alResolver(b.dominio.trim())}
          placeholder="mi-web.ejemplo.com"
          spellcheck="false"
          autocapitalize="off"
        />
      </label>

      <label class="campo">
        <span class="et">Alias</span>
        <input bind:value={b.alias} placeholder="www.mi-web.ejemplo.com" spellcheck="false" />
        <span class="ayuda">
          Separados por comas. Si lo dejas vacío no se añade ninguno —tampoco un
          <code>www.</code>— porque «ninguno» es una respuesta y se manda como tal.
        </span>
      </label>

      {#if resolviendo}
        <p class="dns tenue">comprobando a dónde apunta…</p>
      {:else if resolucion}
        <!-- Es el final F7 —publicada pero sin DNS— convertido en un aviso
             previo. Avisa y nunca impide seguir: un servidor detrás de un proxy
             da direcciones distintas y no está roto. -->
        <div class="dns aviso-dns" class:aviso-dns--sin-mirar={resolucion.coinciden === null}>
          {#if resolucion.coinciden === true}
            <p>Ese dominio ya apunta a este servidor.</p>
          {:else if resolucion.coinciden === false}
            <p>
              <strong>Ese dominio no apunta a este servidor.</strong> La web se
              creará igual y se servirá desde dentro, pero desde fuera nadie
              llegará hasta que cambies el DNS.
            </p>
            <p class="cifras">
              {b.dominio.trim()} → {resolucion.del_dominio.join(', ')}<br />
              {servidor} → {resolucion.del_servidor.join(', ')}
            </p>
          {:else}
            <p>
              No he podido comparar: {resolucion.del_dominio.length === 0
                ? 'ese nombre no resuelve todavía'
                : 'no sé a qué dirección va este servidor'}. Eso
              <strong>no quiere decir que esté mal</strong>, quiere decir que no
              lo sé.
            </p>
          {/if}
          <p class="tenue">
            Lo resuelve esta máquina, con su caché: puede no ser lo que ve el
            resto del mundo si acabas de cambiarlo.
          </p>
        </div>
      {/if}

    {:else if paso === 'Extras'}
      <label class="marca">
        <input type="checkbox" bind:checked={b.https} />
        <span>
          <strong>Emitir el certificado</strong>
          <span class="ayuda">
            Es lo que hace por defecto. Quitarlo publica la web sólo por HTTP.
          </span>
        </span>
      </label>

      {#if b.https}
        <label class="campo">
          <span class="et">Correo para Let's Encrypt</span>
          <input bind:value={b.correo} placeholder="avisos@ejemplo.com" spellcheck="false" />
          <!-- Un efecto secundario que se enseña ANTES y no después: el correo
               se guarda en la configuración global del servidor, no en la de la
               app. Decirlo cuando ocurre evita el «¿por qué ya no me lo
               pregunta?» de dentro de tres meses. -->
          <span class="ayuda">
            Se guarda en la configuración del servidor, no en la de esta web: la
            próxima que crees aquí ya no te lo preguntará.
          </span>
        </label>
      {/if}

      {#if avisoCert}
        <p class="aviso-cert">{avisoCert}</p>
      {/if}

      <label class="marca">
        <input type="checkbox" bind:checked={b.baseDeDatos} />
        <span>
          <strong>Crear una base de datos</strong>
          <span class="ayuda">
            No se crea por defecto. Si la marcas, Orbit la crea y deja sus
            credenciales en el <code>.env</code> de la app.
          </span>
        </span>
      </label>

      <p class="salida">
        El autodespliegue no se configura aquí: es una orden aparte
        (<code>orbit autodeploy</code>) y se activa cuando la web ya existe y
        sirve. Ofrecerlo en este formulario daría a entender que <code>new</code>
        lo hace, y no lo hace.
      </p>

    {:else if paso === 'Repaso'}
      <dl class="repaso">
        <div><dt>Repositorio</dt><dd>{b.repo.trim()} <span class="tenue">rama {b.rama.trim()}</span></dd></div>
        <div><dt>Nombre</dt><dd>{b.nombre.trim()}</dd></div>
        <div>
          <dt>Dominio</dt>
          <dd>
            {b.dominio.trim()}
            {#if listaDeAlias(b).length > 0}
              <span class="tenue">y {listaDeAlias(b).join(', ')}</span>
            {:else}
              <span class="tenue">sin alias</span>
            {/if}
          </dd>
        </div>
        <div>
          <dt>Certificado</dt>
          <dd>{b.https ? 'sí' : 'no, sólo HTTP'}</dd>
        </div>
        <div><dt>Base de datos</dt><dd>{b.baseDeDatos ? 'sí' : 'no'}</dd></div>
        <div>
          <dt>Detección</dt>
          <dd>
            {sinAjustes(b.ajustes) ? 'la del servidor, sin tocar' : 'con ajustes por delante'}
          </dd>
        </div>
      </dl>

      {#if avisoCert}
        <p class="aviso-cert">{avisoCert}</p>
      {/if}

      <!-- La orden literal, y no un resumen de la orden: es la prueba de que
           esto sólo invoca `orbit`, y la única forma de que alguien pueda
           comprobar que lo que va a pasar es lo que ha pedido. La construye el
           mismo código que la prueba compara contra el catálogo del núcleo. -->
      <p class="et">Lo que voy a ejecutar</p>
      <pre class="orden">{orden}</pre>
      <p class="ayuda ayuda--suelta">
        {banderas} banderas. <code>--yes</code> no es «que sí a todo»: es «acepta
        lo que está por defecto», y por defecto no se crea la base de datos ni se
        abre el editor del <code>.env</code>.
      </p>

      <p class="salida">
        Esto tarda: clona, instala, compila y despliega. Puede tardar tres
        minutos y <strong>puede terminar a medias</strong> —creada pero sin
        certificado, creada pero sin compilar—, así que al acabar te diré qué
        existe, qué falta y qué se deshace, preguntándoselo al servidor.
      </p>
    {/if}
  </div>

  {#if problemas.length > 0}
    <ul class="problema">
      {#each problemas as p (p)}<li>{p}</li>{/each}
    </ul>
  {/if}

  <footer>
    <button type="button" onclick={() => ir(-1)} disabled={indice === 0}>Atrás</button>
    {#if paso === 'Repaso'}
      <button type="button" class="primaria" onclick={() => alCrear(b)}>Crear la web</button>
    {:else}
      <button type="button" class="primaria" onclick={() => ir(1)} disabled={!puedeSeguir}>
        Siguiente
      </button>
    {/if}
  </footer>
</section>

<style>
  .asistente {
    display: grid;
    gap: var(--e-4);
    max-width: 78ch;
  }
  header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }
  .cerrar {
    background: none; border: 0; padding: 0; font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer; text-decoration: underline;
  }

  .rail { display: flex; gap: var(--e-1); list-style: none; margin: 0; padding: 0; flex-wrap: wrap; }
  .tramo button {
    display: flex; align-items: center; gap: var(--e-2);
    background: none; border: 0; border-bottom: 2px solid var(--border);
    padding: var(--e-2) var(--e-3) var(--e-2) var(--e-2);
    font: inherit; font-size: 12px; color: var(--fg-faint); cursor: pointer;
  }
  .tramo button:disabled { cursor: default; }
  .tramo button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .num {
    display: inline-grid; place-items: center; width: 18px; height: 18px;
    border-radius: 50%; border: 1px solid var(--border-strong); font-size: 11px;
  }
  .tramo--hecho button { color: var(--fg-muted); border-bottom-color: var(--border-strong); }
  .tramo--aqui button { color: var(--fg); border-bottom-color: var(--accent); font-weight: 600; }
  .tramo--aqui .num { border-color: var(--accent); color: var(--accent-text); }

  .cuerpo { display: grid; gap: var(--e-4); }

  .campo { display: grid; gap: var(--e-1); }
  .campo--corto input { max-width: 32ch; }
  /* Etiqueta a la izquierda y valor a la derecha, como las fichas del resto de
     la aplicación. Apilados se leían como tres bloques sueltos en vez de como
     una lista de tres campos del mismo sitio. */
  .campo--tri {
    grid-template-columns: 140px 1fr;
    align-items: center;
    column-gap: var(--e-3);
  }
  .et { font-size: 12px; color: var(--fg-muted); }
  .ayuda { font-size: 12px; color: var(--fg-faint); max-width: 68ch; }
  .ayuda--suelta { margin: 0; }
  input:not([type]) {
    background: var(--surface); border: 1px solid var(--border-strong);
    border-radius: var(--r-1); padding: var(--e-2);
    font: inherit; font-size: 13px; color: var(--fg);
  }
  input:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }

  .tri { display: flex; align-items: center; gap: var(--e-2); }
  .tri input { flex: 1; max-width: 44ch; }
  .estado-tri {
    flex: 1; font-size: 13px; font-style: italic;
    color: var(--anulacion, var(--fg-faint));
  }
  .tri button, footer button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .tri button:hover, footer button:hover:not(:disabled) { color: var(--fg); }
  .tri button:focus-visible, footer button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }

  .explica, .salida { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 72ch; }
  .salida { font-size: 12px; color: var(--fg-faint); }
  .adelantar { display: grid; gap: var(--e-4); }
  summary { font-size: 13px; color: var(--fg); cursor: pointer; }

  .marca { display: flex; gap: var(--e-3); align-items: start; font-size: 13px; color: var(--fg); }
  .marca > span { display: grid; gap: 2px; }

  .dns { margin: 0; font-size: 13px; display: grid; gap: var(--e-2); }
  .aviso-dns { color: var(--asistente, var(--fg-muted)); }
  .cifras { font-family: var(--mono); font-size: 12px; margin: 0; }
  .tenue { color: var(--fg-faint); font-size: 12px; margin: 0; }

  .aviso-cert {
    margin: 0; font-size: 13px; max-width: 72ch;
    color: var(--asistente, var(--fg-muted));
    border-left: 3px solid var(--asistente, var(--border-strong));
    padding-left: var(--e-3);
  }

  .repaso { display: grid; gap: var(--e-2); margin: 0; }
  .repaso > div { display: grid; grid-template-columns: 140px 1fr; gap: var(--e-3); align-items: baseline; }
  dt { font-size: 12px; color: var(--fg-faint); }
  dd { margin: 0; font-size: 13px; color: var(--fg); }

  .orden {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    /* Parte entre argumentos, nunca dentro de uno: `break-all` cortaba
       «www.tienda.ejemplo.com» por la mitad, que es justo el argumento que
       alguien viene a comprobar a esta pantalla. `anywhere` sólo corta si un
       argumento suelto no cabe en la línea. */
    white-space: pre-wrap; word-break: normal; overflow-wrap: anywhere;
    user-select: all;
  }

  .problema {
    margin: 0; padding-left: var(--e-4);
    font-size: 12px; color: var(--asistente, var(--fg-muted)); max-width: 72ch;
  }

  footer { display: flex; gap: var(--e-2); justify-content: flex-end; }
  .primaria {
    background: var(--accent-fill); color: var(--on-accent); border: 0;
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
  }
  /* Los dos, no sólo el primario: un control deshabilitado que se ve igual que
     uno activo es peor que no tenerlo, porque invita a pulsarlo y no contesta. */
  .primaria:disabled, footer button:disabled { opacity: .45; cursor: default; }
  code { font-family: var(--mono); font-size: 12px; }
</style>
