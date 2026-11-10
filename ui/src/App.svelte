<script lang="ts">
  import { untrack } from 'svelte'
  import ListaApps from './componentes/ListaApps.svelte'
  import DetalleApp from './componentes/DetalleApp.svelte'
  import Esqueleto from './componentes/Esqueleto.svelte'
  import Fallo from './componentes/Fallo.svelte'
  import Diagnostico from './componentes/Diagnostico.svelte'
  import VisorLog from './componentes/VisorLog.svelte'
  import DespliegueVista from './componentes/Despliegue.svelte'
  import HojaDeComando from './componentes/HojaDeComando.svelte'
  import HojaDeDespliegue from './componentes/HojaDeDespliegue.svelte'
  import AvisoDeCierre from './componentes/AvisoDeCierre.svelte'
  import * as vivos from './lib/vivos.svelte'
  import * as historial from './lib/historial.svelte'
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
  import ServidorVista from './componentes/ServidorVista.svelte'
  import Paleta from './componentes/Paleta.svelte'
  import Frontera from './componentes/Frontera.svelte'
  import Consola from './componentes/Consola.svelte'
  import { Salida } from './lib/consola'
  import type { Accion } from './lib/paleta'
  import { devolverFoco } from './lib/foco'
  import { clasificar, type Borrador, type Desenlace as Final } from './lib/asistente'
  import {
    argvDeCongelar, argvDeDescongelar, argvDeEsperarCi, ordenLegible, periodoDelMonitor,
    chocaConPalabraDeOrbit,
    type VariableNueva,
  } from './lib/despliegue'
  import {
    sabe, seQuedaCorto, nombreOperable,
    type Host, type ServicioDelHost,
    type App, type AppInfo, type ComprobacionDeEntorno, type Doctor, type Entorno, type Log,
    type Lote, type Metricas, type Monitor, type Saludo, type Tareas, type Trafico,
  } from './lib/contrato'
  import {
    arreglar, cancelar, desplegar, diagnostico, entorno as pedirEntorno,
    entornoValor, hayPuente, log as pedirLog, monitor as pedirMonitor,
    correr, detalle as pedirDetalle, metricas as pedirMetricas, portada,
    retirar, retirarYBorrar, revertir, saludar, servidoresDelConfig, trafico as pedirTrafico,
    crear, resolver, desplegarTodo,
    servidoresGuardados, guardarServidor, olvidarServidor,
    requisitosDeInstalacion, instalarOrbit,
    version as pedirVersion, comprobarEntorno, tareas as pedirTareas,
    congelar, descongelar, esperarCi,
    type AliasSsh, type ErrorDelPuente, type Resolucion,
    type ServidorPropio, type Requisitos, type Instalacion,
  } from './lib/puente'

  let servidores = $state<AliasSsh[]>([])
  let alias = $state('')
  let apps = $state<App[] | null>(null)
  let error = $state<ErrorDelPuente | null>(null)
  // El error de la PORTADA, y sólo de ella. Lo de cada vista, cada pestaña y
  // cada flujo va aparte: la primera versión lo metía todo aquí, y un `top` que
  // fallaba una vez cambiaba la lista entera por una pantalla de fallo hasta
  // cambiar de servidor, mientras la pestaña que había fallado se quedaba con
  // su esqueleto para siempre. Y los fallos de crear una web o de la pasada se
  // guardaban aquí y no se pintaban en ningún sitio.
  let errorVista = $state<ErrorDelPuente | null>(null)
  let errorPestana = $state<ErrorDelPuente | null>(null)
  let errorFlujo = $state<ErrorDelPuente | null>(null)
  let cargando = $state(false)
  let elegida = $state<App | null>(null)

  // Qué se está mirando del servidor. Las apps son la portada y lo demás se
  // pide al entrar, no antes: abrir una pantalla no puede significar cuatro
  // llamadas SSH de las que tres no se van a mirar.
  let vista = $state<'apps' | 'diagnostico' | 'monitor' | 'servidor'>('apps')
  // El host y sus servicios: llegan con la portada, en la misma llamada.
  let host = $state<Host | null>(null)
  let servicios = $state<ServicioDelHost[] | null>(null)
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
  let tareasApp = $state<Tareas | null>(null)
  // Lo que contestó la última orden sobre la app elegida —congelar, la CI—, con
  // la frase de Orbit tal cual. Un error aquí no tapa la lista: es de una app.
  let avisoDetalle = $state<{ ok: boolean; texto: string } | null>(null)

  // La versión de Orbit de cada servidor. Se pide UNA vez por sesión y en
  // paralelo con la portada —son 72 ms en frío y ~13 con la conexión
  // multiplexada—, y de ella salen los botones que sólo existen desde cierta
  // versión. `null` es «no he podido saberlo»: entonces no se ofrecen.
  let versiones = $state<Record<string, string | null>>({})
  const versionActual = $derived(versiones[alias] ?? null)

  // Cada carga de la portada lleva un turno. Si alguien cambia de servidor
  // mientras la anterior está en vuelo, la respuesta vieja llega DESPUÉS y,
  // sin esto, pintaba las apps de un servidor bajo el nombre de otro: el
  // accidente más caro de un cliente multiservidor, provocado por la red y no
  // por nadie. Lo mismo para todo lo que se pide de una app.
  let turno = 0
  // `app` con valor por defecto y no `app?:`: el compilador de Svelte 5.42 deja
  // el `?` de un parámetro opcional en lo que genera, y el build se rompe con
  // un error de sintaxis que svelte-check no ve.
  function sigue(a: string, app: string | null = null): boolean {
    return alias === a && (app === null || elegida?.name === app)
  }

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
  // El rail enseña los DOS orígenes. La primera versión sólo pintaba los del
  // ~/.ssh/config, así que un servidor añadido a mano se podía usar desde el
  // alta pero no aparecía en el rail: no había forma de volver a él, ni de ver
  // que era el activo, ni su contador de despliegues en curso.
  // Sin repetidos: un alias guardado a mano y luego añadido también al
  // ~/.ssh/config salía dos veces, y la lista entera se caía por la clave
  // repetida. Gana el guardado, que es el que usa el núcleo al conectar.
  const enElRail = $derived.by<AliasSsh[]>(() => {
    const vistos = new Set<string>()
    const r: AliasSsh[] = []
    for (const s of [
      ...propios.map((p) => ({
        alias: p.alias, hostname: p.host, usuario: p.usuario, puerto: p.puerto,
        salto: null, identidad: p.clave,
      })),
      ...servidores,
    ]) {
      if (vistos.has(s.alias)) continue
      vistos.add(s.alias)
      r.push(s)
    }
    return r
  })

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
      errorFlujo = e as ErrorDelPuente
    }
  }

  /** Mira qué hay antes de instalar nada. */
  /**
   * Escucha el progreso de una clave (`servidor:app`).
   *
   * El núcleo manda las líneas **por tandas** —`[clave, líneas]`, cada 50 ms
   * como mucho—: una por evento eran treinta mil despachos en un build largo.
   * Se acepta también una línea suelta, la forma anterior.
   *
   * Si no se puede escuchar, la orden se lanza igual y sólo se pierde la
   * salida en vivo. La primera versión hacía el `listen` antes del `try`, y
   * sin el permiso de eventos en las capacidades de Tauri tiraba la pantalla
   * entera: crear una web o instalar Orbit no llegaba a empezar.
   */
  async function escucharProgreso(clave: string, alLlegar: (lineas: string[]) => void): Promise<(() => void) | null> {
    if (!hayPuente()) return null
    try {
      const { listen } = await import('@tauri-apps/api/event')
      return await listen<[string, string[] | string]>('orbit://progreso', (e) => {
        const [k, l] = e.payload
        if (k === clave) alLlegar(Array.isArray(l) ? l : [l])
      })
    } catch (e) {
      console.warn('sin salida en vivo para', clave, e)
      return null
    }
  }

  async function abrirInstalacion(a: string) {
    instalandoEn = a
    errorFlujo = null
    requisitos = null
    resultadoInstalacion = null
    salidaInstalacion = ''
    mirandoRequisitos = true
    try {
      requisitos = await requisitosDeInstalacion(a)
    } catch (e) {
      errorFlujo = e as ErrorDelPuente
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
    paraDeEscucharInstalacion = await escucharProgreso(`${a}:!instalar`, (ls) => {
      salidaInstalacion += ls.join('\n') + '\n'
    })

    try {
      resultadoInstalacion = await instalarOrbit(a)
      // El saludo de ese servidor ya no vale: acaba de cambiar lo que hay. Y
      // su versión tampoco, que es de donde salen los botones que ofrece.
      delete saludos[a]
      delete versiones[a]
      if (resultadoInstalacion.version) void comprobar(a)
    } catch (e) {
      errorFlujo = e as ErrorDelPuente
    } finally {
      paraDeEscucharInstalacion?.()
      paraDeEscucharInstalacion = null
      instalando = false
    }
  }

  function cerrarInstalacion() {
    errorFlujo = null
    instalandoEn = null
    requisitos = null
    resultadoInstalacion = null
    salidaInstalacion = ''
    instalando = false
    paraDeEscucharInstalacion?.()
    paraDeEscucharInstalacion = null

    // Al cerrar una vista, el foco va a su título y no se queda en <body>.
    devolverFoco(null)
  }

  // El asistente de web nueva. Vive aquí y no dentro de una pestaña de app
  // porque todavía no hay app: es la única pantalla que crea al sujeto del que
  // habla el resto de la interfaz.
  let enNueva = $state(false)
  let creando = $state(false)
  // Las líneas tal como llegan. `orbit new` no tiene `--json`, así que esto es
  // prosa y se enseña como prosa: fingir una barra de progreso a partir de un
  // texto que no la lleva sería inventarse el avance.
  // Fuera de `$state`: la consola se entera sola de lo que llega, y meter
  // treinta mil líneas en un arreglo reactivo era copiarlo en cada una.
  const salidaNueva = new Salida()
  let finalNueva = $state<Final | null>(null)
  let configNueva = $state<Record<string, string> | null>(null)
  let nombreNueva = $state('')
  let dns = $state<Resolucion | null>(null)
  let resolviendoDns = $state(false)
  let paraDeEscuchar: (() => void) | null = null

  // Cada consulta lleva su turno: se lanza al salir del campo y al cambiar de
  // paso, y la respuesta de un dominio anterior que llegara tarde decidía el
  // veredicto del DNS del que se está creando.
  let turnoDns = 0
  async function comprobarDns(dominio: string) {
    const t = ++turnoDns
    resolviendoDns = true
    dns = null
    let r: Resolucion
    try {
      r = await resolver(dominio, alias)
    } catch {
      // Que no se pueda resolver no es un error de la pantalla: es una de las
      // respuestas posibles, y la pinta el propio asistente como «no lo sé».
      r = { del_dominio: [], del_servidor: [], coinciden: null }
    }
    if (t !== turnoDns) return
    dns = r
    resolviendoDns = false
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
    // El servidor se fija AQUÍ y se usa ése hasta el final. La primera versión
    // leía `alias` después de cada `await`: si alguien cambiaba de servidor a
    // mitad, la web se creaba en uno y luego se le preguntaba por ella al
    // otro, que contestaba «no existe» —el final F6, «no se ha creado nada»—
    // e invitaba a crearla otra vez.
    const yo = alias
    const nombre = b.nombre.trim()
    creando = true
    errorFlujo = null
    salidaNueva.vaciar()
    finalNueva = null
    configNueva = null
    nombreNueva = nombre

    // La prosa llega por el mismo canal que el progreso de un despliegue, con
    // la clave `alias:nombre`. Se añade sin copiar el arreglo entero: cada
    // línea lo copiaba y lo volvía a unir, y 30.000 líneas eran 12 s.
    paraDeEscuchar = await escucharProgreso(`${yo}:${nombre}`, (ls) => {
      salidaNueva.anotarVarias(ls)
    })

    try {
      await crear(yo, b)
    } catch (e) {
      // Aquí sólo llegan los fallos de transporte: no haber llegado al
      // servidor, o que la clave de host haya cambiado. Un `new` que termina
      // regular NO pasa por aquí, porque su código de salida no es informativo.
      errorFlujo = e as ErrorDelPuente
      creando = false
      return
    } finally {
      paraDeEscuchar?.()
      paraDeEscuchar = null
    }

    try {
      const d = await pedirDetalle(yo, nombre)
      configNueva = d.config
      finalNueva = clasificar(nombre, d, dns?.coinciden ?? null)
    } catch {
      // Que `info` no encuentre la app es el final F6, y es un dato, no un
      // fallo de la pantalla: quiere decir que no ha llegado a crearse nada.
      finalNueva = clasificar(nombre, null)
    }
    creando = false
    anunciar(`Nueva web ${nombre} en ${yo}: ${finalNueva.titulo}.`)
    // La portada ya no dice la verdad: hay una app más, o no la hay.
    if (sigue(yo)) await cargar(yo)
  }

  // La pasada por todas las apps. Vive aquí, al lado del asistente, porque las
  // dos son operaciones del SERVIDOR y no de una app: son las dos únicas cosas
  // que este cliente hace sin haber elegido antes de qué app se habla.
  let enPasada = $state(false)
  let pasando = $state(false)
  let modoPasada = $state<'si-cambia' | 'todo' | null>(null)
  // Leída línea a línea y una sola vez: la primera versión acumulaba el texto
  // y la pantalla lo volvía a leer entero con cada evento (18 s de CPU en una
  // pasada de 40 apps).
  const pasadaViva = new vivos.PasadaEnVivo()
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
  // El servidor de la pasada en curso. Parar tiene que ir a ÉSE: la primera
  // versión cancelaba `alias:*` del servidor que se estuviera mirando, y tras
  // cambiar de servidor la pasada ya no se podía parar.
  let pasadaEn = $state('')

  async function lanzarPasada(soloSiCambia: boolean) {
    const yo = alias
    pasadaEn = yo
    pasando = true
    errorFlujo = null
    modoPasada = soloSiCambia ? 'si-cambia' : 'todo'
    pasadaViva.vaciar()
    lote = null

    // Misma clave que usa el envoltorio: `alias:*`. El asterisco no puede ser
    // el nombre de ninguna app —la regla del servidor empieza por [a-z0-9]— así
    // que no puede chocar con la clave de un despliegue suelto.
    paraDeEscucharPasada = await escucharProgreso(`${yo}:*`, (ls) => {
      pasadaViva.anotar(ls)
    })

    try {
      lote = await desplegarTodo(yo, soloSiCambia)
      anunciar(`Desplegar todo en ${yo}: terminado.`)
    } catch (e) {
      errorFlujo = e as ErrorDelPuente
      anunciar(`Desplegar todo en ${yo}: ha fallado.`)
    } finally {
      paraDeEscucharPasada?.()
      paraDeEscucharPasada = null
      pasando = false
    }
    // La portada ya no dice la verdad: hay releases nuevas y estados nuevos.
    if (!errorFlujo && sigue(yo)) await refrescar()
  }

  function pararPasada() {
    // El asterisco otra vez: es la app con la que se registró en `Vivos`.
    if (pasadaEn) void cancelar(pasadaEn, '*')
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
    // Sólo cuenta la respuesta del servidor que sigue elegido: uno lento al
    // que se renunció contestaba después y pintaba sus apps bajo el nombre
    // del que se eligió luego.
    const vale = () => enComparar && otro === x
    try {
      const r = (await portada(x)).apps
      if (vale()) appsOtro = r
    } catch (e) {
      // El fallo se guarda **aparte** del error general de la aplicación: que
      // el otro servidor no conteste no rompe la pantalla del que sí, y la
      // comparación tiene que poder decir exactamente eso.
      if (vale()) falloOtro = e as ErrorDelPuente
    } finally {
      if (vale()) comparando = false
    }
  }

  function cerrarComparar() {
    enComparar = false
    otro = null
    appsOtro = null
    falloOtro = null
    comparando = false

    // Al cerrar una vista, el foco va a su título y no se queda en <body>.
    devolverFoco(null)
  }

  function cerrarPasada() {
    errorFlujo = null
    enPasada = false
    pasando = false
    modoPasada = null
    pasadaViva.vaciar()
    lote = null
    paraDeEscucharPasada?.()
    paraDeEscucharPasada = null

    // Al cerrar una vista, el foco va a su título y no se queda en <body>.
    devolverFoco(null)
  }

  function cerrarNueva() {
    errorFlujo = null
    enNueva = false
    creando = false
    salidaNueva.vaciar()
    finalNueva = null
    configNueva = null
    dns = null
    paraDeEscuchar?.()
    paraDeEscuchar = null

    // Al cerrar una vista, el foco va a su título y no se queda en <body>.
    devolverFoco(null)
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
  // El servidor de la hoja, fijado al abrirla. La hoja pintaba el `alias`
  // vivo: si algo cambiaba de servidor con ella abierta, decía «en bravo» y
  // la orden corría en alfa.
  let hojaEn = $state('')
  let comprobacion = $state<ComprobacionDeEntorno | null>(null)
  let estadoComprobacion = $state<'no-sabe' | 'mirando' | 'hecho' | 'no-se-pudo'>('no-sabe')
  let faltanDeAntes = $state<string[]>([])

  vivos.escuchar()

  /**
   * Abre la hoja del despliegue y, mientras se lee, pregunta qué le falta.
   *
   * `env check` cuesta una llamada corta y ahorra la cara: un despliegue que
   * el servidor va a parar antes del build porque falta una obligatoria, y que
   * sin terminal no tiene a quién preguntársela. Se lanza en paralelo con la
   * hoja ya abierta, así que no retrasa nada.
   */
  async function prepararDespliegue(a: App, faltan: string[] = []) {
    // Uno a la vez por app. Dos sobre la misma app mezclaban su progreso en el
    // mismo registro, y en el servidor el segundo sólo esperaba al cerrojo del
    // primero: lo útil es volver al que ya corre.
    if (vivos.corriendo(alias, a.name)) {
      pestana = 'despliegue'
      return
    }
    hoja = a
    hojaEn = alias
    faltanDeAntes = faltan
    comprobacion = null
    if (!sabe(versionActual, 'entornoDeclarado')) {
      estadoComprobacion = 'no-sabe'
      return
    }
    estadoComprobacion = 'mirando'
    const yo = alias
    try {
      const c = await comprobarEntorno(yo, a.name)
      if (hojaEn !== yo || hoja?.name !== a.name) return
      comprobacion = c
      estadoComprobacion = 'hecho'
    } catch {
      if (hojaEn === yo && hoja?.name === a.name) estadoComprobacion = 'no-se-pudo'
    }
  }

  function cerrarHoja() {
    hoja = null
    comprobacion = null
    faltanDeAntes = []
  }

  async function lanzar(a: App, entorno: VariableNueva[] = [], forzar = false, en = alias) {
    cerrarHoja()
    const yo = en
    if (sigue(yo)) pestana = 'despliegue'
    if (vivos.corriendo(yo, a.name)) return
    const k = vivos.clave(yo, a.name)
    vivos.empezar(yo, a.name)
    // El foco, al despliegue que se acaba de lanzar. El botón que abrió la hoja
    // se ha cambiado por «Desplegando…», y sin esto acababa en el título del
    // servidor, a veinticinco tabuladores del progreso.
    if (sigue(yo, a.name)) {
      setTimeout(() => {
        if (sigue(yo, a.name) && pestana === 'despliegue') {
          document.querySelector<HTMLElement>('[data-foco-despliegue]')?.focus()
        }
      }, 10)
    }
    try {
      const r = await desplegar(yo, a.name, { entorno, forzar })
      vivos.terminar(k, r)
      // Si se está mirando su pestaña, ya lo anuncia ella con el detalle.
      if (!mirandoDespliegue(yo, a.name)) anunciar(`Despliegue de ${a.name} en ${yo}: ${r.ok === true ? 'terminado' : 'ha fallado'}.`)
      // Un despliegue cambia el estado de la app: la portada deja de ser de
      // fiar y se vuelve a pedir, en vez de parchear la fila a mano. Sin
      // soltar la app elegida: la primera versión recargaba con `cargar`, que
      // la deseleccionaba, y el resultado del despliegue desaparecía de la
      // pantalla justo cuando llegaba.
      if (sigue(yo)) void refrescar()
    } catch (e) {
      const err = e as ErrorDelPuente
      // NO se llama fallo. Si se perdió el contacto, el despliegue sigue en el
      // servidor y el cliente ya no sabe qué pasó.
      vivos.perder(k, err.mensaje)
      if (!mirandoDespliegue(yo, a.name)) anunciar(`Despliegue de ${a.name} en ${yo}: estado desconocido.`)
    } finally {
      // Los valores no se quedan en ningún sitio: se olvidan aquí también.
      entorno.length = 0
    }
  }

  function mirandoDespliegue(servidor: string, app: string): boolean {
    // Con una hoja o la paleta abiertas, la pestaña está detrás de `inert` y su
    // región viva no la oye nadie: entonces avisa la de la aplicación.
    if (hoja || pendiente || enPaleta) return false
    return sigue(servidor, app) && vista === 'apps' && pestana === 'despliegue'
  }

  /** Vuelve a pedir la portada **sin soltar la app elegida**, y su detalle. */
  async function refrescar() {
    const yo = alias
    const t = ++turno
    try {
      const r = await portada(yo)
      if (t !== turno || !sigue(yo)) return
      apps = r.apps
      host = r.host ?? null
      servicios = r.services ?? null
      // Esta respuesta gana el turno, así que es la que manda: quita el fallo
      // de antes —uno solo dejaba la pantalla de fallo puesta hasta cambiar
      // de servidor— y el esqueleto de una carga a la que le ha quitado el
      // turno, que si no se quedaba para siempre.
      error = null
      cargando = false
      if (elegida) {
        const misma = r.apps.find((x) => x.name === elegida!.name) ?? null
        elegida = misma
        // Sin cambiar de pestaña: si se está mirando el resultado de un
        // despliegue, ahí se queda. Y el detalle viejo se queda hasta que llega
        // el nuevo: ponerlo a `null` desmontaba «retirar» y «revertir», con la
        // confirmación a medio escribir, cada vez que terminaba un despliegue
        // de OTRA app.
        if (misma) void cargarDetalle(misma, true)
        else {
          detalleApp = null
          tareasApp = null
        }
      }
    } catch (e) {
      if (t === turno && sigue(yo)) {
        error = e as ErrorDelPuente
        cargando = false
      }
    }
  }

  /** El detalle de la app: `info` y sus tareas, a la vez y bajo demanda. */
  async function verDetalle(a: App) {
    pestana = 'detalle'
    await cargarDetalle(a)
  }

  async function cargarDetalle(a: App, forzar = false) {
    const yo = alias
    if (!forzar && detalleApp?.name === a.name && (tareasApp?.app === a.name || !sabe(versionActual, 'tareas'))) return
    const [d, t] = await Promise.all([
      pedirDetalle(yo, a.name).catch(() => null),
      sabe(versionActual, 'tareas') ? pedirTareas(yo, a.name).catch(() => null) : Promise.resolve(null),
    ])
    if (!sigue(yo, a.name)) return
    // Que `info` falle no rompe el detalle: se sigue con lo que trajo la lista,
    // que es lo que había antes de que esto existiera. Al refrescar, uno que
    // falla no borra el que había.
    if (d || !forzar) detalleApp = d
    if (t || !forzar) tareasApp = t
  }

  // La hoja de una orden corta sobre la app —congelar, descongelar, la CI—,
  // con la orden literal a la vista antes de lanzarla.
  let pendiente = $state<{
    titulo: string
    orden: string
    consecuencia: string
    verbo: string
    /** El servidor en el que corre, fijado al pedirla, por lo mismo que `hojaEn`. */
    servidor: string
    correr: () => Promise<{ salida: string }>
  } | null>(null)

  async function ejecutarPendiente() {
    const p = pendiente
    pendiente = null
    if (!p) return
    const yo = p.servidor
    try {
      const r = await p.correr()
      if (!sigue(yo)) return
      avisoDetalle = { ok: true, texto: r.salida.trim() || 'Hecho.' }
    } catch (e) {
      // La frase de Orbit tal cual: «'viejo.com' es una redirección…», no
      // «ha salido con 1».
      if (sigue(yo)) avisoDetalle = { ok: false, texto: (e as ErrorDelPuente).mensaje }
    }
    if (sigue(yo)) await refrescar()
  }

  /** Una app que se llama como una palabra de Orbit —`no`, `help`…— haría que
   *  el servidor entendiera otra orden, o la aplicara a la primera app por
   *  orden alfabético. No se abre la hoja: se dice por qué. */
  function rehusa(a: App, orden: Parameters<typeof chocaConPalabraDeOrbit>[1]): boolean {
    const motivo = chocaConPalabraDeOrbit(a.name, orden)
    if (motivo === null) return false
    avisoDetalle = { ok: false, texto: motivo }
    return true
  }

  function pedirCongelar(a: App, motivo: string | null) {
    if (rehusa(a, 'congelar')) return
    const yo = alias
    pendiente = {
      titulo: `Congelar los despliegues de ${a.name}`,
      orden: ordenLegible(argvDeCongelar(a.name, motivo)),
      consecuencia:
        'El autodespliegue y «desplegar todo» la saltarán, y desplegarla a mano pedirá confirmarlo aparte. Lo que se sirve no cambia, y revertir sigue permitido.',
      verbo: 'Congelar',
      servidor: yo,
      correr: () => congelar(yo, a.name, motivo),
    }
  }

  function pedirDescongelar(a: App) {
    if (rehusa(a, 'descongelar')) return
    const yo = alias
    pendiente = {
      titulo: `Descongelar ${a.name}`,
      orden: ordenLegible(argvDeDescongelar(a.name)),
      consecuencia: a.state.autodeploy
        ? 'Vuelve a desplegarse con normalidad. Tiene el autodespliegue puesto: si su rama ha avanzado, se desplegará sola en la próxima pasada.'
        : 'Vuelve a desplegarse con normalidad.',
      verbo: 'Descongelar',
      servidor: yo,
      correr: () => descongelar(yo, a.name),
    }
  }

  function pedirEsperarCi(a: App, esperar: boolean) {
    if (rehusa(a, 'esperarCi')) return
    const yo = alias
    pendiente = {
      titulo: esperar ? `Esperar a la CI en ${a.name}` : `Dejar de esperar a la CI en ${a.name}`,
      orden: ordenLegible(argvDeEsperarCi(a.name, esperar)),
      consecuencia: esperar
        ? 'El autodespliegue y «desplegar todo» sólo desplegarán un commit cuya CI de GitHub haya terminado bien; si no pueden preguntar, no despliegan. Si el servidor no tiene sesión de gh, avisará.'
        : 'El autodespliegue volverá a desplegar sin mirar la CI.',
      verbo: esperar ? 'Esperar a la CI' : 'Dejar de esperar',
      servidor: yo,
      correr: () => esperarCi(yo, a.name, esperar),
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
    host = null
    servicios = null
    error = null
    doctor = null
    log = null
    entorno = null
    monitor = null
    trafico = null
    metricas = null
    detalleApp = null
    tareasApp = null
    avisoDetalle = null
    errorVista = null
    errorPestana = null
    vista = 'apps'
    // Lo que se iba a confirmar era de otro servidor.
    cerrarHoja()
    pendiente = null
    // Y el ritmo del monitor es de otro servidor: un minuto de espera heredado
    // de uno que no contestaba no es el ritmo de éste.
    periodo = 3
    fallosMonitor = 0
    cargando = true
    const t = ++turno
    // La versión, una vez por servidor y en paralelo: no retrasa la portada.
    if (!versiones[a]) {
      void pedirVersion(a)
        .then((v) => { versiones = { ...versiones, [a]: v.version } })
        .catch(() => { versiones = { ...versiones, [a]: null } })
    }
    try {
      const r = await portada(a)
      if (t !== turno) return
      apps = r.apps
      host = r.host ?? null
      servicios = r.services ?? null
    } catch (e) {
      if (t !== turno) return
      // Un fallo se pinta como un fallo. Lo que NO se hace es dejar los datos
      // viejos en pantalla sin decir que son viejos: un panel que dice «todo
      // verde» cuando no ha podido preguntar oculta un incidente en curso.
      error = e as ErrorDelPuente
    } finally {
      if (t === turno) cargando = false
    }
  }

  async function verDiagnostico(forzar = false) {
    vista = 'diagnostico'
    if (doctor && !forzar) return
    const yo = alias
    errorVista = null
    diagnosticando = true
    try {
      const d = await diagnostico(yo)
      if (sigue(yo)) doctor = d
    } catch (e) {
      if (sigue(yo)) errorVista = e as ErrorDelPuente
    } finally {
      diagnosticando = false
    }
  }

  let diagnosticando = $state(false)

  async function aplicarArreglos() {
    arreglando = true
    const yo = alias
    try {
      // Se vuelve a diagnosticar con lo que devuelve el propio comando: lo que
      // cuenta es el estado del servidor después, no lo que dijeron los
      // arreglos. Es la misma regla que sigue `orbit doctor --fix`.
      const d = await arreglar(yo)
      if (!sigue(yo)) return
      doctor = d
      // Y las apps pueden haber cambiado —un vhost regenerado cambia
      // `served`— así que la portada deja de ser de fiar. Se vuelve a pedir:
      // la primera versión la ponía a `null` y no la pedía nunca, y al volver
      // a «apps» la lista estaba en blanco.
      void refrescar()
    } catch (e) {
      if (sigue(yo)) errorVista = e as ErrorDelPuente
    } finally {
      arreglando = false
    }
  }

  // Una sola petición del monitor en vuelo, nunca dos. Con `setInterval`, un
  // `top` que tardara más que el periodo —2,1 s con 40 apps, más con la red
  // lenta— dejaba otro esperando detrás, y otro, y otro: una cola de sesiones
  // SSH que nadie iba a mirar.
  // Por servidor: con un booleano, un `top` lento de alfa hacía que entrar en
  // el monitor de bravo no preguntara nada y enseñara la lista de apps.
  let monitorEnVuelo: string | null = null

  // El fallo de una vista es de esa vista: al cambiar de una a otra se olvida,
  // o el del monitor acababa pintado encima del diagnóstico.
  $effect(() => {
    void vista
    untrack(() => { errorVista = null })
  })
  let fallosMonitor = $state(0)
  let monitorPausado = $state(false)

  async function verMonitor() {
    vista = 'monitor'
    const yo = alias
    if (monitorEnVuelo === yo) return
    monitorEnVuelo = yo
    try {
      // El periodo sale de lo que TARDÓ, no de un número elegido: `top --json`
      // cuesta ~2,1 s con 40 apps porque la CPU es la diferencia entre dos
      // lecturas, y encadenar peticiones más rápido de lo que contestan no da
      // más frescura, da una cola.
      const t0 = performance.now()
      const m = await pedirMonitor(yo)
      if (!sigue(yo)) return
      monitor = m
      errorVista = null
      fallosMonitor = 0
      periodo = periodoDelMonitor(performance.now() - t0)
      historial.anotar(yo, m)
    } catch (e) {
      if (!sigue(yo)) return
      errorVista = e as ErrorDelPuente
      // Espera creciente: un servidor que no contesta no se arregla
      // preguntándole más a menudo. Hasta un minuto, y vuelve al ritmo medido
      // en cuanto conteste.
      fallosMonitor += 1
      periodo = Math.min(60, periodo * 2)
    } finally {
      if (monitorEnVuelo === yo) monitorEnVuelo = null
    }
  }

  // Sólo mientras se está mirando, y sólo con la ventana enfocada. Sondear una
  // pantalla que nadie ve es gastar una sesión SSH del usuario en nada.
  //
  // Encadenado y no a intervalo fijo: la siguiente vuelta se programa cuando la
  // anterior TERMINA, con el periodo que ella misma midió. Así no puede haber
  // dos a la vez, y un servidor lento se pregunta menos, no más.
  //
  // Con la ventana oculta no se programa nada, y al volver se pregunta en el
  // acto: la primera versión seguía despertándose y, al volver, hacía esperar
  // un periodo entero con datos viejos en pantalla.
  $effect(() => {
    if (vista !== 'monitor' || monitorPausado) return
    let vivo = true
    let t: ReturnType<typeof setTimeout> | undefined
    const vuelta = async () => {
      t = undefined
      if (document.hidden) return
      await verMonitor()
      if (vivo && !document.hidden) t = setTimeout(vuelta, periodo * 1000)
    }
    const alVolver = () => {
      if (!document.hidden && vivo && t === undefined && monitorEnVuelo !== alias) void vuelta()
    }
    document.addEventListener('visibilitychange', alVolver)
    t = setTimeout(vuelta, untrack(() => periodo) * 1000)
    return () => {
      vivo = false
      clearTimeout(t)
      document.removeEventListener('visibilitychange', alVolver)
    }
  })

  async function verEntorno(a: App, forzar = false) {
    pestana = 'entorno'
    errorPestana = null
    if (entorno && entorno.app === a.name && !forzar) return
    const yo = alias
    try {
      const e = await pedirEntorno(yo, a.name)
      if (sigue(yo, a.name)) entorno = e
    } catch (e) {
      if (sigue(yo, a.name)) errorPestana = e as ErrorDelPuente
    }
  }

  // El detalle se pide EN EL MOMENTO en que hace falta: es de donde sale el
  // inventario de lo que se pierde al borrar, y decirle a alguien que va a
  // perder 3 releases cuando tiene 12 es peor que no decírselo.
  async function verAdmin(a: App, cual: 'retirar' | 'revertir') {
    pestana = cual
    errorPestana = null
    const yo = alias
    try {
      const [d, e] = await Promise.all([
        pedirDetalle(yo, a.name),
        cual === 'retirar' ? pedirEntorno(yo, a.name) : Promise.resolve(null),
      ])
      if (!sigue(yo, a.name)) return
      detalleApp = d
      if (e) entorno = e
    } catch (err) {
      if (sigue(yo, a.name)) errorPestana = err as ErrorDelPuente
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
    errorPestana = null
    const yo = alias
    try {
      const [d, e] = await Promise.all([
        detalleApp?.name === a.name ? Promise.resolve(detalleApp) : pedirDetalle(yo, a.name),
        entorno?.app === a.name ? Promise.resolve(entorno) : pedirEntorno(yo, a.name).catch(() => null),
      ])
      if (!sigue(yo, a.name)) return
      detalleApp = d
      if (e) entorno = e
    } catch (err) {
      if (sigue(yo, a.name)) errorPestana = err as ErrorDelPuente
    }
  }

  async function verTrafico(a: App, forzar = false) {
    pestana = 'trafico'
    errorPestana = null
    if (trafico && trafico.app === a.name && !forzar) return
    const yo = alias
    // Las dos a la vez: se miran juntas y son dos comandos distintos, así que
    // pedirlas en serie doblaría la espera de una pantalla sin ganar nada.
    // Y cada una por su lado: con `Promise.all`, una app sin despliegues
    // medidos —que es un estado, no un error— se llevaba por delante el
    // tráfico, que sí había llegado.
    const [t, m] = await Promise.allSettled([
      pedirTrafico(yo, a.name),
      pedirMetricas(yo, a.name),
    ])
    if (!sigue(yo, a.name)) return
    // Sin una sola visita en la ventana, el puente dice `sin-datos`. No se
    // fabrica un objeto con ceros: se pinta como lo que es, un estado vacío.
    if (t.status === 'fulfilled') trafico = t.value
    else errorPestana = t.reason as ErrorDelPuente
    metricas = m.status === 'fulfilled' ? m.value : null
  }

  async function verLog(a: App, forzar = false) {
    pestana = 'log'
    errorPestana = null
    if (log && !forzar) return
    // Un servidor anterior a la 1.3.7 no tiene `logs --json`: la pestaña lo
    // dice ella, sin gastar una llamada que va a fallar.
    if (seQuedaCorto(versionActual, 'contratoCompleto')) return
    const yo = alias
    try {
      const l = await pedirLog(yo, a.name)
      if (sigue(yo, a.name)) log = l
    } catch (e) {
      if (sigue(yo, a.name)) errorPestana = e as ErrorDelPuente
    }
  }

  /** Vuelve a pedir lo de la pestaña que se está mirando. Antes no había forma:
   *  el log, el entorno y el tráfico se pedían una vez y se quedaban así. */
  let recargando = $state(false)
  async function recargarPestana() {
    const a = elegida
    if (!a || recargando) return
    recargando = true
    try {
      if (pestana === 'log') await verLog(a, true)
      else if (pestana === 'entorno') await verEntorno(a, true)
      else if (pestana === 'trafico') await verTrafico(a, true)
      else if (pestana === 'detalle') {
        detalleApp = null
        tareasApp = null
        await refrescar()
      }
    } finally {
      recargando = false
    }
  }

  // Mantener pulsada j/k elegía una app por pulsación y pedía `info` + `cron`
  // de cada una: con 200 apps, cientos de sesiones SSH a la vez. Un clic suelto
  // pide en el acto; una ráfaga, sólo cuando se para.
  let ultimaEleccion = 0
  let detalleDiferido: ReturnType<typeof setTimeout> | undefined
  function elegirApp(a: App) {
    if (elegida?.name === a.name) return
    elegida = a
    // El log es de una app concreta: al cambiar de app, el que había deja de
    // valer. Conservarlo enseñaría el log de una bajo el nombre de otra.
    log = null
    // El entorno es de una app concreta: al cambiar de app, el que había deja
    // de valer, y con él cualquier valor revelado.
    entorno = null
    trafico = null
    metricas = null
    detalleApp = null
    tareasApp = null
    avisoDetalle = null
    errorPestana = null
    pestana = 'detalle'
    // El detalle se pide al elegir: trae lo que la lista no calcula —los días
    // del certificado, el motivo de una congelación, las tareas— y es una sola
    // app.
    const ahora = performance.now()
    const enRafaga = ahora - ultimaEleccion < 300
    ultimaEleccion = ahora
    clearTimeout(detalleDiferido)
    if (!enRafaga) {
      void verDetalle(a)
      return
    }
    const yo = alias
    // `cargarDetalle` y no `verDetalle`: si mientras tanto se ha elegido otra
    // pestaña, ahí se queda.
    detalleDiferido = setTimeout(() => {
      if (sigue(yo, a.name)) void cargarDetalle(a)
    }, 250)
  }

  // ── la paleta, los atajos y el tema ─────────────────────────────────────
  //
  // Todo lo que la paleta hace ya existe en un botón: no hay ninguna acción que
  // sólo se pueda hacer desde aquí, y las que cambian algo abren su hoja de
  // confirmación igual que desde el botón. Es una forma de llegar, no un
  // atajo para saltarse la orden literal.
  let enPaleta = $state(false)

  // Una región viva que existe SIEMPRE, para lo que termina mientras se mira
  // otra cosa: un despliegue, una pasada, una web nueva. Una región que se
  // monta con su texto dentro no la anuncian los lectores de pantalla.
  let anuncio = $state('')
  let porAnunciar: string[] = []
  let tAnuncio: ReturnType<typeof setTimeout> | undefined
  function anunciar(t: string) {
    // Vaciar y volver a poner: dos anuncios iguales seguidos se leen dos veces.
    // Y los que llegan juntos se dicen juntos: con un temporizador por anuncio,
    // el segundo pisaba al primero antes de que nadie lo leyera.
    porAnunciar.push(t)
    anuncio = ''
    clearTimeout(tAnuncio)
    tAnuncio = setTimeout(() => {
      anuncio = porAnunciar.join(' ')
      porAnunciar = []
    }, 50)
  }

  // Qué hay en el panel, para nombrarlo si falla al pintarse, y la clave que
  // monta una frontera nueva al cambiar de pantalla: un fallo en el monitor no
  // se queda puesto al volver a la lista.
  const quePanel = $derived(
    enComparar ? 'la comparación'
      : enPasada ? '«desplegar todo»'
        : enNueva ? 'el asistente de web nueva'
          : enAlta ? 'la lista de servidores'
            : vista === 'monitor' ? 'el monitor'
              : vista === 'diagnostico' ? 'el diagnóstico'
                : vista === 'servidor' ? 'la vista del servidor'
                  : 'la lista de apps',
  )
  const clavePanel = $derived(`${alias}|${quePanel}`)
  const QUE_PESTANA: Record<string, string> = {
    detalle: 'el detalle', log: 'el log', entorno: 'el entorno', trafico: 'el tráfico',
    exec: 'exec', retirar: 'retirar', revertir: 'revertir', despliegue: 'el despliegue',
    descriptor: 'el orbit.json',
  }

  /** Hay algo en marcha que no se puede dejar a medias navegando. */
  const ocupado = $derived(creando || pasando || instalando)

  /** Salir de los modos que no son «mirar el servidor», si no hay nada en
   *  marcha dentro: una web a medio crear o una pasada no se cierran por
   *  navegar. */
  function aLoNormal(): boolean {
    if (creando || pasando || instalando) return false
    if (enNueva) cerrarNueva()
    if (enPasada) cerrarPasada()
    if (enComparar) cerrarComparar()
    if (instalandoEn) cerrarInstalacion()
    enAlta = false
    anadiendo = false
    return true
  }

  function verVista(v: typeof vista) {
    if (!aLoNormal()) return
    if (v === 'diagnostico') void verDiagnostico()
    else if (v === 'monitor') void verMonitor()
    else vista = v
  }

  const acciones = $derived.by<Accion[]>(() => {
    const r: Accion[] = []
    for (const s of enElRail) {
      r.push({
        id: `servidor:${s.alias}`, grupo: 'servidor', texto: s.alias,
        pista: s.hostname ?? undefined,
        hacer: () => { if (aLoNormal()) void cargar(s.alias) },
      })
    }
    if (alias) {
      for (const [v, t] of [['apps', 'Apps'], ['diagnostico', 'Diagnóstico'], ['monitor', 'Monitor'], ['servidor', 'Servidor: host, copias, bases de datos…']] as const) {
        r.push({ id: `ver:${v}`, grupo: 'ver', texto: t, pista: alias, hacer: () => verVista(v) })
      }
    }
    if (elegida && vista === 'apps') {
      const a = elegida
      r.push({ id: 'accion:desplegar', grupo: 'hacer', texto: `Desplegar ${a.name}`, pista: alias, claves: 'deploy', hacer: () => prepararDespliegue(a) })
      r.push({ id: 'accion:log', grupo: 'ver', texto: `Log de ${a.name}`, pista: alias, claves: 'logs', hacer: () => verLog(a) })
      r.push({ id: 'accion:entorno', grupo: 'ver', texto: `Entorno de ${a.name}`, pista: alias, claves: 'env variables', hacer: () => verEntorno(a) })
      r.push({ id: 'accion:trafico', grupo: 'ver', texto: `Tráfico de ${a.name}`, pista: alias, claves: 'traffic visitas', hacer: () => verTrafico(a) })
      r.push({ id: 'accion:orbitjson', grupo: 'ver', texto: `orbit.json de ${a.name}`, pista: alias, claves: 'descriptor csp', hacer: () => verDescriptor(a) })
      r.push({ id: 'accion:revertir', grupo: 'hacer', texto: `Revertir ${a.name}…`, pista: alias, claves: 'rollback', hacer: () => verAdmin(a, 'revertir') })
    }
    for (const a of apps ?? []) {
      if (!nombreOperable(a.name)) continue
      r.push({
        id: `app:${a.name}`, grupo: 'app', texto: a.name, pista: a.domain, claves: a.type,
        hacer: () => { if (aLoNormal()) { vista = 'apps'; elegirApp(a) } },
      })
    }
    if (alias) {
      r.push({ id: 'accion:nueva', grupo: 'hacer', texto: 'Nueva web', pista: alias, claves: 'new crear', hacer: () => { if (aLoNormal()) { enNueva = true; dns = null } } })
      r.push({ id: 'accion:pasada', grupo: 'hacer', texto: 'Desplegar todo…', pista: alias, claves: 'deploy all pasada', hacer: () => { if (aLoNormal()) enPasada = true } })
      if (enElRail.length > 1) r.push({ id: 'accion:comparar', grupo: 'hacer', texto: 'Comparar con otro servidor', pista: alias, hacer: () => { if (aLoNormal()) enComparar = true } })
    }
    r.push({ id: 'accion:alta', grupo: 'hacer', texto: 'Servidores: añadir o comprobar', claves: 'ssh alta', hacer: () => { if (instalando || aLoNormal()) enAlta = true } })
    r.push({ id: 'accion:tema', grupo: 'hacer', texto: `Tema: ${TEMAS[(TEMAS.indexOf(tema) + 1) % TEMAS.length]}`, claves: 'oscuro claro theme', hacer: cambiarTema })
    // Y sin repetidos, por si dos apps se llaman igual en la misma portada.
    const vistos = new Set<string>()
    return r.filter((x) => !vistos.has(x.id) && !!vistos.add(x.id))
  })

  // El tema. Por defecto, el del sistema; elegirlo dura la sesión y no se
  // guarda en ningún sitio: esta interfaz no escribe nada en el navegador (T-12).
  const TEMAS = ['sistema', 'claro', 'oscuro'] as const
  let tema = $state<(typeof TEMAS)[number]>('sistema')
  function cambiarTema() {
    tema = TEMAS[(TEMAS.indexOf(tema) + 1) % TEMAS.length]!
  }
  $effect(() => {
    const r = document.documentElement
    if (tema === 'claro') r.dataset.theme = 'light'
    else if (tema === 'oscuro') r.dataset.theme = 'dark'
    else delete r.dataset.theme
  })

  function atajos(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey
    // Con una hoja de confirmación abierta no se abre la paleta ni se recarga:
    // la paleta podía cambiar de servidor por debajo de la hoja, que seguía
    // diciendo un servidor mientras la orden corría en otro.
    if (mod && (hoja || pendiente) && ['k', 'K', 'r', 'R'].includes(e.key)) {
      e.preventDefault()
      return
    }
    if (mod && (e.key === 'k' || e.key === 'K')) {
      e.preventDefault()
      enPaleta = !enPaleta
      return
    }
    // Ctrl+R vuelve a pedir lo que se mira, en vez de recargar la ventana
    // entera —que perdería los despliegues en curso de vista—.
    if (mod && (e.key === 'r' || e.key === 'R') && !e.shiftKey) {
      e.preventDefault()
      if (elegida && vista === 'apps') void recargarPestana()
      else if (vista === 'diagnostico') void verDiagnostico(true)
      else if (alias && vista === 'apps') void refrescar()
    }
  }

  $effect(() => {
    // Las DOS fuentes. Si no hay ninguna en ninguna de las dos, se abre
    // directamente la pantalla de servidores: una portada vacía sin explicación
    // es lo que hacía que esta aplicación pareciera rota al abrirla por primera
    // vez, y es literalmente lo que pasaba en un Windows recién instalado.
    // Cada fuente por su lado: si leer el ~/.ssh/config falla, la ventana se
    // quedaba en blanco en vez de abrir el alta.
    void Promise.allSettled([servidoresDelConfig(), servidoresGuardados()]).then(([c, m]) => {
      const config = c.status === 'fulfilled' ? c.value : []
      const mios = m.status === 'fulfilled' ? m.value : []
      servidores = config
      propios = mios
      const primero = mios[0]?.alias ?? config[0]?.alias
      if (primero && !alias) cargar(primero)
      else if (!primero) enAlta = true
    })
  })
</script>

<svelte:window onkeydown={atajos} />

{#if enPaleta}
  <Paleta {acciones} alCerrar={() => (enPaleta = false)} />
{/if}

{#if hoja}
  <HojaDeDespliegue
    app={hoja}
    servidor={hojaEn}
    {comprobacion}
    {estadoComprobacion}
    {faltanDeAntes}
    porEntrada={sabe(versionActual, 'entornoPorEntrada')}
    alConfirmar={(entorno, forzar) => lanzar(hoja!, entorno, forzar, hojaEn)}
    alCancelar={cerrarHoja}
  />
{/if}

{#if pendiente}
  <HojaDeComando
    titulo={pendiente.titulo}
    servidor={pendiente.servidor}
    orden={pendiente.orden}
    consecuencia={pendiente.consecuencia}
    verbo={pendiente.verbo}
    alConfirmar={ejecutarPendiente}
    alCancelar={() => (pendiente = null)}
  />
{/if}

<!-- Con una hoja o la paleta abiertas, lo de detrás no recibe ni foco ni
     clics: `inert` es lo que hace que un diálogo sea modal de verdad, y no
     sólo de aspecto. -->
<p class="sr" role="status" aria-live="polite">{anuncio}</p>

<div class="marco" inert={!!(hoja || pendiente || enPaleta)}>
  <!-- El servidor activo está SIEMPRE visible, no escondido en un desplegable
       que se lee al entrar. El accidente más caro de un cliente multiservidor
       no es un ataque: es ejecutar lo correcto contra el servidor equivocado.
       Hay precedente — la suite de pruebas de Orbit borró el vhost de una app
       de producción llamada `tienda` por eso mismo. -->
  <nav class="rail" aria-label="Servidores">
    <p class="marca">Orbit</p>
    <ul>
      {#each enElRail as s (s.alias)}
        {@const enMarcha = vivos.enCurso(s.alias).length}
        <li>
          <button
            type="button"
            class="servidor"
            class:servidor--activo={alias === s.alias}
            aria-current={alias === s.alias ? 'true' : undefined}
            aria-disabled={ocupado && alias !== s.alias ? 'true' : undefined}
            title={ocupado && alias !== s.alias
              ? 'Hay una operación en marcha: espera a que termine'
              : s.hostname ? `${s.usuario ?? ''}@${s.hostname}:${s.puerto ?? 22}` : undefined}
            onclick={() => { if (alias !== s.alias && aLoNormal()) void cargar(s.alias) }}
          >
            {s.alias}
            {#if enMarcha > 0}
              <!-- Volver a un despliegue tiene que ser un clic: mientras corre,
                   el servidor lleva su contador. -->
              <span
                class="corriendo"
                title={enMarcha === 1 ? 'un despliegue en curso' : `${enMarcha} despliegues en curso`}
              ><span aria-hidden="true">◐ {enMarcha}</span><span class="sr">, {enMarcha === 1 ? 'un despliegue en curso' : `${enMarcha} despliegues en curso`}</span></span>
            {/if}
            {#if s.salto}
              <!-- Un salto se anuncia porque cambia lo que se puede prometer
                   sobre la latencia: el saludo se paga dos veces. -->
              <span class="salto" title="Por {s.salto}"><span aria-hidden="true">↪</span><span class="sr">, por {s.salto}</span></span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    <button
      type="button"
      class="alta"
      aria-disabled={enAlta && instalando ? 'true' : undefined}
      title={enAlta && instalando ? 'Se está instalando Orbit: espera a que termine' : undefined}
      onclick={() => {
        // Con una instalación en marcha no se sale de aquí: después no había
        // forma de volver a su progreso ni a su botón de parar.
        if (enAlta) { if (!instalando) enAlta = false }
        else if (instalando) enAlta = true
        else if (aLoNormal()) enAlta = true
      }}
    >
      {enAlta ? '← volver' : '+ servidores'}
    </button>

    <div class="pie-rail">
      <button type="button" class="atajo" onclick={() => (enPaleta = true)} title="Paleta de órdenes">
        <span>buscar</span> <kbd>Ctrl K</kbd>
      </button>
      <button type="button" class="atajo" onclick={cambiarTema} aria-label="Tema: {tema}. Cambiar" title="Cambiar el tema">
        <span>{tema === 'oscuro' ? '☾' : tema === 'claro' ? '☀' : '◐'} {tema}</span>
      </button>
    </div>

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
      <h1 tabindex="-1">
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
        <button type="button" class:activo={vista === 'apps'} aria-current={vista === 'apps' ? 'true' : undefined} onclick={() => (vista = 'apps')}>
          apps
        </button>
        <button type="button" class:activo={vista === 'diagnostico'} aria-current={vista === 'diagnostico' ? 'true' : undefined} onclick={() => verDiagnostico()}>
          diagnóstico
        </button>
        <button type="button" class:activo={vista === 'monitor'} aria-current={vista === 'monitor' ? 'true' : undefined} onclick={verMonitor}>
          monitor
        </button>
        <button type="button" class:activo={vista === 'servidor'} aria-current={vista === 'servidor' ? 'true' : undefined} onclick={() => (vista = 'servidor')}>
          servidor
        </button>
        {#if alias}
          <button type="button" class="nueva" onclick={() => { enNueva = true; dns = null }}>
            nueva web
          </button>
          <button type="button" onclick={() => (enPasada = true)}>desplegar todo</button>
          {#if enElRail.length > 1}
            <button type="button" onclick={() => (enComparar = true)}>comparar</button>
          {/if}
        {/if}
      </nav>
      {/if}
    </header>

    <section class="panel">
      {#key clavePanel}
      <Frontera que={quePanel} alReintentar={() => { if (alias && quePanel === 'la lista de apps') void refrescar() }}>
      {#if (enComparar || enPasada) && apps === null}
        <!-- Sin la portada de este servidor no hay con qué comparar ni qué
             pasar: con una lista vacía, cada app del otro salía como «sólo
             en el otro» y la pasada decía que iba a tocar cero apps. -->
        {#if error}<Fallo {error} {alias} />{:else}<Esqueleto />{/if}
      {:else if enComparar}
        <Comparar
          a={alias}
          b={otro}
          appsA={apps!}
          appsB={appsOtro}
          fallo={falloOtro}
          cargando={comparando}
          candidatos={enElRail.map((x) => x.alias).filter((x) => x !== alias)}
          alElegir={compararCon}
          alCerrar={cerrarComparar}
        />
      {:else if enPasada}
        {#if errorFlujo}<Fallo error={errorFlujo} {alias} />{/if}
        <Pasada
          servidor={alias}
          apps={apps!}
          pasada={pasadaViva}
          resultado={lote}
          corriendo={pasando}
          modo={modoPasada}
          alLanzar={lanzarPasada}
          alCancelar={pararPasada}
          alCerrar={cerrarPasada}
        />
      {:else if enNueva}
        {#if errorFlujo}<Fallo error={errorFlujo} {alias} />{/if}
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
            {#if salidaNueva.total > 0}
              <details>
                <summary>Lo que dijo el servidor</summary>
                <Consola salida={salidaNueva} vivo={false} />
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
            <Consola salida={salidaNueva} vivo={true} />
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
        {#if errorFlujo}<Fallo error={errorFlujo} alias={instalandoEn ?? alias} />{/if}
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
            alCerrar={() => { anadiendo = false; falloAlGuardar = null; devolverFoco(null) }}
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
      {:else if vista === 'servidor' && !cargando && !error}
        <ServidorVista servidor={alias} {host} {servicios} version={versionActual} />
      {:else if vista === 'monitor' && monitor}
        <MonitorVista
          {monitor}
          portada={apps}
          servidor={alias}
          {periodo}
          pausado={monitorPausado}
          alPausar={hayPuente() ? (p) => (monitorPausado = p) : null}
          falloUltimo={errorVista?.mensaje ?? null}
        />
      {:else if vista === 'diagnostico' && doctor}
        {#if errorVista}<Fallo error={errorVista} {alias} />{/if}
        <Diagnostico
          {doctor}
          servidor={alias}
          {arreglando}
          alArreglar={hayPuente() && !seQuedaCorto(versionActual, 'contratoCompleto') ? aplicarArreglos : null}
        />
        <button type="button" class="otra-vez" disabled={diagnosticando} onclick={() => verDiagnostico(true)}>
          {diagnosticando ? 'diagnosticando…' : '↻ volver a diagnosticar'}
        </button>
      {:else if vista !== 'apps' && errorVista}
        <Fallo error={errorVista} {alias} />
      {:else if vista === 'monitor' || vista === 'diagnostico'}
        <!-- Su propia espera. Compartían el `cargando` de la portada, y la
             lista llegaba antes y se pintaba en la pestaña del monitor. -->
        <Esqueleto />
      {:else if cargando}
        <Esqueleto />
      {:else if error}
        <Fallo {error} {alias} />
      {:else if apps}
        <ListaApps
          {apps}
          servidor={alias}
          seleccionada={elegida?.name ?? null}
          alElegir={elegirApp}
        />
      {/if}
      </Frontera>
      {/key}
    </section>

    {#if elegida && vista === 'apps' && !enNueva && !enPasada && !enComparar}
      <section class="panel panel--detalle">
        <nav class="pestanas" aria-label="Qué mirar de esta app">
          <button type="button" class:activo={pestana === 'detalle'} aria-current={pestana === 'detalle' ? 'true' : undefined} onclick={() => verDetalle(elegida!)}>
            detalle
          </button>
          <button type="button" class:activo={pestana === 'log'} aria-current={pestana === 'log' ? 'true' : undefined} onclick={() => verLog(elegida!)}>
            log
          </button>
          <button type="button" class:activo={pestana === 'entorno'} aria-current={pestana === 'entorno' ? 'true' : undefined} onclick={() => verEntorno(elegida!)}>
            entorno
          </button>
          <button type="button" class:activo={pestana === 'trafico'} aria-current={pestana === 'trafico' ? 'true' : undefined} onclick={() => verTrafico(elegida!)}>
            tráfico
          </button>
          <button type="button" class:activo={pestana === 'descriptor'} aria-current={pestana === 'descriptor' ? 'true' : undefined} onclick={() => verDescriptor(elegida!)}>
            orbit.json
          </button>
          <button type="button" class:activo={pestana === 'exec'} aria-current={pestana === 'exec' ? 'true' : undefined} onclick={() => (pestana = 'exec')}>
            exec
          </button>
          <button type="button" class:activo={pestana === 'revertir'} aria-current={pestana === 'revertir' ? 'true' : undefined} onclick={() => verAdmin(elegida!, 'revertir')}>
            revertir
          </button>
          <button type="button" class:activo={pestana === 'retirar'} aria-current={pestana === 'retirar' ? 'true' : undefined} onclick={() => verAdmin(elegida!, 'retirar')}>
            retirar
          </button>
          {#if vivos.ver(alias, elegida.name)}
            <button type="button" class:activo={pestana === 'despliegue'} aria-current={pestana === 'despliegue' ? 'true' : undefined} onclick={() => (pestana = 'despliegue')}>
              despliegue
            </button>
          {/if}
          {#if pestana === 'log' || pestana === 'entorno' || pestana === 'trafico' || pestana === 'detalle'}
            <button
              type="button"
              class="recargar"
              disabled={recargando}
              onclick={recargarPestana}
              aria-label="Volver a pedir {pestana} de {elegida.name}"
              title="Volver a pedir"
            >↻</button>
          {/if}
          {#if vivos.corriendo(alias, elegida.name)}
            <button type="button" class="lanzar lanzar--vivo" onclick={() => (pestana = 'despliegue')}>
              <span class="pulso" aria-hidden="true"></span> Desplegando…
            </button>
          {:else}
            <button type="button" class="lanzar" onclick={() => prepararDespliegue(elegida!)}>
              Desplegar
            </button>
          {/if}
        </nav>
        {#key `${alias}|${elegida.name}|${pestana}`}
        <Frontera que="{QUE_PESTANA[pestana] ?? pestana} de {elegida.name}">
        {#if errorPestana && pestana !== 'despliegue'}
          <div class="log-envoltorio">
            {#if errorPestana.clase === 'sin-datos'}
              <!-- Un estado, no un fallo: una app sin visitas en la ventana
                   pedida no tiene tráfico que contar, y eso no es un cero. -->
              <p class="vacio">{errorPestana.mensaje} en los últimos siete días.</p>
            {:else}
              <Fallo error={errorPestana} {alias} />
            {/if}
          </div>
        {:else if pestana === 'detalle'}
          {#if avisoDetalle}
            <!-- Lo que dijo Orbit, tal cual. Si falló, es su frase y no un
                 código de salida. -->
            <p class="aviso-detalle" class:aviso-detalle--mal={!avisoDetalle.ok} role="status">
              {avisoDetalle.texto}
            </p>
          {/if}
          <DetalleApp
            app={elegida}
            servidor={alias}
            info={detalleApp?.name === elegida.name ? detalleApp : null}
            tareas={tareasApp?.app === elegida.name ? tareasApp : null}
            puedeCongelar={hayPuente() && elegida.type !== 'redirect'}
            puedeEsperarCi={hayPuente() && elegida.type !== 'redirect'}
            alCongelar={(m) => pedirCongelar(elegida!, m)}
            alDescongelar={() => pedirDescongelar(elegida!)}
            alEsperarCi={(e) => pedirEsperarCi(elegida!, e)}
          />
        {:else if pestana === 'descriptor'}
          <OrbitJson app={elegida.name} info={detalleApp} {entorno} version={versionActual} />
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
                const yo = alias, app = elegida!.name
                try {
                  const res = await revertir(yo, app, r)
                  if (sigue(yo, app)) avisoDetalle = { ok: true, texto: res.salida.trim() || `Revertida a ${r}.` }
                } catch (e) {
                  if (sigue(yo, app)) avisoDetalle = { ok: false, texto: (e as ErrorDelPuente).mensaje }
                }
                // Lo que cuenta es cómo queda el servidor, no lo que dijo el
                // comando: se vuelve a preguntar en vez de parchear la fila.
                if (sigue(yo, app)) { pestana = 'detalle'; await refrescar() }
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
                const yo = alias, app = elegida!.name
                try {
                  if (borrarDatos) await retirarYBorrar(yo, app)
                  else await retirar(yo, app)
                } catch (e) {
                  // Si falla, la app sigue ahí: se dice en su pestaña y no se
                  // recarga como si se hubiera ido.
                  if (sigue(yo, app)) errorPestana = e as ErrorDelPuente
                  return
                }
                if (sigue(yo)) { elegida = null; apps = null; cargar(yo) }
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
                salida={v.salida}
                desde={v.desde}
                alDarVariables={hayPuente() ? (f) => prepararDespliegue(elegida!, f) : null}
                alVerLog={() => { log = null; verLog(elegida!) }}
                alDiagnostico={() => { doctor = null; verDiagnostico() }}
                alRevertir={() => verAdmin(elegida!, 'revertir')}
                alCancelar={hayPuente() ? () => cancelar(alias, elegida!.name) : null}
              />
            {/if}
          {/if}
        {:else if pestana === 'log' && seQuedaCorto(versionActual, 'contratoCompleto')}
          <div class="log-envoltorio">
            <p class="vacio">
              Este servidor tiene Orbit {versionActual}, y el log con contrato
              (<code>orbit logs --json</code>) llegó en la 1.3.7. Desde un terminal:
              <code>ssh {alias} orbit logs {elegida.name}</code>.
            </p>
          </div>
        {:else if log}
          <div class="log-envoltorio"><VisorLog {log} app={elegida.name} /></div>
        {:else}
          <div class="log-envoltorio"><Esqueleto filas={5} /></div>
        {/if}
        </Frontera>
        {/key}
      </section>
    {/if}
  </main>
</div>

<style>
  .marco { display: grid; grid-template-columns: 200px minmax(0, 1fr); min-height: 100vh; }
  .sr {
    position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; border: 0;
  }
  h1:focus { outline: none; }
  h1:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .servidor[aria-disabled='true'] { opacity: .55; cursor: not-allowed; }
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
  .pie-rail { margin-top: auto; padding-top: var(--e-4); display: grid; gap: 2px; }
  .atajo {
    display: flex; align-items: center; justify-content: space-between; gap: var(--e-2);
    background: none; border: 0; border-radius: var(--r-2); padding: var(--e-1) var(--e-2);
    font: inherit; font-size: 12px; color: var(--fg-faint); cursor: pointer; text-align: left;
  }
  .atajo:hover { color: var(--fg); background: var(--surface-2); }
  .atajo:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .atajo kbd {
    font-family: var(--mono); font-size: 10px; padding: 0 4px;
    border: 1px solid var(--border-strong); border-radius: 3px;
  }
  .aviso {
    padding: var(--e-3) var(--e-2) 0;
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
  .vistas, .pestanas { display: flex; flex-wrap: wrap; gap: var(--e-1); margin-left: auto; }
  .cabecera { flex-wrap: wrap; }
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
  .recargar { margin-left: auto; font-size: 13px !important; }
  .recargar:disabled { opacity: .5; cursor: progress; }
  .recargar + .lanzar { margin-left: var(--e-2); }
  .lanzar--vivo { display: inline-flex; align-items: center; gap: var(--e-2); color: var(--accent-text) !important; }
  .pulso {
    width: 7px; height: 7px; border-radius: 50%; background: var(--accent-fill);
    animation: latido 1.6s ease-in-out infinite;
  }
  @keyframes latido { 0%, 100% { opacity: .35 } 50% { opacity: 1 } }
  @media (prefers-reduced-motion: reduce) { .pulso { animation: none } }
  .vacio { margin: 0; font-size: 13px; color: var(--fg-muted); }
  .otra-vez {
    margin-top: var(--e-4);
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-1) var(--e-3); font: inherit; font-size: 12px; color: var(--fg-muted); cursor: pointer;
  }
  .otra-vez:disabled { opacity: .6; cursor: progress; }
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
  .acabada summary { font-size: 12px; color: var(--fg-muted); cursor: pointer; }
  .cerrar-nueva {
    justify-self: start;
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .aviso-detalle {
    margin: var(--e-4) var(--e-5) 0; padding: var(--e-2) var(--e-3);
    border: 1px solid var(--border); border-radius: var(--r-2);
    font-size: 12px; color: var(--fg-muted); white-space: pre-wrap;
  }
  .aviso-detalle--mal { color: var(--fg); border-color: var(--border-strong); }

  /* Con la ventana estrecha o al 200 % (WCAG 1.4.10), el rail deja de ser una
     columna de 200 px y pasa arriba, en una fila que se puede desplazar; y
     nada empuja la página hacia un lado. */
  @media (max-width: 900px) {
    .marco { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto 1fr; }
    .rail {
      flex-direction: row; flex-wrap: wrap; align-items: center; gap: var(--e-2);
      border-right: 0; border-bottom: 1px solid var(--border);
      padding: var(--e-2) var(--e-3);
    }
    .marca { margin: 0 var(--e-2) 0 0; }
    .rail ul { display: flex; flex-wrap: wrap; gap: 2px; }
    .servidor { width: auto; }
    .alta { margin-top: 0; }
    .pie-rail { margin-top: 0; padding-top: 0; margin-left: auto; display: flex; }
    .aviso { flex-basis: 100%; padding: 0 var(--e-2); }
    main { padding: var(--e-4) var(--e-3); }
    .vistas { margin-left: 0; }
    .pestanas { padding: var(--e-3) var(--e-3) 0; }
    .log-envoltorio { padding: var(--e-3); }
  }
</style>
