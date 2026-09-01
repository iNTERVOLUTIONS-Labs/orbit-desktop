/**
 * Comparar dos servidores.
 *
 * Es la pantalla más peligrosa del cliente, y no por lo que hace —dos lecturas,
 * no escribe nada— sino por lo que puede hacer creer. Casi todas las pruebas de
 * aquí son sobre lo que **no** puede afirmar.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'

import Comparar from '../src/componentes/Comparar.svelte'
import { comparar, sha } from '../src/lib/comparar'
import type { App, Estado } from '../src/lib/contrato'

const BASE: Estado = {
  service: 'active', port: 3000, ssl: true, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 3, last_deploy: '20260830-120000', last_deploy_sha: 'abc1234def',
}
const app = (name: string, p: Partial<App> = {}, e: Partial<Estado> = {}): App => ({
  name,
  type: 'node',
  domain: `${name}.ejemplo.com`,
  aliases: [],
  state: { ...BASE, ...e },
  ...p,
})

function montar(props: Record<string, unknown> = {}) {
  const alElegir = vi.fn()
  const alCerrar = vi.fn()
  const r = render(Comparar, {
    a: 'produccion',
    b: 'staging',
    appsA: [app('tienda')],
    appsB: [app('tienda')],
    candidatos: ['staging', 'otro'],
    alElegir,
    alCerrar,
    ...props,
  })
  return { ...r, alElegir, alCerrar }
}

// ── El comparador ───────────────────────────────────────────────────────────

describe('qué se compara', () => {
  it('reparte las apps en los tres montones', () => {
    const c = comparar(
      [app('tienda'), app('blog'), app('viejo')],
      [app('tienda'), app('blog'), app('nuevo')],
    )
    expect(c.enLosDos.map((f) => f.app)).toEqual(['blog', 'tienda'])
    expect(c.soloA.map((f) => f.app)).toEqual(['viejo'])
    expect(c.soloB.map((f) => f.app)).toEqual(['nuevo'])
  })

  it('en orden alfabético: dos listas ordenadas distinto no se leen en paralelo', () => {
    const c = comparar([app('zeta'), app('alfa')], [app('zeta'), app('alfa')])
    expect(c.enLosDos.map((f) => f.app)).toEqual(['alfa', 'zeta'])
  })

  it('encuentra el commit distinto, que es la pregunta de verdad', () => {
    const c = comparar(
      [app('tienda', {}, { last_deploy_sha: 'aaaaaaa111' })],
      [app('tienda', {}, { last_deploy_sha: 'bbbbbbb222' })],
    )
    const d = c.enLosDos[0]!.diferencias.find((x) => x.campo === 'last_deploy_sha')
    expect(d).toBeDefined()
    expect(c.conDiferencias).toBe(1)
  })

  it('y el autodespliegue, que es la diferencia que más caro sale', () => {
    // Uno se despliega solo al empujar y el otro no, y nadie se acuerda de cuál.
    const c = comparar(
      [app('tienda', {}, { autodeploy: true })],
      [app('tienda', {}, { autodeploy: false })],
    )
    const d = c.enLosDos[0]!.diferencias.find((x) => x.campo === 'autodeploy')
    expect(d?.a).toBe('sí')
    expect(d?.b).toBe('no')
  })

  it('lo que cambia solo NO se compara', () => {
    // `service`, `port`, `releases` y `last_deploy` difieren casi siempre entre
    // dos servidores y no dicen nada. Una lista de diferencias con ruido es una
    // lista que se deja de leer.
    const c = comparar(
      [app('tienda', {}, { service: 'active', port: 3000, releases: 12, last_deploy: '20260830-120000' })],
      [app('tienda', {}, { service: 'failed', port: 3999, releases: 2, last_deploy: '20260101-000000' })],
    )
    expect(c.enLosDos[0]!.diferencias).toEqual([])
  })

  it('un hueco no es una diferencia', () => {
    // `last_deploy_sha: null` es «no lo ha desplegado nunca», no «otro commit».
    // Es la misma regla por la que un null no se pinta como un cero.
    const c = comparar(
      [app('tienda', {}, { last_deploy_sha: null })],
      [app('tienda', {}, { last_deploy_sha: 'bbbbbbb' })],
    )
    expect(c.enLosDos[0]!.diferencias).toEqual([])
  })

  it('«cert_days» no se compara nunca, porque list no lo calcula', () => {
    // Es null en `list` y en `status` SIEMPRE: sólo lo calcula `info`. Una
    // comparación que lo mirara compararía dos huecos.
    const c = comparar(
      [app('tienda', {}, { cert_days: 3 })],
      [app('tienda', {}, { cert_days: 80 })],
    )
    expect(c.enLosDos[0]!.diferencias.map((d) => d.campo)).not.toContain('cert_days')
  })

  it('mismo nombre y distinto dominio se marca como duda, no como conclusión', () => {
    // Que dos apps se llamen igual no las hace la misma app.
    const c = comparar(
      [app('blog', { domain: 'blog.uno.com' })],
      [app('blog', { domain: 'blog.dos.com' })],
    )
    expect(c.enLosDos[0]!.nombreIgualDominioDistinto).toBe(true)
  })

  it('el commit se recorta para leerlo pero un null no se recorta a nada', () => {
    expect(sha('abc1234def5678')).toBe('abc1234')
    expect(sha(null)).toBe('')
  })
})

// ── La pantalla ─────────────────────────────────────────────────────────────

describe('media comparación no es una comparación', () => {
  it('si el otro no contestó, NO se enseña la lista del que sí', () => {
    // El fallo que esta pantalla existe para no cometer. Enseñar la lista de
    // «a» con los huecos de «b» en blanco sacaría todas sus apps como «sólo en
    // produccion», y eso invita a crearlas otra vez en un servidor donde puede
    // que ya existan. Es confundir «no he podido preguntar» con «no lo tiene»,
    // con peores consecuencias que en el lote.
    const { container } = montar({
      appsA: [app('tienda'), app('blog')],
      appsB: null,
      fallo: { clase: 'no-llego', mensaje: 'no he llegado al servidor' },
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('No he podido comparar')
    expect(t).toContain('no quiere decir que no tenga nada')
    expect(container.querySelector('.tabla')).toBeNull()
    expect(container.querySelector('.solo')).toBeNull()
    expect(t).not.toContain('blog')
  })

  it('y dice por qué no contestó, sin interpretarlo', () => {
    const { container } = montar({
      appsB: null,
      fallo: { clase: 'clave-de-host-cambiada', mensaje: 'la clave del host ha cambiado' },
    })
    expect(container.querySelector('.motivo')?.textContent).toContain('la clave del host')
  })

  it('una lista vacía SÍ es una comparación: quiere decir que no tiene apps', () => {
    // `appsB: []` y `appsB: null` son dos cosas distintas, y ésta es la que sí
    // se puede comparar.
    const { container } = montar({ appsA: [app('tienda')], appsB: [] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('1 sólo en produccion')
    expect(container.querySelector('.sin-comparar')).toBeNull()
  })
})

describe('de qué servidor es cada celda', () => {
  it('la cabecera lleva el alias escrito, en las dos columnas', () => {
    // La regla del cliente multiservidor: la clave es `servidor:app` y nunca la
    // app sola. Esta pantalla pone dos servidores a la misma altura, así que la
    // atribución no puede depender de recordar cuál era cuál.
    const { container } = montar({
      appsA: [app('tienda', {}, { autodeploy: true })],
      appsB: [app('tienda', {}, { autodeploy: false })],
    })
    const cabeceras = [...container.querySelectorAll('th')].map((x) => x.textContent?.trim())
    expect(cabeceras).toContain('produccion')
    expect(cabeceras).toContain('staging')
  })

  it('y las listas de «sólo en» también', () => {
    const { container } = montar({ appsA: [app('tienda')], appsB: [app('otra')] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Sólo en produccion')
    expect(t).toContain('Sólo en staging')
  })

  it('el color no es la señal: el nombre va escrito igualmente', () => {
    // Lo que sobrevive al daltonismo y a una captura en blanco y negro.
    const { container } = montar({ appsA: [app('tienda')], appsB: [app('otra')] })
    const marcas = [...container.querySelectorAll('.lado--a, .lado--b')]
    expect(marcas.length).toBeGreaterThan(0)
    expect(marcas.every((m) => (m.textContent ?? '').trim().length > 0)).toBe(true)
  })
})

describe('lo que la comparación no puede decir', () => {
  it('lo dice ella misma', () => {
    // `list --json` no trae la rama ni el repositorio: dos apps en el mismo
    // commit pueden venir de repositorios distintos. Callarlo dejaría a alguien
    // creyendo que «sin diferencias» quiere decir «idénticas».
    const { container } = montar()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('La rama y el repositorio no salen ahí')
  })

  it('sin diferencias se dice en voz alta, no con una tabla vacía', () => {
    const { container } = montar()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('están igual en todo lo que se compara')
    expect(container.querySelector('.tabla')).toBeNull()
  })

  it('con un solo servidor no se finge que haya algo que comparar', () => {
    const { container } = montar({ b: null, candidatos: [] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Con uno no hay nada que comparar')
  })

  it('y antes de elegir se dice que sólo va a leer', () => {
    const { container } = montar({ b: null })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('no cambia nada en ninguno de los dos')
  })
})
