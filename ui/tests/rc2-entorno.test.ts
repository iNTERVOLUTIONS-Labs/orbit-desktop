/**
 * Ronda 2 adversaria (UI): revelar un valor del entorno «por posición».
 *
 * La ronda 1 pasó `revelada` de la clave a la POSICIÓN (por las claves
 * repetidas). Pero la posición no es estable entre dos `env list`: al recargar
 * la pestaña (↻) con un `.env` que ha cambiado, el valor revelado se queda en
 * la fila N, que ahora es OTRA clave. Esta sonda afirma lo correcto —el valor
 * nunca aparece junto a una clave que no es la suya— y falla con el fallo.
 */
import { afterEach, describe, expect, it } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/svelte'
import Entorno from '../src/componentes/Entorno.svelte'

afterEach(() => cleanup())
const espera = (ms: number) => new Promise((r) => setTimeout(r, ms))

function filaQueEnsena(c: HTMLElement, valor: string): string | null {
  for (const li of c.querySelectorAll('li.fila')) {
    if (li.querySelector('code.valor')?.textContent === valor) return li.querySelector('code.clave')?.firstChild?.textContent ?? null
  }
  return null
}

describe('Entorno: un valor revelado no cambia de dueño al recargar', () => {
  it('recargar con una clave nueva delante no pone el secreto de B junto a A', async () => {
    const valores: Record<string, string> = { A: 'valor-de-A', B: 'secreto-de-B', NUEVA: 'x' }
    const props = {
      entorno: { schema: 1, app: 'tienda', keys: ['A', 'B'] },
      app: 'tienda', servidor: 'alfa',
      pedirValor: async (k: string) => valores[k]!,
    }
    const { container, rerender } = render(Entorno, props)
    await fireEvent.click(container.querySelector<HTMLButtonElement>('button[aria-label="Revelar el valor de B en alfa"]')!)
    await espera(10)
    expect(filaQueEnsena(container, 'secreto-de-B')).toBe('B')
    // ↻: llega otro `env list` (alguien añadió NUEVA arriba desde un terminal).
    await rerender({ ...props, entorno: { schema: 1, app: 'tienda', keys: ['NUEVA', 'A', 'B'] } })
    await espera(10)
    const dueno = filaQueEnsena(container, 'secreto-de-B')
    // Lo correcto: o sigue junto a B, o se ha ocultado. Nunca junto a A.
    expect(dueno === null || dueno === 'B').toBe(true)
  })

  it('una revelación en vuelo que llega después de recargar tampoco cae en otra fila', async () => {
    let soltar: (v: string) => void = () => {}
    const props = {
      entorno: { schema: 1, app: 'tienda', keys: ['A', 'B'] },
      app: 'tienda', servidor: 'alfa',
      pedirValor: (_k: string) => new Promise<string>((r) => { soltar = r }),
    }
    const { container, rerender } = render(Entorno, props)
    await fireEvent.click(container.querySelector<HTMLButtonElement>('button[aria-label="Revelar el valor de B en alfa"]')!)
    await rerender({ ...props, entorno: { schema: 1, app: 'tienda', keys: ['X', 'A'] } })
    soltar('secreto-de-B')
    await espera(10)
    const dueno = filaQueEnsena(container, 'secreto-de-B')
    expect(dueno === null || dueno === 'B').toBe(true)
  })
})
