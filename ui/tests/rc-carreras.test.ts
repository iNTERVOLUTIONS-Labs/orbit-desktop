/**
 * Regresiones de la ronda adversa «el sysadmin con prisa».
 *
 * Dos servidores, `alfa` y `bravo`, con apps que se llaman igual. Cada llamada
 * se puede retrasar por servidor, para que el orden de las respuestas lo elija
 * la prueba y no la red.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/svelte'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }))

const ctl = vi.hoisted(() => ({
  retraso: {} as Record<string, Record<string, number>>,
  despliegues: [] as { alias: string; app: string; forzar: boolean; resolver: (v: unknown) => void }[],
  pasadas: [] as { alias: string; resolver: (v: unknown) => void }[],
  servidores: ['alfa', 'bravo'],
  fallar: {} as Record<string, number>,
}))

const espera = (ms: number) => new Promise((r) => setTimeout(r, ms))

vi.mock('../src/lib/puente', async (original) => {
  const m = await original<typeof import('../src/lib/puente')>()
  const lista14 = (await import('../src/lib/muestras/list-1.4.json')).default as any
  const info14 = (await import('../src/lib/muestras/info-1.4.json')).default as any
  const status = (await import('../src/lib/muestras/status.json')).default as any
  const apps: Record<string, any[]> = {
    alfa: lista14.apps,
    bravo: lista14.apps.map((a: any) => ({ ...a, state: { ...a.state, locked: false, lock: null } })),
    charlie: ((await import('../src/lib/muestras/list.json')).default as any).apps,
  }
  const tarde = async (alias: string, que: string) => {
    const ms = ctl.retraso[alias]?.[que] ?? 0
    if (ms) await espera(ms)
  }
  return {
    ...m,
    hayPuente: () => true,
    servidoresDelConfig: vi.fn(async () => ctl.servidores.map((alias) => ({
      alias, hostname: null, usuario: null, puerto: null, salto: null, identidad: null,
    }))),
    servidoresGuardados: vi.fn(async () => []),
    version: vi.fn(async () => ({ schema: 1, version: '1.5.1', contract: 1 })),
    portada: vi.fn(async (alias: string) => {
      await tarde(alias, 'portada')
      if ((ctl.fallar[alias] ?? 0) > 0) { ctl.fallar[alias]!--; throw { clase: 'no-llego', mensaje: 'Connection reset' } }
      return { host: status.host, services: status.services, apps: apps[alias] }
    }),
    detalle: vi.fn(async (alias: string, app: string) => {
      await tarde(alias, 'detalle')
      const a = apps[alias]!.find((x) => x.name === app)
      return { ...info14.app, name: app, state: a.state }
    }),
    tareas: vi.fn(async (_alias: string, app: string) => ({ schema: 1, app, tasks: [] })),
    comprobarEntorno: vi.fn(async (alias: string, app: string) => {
      await tarde(alias, 'env-check')
      return { schema: 1, app, declared: true, vars: [] }
    }),
    congelar: vi.fn(async () => ({ salida: 'congelada', codigo: 0 })),
    cancelar: vi.fn(async () => true),
    desplegar: vi.fn((alias: string, app: string, o: { forzar?: boolean } = {}) =>
      new Promise((resolver) => ctl.despliegues.push({ alias, app, forzar: !!o.forzar, resolver }))),
    desplegarTodo: vi.fn((alias: string) =>
      new Promise((resolver) => ctl.pasadas.push({ alias, resolver }))),
    diagnostico: vi.fn(async (alias: string) => {
      await tarde(alias, 'doctor')
      return (await import('../src/lib/muestras/doctor.json')).default as any
    }),
    monitor: vi.fn(async (alias: string) => {
      await tarde(alias, 'monitor')
      const top = (await import('../src/lib/muestras/top.json')).default as any
      return top
    }),
  }
})

import App from '../src/App.svelte'
import * as puente from '../src/lib/puente'
import desplegadoOk from '../src/lib/muestras/deploy-ok.json'

afterEach(() => {
  cleanup()
  ctl.retraso = {}
  ctl.despliegues = []
  ctl.pasadas = []
  ctl.servidores = ['alfa', 'bravo']
  ctl.fallar = {}
  vi.mocked(puente.congelar).mockClear()
  vi.mocked(puente.cancelar).mockClear()
  vi.mocked(puente.desplegar).mockClear()
  vi.mocked(puente.monitor).mockClear()
  vi.mocked(puente.portada).mockClear()
})

function rail(c: HTMLElement, alias: string) {
  return [...c.querySelectorAll<HTMLButtonElement>('.rail button.servidor')].find((b) => b.textContent?.trim().startsWith(alias))!
}
function porTexto(c: ParentNode, sel: string, texto: string) {
  return [...c.querySelectorAll<HTMLButtonElement>(sel)].find((b) => b.textContent?.trim().startsWith(texto))
}
async function elegir(c: HTMLElement, app: string) {
  const b = [...c.querySelectorAll<HTMLButtonElement>('td.celda-nombre button')].find((x) => x.textContent?.trim() === app)!
  await fireEvent.click(b)
  await espera(20)
}

describe('una hoja de confirmación no cambia de servidor por debajo', () => {
  it('con la hoja abierta, Ctrl+K no abre la paleta y la orden corre donde dice', async () => {
    const { container } = render(App)
    await espera(40)
    await elegir(container, 'api')
    await fireEvent.click(porTexto(container, 'button', 'Congelar despliegues')!)
    await fireEvent.click(porTexto(container, 'button', 'Congelar…')!)
    await espera(10)
    expect(document.querySelector('#hoja-titulo')?.textContent).toContain('en alfa')
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await espera(10)
    expect(document.querySelector('.paleta')).toBeNull()
    expect(container.querySelector('h1')?.textContent).toBe('alfa')
    await fireEvent.click(porTexto(document, '.hoja button.confirmar', 'Congelar')!)
    await espera(30)
    expect(puente.congelar).toHaveBeenCalledWith('alfa', 'api', null)
  })

  it('la hoja del despliegue lleva el servidor con el que se abrió', async () => {
    ctl.retraso = { alfa: { 'env-check': 60 } }
    const { container } = render(App)
    await espera(40)
    await elegir(container, 'blog')
    await fireEvent.click(container.querySelector<HTMLButtonElement>('button.lanzar')!)
    await espera(5)
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await espera(120)
    expect(document.querySelector('.paleta')).toBeNull()
    expect(document.querySelector('#hoja-despliegue-titulo')?.textContent).toContain('blog en alfa')
    // La comprobación del entorno llega y se usa: no se queda «mirando».
    expect(document.querySelector('.hoja')?.textContent).not.toContain('Mirando si le falta alguna variable')
    await fireEvent.click(document.querySelector<HTMLButtonElement>('.hoja button.confirmar')!)
    await espera(10)
    expect(ctl.despliegues[0]).toMatchObject({ alias: 'alfa', app: 'blog' })
  })
})

describe('Ctrl+R mientras carga la portada', () => {
  it('la lista sale cuando llega, sin esqueleto para siempre', async () => {
    const { container } = render(App)
    await espera(40)
    ctl.retraso = { bravo: { portada: 60 } }
    await fireEvent.click(rail(container, 'bravo'))
    await fireEvent.keyDown(window, { key: 'r', ctrlKey: true })
    await espera(400)
    expect(container.querySelector('h1')?.textContent).toBe('bravo')
    expect(container.querySelector('.esqueleto')).toBeNull()
    expect(container.querySelector('table.tabla')).not.toBeNull()
  })
})

describe('una operación del servidor entero', () => {
  it('con una pasada en marcha, el rail no cambia de servidor y «Parar» va al suyo', async () => {
    const { container } = render(App)
    await espera(40)
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'desplegar todo')!)
    await espera(10)
    await fireEvent.click(porTexto(container, 'button.primaria', 'Desplegar lo que haya cambiado')!)
    await espera(10)
    expect(ctl.pasadas[0]?.alias).toBe('alfa')
    expect(rail(container, 'bravo').getAttribute('aria-disabled')).toBe('true')
    await fireEvent.click(rail(container, 'bravo'))
    await espera(30)
    expect(container.querySelector('.pasada h3')?.textContent).toBe('Desplegar todo en alfa')
    await fireEvent.click(porTexto(container, 'button', 'Parar la pasada')!)
    expect(puente.cancelar).toHaveBeenCalledWith('alfa', '*')
  })
})

describe('cada vista espera lo suyo', () => {
  it('el monitor de bravo pregunta a bravo aunque el de alfa siga en vuelo', async () => {
    const { container } = render(App)
    await espera(40)
    ctl.retraso = { alfa: { monitor: 200 } }
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'monitor')!)
    await espera(10)
    await fireEvent.click(rail(container, 'bravo'))
    await espera(30)
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'monitor')!)
    await espera(20)
    // En la pestaña del monitor sale el monitor, no la lista de apps.
    expect(container.querySelector('.panel .periodo')).not.toBeNull()
    expect(container.querySelector('td.celda-nombre button')).toBeNull()
    expect(vi.mocked(puente.monitor).mock.calls.filter(([a]) => a === 'bravo').length).toBeGreaterThan(0)
  })

  it('el diagnóstico recién cambiado de servidor no enseña la lista', async () => {
    const { container } = render(App)
    await espera(40)
    ctl.retraso = { bravo: { portada: 30, doctor: 200 } }
    await fireEvent.click(rail(container, 'bravo'))
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'diagnóstico')!)
    await espera(80)
    expect(container.querySelector('nav.vistas button[aria-current="true"]')?.textContent?.trim()).toBe('diagnóstico')
    expect(container.querySelector('table.tabla')).toBeNull()
    expect(container.querySelector('.esqueleto')).not.toBeNull()
  })
})

describe('comparar', () => {
  it('la respuesta tardía del servidor abandonado no se pinta como la del elegido', async () => {
    ctl.servidores = ['alfa', 'bravo', 'charlie']
    const { container } = render(App)
    await espera(40)
    ctl.retraso = { charlie: { portada: 150 } }
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'comparar')!)
    await fireEvent.click(porTexto(container, '.comparar .elegir button', 'charlie')!)
    await fireEvent.click(porTexto(container, '.comparar button.cerrar', 'Cerrar')!)
    await fireEvent.click(porTexto(container, 'nav.vistas button', 'comparar')!)
    await fireEvent.click(porTexto(container, '.comparar .elegir button', 'bravo')!)
    await espera(250)
    expect(container.querySelector('.comparar h3')?.textContent).toBe('Comparar alfa con bravo')
    expect(container.querySelector('.comparar')?.textContent ?? '').not.toContain('app001')
  })
})

describe('un refresco que falla', () => {
  it('el siguiente que funciona quita la pantalla de fallo', async () => {
    const { container } = render(App)
    await espera(40)
    ctl.fallar = { alfa: 1 }
    await fireEvent.keyDown(window, { key: 'r', ctrlKey: true })
    await espera(30)
    expect(container.querySelector('table.tabla')).toBeNull()
    await fireEvent.keyDown(window, { key: 'r', ctrlKey: true })
    await espera(30)
    expect(container.querySelector('table.tabla')).not.toBeNull()
  })
})

describe('mantener pulsada j', () => {
  it('una ráfaga pide el detalle de la primera y de la última, no de cada fila', async () => {
    const { container } = render(App)
    await espera(40)
    ctl.retraso = { alfa: { detalle: 100 } }
    vi.mocked(puente.detalle).mockClear()
    vi.mocked(puente.tareas).mockClear()
    for (let i = 0; i < 4; i++) await fireEvent.keyDown(window, { key: 'j' })
    for (let i = 0; i < 4; i++) await fireEvent.keyDown(window, { key: 'k' })
    await espera(400)
    expect(vi.mocked(puente.detalle).mock.calls.length).toBeLessThanOrEqual(2)
    void container
  })
})
