/**
 * El contraste, calculado desde los tokens.
 *
 * Los ratios de los comentarios de `tokens.css` se escribieron a mano, y uno
 * era mentira: el tenue daba 4,47:1 sobre el fondo y 4,20:1 sobre la superficie
 * hundida —el raíl, la caja de buscar—, y los chips neutros 4,10:1 dentro de su
 * propio tinte. Ninguna prueba lo veía porque ninguna hacía la cuenta. Ésta la
 * hace, para cada texto sobre cada superficie, en los dos temas.
 */
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const TOKENS = readFileSync(join(import.meta.dirname, '..', 'src', 'estilos', 'tokens.css'), 'utf8')
  .replace(/\/\*[\s\S]*?\*\//g, '')

/** Los bloques `selector { … }` de primer nivel o dentro del media query. */
function bloques(): { selector: string; cuerpo: string }[] {
  const out: { selector: string; cuerpo: string }[] = []
  for (const m of TOKENS.matchAll(/([^{}]+)\{([^{}]*)\}/g)) out.push({ selector: m[1]!.trim(), cuerpo: m[2]! })
  return out
}

function variables(cuerpo: string): Record<string, string> {
  const v: Record<string, string> = {}
  for (const m of cuerpo.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) v[m[1]!] = m[2]!.trim()
  return v
}

const todos = bloques()
const raices = todos.filter((b) => b.selector === ':root').map((b) => variables(b.cuerpo))
const paleta = Object.assign({}, ...raices) as Record<string, string>
const claro = raices.find((v) => '--bg' in v)!
const oscuroExplicito = variables(todos.find((b) => b.selector === ':root[data-theme="dark"]')!.cuerpo)
const oscuroSistema = variables(todos.find((b) => b.selector === ':root:not([data-theme="light"])')!.cuerpo)

function resolver(tema: Record<string, string>, nombre: string): string {
  let v = tema[nombre] ?? paleta[nombre]
  for (let i = 0; v && i < 10; i++) {
    const m = v.match(/^var\((--[a-z0-9-]+)\)$/)
    if (!m) break
    v = tema[m[1]!] ?? paleta[m[1]!]
  }
  if (!v || !/^#[0-9a-f]{6}$/i.test(v)) throw new Error(`${nombre} no resuelve a un color: ${v}`)
  return v
}

const rgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16))
const lum = (c: number[]) => {
  const [r, g, b] = c.map((x) => {
    x /= 255
    return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!
}
const ratio = (a: number[], b: number[]) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p)
  return (x! + 0.05) / (y! + 0.05)
}
/** `color-mix(in srgb, A p, B)`, que es como `estado.css` pinta un chip. */
const mezcla = (a: number[], b: number[], p: number) => a.map((x, i) => x * p + b[i]! * (1 - p))

const SUPERFICIES = ['--bg', '--surface', '--surface-2', '--surface-sunken']
const TEXTOS = ['--fg', '--fg-muted', '--fg-faint', '--accent-text', '--st-ok', '--st-warn', '--st-error', '--st-unknown', '--st-na']
const CHIPS = ['--st-ok', '--st-warn', '--st-error', '--st-unknown', '--st-na']

describe.each([
  ['claro', claro],
  ['oscuro elegido', { ...claro, ...oscuroExplicito }],
  ['oscuro del sistema', { ...claro, ...oscuroSistema }],
])('tema %s', (_, tema) => {
  it.each(TEXTOS)('%s cumple AA (4,5:1) sobre las cuatro superficies', (t) => {
    const fg = rgb(resolver(tema, t))
    for (const s of SUPERFICIES) {
      const k = ratio(fg, rgb(resolver(tema, s)))
      expect(k, `${t} sobre ${s}: ${k.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5)
    }
  })

  it.each(CHIPS)('el chip de %s cumple AA dentro de su propio tinte', (t) => {
    const fg = rgb(resolver(tema, t))
    const tinte = parseFloat(tema['--chip-tinte']!) / 100
    const fondo = mezcla(fg, rgb(resolver(tema, '--surface')), tinte)
    const k = ratio(fg, fondo)
    expect(k, `${t} en su chip: ${k.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5)
  })
})

describe('los dos bloques oscuros', () => {
  it('dicen lo mismo: el elegido y el del sistema no pueden divergir', () => {
    // Van duplicados a propósito (la elección explícita gana en los dos
    // sentidos). Si un día se corrige uno y no el otro, el tema «sistema»
    // vuelve a fallar sin que nadie lo vea.
    expect(oscuroSistema).toEqual(oscuroExplicito)
  })
})
