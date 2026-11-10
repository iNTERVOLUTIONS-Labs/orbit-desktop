/**
 * Segunda ronda adversarial: lo que se mide en Chrome y jsdom no ve —reflujo a
 * 320 px (WCAG 1.4.10) y contraste por `opacity` (1.4.3)—, fijado leyendo el
 * CSS de cada componente. Fallan mientras el fallo exista.
 *
 * Medido (adv2-ux/ancho2.mjs, 320×640): la página se ensancha a 360 px en el
 * detalle de una app, a 365 en el log y a 357 en revertir.
 */
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

const css = (f: string) => {
  const t = readFileSync(join(import.meta.dirname, '..', 'src', 'componentes', f), 'utf8')
  return t.slice(t.indexOf('<style'))
}

describe('reflujo a 320 px', () => {
  // Una columna fija de 160–180 px más una de contenido no cabe en los ~280 px
  // que quedan a 320 px de ancho: hace falta una salida estrecha.
  it.each([
    ['DetalleApp.svelte', /grid-template-columns:\s*160px 1fr/],
    ['Revertir.svelte', /grid-template-columns:\s*180px 1fr auto/],
  ])('%s tiene una salida para pantallas estrechas', (f, fija) => {
    const c = css(f)
    expect(fija.test(c)).toBe(true) // la columna fija sigue ahí…
    expect(c).toMatch(/@media\s*\((max-width|width\s*<)|@container/) // …y algo la deshace
  })
})

describe('contraste: nada informativo se atenúa con opacity', () => {
  it('Lote: un recuento a cero («0 sin contacto») se lee igual de bien que los demás', () => {
    // Medido: 1,98:1 a 2,86:1 en claro y oscuro (`.cuenta--cero { opacity: .5 }`).
    // El propio componente dice que un cero es una respuesta, no un hueco.
    expect(css('Lote.svelte')).not.toMatch(/\.cuenta--cero\s*\{[^}]*opacity/)
  })
})
