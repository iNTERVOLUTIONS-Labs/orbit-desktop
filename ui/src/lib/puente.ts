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

import { leerLog, type App, type Doctor, type Lista, type Log } from './contrato'

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
