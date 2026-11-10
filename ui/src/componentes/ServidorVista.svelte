<script lang="ts">
  // Lo del servidor que no es una app.
  //
  // La cabecera —el host y sus servicios— llega en la MISMA llamada que la
  // portada (`status --json`) y hasta ahora se tiraba: enseñarla no cuesta ni
  // un saludo SSH. Las secciones de debajo son una orden cada una y se piden
  // **al abrirlas**, no al entrar: abrir una pantalla no puede significar cinco
  // sesiones SSH de las que cuatro no se van a mirar.
  //
  // Todo es de sólo lectura salvo «verificar las copias», que tampoco cambia
  // nada en el servidor: abre cada fichero y dice si `orbit restore` lo
  // aceptaría.
  import Fallo from './Fallo.svelte'
  import Esqueleto from './Esqueleto.svelte'
  import {
    bytes, duracion, epoca, marcarInvisibles, num, seQuedaCorto,
    type Colas, type Copias, type BasesDeDatos, type Host, type Redirecciones,
    type ServicioDelHost, type Verificacion, type Vigilancia,
  } from '../lib/contrato'
  import {
    basesDeDatos, colas, copias, redirecciones, verificarCopias, vigilancia,
    type ErrorDelPuente,
  } from '../lib/puente'

  let {
    servidor,
    host = null,
    servicios = null,
    version = null,
  }: {
    servidor: string
    /** `null` es «este servidor no lo ha dicho», y se pinta así. */
    host?: Host | null
    servicios?: ServicioDelHost[] | null
    version?: string | null
  } = $props()

  type Seccion = 'vigilancia' | 'copias' | 'bases' | 'redirecciones' | 'colas'
  let abierta = $state<Seccion>('vigilancia')

  // Cada sección con su dato, su fallo y su carga: un PostgreSQL caído no
  // tumba la vigilancia.
  let vig = $state<Vigilancia | null>(null)
  let cop = $state<Copias | null>(null)
  let ver = $state<Verificacion | null>(null)
  let bds = $state<BasesDeDatos | null>(null)
  let red = $state<Redirecciones | null>(null)
  let col = $state<Colas | null>(null)
  let fallos = $state<Partial<Record<Seccion | 'verificar', ErrorDelPuente>>>({})
  let pidiendo = $state<Partial<Record<Seccion | 'verificar', boolean>>>({})

  // `seQuedaCorto` y no `!sabe`: con la versión sin saber —no ha llegado aún, o
  // el saludo falló— las copias se piden igual. Esconderlas por no haber podido
  // preguntar era quitarle a alguien una sección que su servidor sí tiene, y si
  // la versión llegaba después nadie volvía a pedirlas: la sección se quedaba
  // en blanco.
  const copiasNoEstan = $derived(seQuedaCorto(version, 'contratoCompleto'))

  // El servidor que se mira manda: si cambia, todo lo de antes es de otro.
  let de = ''
  $effect(() => {
    if (servidor === de) return
    de = servidor
    vig = cop = ver = bds = red = col = null
    fallos = {}
    pidiendo = {}
    void abrir(abierta, true)
  })

  // La sección abierta sin dato, sin fallo y sin nada en marcha se pide. Cubre
  // lo que el clic no puede: que la versión llegue —o cambie— con la sección ya
  // abierta.
  $effect(() => {
    const s = abierta
    const ya = { vigilancia: vig, copias: cop, bases: bds, redirecciones: red, colas: col }[s]
    if (ya || pidiendo[s] || fallos[s]) return
    if (s === 'copias' && copiasNoEstan) return
    void abrir(s)
  })

  async function abrir(s: Seccion, forzar = false) {
    abierta = s
    const yo = servidor
    const ya = { vigilancia: vig, copias: cop, bases: bds, redirecciones: red, colas: col }[s]
    if (ya && !forzar) return
    if (s === 'copias' && copiasNoEstan) return
    pidiendo = { ...pidiendo, [s]: true }
    const { [s]: _, ...resto } = fallos
    fallos = resto
    try {
      if (s === 'vigilancia') { const r = await vigilancia(yo); if (yo === servidor) vig = r }
      else if (s === 'copias') { const r = await copias(yo); if (yo === servidor) cop = r }
      else if (s === 'bases') { const r = await basesDeDatos(yo); if (yo === servidor) bds = r }
      else if (s === 'redirecciones') { const r = await redirecciones(yo); if (yo === servidor) red = r }
      else { const r = await colas(yo); if (yo === servidor) col = r }
    } catch (e) {
      if (yo === servidor) fallos = { ...fallos, [s]: e as ErrorDelPuente }
    } finally {
      if (yo === servidor) pidiendo = { ...pidiendo, [s]: false }
    }
  }

  async function verificar() {
    const yo = servidor
    pidiendo = { ...pidiendo, verificar: true }
    const { verificar: _, ...resto } = fallos
    fallos = resto
    try {
      const r = await verificarCopias(yo)
      if (yo === servidor) ver = r
    } catch (e) {
      if (yo === servidor) fallos = { ...fallos, verificar: e as ErrorDelPuente }
    } finally {
      if (yo === servidor) pidiendo = { ...pidiendo, verificar: false }
    }
  }

  const porFichero = $derived(new Map((ver?.backups ?? []).map((b) => [b.file, b])))

  /** Un porcentaje que se pueda pintar, o `null` si no se sabe. */
  function parte(usado: number | null | undefined, total: number | null | undefined): number | null {
    if (typeof usado !== 'number' || typeof total !== 'number' || total <= 0) return null
    return Math.min(100, Math.max(0, (usado / total) * 100))
  }
  // Con `?.`: lo que llega del puente ya está validado, pero la galería y las
  // pruebas pasan objetos a mano, y un host a medias no puede tumbar la vista.
  const memoria = $derived(host ? parte(host.memory_kb?.used, host.memory_kb?.total) : null)
  const disco = $derived(
    host
      ? typeof host.disk_kb?.use_percent === 'number'
        ? host.disk_kb.use_percent
        : parte(host.disk_kb?.used, host.disk_kb?.total)
      : null,
  )
  /** Las tres cargas, sólo si son las tres y son números. Vacía no es «carga
   *  cero»: no se ha podido leer. */
  const carga = $derived(
    host && Array.isArray(host.load) && host.load.length === 3
      && host.load.every((x) => typeof x === 'number' && Number.isFinite(x))
      ? host.load
      : null,
  )
  /** Lo que identifica algo y viene del servidor —un nombre, un fichero, un
   *  destino— se pinta con lo invisible a la vista. La prosa (`message`) no:
   *  viene en el idioma del servidor y lleva tildes. */
  const vis = marcarInvisibles
  const kb = (n: number | null | undefined) => (typeof n === 'number' ? bytes(n * 1024) : '·')

  function claseVigilado(estado: string): string {
    return estado === 'ok' || estado === 'fallo' || estado === 'rendido' ? `vigilado--${estado}` : 'vigilado--otro'
  }
  function hora(d: Date | null): string {
    return d ? d.toLocaleString() : 'no se sabe'
  }
  /** Una fecha ISO del servidor. Una que no se entiende es «·», no «Invalid
   *  Date». */
  function fecha(iso: string): string {
    const d = new Date(iso)
    return Number.isNaN(d.getTime()) ? '·' : d.toLocaleString()
  }
  function nombreDeFichero(f: string): string {
    return f.split('/').pop() ?? f
  }

  const SECCIONES: { id: Seccion; texto: string }[] = [
    { id: 'vigilancia', texto: 'vigilancia' },
    { id: 'copias', texto: 'copias' },
    { id: 'bases', texto: 'bases de datos' },
    { id: 'redirecciones', texto: 'redirecciones' },
    { id: 'colas', texto: 'colas' },
  ]
</script>

<section class="servidor" aria-label="El servidor {servidor}">
  {#if host}
    <div class="host">
      <div class="identidad">
        <p class="nombre">{vis(host.hostname)}</p>
        <p class="tenue mono">{vis(host.ip)} · encendido hace {duracion(host.uptime_seconds)}</p>
      </div>

      <div class="medidas">
        <div class="medida">
          <p class="et">Carga</p>
          {#if carga}
            <p class="valor mono">{carga.map((x) => x.toFixed(2)).join(' · ')}</p>
            <p class="tenue">1, 5 y 15 min</p>
          {:else}
            <!-- Vacía no es «carga cero»: no se ha podido leer. -->
            <p class="valor">·</p>
            <p class="tenue">no se ha podido leer</p>
          {/if}
        </div>

        <div class="medida">
          <p class="et">Memoria</p>
          <p class="valor mono">{kb(host.memory_kb?.used)} <span class="tenue">de {kb(host.memory_kb?.total)}</span></p>
          <div
            class="medidor"
            class:medidor--alto={memoria !== null && memoria >= 90}
            role="meter"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={memoria === null ? undefined : Math.round(memoria)}
            aria-label="Memoria usada"
          >
            {#if memoria !== null}<span style="width: {memoria}%"></span>{/if}
          </div>
        </div>

        <div class="medida">
          <p class="et">Disco <span class="tenue mono">{vis(host.disk_kb?.mount ?? '·')}</span></p>
          <p class="valor mono">{kb(host.disk_kb?.used)} <span class="tenue">de {kb(host.disk_kb?.total)}</span></p>
          <div
            class="medidor"
            class:medidor--alto={disco !== null && disco >= 90}
            role="meter"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={disco === null ? undefined : Math.round(disco)}
            aria-label="Disco usado"
          >
            {#if disco !== null}<span style="width: {disco}%"></span>{/if}
          </div>
        </div>
      </div>

      {#if servicios && servicios.length > 0}
        <ul class="servicios" aria-label="Servicios del sistema">
          <!-- Por posición: los nombres los pone el servidor, y una clave
               repetida revienta en desarrollo y se come filas en producción. -->
          {#each servicios as s, i (i)}
            <li class="servicio {s.active ? 'servicio--activo' : 'servicio--inactivo'}">
              <span class="punto" aria-hidden="true"></span>
              <span class="mono">{vis(s.name)}</span>
              <span class="sr">{s.active ? 'activo' : 'no activo'}</span>
            </li>
          {/each}
        </ul>
        <p class="nota">
          Un servicio apagado no es una avería por sí mismo: ufw, fail2ban o
          PostgreSQL pueden no usarse en este servidor.
        </p>
      {/if}
    </div>
  {:else}
    <p class="tenue">Este servidor no ha dicho cómo está el host en la portada.</p>
  {/if}

  <nav class="secciones" aria-label="Qué mirar del servidor">
    {#each SECCIONES as s (s.id)}
      <button type="button" class:activo={abierta === s.id} aria-pressed={abierta === s.id} onclick={() => abrir(s.id)}>
        {s.texto}
      </button>
    {/each}
    <button
      type="button"
      class="recargar"
      disabled={pidiendo[abierta]}
      onclick={() => abrir(abierta, true)}
      aria-label="Volver a pedir {abierta}"
      title="Volver a pedir"
    >↻</button>
  </nav>

  <div class="cuerpo">
    <!-- Lo de la versión va antes que el fallo: en un servidor que sabemos
         viejo, que `backup list --json` falle es lo esperado y no lo que hay
         que contar. -->
    {#if abierta === 'copias' && copiasNoEstan}
      <p class="tenue">
        {version ? `Este servidor tiene Orbit ${version}.` : 'No sé qué versión de Orbit tiene este servidor.'}
        Las copias se leen con <code>orbit backup list --json</code>, que llegó en la 1.3.7.
      </p>
    {:else if fallos[abierta]}
      <Fallo error={fallos[abierta]!} alias={servidor} />
    {:else if pidiendo[abierta] && !{ vigilancia: vig, copias: cop, bases: bds, redirecciones: red, colas: col }[abierta]}
      <Esqueleto filas={3} />
    {:else if abierta === 'vigilancia' && vig}
      <p class="resumen">
        {#if !vig.timer_active}
          <strong>El vigilante está parado</strong>: nadie mira si las webs contestan.
        {:else if vig.summary.down > 0}
          <strong>{vig.summary.down} {vig.summary.down === 1 ? 'caída' : 'caídas'}</strong> de {vig.summary.total} vigiladas.
        {:else if vig.summary.warning > 0}
          {vig.summary.warning} con avisos de {vig.summary.total} vigiladas.
        {:else}
          Todo lo vigilado contesta ({vig.summary.total}).
        {/if}
      </p>
      {#if vig.subjects.length === 0}
        <p class="tenue">No vigila nada todavía.</p>
      {:else}
        <ul class="filas">
          {#each vig.subjects as v, i (i)}
            <li class="fila {claseVigilado(v.state)}">
              <span class="punto" aria-hidden="true"></span>
              <span class="mono">{vis(v.subject)}</span>
              <span class="estado">{vis(v.state)}{v.tries !== null && v.tries > 0 ? ` · ${v.tries} intentos` : ''}</span>
              <span class="tenue">desde {hora(epoca(v.since))}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {:else if abierta === 'copias' && cop}
      <div class="barra-copias">
        <p class="resumen">
          {cop.total} {cop.total === 1 ? 'copia' : 'copias'} · {bytes(cop.bytes)}
          {#if cop.keep_days !== null}· se guardan {cop.keep_days} días{/if}
          <span class="tenue mono">en {vis(cop.dir)}</span>
        </p>
        <button type="button" class="accion" disabled={pidiendo.verificar || cop.total === 0} onclick={verificar}>
          {pidiendo.verificar ? 'Verificando…' : ver ? 'Verificar otra vez' : 'Verificar todas'}
        </button>
      </div>
      {#if fallos.verificar}
        <Fallo error={fallos.verificar} alias={servidor} />
      {:else if ver}
        <p class="resumen" role="status">
          {#if ver.ok}
            Las {ver.good} se pueden restaurar.
          {:else}
            <strong>{ver.bad} de {ver.total} no se podrían restaurar.</strong>
          {/if}
        </p>
      {/if}
      {#if cop.backups.length === 0}
        <p class="tenue">No hay ninguna copia en este servidor.</p>
      {:else}
        <ul class="filas">
          {#each cop.backups as c, i (i)}
            {@const v = porFichero.get(c.file)}
            <li class="fila {v ? (v.ok ? 'copia--buena' : 'copia--mala') : 'copia--sin-mirar'}">
              <span class="punto" aria-hidden="true"></span>
              <span class="mono" title={vis(c.file)}>{vis(nombreDeFichero(c.file))}</span>
              <span class="estado">{c.kind === 'config' ? 'configuración global' : c.app === null ? '·' : vis(c.app)}</span>
              <span class="tenue">{fecha(c.created)} · {bytes(c.size_bytes)}</span>
              {#if v}<span class="mensaje">{v.message}</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    {:else if abierta === 'bases' && bds}
      {#if bds.databases.length === 0}
        <p class="tenue">No hay ninguna base de datos.</p>
      {:else}
        <table class="tabla">
          <caption class="sr">Bases de datos de {servidor}</caption>
          <thead><tr><th scope="col">Base</th><th scope="col">Dueño</th><th scope="col" class="num">Tamaño</th></tr></thead>
          <tbody>
            {#each bds.databases as b, i (i)}
              <tr><td class="mono">{vis(b.name)}</td><td class="mono tenue">{vis(b.owner)}</td><td class="num mono">{bytes(b.size_bytes)}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if abierta === 'redirecciones' && red}
      {#if red.redirects.length === 0}
        <p class="tenue">No hay ninguna redirección.</p>
      {:else}
        <table class="tabla">
          <caption class="sr">Redirecciones de {servidor}</caption>
          <thead><tr><th scope="col">App</th><th scope="col">Desde</th><th scope="col">Hacia</th><th scope="col" class="num">Código</th></tr></thead>
          <tbody>
            {#each red.redirects as r, i (i)}
              <tr>
                <td class="mono">{vis(r.app)}</td>
                <!-- En una redirección de dominio, Orbit pone en `from` el
                     nombre de la app, no el dominio: se dice qué es. -->
                <td class="mono">{r.kind === 'domain' ? 'todo el dominio' : vis(r.from)}</td>
                <td class="mono">{vis(r.to)}{r.query ? '' : ' (sin la consulta)'}</td>
                <td class="num mono">{num(r.code)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if abierta === 'colas' && col}
      <p class="resumen">
        {#if col.timer_active}
          El trabajador corre cada {col.every_minutes ?? '·'} min, hasta {col.max_seconds ?? '·'} s cada vez.
        {:else}
          <strong>El temporizador de las colas está parado.</strong>
        {/if}
      </p>
      {#if col.apps.length === 0}
        <p class="tenue">Ninguna app tiene las colas encendidas.</p>
      {:else}
        <ul class="filas">
          {#each col.apps as a, i (i)}
            <li class="fila"><span class="mono">{vis(a.app)}</span><span class="estado">{vis(a.type)}</span><span class="tenue mono">{a.connection === null ? 'conexión por defecto' : vis(a.connection)}</span></li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
</section>

<style>
  .servidor { display: grid; gap: var(--e-5); }
  .host { display: grid; gap: var(--e-4); }
  .identidad .nombre { margin: 0; font-size: 16px; font-weight: 600; color: var(--fg); }
  .tenue { color: var(--fg-faint); font-size: 12px; margin: 0; }
  .mono { font-family: var(--mono); }
  .medidas { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--e-4); }
  .medida { padding: var(--e-3); border: 1px solid var(--border); border-radius: var(--r-2); background: var(--surface-sunken); }
  .et { margin: 0 0 var(--e-1); font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--fg-faint); }
  .valor { margin: 0; font-size: 14px; color: var(--fg); }
  .medidor {
    margin-top: var(--e-2); height: 6px; border-radius: 999px; overflow: hidden;
    background: var(--surface-2);
  }
  .medidor span {
    display: block; height: 100%; border-radius: inherit;
    background: var(--medidor, var(--accent-fill));
    transition: width var(--t-normal) ease-out;
  }
  .servicios { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: var(--e-2); }
  .servicio {
    display: inline-flex; align-items: center; gap: var(--e-2);
    padding: 2px var(--e-3); border: 1px solid var(--border); border-radius: 999px; font-size: 12px; color: var(--fg-muted);
  }
  .punto { width: 7px; height: 7px; border-radius: 50%; background: var(--punto, var(--fg-faint)); flex: none; }
  .nota { margin: 0; font-size: 11px; color: var(--fg-faint); max-width: 72ch; }
  .secciones { display: flex; gap: var(--e-1); flex-wrap: wrap; border-top: 1px solid var(--border); padding-top: var(--e-4); }
  .secciones button, .accion {
    background: none; border: 1px solid transparent; border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px; color: var(--fg-muted); cursor: pointer;
  }
  .secciones button:hover { color: var(--fg); }
  .secciones button.activo { border-color: var(--border-strong); color: var(--fg); }
  .secciones button:focus-visible, .accion:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .recargar { margin-left: auto; }
  .recargar:disabled, .accion:disabled { opacity: .5; cursor: progress; }
  .accion { border-color: var(--border-strong); color: var(--fg); padding: var(--e-1) var(--e-3); }
  .cuerpo { display: grid; gap: var(--e-3); }
  .resumen { margin: 0; font-size: 13px; color: var(--fg-muted); }
  .barra-copias { display: flex; align-items: center; justify-content: space-between; gap: var(--e-3); flex-wrap: wrap; }
  .filas { list-style: none; margin: 0; padding: 0; }
  .fila {
    display: grid; grid-template-columns: 10px minmax(160px, 1.2fr) minmax(100px, .8fr) 1fr;
    gap: var(--e-3); align-items: baseline;
    padding: var(--e-2) 0; border-top: 1px solid var(--border); font-size: 13px;
  }
  .fila .punto { align-self: center; }
  .estado { color: var(--fg-muted); }
  .mensaje { grid-column: 2 / -1; font-size: 12px; color: var(--fg-muted); }
  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabla th {
    text-align: left; font-size: 11px; font-weight: 600; letter-spacing: .04em; text-transform: uppercase;
    color: var(--fg-faint); padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border);
  }
  .tabla td { padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border); }
  .num { text-align: right; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  code { font-family: var(--mono); font-size: 12px; }
  @media (prefers-reduced-motion: reduce) { .medidor span { transition: none; } }
</style>
