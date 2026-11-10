// El puente hacia el núcleo.
//
// La interfaz **no habla SSH**, no construye órdenes y no toca el disco: pide
// cosas por nombre a la lista de comandos declarada en `crates/orbit-app`, que
// es toda la superficie que tiene. En Tauri el renderizador no tiene Node —no
// hay `fs`, ni `child_process`— así que esto no es una convención: es lo único
// que se puede hacer desde aquí.
//
// Fuera de Tauri —en `vite dev`, en las pruebas del DOM y en las capturas— no
// hay puente, y entonces se sirven las muestras. Eso NO es un modo de mentira
// que haya que mantener aparte: son **las mismas respuestas del servidor
// falso** que usan las pruebas del núcleo, así que la interfaz y el contrato no
// pueden divergir.

import { listaDeAlias, type Borrador } from './asistente'
import type { VariableNueva } from './despliegue'
import {
  leerLog, validar, objeto, lista, texto, numero, booleano, nulo,
  leerVersion, leerPortada, leerInfo, leerDoctor, leerDespliegue, leerLote,
  leerComprobacionDeEntorno, leerTareas, leerEntorno, leerMonitor, leerTraficos,
  leerMetricasEnvueltas, leerSalidaDeExec, leerSaludo, leerCopias, leerVerificacion,
  leerRedirecciones, leerBasesDeDatos, leerVigilancia, leerColas, leerLista,
  type Lector,
  type ComprobacionDeEntorno, type Despliegue, type Doctor, type Entorno,
  type Log, type Lote, type Metricas, type Monitor,
  type AppInfo, type SalidaDeExec, type Saludo, type Tareas, type Trafico,
  type Version, type Portada, type Copias, type Verificacion, type Redirecciones,
  type BasesDeDatos, type Vigilancia, type Colas,
} from './contrato'

export interface ErrorDelPuente {
  /** Un identificador estable. La interfaz decide **con esto** y nunca leyendo
   *  el texto: enseñar la pantalla bloqueante de la clave de host cambiada es
   *  una decisión, y tomarla comparando una cadena traducida es cómo se rompe
   *  al traducirla.
   *
   *  `forma` es la respuesta que llegó y no tiene la forma del contrato: la
   *  pone `validar()` aquí, no el núcleo. */
  clase: string
  mensaje: string
  detalle?: string | null
}

// Las muestras se cargan con `import()` y no con un `import` arriba: así
// viajan en trozos aparte que la ventana de verdad —que siempre tiene puente—
// no pide nunca. Importadas arriba iban en el paquete principal: ~6 KB de
// datos de mentira, ya minificados, en cada instalación.
const MUESTRAS: Record<string, () => Promise<unknown>> = {
  'vps-ovh': async () => (await import('./muestras/list.json')).default,
  pruebas: async () => (await import('./muestras/list-estados.json')).default,
  comprometido: async () => (await import('./muestras/list-nombre-hostil.json')).default,
  'recien-instalado': async () => (await import('./muestras/list-vacia.json')).default,
  // Un 1.4.8: congeladas, la puerta de la CI y el certificado de origen. Los
  // otros cuatro siguen siendo de un 1.3.6, a propósito: hablar con un
  // servidor viejo es el caso normal y la interfaz se mira también así.
  produccion: async () => (await import('./muestras/list-1.4.json')).default,
}

/** La versión de Orbit de cada muestra, para lo que depende de ella. */
function versionDeMuestra(alias: string): string {
  return alias === 'produccion' ? '1.4.8' : '1.3.6'
}

/** Si hay envoltorio de escritorio detrás. */
export function hayPuente(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function invocar(nombre: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<unknown>(nombre, args)
}

/** Pide y **mira la forma** antes de devolver. Es la única puerta: nada de lo
 *  que cruza el puente llega a la ventana sin pasar por su lector, y las
 *  muestras tampoco —que pasen es la prueba de que dicen lo mismo que el
 *  contrato—. `comando` es cómo se nombra en el error. */
async function pedir<T>(
  comando: string, lector: Lector<T>, nombre: string, args: Record<string, unknown> = {},
): Promise<T> {
  return validar(comando, lector, await invocar(nombre, args))
}

export interface AliasSsh {
  alias: string
  hostname: string | null
  usuario: string | null
  puerto: number | null
  salto: string | null
  identidad: string | null
}

const leerAliasSsh: Lector<AliasSsh> = objeto({
  alias: texto,
  hostname: nulo(texto),
  usuario: nulo(texto),
  puerto: nulo(numero),
  salto: nulo(texto),
  identidad: nulo(texto),
})

/** Los alias de `~/.ssh/config`. No conecta con ninguno: enumerar no es
 *  visitar, y abrir una pantalla no puede significar abrir cuarenta sesiones. */
export async function servidoresDelConfig(): Promise<AliasSsh[]> {
  if (!hayPuente()) return Object.keys(MUESTRAS).map((alias) => ({
    alias, hostname: null, usuario: null, puerto: null, salto: null, identidad: null,
  }))
  return pedir('servidores_del_config', lista(leerAliasSsh), 'servidores_del_config')
}

/**
 * La portada. Pide `status` y no `list` **a propósito**: `status --json` trae
 * el array de apps completo e idéntico al de `list --json` —comprobado
 * comparando los dos objetos— así que una llamada de 389 ms sustituye a dos que
 * suman 695. Un 44 % menos en la carga que forma la primera impresión, y no
 * cuesta nada: es elegir bien qué comando se pide.
 */
export async function portada(alias: string): Promise<Portada> {
  if (!hayPuente()) {
    // `hasOwn`: un alias se llama como quiera quien escribió su `ssh_config`,
    // y `constructor` no es una muestra.
    if (!Object.hasOwn(MUESTRAS, alias)) throw { clase: 'sin-muestra', mensaje: `no hay muestra para «${alias}»` }
    const m = validar('list', leerLista, await MUESTRAS[alias]!())
    // El host y los servicios de la muestra son los del banco: así la vista del
    // servidor se puede mirar sin servidor. Las apps, las de cada muestra.
    const st = (await import('./muestras/status.json')).default as Record<string, unknown>
    return validar('status', leerPortada, { ...st, apps: m.apps })
  }
  // El objeto entero, no sólo las apps: el host y los servicios llegan en la
  // misma llamada y antes se tiraban.
  return pedir('status', leerPortada, 'portada', { alias })
}

// ── lo del servidor que no es una app ──────────────────────────────────────
//
// Todo bajo demanda: se pide al abrir su sección, no al abrir el servidor. Las
// muestras se cargan con `import()` para que no viajen en el paquete de la
// ventana de verdad, que nunca las usa.

/** `backup list --json`, 1.3.7. */
export async function copias(alias: string): Promise<Copias> {
  if (!hayPuente()) return validar('backup list', leerCopias, (await import('./muestras/backup-list.json')).default)
  return pedir('backup list', leerCopias, 'copias', { alias })
}

/** `backup verify --json`, 1.3.7. **Abre cada fichero**, y puede tardar: es un
 *  gesto aparte, nunca algo que se lance al entrar. Con alguna mala el
 *  servidor sale con 1, y eso no es un fallo de transporte: llega el objeto. */
export async function verificarCopias(alias: string): Promise<Verificacion> {
  if (!hayPuente()) {
    return validar('backup verify', leerVerificacion, (await import('./muestras/backup-verify.json')).default)
  }
  return pedir('backup verify', leerVerificacion, 'verificar_copias', { alias })
}

export async function redirecciones(alias: string): Promise<Redirecciones> {
  if (!hayPuente()) {
    return validar('redirect list', leerRedirecciones, (await import('./muestras/redirect-list.json')).default)
  }
  return pedir('redirect list', leerRedirecciones, 'redirecciones', { alias })
}

/** `db list --json`. Con PostgreSQL caído el servidor **no** dice «ninguna»:
 *  sale con error, y aquí llega como error. */
export async function basesDeDatos(alias: string): Promise<BasesDeDatos> {
  if (!hayPuente()) return validar('db list', leerBasesDeDatos, (await import('./muestras/db-list.json')).default)
  return pedir('db list', leerBasesDeDatos, 'bases_de_datos', { alias })
}

export async function vigilancia(alias: string): Promise<Vigilancia> {
  if (!hayPuente()) return validar('watch status', leerVigilancia, (await import('./muestras/watch-status.json')).default)
  return pedir('watch status', leerVigilancia, 'vigilancia', { alias })
}

export async function colas(alias: string): Promise<Colas> {
  if (!hayPuente()) return validar('queue status', leerColas, (await import('./muestras/queue-status.json')).default)
  return pedir('queue status', leerColas, 'colas', { alias })
}

/** El saludo corto: qué versión de Orbit tiene un servidor.
 *
 *  Es la llamada más barata del contrato (72 ms en el banco, ~13 con la
 *  conexión multiplexada) y se pide **una vez por servidor y sesión**: de ella
 *  sale qué botones se pueden ofrecer, y la versión de un servidor no cambia
 *  mientras se mira salvo que alguien lo actualice — y entonces se olvida. */
export async function version(alias: string): Promise<Version> {
  if (!hayPuente()) return validar('version', leerVersion, { schema: 1, version: versionDeMuestra(alias), contract: 1 })
  return pedir('version', leerVersion, 'version', { alias })
}

/** El diagnóstico del servidor. */
export async function diagnostico(alias: string): Promise<Doctor> {
  if (!hayPuente()) return validar('doctor', leerDoctor, (await import('./muestras/doctor.json')).default)
  return pedir('doctor', leerDoctor, 'doctor', { alias })
}

/**
 * El log de una app.
 *
 * Se pide **sin seguir en vivo**, que es lo que hace `orbit logs --json` por
 * defecto desde que existe el contrato: en modo máquina, una foto. El flujo en
 * vivo es otra pantalla y otro canal, porque no termina nunca y hay que poder
 * cerrarlo.
 */
export async function log(alias: string, app: string, desde = '1h'): Promise<Log> {
  if (!hayPuente()) return leerLog((await import('./muestras/logs.ndjson?raw')).default)
  // El NDJSON crudo: cada línea la mira `leerLog`, y lo que no se entiende se
  // cuenta. Lo de fuera tiene que ser texto, o no hay nada que leer.
  return leerLog(await pedir('logs', texto, 'logs', { alias, app, desde }))
}

/**
 * Aplica lo que el servidor sabe arreglar solo.
 *
 * Existe desde que `orbit doctor --fix --json --yes` funciona: estuvo
 * documentado y muerto durante versiones, y hasta que se arregló esta pantalla
 * sólo podía enseñar el texto. Si el servidor es más viejo, esto falla y la
 * interfaz vuelve a enseñar la orden para copiar — que es por lo que la
 * capacidad se pregunta y no se supone.
 */
export async function arreglar(alias: string): Promise<Doctor> {
  return pedir('doctor --fix', leerDoctor, 'doctor_arreglar', { alias })
}

/**
 * Lanza un despliegue.
 *
 * El progreso NO vuelve por aquí: sale como eventos `orbit://progreso` según
 * ocurre, y esta promesa se resuelve con el objeto final. Devolverlo todo junto
 * al terminar convertiría tres minutos de información en un bloque de texto que
 * llega cuando ya no sirve.
 */
export async function desplegar(
  alias: string,
  app: string,
  opciones: { entorno?: VariableNueva[]; forzar?: boolean } = {},
): Promise<Despliegue> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  // Los valores cruzan el puente una vez y no se guardan aquí: quien llama
  // tiene que olvidarlos en cuanto esto vuelve, haya ido como haya ido.
  return pedir('deploy', leerDespliegue, 'desplegar', {
    alias,
    app,
    entorno: opciones.entorno ?? [],
    forzar: opciones.forzar ?? false,
  })
}

/** Lo que la app declara frente a lo que tiene, **sin un solo valor**. 1.3.8. */
export async function comprobarEntorno(alias: string, app: string): Promise<ComprobacionDeEntorno> {
  if (!hayPuente()) {
    const m = (await import('./muestras/env-check.json')).default
    return validar('env check', leerComprobacionDeEntorno, { ...m, app })
  }
  return pedir('env check', leerComprobacionDeEntorno, 'comprobar_entorno', { alias, app })
}

/** Las tareas programadas de una app. 1.3.9. */
export async function tareas(alias: string, app: string): Promise<Tareas> {
  if (!hayPuente()) return validar('cron list', leerTareas, { ...(await import('./muestras/cron.json')).default, app })
  return pedir('cron list', leerTareas, 'tareas', { alias, app })
}

/** Congela los despliegues de una app. El rollback sigue permitido. 1.4.3. */
export async function congelar(alias: string, app: string, motivo: string | null): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('lock', leerResultado, 'congelar', { alias, app, motivo })
}

export async function descongelar(alias: string, app: string): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('unlock', leerResultado, 'descongelar', { alias, app })
}

/** Que el autodespliegue y `deploy --all` esperen a la CI de GitHub. 1.4.3. */
export async function esperarCi(alias: string, app: string, esperar: boolean): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('autodeploy checks', leerResultado, 'esperar_ci', { alias, app, esperar })
}

/** Cancela un despliegue en curso. Devuelve si había uno que cancelar. */
export async function cancelar(alias: string, app: string): Promise<boolean> {
  if (!hayPuente()) return false
  return pedir('cancelar', booleano, 'cancelar', { alias, app })
}

/** Los **nombres** de las variables. Nunca los valores. */
export async function entorno(alias: string, app: string): Promise<Entorno> {
  if (!hayPuente()) return validar('env list', leerEntorno, { ...(await import('./muestras/env.json')).default, app })
  return pedir('env list', leerEntorno, 'entorno', { alias, app })
}

/**
 * **Un** valor, de uno en uno.
 *
 * Cuesta una llamada de verdad al servidor, con su latencia, y así debe ser:
 * pedir un secreto tiene que ser un acto explícito y visible, no un campo que
 * ya venía en la respuesta anterior.
 */
export async function entornoValor(alias: string, app: string, clave: string): Promise<string> {
  if (!hayPuente()) {
    // En la muestra no hay secretos que enseñar, y **no se inventa uno que
    // parezca real**: una contraseña de mentira en una captura de la
    // documentación acaba pareciendo una de verdad.
    await new Promise((r) => setTimeout(r, 250))
    return `(sin servidor: el valor de ${clave} vendría de «orbit env get»)`
  }
  return pedir('env get', texto, 'entorno_valor', { alias, app, clave })
}

/** El monitor. Tarda ~2,1 s con 40 apps, y no es lentitud. */
export async function monitor(alias: string): Promise<Monitor> {
  if (!hayPuente()) return validar('top', leerMonitor, (await import('./muestras/top.json')).default)
  return pedir('top', leerMonitor, 'monitor', { alias })
}

/** El tráfico. Viene **envuelto** —`{schema, apps:[…]}`— también pidiendo una
 *  sola app: lo comprobamos ejecutando, porque la documentación lo daba plano. */
export async function trafico(alias: string, app: string, desde = '7d'): Promise<Trafico> {
  const r = hayPuente()
    ? await pedir('traffic', leerTraficos, 'trafico', { alias, app, desde })
    : validar('traffic', leerTraficos, (await import('./muestras/traffic.json')).default)
  const t = r.apps[0]
  if (!t) throw { clase: 'sin-datos', mensaje: `no hay tráfico de «${app}»` }
  return t
}

/** Las métricas. También envueltas. */
export async function metricas(alias: string, app: string): Promise<Metricas> {
  const r = hayPuente()
    ? await pedir('metrics', leerMetricasEnvueltas, 'metricas', { alias, app })
    : validar('metrics', leerMetricasEnvueltas, (await import('./muestras/metrics.json')).default)
  const m = r.apps[0]
  if (!m) throw { clase: 'sin-datos', mensaje: `no hay métricas de «${app}»` }
  return m
}

/**
 * Ejecuta algo dentro de una app. **La puerta trasera.**
 *
 * `shell: true` manda el texto como un argumento y el servidor lo pasa a
 * `bash -lc`; `false` manda los argumentos separados. La diferencia la elige
 * quien escribe, y la ve.
 */
export async function correr(
  alias: string, app: string, shell: boolean, argumentos: string[],
): Promise<SalidaDeExec> {
  if (!hayPuente()) {
    // Sin servidor **no se simula una salida**. Fingir que un comando se
    // ejecutó en un servidor que no existe es la clase de mentira que esta
    // pantalla no se puede permitir: alguien miraría la salida y creería que su
    // migración corrió.
    await new Promise((r) => setTimeout(r, 200))
    return {
      orden: `orbit exec ${app} ${shell ? JSON.stringify(argumentos.join(' ')) : argumentos.join(' ')}`,
      stdout: '',
      stderr: 'Sin envoltorio de escritorio: aquí no se ejecuta nada.\n' +
              'Esta pantalla no simula salidas, porque una salida de mentira se lee igual que una de verdad.\n',
      codigo: 1,
    }
  }
  return pedir('exec', leerSalidaDeExec, 'correr', { alias, app, shell, argumentos })
}

/** El detalle de una app. De aquí sale el inventario de lo que se pierde al
 *  borrarla, y se pide **en ese momento**. */
export async function detalle(alias: string, app: string): Promise<AppInfo> {
  if (!hayPuente()) {
    if (alias === 'produccion') {
      // El estado de la muestra es el de la app que se pidió, no el de la que
      // trae el fichero: una congelación pintada en otra app sería mentir.
      const l = validar('list', leerLista, await MUESTRAS.produccion!())
      const a = l.apps.find((x) => x.name === app)
      const i = validar('info', leerInfo, (await import('./muestras/info-1.4.json')).default).app
      return { ...i, name: app, state: a ? { ...a.state, cert_days: a.state.ssl ? 61 : null } : i.state }
    }
    return { ...validar('info', leerInfo, (await import('./muestras/info.json')).default).app, name: app }
  }
  return (await pedir('info', leerInfo, 'detalle', { alias, app })).app
}

export interface Resultado { salida: string; codigo: number }

/** `{salida, codigo}`: lo compone el núcleo con la prosa de una orden sin
 *  `--json`. */
const leerResultado: Lector<Resultado> = objeto({ salida: texto, codigo: numero })

/** Retira una app **sin borrar sus datos**. Reversible. */
export async function retirar(alias: string, app: string): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('remove', leerResultado, 'retirar', { alias, app })
}

/** Retira una app **y borra sus datos**. Irreversible.
 *
 *  Aquí no hay red debajo: `orbit remove -y --purge` no pregunta nada, así que
 *  toda la protección está en la pantalla que llama a esto. */
export async function retirarYBorrar(alias: string, app: string): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('remove --purge', leerResultado, 'retirar_y_borrar', { alias, app })
}

export async function revertir(alias: string, app: string, release: string): Promise<Resultado> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('rollback', leerResultado, 'revertir', { alias, app, release })
}

/**
 * Una pasada por todas las apps del servidor.
 *
 * `soloSiCambia` **no es una opción, es la mitad del significado**: con él se le
 * pregunta primero al remoto de cada app y sólo se despliega lo que se ha
 * movido; sin él se recompila todo, haya cambiado o no. Con cuarenta apps eso
 * son cuarenta builds y cuarenta releases nuevas de un código idéntico, así que
 * en la interfaz son dos entradas y nunca una casilla.
 *
 * El progreso llega por `orbit://progreso` con la clave `alias:*`.
 */
export async function desplegarTodo(alias: string, soloSiCambia: boolean): Promise<Lote> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('deploy --all', leerLote, 'desplegar_todo', { alias, soloSiCambia })
}

/** Lo que devuelve `orbit new`: el texto y el código, **sin interpretar
 *  ninguno de los dos**. */
export interface Creacion {
  codigo: number
  salida: string
}

const leerCreacion: Lector<Creacion> = objeto({ codigo: numero, salida: texto })

/** Crea una web.
 *
 *  Es la única llamada del puente que **devuelve un código de salida sin
 *  clasificarlo**, y no es un descuido: `orbit new` despliega por dentro y
 *  puede terminar en 1 con la aplicación creada, registrada y con vhost. Leer
 *  ese 1 como «ha fallado» sería decirle a alguien que no tiene nada cuando
 *  tiene una web publicada a la que le falta el certificado.
 *
 *  Quien llama a esto no mira ni el código ni la prosa: al terminar pregunta
 *  con `detalle()`, que sí tiene contrato, y clasifica con `clasificar()`. Es
 *  la única forma que no depende del idioma del servidor.
 *
 *  El progreso llega por `orbit://progreso`, igual que el de un despliegue, con
 *  la clave `alias:nombre`. */
export async function crear(
  alias: string,
  b: Borrador,
  binario?: string,
): Promise<Creacion> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  const a = b.ajustes
  return pedir('new', leerCreacion, 'crear', {
    alias,
    nombre: b.nombre.trim(),
    repo: b.repo.trim(),
    rama: b.rama.trim(),
    dominio: b.dominio.trim(),
    aliasDominio: listaDeAlias(b),
    correo: b.correo.trim() === '' ? null : b.correo.trim(),
    baseDeDatos: b.baseDeDatos,
    https: b.https,
    ajustes: {
      carpeta: a.carpeta.trim() === '' ? null : a.carpeta.trim(),
      tipo: a.tipo.trim() === '' ? null : a.tipo.trim(),
      // La unión etiquetada cruza TAL CUAL, con su `modo` escrito.
      //
      // La primera versión mandaba `undefined` para «callar» y `null` para
      // «vacío a propósito», y no funcionaba: al otro lado serde colapsa las dos
      // en el mismo valor, así que el `--build \'\'` no llegaba nunca y esta
      // pantalla habría enseñado en el repaso una orden que el servidor no
      // recibía. Es exactamente el fallo que el repaso existe para hacer
      // imposible.
      build: a.build,
      arranque: a.arranque,
      outdir: a.outdir,
    },
    binario,
  })
}

export interface Resolucion {
  del_dominio: string[]
  del_servidor: string[]
  /** `null` es **no lo sé** —falta una de las dos listas— y no se pinta como
   *  «no coinciden». */
  coinciden: boolean | null
}

const leerResolucion: Lector<Resolucion> = objeto({
  del_dominio: lista(texto), del_servidor: lista(texto), coinciden: nulo(booleano),
})

/** A dónde apunta un nombre, y a dónde apunta el servidor.
 *
 *  Contesta antes de crear la única pregunta del asistente que no sale del
 *  contrato: una web publicada cuyo dominio no apunta aquí se ve, desde dentro,
 *  exactamente igual que una que sí. Es el final F7, y descubrirlo después de
 *  tres minutos de build es descubrirlo tarde.
 *
 *  Sin envoltorio no se inventa una respuesta: se dice que no se sabe. */
export async function resolver(nombre: string, alias: string): Promise<Resolucion> {
  if (!hayPuente()) return { del_dominio: [], del_servidor: [], coinciden: null }
  return pedir('resolver', leerResolucion, 'resolver', { nombre, alias })
}

// ── los servidores propios ──────────────────────────────────────────────────

/** Un servidor añadido a mano.
 *
 *  Existe porque la primera versión sacaba los servidores **sólo** del
 *  `~/.ssh/config`, y eso era un error de base disfrazado de decisión elegante:
 *  mucha gente no tiene ese fichero, y en Windows casi nadie. La aplicación
 *  abría vacía y sin salida. */
export interface ServidorPropio {
  alias: string
  host: string
  usuario: string
  puerto: number
  /** La **ruta** de la clave. Nunca su contenido ni su frase de paso: eso no
   *  cruza este puente en ninguna dirección. */
  clave: string | null
  binario: string | null
}

const leerServidoresPropios: Lector<ServidorPropio[]> = lista(objeto({
  alias: texto, host: texto, usuario: texto, puerto: numero, clave: nulo(texto), binario: nulo(texto),
}))

export async function servidoresGuardados(): Promise<ServidorPropio[]> {
  if (!hayPuente()) return []
  return pedir('servidores_guardados', leerServidoresPropios, 'servidores_guardados')
}

export async function guardarServidor(s: ServidorPropio): Promise<ServidorPropio[]> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('guardar_servidor', leerServidoresPropios, 'guardar_servidor', { servidor: s })
}

export async function olvidarServidor(alias: string): Promise<ServidorPropio[]> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('olvidar_servidor', leerServidoresPropios, 'olvidar_servidor', { alias })
}

// ── instalar Orbit ──────────────────────────────────────────────────────────

export interface Impedimento {
  /** Identificador estable: se decide con esto y nunca leyendo el texto. */
  clase: string
  que: string
  /** Qué haría falta, cuando hay algo que se pueda decir. `null` cuando la
   *  salida es una decisión de quien manda en el servidor y no una orden. */
  arreglo: string | null
}

export interface Requisitos {
  git: boolean
  root: boolean
  sudo_sin_contrasena: boolean
  ya_instalado: boolean
  sistema: string
  puede: boolean
  impedimentos: Impedimento[]
  avisos: string[]
  /** La secuencia literal. Llega **siempre**, también cuando no se puede
   *  instalar desde aquí: quien no pueda igualmente la puede copiar. */
  pasos: string[]
}

const leerRequisitos: Lector<Requisitos> = objeto({
  git: booleano,
  root: booleano,
  sudo_sin_contrasena: booleano,
  ya_instalado: booleano,
  sistema: texto,
  puede: booleano,
  impedimentos: lista(objeto({ clase: texto, que: texto, arreglo: nulo(texto) })),
  avisos: lista(texto),
  pasos: lista(texto),
})

/** Qué hay en el otro lado, **antes de tocar nada**.
 *
 *  Una sola conexión. Descubrir a mitad de una instalación que falta `git` deja
 *  un servidor a medias y a quien mira sin saber qué ha quedado hecho. */
export async function requisitosDeInstalacion(alias: string): Promise<Requisitos> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('requisitos_de_instalacion', leerRequisitos, 'requisitos_de_instalacion', { alias })
}

export interface Instalacion {
  codigo: number
  salida: string
  /** **Lo único en lo que se cree.** Sale de preguntarle a `orbit version
   *  --json` después, no del código de salida ni de la prosa. */
  version: string | null
}

const leerInstalacion: Lector<Instalacion> = objeto({ codigo: numero, salida: texto, version: nulo(texto) })

/** Instala Orbit. Tarda entre cinco y diez minutos y no se puede acortar: son
 *  `apt`, Node, PostgreSQL y PHP.
 *
 *  El progreso llega por `orbit://progreso` con la clave `alias:!instalar`. */
export async function instalarOrbit(alias: string): Promise<Instalacion> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return pedir('instalar_orbit', leerInstalacion, 'instalar_orbit', { alias })
}

/**
 * Qué hay al otro lado de un alias.
 *
 * **No se llama al enumerar**: enumerar no es visitar, y abrir la pantalla de
 * servidores no puede significar abrir cuarenta sesiones SSH. Se pregunta por
 * uno, cuando alguien lo pide.
 */
export async function saludar(alias: string): Promise<Saludo> {
  if (!hayPuente()) {
    // Sin envoltorio se contesta lo que se sabe de verdad: que no hay servidor.
    // No se finge un «listo» — un servidor de mentira que dice estar bien es la
    // clase de respuesta que hace perder media hora buscando el problema donde
    // no está.
    await new Promise((r) => setTimeout(r, 300))
    return {
      clase: 'no-se-llega', version: null, contrato: null,
      motivo: 'Sin envoltorio de escritorio no hay SSH: esto no ha preguntado a nadie.',
      puede_operar: false, puede_leer: false,
      orden_de_instalacion: 'curl -fsSL https://raw.githubusercontent.com/iNTERVOLUTIONS-Labs/orbit/main/install.sh | sudo bash',
    }
  }
  return pedir('saludo', leerSaludo, 'saludar', { alias })
}
