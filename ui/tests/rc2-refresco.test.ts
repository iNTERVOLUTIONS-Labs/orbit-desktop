/**
 * Ronda 2 adversaria (UI): un `refrescar()` ajeno desmonta lo que se está
 * confirmando.
 *
 * `refrescar()` (que lanza, entre otros, el final de CUALQUIER despliegue del
 * servidor) pone `detalleApp = null` y lo vuelve a pedir. Las pestañas
 * «retirar» y «revertir» se pintan con `{#if detalleApp}`, así que se
 * desmontan y con ellas su hoja de confirmación, a medio escribir.
 *
 * Afirma lo correcto: la confirmación abierta sigue ahí. Falla con el fallo.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/svelte'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }))

const ctl = vi.hoisted(() => ({
  despliegues: [] as { app: string; resolver: (v: unknown) => void }[],
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
    servidoresDelConfig: vi.fn(async () => [{ alias: 'alfa', hostname: null, usuario: null, puerto: null, salto: null, identidad: null }]),
    servidoresGuardados: vi.fn(async () => []),
    version: vi.fn(async () => ({ schema: 1, version: '1.5.1', contract: 1 })),
    portada: vi.fn(async () => ({ host: status.host, services: status.services, apps: lista14.apps })),
    detalle: vi.fn(async (_a: string, app: string) => {
      await espera(30)
      return { ...info14.app, name: app, state: lista14.apps.find((x: any) => x.name === app).state }
    }),
    tareas: vi.fn(async (_a: string, app: string) => ({ schema: 1, app, tasks: [] })),
    entorno: vi.fn(async (_a: string, app: string) => ({ schema: 1, app, keys: ['A'] })),
    comprobarEntorno: vi.fn(async (_a: string, app: string) => ({ schema: 1, app, file: '.env', ok: true, vars: [] })),
    desplegar: vi.fn((_alias: string, app: string) => new Promise((resolver) => ctl.despliegues.push({ app, resolver }))),
  }
})

import App from '../src/App.svelte'
import desplegadoOk from '../src/lib/muestras/deploy-ok.json'

afterEach(() => {
  cleanup()
  ctl.despliegues = []
})

function porTexto(c: ParentNode, sel: string, texto: string) {
  return [...c.querySelectorAll<HTMLButtonElement>(sel)].find((b) => b.textContent?.trim().startsWith(texto))
}
async function elegir(c: HTMLElement, app: string) {
  const b = [...c.querySelectorAll<HTMLButtonElement>('td.celda-nombre button')].find((x) => x.textContent?.trim() === app)!
  await fireEvent.click(b)
  await espera(60)
}

describe('el final de otro despliegue no cierra una confirmación de borrado', () => {
  it('con «Retirar y borrar» a medio escribir, terminar el despliegue de otra app no la desmonta', async () => {
    const { container } = render(App)
    await espera(40)
    // Despliegue de «api», que se queda en marcha.
    await elegir(container, 'api')
    await fireEvent.click(container.querySelector<HTMLButtonElement>('button.lanzar')!)
    await espera(60)
    await fireEvent.click(document.querySelector<HTMLButtonElement>('.hoja button.confirmar')!)
    await espera(10)
    expect(ctl.despliegues[0]?.app).toBe('api')
    // Mientras, en «blog»: retirar y borrar, escribiendo el nombre.
    await elegir(container, 'blog')
    await fireEvent.click(porTexto(container, 'nav.pestanas button', 'retirar')!)
    await espera(80)
    await fireEvent.click(porTexto(container, 'button', 'Retirar y borrar los datos')!)
    await espera(10)
    const caja = document.querySelector<HTMLInputElement>('.escribir input')!
    expect(caja).not.toBeNull()
    await fireEvent.input(caja, { target: { value: 'bl' } })
    // Termina el despliegue de api: `lanzar` llama a refrescar().
    ctl.despliegues[0]!.resolver({ ...desplegadoOk, app: 'api' })
    await espera(5)
    // Lo correcto: la hoja sigue abierta y con lo escrito.
    expect(document.querySelector<HTMLInputElement>('.escribir input')?.value).toBe('bl')
    await espera(100)
    expect(document.querySelector<HTMLInputElement>('.escribir input')?.value).toBe('bl')
  })
})
