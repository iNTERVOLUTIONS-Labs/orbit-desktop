// Comparar dos servidores.
//
// La pantalla más peligrosa del cliente, y no por lo que hace —dos lecturas, no
// escribe nada— sino por lo que puede hacer creer. Aquí conviven dos reglas que
// el resto del producto ya tenía por separado, y las dos se rompen justo aquí:
//
//   1. **La clave es `servidor:app`, nunca la app sola.** «tienda» existe en
//      tres servidores y son tres apps distintas. Esta pantalla las pone una al
//      lado de la otra a propósito, que es la situación en la que confundirlas
//      es más fácil.
//   2. **«No he podido preguntar» no es «no lo tiene».** Es literalmente el
//      fallo que costó que un remoto caído se anunciara como «nada que hacer»
//      durante días, y aquí sería peor: una app que existe en los dos, pintada
//      como «sólo en A» porque B no contestó, invita a crearla otra vez en B.
//
// De la segunda sale la decisión que sostiene todo el módulo:
//
// > **Media comparación no es una comparación.** Si uno de los dos lados no
// > contestó, no se compara nada. No se enseña la lista del que sí contestó con
// > los huecos del otro en blanco.

import type { App } from './contrato'

/** Qué campos se comparan, y con qué nombre se enseñan.
 *
 *  La lista es corta a propósito. Lo que se compara es lo que **está
 *  configurado**, no lo que está pasando ahora mismo:
 *
 *  · `service` y `port` cambian solos y ya salen en la portada de cada
 *    servidor. Meterlos aquí llenaría la comparación de diferencias ciertas y
 *    sin interés, y una lista de diferencias que casi siempre tiene ruido es una
 *    lista que se deja de leer.
 *  · `releases` y `last_deploy` difieren **siempre** entre dos servidores: uno
 *    lleva más tiempo o se despliega más. Que difieran no dice nada.
 *  · `cert_days` no se puede comparar aunque se quisiera: es `null` en `list` y
 *    en `status` **siempre**, y sólo lo calcula `info`.
 *  · `maintenance` es un interruptor momentáneo, y la portada ya lo enseña.
 */
export const CAMPOS: { id: string; etiqueta: string; porque: string }[] = [
  { id: 'type', etiqueta: 'tipo', porque: 'El stack detectado. Si difiere, no se despliegan igual.' },
  { id: 'domain', etiqueta: 'dominio', porque: 'Lo que sirve cada una.' },
  { id: 'aliases', etiqueta: 'alias', porque: 'Los dominios extra del certificado.' },
  {
    id: 'last_deploy_sha',
    etiqueta: 'commit desplegado',
    porque: 'Si difiere, uno de los dos va por delante — o van por ramas distintas.',
  },
  { id: 'served', etiqueta: 'vhost', porque: 'Sin él nginx cierra la conexión: ni 404 ni 502.' },
  { id: 'ssl', etiqueta: 'certificado', porque: 'Uno por HTTPS y el otro no.' },
  {
    id: 'autodeploy',
    etiqueta: 'autodespliegue',
    porque:
      'La diferencia que más caro sale: uno se despliega solo al empujar y el otro no, y nadie se acuerda de cuál.',
  },
]

export interface Diferencia {
  campo: string
  etiqueta: string
  porque: string
  a: string
  b: string
}

/** Una app, mirada en los dos servidores. */
export interface Fila {
  app: string
  a: App | null
  b: App | null
  diferencias: Diferencia[]
  /**
   * El nombre coincide y el dominio no.
   *
   * **No se afirma que sean la misma app**, porque no se sabe: un «blog» en dos
   * servidores puede ser el mismo proyecto en dos entornos o dos proyectos que
   * se llaman igual. Lo único que se puede hacer es enseñar los dos dominios y
   * dejar que lo decida quien mira, que sí lo sabe.
   */
  nombreIgualDominioDistinto: boolean
}

export interface Comparacion {
  enLosDos: Fila[]
  soloA: Fila[]
  soloB: Fila[]
  /** Cuántas de las que están en los dos tienen alguna diferencia. */
  conDiferencias: number
}

function texto(app: App, campo: string): string {
  if (campo === 'type') return app.type
  if (campo === 'domain') return app.domain
  if (campo === 'aliases') return app.aliases.length === 0 ? '—' : app.aliases.join(' ')
  const v = (app.state as unknown as Record<string, unknown>)[campo]
  if (v === null || v === undefined) return ''
  if (typeof v === 'boolean') return v ? 'sí' : 'no'
  return String(v)
}

/**
 * Compara dos portadas.
 *
 * Las dos listas tienen que venir **de una lectura de verdad de cada servidor**.
 * Esta función no sabe si uno de los dos falló, y por eso quien la llama tiene
 * prohibido pasarle una lista vacía por «no contestó»: eso saldría de aquí como
 * «no tiene ninguna app», que es una frase distinta y falsa. Esa comprobación
 * vive en la pantalla, antes de llegar hasta aquí.
 */
export function comparar(a: App[], b: App[]): Comparacion {
  const porA = new Map(a.map((x) => [x.name, x]))
  const porB = new Map(b.map((x) => [x.name, x]))

  const enLosDos: Fila[] = []
  const soloA: Fila[] = []
  const soloB: Fila[] = []

  // En orden alfabético y no en el que vinieron: aquí no hay nada ocurriendo,
  // y dos listas ordenadas distinto no se pueden leer en paralelo.
  const nombres = [...new Set([...porA.keys(), ...porB.keys()])].sort()

  for (const n of nombres) {
    const ea = porA.get(n) ?? null
    const eb = porB.get(n) ?? null

    if (ea && eb) {
      const diferencias: Diferencia[] = []
      for (const c of CAMPOS) {
        const ta = texto(ea, c.id)
        const tb = texto(eb, c.id)
        // Una cadena vacía es «el servidor no lo dice». Comparar contra eso
        // daría una diferencia donde sólo hay un hueco, que es la misma regla
        // por la que un `null` no se pinta como un cero.
        if (ta === '' || tb === '') continue
        if (ta !== tb) {
          diferencias.push({ campo: c.id, etiqueta: c.etiqueta, porque: c.porque, a: ta, b: tb })
        }
      }
      enLosDos.push({
        app: n,
        a: ea,
        b: eb,
        diferencias,
        nombreIgualDominioDistinto: ea.domain !== eb.domain,
      })
    } else if (ea) {
      soloA.push({ app: n, a: ea, b: null, diferencias: [], nombreIgualDominioDistinto: false })
    } else if (eb) {
      soloB.push({ app: n, a: null, b: eb, diferencias: [], nombreIgualDominioDistinto: false })
    }
  }

  return {
    enLosDos,
    soloA,
    soloB,
    conDiferencias: enLosDos.filter((f) => f.diferencias.length > 0).length,
  }
}

/** El commit, recortado para leerlo, **sin dejar de ser el commit**.
 *
 *  Siete caracteres es lo que enseña git, y es lo que hay que enseñar para poder
 *  comparar dos de un vistazo. El completo se conserva en el título, porque el
 *  recortado no sirve para pegarlo en ningún sitio. */
export function sha(s: string | null): string {
  return s === null ? '' : s.slice(0, 7)
}
