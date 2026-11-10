<script lang="ts">
  import ChipEstado from './ChipEstado.svelte'
  import FacetaSsl from './FacetaSsl.svelte'
  import { PRESENTACION, marcarInvisibles as vis, salud, num, type App, type AppInfo, type Tareas } from '../lib/contrato'
  import { problemaDelMotivo } from '../lib/despliegue'

  let {
    app,
    servidor,
    info = null,
    tareas = null,
    puedeCongelar = false,
    puedeEsperarCi = false,
    alCongelar = null,
    alDescongelar = null,
    alEsperarCi = null,
  }: {
    app: App
    servidor: string
    /** `info --json`, cuando ya ha llegado. Trae lo que `list` no calcula
     *  —los días del certificado— y la configuración. `null` mientras no. */
    info?: AppInfo | null
    /** `cron list --json`. `null` mientras no ha llegado o si el servidor no
     *  lo sabe contar (anterior a la 1.3.9): en los dos casos no se pinta una
     *  lista vacía, que diría «no tiene tareas». */
    tareas?: Tareas | null
    puedeCongelar?: boolean
    puedeEsperarCi?: boolean
    alCongelar?: ((motivo: string | null) => void) | null
    alDescongelar?: (() => void) | null
    alEsperarCi?: ((esperar: boolean) => void) | null
  } = $props()

  // El estado más reciente que se tenga: el del detalle si ha llegado —trae
  // los días del certificado— y el de la lista mientras tanto. Los dos son
  // del mismo servidor y de la misma app; si no lo fueran, quien llama no
  // habría pasado `info`.
  const estado = $derived(info?.state ?? app.state)
  const s = $derived(salud(estado))
  const lock = $derived(estado.lock ?? null)
  const cfg = $derived(info?.config ?? null)

  // El formulario del motivo, a la vista sólo cuando se pide: congelar es una
  // decisión, y un campo de texto siempre abierto invita a escribir en él.
  let congelando = $state(false)
  let motivo = $state('')
  const problema = $derived(problemaDelMotivo(motivo))

  function cuando(iso: string | null): string {
    // Se enseña tal cual, sin la hora del servidor convertida a la de aquí: la
    // conversión necesitaría saber el huso del servidor, que el ISO trae, y
    // equivocarse en una hora es peor que enseñarla como viene.
    return iso ? iso.replace('T', ' ').slice(0, 16) : '?'
  }

  function tipoDeCertificado(t: string | null | undefined): string | null {
    if (t === undefined) return null
    if (t === null) return null
    if (t === 'letsencrypt') return "Let's Encrypt, lo renueva certbot"
    if (t === 'origin') return 'origen de Cloudflare: sólo vale detrás de su proxy, y no lo renueva certbot'
    return t
  }
</script>

<article class="detalle">
  <header>
    <!-- Siempre `servidor : app`, nunca la app sola. El nombre de una app no
         identifica nada por sí solo: `tienda` existe en tres servidores, y ese
         accidente tiene precedente — la suite de pruebas de Orbit borró el
         vhost de una app de producción llamada así. -->
    <!-- Con lo invisible a la vista: un nombre o un dominio con un carácter de
         ancho cero o un homoglifo se lee como otro, y aquí se confirman cosas. -->
    <p class="ruta"><span class="servidor">{servidor}</span> : <strong>{vis(app.name)}</strong></p>
    <ChipEstado estado={estado} />
  </header>

  <!-- La frase, no sólo el color. El color dice que pasa algo; la frase dice
       qué, y es lo único accionable. -->
  <p class="frase">{PRESENTACION[s].frase}</p>

  <dl>
    <div><dt>Dominio</dt><dd>{vis(app.domain)}</dd></div>
    {#if app.aliases.length > 0}
      <div><dt>Alias</dt><dd>{app.aliases.map(vis).join(' · ')}</dd></div>
    {/if}
    <div><dt>Tipo</dt><dd>{vis(app.type)}</dd></div>
    <div><dt>Puerto</dt><dd class="mono">{num(estado.port)}</dd></div>
    <div>
      <dt>HTTPS</dt>
      <dd>
        <FacetaSsl estado={estado} />
        {#if estado.ssl && tipoDeCertificado(estado.cert_type)}
          <span class="tenue">· {tipoDeCertificado(estado.cert_type)}</span>
        {/if}
      </dd>
    </div>
    <div><dt>Releases</dt><dd class="mono">{num(estado.releases)}</dd></div>
    <div>
      <dt>Último despliegue</dt>
      <!-- `null` es «nunca», y decirlo con la palabra es mejor que con un
           guion: un guion aquí se lee como «no lo sé». -->
      <dd class="mono">{estado.last_deploy ?? 'nunca'}</dd>
    </div>
    <div>
      <dt>Autodespliegue</dt>
      <dd>
        {estado.autodeploy ? 'activado' : 'desactivado'}
        {#if estado.checks === true}
          <span class="tenue">· espera a la CI de GitHub</span>
        {/if}
      </dd>
    </div>
    {#if cfg && (cfg.health_path || cfg.health_timeout)}
      <div>
        <dt>Comprobación de salud</dt>
        <dd class="mono">{cfg.health_path || '/'}{#if cfg.health_timeout} · {cfg.health_timeout} s{/if}</dd>
      </div>
    {/if}
    {#if cfg && (cfg.limit_mem || cfg.limit_cpu)}
      <div>
        <dt>Límites</dt>
        <dd class="mono">
          {#if cfg.limit_mem}{cfg.limit_mem} de memoria{/if}{#if cfg.limit_mem && cfg.limit_cpu} · {/if}{#if cfg.limit_cpu}{cfg.limit_cpu} de CPU{/if}
        </dd>
      </div>
    {/if}
  </dl>

  <!--
    Los despliegues. `locked` puede ser true, false o NO ESTAR: un servidor
    anterior a la 1.4.3 no sabe congelar, y eso no es «no está congelada». En
    ese caso no se dice nada y no se ofrece nada.
  -->
  {#if estado.locked === true}
    <section class="seccion congelada" aria-labelledby="t-congelada">
      <h3 id="t-congelada">Despliegues congelados</h3>
      <p>
        Desde <span title={lock?.at ?? ''}>{cuando(lock?.at ?? null)}</span>{#if lock?.by} por <strong>{vis(lock.by)}</strong>{/if}{#if lock?.reason}: «{lock.reason}»{/if}.
      </p>
      <p class="tenue">
        El autodespliegue y «desplegar todo» la saltan; desplegarla a mano pide
        confirmarlo aparte y no la descongela. <strong>Revertir sigue
        permitido</strong>: es la salida de emergencia.
      </p>
      {#if alDescongelar}
        <button type="button" class="boton" onclick={() => alDescongelar!()}>Descongelar…</button>
      {/if}
    </section>
  {:else if estado.locked === false && puedeCongelar && alCongelar}
    <section class="seccion" aria-labelledby="t-congelar">
      <h3 id="t-congelar">Despliegues</h3>
      {#if congelando}
        <label class="campo">
          Por qué, para quien lo vea después (opcional):
          <input type="text" bind:value={motivo} maxlength="200" autocomplete="off"
                 aria-invalid={problema ? 'true' : undefined} />
        </label>
        {#if problema}<p class="mal">El motivo {problema}.</p>{/if}
        <div class="acciones">
          <button type="button" class="secundaria" onclick={() => { congelando = false; motivo = '' }}>Cancelar</button>
          <button type="button" class="boton" disabled={problema !== null}
                  onclick={() => { alCongelar!(motivo.trim() === '' ? null : motivo); congelando = false; motivo = '' }}>
            Congelar…
          </button>
        </div>
      {:else}
        <p class="tenue">
          Congelar para que ni el autodespliegue ni «desplegar todo» la toquen
          —un lanzamiento, una migración a medias—. Revertir seguirá permitido.
        </p>
        <button type="button" class="boton" onclick={() => (congelando = true)}>Congelar despliegues</button>
      {/if}
    </section>
  {/if}

  {#if estado.checks !== undefined && puedeEsperarCi && alEsperarCi}
    <section class="seccion" aria-labelledby="t-ci">
      <h3 id="t-ci">CI de GitHub</h3>
      {#if estado.autodeploy && !estado.checks}
        <!-- Lo que Orbit 1.4.9 dejó de hacer por defecto, y que `orbit doctor`
             cuenta sin arreglarlo: el automático cómodo desplegaba un commit en
             rojo de madrugada. Una app que ya estaba así sigue así hasta que
             alguien lo decida. -->
        <p class="aviso-ci">
          <strong>Se despliega sola sin esperar a la CI</strong>: un commit en
          rojo llegaría a producción. Desde Orbit 1.4.9, activar el
          autodespliegue en un repositorio de GitHub enciende esta espera por
          defecto; las apps que ya estaban en automático siguen como estaban.
        </p>
      {/if}
      {#if estado.checks}
        <p class="tenue">
          El autodespliegue y «desplegar todo» sólo despliegan un commit cuya CI
          ha terminado bien. Si no se puede preguntar a GitHub, no se despliega.
          Desplegar a mano no espera: sólo avisa si está en rojo.
        </p>
        <button type="button" class="boton" onclick={() => alEsperarCi!(false)}>Dejar de esperar a la CI…</button>
      {:else}
        <p class="tenue">
          Ahora se despliega sin mirar la CI. Esperarla sólo funciona con un
          repositorio de GitHub y con la sesión de <code>gh</code> del servidor.
        </p>
        <button type="button" class="boton" onclick={() => alEsperarCi!(true)}>Esperar a la CI…</button>
      {/if}
    </section>
  {/if}

  {#if tareas}
    <section class="seccion" aria-labelledby="t-tareas">
      <h3 id="t-tareas">Tareas programadas</h3>
      {#if tareas.tasks.length === 0}
        <!-- Una lista vacía es una respuesta: el servidor ha mirado. -->
        <p class="tenue">No declara ninguna en el <code>cron</code> de su orbit.json.</p>
      {:else}
        <table class="tabla">
          <thead>
            <tr><th scope="col">Tarea</th><th scope="col">Última</th><th scope="col">Resultado</th><th scope="col">Próxima</th></tr>
          </thead>
          <tbody>
            <!-- Por posición: el nombre lo pone el orbit.json y nadie impide
                 que se repita. -->
            {#each tareas.tasks as t, i (i)}
              <tr>
                <td class="mono">{vis(t.name)}</td>
                <!-- `null` es «nunca», no una fecha vacía. -->
                <td class="mono">{t.last_run ?? 'nunca'}</td>
                <td class="mono">{t.result ?? '·'}</td>
                <td class="mono">{t.next_run ?? '·'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>
  {/if}
</article>

<style>
  .detalle { padding: var(--e-5); }
  header { display: flex; align-items: center; gap: var(--e-3); margin-bottom: var(--e-2); }
  .ruta { margin: 0; font-size: 18px; color: var(--fg); }
  .servidor { color: var(--fg-muted); }
  .frase { margin: 0 0 var(--e-5); color: var(--fg-muted); font-size: 13px; max-width: 62ch; }
  dl { display: grid; grid-template-columns: 1fr; gap: var(--e-3); margin: 0; }
  dl > div { display: grid; grid-template-columns: 160px 1fr; gap: var(--e-3); align-items: baseline; }
  dt { color: var(--fg-faint); font-size: 12px; }
  dd { margin: 0; color: var(--fg); font-size: 13px; }
  .mono { font-family: var(--mono); }
  .tenue { color: var(--fg-muted); font-size: 12px; }
  .seccion { margin-top: var(--e-5); padding-top: var(--e-4); border-top: 1px solid var(--border); max-width: 72ch; }
  .seccion p { margin: 0 0 var(--e-2); font-size: 13px; color: var(--fg); }
  .seccion p.tenue { color: var(--fg-muted); font-size: 12px; }
  h3 { margin: 0 0 var(--e-2); font-size: 13px; font-weight: 600; color: var(--fg); }
  .campo { display: grid; gap: var(--e-1); font-size: 12px; color: var(--fg-muted); margin-bottom: var(--e-2); }
  .campo input {
    padding: var(--e-2); border-radius: var(--r-2);
    border: 1px solid var(--border-strong); background: var(--surface-2);
    font: inherit; font-size: 13px; color: var(--fg);
  }
  .mal { font-size: 12px; }
  .acciones { display: flex; gap: var(--e-2); }
  .boton, .secundaria {
    border-radius: var(--r-2); padding: var(--e-1) var(--e-3);
    font: inherit; font-size: 12px; cursor: pointer;
    background: none; border: 1px solid var(--border-strong); color: var(--fg);
  }
  .secundaria { border-color: transparent; color: var(--fg-muted); }
  .boton:disabled { opacity: .45; cursor: not-allowed; }
  .boton:focus-visible, .secundaria:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .tabla { width: 100%; border-collapse: collapse; font-size: 12px; }
  .tabla th {
    text-align: left; font-size: 11px; font-weight: 600; color: var(--fg-faint);
    padding: var(--e-1) var(--e-3) var(--e-1) 0; border-bottom: 1px solid var(--border);
  }
  .tabla td { padding: var(--e-1) var(--e-3) var(--e-1) 0; border-bottom: 1px solid var(--border); color: var(--fg); }
  code { font-family: var(--mono); font-size: 12px; }

  /* A 320 px (o al 200 % en una ventana estrecha) las columnas fijas
     empujaban la página hacia un lado: una sola columna. */
  @media (max-width: 480px) {
    dl > div { grid-template-columns: minmax(0, 1fr); }
  }
</style>
