/**
 * Las variables de entorno: las reglas más duras del producto.
 *
 * No son de estilo. §13.2 de la arquitectura de Orbit lo dice sin rodeos: «un
 * panel que enseñe el `.env` entero es un panel que filtra la contraseña de la
 * base de datos en la primera captura de pantalla que alguien pegue en un
 * issue». Orbit cumple su mitad devolviendo sólo nombres; ésta es la nuestra.
 */
import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/svelte'
import { tick } from 'svelte'

import Entorno from '../src/componentes/Entorno.svelte'
import MonitorVista from '../src/componentes/MonitorVista.svelte'
import { bytes } from '../src/lib/contrato'
import { periodoDelMonitor } from '../src/lib/despliegue'

const ENV = { schema: 1, app: 'tienda', keys: ['DB_PASSWORD', 'STRIPE_KEY', 'APP_ENV'] }

function montar(valor = 'hunter2') {
  const pedirValor = vi.fn(async () => valor)
  const r = render(Entorno, { entorno: ENV, app: 'tienda', servidor: 'vps', pedirValor })
  return { ...r, pedirValor }
}

describe('el .env nunca se pinta entero', () => {
  it('de entrada sólo se ven los nombres', () => {
    const { container } = montar()
    expect(container.textContent).toContain('DB_PASSWORD')
    expect(container.textContent).not.toContain('hunter2')
    expect(container.querySelectorAll('.oculto').length).toBe(3)
  })

  it('no existe «revelar todo»', () => {
    // No es una función pendiente de hacer: enseñar el .env entero es lo que
    // convierte una captura de pantalla en una fuga.
    const { container } = montar()
    const textos = [...container.querySelectorAll('button')].map((b) => b.textContent?.trim())
    expect(textos.every((t) => t === 'revelar')).toBe(true)
    expect(textos.some((t) => /todo|todas/i.test(t ?? ''))).toBe(false)
  })

  it('cada valor cuesta una llamada de verdad al servidor', async () => {
    // El mismo acto deliberado que el contrato exige por comando: pedir un
    // secreto no puede ser gratis ni pasar desapercibido en el servidor.
    const { container, pedirValor } = montar()
    const botones = container.querySelectorAll('button')
    ;(botones[0] as HTMLButtonElement).click()
    await tick(); await tick()
    expect(pedirValor).toHaveBeenCalledOnce()
    expect(pedirValor).toHaveBeenCalledWith('DB_PASSWORD')
  })

  it('sólo hay uno revelado a la vez', async () => {
    const { container } = montar()
    const b = container.querySelectorAll('button')
    ;(b[0] as HTMLButtonElement).click(); await tick(); await tick()
    ;(b[1] as HTMLButtonElement).click(); await tick(); await tick()
    expect(container.querySelectorAll('.valor').length).toBe(1)
  })

  it('se oculta al perder el foco la ventana', async () => {
    // Es el caso de la videollamada compartida y el del portátil abierto en una
    // mesa, que son los reales.
    const { container } = montar()
    ;(container.querySelector('button') as HTMLButtonElement).click()
    await tick(); await tick()
    expect(container.querySelector('.valor')).not.toBeNull()

    window.dispatchEvent(new Event('blur'))
    await tick()
    expect(container.querySelector('.valor')).toBeNull()
  })

  it('se oculta solo a los 30 segundos', async () => {
    vi.useFakeTimers()
    const { container } = montar()
    ;(container.querySelector('button') as HTMLButtonElement).click()
    await Promise.resolve(); await tick(); await tick()
    expect(container.querySelector('.valor')).not.toBeNull()

    await vi.advanceTimersByTimeAsync(31_000)
    await tick()
    expect(container.querySelector('.valor')).toBeNull()
    vi.useRealTimers()
  })

  it('el reloj se ve: quien lo revela sabe cuánto le queda', async () => {
    const { container } = montar()
    ;(container.querySelector('button') as HTMLButtonElement).click()
    await tick(); await tick()
    expect(container.querySelector('.reloj')?.textContent).toContain('s')
  })

  it('el botón se anuncia con la app y el servidor, no sólo «revelar»', async () => {
    // Con un lector de pantalla, tres botones que dicen «revelar» son tres
    // botones idénticos.
    montar()
    expect(screen.getByLabelText(/Revelar el valor de DB_PASSWORD en vps/)).toBeTruthy()
  })
})

describe('el monitor', () => {
  const APP = {
    name: 'web', type: 'node', domain: 'a.test', port: 3001, service: 'active',
    cpu_percent: null, memory_bytes: 52428800,
    requests_last_minute: 5000, requests_capped: true,
  }

  it('una CPU que no se sabe NO se pinta como 0 %', () => {
    // La CPU es la diferencia entre dos lecturas, así que la primera vez no hay
    // porcentaje. Inventar un cero sería mentir: un cero es una afirmación.
    const { container } = render(MonitorVista, {
      monitor: { schema: 1, apps: [APP] }, servidor: 'vps', periodo: 3,
    })
    const celdas = [...container.querySelectorAll('td')].map((c) => c.textContent?.trim())
    expect(celdas).toContain('·')
    expect(celdas).not.toContain('0 %')
  })

  it('el tope de peticiones se anuncia con un «+»', () => {
    // Un número corto sin avisar se lee como «hay poco tráfico», que es justo lo
    // contrario de lo que está pasando.
    const { container } = render(MonitorVista, {
      monitor: { schema: 1, apps: [APP] }, servidor: 'vps', periodo: 3,
    })
    expect(container.querySelector('.tope')?.textContent).toBe('+')
  })

  it('el periodo se enseña, porque «en vivo» sin decir cada cuánto es mentir', () => {
    const { container } = render(MonitorVista, {
      monitor: { schema: 1, apps: [APP] }, servidor: 'vps', periodo: 4,
    })
    expect(container.querySelector('.periodo')?.textContent).toContain('4 s')
  })

  it('el periodo se adapta a lo que de verdad tarda', () => {
    // `top --json` cuesta ~2,1 s con 40 apps: refrescar cada dos segundos era
    // físicamente imposible, y encadenar peticiones más rápido de lo que
    // contestan no da más frescura, da una cola.
    expect(periodoDelMonitor(null)).toBe(3)
    expect(periodoDelMonitor(300)).toBe(3)        // rápido: el mínimo manda
    expect(periodoDelMonitor(2100)).toBeGreaterThanOrEqual(4)
    expect(periodoDelMonitor(60_000)).toBeLessThanOrEqual(30)
  })

  it('los bytes se presentan aquí, porque el contrato los da en bruto', () => {
    // Y los da así a propósito: `du -h` es presentación, y su separador decimal
    // depende de la configuración regional del servidor.
    expect(bytes(52428800)).toBe('50 MB')
    expect(bytes(null)).toBe('·')
    expect(bytes(0)).toBe('0 B')
  })
})
