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
  import AnadirServidor from './componentes/AnadirServidor.svelte'
  import InstalarOrbit from './componentes/InstalarOrbit.svelte'
  import AsistenteNueva from './componentes/AsistenteNueva.svelte'
  import Pasada from './componentes/Pasada.svelte'
  import Comparar from './componentes/Comparar.svelte'
  import OrbitJson from './componentes/OrbitJson.svelte'
  import LoDetectado from './componentes/LoDetectado.svelte'
  import Desenlace from './componentes/Desenlace.svelte'
  import { clasificar, type Borrador, type Desenlace as Final } from './lib/asistente'
  import { periodoDelMonitor } from './lib/despliegue'
  import type { App, AppInfo, Doctor, Entorno, Log, Lote, Metricas, Monitor, Saludo, Trafico } from './lib/contrato'
  import {
    arreglar, cancelar, desplegar, diagnostico, entorno as pedirEntorno,
    entornoValor, hayPuente, log as pedirLog, monitor as pedirMonitor,
    correr, detalle as pedirDetalle, metricas as pedirMetricas, portada,
    retirar, retirarYBorrar, revertir, saludar, servidoresDelConfig, trafico as pedirTrafico,
    crear, resolver, desplegarTodo,
    servidoresGuardados, guardarServidor, olvidarServidor,
    requisitosDeInstalacion, instalarOrbit,
    type AliasSsh, type ErrorDelPuente, type Resolucion,
    type ServidorPropio, type Requisitos, type Instalacion,
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

  // Los servidores añadidos a mano. La primera versión sacaba los servidores
  // **sólo** del ~/.ssh/config, y eso dejaba sin salida a quien no tuviera ese
  // fichero — que en Windows es casi todo el mundo.
  let propios = $state<ServidorPropio[]>([])
  let anadiendo = $state(false)
  let guardando = $state(false)
  let falloAlGuardar = $state<ErrorDelPuente | null>(null)

  // Instalar Orbit en un servidor que no lo tiene.
  let instalandoEn = $state<string | null>(null)
  let requisitos = $state<Requisitos | null>(null)
  let mirandoRequisitos = $state(false)
  let instalando = $state(false)
  let salidaInstalacion = $state('')
  let resultadoInstalacion = $state<Instalacion | null>(null)
  let paraDeEscucharInstalacion: (() => void) | null = null

  /** Todos los alias ocupados, de las dos fuentes. Un alias repetido haría que
   *  `ssh` resolviera el del fichero y la aplicación enseñara un servidor
   *  mientras habla con otro. */
  const aliasOcupados = $derived([
    ...servidores.map((s) => s.alias),
    ...propios.map((s) => s.alias),
  ])

  async function cargarPropios() {
    try {
      propios = await servidoresGuardados()
    } catch {
      // Que no se pueda leer la lista no tumba la aplicación: se sigue con los
      // del ~/.ssh/config, que es lo que había antes de que esto existiera.
      propios = []
    }
  }

  async function anadirServidor(b: {
    alias: string
    host: string
    usuario: string
    puerto: number
    clave: string
  }) {
    guardando = true
    falloAlGuardar = null
    try {
      propios = await guardarServidor({
        alias: b.alias.trim(),
        host: b.host.trim(),
        usuario: b.usuario.trim(),
        puerto: b.puerto,
        clave: b.clave.trim() === '' ? null : b.clave.trim(),
        binario: null,
      })
      anadiendo = false
      // Y se le pregunta enseguida: acabar de añadir un servidor y no saber si
      // contesta es dejar a alguien delante de una fila muda.
      void comprobar(b.alias.trim())
    } catch (e) {
      falloAlGuardar = e as ErrorDelPuente
    } finally {
      guardando = false
    }
  }

  async function quitarServidor(a: string) {
    try {
      propios = await olvidarServidor(a)
      delete saludos[a]
    } catch (e) {
      error = e as ErrorDelPuente
    }
  }

  /** Mira qué hay antes de instalar nada. */
  async function abrirInstalacion(a: string) {
    instalandoEn = a
    requisitos = null
    resultadoInstalacion = null
    salidaInstalacion = ''
    mirandoRequisitos = true
    try {
      requisitos = await requisitosDeInstalacion(a)
    } catch (e) {
      error = e as ErrorDelPuente
      instalandoEn = null
    } finally {
      mirandoRequisitos = false
    }
  }

  async function lanzarInstalacion() {
    const a = instalandoEn
    if (!a) return
    instalando = true
    salidaInstalacion = ''
    resultadoInstalacion = null

    if (hayPuente()) {
      const { listen } = await import('@tauri-apps/api/event')
      const clave = `${a}:!instalar`
      paraDeEscucharInstalacion = await listen<[string, string]>('orbit://progreso', (e) => {
        const [k, linea] = e.payload
        if (k === clave) salidaInstalacion += linea + '\n'
      })
    }

    try {
      resultadoInstalacion = await instalarOrbit(a)
      // El saludo de ese servidor ya no vale: acaba de cambiar lo que hay.
      delete saludos[a]
      if (resultadoInstalacion.version) void comprobar(a)
    } catch (e) {
      error = e as ErrorDelPuente
    } finally {
      paraDeEscucharInstalacion?.()
      paraDeEscucharInstalacion = null
      instalando = false
    }
  }

  function cerrarInstalacion() {
    instalandoEn = null
    requisitos = null
    resultadoInstalacion = null
    salidaInstalacion = ''
    instalando = false
    paraDeEscucharInstalacion?.()
    paraDeEscucharInstalacion = null
  }

  // El asistente de web nueva. Vive aquí y no dentro de una pestaña de app
  // porque todavía no hay app: es la única pantalla que crea al sujeto del que
  // habla el resto de la interfaz.
  let enNueva = $state(false)
  let creando = $state(false)
  // Las líneas tal como llegan. `orbit new` no tiene `--json`, así que esto es
  // prosa y se enseña como prosa: fingir una barra de progreso a partir de un
  // texto que no la lleva sería inventarse el avance.
  let salidaNueva = $state<string[]>([])
  let finalNueva = $state<Final | null>(null)
  let configNueva = $state<Record<string, string> | null>(null)
  let nombreNueva = $state('')
  let dns = $state<Resolucion | null>(null)
  let resolviendoDns = $state(false)
  let paraDeEscuchar: (() => void) | null = null

  async function comprobarDns(dominio: string) {
    resolviendoDns = true
    dns = null
    try {
      dns = await resolver(dominio, alias)
    } catch {
      // Que no se pueda resolver no es un error de la pantalla: es una de las
      // respuestas posibles, y la pinta el propio asistente como «no lo sé».
      dns = { del_dominio: [], del_servidor: [], coinciden: null }
    } finally {
      resolviendoDns = false
    }
  }

  /**
   * Crea la web y **le vuelve a preguntar al servidor** qué ha quedado.
   *
   * No se lee ni el código de salida ni la prosa: `orbit new` puede terminar en
   * 1 con la aplicación creada, registrada y con vhost, y su resumen distingue
   * los casos en castellano. Al acabar se pide `info --json`, que sí tiene
   * contrato, y de ahí sale el final — que son siete, y cinco parciales.
   */
  async function crearWeb(b: Borrador) {
    creando = true
    salidaNueva = []
    finalNueva = null
    configNueva = null
    nombreNueva = b.nombre.trim()

    // La prosa llega por el mismo canal que el progreso de un despliegue, con
    // la clave `alias:nombre`.
    if (hayPuente()) {
      const { listen } = await import('@tauri-apps/api/event')
      const clave = `${alias}:${nombreNueva}`
      paraDeEscuchar = await listen<[string, string]>('orbit://progreso', (e) => {
        const [k, linea] = e.payload
        if (k === clave) salidaNueva = [...salidaNueva, linea]
      })
    }

    try {
      await crear(alias, b)
    } catch (e) {
      // Aquí sólo llegan los fallos de transporte: no haber llegado al
      // servidor, o que la clave de host haya cambiado. Un `new` que termina
      // regular NO pasa por aquí, porque su código de salida no es informativo.
      error = e as ErrorDelPuente
      creando = false
      paraDeEscuchar?.()
      return
    } finally {
      paraDeEscuchar?.()
      paraDeEscuchar = null
    }

    try {
      const d = await pedirDetalle(alias, nombreNueva)
      configNueva = d.config
      finalNueva = clasificar(nombreNueva, d, dns?.coinciden ?? null)
    } catch {
      // Que `info` no encuentre la app es el final F6, y es un dato, no un
      // fallo de la pantalla: quiere decir que no ha llegado a crearse nada.
      finalNueva = clasificar(nombreNueva, null)
    }
    creando = false
    // La portada ya no dice la verdad: hay una app más, o no la hay.
    await cargar(alias)
  }

  // La pasada por todas las apps. Vive aquí, al lado del asistente, porque las
  // dos son operaciones del SERVIDOR y no de una app: son las dos únicas cosas
  // que este cliente hace sin haber elegido antes de qué app se habla.
  let enPasada = $state(false)
  let pasando = $state(false)
  let modoPasada = $state<'si-cambia' | 'todo' | null>(null)
  let crudoPasada = $state('')
  let lote = $state<Lote | null>(null)
  let paraDeEscucharPasada: (() => void) | null = null

  /**
   * Lanza la pasada.
   *
   * `soloSiCambia` es la mitad del significado de la orden, no una opción: sin
   * ella se recompilan **todas** las apps, hayan cambiado o no. Por eso llega
   * desde la pantalla como una elección explícita entre dos botones y nunca con
   * un valor por defecto.
   */
  async function lanzarPasada(soloSiCambia: boolean) {
    pasando = true
    modoPasada = soloSiCambia ? 'si-cambia' : 'todo'
    crudoPasada = ''
    lote = null

    // Misma clave que usa el envoltorio: `alias:*`. El asterisco no puede ser
    // el nombre de ninguna app —la regla del servidor empieza por [a-z0-9]— así
    // que no puede chocar con la clave de un despliegue suelto.
    if (hayPuente()) {
      const { listen } = await import('@tauri-apps/api/event')
      const clave = `${alias}:*`
      paraDeEscucharPasada = await listen<[string, string]>('orbit://progreso', (e) => {
        const [k, linea] = e.payload
        if (k === clave) crudoPasada += linea + '\n'
      })
    }

    try {
      lote = await desplegarTodo(alias, soloSiCambia)
    } catch (e) {
      error = e as ErrorDelPuente
    } finally {
      paraDeEscucharPasada?.()
      paraDeEscucharPasada = null
      pasando = false
    }
    // La portada ya no dice la verdad: hay releases nuevas y estados nuevos.
    if (!error) await cargar(alias)
  }

  function pararPasada() {
    // El asterisco otra vez: es la app con la que se registró en `Vivos`.
    void cancelar(alias, '*')
  }

  // Comparar con otro servidor. Los dos lados se guardan por separado y con su
  // alias al lado: la clave de este cliente es `servidor:app` y nunca la app
  // sola, y ésta es la única pantalla donde dos servidores están a la misma
  // altura.
  let enComparar = $state(false)
  let otro = $state<string | null>(null)
  let appsOtro = $state<App[] | null>(null)
  let falloOtro = $state<ErrorDelPuente | null>(null)
  let comparando = $state(false)

  async function compararCon(x: string) {
    otro = x
    // A null y no a []: mientras no conteste no se sabe lo que tiene, y una
    // lista vacía querría decir «no tiene ninguna app».
    appsOtro = null
    falloOtro = null
    comparando = true
    try {
      appsOtro = (await portada(x)).apps
    } catch (e) {
      // El fallo se guarda **aparte** del error general de la aplicación: que
      // el otro servidor no conteste no rompe la pantalla del que sí, y la
      // comparación tiene que poder decir exactamente eso.
      falloOtro = e as ErrorDelPuente
    } finally {
      comparando = false
    }
  }

  function cerrarComparar() {
    enComparar = false
    otro = null
    appsOtro = null
    falloOtro = null
    comparando = false
  }

  function cerrarPasada() {
    enPasada = false
    pasando = false
    modoPasada = null
    crudoPasada = ''
    lote = null
    paraDeEscucharPasada?.()
    paraDeEscucharPasada = null
  }

  function cerrarNueva() {
    enNueva = false
    creando = false
    salidaNueva = []
    finalNueva = null
    configNueva = null
    dns = null
    paraDeEscuchar?.()
    paraDeEscuchar = null
  }

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
  let pestana = $state<'detalle' | 'log' | 'entorno' | 'trafico' | 'exec' | 'retirar' | 'revertir' | 'despliegue' | 'descriptor'>('detalle')

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

  /**
   * El descriptor, para poder generar el `orbit.json` de una app que ya
   * funciona.
   *
   * Pide las dos cosas a la vez porque las dos hacen falta y son de la misma
   * app: `info` trae la configuración y `env list` los **nombres** de las
   * variables. Los valores no se piden, y no por prudencia: el contrato no los
   * deja pasar.
   *
   * Si `env list` falla, el descriptor se enseña igual y el fichero sale sin su
   * bloque `env`, diciendo que falta. Media respuesta es mejor que ninguna
   * mientras se sepa cuál es la mitad que falta.
   */
  async function verDescriptor(a: App) {
    pestana = 'descriptor'
    try {
      const [d, e] = await Promise.all([
        detalleApp?.name === a.name ? Promise.resolve(detalleApp) : pedirDetalle(alias, a.name),
        entorno?.app === a.name ? Promise.resolve(entorno) : pedirEntorno(alias, a.name).catch(() => null),
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
    // Las DOS fuentes. Si no hay ninguna en ninguna de las dos, se abre
    // directamente la pantalla de servidores: una portada vacía sin explicación
    // es lo que hacía que esta aplicación pareciera rota al abrirla por primera
    // vez, y es literalmente lo que pasaba en un Windows recién instalado.
    void Promise.all([servidoresDelConfig(), servidoresGuardados()]).then(([config, mios]) => {
      servidores = config
      propios = mios
      const primero = mios[0]?.alias ?? config[0]?.alias
      if (primero && !alias) cargar(primero)
      else if (!primero) enAlta = true
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
      <h1>
        {enAlta
          ? 'Servidores'
          : enNueva
            ? 'Nueva web'
            : enPasada
              ? 'Desplegar todo'
              : enComparar
                ? 'Comparar'
                : alias || '—'}
      </h1>
      {#if apps && vista === 'apps' && !enAlta && !enNueva && !enPasada && !enComparar}
        <p class="cuenta">{apps.length} {apps.length === 1 ? 'app' : 'apps'}</p>
      {/if}
      {#if !enAlta && !enNueva && !enPasada && !enComparar}
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
        {#if alias}
          <button type="button" class="nueva" onclick={() => { enNueva = true; dns = null }}>
            nueva web
          </button>
          <button type="button" onclick={() => (enPasada = true)}>desplegar todo</button>
          {#if servidores.length > 1}
            <button type="button" onclick={() => (enComparar = true)}>comparar</button>
          {/if}
        {/if}
      </nav>
      {/if}
    </header>

    <section class="panel">
      {#if enComparar}
        <Comparar
          a={alias}
          b={otro}
          appsA={apps ?? []}
          appsB={appsOtro}
          fallo={falloOtro}
          cargando={comparando}
          candidatos={servidores.map((x) => x.alias).filter((x) => x !== alias)}
          alElegir={compararCon}
          alCerrar={cerrarComparar}
        />
      {:else if enPasada}
        <Pasada
          servidor={alias}
          apps={apps ?? []}
          crudo={crudoPasada}
          resultado={lote}
          corriendo={pasando}
          modo={modoPasada}
          alLanzar={lanzarPasada}
          alCancelar={pararPasada}
          alCerrar={cerrarPasada}
        />
      {:else if enNueva}
        {#if finalNueva}
          <!-- Terminado. El final sale de preguntarle al servidor, no de leer
               la prosa: son siete y cinco son parciales, y decir «ha fallado»
               cuando hay una web publicada sin certificado es la peor de las
               respuestas posibles. -->
          <div class="acabada">
            <Desenlace d={finalNueva} app={nombreNueva} />
            {#if configNueva}
              <LoDetectado app={nombreNueva} config={configNueva} />
            {/if}
            {#if salidaNueva.length > 0}
              <details>
                <summary>Lo que dijo el servidor</summary>
                <pre class="prosa">{salidaNueva.join('\n')}</pre>
              </details>
            {/if}
            <button type="button" class="cerrar-nueva" onclick={cerrarNueva}>Cerrar</button>
          </div>
        {:else if creando}
          <div class="creando">
            <p class="que">
              Creando «{nombreNueva}» en {alias}. Clona, instala, compila y
              despliega: puede tardar unos minutos.
            </p>
            <!-- Prosa, y enseñada como prosa. `orbit new` no tiene `--progress`
                 ni `--json`, así que aquí no hay pasos que contar y una barra
                 sería un número inventado. -->
            <pre class="prosa">{salidaNueva.join('\n') || 'esperando al servidor…'}</pre>
          </div>
        {:else}
          <AsistenteNueva
            servidor={alias}
            resolucion={dns}
            resolviendo={resolviendoDns}
            alResolver={comprobarDns}
            alCrear={crearWeb}
            alCerrar={cerrarNueva}
          />
        {/if}
      {:else if enAlta}
        {#if instalandoEn}
          <InstalarOrbit
            alias={instalandoEn}
            {requisitos}
            mirando={mirandoRequisitos}
            {instalando}
            salida={salidaInstalacion}
            resultado={resultadoInstalacion}
            alInstalar={lanzarInstalacion}
            alCancelar={() => cancelar(instalandoEn!, '!instalar')}
            alCerrar={cerrarInstalacion}
          />
        {:else if anadiendo}
          <AnadirServidor
            yaUsados={aliasOcupados}
            {guardando}
            fallo={falloAlGuardar}
            alGuardar={anadirServidor}
            alCerrar={() => { anadiendo = false; falloAlGuardar = null }}
          />
        {:else}
          <AltaServidores
            alias={servidores}
            {propios}
            {saludos}
            {comprobando}
            alComprobar={comprobar}
            alUsar={(a) => { enAlta = false; cargar(a) }}
            alAnadir={() => { anadiendo = true; falloAlGuardar = null }}
            alOlvidar={quitarServidor}
            alInstalar={abrirInstalacion}
          />
        {/if}
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

    {#if elegida && vista === 'apps' && !enNueva && !enPasada && !enComparar}
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
          <button type="button" class:activo={pestana === 'descriptor'} onclick={() => verDescriptor(elegida!)}>
            orbit.json
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
        {:else if pestana === 'descriptor'}
          <OrbitJson app={elegida.name} info={detalleApp} {entorno} />
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
  .nueva { margin-left: var(--e-3); }
  .creando, .acabada { display: grid; gap: var(--e-4); }
  .creando .que { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 72ch; }
  .prosa {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    white-space: pre-wrap; max-height: 24rem; overflow-y: auto;
  }
  .acabada summary { font-size: 12px; color: var(--fg-muted); cursor: pointer; }
  .cerrar-nueva {
    justify-self: start;
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
</style>
