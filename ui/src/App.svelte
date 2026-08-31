<script lang="ts">
  import ListaApps from './componentes/ListaApps.svelte'
  import DetalleApp from './componentes/DetalleApp.svelte'
  import Esqueleto from './componentes/Esqueleto.svelte'
  import Fallo from './componentes/Fallo.svelte'
  import Diagnostico from './componentes/Diagnostico.svelte'
  import VisorLog from './componentes/VisorLog.svelte'
  import DespliegueVista from './componentes/Despliegue.svelte'
  import HojaDeComando from './componentes/HojaDeComando.svelte'
  import AvisoDeCierre from './componentes/AvisoDeCierre.svelte'
  import * as vivos from './lib/vivos.svelte'
  import EntornoVista from './componentes/Entorno.svelte'
  import MonitorVista from './componentes/MonitorVista.svelte'
  import TraficoVista from './componentes/TraficoVista.svelte'
  import MetricasVista from './componentes/MetricasVista.svelte'
  import Exec from './componentes/Exec.svelte'
  import Retirar from './componentes/Retirar.svelte'
  import Revertir from './componentes/Revertir.svelte'
  import AltaServidores from './componentes/AltaServidores.svelte'
  import { periodoDelMonitor } from './lib/despliegue'
  import type { App, AppInfo, Doctor, Entorno, Log, Metricas, Monitor, Saludo, Trafico } from './lib/contrato'
  import {
    arreglar, cancelar, desplegar, diagnostico, entorno as pedirEntorno,
    entornoValor, hayPuente, log as pedirLog, monitor as pedirMonitor,
    correr, detalle as pedirDetalle, metricas as pedirMetricas, portada,
    retirar, retirarYBorrar, revertir, saludar, servidoresDelConfig, trafico as pedirTrafico,
    type AliasSsh, type ErrorDelPuente,
  } from './lib/puente'

  let servidores = $state<AliasSsh[]>([])
  let alias = $state('')
  let apps = $state<App[] | null>(null)
  let error = $state<ErrorDelPuente | null>(null)
  let cargando = $state(false)
  let elegida = $state<App | null>(null)

  // Qué se está mirando del servidor. Las apps son la portada y lo demás se
  // pide al entrar, no antes: abrir una pantalla no puede significar cuatro
  // llamadas SSH de las que tres no se van a mirar.
  let vista = $state<'apps' | 'diagnostico' | 'monitor'>('apps')
  let doctor = $state<Doctor | null>(null)
  let arreglando = $state(false)

  // El log de la app elegida. También bajo demanda.
  let log = $state<Log | null>(null)
  let entorno = $state<Entorno | null>(null)
  let monitor = $state<Monitor | null>(null)
  let periodo = $state(3)
  let trafico = $state<Trafico | null>(null)
  let metricas = $state<Metricas | null>(null)
  let detalleApp = $state<AppInfo | null>(null)

  // El alta de servidores. Vive en su propia vista porque enumerar y usar son
  // dos cosas distintas: la lista sale del ~/.ssh/config sin hablar con nadie,
  // y preguntar por uno es un gesto aparte.
  let enAlta = $state(false)
  let saludos = $state<Record<string, Saludo | null>>({})
  let comprobando = $state<string | null>(null)

  async function comprobar(a: string) {
    comprobando = a
    try {
      saludos = { ...saludos, [a]: await saludar(a) }
    } catch {
      saludos = { ...saludos, [a]: null }
    } finally {
      comprobando = null
    }
  }
  let pestana = $state<'detalle' | 'log' | 'entorno' | 'trafico' | 'exec' | 'retirar' | 'revertir' | 'despliegue'>('detalle')

  // La hoja de comando de un despliegue. Se enseña la orden literal ANTES de
  // ejecutarla: es la prueba visible de que esto sólo invoca `orbit`.
  let hoja = $state<App | null>(null)

  vivos.escuchar()

  async function lanzar(a: App) {
    hoja = null
    pestana = 'despliegue'
    const k = vivos.clave(alias, a.name)
    vivos.empezar(alias, a.name)
    try {
      vivos.terminar(k, await desplegar(alias, a.name))
      // Un despliegue cambia el estado de la app: la portada deja de ser de
      // fiar y se vuelve a pedir, en vez de parchear la fila a mano.
      apps = null
      cargar(alias)
    } catch (e) {
      const err = e as ErrorDelPuente
      // NO se llama fallo. Si se perdió el contacto, el despliegue sigue en el
      // servidor y el cliente ya no sabe qué pasó.
      vivos.perder(k, err.mensaje)
    }
  }

  async function cargar(a: string) {
    alias = a
    // La selección NO sobrevive a un cambio de servidor. `tienda` existe en
    // tres servidores y son apps distintas: conservar el nombre al cambiar
    // sería enseñar los datos de una bajo el nombre de otra, que es el
    // accidente más caro de un cliente multiservidor.
    elegida = null
    apps = null
    error = null
    doctor = null
    log = null
    entorno = null
    monitor = null
    trafico = null
    metricas = null
    vista = 'apps'
    cargando = true
    try {
      apps = (await portada(a)).apps
    } catch (e) {
      // Un fallo se pinta como un fallo. Lo que NO se hace es dejar los datos
      // viejos en pantalla sin decir que son viejos: un panel que dice «todo
      // verde» cuando no ha podido preguntar oculta un incidente en curso.
      error = e as ErrorDelPuente
    } finally {
      cargando = false
    }
  }

  async function verDiagnostico() {
    vista = 'diagnostico'
    if (doctor) return
    cargando = true
    try {
      doctor = await diagnostico(alias)
    } catch (e) {
      error = e as ErrorDelPuente
    } finally {
      cargando = false
    }
  }

  async function aplicarArreglos() {
    arreglando = true
    try {
      // Se vuelve a diagnosticar con lo que devuelve el propio comando: lo que
      // cuenta es el estado del servidor después, no lo que dijeron los
      // arreglos. Es la misma regla que sigue `orbit doctor --fix`.
      doctor = await arreglar(alias)
      // Y las apps pueden haber cambiado —un vhost regenerado cambia
      // `served`— así que la portada deja de ser de fiar.
      apps = null
    } catch (e) {
      error = e as ErrorDelPuente
    } finally {
      arreglando = false
    }
  }

  async function verMonitor() {
    vista = 'monitor'
    cargando = monitor === null
    try {
      // El periodo sale de lo que TARDÓ, no de un número elegido: `top --json`
      // cuesta ~2,1 s con 40 apps porque la CPU es la diferencia entre dos
      // lecturas, y encadenar peticiones más rápido de lo que contestan no da
      // más frescura, da una cola.
      const t0 = performance.now()
      monitor = await pedirMonitor(alias)
      periodo = periodoDelMonitor(performance.now() - t0)
    } catch (e) {
      error = e as ErrorDelPuente
    } finally {
      cargando = false
    }
  }

  // Sólo mientras se está mirando, y sólo con la ventana enfocada. Sondear una
  // pantalla que nadie ve es gastar una sesión SSH del usuario en nada.
  $effect(() => {
    if (vista !== 'monitor') return
    const t = setInterval(() => {
      if (!document.hidden) verMonitor()
    }, periodo * 1000)
    return () => clearInterval(t)
  })

  async function verEntorno(a: App) {
    pestana = 'entorno'
    if (entorno && entorno.app === a.name) return
    try {
      entorno = await pedirEntorno(alias, a.name)
    } catch (e) {
      error = e as ErrorDelPuente
    }
  }

  // El detalle se pide EN EL MOMENTO en que hace falta: es de donde sale el
  // inventario de lo que se pierde al borrar, y decirle a alguien que va a
  // perder 3 releases cuando tiene 12 es peor que no decírselo.
  async function verAdmin(a: App, cual: 'retirar' | 'revertir') {
    pestana = cual
    try {
      const [d, e] = await Promise.all([
        pedirDetalle(alias, a.name),
        cual === 'retirar' ? pedirEntorno(alias, a.name) : Promise.resolve(null),
      ])
      detalleApp = d
      if (e) entorno = e
    } catch (err) {
      error = err as ErrorDelPuente
    }
  }

  async function verTrafico(a: App) {
    pestana = 'trafico'
    if (trafico && trafico.app === a.name) return
    try {
      // Las dos a la vez: se miran juntas y son dos comandos distintos, así que
      // pedirlas en serie doblaría la espera de una pantalla sin ganar nada.
      const [t, m] = await Promise.all([
        pedirTrafico(alias, a.name),
        pedirMetricas(alias, a.name),
      ])
      trafico = t
      metricas = m
    } catch (e) {
      error = e as ErrorDelPuente
    }
  }

  async function verLog(a: App) {
    pestana = 'log'
    if (log) return
    try {
      log = await pedirLog(alias, a.name)
    } catch (e) {
      error = e as ErrorDelPuente
    }
  }

  $effect(() => {
    servidoresDelConfig().then((s) => {
      servidores = s
      if (s.length > 0 && !alias) cargar(s[0]!.alias)
    })
  })
</script>

{#if hoja}
  <HojaDeComando
    titulo="Desplegar {hoja.name}"
    servidor={alias}
    orden="orbit deploy {hoja.name}"
    consecuencia="Actualiza el clon, compila en una release nueva y sólo al final mueve el symlink. Si el build falla, la versión que está publicada ahora ni se entera. Al reiniciar el proceso puede haber uno o dos segundos sin respuesta."
    verbo="Desplegar"
    alConfirmar={() => lanzar(hoja!)}
    alCancelar={() => (hoja = null)}
  />
{/if}

<div class="marco">
  <!-- El servidor activo está SIEMPRE visible, no escondido en un desplegable
       que se lee al entrar. El accidente más caro de un cliente multiservidor
       no es un ataque: es ejecutar lo correcto contra el servidor equivocado.
       Hay precedente — la suite de pruebas de Orbit borró el vhost de una app
       de producción llamada `tienda` por eso mismo. -->
  <nav class="rail" aria-label="Servidores">
    <p class="marca">Orbit</p>
    <ul>
      {#each servidores as s (s.alias)}
        {@const enMarcha = vivos.enCurso(s.alias).length}
        <li>
          <button
            type="button"
            class="servidor"
            class:servidor--activo={alias === s.alias}
            aria-current={alias === s.alias ? 'true' : undefined}
            title={s.hostname ? `${s.usuario ?? ''}@${s.hostname}:${s.puerto ?? 22}` : undefined}
            onclick={() => cargar(s.alias)}
          >
            {s.alias}
            {#if enMarcha > 0}
              <!-- Volver a un despliegue tiene que ser un clic: mientras corre,
                   el servidor lleva su contador. -->
              <span
                class="corriendo"
                title={enMarcha === 1 ? 'un despliegue en curso' : `${enMarcha} despliegues en curso`}
              >◐ {enMarcha}</span>
            {/if}
            {#if s.salto}
              <!-- Un salto se anuncia porque cambia lo que se puede prometer
                   sobre la latencia: el saludo se paga dos veces. -->
              <span class="salto" title="Por {s.salto}">↪</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    <button type="button" class="alta" onclick={() => (enAlta = !enAlta)}>
      {enAlta ? '← volver' : '+ servidores'}
    </button>

    {#if !hayPuente()}
      <p class="aviso">
        Sin envoltorio de escritorio: estás viendo las respuestas de ejemplo del
        servidor de pruebas, no un servidor de verdad.
      </p>
    {/if}
  </nav>

  <main>
    <AvisoDeCierre />
    <header class="cabecera">
      <h1>{enAlta ? 'Servidores' : alias || '—'}</h1>
      {#if apps && vista === 'apps' && !enAlta}
        <p class="cuenta">{apps.length} {apps.length === 1 ? 'app' : 'apps'}</p>
      {/if}
      {#if !enAlta}
      <nav class="vistas" aria-label="Qué mirar de este servidor">
        <button type="button" class:activo={vista === 'apps'} onclick={() => (vista = 'apps')}>
          apps
        </button>
        <button type="button" class:activo={vista === 'diagnostico'} onclick={verDiagnostico}>
          diagnóstico
        </button>
        <button type="button" class:activo={vista === 'monitor'} onclick={verMonitor}>
          monitor
        </button>
      </nav>
      {/if}
    </header>

    <section class="panel">
      {#if enAlta}
        <AltaServidores
          alias={servidores}
          {saludos}
          {comprobando}
          alComprobar={comprobar}
          alUsar={(a) => { enAlta = false; cargar(a) }}
        />
      {:else if cargando}
        <Esqueleto />
      {:else if error}
        <Fallo {error} {alias} />
      {:else if vista === 'monitor' && monitor}
        <MonitorVista {monitor} servidor={alias} {periodo} />
      {:else if vista === 'diagnostico' && doctor}
        <Diagnostico
          {doctor}
          servidor={alias}
          {arreglando}
          alArreglar={hayPuente() ? aplicarArreglos : null}
        />
      {:else if apps}
        <ListaApps
          {apps}
          servidor={alias}
          seleccionada={elegida?.name ?? null}
          alElegir={(a) => {
            elegida = a
            // El log es de una app concreta: al cambiar de app, el que había
            // deja de valer. Conservarlo enseñaría el log de una bajo el nombre
            // de otra.
            log = null
            // El entorno es de una app concreta: al cambiar de app, el que
            // había deja de valer, y con él cualquier valor revelado.
            entorno = null
            trafico = null
            metricas = null
            detalleApp = null
            pestana = 'detalle'
          }}
        />
      {/if}
    </section>

    {#if elegida && vista === 'apps'}
      <section class="panel panel--detalle">
        <nav class="pestanas" aria-label="Qué mirar de esta app">
          <button type="button" class:activo={pestana === 'detalle'} onclick={() => (pestana = 'detalle')}>
            detalle
          </button>
          <button type="button" class:activo={pestana === 'log'} onclick={() => verLog(elegida!)}>
            log
          </button>
          <button type="button" class:activo={pestana === 'entorno'} onclick={() => verEntorno(elegida!)}>
            entorno
          </button>
          <button type="button" class:activo={pestana === 'trafico'} onclick={() => verTrafico(elegida!)}>
            tráfico
          </button>
          <button type="button" class:activo={pestana === 'exec'} onclick={() => (pestana = 'exec')}>
            exec
          </button>
          <button type="button" class:activo={pestana === 'revertir'} onclick={() => verAdmin(elegida!, 'revertir')}>
            revertir
          </button>
          <button type="button" class:activo={pestana === 'retirar'} onclick={() => verAdmin(elegida!, 'retirar')}>
            retirar
          </button>
          {#if vivos.ver(alias, elegida.name)}
            <button type="button" class:activo={pestana === 'despliegue'} onclick={() => (pestana = 'despliegue')}>
              despliegue
            </button>
          {/if}
          <button type="button" class="lanzar" onclick={() => (hoja = elegida)}>
            Desplegar
          </button>
        </nav>
        {#if pestana === 'detalle'}
          <DetalleApp app={elegida} servidor={alias} />
        {:else if pestana === 'entorno'}
          {#if entorno}
            <div class="log-envoltorio">
              <EntornoVista
                {entorno}
                app={elegida.name}
                servidor={alias}
                pedirValor={(k) => entornoValor(alias, elegida!.name, k)}
              />
            </div>
          {:else}
            <div class="log-envoltorio"><Esqueleto filas={4} /></div>
          {/if}
        {:else if pestana === 'trafico'}
          {#if trafico}
            <div class="log-envoltorio">
              <TraficoVista {trafico} servidor={alias} />
              {#if metricas}
                <h3 class="sub">Despliegues</h3>
                <MetricasVista {metricas} />
              {/if}
            </div>
          {:else}
            <div class="log-envoltorio"><Esqueleto filas={4} /></div>
          {/if}
        {:else if pestana === 'exec'}
          <div class="log-envoltorio">
            <Exec
              app={elegida.name}
              servidor={alias}
              usuario={`orbit-${elegida.name}`}
              correr={(shell, args) => correr(alias, elegida!.name, shell, args)}
            />
          </div>
        {:else if pestana === 'revertir'}
          {#if detalleApp}
            <Revertir
              info={detalleApp}
              servidor={alias}
              alRevertir={async (r) => {
                await revertir(alias, elegida!.name, r)
                // Lo que cuenta es cómo queda el servidor, no lo que dijo el
                // comando: se vuelve a preguntar en vez de parchear la fila.
                apps = null; detalleApp = null; cargar(alias)
              }}
            />
          {:else}
            <div class="log-envoltorio"><Esqueleto filas={4} /></div>
          {/if}
        {:else if pestana === 'retirar'}
          {#if detalleApp}
            <Retirar
              app={elegida.name}
              servidor={alias}
              info={detalleApp}
              {entorno}
              alCerrar={() => (pestana = 'detalle')}
              alRetirar={async (borrarDatos) => {
                if (borrarDatos) await retirarYBorrar(alias, elegida!.name)
                else await retirar(alias, elegida!.name)
                elegida = null; apps = null; cargar(alias)
              }}
            />
          {:else}
            <div class="log-envoltorio"><Esqueleto filas={4} /></div>
          {/if}
        {:else if pestana === 'despliegue'}
          {@const v = vivos.ver(alias, elegida.name)}
          {#if v}
            {#if v.error}
              <!-- Perder el contacto NO es fallar. El despliegue sigue en el
                   servidor y ya no sabemos qué pasó; decirlo es incómodo y
                   verdadero, y viene con la forma de averiguarlo. -->
              <div class="log-envoltorio">
                <p class="perdido" role="alert">
                  He perdido el contacto durante el despliegue. <strong>El estado es
                  desconocido</strong>: puede haber terminado, puede seguir. No lo
                  reintento solo — dos despliegues a la vez sobre la misma app son,
                  en el mejor caso, dos releases.
                </p>
                <p class="perdido-que">
                  Para saberlo: mira las releases y el último despliegue en la
                  pestaña de detalle. <span class="motivo">{v.error}</span>
                </p>
              </div>
            {:else}
              <DespliegueVista
                app={elegida.name}
                servidor={alias}
                progreso={v.progreso}
                resultado={v.resultado}
                crudo={v.crudo}
              />
            {/if}
          {/if}
        {:else if log}
          <div class="log-envoltorio"><VisorLog {log} app={elegida.name} /></div>
        {:else}
          <div class="log-envoltorio"><Esqueleto filas={5} /></div>
        {/if}
      </section>
    {/if}
  </main>
</div>

<style>
  .marco { display: grid; grid-template-columns: 200px 1fr; min-height: 100vh; }
  .rail {
    background: var(--surface-sunken);
    border-right: 1px solid var(--border);
    padding: var(--e-4) var(--e-3);
    display: flex; flex-direction: column;
  }
  .marca {
    margin: 0 0 var(--e-5); padding-left: var(--e-2);
    font-weight: 700; letter-spacing: .12em; text-transform: uppercase;
    font-size: 12px; color: var(--accent-text);
  }
  .rail ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .servidor {
    width: 100%; text-align: left;
    display: flex; align-items: center; justify-content: space-between; gap: var(--e-2);
    background: none; border: 0; border-radius: var(--r-2);
    padding: var(--e-2);
    font: inherit; font-size: 13px; color: var(--fg-muted); cursor: pointer;
  }
  .servidor:hover { background: var(--surface-2); color: var(--fg); }
  .servidor--activo { background: var(--surface-2); color: var(--fg); font-weight: 600; }
  .servidor:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .salto { font-size: 11px; opacity: .7; }
  .alta {
    margin-top: var(--e-3);
    background: none; border: 0; padding: var(--e-2);
    font: inherit; font-size: 12px; color: var(--accent-text);
    cursor: pointer; text-align: left;
  }
  .alta:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .aviso {
    margin-top: auto; padding: var(--e-3) var(--e-2) 0;
    font-size: 11px; line-height: 1.5; color: var(--fg-faint);
  }
  main { padding: var(--e-5) var(--e-6); background: var(--bg); }
  .cabecera { display: flex; align-items: baseline; gap: var(--e-3); margin-bottom: var(--e-5); }
  h1 { margin: 0; font-size: 20px; color: var(--fg); }
  .cuenta { margin: 0; color: var(--fg-faint); font-size: 13px; }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-1);
    padding: var(--e-4);
  }
  .panel--detalle { margin-top: var(--e-5); padding: 0; }
  .vistas, .pestanas { display: flex; gap: var(--e-1); margin-left: auto; }
  .pestanas { margin: 0; padding: var(--e-3) var(--e-5) 0; }
  .vistas button, .pestanas button {
    background: none; border: 1px solid transparent; border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .vistas button:hover, .pestanas button:hover { color: var(--fg); }
  .vistas button.activo, .pestanas button.activo { border-color: var(--border-strong); color: var(--fg); }
  .vistas button:focus-visible, .pestanas button:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .log-envoltorio { padding: var(--e-3) var(--e-5) var(--e-5); }
  .corriendo { font-family: var(--mono); font-size: 11px; color: var(--accent-text); }
  .lanzar {
    margin-left: auto;
    border: 1px solid var(--border-strong) !important;
    color: var(--fg) !important;
  }
  .perdido { margin: 0; font-size: 13px; color: var(--fg); max-width: 68ch; }
  .perdido-que { margin: var(--e-3) 0 0; font-size: 12px; color: var(--fg-muted); max-width: 68ch; }
  .motivo { font-family: var(--mono); }
  .sub { font-size: 12px; text-transform: uppercase; letter-spacing: .04em;
         color: var(--fg-faint); margin: var(--e-6) 0 var(--e-3); }
</style>
