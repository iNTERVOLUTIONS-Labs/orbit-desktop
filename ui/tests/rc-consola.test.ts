/**
 * Las consolas en vivo, el registro de despliegues y el historial del monitor:
 * lo que la ronda adversaria encontró y cómo se queda arreglado.
 */
import { describe, expect, it } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'

import Consola from '../src/componentes/Consola.svelte'
import Despliegue from '../src/componentes/Despliegue.svelte'
import Pasada from '../src/componentes/Pasada.svelte'
import VisorLog from '../src/componentes/VisorLog.svelte'
import { Salida, salidaDeTexto, TOPE_DE_LINEA } from '../src/lib/consola'
import {
  argvDeCongelar, argvDeDescongelar, argvDeEsperarCi, chocaConPalabraDeOrbit,
  finalDe, lectorDePasada, leerPasada, leerProgreso, queMirar,
} from '../src/lib/despliegue'
import * as vivos from '../src/lib/vivos.svelte'
import * as historial from '../src/lib/historial.svelte'
import { leerLog, type App, type Despliegue as Obj, type Monitor } from '../src/lib/contrato'

import correcto from '../src/lib/muestras/deploy-ok.json'

const fotograma = () => new Promise((r) => setTimeout(r, 40))
const lineasDe = (c: Element) => c.querySelectorAll('pre.crudo .l:not(.vacia)')

// ── El anillo ───────────────────────────────────────────────────────────────

describe('la salida guardada', () => {
  it('tiene tope, y dice cuántas se han perdido por él', () => {
    const s = new Salida(5)
    for (let i = 0; i < 12; i++) s.anotar(`l${i}`)
    expect(s.total).toBe(12)
    expect(s.guardadas).toBe(5)
    expect(s.perdidas).toBe(7)
    expect(s.ultimas(3)).toEqual(['l9', 'l10', 'l11'])
    expect(s.entre(0, 2)).toEqual(['l7', 'l8'])
    expect(s.texto()).toBe('l7\nl8\nl9\nl10\nl11\n')
  })

  it('también con tope de caracteres, y una línea enorme se recorta diciéndolo', () => {
    const s = new Salida(1000, 10)
    s.anotarVarias(['aaaa', 'bbbb', 'cccc'])
    expect(s.ultimas(10)).toEqual(['bbbb', 'cccc'])
    const larga = new Salida()
    larga.anotar('x'.repeat(TOPE_DE_LINEA + 10))
    expect(larga.ultimas(1)[0]).toContain('línea recortada')
  })

  it('compacta sin perder el orden con miles de líneas', () => {
    const s = new Salida(100)
    for (let i = 0; i < 50_000; i++) s.anotar(String(i))
    expect(s.ultimas(2)).toEqual(['49998', '49999'])
    expect(s.guardadas).toBe(100)
  })

  it('vaciar avisa y empieza de cero', () => {
    const s = new Salida()
    let avisos = 0
    const baja = s.alCambiar(() => (avisos += 1))
    s.anotarVarias(['a', 'b'])
    s.vaciar()
    baja()
    s.anotar('c')
    expect(avisos).toBe(2)
    expect(s.total).toBe(1)
  })
})

// ── La consola ──────────────────────────────────────────────────────────────

describe('la consola', () => {
  it('es un log que no se lee en voz alta, y pinta lo que ya había al montarse', () => {
    const s = salidaDeTexto('uno\ndos\n')
    const { container } = render(Consola, { salida: s })
    const pre = container.querySelector('pre.crudo')!
    expect(pre.getAttribute('role')).toBe('log')
    expect(pre.getAttribute('aria-live')).toBe('off')
    expect(lineasDe(container)).toHaveLength(2)
  })

  it('junta las líneas de un fotograma en una sola pintada', async () => {
    const s = new Salida()
    const { container } = render(Consola, { salida: s })
    expect(container.textContent).toContain('esperando al servidor')
    for (let i = 0; i < 500; i++) s.anotar(`línea ${i}`)
    // Nada todavía: se pinta en el fotograma, no en cada línea.
    expect(lineasDe(container)).toHaveLength(0)
    await fotograma()
    expect(lineasDe(container)).toHaveLength(500)
    expect(container.textContent).not.toContain('esperando al servidor')
  })

  it('en la página sólo hay un tramo; lo de arriba se pide con «ver más»', async () => {
    const s = new Salida()
    const { container } = render(Consola, { salida: s, tramo: 100 })
    for (let i = 0; i < 350; i++) s.anotar(`l${i}`)
    await fotograma()
    expect(lineasDe(container)).toHaveLength(100)
    expect(lineasDe(container)[0]!.textContent).toBe('l250')
    const mas = container.querySelector<HTMLButtonElement>('button.mas')!
    expect(mas.textContent).toContain('250 más arriba')
    await fireEvent.click(mas)
    expect(lineasDe(container)).toHaveLength(200)
    expect(lineasDe(container)[0]!.textContent).toBe('l150')
  })

  it('dice que el principio ya no se guarda', async () => {
    const s = new Salida(10)
    const { container } = render(Consola, { salida: s })
    for (let i = 0; i < 25; i++) s.anotar(`l${i}`)
    await fotograma()
    expect(container.textContent).toContain('Las primeras 15 líneas ya no se guardan')
  })

  it('vaciar la salida vacía la consola', async () => {
    const s = salidaDeTexto('a\nb\n')
    const { container } = render(Consola, { salida: s })
    s.vaciar()
    await fotograma()
    expect(lineasDe(container)).toHaveLength(0)
    expect(container.textContent).toContain('esperando al servidor')
  })

  it('el texto es texto: nada del servidor entra como HTML', async () => {
    const s = new Salida()
    const { container } = render(Consola, { salida: s })
    s.anotar('<img src=x onerror=alert(1)>')
    await fotograma()
    expect(container.querySelector('pre.crudo img')).toBeNull()
    expect(container.textContent).toContain('<img src=x')
  })

  it('«↓ seguir» tiene nombre accesible y vuelve al final', async () => {
    const s = salidaDeTexto(Array.from({ length: 50 }, (_, i) => `l${i}`).join('\n'))
    const { container } = render(Consola, { salida: s })
    const pre = container.querySelector<HTMLPreElement>('pre.crudo')!
    // jsdom no maqueta: se simula que alguien ha subido a leer.
    Object.defineProperty(pre, 'scrollHeight', { configurable: true, value: 1000 })
    Object.defineProperty(pre, 'clientHeight', { configurable: true, value: 200 })
    pre.scrollTop = 100
    await fireEvent.scroll(pre)
    const b = container.querySelector<HTMLButtonElement>('button.seguir')!
    expect(b.getAttribute('aria-label')).toBe('Ir al final y seguir la salida')
    await fireEvent.click(b)
    expect(container.querySelector('button.seguir')).toBeNull()
  })
})

// ── El registro de despliegues vivos ────────────────────────────────────────

describe('el registro de despliegues', () => {
  it('apunta las tandas del núcleo, y la forma vieja de una línea', () => {
    vivos.empezar('rc', 'web')
    const k = vivos.clave('rc', 'web')
    vivos.anotar(k, ['a', '{"event":"step","app":"web","step":"code","status":"ok","elapsed_s":1}'])
    vivos.anotar(k, 'b')
    const v = vivos.ver('rc', 'web')!
    expect(v.salida.ultimas(10)).toEqual(['a', '{"event":"step","app":"web","step":"code","status":"ok","elapsed_s":1}', 'b'])
    expect(v.progreso.pasos[0]!.estado).toBe('hecho')
    vivos.olvidar(k)
  })

  it('la prosa del build no rehace el progreso', () => {
    vivos.empezar('rc', 'prosa')
    const k = vivos.clave('rc', 'prosa')
    vivos.anotar(k, '{"event":"step","app":"prosa","step":"build","status":"start","elapsed_s":2}')
    flushSync()
    const antes = vivos.ver('rc', 'prosa')!.progreso
    for (let i = 0; i < 100; i++) vivos.anotar(k, `  [build] módulo ${i}`)
    expect(vivos.ver('rc', 'prosa')!.progreso).toBe(antes)
    vivos.olvidar(k)
  })

  it('la salida tiene tope aunque el build no se calle', () => {
    vivos.empezar('rc', 'ruido')
    const k = vivos.clave('rc', 'ruido')
    for (let i = 0; i < 25_000; i++) vivos.anotar(k, `l${i}`)
    const s = vivos.ver('rc', 'ruido')!.salida
    expect(s.guardadas).toBeLessThanOrEqual(20_000)
    expect(s.perdidas).toBeGreaterThan(0)
    vivos.olvidar(k)
  })

  it('olvidarTerminado no suelta uno que sigue corriendo', () => {
    vivos.empezar('rc', 'suelta')
    const k = vivos.clave('rc', 'suelta')
    expect(vivos.olvidarTerminado(k)).toBe(false)
    expect(vivos.ver('rc', 'suelta')).toBeDefined()
    vivos.terminar(k, { ...(correcto as Obj), app: 'suelta' })
    expect(vivos.olvidarTerminado(k)).toBe(true)
    expect(vivos.ver('rc', 'suelta')).toBeUndefined()
  })

  it('el evento se valida: lo que no tenga forma se descarta', () => {
    expect(vivos.lineasDelEvento(['k', ['a', 'b']])).toEqual(['k', ['a', 'b']])
    expect(vivos.lineasDelEvento(['k', 'a'])).toEqual(['k', ['a']])
    expect(vivos.lineasDelEvento(['k', ['a', 3, null]])).toEqual(['k', ['a']])
    expect(vivos.lineasDelEvento(['k'])).toBeNull()
    expect(vivos.lineasDelEvento({ k: 1 })).toBeNull()
    expect(vivos.lineasDelEvento([1, 'a'])).toBeNull()
  })
})

// ── La pasada ───────────────────────────────────────────────────────────────

function lineasDePasada(apps: number, prosa: number): string[] {
  const out: string[] = []
  let t = 0
  for (let a = 0; a < apps; a++) {
    const app = `app${a}`
    out.push(JSON.stringify({ event: 'app', app, status: 'start', elapsed_s: t }))
    for (const step of ['code', 'build', 'service']) {
      out.push(JSON.stringify({ event: 'step', app, step, status: 'start', elapsed_s: 1 }))
      for (let i = 0; i < prosa; i++) out.push(`  [${app}] ${step} ${i}`)
    }
    if (a % 7 === 3) out.push('{"event":"app","app":"roto"')
    if (a % 5 === 2) out.push('{"event":"delfuturo","app":"x"}')
    t += 10
    out.push(JSON.stringify({ event: 'app', app, status: a % 4 === 1 ? 'failed' : 'deployed', elapsed_s: t }))
  }
  // Una que vuelve a empezar después de terminar, y un paso sin «start».
  out.push(JSON.stringify({ event: 'app', app: 'app0', status: 'start', elapsed_s: t + 1 }))
  out.push(JSON.stringify({ event: 'step', app: 'huerfana', step: 'build', status: 'start', elapsed_s: 1 }))
  return out
}

describe('la pasada leída línea a línea', () => {
  it('da lo mismo que leerla entera, en cada punto', () => {
    const lineas = lineasDePasada(12, 3)
    const l = lectorDePasada()
    lineas.forEach((x, i) => {
      l.linea(x)
      if (i % 9 === 0 || i === lineas.length - 1) {
        expect(l.foto()).toEqual(leerPasada(lineas.slice(0, i + 1).join('\n')))
      }
    })
  })

  it('la foto es una copia', () => {
    const l = lectorDePasada()
    l.linea('{"event":"app","app":"a","status":"start","elapsed_s":0}')
    const f = l.foto()
    l.linea('{"event":"app","app":"a","status":"deployed","elapsed_s":3}')
    expect(f.apps[0]!.final).toBeNull()
  })

  it('en vivo, la pantalla sigue el estado sin volver a leer la salida', async () => {
    const p = new vivos.PasadaEnVivo()
    const prosa = p.estado
    p.anotar(['prosa', 'más prosa'])
    expect(p.estado).toBe(prosa) // nada que cambiar, nada que pintar
    const apps: App[] = ['tienda', 'blog'].map((name) => ({
      name, type: 'node', domain: `${name}.t`, aliases: [], state: { served: true } as App['state'],
    }))
    const { container } = render(Pasada, {
      servidor: 'vps', apps, pasada: p, corriendo: true, modo: 'si-cambia',
      alLanzar: () => {}, alCancelar: () => {}, alCerrar: () => {},
    })
    p.anotar([
      '{"event":"app","app":"tienda","status":"deployed","elapsed_s":90}',
      '{"event":"app","app":"blog","status":"start","elapsed_s":90}',
      '{"event":"step","app":"blog","step":"build","status":"start","elapsed_s":3}',
    ])
    await tick()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('1 app hecha')
    expect(t).toContain('build')
    p.vaciar()
    await tick()
    expect(container.textContent).toContain('esperando al servidor')
  })

  it('un final desconocido no elige clases de la hoja de estilos', () => {
    const { container } = render(Pasada, {
      servidor: 'vps', apps: [], corriendo: true,
      crudo: '{"event":"app","app":"a","status":"x final--bien","elapsed_s":1}',
      alLanzar: () => {}, alCancelar: () => {}, alCerrar: () => {},
    })
    const chip = container.querySelector('.chip')!
    expect(chip.classList.contains('final--bien')).toBe(false)
    expect(chip.getAttribute('title')).toBe('x final--bien')
  })
})

// ── El historial del monitor ────────────────────────────────────────────────

function mon(n: number, t: number, nombres?: string[]): Monitor {
  return {
    schema: 1,
    apps: (nombres ?? Array.from({ length: n }, (_, i) => `app${i}`)).map((name, i) => ({
      name, service: 'active', port: 3000 + i,
      cpu_percent: t, memory_bytes: 1e8 + t, requests_last_minute: t, requests_capped: false,
    })),
  } as unknown as Monitor
}

describe('el historial del monitor', () => {
  it('tiene tope de 90 por serie, y la clave es servidor:app', () => {
    for (let t = 0; t < 130; t++) historial.anotar('h-a', mon(3, t))
    historial.anotar('h-b', mon(3, 999))
    const s = historial.serie('h-a', 'app1')!
    expect(s.cpu).toHaveLength(historial.MUESTRAS)
    expect(s.cpu[s.cpu.length - 1]).toBe(129)
    expect(s.cpu[0]).toBe(40)
    expect(historial.serie('h-b', 'app1')!.cpu).toEqual([999])
  })

  it('lo entregado no cambia con la pasada siguiente', () => {
    historial.anotar('h-c', mon(1, 1))
    const antes = historial.serie('h-c', 'app0')!
    historial.anotar('h-c', mon(1, 2))
    expect(antes.cpu).toEqual([1])
    expect(historial.serie('h-c', 'app0')!.cpu).toEqual([1, 2])
  })

  it('una app llamada __proto__ es una app más', () => {
    historial.anotar('h-d', mon(0, 5, ['__proto__', 'constructor']))
    expect(historial.serie('h-d', '__proto__')!.cpu).toEqual([5])
    expect(historial.serie('h-d', 'constructor')!.cpu).toEqual([5])
    expect(historial.serie('h-d', 'toString')).toBeUndefined()
  })

  it('olvidar un servidor no toca los demás', () => {
    historial.anotar('h-e', mon(1, 1))
    historial.anotar('h-f', mon(1, 1))
    historial.olvidar('h-e')
    expect(historial.serie('h-e', 'app0')).toBeUndefined()
    expect(historial.serie('h-f', 'app0')).toBeDefined()
  })
})

// ── La pantalla del despliegue ──────────────────────────────────────────────

const obj = (o: Partial<Obj>): Obj => ({ ...(correcto as Obj), ...o })
const boton = (c: Element, t: string) =>
  [...c.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.includes(t))

describe('cancelar un despliegue', () => {
  it('un despliegue nuevo vuelve a poder cancelarse', async () => {
    const primera = new Salida()
    const r = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''), salida: primera, desde: 1,
      alCancelar: async () => true,
    })
    await fireEvent.click(boton(r.container, 'Cancelar')!)
    expect(boton(r.container, 'Cancelando')!.disabled).toBe(true)
    // Termina (cancelado) y se lanza otro desde la misma pestaña.
    await r.rerender({ resultado: obj({ ok: false, failed_step: 'cancelled', error: 'cancelado' }) })
    await r.rerender({ resultado: null, salida: new Salida(), desde: 2 })
    const b = boton(r.container, 'Cancelar')!
    expect(b.textContent?.trim()).toBe('Cancelar')
    expect(b.disabled).toBe(false)
  })

  it('si no se pudo cancelar, el botón vuelve a servir y se dice', async () => {
    for (const alCancelar of [async () => false, async () => { throw new Error('x') }]) {
      const { container, unmount } = render(Despliegue, {
        app: 'web', servidor: 'vps', progreso: leerProgreso(''), salida: new Salida(), desde: 1,
        alCancelar,
      })
      await fireEvent.click(boton(container, 'Cancelar')!)
      await tick()
      await tick()
      const b = boton(container, 'Cancelar')!
      expect(b.disabled).toBe(false)
      expect(container.textContent).toContain('No se ha podido cancelar')
      unmount()
    }
  })
})

describe('lo que se oye del despliegue', () => {
  it('la región del resultado ya está antes del final, y cambia de texto', async () => {
    const r = render(Despliegue, { app: 'web', servidor: 'vps', progreso: leerProgreso(''), salida: new Salida() })
    const region = r.container.querySelector('[role="status"]')!
    expect(region.textContent?.trim()).toBe('')
    await r.rerender({ resultado: obj({ ok: false, rolled_back: true, failed_step: 'service', previous: 'r1' }) })
    expect(r.container.querySelector('[role="status"]')).toBe(region)
    expect(region.textContent).toContain('sigue en pie')
    // La sección del final ya no es una región viva que se monta llena.
    expect(r.container.querySelector('section.final')?.getAttribute('role')).toBeNull()
  })

  it('el paso que se rompió se oye como «falló», y con su nombre en castellano', () => {
    const p = leerProgreso([
      '{"event":"step","app":"web","step":"code","status":"ok","elapsed_s":1}',
      '{"event":"step","app":"web","step":"build","status":"start","elapsed_s":1}',
    ].join('\n'), { app: 'web' })
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: p,
      resultado: obj({ ok: false, failed_step: 'build', error: 'npm ERR!' }),
    })
    const build = [...container.querySelectorAll('.pasos li')][2]!
    expect(build.className).toContain('paso--fallo')
    expect(build.querySelector('.nombre')!.textContent).toBe('compilación')
    expect(build.querySelector('.nombre')!.getAttribute('title')).toBe('build')
    expect(build.querySelector('.sr')!.textContent).toContain('falló')
  })

  it('«constructor» no encuentra nada en el prototipo', () => {
    expect(queMirar('constructor')).toBeNull()
    expect(queMirar('__proto__')).toBeNull()
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''),
      resultado: obj({ ok: false, release: 'r', failed_step: 'constructor', error: 'x' }),
      alVerLog: () => {}, alDiagnostico: () => {},
    })
    expect(container.querySelector('button.primaria')).toBeNull()
    expect(container.querySelector('.final .detalle')).toBeNull()
  })

  it('sólo `ok: true` es un éxito', () => {
    expect(finalDe(obj({ ok: 'false' as unknown as boolean, rolled_back: false }))).not.toBe('bien')
    expect(finalDe(obj({ ok: 1 as unknown as boolean, rolled_back: false }))).not.toBe('bien')
    expect(finalDe(obj({ ok: true }))).toBe('bien')
  })
})

// ── El visor de log ─────────────────────────────────────────────────────────

const LOG = leerLog([
  '{"schema":1,"event":"meta","app":"web","source":"nginx","unit":null,"since":null,"follow":false,"lines":80}',
  '{"event":"line","ts":null,"stream":"access","text":"GET /uno 200"}',
  '{"event":"line","ts":null,"stream":"access","text":"GET /dos 200"}',
  '{"event":"line","ts":null,"stream":"error","text":"open() failed"}',
  '{"event":"end","lines":3,"truncated":false}',
].join('\n'))

describe('el visor de log, al buscar', () => {
  it('espera a que se deje de teclear, y las filas que siguen no se rehacen', async () => {
    const { container } = render(VisorLog, { log: LOG, app: 'web' })
    const fila = () => [...container.querySelectorAll('li.linea')].find((l) => l.textContent?.includes('/dos'))!
    const antes = fila()
    const input = container.querySelector<HTMLInputElement>('input.busca')!
    await fireEvent.input(input, { target: { value: 'd' } })
    await fireEvent.input(input, { target: { value: 'do' } })
    expect(container.querySelectorAll('li.linea')).toHaveLength(3)
    await new Promise((r) => setTimeout(r, 150))
    await tick()
    expect(container.querySelectorAll('li.linea')).toHaveLength(1)
    expect(fila()).toBe(antes)
    expect(fila().querySelector('mark')?.textContent).toBe('do')
  })

  it('el canal de error se oye', () => {
    const { container } = render(VisorLog, { log: LOG, app: 'web' })
    const err = container.querySelector('li.log--error')!
    expect(err.querySelector('.sr')?.textContent).toContain('error')
  })
})

// ── Nombres que Orbit leería como palabra suya ──────────────────────────────

describe('nombres que chocan con una palabra de Orbit', () => {
  it('se niegan con su motivo, y lo demás no cambia', () => {
    for (const n of ['on', 'yes', 'si', 'sí', 'enable', 'off', 'no', 'disable']) {
      expect(chocaConPalabraDeOrbit(n, 'esperarCi')).toMatch(/otra/)
      expect(() => argvDeEsperarCi(n, true)).toThrow(/Orbit leería/)
    }
    for (const n of ['help', 'ayuda']) {
      expect(() => argvDeCongelar(n, null)).toThrow(/Orbit leería/)
      expect(() => argvDeDescongelar(n)).toThrow(/Orbit leería/)
      expect(chocaConPalabraDeOrbit(n, 'retirar')).not.toBeNull()
    }
    expect(chocaConPalabraDeOrbit('prom', 'metricas')).not.toBeNull()
    expect(chocaConPalabraDeOrbit('prom', 'congelar')).toBeNull()
    expect(chocaConPalabraDeOrbit('on', 'congelar')).toBeNull()
    expect(chocaConPalabraDeOrbit('tienda', 'esperarCi')).toBeNull()
    expect(argvDeEsperarCi('tienda', false)).toEqual(['autodeploy', 'checks', 'off', 'tienda'])
    expect(argvDeCongelar('tienda', 'black friday')).toEqual(['lock', 'tienda', '-m', 'black friday'])
  })
})
