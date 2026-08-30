// Los despliegues en curso.
//
// Se pueden lanzar **varios a la vez**, en apps y servidores distintos: cada
// uno es un proceso SSH independiente y nada en el servidor los coordina, ni
// falta. La clave es `servidor:app` y nunca la app sola, porque `tienda` existe
// en tres servidores y son tres despliegues distintos — el accidente más caro
// de un cliente multiservidor no es un ataque, es confundir dos.
//
// Un despliegue **sobrevive a que alguien se vaya a otra pantalla**: por eso
// esto vive aquí y no dentro del componente. Un modal de cuatro minutos
// secuestraría la aplicación.

import { leerProgreso, type Progreso } from './despliegue'
import type { Despliegue } from './contrato'
import { hayPuente } from './puente'

export interface Vivo {
  servidor: string
  app: string
  /** Las líneas crudas, tal cual llegaron. Es lo que se pega en un issue. */
  crudo: string
  progreso: Progreso
  resultado: Despliegue | null
  error: string | null
}

const registro = $state<Record<string, Vivo>>({})

export function clave(servidor: string, app: string): string {
  return `${servidor}:${app}`
}

export function vivos(): Vivo[] {
  return Object.values(registro)
}

/** Cuántos hay corriendo **ahora mismo** en un servidor. Es lo que lleva el
 *  contador del rail: volver a un despliegue tiene que ser un clic. */
export function enCurso(servidor?: string): Vivo[] {
  return vivos().filter(
    (v) => v.resultado === null && v.error === null && (!servidor || v.servidor === servidor),
  )
}

export function ver(servidor: string, app: string): Vivo | undefined {
  return registro[clave(servidor, app)]
}

export function empezar(servidor: string, app: string): void {
  registro[clave(servidor, app)] = {
    servidor, app, crudo: '',
    progreso: leerProgreso(''),
    resultado: null, error: null,
  }
}

/** Una línea de progreso, según llega. */
export function anotar(k: string, linea: string): void {
  const v = registro[k]
  if (!v) return
  v.crudo += linea + '\n'
  // El avance anterior se pasa siempre: la barra es monótona creciente y una
  // que retrocede destruye más confianza que cualquier error.
  v.progreso = leerProgreso(v.crudo, { app: v.app, anterior: v.progreso.avance })
}

export function terminar(k: string, r: Despliegue): void {
  const v = registro[k]
  if (!v) return
  v.resultado = r
}

/**
 * Se ha perdido el contacto.
 *
 * **No se llama fallo**, y esto es lo importante: el despliegue sigue en el
 * servidor y el cliente ya no sabe qué pasó. Decir «ha fallado» sería afirmar
 * algo que no se sabe, y decir «ha ido bien» también. La frase es incómoda y
 * verdadera, y viene con la forma de averiguarlo.
 */
export function perder(k: string, motivo: string): void {
  const v = registro[k]
  if (!v) return
  v.error = motivo
}

export function olvidar(k: string): void {
  delete registro[k]
}

/** Engancha el flujo del envoltorio. Fuera de él no hay eventos, así que la
 *  interfaz se puede mirar sin servidor y sin fingir uno. */
export async function escuchar(): Promise<void> {
  if (!hayPuente()) return
  const { listen } = await import('@tauri-apps/api/event')
  await listen<[string, string]>('orbit://progreso', (e) => {
    const [k, linea] = e.payload
    anotar(k, linea)
  })
}
