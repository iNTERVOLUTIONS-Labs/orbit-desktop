/**
 * Respuestas que llegan tarde, y a quién se le pintan.
 *
 * Con un servidor lento y otro rápido, la portada del lento llega DESPUÉS de la
 * del rápido. La primera versión la pintaba igual, bajo el nombre del servidor
 * que se estaba mirando: las apps de uno con la cabecera del otro, que es el
 * accidente más caro de un cliente multiservidor, provocado por la red y no por
 * nadie. Contra el servidor falso no se veía, porque contesta enseguida; aquí se
 * le pone a uno de los dos un retraso de verdad.
 */
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'

vi.mock('../src/lib/puente', async (original) => {
  const m = await original<typeof import('../src/lib/puente')>()
  return {
    ...m,
    portada: vi.fn(async (alias: string) => {
      if (alias === 'pruebas') await new Promise((r) => setTimeout(r, 80))
      return m.portada(alias)
    }),
    // Un servidor añadido a mano, que sin puente no existe.
    servidoresGuardados: vi.fn(async () => [
      { alias: 'a-mano', host: '203.0.113.7', usuario: 'deploy', puerto: 22, clave: null, binario: null },
    ]),
  }
})

import App from '../src/App.svelte'

const espera = (ms: number) => new Promise((r) => setTimeout(r, ms))

function boton(c: HTMLElement, texto: string): HTMLButtonElement {
  return [...c.querySelectorAll<HTMLButtonElement>('.rail button.servidor')]
    .find((b) => b.textContent?.trim().startsWith(texto))!
}

describe('cambiar de servidor a mitad de una carga', () => {
  it('la portada que llega tarde no se pinta bajo el nombre del otro', async () => {
    const { container } = render(App)
    await espera(30)

    // El lento primero, el rápido justo después.
    await fireEvent.click(boton(container, 'pruebas'))
    await fireEvent.click(boton(container, 'vps-ovh'))
    await espera(150)

    expect(container.querySelector('h1')?.textContent).toBe('vps-ovh')
    const t = container.textContent ?? ''
    // Las de vps-ovh, y ninguna de las de «pruebas».
    expect(t).toContain('app001')
    expect(t).not.toContain('mantenimiento')
    expect(container.querySelector('.chip--sin-vhost')?.closest('tr')?.textContent ?? '').not.toContain('sin-vhost')
  })

  it('los servidores añadidos a mano también salen en el rail', async () => {
    // Se podían usar desde el alta, pero el rail sólo pintaba los del
    // ~/.ssh/config: no había forma de volver a ellos.
    const { container } = render(App)
    await espera(30)
    const rail = [...container.querySelectorAll('.rail button.servidor')].map((b) => b.textContent?.trim())
    expect(rail).toContain('a-mano')
    expect(rail).toContain('vps-ovh')
  })
})
