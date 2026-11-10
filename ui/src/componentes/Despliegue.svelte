<script lang="ts" module>
  // Las ejecuciones a las que ya se les ha pedido cancelar. Fuera del
  // componente: cambiar de pestaña lo desmonta, y al volver «Cancelar» salía
  // otra vez activo para un despliegue al que ya se le había pedido parar.
  const pedidas = new WeakSet<object>()
</script>

<script lang="ts">
  import { recogerFoco } from '../lib/foco'
  import Consola from './Consola.svelte'
  import { faltanVariables, finalDe, nombreDePaso, ofreceRollback, queMirar, type Progreso } from '../lib/despliegue'
  import { salidaDeTexto, type Salida } from '../lib/consola'
  import type { Despliegue } from '../lib/contrato'

  let {
    app,
    servidor,
    progreso,
    resultado = null,
    salida = null,
    crudo = '',
    desde = null,
    alDarVariables = null,
    alVerLog = null,
    alDiagnostico = null,
    alRevertir = null,
    alCancelar = null,
  }: {
    app: string
    servidor: string
    progreso: Progreso
    resultado?: Despliegue | null
    /** La salida en vivo (`vivos.ver(…).salida`). Es también lo que identifica
     *  la ejecución: cada despliegue trae una nueva. */
    salida?: Salida | null
    /** La salida ya entera, como texto: para la galería y las pruebas. Sólo se
     *  mira si no hay `salida`. */
    crudo?: string
    /** Volver a desplegar dando las variables que faltaron. `null` donde no
     *  hay a quién pedírselas (la galería, un servidor sin puente). */
    alDarVariables?: ((faltan: string[]) => void) | null
    /** Cuándo empezó, en el reloj de este portátil. Con él el cronómetro
     *  avanza cada segundo aunque el servidor esté callado. */
    desde?: number | null
    /** Las acciones del resultado. `null` donde no hay a dónde ir (la
     *  galería): entonces el botón no se pinta, en vez de pintarse muerto. */
    alVerLog?: (() => void) | null
    alDiagnostico?: (() => void) | null
    /** Lleva a la pantalla de revertir con esa release elegida: revertir sigue
     *  pidiendo su confirmación allí, no se hace desde aquí de un clic. */
    alRevertir?: ((release: string) => void) | null
    /** Pide cancelar. Devuelve la promesa de `puente.cancelar`: `true` es que
     *  la petición llegó y el resultado vendrá por su camino; `false` o un
     *  rechazo, que no se pudo, y el botón vuelve a servir. Sin valor (la
     *  galería) cuenta como pedida. */
    alCancelar?: (() => Promise<boolean> | boolean | void) | null
  } = $props()

  const fuente = $derived(salida ?? salidaDeTexto(crudo))

  let verCrudo = $state(false)
  const final = $derived(resultado ? finalDe(resultado) : null)
  const enMarcha = $derived(resultado === null)

  // El reloj de pared. El servidor sólo cuenta el tiempo cuando emite una
  // línea, y un build de dos minutos sin decir nada dejaba el reloj parado,
  // que es exactamente lo que parece un cuelgue.
  let ahora = $state(Date.now())
  $effect(() => {
    if (!enMarcha || desde === null) return
    const t = setInterval(() => (ahora = Date.now()), 1000)
    return () => clearInterval(t)
  })
  const reloj = $derived(
    enMarcha && desde !== null
      ? Math.max(progreso.transcurrido, Math.floor((ahora - desde) / 1000))
      : progreso.transcurrido,
  )

  // «Cancelando…» vale para UNA ejecución. La primera versión era un booleano
  // que nunca volvía a `false`: tras cancelar un despliegue, el siguiente desde
  // la misma pestaña salía con el botón muerto. Se guarda a cuál se le pidió, y
  // al empezar otra (otra `salida`, u otro `desde`) deja de valer sola.
  const NINGUNA = Symbol('ninguna')
  const ejecucion = $derived<unknown>(salida ?? desde)
  let pedidaA = $state.raw<unknown>(NINGUNA)
  let aviso = $state.raw<{ a: unknown; texto: string } | null>(null)
  const cancelando = $derived(
    enMarcha && (pedidaA === ejecucion
      || (typeof ejecucion === 'object' && ejecucion !== null && pedidas.has(ejecucion))),
  )
  const avisoCancelar = $derived(aviso !== null && aviso.a === ejecucion ? aviso.texto : null)

  // Al terminar, la consola y «Cancelar» desaparecen. Si el foco estaba ahí,
  // se iba al <body> y un lector de pantalla volvía a empezar la página: se
  // lleva a la cabecera del despliegue, que es donde está el resultado.
  let articulo = $state<HTMLElement | null>(null)
  let estabaEnMarcha = false
  $effect.pre(() => {
    const ahora = enMarcha
    if (estabaEnMarcha && !ahora && articulo?.contains(document.activeElement)) {
      recogerFoco(() => articulo?.querySelector<HTMLElement>('[data-foco-despliegue]'))
    }
    estabaEnMarcha = ahora
  })

  async function pedirCancelar() {
    const esta = ejecucion
    pedidaA = esta
    if (typeof esta === 'object' && esta !== null) pedidas.add(esta)
    aviso = null
    let ok: boolean
    try {
      ok = (await alCancelar!()) !== false
    } catch {
      ok = false
    }
    if (!ok && typeof esta === 'object' && esta !== null) pedidas.delete(esta)
    if (!ok && pedidaA === esta) {
      pedidaA = NINGUNA
      aviso = { a: esta, texto: 'No se ha podido cancelar: el despliegue sigue en marcha.' }
    }
  }

  /** Lo que hace el botón primario de cada paso fallido. Antes los dos se
   *  pintaban sin `onclick`: «Ver el log» con un build roto no hacía nada. */
  function accionPrimaria(paso: string): (() => void) | null {
    const a = queMirar(paso)?.accion
    if (a === 'Ver el log') return alVerLog
    if (a === 'Ir al diagnóstico') return alDiagnostico
    if (a === 'Volver a la anterior')
      return resultado?.previous && alRevertir ? () => alRevertir!(resultado!.previous!) : null
    // «Revisar el repositorio»: lo que hay que leer es el fatal de git, que
    // está en la salida cruda.
    return () => (verCrudo = true)
  }

  const fallo = $derived(resultado && resultado.ok !== true ? resultado.failed_step : null)
  const mirar = $derived(queMirar(fallo))

  /** El estado de un paso, con el que se rompió marcado como tal: el lector
   *  de progreso sólo sabe «pendiente / haciendo / hecho». */
  function estadoDe(p: { id: string; estado: string }): 'pendiente' | 'haciendo' | 'hecho' | 'fallo' {
    if (fallo !== null && p.id === fallo && p.estado !== 'hecho') return 'fallo'
    return p.estado === 'hecho' || p.estado === 'haciendo' ? p.estado : 'pendiente'
  }
  const GLIFO = { pendiente: '○', haciendo: '◐', hecho: '✓', fallo: '✕' } as const
  // El glifo y el color no se oyen: la palabra va al lado, para el lector.
  const PALABRA = { pendiente: 'pendiente', haciendo: 'en curso', hecho: 'hecho', fallo: 'falló' } as const

  function dur(s: number | null): string {
    if (s === null) return ''
    if (s < 60) return `${s} s`
    return `${Math.floor(s / 60)} min ${s % 60} s`
  }

  /** Lo que se anuncia al terminar. La región existe SIEMPRE y cambia de
   *  texto: una región que se monta con su contenido dentro no la lee casi
   *  ningún lector de pantalla, y el resultado se perdía. */
  const anuncio = $derived.by(() => {
    if (avisoCancelar) return avisoCancelar
    if (!resultado) return ''
    switch (final) {
      case 'bien': return `${app}: desplegada en ${dur(resultado.duration_s)}.`
      case 'recuperado': return `${app}: desplegada al segundo intento.`
      case 'negado': return `${app}: no se ha desplegado, el servidor no ha empezado.`
      case 'faltan': return `${app}: se ha parado antes del build, faltan variables.`
      case 'revertido': return `${app}: tu web sigue en pie. El despliegue falló y Orbit volvió a la release anterior.`
      default: return `${app}: el despliegue ha fallado${fallo ? ` en ${nombreDePaso(fallo)}` : ''}.`
    }
  })
</script>

<article class="despliegue" bind:this={articulo}>
  <header>
    <!-- Siempre `servidor : app`. El nombre de una app no identifica nada por
         sí solo: `tienda` existe en tres servidores. -->
    <p class="ruta" tabindex="-1" data-foco-despliegue><span class="servidor">{servidor}</span> : <strong>{app}</strong></p>
    <p class="reloj" aria-live="off">{dur(reloj)}</p>
  </header>

  {#if resultado === null}
    <div class="en-marcha">
      <p class="estado"><span class="pulso" aria-hidden="true"></span>Desplegando…</p>
      {#if alCancelar}
        <button
          type="button"
          class="secundaria"
          disabled={cancelando}
          onclick={pedirCancelar}
        >{cancelando ? 'Cancelando…' : 'Cancelar'}</button>
      {/if}
    </div>
    {#if avisoCancelar}<p class="nota">{avisoCancelar}</p>{/if}
  {/if}

  <!-- La barra está ponderada: seis pasos no valen un sexto cada uno, y el
       build es el 70-85 % del tiempo. Una lineal se quedaría clavada en el 33 %
       durante dos minutos y luego correría hasta el final. -->
  <div
    class="barra"
    class:barra--estimada={progreso.sinHistorico}
    role="progressbar"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(progreso.avance * 100)}
    aria-label="Progreso del despliegue de {app}"
  >
    <div class="relleno" style="width: {progreso.avance * 100}%"></div>
  </div>
  {#if progreso.sinHistorico && resultado === null}
    <!-- Orbit se calla la tendencia con menos de seis builds, porque dos datos
         no son una tendencia. La interfaz respeta ese silencio en vez de
         rellenarlo, y lo dice. -->
    <p class="nota">Sin histórico suficiente: la estimación es aproximada.</p>
  {/if}

  <ol class="pasos">
    {#each progreso.pasos as p (p.id)}
      {@const e = estadoDe(p)}
      <li class="paso paso--{e}">
        <span class="glifo" aria-hidden="true">{GLIFO[e]}</span>
        <span class="nombre" title={p.id}>{nombreDePaso(p.id)}</span>
        <span class="sr">: {PALABRA[e]}.</span>
        <!-- El cronómetro de cada paso se conserva al terminar: en tres
             despliegues alguien aprende cuánto tarda su build. -->
        <span class="tiempo">{p.segundos !== null ? dur(p.segundos) : ''}</span>
        <span class="que">{p.texto}</span>
      </li>
    {/each}
  </ol>

  {#if progreso.rotas > 0}
    <p class="nota">
      {progreso.rotas} {progreso.rotas === 1 ? 'línea de progreso no se ha entendido' : 'líneas de progreso no se han entendido'}.
      El despliegue sigue: una línea rota no lo tumba.
    </p>
  {/if}

  <p class="sr" role="status" aria-live="polite">{anuncio}</p>

  {#if resultado}
    <section class="final final--{final}">
      {#if final === 'bien'}
        <p class="titular">Desplegada en {dur(resultado.duration_s)}.</p>
        <p class="detalle">Release <code>{resultado.release}</code>.</p>
      {:else if final === 'recuperado'}
        <!-- Ni rojo ni exactamente igual que un éxito limpio: cuatro
             «recovered» seguidos son un patrón que alguien debería mirar. -->
        <p class="titular">Desplegada en {dur(resultado.duration_s)}, al segundo intento.</p>
        <p class="detalle">
          Orbit reconoció el fallo del build, lo arregló y reintentó una vez. El
          arreglo queda apuntado, así que el próximo sale a la primera.
        </p>
      {:else if final === 'negado'}
        <!-- No ha fallado nada: el servidor no empezó. Pintarlo como un fallo
             mandaría a buscar una avería que no existe. -->
        <p class="titular">No se ha desplegado: el servidor no ha empezado.</p>
        <p class="detalle">
          {resultado.error ?? 'Se negó sin decir por qué.'} La web es exactamente la de antes.
        </p>
      {:else if final === 'faltan'}
        {@const faltan = faltanVariables(resultado)}
        <p class="titular">Se ha parado antes del build: faltan variables.</p>
        <p class="detalle">
          {faltan.length === 1 ? 'Le falta' : 'Le faltan'} <code>{faltan.join(', ')}</code>,
          y sin nadie al teclado no hay a quién preguntárselas. No se ha tocado
          nada: la web sigue como estaba.
        </p>
      {:else if final === 'revertido'}
        <!-- La primera línea NO habla del fallo. Es lo único que hace falta
             saber en el primer segundo, y ponerlo debajo del error sería
             enterrar la buena noticia. -->
        <p class="titular">Tu web sigue en pie.</p>
        <p class="detalle">
          El despliegue falló en <code title={resultado.failed_step}>{nombreDePaso(resultado.failed_step ?? '')}</code> y Orbit volvió a
          <code>{resultado.previous}</code>, que es lo que se está sirviendo.
        </p>
        {#if resultado.error}<pre>{resultado.error}</pre>{/if}
      {:else}
        <p class="titular">
          El despliegue ha fallado{#if resultado.failed_step}
            en <code title={resultado.failed_step}>{nombreDePaso(resultado.failed_step)}</code>{/if}.
        </p>
        {#if mirar}
          <p class="detalle">{mirar.mira}</p>
        {/if}
        {#if resultado.error}<pre>{resultado.error}</pre>{/if}
      {/if}

      <div class="acciones">
        {#if final === 'faltan'}
          {#if alDarVariables}
            <!-- Lo único que hace falta, y por eso es la acción primaria: no
                 hay un log que mirar ni un diagnóstico que abrir. -->
            <button type="button" class="primaria" onclick={() => alDarVariables!(faltanVariables(resultado!))}>
              Darlas y volver a desplegar
            </button>
          {/if}
        {:else if mirar && fallo}
          {@const hacer = accionPrimaria(fallo)}
          {#if hacer}
            <button type="button" class="primaria" onclick={hacer}>{mirar.accion}</button>
          {/if}
        {/if}
        {#if ofreceRollback(resultado) && alRevertir && resultado.previous}
          <!-- No es la acción primaria. En un fallo, lo primero es ENTENDER; y
               en buena parte de los casos volver atrás ni siquiera es lo que se
               quiere: si el build falló, `current` no se movió y no hay nada a
               lo que volver. -->
          <button type="button" class="secundaria" onclick={() => alRevertir!(resultado!.previous!)}>
            Volver a {resultado.previous}
          </button>
        {/if}
        <button type="button" class="secundaria" onclick={() => (verCrudo = !verCrudo)}>
          {verCrudo ? 'Ocultar' : 'Ver'} la salida cruda
        </button>
      </div>
    </section>
  {/if}

  {#if enMarcha || verCrudo}
    <!-- Sin traducir y tal cual. Es lo que se pega en un issue. Mientras corre
         se ve siempre: cuatro minutos mirando una barra sin saber qué dice el
         servidor es justo lo que esta pantalla existe para evitar. -->
    <Consola salida={fuente} vivo={enMarcha} />
  {/if}
</article>

<style>
  .despliegue { padding: var(--e-5); }
  header { display: flex; align-items: baseline; justify-content: space-between; }
  .ruta { margin: 0; font-size: 18px; color: var(--fg); }
  .servidor { color: var(--fg-muted); }
  .reloj { margin: 0; font-family: var(--mono); font-size: 13px; color: var(--fg-muted); }
  .en-marcha { display: flex; align-items: center; justify-content: space-between; gap: var(--e-3); margin: var(--e-1) 0 var(--e-4); }
  .estado { margin: 0; color: var(--fg-muted); font-size: 13px; display: flex; align-items: center; gap: var(--e-2); }
  .pulso {
    width: 8px; height: 8px; border-radius: 50%; background: var(--accent-fill);
    animation: latido 1.6s ease-in-out infinite;
  }
  @keyframes latido { 0%, 100% { opacity: .35; transform: scale(.85) } 50% { opacity: 1; transform: scale(1) } }
  @media (prefers-reduced-motion: reduce) { .pulso { animation: none } .relleno { transition: none } }
  .barra {
    height: 8px; border-radius: 999px; overflow: hidden;
    background: var(--surface-sunken); margin-bottom: var(--e-2);
  }
  .relleno { height: 100%; background: var(--accent-fill); transition: width .25s linear; }
  .barra--estimada .relleno {
    background: repeating-linear-gradient(
      90deg, var(--accent-fill) 0 8px, color-mix(in srgb, var(--accent-fill) 55%, transparent) 8px 16px);
  }
  .nota { margin: 0 0 var(--e-4); font-size: 12px; color: var(--fg-faint); }
  .pasos { list-style: none; margin: var(--e-4) 0 0; padding: 0; }
  .paso {
    display: grid; grid-template-columns: 16px 92px 74px 1fr;
    gap: var(--e-2); align-items: baseline;
    padding: 3px 0; font-size: 13px;
  }
  .glifo { font-family: var(--mono); font-size: 11px; }
  .nombre { font-family: var(--mono); font-size: 12px; color: var(--fg); }
  .tiempo { font-family: var(--mono); font-size: 12px; color: var(--fg-faint); text-align: right; }
  .que { color: var(--fg-muted); }
  .paso--pendiente .nombre, .paso--pendiente .que { color: var(--fg-faint); }
  .final {
    margin-top: var(--e-5); padding: var(--e-4);
    border: 1px solid var(--border); border-radius: var(--r-3);
  }
  .ruta:focus { outline: none; }
  .ruta:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .titular { margin: 0; font-size: 15px; font-weight: 600; color: var(--titular, var(--fg)); }
  .detalle { margin: var(--e-2) 0 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  code { font-family: var(--mono); font-size: 12px; }
  pre {
    margin: var(--e-3) 0 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; white-space: pre-wrap; color: var(--fg-muted);
    max-height: 300px; overflow: auto;
  }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  .acciones { display: flex; gap: var(--e-2); margin-top: var(--e-4); flex-wrap: wrap; }
  .primaria, .secundaria {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; cursor: pointer;
  }
  .primaria { background: var(--accent-fill); color: var(--on-accent); border: 0; font-weight: 600; }
  .secundaria { background: none; color: var(--fg); border: 1px solid var(--border-strong); }
  .secundaria:disabled { opacity: .6; cursor: progress; }
  .primaria:focus-visible, .secundaria:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
