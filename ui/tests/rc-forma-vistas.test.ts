/**
 * Las vistas de sólo lectura frente a lo que el servidor puede mandar aunque
 * tenga la forma del contrato: nombres repetidos, caracteres que no se ven,
 * campos que el comando no trae. Y la frontera, para lo que quede.
 */
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { createRawSnippet, flushSync } from 'svelte'

import Frontera from '../src/componentes/Frontera.svelte'
import MonitorVista from '../src/componentes/MonitorVista.svelte'
import ServidorVista from '../src/componentes/ServidorVista.svelte'
import DetalleApp from '../src/componentes/DetalleApp.svelte'
import Diagnostico from '../src/componentes/Diagnostico.svelte'
import LoteVista from '../src/componentes/Lote.svelte'
import TraficoVista from '../src/componentes/TraficoVista.svelte'
import Comparar from '../src/componentes/Comparar.svelte'
import { comparar } from '../src/lib/comparar'
import { generar } from '../src/lib/orbitjson'
import type { App, AppDelMonitor, AppInfo, Doctor, Estado, Host, Lote, Trafico } from '../src/lib/contrato'
import estadoMuestra from '../src/lib/muestras/status.json'
import infoMuestra from '../src/lib/muestras/info.json'

const E: Estado = {
  service: 'running', port: 3001, ssl: false, cert_days: null, maintenance: false, served: true,
  autodeploy: false, queue: false, releases: 1, last_deploy: null, last_deploy_sha: null,
}
const app = (name: string, p: Partial<Estado> = {}, domain = `${name}.test`): App =>
  ({ name, type: 'node', domain, aliases: [], state: { ...E, ...p } })
const fila = (name: string, p: Partial<AppDelMonitor> = {}): AppDelMonitor => ({
  name, type: 'node', domain: `${name}.test`, port: 3001, service: 'running',
  cpu_percent: 1, memory_bytes: 1024, requests_last_minute: 3, requests_capped: false, ...p,
})
const celdas = (c: HTMLElement, fila: number) =>
  [...c.querySelectorAll('tbody tr')[fila]!.querySelectorAll('td')].map((x) => x.textContent?.trim())

describe('la frontera', () => {
  it('lo que revienta al pintar se queda dentro, dice qué era y se puede reintentar', async () => {
    const consola = vi.spyOn(console, 'error').mockImplementation(() => {})
    let romper = true
    const hijo = createRawSnippet(() => ({
      render: () => {
        if (romper) throw new TypeError("Cannot read properties of undefined (reading 'name')")
        return '<p class="bien">ya está</p>'
      },
    }))
    const r = render(Frontera, { que: 'la lista de apps', children: hijo })
    const alerta = r.getByRole('alert')
    expect(alerta.textContent).toContain('No se ha podido pintar la lista de apps')
    expect(alerta.textContent).toContain("reading 'name'")
    expect(consola).toHaveBeenCalled()
    expect(String(consola.mock.calls[0]![0])).toContain('la lista de apps')

    romper = false
    await fireEvent.click(r.getByRole('button', { name: 'Volver a intentarlo' }))
    flushSync()
    expect(r.container.querySelector('.bien')?.textContent).toBe('ya está')
    expect(r.queryByRole('alert')).toBeNull()
    consola.mockRestore()
  })

  it('el reintento llama antes a quien sabe volver a pedir el dato', async () => {
    const consola = vi.spyOn(console, 'error').mockImplementation(() => {})
    const alReintentar = vi.fn()
    const hijo = createRawSnippet(() => ({ render: () => { throw new Error('x') } }))
    const r = render(Frontera, { que: 'el tráfico', alReintentar, children: hijo })
    await fireEvent.click(r.getByRole('button', { name: 'Volver a intentarlo' }))
    expect(alReintentar).toHaveBeenCalledTimes(1)
    consola.mockRestore()
  })

  it('un error del puente que llega hasta aquí se pinta con su propio mensaje', () => {
    const consola = vi.spyOn(console, 'error').mockImplementation(() => {})
    const hijo = createRawSnippet(() => ({
      render: () => { throw { clase: 'forma', mensaje: '«top»: apps[0].name debería ser texto y es un número', detalle: '42' } },
    }))
    const r = render(Frontera, { que: 'el monitor', children: hijo })
    expect(r.getByRole('alert').textContent).toContain('«top»: apps[0].name')
    consola.mockRestore()
  })
})

describe('el monitor', () => {
  it('dos apps con el mismo nombre son dos filas', () => {
    const { container } = render(MonitorVista, {
      monitor: { schema: 1, apps: [fila('tienda'), fila('tienda', { cpu_percent: 50 })] }, servidor: 's', periodo: 3,
    })
    expect(container.querySelectorAll('tbody tr').length).toBe(2)
  })

  it('sin la portada no se inventa el estado: «no se sabe»', () => {
    const { container } = render(MonitorVista, { monitor: { schema: 1, apps: [fila('tienda')] }, servidor: 's', periodo: 3 })
    expect(celdas(container, 0)[1]).toBe('·')
  })

  it('con la portada, el vhost y el mantenimiento salen de ella y el proceso de top', () => {
    const { container } = render(MonitorVista, {
      monitor: { schema: 1, apps: [fila('a'), fila('b'), fila('c', { service: 'stopped' }), fila('d'), fila('fuera')] },
      servidor: 's', periodo: 3,
      portada: [app('a', { served: false }), app('b', { maintenance: true }), app('c', { service: 'running' }), app('d'), app('d')],
    })
    expect(celdas(container, 0)[1]).toBe('sin vhost')
    expect(celdas(container, 1)[1]).toBe('mantenimiento')
    // top dice parada aunque la portada, más vieja, la diera en marcha.
    expect(celdas(container, 2)[1]).toBe('parado')
    // Dos «d» en la portada: no se sabe cuál es.
    expect(celdas(container, 3)[1]).toBe('·')
    expect(celdas(container, 4)[1]).toBe('·')
  })

  it('un nombre con un carácter de ancho cero se ve', () => {
    const { container } = render(MonitorVista, { monitor: { schema: 1, apps: [fila('tien​da')] }, servidor: 's', periodo: 3 })
    expect(celdas(container, 0)[0]).toBe('tien‹U+200B›da')
  })
})

describe('la vista del servidor', () => {
  const host = (estadoMuestra as unknown as { host: Host }).host

  it('servicios y vigilados repetidos se pintan todos', async () => {
    const { container } = render(ServidorVista, {
      servidor: 'pruebas', host, servicios: [{ name: 'nginx', active: true }, { name: 'nginx', active: false }],
    })
    expect(container.querySelectorAll('.servicios li').length).toBe(2)
  })

  it('unas cargas que no son tres números son «·», no una excepción', () => {
    const { container } = render(ServidorVista, {
      servidor: 'pruebas', host: { ...host, load: [null, null, null] } as never, servicios: [],
    })
    expect(container.querySelector('.medida .valor')?.textContent?.trim()).toBe('·')
  })

  it('con la versión sin saber, las copias se piden y se pintan', async () => {
    const r = render(ServidorVista, { servidor: 'pruebas', host, servicios: [], version: null })
    await fireEvent.click(r.getByRole('button', { name: 'copias' }))
    await waitFor(() => expect(r.container.textContent).toContain('se guardan'))
  })

  it('con un servidor que sabemos viejo se dice, y si la versión llega después se piden', async () => {
    const r = render(ServidorVista, { servidor: 'pruebas', host, servicios: [], version: '1.3.6' })
    await fireEvent.click(r.getByRole('button', { name: 'copias' }))
    expect(r.container.textContent).toContain('llegó en la 1.3.7')
    await r.rerender({ servidor: 'pruebas', host, servicios: [], version: '1.5.1' })
    await waitFor(() => expect(r.container.textContent).toContain('se guardan'))
  })
})

describe('el detalle, el diagnóstico, el lote y el tráfico con valores repetidos', () => {
  it('dos tareas que se llaman igual son dos filas, y el nombre de la app enseña lo invisible', () => {
    const { container } = render(DetalleApp, {
      app: app('tien‮da'), servidor: 's',
      tareas: { schema: 1, app: 't', tasks: [
        { name: 'limpiar', last_run: null, next_run: null, result: null },
        { name: 'limpiar', last_run: null, next_run: null, result: 'success' },
      ] },
    })
    expect(container.querySelectorAll('tbody tr').length).toBe(2)
    expect(container.querySelector('.ruta')?.textContent).toContain('tien‹U+202E›da')
  })

  it('dos comprobaciones con el mismo id', () => {
    const d: Doctor = {
      schema: 1, summary: { ok: 1, warn: 1, error: 0 },
      checks: [
        { id: 'disk', level: 'ok', message: 'uno', fix: null, fixable: false },
        { id: 'disk', level: 'warn', message: 'dos', fix: null, fixable: false },
      ],
    }
    const { container } = render(Diagnostico, { doctor: d, servidor: 's' })
    expect(container.querySelectorAll('li.fila').length).toBe(2)
  })

  it('dos entradas del lote con el mismo nombre', () => {
    const l: Lote = {
      schema: 1, total: 2, deployed: 0, failed: 0, unchanged: 2, unreachable: 0, gone: 0, skipped: 0,
      ok: true, duration_s: 1,
      apps: [{ app: 'tienda', status: 'unchanged', result: null, error: null }, { app: 'tienda', status: 'unchanged', result: null, error: null }],
    }
    const { container } = render(LoteVista, { lote: l, servidor: 's' })
    expect(container.querySelectorAll('tbody tr').length).toBe(2)
  })

  it('dos rutas iguales en el tráfico', () => {
    const t: Trafico = {
      app: 't', since: '7d', from: '20261101000000', complete: true, requests: 2, ips: 1, ips_capped: false,
      bytes: 1, automated: 0, status: {}, latency_ms: { p50: null, p95: null, max: null, lines: 0 },
      paths: [{ path: '/a', requests: 1 }, { path: '/a', requests: 1 }],
    }
    const { container } = render(TraficoVista, { trafico: t, servidor: 's' })
    expect(container.querySelectorAll('.rutas li').length).toBe(2)
  })
})

describe('comparar', () => {
  it('un nombre repetido en un servidor se dice, en vez de perder una de las dos', () => {
    const c = comparar([app('tienda'), app('tienda', {}, 'otra.test')], [app('tienda')])
    expect(c.repetidasA).toEqual(['tienda'])
    expect(c.repetidasB).toEqual([])
    const r = render(Comparar, {
      a: 'uno', b: 'dos', appsA: [app('tienda'), app('tienda')], appsB: [app('tienda')],
      candidatos: [], alElegir: () => {}, alCerrar: () => {},
    })
    expect(r.container.textContent).toContain('más de una app con el mismo nombre')
  })

  it('el dominio que difiere enseña lo invisible; «sí» y «no» no se tocan', () => {
    const r = render(Comparar, {
      a: 'uno', b: 'dos',
      appsA: [app('tienda', { autodeploy: true }, 'tienda.test')],
      appsB: [app('tienda', { autodeploy: false }, 'tiendа.test')],
      candidatos: [], alElegir: () => {}, alCerrar: () => {},
    })
    const texto = r.container.textContent ?? ''
    expect(texto).toContain('tiend‹U+0430›.test')
    expect(texto).toContain('sí')
  })
})

describe('el orbit.json con un nombre de variable que es del prototipo', () => {
  it('«__proto__» se escribe como variable, no desaparece', () => {
    const info = (infoMuestra as { app: AppInfo }).app
    const g = generar(info, ['__proto__', 'DB_URL'])
    const vars = JSON.parse(g.texto).env.vars
    expect(Object.keys(vars)).toEqual(['__proto__', 'DB_URL'])
  })
})
