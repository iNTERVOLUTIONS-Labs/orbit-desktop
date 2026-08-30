/**
 * La hoja de comando y el registro de despliegues vivos.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'

import HojaDeComando from '../src/componentes/HojaDeComando.svelte'

describe('la hoja de comando', () => {
  const base = {
    titulo: 'Volver atrás',
    servidor: 'vps-ovh',
    orden: 'orbit rollback tienda 20260805-041230',
    consecuencia: 'Reinicia el servicio y recarga nginx.',
    verbo: 'Volver atrás',
    alConfirmar: () => {},
    alCancelar: () => {},
  }

  it('enseña la orden LITERAL, que es la prueba de que sólo se invoca orbit', () => {
    // Es la promesa que deja existir a esta aplicación, y un panel web de los
    // que Orbit rechaza no podría enseñar esto nunca.
    const { container } = render(HojaDeComando, base)
    expect(container.querySelector('.orden')?.textContent).toBe(base.orden)
  })

  it('el servidor va en el TÍTULO, no en la letra pequeña', () => {
    // `tienda` existe en tres servidores, y el accidente más caro de un cliente
    // multiservidor no es un ataque: es ejecutar lo correcto contra el
    // equivocado.
    const { container } = render(HojaDeComando, base)
    expect(container.querySelector('h2')?.textContent).toContain('vps-ovh')
  })

  it('dice lo que va a costar', () => {
    const { container } = render(HojaDeComando, base)
    expect(container.querySelector('.consecuencia')?.textContent).toContain('recarga nginx')
  })

  it('el botón de confirmar no está deshabilitado cuando no hace falta escribir', () => {
    // La fricción es un recurso escaso: se gasta donde el daño es irreversible
    // y en ningún otro sitio. Pedirla para todo enseña a teclear sin leer.
    const { container } = render(HojaDeComando, base)
    expect(container.querySelector<HTMLButtonElement>('.confirmar')!.disabled).toBe(false)
  })

  it('y sí lo está hasta escribir el nombre, cuando el daño es irreversible', () => {
    const { container } = render(HojaDeComando, {
      ...base, peligrosa: true, confirmarEscribiendo: 'tienda',
    })
    expect(container.querySelector<HTMLButtonElement>('.confirmar')!.disabled).toBe(true)
    expect(container.querySelector('input')).not.toBeNull()
  })

  it('el foco entra en el diálogo y NO en el botón de confirmar', () => {
    // Un destructivo cuyo primer anuncio es «Botón: Eliminar» es una trampa.
    const { container } = render(HojaDeComando, { ...base, peligrosa: true })
    expect(document.activeElement).not.toBe(container.querySelector('.confirmar'))
    expect(container.querySelector('[role="dialog"]')?.getAttribute('aria-modal')).toBe('true')
  })
})

describe('el registro de despliegues vivos', () => {
  it('la clave es servidor:app, nunca la app sola', async () => {
    // `tienda` existe en tres servidores y son tres despliegues distintos.
    const v = await import('../src/lib/vivos.svelte')
    expect(v.clave('vps-a', 'tienda')).not.toBe(v.clave('vps-b', 'tienda'))
  })

  it('la barra no retrocede al ir llegando líneas', async () => {
    const v = await import('../src/lib/vivos.svelte')
    v.empezar('vps', 'web')
    const k = v.clave('vps', 'web')
    v.anotar(k, '{"event":"step","app":"web","step":"code","status":"ok","elapsed_s":1}')
    v.anotar(k, '{"event":"step","app":"web","step":"release","status":"ok","elapsed_s":2}')
    const a = v.ver('vps', 'web')!.progreso.avance
    // Una línea que no aporta no puede hacerla bajar.
    v.anotar(k, 'ruido que no es json')
    expect(v.ver('vps', 'web')!.progreso.avance).toBeGreaterThanOrEqual(a)
    v.olvidar(k)
  })

  it('perder el contacto NO se registra como fallo', async () => {
    // El despliegue sigue en el servidor y el cliente ya no sabe qué pasó.
    const v = await import('../src/lib/vivos.svelte')
    v.empezar('vps', 'otra')
    const k = v.clave('vps', 'otra')
    v.perder(k, 'se cortó la conexión')
    const x = v.ver('vps', 'otra')!
    expect(x.error).not.toBeNull()
    expect(x.resultado).toBeNull()   // ni fallido ni correcto: desconocido
    expect(v.enCurso('vps').length).toBe(0)
    v.olvidar(k)
  })
})
