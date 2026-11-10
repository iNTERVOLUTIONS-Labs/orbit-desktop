/**
 * Segunda ronda adversarial (usuario nuevo con NVDA y sólo teclado): adónde va
 * el foco cuando desaparece lo que lo tenía.
 *
 * Cada prueba describe el comportamiento CORRECTO, así que falla mientras el
 * fallo exista. Los recorridos en Chrome que las motivan están en el informe
 * (`adv2-ux/*.mjs`).
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'
import { tick } from 'svelte'

import Consola from '../src/componentes/Consola.svelte'
import Despliegue from '../src/componentes/Despliegue.svelte'
import ListaApps from '../src/componentes/ListaApps.svelte'
import { Salida } from '../src/lib/consola'
import { devolverFoco } from '../src/lib/foco'
import { leerProgreso } from '../src/lib/despliegue'
import type { App, Despliegue as Obj, Estado } from '../src/lib/contrato'
import correcto from '../src/lib/muestras/deploy-ok.json'

const espera = (ms = 0) => new Promise((r) => setTimeout(r, ms))
const enBody = () => document.activeElement === document.body || document.activeElement === null

afterEach(() => {
  document.body.innerHTML = ''
})

describe('devolverFoco', () => {
  it('si lo que había antes era el <body>, no lo «devuelve» al <body>: va al título', async () => {
    // Una hoja abierta desde la paleta (Ctrl+K → «Desplegar tienda») captura
    // `document.activeElement` cuando la paleta ya se ha ido: el <body>.
    // `previo.focus()` sobre el body «funciona» y la función se da por
    // satisfecha. En Chrome: Escape en esa hoja deja el foco en el <body>.
    document.body.innerHTML = '<main><h1 tabindex="-1">vps</h1></main>'
    devolverFoco(document.body)
    await espera(5)
    expect(document.activeElement?.tagName).toBe('H1')
  })
})

describe('la consola en vivo', () => {
  function conAltura(pre: HTMLElement, alto: number, visible: number) {
    Object.defineProperty(pre, 'scrollHeight', { configurable: true, get: () => alto })
    Object.defineProperty(pre, 'clientHeight', { configurable: true, get: () => visible })
  }

  it('«seguir» desaparece al pulsarlo, y el foco no se cae al <body>', async () => {
    const s = new Salida()
    s.anotarVarias(Array.from({ length: 300 }, (_, i) => `línea ${i}`))
    const { container } = render(Consola, { salida: s, vivo: true })
    await tick()
    const pre = container.querySelector('pre')!
    conAltura(pre, 5000, 260)
    pre.scrollTop = 0
    await fireEvent.scroll(pre)
    const seguir = container.querySelector<HTMLButtonElement>('button.seguir')
    expect(seguir).not.toBeNull()
    seguir!.focus()
    await fireEvent.click(seguir!)
    await tick()
    expect(container.querySelector('button.seguir')).toBeNull()
    expect(enBody()).toBe(false)
  })

  it('«ver más» de la última tanda desaparece, y el foco no se cae al <body>', async () => {
    const s = new Salida()
    s.anotarVarias(Array.from({ length: 2500 }, (_, i) => `línea ${i}`))
    const { container } = render(Consola, { salida: s, vivo: false })
    await tick()
    const mas = container.querySelector<HTMLButtonElement>('button.mas')
    expect(mas?.textContent).toContain('500 más arriba')
    mas!.focus()
    await fireEvent.click(mas!)
    await tick()
    expect(container.querySelector('button.mas')).toBeNull()
    expect(enBody()).toBe(false)
  })
})

describe('el final de un despliegue', () => {
  it('quien estaba leyendo la salida no pierde el foco cuando el despliegue termina bien', async () => {
    // Con éxito la consola se desmonta (`enMarcha || verCrudo`): en Chrome, el
    // foco en la salida —o en «Cancelar»— acaba en el <body> al terminar.
    const s = new Salida()
    s.anotarVarias(['{"event":"step","app":"web","step":"build","status":"start","elapsed_s":1}', 'compilando…'])
    const props = { app: 'web', servidor: 'vps', progreso: leerProgreso(''), salida: s, resultado: null as Obj | null }
    const r = render(Despliegue, props)
    await tick()
    const pre = r.container.querySelector<HTMLElement>('pre[role="log"]')!
    pre.focus()
    expect(document.activeElement).toBe(pre)
    await r.rerender({ ...props, resultado: { ...(correcto as Obj), app: 'web' } })
    await tick()
    await espera(5)
    expect(enBody()).toBe(false)
  })
})

describe('la caja de filtrar de la lista', () => {
  const estado = { service: 'active', port: 3000, ssl: true, cert_days: 60, maintenance: false, served: true, autodeploy: false, queue: false, releases: 2, last_deploy: null, last_deploy_sha: null } as Estado
  const app = (name: string): App => ({ name, domain: `${name}.test`, type: 'node', aliases: [], state: estado }) as unknown as App

  it('Escape vacía el filtro sin tirar el foco al <body>', async () => {
    Element.prototype.scrollIntoView = vi.fn() as unknown as typeof Element.prototype.scrollIntoView
    const { container } = render(ListaApps, { apps: [app('tienda'), app('api')], servidor: 'vps', alElegir: vi.fn() })
    const caja = container.querySelector<HTMLInputElement>('input[type="search"]')!
    caja.focus()
    await fireEvent.input(caja, { target: { value: 'ti' } })
    await fireEvent.keyDown(caja, { key: 'Escape' })
    await tick()
    expect(caja.value).toBe('')
    expect(enBody()).toBe(false)
    delete (Element.prototype as Partial<Element>).scrollIntoView
  })
})
