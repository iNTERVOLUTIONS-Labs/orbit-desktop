/**
 * Ronda 2 adversaria (UI): sondas sobre el App entero.
 *
 * Cada `it` afirma el comportamiento CORRECTO: si falla, el fallo descrito está
 * presente. Mocks al estilo de rc-carreras.test.ts.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/svelte'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }))

const ctl = vi.hoisted(() => ({
  instalaciones: [] as { alias: string; resolver: (v: unknown) => void }[],
  retraso: {} as Record<string, number>,
}))
const espera = (ms: number) => new Promise((r) => setTimeout(r, ms))

vi.mock('../src/lib/puente', async (original) => {
  const m = await original<typeof import('../src/lib/puente')>()
  const lista14 = (await import('../src/lib/muestras/list-1.4.json')).default as any
  const info14 = (await import('../src/lib/muestras/info-1.4.json')).default as any
  const status = (await import('../src/lib/muestras/status.json')).default as any
  return {
    ...m,
    hayPuente: () => true,
    servidoresDelConfig: vi.fn(async () => ['alfa', 'bravo'].map((alias) => ({
      alias, hostname: null, usuario: null, puerto: null, salto: null, identidad: null,
    }))),
    servidoresGuardados: vi.fn(async () => []),
    version: vi.fn(async () => ({ schema: 1, version: '1.5.1', contract: 1 })),
    portada: vi.fn(async () => ({ host: status.host, services: status.services, apps: lista14.apps })),
    detalle: vi.fn(async (_alias: string, app: string) => {
      if (ctl.retraso.detalle) await espera(ctl.retraso.detalle)
      const a = lista14.apps.find((x: any) => x.name === app)
      return { ...info14.app, name: app, state: a.state }
    }),
    tareas: vi.fn(async (_alias: string, app: string) => ({ schema: 1, app, tasks: [] })),
    log: vi.fn(async () => ({ meta: null, lineas: [], rotas: 0, truncado: false })),
    saludar: vi.fn(async () => ({
      clase: 'no-instalado', version: null, contrato: null, motivo: 'orbit: command not found',
      puede_operar: false, puede_leer: false, orden_de_instalacion: 'x',
    })),
    requisitosDeInstalacion: vi.fn(async () => ({
      git: true, root: true, sudo_sin_contrasena: true, ya_instalado: false, sistema: 'Ubuntu 24.04',
      puede: true, impedimentos: [], avisos: [], pasos: ['git clone …', 'sudo ./install.sh'],
    })),
    instalarOrbit: vi.fn((alias: string) => new Promise((resolver) => ctl.instalaciones.push({ alias, resolver }))),
    cancelar: vi.fn(async () => true),
  }
})

import App from '../src/App.svelte'
import * as puente from '../src/lib/puente'

afterEach(() => {
  cleanup()
  ctl.instalaciones = []
  ctl.retraso = {}
  vi.mocked(puente.detalle).mockClear()
})

function porTexto(c: ParentNode, sel: string, texto: string) {
  return [...c.querySelectorAll<HTMLButtonElement>(sel)].find((b) => b.textContent?.trim().startsWith(texto))
}

describe('instalar Orbit: la instalación en marcha no se puede perder de vista', () => {
  it('tras «← volver» con la instalación corriendo, se puede volver a ella (ver el progreso, pararla)', async () => {
    const { container } = render(App)
    await espera(40)
    // A la pantalla de servidores, comprobar bravo, instalar.
    await fireEvent.click(container.querySelector<HTMLButtonElement>('.rail button.alta')!)
    await espera(10)
    await fireEvent.click(container.querySelector<HTMLButtonElement>('button[aria-label="Comprobar bravo"]')!)
    await espera(20)
    await fireEvent.click(porTexto(container, 'button', 'Instalar Orbit')!)
    await espera(20)
    await fireEvent.click(porTexto(container, '.instalar button.primario', 'Instalar Orbit')!)
    await espera(20)
    expect(ctl.instalaciones[0]?.alias).toBe('bravo')
    // La vista de instalación esconde su «Cerrar» mientras instala…
    expect(porTexto(container, '.instalar button', 'Cerrar')).toBeUndefined()
    // …pero el botón del rail sigue sacando de ella.
    await fireEvent.click(container.querySelector<HTMLButtonElement>('.rail button.alta')!)
    await espera(20)
    // Y volver: «+ servidores» tiene que llevar otra vez a la instalación en curso.
    await fireEvent.click(container.querySelector<HTMLButtonElement>('.rail button.alta')!)
    await espera(20)
    // Con el fallo: «+ servidores» no hace nada (aLoNormal() da false mientras
    // `instalando`), así que se queda en la lista de apps de alfa.
    expect(container.querySelector('h1')?.textContent?.trim()).toBe('Servidores')
    expect(container.querySelector('.instalar h3')?.textContent ?? '').toContain('Instalar Orbit en bravo')
    expect(porTexto(container, '.instalar button', 'Parar')).toBeDefined()
  })
})

describe('j/k en ráfaga no le quita la pestaña a quien ya ha elegido otra', () => {
  it('el detalle diferido de una ráfaga no devuelve la pestaña a «detalle»', async () => {
    const { container } = render(App)
    await espera(40)
    // Dos pulsaciones de j seguidas: la segunda entra como ráfaga (< 300 ms).
    await fireEvent.keyDown(document.body, { key: 'j' })
    await fireEvent.keyDown(document.body, { key: 'j' })
    // Antes de los 250 ms, a la pestaña «log».
    await espera(30)
    await fireEvent.click(porTexto(container, 'nav.pestanas button', 'log')!)
    await espera(10)
    expect(container.querySelector('nav.pestanas button[aria-current="true"]')?.textContent?.trim()).toBe('log')
    // Pasan los 250 ms del diferido.
    await espera(320)
    expect(container.querySelector('nav.pestanas button[aria-current="true"]')?.textContent?.trim()).toBe('log')
  })
})

describe('el teclado de la lista después de cerrar una vista', () => {
  it('(control) con el foco en <body>, j elige app', async () => {
    const { container } = render(App)
    await espera(40)
    await fireEvent.keyDown(document.body, { key: 'j' })
    await espera(20)
    expect(container.querySelector('.panel--detalle')).not.toBeNull()
  })

  it('tras cerrar «nueva web», j sigue eligiendo app (el foco en el h1 no es «de nadie»)', async () => {
    const { container } = render(App)
    await espera(40)
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'nueva web')!)
    await espera(20)
    await fireEvent.click(porTexto(container, '.asistente button.cerrar', 'Cancelar')!)
    await espera(20)
    // devolverFoco(null) deja el foco en el título de la página.
    expect(document.activeElement?.tagName).toBe('H1')
    await fireEvent.keyDown(document.activeElement!, { key: 'j' })
    await espera(20)
    expect(container.querySelector('.panel--detalle')).not.toBeNull()
  })
})
