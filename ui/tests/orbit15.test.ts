/**
 * Orbit 1.5.1, y lo que salió por el camino.
 *
 * Tres hallazgos de la auditoría del contrato que las pruebas de antes no
 * podían ver, porque sus muestras decían otra cosa que el Orbit de verdad:
 *
 *  · `svc_state` escribe `running`, no `active`. Con sólo `active`, toda app con
 *    proceso salía «no se sabe» contra un servidor real.
 *  · `logs --json`, `backup … --json` y `doctor --fix --yes` llegaron en la
 *    1.3.7, no en la línea base.
 *  · `status --json` trae el host y sus servicios, y se tiraban.
 */
import { describe, expect, it } from 'vitest'
import { fireEvent, render, waitFor } from '@testing-library/svelte'

import ServidorVista from '../src/componentes/ServidorVista.svelte'
import ListaApps from '../src/componentes/ListaApps.svelte'
import Tendencia from '../src/componentes/Tendencia.svelte'
import VisorLog from '../src/componentes/VisorLog.svelte'
import { puntuar } from '../src/lib/paleta'
import { duracion, epoca, leerLog, sabe, salud, seQuedaCorto, type App, type Estado, type Portada } from '../src/lib/contrato'
import { leerProgreso, lectorDeProgreso } from '../src/lib/despliegue'
import { portada } from '../src/lib/puente'
import estado from '../src/lib/muestras/status.json'

const E: Estado = {
  service: 'running', port: 3001, ssl: true, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 3, last_deploy: null, last_deploy_sha: null,
}

describe('lo que dice Orbit de una app en marcha', () => {
  it('running es activa', () => {
    expect(salud(E)).toBe('activo')
  })
  it('stopped es parada, y active se sigue entendiendo', () => {
    expect(salud({ ...E, service: 'stopped' })).toBe('parado')
    expect(salud({ ...E, service: 'active' })).toBe('activo')
  })
  it('lo que no se conoce sigue siendo «no se sabe»', () => {
    expect(salud({ ...E, service: 'reloading' })).toBe('desconocido')
  })
})

describe('las capacidades nuevas', () => {
  it('el contrato completo es de la 1.3.7', () => {
    expect(sabe('1.3.6', 'contratoCompleto')).toBe(false)
    expect(sabe('1.3.7', 'contratoCompleto')).toBe(true)
  })
  it('sin versión no se esconde lo que ya se ofrecía', () => {
    // Que falle el saludo no puede quitarle a alguien el log.
    expect(seQuedaCorto(null, 'contratoCompleto')).toBe(false)
    expect(seQuedaCorto('desarrollo', 'contratoCompleto')).toBe(false)
    expect(seQuedaCorto('1.3.6', 'contratoCompleto')).toBe(true)
    expect(seQuedaCorto('1.5.1', 'contratoCompleto')).toBe(false)
  })
})

describe('las épocas y las duraciones de Orbit', () => {
  it('un 0 es «no se sabe», no 1970', () => {
    expect(epoca(0)).toBeNull()
    expect(epoca(null)).toBeNull()
    expect(epoca(1762714000)?.getUTCFullYear()).toBe(2025)
  })
  it('las duraciones se leen', () => {
    expect(duracion(41524)).toBe('11 h 32 min')
    expect(duracion(3 * 86400 + 7200)).toBe('3 d 2 h')
    expect(duracion(null)).toBe('·')
  })
})

describe('la portada trae el host', () => {
  it('sin envoltorio, la muestra lleva host y servicios', async () => {
    const p = await portada('vps-ovh')
    expect(p.host?.hostname).toBeTruthy()
    expect(p.services?.length).toBeGreaterThan(0)
    expect(p.apps.length).toBeGreaterThan(0)
  })
})

describe('la vista del servidor', () => {
  const p = estado as unknown as Portada

  it('pinta el host y los servicios sin pedir nada', () => {
    const r = render(ServidorVista, { servidor: 'vps', host: p.host!, servicios: p.services!, version: '1.5.1' })
    expect(r.container.textContent).toContain(p.host!.hostname)
    expect(r.container.textContent).toContain('nginx')
    expect(r.container.querySelectorAll('[role="meter"]').length).toBe(2)
  })

  it('una carga vacía no se pinta como cero', () => {
    const r = render(ServidorVista, {
      servidor: 'vps', host: { ...p.host!, load: [] }, servicios: p.services!, version: '1.5.1',
    })
    expect(r.container.textContent).toContain('no se ha podido leer')
    expect(r.container.textContent).not.toMatch(/0\.00 · 0\.00/)
  })

  it('las copias de un servidor anterior a la 1.3.7 no se piden: se dice por qué', async () => {
    const r = render(ServidorVista, { servidor: 'vps', host: p.host!, servicios: p.services!, version: '1.3.6' })
    await fireEvent.click(r.getByRole('button', { name: 'copias' }))
    expect(r.container.textContent).toMatch(/1\.3\.7/)
  })

  it('verificar las copias marca cuáles no se podrían restaurar', async () => {
    const r = render(ServidorVista, { servidor: 'vps', host: p.host!, servicios: p.services!, version: '1.5.1' })
    await fireEvent.click(r.getByRole('button', { name: 'copias' }))
    await waitFor(() => expect(r.getByRole('button', { name: /Verificar todas/ })).toBeTruthy())
    await fireEvent.click(r.getByRole('button', { name: /Verificar todas/ }))
    await waitFor(() => expect(r.container.textContent).toMatch(/1 de 2 no se podrían restaurar/))
    expect(r.container.querySelectorAll('.copia--mala').length).toBe(1)
  })

  it('en una redirección de dominio no se enseña el nombre de la app como origen', async () => {
    const r = render(ServidorVista, { servidor: 'vps', host: p.host!, servicios: p.services!, version: '1.5.1' })
    await fireEvent.click(r.getByRole('button', { name: 'redirecciones' }))
    await waitFor(() => expect(r.container.textContent).toContain('todo el dominio'))
  })
})

describe('la paleta', () => {
  it('casa por subsecuencia y prefiere lo contiguo', () => {
    expect(puntuar('tnd', 'tienda')).not.toBeNull()
    expect(puntuar('xyz', 'tienda')).toBeNull()
    expect(puntuar('tie', 'tienda')!).toBeGreaterThan(puntuar('tda', 'tienda')!)
  })
  it('una consulta vacía lo deja todo', () => {
    expect(puntuar('', 'lo que sea')).toBe(0)
  })
})

describe('el filtro de la lista', () => {
  const app = (name: string, domain: string, st: Partial<Estado> = {}): App => ({
    name, type: 'node', domain, aliases: [], state: { ...E, ...st },
  })
  const apps = [app('tienda', 'tienda.es'), app('api', 'api.tienda.es', { service: 'stopped' }), app('blog', 'blog.es')]

  it('filtra por nombre o dominio, y dice cuántas no se ven', async () => {
    const r = render(ListaApps, { apps, servidor: 'vps' })
    const caja = r.getByRole('searchbox')
    await fireEvent.input(caja, { target: { value: 'tienda' } })
    expect(r.container.querySelectorAll('tbody tr').length).toBe(2)
    expect(r.container.textContent).toMatch(/2 de 3/)
  })

  it('el resumen por estado también filtra', async () => {
    const r = render(ListaApps, { apps, servidor: 'vps' })
    await fireEvent.click(r.getByRole('button', { name: /paradas/ }))
    expect(r.container.querySelectorAll('tbody tr').length).toBe(1)
  })

  it('las flechas eligen la siguiente app', async () => {
    let elegida: string | null = null
    const r = render(ListaApps, { apps, servidor: 'vps', alElegir: (a: App) => { elegida = a.name } })
    // Desde la lista: con el foco en otro sitio, la flecha es de ese sitio.
    await fireEvent.keyDown(r.getByRole('searchbox'), { key: 'ArrowDown' })
    expect(elegida).toBe('tienda')
  })
})

describe('la tendencia', () => {
  it('un null corta la línea: no se une lo que nadie ha medido', () => {
    const r = render(Tendencia, { valores: [1, 2, null, 3, 4], etiqueta: 'cpu' })
    expect(r.container.querySelectorAll('path').length).toBe(2)
  })
  it('sin dos medidas no hay línea, pero sí etiqueta', () => {
    const r = render(Tendencia, { valores: [null], etiqueta: 'cpu de web: no se sabe' })
    expect(r.container.querySelectorAll('path').length).toBe(0)
    expect(r.getByRole('img').getAttribute('aria-label')).toContain('no se sabe')
  })
})

describe('el progreso, línea a línea', () => {
  it('da lo mismo que leerlo todo de golpe', () => {
    const lineas = [
      '{"event":"step","step":"code","status":"start","elapsed_s":0}',
      '{"event":"step","step":"code","status":"ok","elapsed_s":2}',
      'texto para una persona',
      '{roto',
      '{"event":"step","step":"build","status":"start","elapsed_s":2}',
      '{"event":"step","step":"build","status":"ok","elapsed_s":40}',
      '{"event":"step","step":"septimo","status":"start","elapsed_s":41}',
    ]
    const l = lectorDeProgreso()
    for (const x of lineas) l.linea(x)
    expect(l.foto()).toEqual(leerProgreso(lineas.join('\n')))
  })
  it('una foto no cambia debajo de quien la tiene', () => {
    const l = lectorDeProgreso()
    l.linea('{"event":"step","step":"code","status":"start","elapsed_s":0}')
    const f = l.foto()
    l.linea('{"event":"step","step":"code","status":"ok","elapsed_s":1}')
    expect(f.pasos[0]!.estado).toBe('haciendo')
  })
})

describe('el visor del log', () => {
  const linea = (text: string, stream = 'access') => JSON.stringify({ event: 'line', ts: null, stream, text })

  it('una línea con una forma que no es la del contrato se cuenta como rota', () => {
    const l = leerLog([linea('bien'), JSON.stringify({ event: 'line', stream: 'otro', text: 'x' }), JSON.stringify({ event: 'line', stream: 'error', text: 7 })].join('\n'))
    expect(l.lineas.length).toBe(1)
    expect(l.rotas).toBe(2)
  })

  it('la búsqueda filtra y resalta sin meter HTML del servidor', async () => {
    const log = leerLog([linea('GET /hola'), linea('<script>alert(1)</script> GET /x'), linea('POST /y')].join('\n'))
    const r = render(VisorLog, { log, app: 'web' })
    await fireEvent.input(r.getByRole('searchbox'), { target: { value: 'get' } })
    // La búsqueda espera 100 ms a que se deje de escribir.
    await new Promise((ok) => setTimeout(ok, 150))
    expect(r.container.querySelectorAll('li.linea').length).toBe(2)
    expect(r.container.querySelectorAll('mark').length).toBe(2)
    expect(r.container.querySelector('script')).toBeNull()
    expect(r.container.textContent).toContain('<script>')
  })

  it('con muchas líneas pinta las últimas y dice cuántas quedan arriba', () => {
    const muchas = Array.from({ length: 1600 }, (_, i) => linea(`l${i}`)).join('\n')
    const r = render(VisorLog, { log: leerLog(muchas), app: 'web' })
    expect(r.container.querySelectorAll('li.linea').length).toBe(1500)
    expect(r.container.textContent).toContain('100 más arriba')
    expect(r.container.textContent).toContain('l1599')
  })
})
