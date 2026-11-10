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

import {
  lectorDePasada, lectorDeProgreso, leerPasada, leerProgreso,
  type LectorDePasada, type LectorDeProgreso, type Pasada, type Progreso,
} from './despliegue'
import type { Despliegue } from './contrato'
import { Salida } from './consola'
import { hayPuente } from './puente'

export interface Vivo {
  servidor: string
  app: string
  /** Las líneas crudas, tal cual llegaron: es lo que se pega en un issue.
   *  Un anillo con tope y **fuera de la reactividad**: sumarlas a un `string`
   *  en `$state` copiaba la salida entera en cada línea. Se pinta con
   *  `<Consola salida={v.salida} />`, que se suscribe sola. Una instancia
   *  nueva en cada `empezar`, así que también identifica la ejecución. */
  salida: Salida
  progreso: Progreso
  resultado: Despliegue | null
  error: string | null
  /** Cuándo se lanzó, en el reloj de este portátil. Es lo que hace que el
   *  cronómetro avance cada segundo aunque el servidor esté callado en mitad
   *  de un build: un reloj parado parece un cuelgue. */
  desde: number
}

const registro = $state<Record<string, Vivo>>({})
// Fuera del estado reactivo a propósito: es el analizador, no algo que se pinte.
const lectores = new Map<string, LectorDeProgreso>()

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
  const k = clave(servidor, app)
  lectores.set(k, lectorDeProgreso({ app }))
  registro[k] = {
    servidor, app, salida: new Salida(),
    progreso: leerProgreso(''),
    resultado: null, error: null,
    desde: Date.now(),
  }
}

/** Si esa app tiene un despliegue **corriendo** ahora mismo. Dos a la vez sobre
 *  la misma app mezclaban su progreso en el mismo registro, y en el servidor
 *  el segundo sólo espera al cerrojo del primero. */
export function corriendo(servidor: string, app: string): boolean {
  const v = registro[clave(servidor, app)]
  return !!v && v.resultado === null && v.error === null
}

/** Líneas de progreso, según llegan: una o la tanda que manda el núcleo. */
export function anotar(k: string, lineas: string | readonly string[]): void {
  const v = registro[k]
  if (!v) return
  const tanda = typeof lineas === 'string' ? [lineas] : lineas
  if (tanda.length === 0) return
  v.salida.anotarVarias(tanda)
  let l = lectores.get(k)
  if (!l) {
    l = lectorDeProgreso({ app: v.app })
    lectores.set(k, l)
  }
  // Sólo las líneas nuevas: volver a leer toda la salida en cada línea era
  // cuadrático, y un build de miles de líneas se iba frenando.
  let cambio = false
  for (const linea of tanda) if (l.linea(linea)) cambio = true
  // Y la foto sólo si algo cambió: casi todo lo que llega es prosa del build,
  // y una foto nueva por línea repintaba los pasos y la barra para nada.
  // El avance anterior se pasa siempre: la barra es monótona creciente y una
  // que retrocede destruye más confianza que cualquier error.
  if (cambio) v.progreso = l.foto(v.progreso.avance)
}

export function terminar(k: string, r: Despliegue): void {
  const v = registro[k]
  if (!v) return
  v.resultado = r
  lectores.delete(k)
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
  lectores.delete(k)
}

/** Lo quita del registro **sin mirar si sigue corriendo**: para las pruebas
 *  y para quien ya lo ha comprobado. La interfaz usa `olvidarTerminado`. */
export function olvidar(k: string): void {
  delete registro[k]
  lectores.delete(k)
}

/**
 * Suelta uno que ya terminó (con resultado o sin contacto), y con él su salida.
 *
 * Con uno **en marcha no hace nada** y devuelve `false`: el proceso SSH sigue
 * vivo, sus líneas seguirían llegando a ninguna parte y el contador del rail
 * diría que no hay nada corriendo cuando sí lo hay.
 */
export function olvidarTerminado(k: string): boolean {
  const v = registro[k]
  if (!v || (v.resultado === null && v.error === null)) return false
  olvidar(k)
  return true
}

/**
 * «Desplegar todo» mientras ocurre: su salida y lo que se lee de ella.
 *
 * Va aparte del registro de arriba porque no es el despliegue de una app:
 * llega con la clave `servidor:*`. El estado es `$state.raw` y se sustituye
 * entero **una vez por tanda** y sólo si algo cambió; volver a leer la salida
 * entera en cada línea costaba 18 s de hilo principal en una pasada de
 * treinta mil líneas.
 */
export class PasadaEnVivo {
  readonly salida = new Salida()
  #lector: LectorDePasada = lectorDePasada()
  estado = $state.raw<Pasada>(leerPasada(''))

  anotar(lineas: string | readonly string[]): void {
    const tanda = typeof lineas === 'string' ? [lineas] : lineas
    if (tanda.length === 0) return
    this.salida.anotarVarias(tanda)
    let cambio = false
    for (const l of tanda) if (this.#lector.linea(l)) cambio = true
    if (cambio) this.estado = this.#lector.foto()
  }

  vaciar(): void {
    this.salida.vaciar()
    this.#lector = lectorDePasada()
    this.estado = this.#lector.foto()
  }
}

/** Las líneas de un evento `orbit://progreso`: `[clave, líneas]`. Se acepta
 *  también la forma vieja, `[clave, línea]`, y lo que no tenga esa forma se
 *  descarta en vez de romper el oyente. */
export function lineasDelEvento(payload: unknown): [string, string[]] | null {
  if (!Array.isArray(payload) || payload.length < 2) return null
  const [k, l] = payload as [unknown, unknown]
  if (typeof k !== 'string') return null
  if (typeof l === 'string') return [k, [l]]
  if (Array.isArray(l)) return [k, l.filter((x): x is string => typeof x === 'string')]
  return null
}

let oyendo: Promise<boolean> | null = null

/** Engancha el flujo del envoltorio. Fuera de él no hay eventos, así que la
 *  interfaz se puede mirar sin servidor y sin fingir uno.
 *
 *  Una sola vez aunque se llame más: dos oyentes apuntarían cada línea dos
 *  veces. Si engancharse falla, **no se queda a medias**: se dice por consola,
 *  devuelve `false` y la siguiente llamada lo vuelve a intentar. */
export function escuchar(): Promise<boolean> {
  if (!hayPuente()) return Promise.resolve(false)
  oyendo ??= (async () => {
    try {
      const { listen } = await import('@tauri-apps/api/event')
      await listen<unknown>('orbit://progreso', (e) => {
        const r = lineasDelEvento(e.payload)
        if (r) anotar(r[0], r[1])
      })
      return true
    } catch (err) {
      console.error('No se ha podido escuchar el progreso de los despliegues:', err)
      oyendo = null
      return false
    }
  })()
  return oyendo
}
