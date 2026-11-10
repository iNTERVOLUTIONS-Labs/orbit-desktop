/**
 * Lo que cuesta la consola en vivo, medido. Imprime cifras y pone un techo
 * holgado: no es una prueba de velocidad, es la alarma de que algo ha vuelto a
 * ser cuadrático. Opcional, para no frenar la batería normal:
 *   RC_PERF=1 npx vitest run tests/rc-consola-perf.test.ts
 */
import { describe, expect, it as it0 } from 'vitest'
import { flushSync } from 'svelte'

import { leerPasada } from '../src/lib/despliegue'
import * as vivos from '../src/lib/vivos.svelte'
import * as historial from '../src/lib/historial.svelte'
import type { Monitor } from '../src/lib/contrato'

const it = (n: string, f: () => unknown) => (process.env.RC_PERF ? it0(n, f, 120_000) : it0.skip(n, f))
const ms = (n: number) => Math.round(n * 100) / 100

/** Igual que la sonda de la ronda adversaria: 40 apps × 3 pasos × N de prosa. */
function lineasDePasada(apps: number, prosaPorApp: number): string[] {
  const out: string[] = []
  let t = 0
  for (let a = 0; a < apps; a++) {
    const app = `app${String(a).padStart(3, '0')}`
    out.push(JSON.stringify({ event: 'app', app, status: 'start', elapsed_s: t }))
    for (const step of ['code', 'release', 'build', 'activate', 'service', 'nginx']) {
      out.push(JSON.stringify({ event: 'step', app, step, status: 'start', elapsed_s: 1 }))
      if (step === 'build') for (let i = 0; i < prosaPorApp; i++) out.push(`  [build] ${app} module ${i}`)
      out.push(JSON.stringify({ event: 'step', app, step, status: 'ok', elapsed_s: 2 }))
    }
    t += 30
    out.push(JSON.stringify({ event: 'app', app, status: 'deployed', elapsed_s: t }))
  }
  return out
}

describe('rc-perf: la consola en vivo', () => {
  it('desplegar todo: 30k líneas, una a una, sin volver a leer la salida', () => {
    const lineas = lineasDePasada(40, 750)
    const p = new vivos.PasadaEnVivo()
    const a = performance.now()
    for (const l of lineas) p.anotar(l)
    flushSync()
    const total = performance.now() - a
    console.log(`[rc-perf] pasada ${lineas.length} líneas: ${ms(total)} ms en total (antes: 18,6 s)`)
    expect(p.estado).toEqual(leerPasada(lineas.join('\n')))
    expect(total).toBeLessThan(1000)
  })

  it('un despliegue: 30k líneas, por tramos de 5.000', () => {
    vivos.empezar('perf', 'tienda')
    const k = vivos.clave('perf', 'tienda')
    const tramos: number[] = []
    let a = performance.now()
    for (let i = 0; i < 30_000; i++) {
      vivos.anotar(k, i % 2500 === 0 ? JSON.stringify({ event: 'step', app: 'tienda', step: 'build', status: 'start', elapsed_s: i }) : `  [build] módulo ${i}`)
      if ((i + 1) % 5000 === 0) { const b = performance.now(); tramos.push(ms(b - a)); a = b }
    }
    console.log(`[rc-perf] vivos.anotar 30k, ms por tramo de 5.000: ${JSON.stringify(tramos)} (antes: [15.4, 11.4, 12.8, 10.6, 11.8, 11.8])`)
    // Plano: el último tramo no puede costar mucho más que el primero.
    expect(Math.max(...tramos)).toBeLessThan(Math.max(50, tramos[0]! * 4))
    vivos.olvidar(k)
  })

  it('el historial: 300 apps, por pasada del monitor con el anillo lleno', () => {
    const mon = (t: number): Monitor => ({
      schema: 1,
      apps: Array.from({ length: 300 }, (_, i) => ({
        name: `app${i}`, service: 'active', port: 3000 + i,
        cpu_percent: (i * t) % 100, memory_bytes: 1e8 + t, requests_last_minute: t % 50, requests_capped: false,
      })),
    }) as unknown as Monitor
    const tiempos: number[] = []
    for (let t = 0; t < 300; t++) {
      const a = performance.now()
      historial.anotar('perf300', mon(t))
      if (t >= 100) tiempos.push(performance.now() - a)
    }
    tiempos.sort((x, y) => x - y)
    const mediana = ms(tiempos[Math.floor(tiempos.length / 2)]!)
    console.log(`[rc-perf] historial 300 apps: mediana ${mediana} ms por pasada (antes: 96,7 ms en jsdom)`)
    expect(historial.serie('perf300', 'app0')!.cpu).toHaveLength(90)
    expect(mediana).toBeLessThan(20)
  })
})
