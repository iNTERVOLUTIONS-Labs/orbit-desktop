/**
 * Las reglas del sistema de diseño, comprobadas leyendo los ficheros.
 *
 * Están aquí y no en una lista de revisión por el mismo motivo que las del
 * núcleo: **una regla que depende de que alguien se acuerde no sobrevive al
 * commit 400.**
 */
import { describe, expect, it } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const SRC = join(import.meta.dirname, '..', 'src')

function ficheros(dir: string, ext: string[]): string[] {
  const out: string[] = []
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name)
    if (e.isDirectory()) out.push(...ficheros(p, ext))
    else if (ext.some((x) => e.name.endsWith(x))) out.push(p)
  }
  return out
}

/** Sin comentarios: un comentario que menciona un color es exactamente lo que
 *  queremos que exista, porque ahí es donde está el porqué. */
function sinComentarios(s: string): string {
  return s.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '')
}

describe('el color vive en la capa de tokens', () => {
  it('no hay colores literales fuera de tokens.css', () => {
    // El día que alguien escriba `#ff0000` en un componente, el sistema de
    // diseño deja de ser el sistema y pasa a ser una sugerencia.
    const malos: string[] = []
    for (const f of ficheros(SRC, ['.svelte', '.css', '.ts'])) {
      if (f.endsWith('tokens.css')) continue
      const c = sinComentarios(readFileSync(f, 'utf8'))
      const hex = c.match(/#[0-9a-fA-F]{3,8}\b/g)
      const rgb = c.match(/\b(rgb|hsl)a?\(/g)
      if (hex || rgb) malos.push(`${f.replace(SRC, 'src')}: ${[...(hex ?? []), ...(rgb ?? [])].join(' ')}`)
    }
    expect(malos, 'los colores se declaran en tokens.css y se usan por variable').toEqual([])
  })
})

describe('el estado semántico vive en estado.css', () => {
  it('ningún componente define el aspecto de un estado por su cuenta', () => {
    // Es la regla que el informe de diseño puso como condición, y su motivo es
    // que **la distinción entre `served:false`, `service:null` y
    // `service:"stopped"` es el activo más valioso del producto**. Es
    // exactamente lo que se disuelve cuando el estado se copia en cuatro
    // componentes: el día que alguien añada una quinta copia con un matiz
    // distinto, la lista y el detalle dejan de decir lo mismo, y nadie lo
    // notará hasta que un usuario vea un incidente donde no lo hay.
    const malos: string[] = []
    for (const f of ficheros(SRC, ['.svelte'])) {
      const c = sinComentarios(readFileSync(f, 'utf8'))
      const estilo = c.split('<style>')[1] ?? ''
      for (const tok of ['--st-ok', '--st-error', '--st-warn', '--st-na', '--st-unknown']) {
        if (estilo.includes(tok)) malos.push(`${f.replace(SRC, 'src')} usa ${tok} en su <style>`)
      }
    }
    expect(malos, 'los tokens de estado son de estado.css').toEqual([])
  })

  it('estado.css nombra los conceptos, no los colores', () => {
    // `.chip--sin-vhost` y no `.chip--rojo`: el día que el rojo cambie, el
    // nombre sigue siendo verdad.
    // Sin comentarios: el propio comentario de `estado.css` cita
    // «`.chip--rojo`» como ejemplo de lo que NO hay que hacer, así que leer el
    // fichero entero hacía que la comprobación se acusara a sí misma. Es el
    // mismo falso positivo que ya apareció en las reglas del núcleo, y por el
    // mismo motivo: una regla escrita sobre texto tiene que leer el texto que
    // manda, no el que lo explica.
    const c = sinComentarios(readFileSync(join(SRC, 'estilos', 'estado.css'), 'utf8'))
    for (const color of ['rojo', 'verde', 'ambar', 'red', 'green', 'amber']) {
      expect(c).not.toMatch(new RegExp(`\\.[a-z-]*--${color}\\b`))
    }
    for (const concepto of ['sin-vhost', 'mantenimiento', 'no-aplica', 'desconocido']) {
      expect(c).toContain(`--${concepto}`)
    }
  })

  it('«sin vhost» sigue siendo el único chip sólido', () => {
    // Si mañana hay dos sólidos, la distinción de forma deja de distinguir, y
    // con ella se va la única señal que sobrevive al daltonismo.
    const c = sinComentarios(readFileSync(join(SRC, 'estilos', 'estado.css'), 'utf8'))
    const solidos = c.match(/\.chip--[a-z-]+\s*\{[^}]*color:\s*var\(--on-solid\)/g) ?? []
    expect(solidos.length).toBe(1)
    expect(solidos[0]).toContain('sin-vhost')
  })

  it('los seis finales del lote tienen su propia clase, sin agrupar', () => {
    const c = sinComentarios(readFileSync(join(SRC, 'estilos', 'estado.css'), 'utf8'))
    for (const f of ['deployed', 'failed', 'unchanged', 'unreachable', 'gone', 'skipped']) {
      expect(c, `falta .lote--${f}`).toContain(`.lote--${f}`)
    }
    // Y ninguna clase que los agrupe.
    expect(c).not.toMatch(/\.lote--(ok|correctas|fallidas|malas)\b/)
  })
})

describe('el tema', () => {
  it('la elección explícita gana en los dos sentidos', () => {
    // El bug clásico de los temas hechos sólo con media query: el botón de tema
    // funciona para poner oscuro y no para volver a claro.
    const c = readFileSync(join(SRC, 'estilos', 'tokens.css'), 'utf8')
    expect(c).toContain(':root:not([data-theme="light"])')
    expect(c).toContain(':root[data-theme="dark"]')
  })
})

describe('los secretos no se guardan en ningún sitio', () => {
  it('nada de la interfaz escribe en el almacenamiento del navegador', () => {
    // T-12 del modelo de amenazas: toda aplicación de escritorio moderna guarda
    // estado sin que nadie lo decida —caché, estado serializado, localStorage—
    // y cualquiera de esos puede acabar con la salida de `orbit env get`
    // dentro.
    //
    // Esta comprobación se pone AHORA, que no hay nada que guardar. Ponerla
    // cuando ya se guardan quince cosas es una auditoría; ponerla ahora es una
    // línea.
    const malos: string[] = []
    for (const f of ficheros(SRC, ['.svelte', '.ts'])) {
      const c = sinComentarios(readFileSync(f, 'utf8'))
      for (const p of ['localStorage', 'sessionStorage', 'indexedDB', 'document.cookie']) {
        if (c.includes(p)) malos.push(`${f.replace(SRC, 'src')} usa ${p}`)
      }
    }
    expect(malos, 'antes de persistir algo hay que decidir qué, y añadir el barrido de secretos').toEqual([])
  })

  it('el valor revelado se borra de la memoria, no sólo de la vista', () => {
    // Dejarlo en una variable «oculta» es dejarlo en el volcado del día que la
    // aplicación se caiga.
    const c = readFileSync(join(SRC, 'componentes', 'Entorno.svelte'), 'utf8')
    expect(c).toMatch(/function ocultar\(\)[\s\S]*?valor = null/)
  })

  it('el registro nunca lleva el valor de una variable', () => {
    // Se hereda de Orbit, que apunta `exec <app>` y nunca el comando: un log no
    // es sitio para secretos.
    const c = sinComentarios(readFileSync(join(SRC, 'componentes', 'Entorno.svelte'), 'utf8'))
    expect(c).not.toMatch(/console\.(log|info|warn|error)/)
  })
})

describe('la capa de estado gana', () => {
  it('ninguna regla de estado pierde contra el <style> de un componente', () => {
    // Una regla anidada de `estado.css` —`.a .b { color: … }`— y un estilo con
    // ámbito de componente sobre `.b` tienen la MISMA especificidad, así que
    // gana el orden de aparición: el componente. La capa que manda sobre el
    // estado perdía en silencio, y se vio en una captura con las pruebas en
    // verde: el botón de borrar no salía rojo.
    //
    // La primera versión de esta comprobación **pasaba sin comprobar nada**:
    // excluía las reglas con `var(--…)`, que son justo todas. Se deja escrito
    // porque una prueba que pasa sin mirar es peor que no tenerla.
    //
    // La forma que no se rompe es el token, porque hereda y no compite.
    const estado = readFileSync(join(SRC, 'estilos', 'estado.css'), 'utf8')
    const sinCom = sinComentarios(estado)

    // Qué propiedades pinta `estado.css` sobre qué clase anidada.
    const anidadas = new Map<string, Set<string>>()
    for (const m of sinCom.matchAll(/\.[a-z0-9_-]+ +\.([a-z0-9_-]+)\s*\{([^}]*)\}/g)) {
      const clase = m[1]!
      const props = [...m[2]!.matchAll(/([a-z-]+)\s*:/g)].map((p) => p[1]!)
      if (!anidadas.has(clase)) anidadas.set(clase, new Set())
      props.forEach((p) => anidadas.get(clase)!.add(p))
    }

    const choques: string[] = []
    for (const f of ficheros(SRC, ['.svelte'])) {
      const c = sinComentarios(readFileSync(f, 'utf8'))
      const estilo = c.split('<style>')[1] ?? ''
      for (const [clase, props] of anidadas) {
        // La misma clase, a secas, en el <style> del componente.
        const re = new RegExp(`\\.${clase}\\s*(,[^{]*)?\\{([^}]*)\\}`, 'g')
        for (const m of estilo.matchAll(re)) {
          for (const p of props) {
            if (new RegExp(`(^|;)\\s*${p}\\s*:`).test(m[2]!)) {
              choques.push(`${f.replace(SRC, 'src')}: .${clase} { ${p} } pisa a estado.css`)
            }
          }
        }
      }
    }
    expect(
      [...new Set(choques)],
      'usa un token que herede en vez de una regla anidada: la anidada pierde',
    ).toEqual([])
  })
})
