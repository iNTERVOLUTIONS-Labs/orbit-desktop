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

import {
  leerLog,
  type App, type Despliegue, type Doctor, type Entorno,
  type Envoltorio, type Lista, type Log, type Metricas, type Monitor, type Trafico,
} from './contrato'

import listaSana from './muestras/list.json'
import listaEstados from './muestras/list-estados.json'
import listaHostil from './muestras/list-nombre-hostil.json'
import listaVacia from './muestras/list-vacia.json'

export interface ErrorDelPuente {
  /** Un identificador estable. La interfaz decide **con esto** y nunca leyendo
   *  el texto: enseñar la pantalla bloqueante de la clave de host cambiada es
   *  una decisión, y tomarla comparando una cadena traducida es cómo se rompe
   *  al traducirla. */
  clase: string
  mensaje: string
  detalle?: string | null
}

export const MUESTRAS: Record<string, Lista> = {
  'vps-ovh': listaSana as Lista,
  pruebas: listaEstados as Lista,
  comprometido: listaHostil as Lista,
  'recien-instalado': listaVacia as Lista,
}

/** Si hay envoltorio de escritorio detrás. */
export function hayPuente(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function invocar<T>(nombre: string, args: Record<string, unknown> = {}): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(nombre, args)
}

export interface AliasSsh {
  alias: string
  hostname: string | null
  usuario: string | null
  puerto: number | null
  salto: string | null
  identidad: string | null
}

/** Los alias de `~/.ssh/config`. No conecta con ninguno: enumerar no es
 *  visitar, y abrir una pantalla no puede significar abrir cuarenta sesiones. */
export async function servidoresDelConfig(): Promise<AliasSsh[]> {
  if (!hayPuente()) return Object.keys(MUESTRAS).map((alias) => ({
    alias, hostname: null, usuario: null, puerto: null, salto: null, identidad: null,
  }))
  return invocar<AliasSsh[]>('servidores_del_config')
}

/**
 * La portada. Pide `status` y no `list` **a propósito**: `status --json` trae
 * el array de apps completo e idéntico al de `list --json` —comprobado
 * comparando los dos objetos— así que una llamada de 389 ms sustituye a dos que
 * suman 695. Un 44 % menos en la carga que forma la primera impresión, y no
 * cuesta nada: es elegir bien qué comando se pide.
 */
export async function portada(alias: string): Promise<{ apps: App[] }> {
  if (!hayPuente()) {
    const m = MUESTRAS[alias]
    if (!m) throw { clase: 'sin-muestra', mensaje: `no hay muestra para «${alias}»` }
    return { apps: m.apps }
  }
  return invocar<{ apps: App[] }>('portada', { alias })
}

import doctorMuestra from './muestras/doctor.json'
import logMuestra from './muestras/logs.ndjson?raw'

/** El diagnóstico del servidor. */
export async function diagnostico(alias: string): Promise<Doctor> {
  if (!hayPuente()) return doctorMuestra as Doctor
  return invocar<Doctor>('doctor', { alias })
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
  if (!hayPuente()) return leerLog(logMuestra)
  const crudo = await invocar<string>('logs', { alias, app, desde })
  return leerLog(crudo)
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
  return invocar<Doctor>('doctor_arreglar', { alias })
}

/**
 * Lanza un despliegue.
 *
 * El progreso NO vuelve por aquí: sale como eventos `orbit://progreso` según
 * ocurre, y esta promesa se resuelve con el objeto final. Devolverlo todo junto
 * al terminar convertiría tres minutos de información en un bloque de texto que
 * llega cuando ya no sirve.
 */
export async function desplegar(alias: string, app: string): Promise<Despliegue> {
  if (!hayPuente()) throw { clase: 'sin-puente', mensaje: 'no hay envoltorio de escritorio' }
  return invocar<Despliegue>('desplegar', { alias, app })
}

/** Cancela un despliegue en curso. Devuelve si había uno que cancelar. */
export async function cancelar(alias: string, app: string): Promise<boolean> {
  if (!hayPuente()) return false
  return invocar<boolean>('cancelar', { alias, app })
}

import entornoMuestra from './muestras/env.json'
import monitorMuestra from './muestras/top.json'

/** Los **nombres** de las variables. Nunca los valores. */
export async function entorno(alias: string, app: string): Promise<Entorno> {
  if (!hayPuente()) return { ...(entornoMuestra as Entorno), app }
  return invocar<Entorno>('entorno', { alias, app })
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
  return invocar<string>('entorno_valor', { alias, app, clave })
}

/** El monitor. Tarda ~2,1 s con 40 apps, y no es lentitud. */
export async function monitor(alias: string): Promise<Monitor> {
  if (!hayPuente()) return monitorMuestra as Monitor
  return invocar<Monitor>('monitor', { alias })
}

import traficoMuestra from './muestras/traffic.json'
import metricasMuestra from './muestras/metrics.json'

/** El tráfico. Viene **envuelto** —`{schema, apps:[…]}`— también pidiendo una
 *  sola app: lo comprobamos ejecutando, porque la documentación lo daba plano. */
export async function trafico(alias: string, app: string, desde = '7d'): Promise<Trafico> {
  const r = hayPuente()
    ? await invocar<Envoltorio<Trafico>>('trafico', { alias, app, desde })
    : (traficoMuestra as Envoltorio<Trafico>)
  const t = r.apps[0]
  if (!t) throw { clase: 'sin-datos', mensaje: `no hay tráfico de «${app}»` }
  return t
}

/** Las métricas. También envueltas. */
export async function metricas(alias: string, app: string): Promise<Metricas> {
  const r = hayPuente()
    ? await invocar<Envoltorio<Metricas>>('metricas', { alias, app })
    : (metricasMuestra as Envoltorio<Metricas>)
  const m = r.apps[0]
  if (!m) throw { clase: 'sin-datos', mensaje: `no hay métricas de «${app}»` }
  return m
}
