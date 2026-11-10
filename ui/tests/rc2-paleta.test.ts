/**
 * Ronda 2 adversaria (UI): claves de `{#each}` que salen de nombres ajenos.
 *
 * La paleta usa `a.id` como clave del `{#each}` y como `id` del DOM. Los ids
 * de las acciones de la app elegida (`app-log`, `app-entorno`, `app-trafico`,
 * `app-revertir`, `app-desplegar`, `app-oj`) comparten espacio con los de cada
 * app (`app-${nombre}`): una app que se llame `log` choca. Y el rail usa el
 * alias como clave, y junta dos fuentes que nadie deduplica entre sí.
 *
 * Fuera de ninguna `Frontera` (la paleta y el rail viven fuera del panel).
 * Cada `it` afirma lo correcto; falla con el fallo presente.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/svelte'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }))

const ctl = vi.hoisted(() => ({
  config: ['alfa'] as string[],
  propios: [] as { alias: string; host: string; usuario: string; puerto: number; clave: null; binario: null }[],
  extra: [] as string[],
}))
const espera = (ms: number) => new Promise((r) => setTimeout(r, ms))

vi.mock('../src/lib/puente', async (original) => {
  const m = await original<typeof import('../src/lib/puente')>()
  const lista14 = (await import('../src/lib/muestras/list-1.4.json')).default as any
  const info14 = (await import('../src/lib/muestras/info-1.4.json')).default as any
  const status = (await import('../src/lib/muestras/status.json')).default as any
  const apps = () => [
    ...lista14.apps,
    ...ctl.extra.map((name) => ({ ...lista14.apps[1], name, domain: `${name}.ejemplo.com` })),
  ]
  return {
    ...m,
    hayPuente: () => true,
    servidoresDelConfig: vi.fn(async () => ctl.config.map((alias) => ({
      alias, hostname: null, usuario: null, puerto: null, salto: null, identidad: null,
    }))),
    servidoresGuardados: vi.fn(async () => ctl.propios),
    version: vi.fn(async () => ({ schema: 1, version: '1.5.1', contract: 1 })),
    portada: vi.fn(async () => ({ host: status.host, services: status.services, apps: apps() })),
    detalle: vi.fn(async (_a: string, app: string) => ({ ...info14.app, name: app, state: apps().find((x: any) => x.name === app).state })),
    tareas: vi.fn(async (_a: string, app: string) => ({ schema: 1, app, tasks: [] })),
  }
})

import App from '../src/App.svelte'

afterEach(() => {
  cleanup()
  ctl.config = ['alfa']
  ctl.propios = []
  ctl.extra = []
})

async function elegir(c: HTMLElement, app: string) {
  const b = [...c.querySelectorAll<HTMLButtonElement>('td.celda-nombre button')].find((x) => x.textContent?.trim() === app)!
  await fireEvent.click(b)
  await espera(20)
}

describe('la paleta con una app que se llama como una acción', () => {
  it('una app llamada «log» y otra elegida: la paleta abre y tiene las dos entradas, con ids únicos', async () => {
    ctl.extra = ['log']
    const errores: unknown[] = []
    const alErr = (e: ErrorEvent) => errores.push(e.error ?? e.message)
    window.addEventListener('error', alErr)
    try {
      const { container } = render(App)
      await espera(40)
      await elegir(container, 'tienda')
      try {
        await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
      } catch (e) {
        errores.push(e)
      }
      await espera(30)
      expect(errores.map(String).join('\n')).not.toMatch(/each_key_duplicate/)
      const opciones = [...document.querySelectorAll('.paleta li[role="option"]')]
      const textos = opciones.map((o) => o.querySelector('.texto')?.textContent)
      // En producción (sin validate_each_keys) no revienta: se pierde una de las
      // dos entradas en silencio. Ver el informe.
      expect(textos, JSON.stringify(textos)).toContain('Log de tienda')
      expect(textos).toContain('log')
      const ids = opciones.map((o) => o.id)
      expect(new Set(ids).size).toBe(ids.length)
    } finally {
      window.removeEventListener('error', alErr)
    }
  })
})

describe('el rail con el mismo alias en las dos fuentes', () => {
  it('un servidor añadido a mano que luego aparece también en ~/.ssh/config no rompe el rail', async () => {
    // Se añadió «vps» a mano; después alguien puso `Host vps` en su config.
    ctl.config = ['vps']
    ctl.propios = [{ alias: 'vps', host: '203.0.113.7', usuario: 'root', puerto: 22, clave: null, binario: null }]
    const errores: unknown[] = []
    const alErr = (e: ErrorEvent) => errores.push(e.error ?? e.message)
    window.addEventListener('error', alErr)
    try {
      let container: HTMLElement | null = null
      try {
        container = render(App).container
      } catch (e) {
        errores.push(e)
      }
      await espera(60)
      expect(errores.map(String).join('\n')).not.toMatch(/each_key_duplicate/)
      // Un solo botón por alias: dos con el mismo nombre no se distinguen, y
      // `ssh` resolvería uno de los dos según quién gane.
      const botones = [...(container ?? document.body).querySelectorAll('.rail button.servidor')]
      expect(botones.filter((b) => b.textContent?.trim().startsWith('vps')).length).toBe(1)
    } finally {
      window.removeEventListener('error', alErr)
    }
  })
})
