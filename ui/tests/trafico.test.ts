/**
 * Tráfico y métricas: dos pantallas cuyo valor está en lo que NO afirman.
 *
 * Lo que hace legítima una analítica no es la suma, es lo que se dice sobre
 * ella. Son IPs y no personas, lo automático va aparte, y una ventana que el
 * log ya no cubre se anuncia recortada en vez de devolver un número más pequeño
 * y callarse.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const SRC_EST = join(import.meta.dirname, '..', 'src', 'estilos')

import Barras from '../src/componentes/Barras.svelte'
import TraficoVista from '../src/componentes/TraficoVista.svelte'
import MetricasVista from '../src/componentes/MetricasVista.svelte'
import type { Envoltorio, Metricas, Trafico } from '../src/lib/contrato'

import traficoCrudo from '../src/lib/muestras/traffic.json'
import metricasCrudo from '../src/lib/muestras/metrics.json'

// Las dos vienen ENVUELTAS, también pidiendo una sola app. Lo comprobamos
// ejecutando, porque la documentación las daba planas.
const T = (traficoCrudo as Envoltorio<Trafico>).apps[0]!
const M = (metricasCrudo as Envoltorio<Metricas>).apps[0]!

describe('la respuesta viene envuelta', () => {
  it('traffic y metrics traen {schema, apps:[…]} aunque se pida una app', () => {
    expect((traficoCrudo as Envoltorio<Trafico>).apps.length).toBe(1)
    expect((metricasCrudo as Envoltorio<Metricas>).apps.length).toBe(1)
  })
})

describe('el tráfico', () => {
  it('una ventana recortada se ANUNCIA, no se recorta en silencio', () => {
    // Un total recortado sin avisar se lee como el total, y entonces alguien
    // concluye que su web tiene menos visitas de las que tiene.
    expect(T.complete).toBe(false)
    const { container } = render(TraficoVista, { trafico: T, servidor: 'vps' })
    expect(container.querySelector('.aviso')?.textContent).toContain('no llega a cubrir')
  })

  it('lo automático va APARTE, no sumado a las personas', () => {
    // En un VPS con IP pública buena parte del tráfico son escáneres buscando
    // /.git/config: sumarlo convierte la analítica en un número que no describe
    // a nadie.
    const { container } = render(TraficoVista, { trafico: T, servidor: 'vps' })
    const etiquetas = [...container.querySelectorAll('.et')].map((n) => n.textContent)
    expect(etiquetas).toContain('automáticas')
    expect(etiquetas).toContain('peticiones de personas')

    const cifras = [...container.querySelectorAll('.n')].map((n) => n.textContent?.trim())
    // La de personas es el total MENOS lo automático, no el total.
    expect(cifras[0]).toBe(String(T.requests! - T.automated!))
    expect(cifras[0]).not.toBe(String(T.requests))
  })

  it('dice que son IPs y no personas', () => {
    const { container } = render(TraficoVista, { trafico: T, servidor: 'vps' })
    expect(container.textContent).toContain('IPs distintas')
    expect(container.textContent).not.toMatch(/\bvisitantes\b|\busuarios únicos\b/)
  })

  it('sin muestras, los percentiles NO se pintan como cero', () => {
    // Un percentil sin muestras no es un cero, es nada. Y se dice por qué,
    // porque el motivo tiene arreglo.
    expect(T.latency_ms.lines).toBe(0)
    const { container } = render(TraficoVista, { trafico: T, servidor: 'vps' })
    expect(container.querySelector('.latencia')).toBeNull()
    expect(container.textContent).toContain('nginx-rebuild')
    expect(container.textContent).not.toContain('p50 0 ms')
  })

  it('con muestras sí se pintan', () => {
    const con: Trafico = { ...T, latency_ms: { p50: 21, p95: 180, max: 940, lines: 412 } }
    const { container } = render(TraficoVista, { trafico: con, servidor: 'vps' })
    expect(container.querySelector('.latencia')?.textContent).toContain('21 ms')
    expect(container.textContent).toContain('412 peticiones')
  })

  it('dice de dónde sale el dato', () => {
    // Sin cookies, sin JavaScript y sin nada nuevo corriendo. Es lo que hace
    // que una analítica quepa aquí cuando un panel web no cabe.
    const { container } = render(TraficoVista, { trafico: T, servidor: 'vps' })
    expect(container.querySelector('.pie')?.textContent).toContain('sin cookies')
  })
})

describe('la gráfica', () => {
  it('un hueco NO es un cero, y se ven distintos', () => {
    // Es el argumento que decidió escribirla en vez de delegarla: las librerías
    // genéricas o interpolan por encima del null o lo tratan como cero, y las
    // dos cosas son mentiras.
    const { container } = render(Barras, {
      datos: [{ x: 'a', y: 10 }, { x: 'b', y: null }, { x: 'c', y: 0 }],
      etiqueta: 'prueba',
    })
    expect(container.querySelectorAll('.hueco').length).toBe(1)
    expect(container.querySelectorAll('.barra').length).toBe(2)
  })

  it('el hueco no se pinta como una alarma', () => {
    // «No se sabe» no es un problema, y pintarlo como si lo fuera enseña a
    // ignorar los que sí lo son.
    const c = readFileSync(join(SRC_EST, 'estado.css'), 'utf8')
    expect(c).toMatch(/\.hueco\s*\{\s*fill:\s*var\(--st-unknown\)/)
  })

  it('la gráfica tiene nombre accesible', () => {
    const { container } = render(Barras, {
      datos: [{ x: 'a', y: 1 }], etiqueta: 'Peticiones por hora',
    })
    expect(container.querySelector('svg')?.getAttribute('aria-label')).toBe('Peticiones por hora')
  })
})

describe('las métricas', () => {
  it('enseña la MEDIANA, no la media', () => {
    // Un build que una vez tardó 400 s no describe ningún despliegue real, y la
    // media se lo lleva todo.
    const { container } = render(MetricasVista, { metricas: M })
    expect(container.textContent).toContain('mediana')
    expect(container.textContent).not.toMatch(/\bmedia\b(?!na)/)
  })

  it('sin tendencia NO se pinta una flecha plana', () => {
    // Orbit se calla con menos de seis builds porque dos datos no son una
    // tendencia. Una flecha plana afirmaría «no cambia», que es justo lo que no
    // se sabe.
    const sin: Metricas = { ...M, build_trend_s: null }
    const { container } = render(MetricasVista, { metricas: sin })
    expect(container.querySelector('.tendencia')).toBeNull()
    expect(container.textContent).toContain('no se rellena el hueco')
  })

  it('y con tendencia se dice en qué dirección', () => {
    const peor: Metricas = { ...M, build_trend_s: 40 }
    const { container: a } = render(MetricasVista, { metricas: peor })
    expect(a.textContent).toContain('más que antes')
    expect(a.querySelector('.tendencia--peor')).not.toBeNull()

    const mejor: Metricas = { ...M, build_trend_s: -40 }
    const { container: b } = render(MetricasVista, { metricas: mejor })
    expect(b.textContent).toContain('menos que antes')
    expect(b.querySelector('.tendencia--peor')).toBeNull()
  })
})
