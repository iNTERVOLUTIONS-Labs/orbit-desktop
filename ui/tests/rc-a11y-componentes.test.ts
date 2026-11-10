/**
 * Lo que arregló la ronda de accesibilidad del candidato, fijado.
 *
 * Cada bloque es un hallazgo de `adv-a11y-recorrido` (usuario nuevo, NVDA y
 * sólo teclado) convertido en comportamiento: lo que se oye, adónde va el foco
 * y de quién es cada tecla.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'
import { tick } from 'svelte'

import ListaApps from '../src/componentes/ListaApps.svelte'
import ChipEstado from '../src/componentes/ChipEstado.svelte'
import Barras from '../src/componentes/Barras.svelte'
import InstalarOrbit from '../src/componentes/InstalarOrbit.svelte'
import AltaServidores from '../src/componentes/AltaServidores.svelte'
import AnadirServidor from '../src/componentes/AnadirServidor.svelte'
import AsistenteNueva from '../src/componentes/AsistenteNueva.svelte'
import Fallo from '../src/componentes/Fallo.svelte'
import Entorno from '../src/componentes/Entorno.svelte'
import { consejoSsh } from '../src/lib/consejos'
import { borradorNuevo } from '../src/lib/asistente'
import type { App, Estado, Saludo } from '../src/lib/contrato'

/** Lo que diría un lector de pantalla: el texto sin lo que está oculto. */
function leible(el: Element): string {
  const c = el.cloneNode(true) as Element
  c.querySelectorAll('[aria-hidden="true"]').forEach((x) => x.remove())
  return (c.textContent ?? '').replace(/\s+/g, ' ').trim()
}

const espera = () => new Promise((r) => setTimeout(r, 0))

// ── La lista de apps y el teclado ───────────────────────────────────────────

const estado = (o: Partial<Estado> = {}): Estado =>
  ({
    service: 'active', port: 3000, ssl: true, cert_days: 60, maintenance: false,
    served: true, autodeploy: false, queue: false, releases: 2, last_deploy: null, last_deploy_sha: null,
    ...o,
  }) as Estado
const app = (name: string, o: Partial<Estado> = {}): App =>
  ({ name, domain: `${name}.test`, type: 'node', aliases: [], state: estado(o) }) as unknown as App

describe('la lista: de quién es cada tecla', () => {
  let scroll: ReturnType<typeof vi.fn>
  beforeEach(() => {
    scroll = vi.fn()
    Element.prototype.scrollIntoView = scroll as unknown as typeof Element.prototype.scrollIntoView
  })
  afterEach(() => {
    delete (Element.prototype as Partial<Element>).scrollIntoView
  })

  function montar(apps = [app('tienda'), app('api'), app('blog')]) {
    const elegir = vi.fn()
    // Un botón FUERA de la lista, como la pestaña «log» del detalle.
    const fuera = document.createElement('button')
    fuera.textContent = 'log'
    document.body.appendChild(fuera)
    const r = render(ListaApps, { apps, servidor: 'vps', alElegir: elegir })
    return { ...r, elegir, fuera }
  }

  afterEach(() => document.querySelectorAll('body > button').forEach((b) => b.remove()))

  it('una flecha en un botón de fuera es de ese botón: no cambia la app', async () => {
    const { elegir, fuera } = montar()
    fuera.focus()
    const ev = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true })
    fuera.dispatchEvent(ev)
    expect(elegir).not.toHaveBeenCalled()
    expect(ev.defaultPrevented).toBe(false)
  })

  it('una «j» en un botón de fuera tampoco', async () => {
    const { elegir, fuera } = montar()
    fuera.focus()
    await fireEvent.keyDown(fuera, { key: 'j' })
    expect(elegir).not.toHaveBeenCalled()
  })

  it('en el <body>, la flecha desplaza la página: ni se come ni elige', async () => {
    const { elegir } = montar()
    const ev = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true })
    document.body.dispatchEvent(ev)
    expect(ev.defaultPrevented).toBe(false)
    expect(elegir).not.toHaveBeenCalled()
  })

  it('en el <body>, «j» sí elige: no le quita nada a nadie', async () => {
    const { elegir } = montar()
    await fireEvent.keyDown(document.body, { key: 'j' })
    expect(elegir).toHaveBeenCalledWith(expect.objectContaining({ name: 'tienda' }))
  })

  it('en la caja de filtrar, la flecha elige y el foco se queda en la caja', async () => {
    const { elegir, getByRole } = montar()
    const caja = getByRole('searchbox')
    caja.focus()
    await fireEvent.keyDown(caja, { key: 'ArrowDown' })
    expect(elegir).toHaveBeenCalledWith(expect.objectContaining({ name: 'tienda' }))
    expect(document.activeElement).toBe(caja)
  })

  it('en la caja, «j» se escribe y no elige', async () => {
    const { elegir, getByRole } = montar()
    const caja = getByRole('searchbox')
    caja.focus()
    await fireEvent.keyDown(caja, { key: 'j' })
    expect(elegir).not.toHaveBeenCalled()
  })

  it('en una fila, la flecha elige la siguiente y el foco va con ella', async () => {
    const elegir = vi.fn()
    const { container } = render(ListaApps, {
      apps: [app('tienda'), app('api'), app('blog')], servidor: 'vps', seleccionada: 'tienda', alElegir: elegir,
    })
    const botones = [...container.querySelectorAll('tbody button')] as HTMLButtonElement[]
    botones[0]!.focus()
    await fireEvent.keyDown(botones[0]!, { key: 'ArrowDown' })
    expect(elegir).toHaveBeenCalledWith(expect.objectContaining({ name: 'api' }))
    expect(document.activeElement).toBe(botones[1])
  })

  it('un nombre con punto o que empieza por número se trae a la vista', async () => {
    // `CSS.escape` en un `getElementById` no encontraba la fila.
    const { getByRole } = montar([app('1web'), app('mi.web')])
    const caja = getByRole('searchbox')
    caja.focus()
    await fireEvent.keyDown(caja, { key: 'ArrowDown' })
    expect(scroll).toHaveBeenCalled()
  })

  it('dos apps con el mismo nombre se pintan las dos, sin romper', () => {
    const { container } = montar([app('web'), app('web'), app('api')])
    expect(container.querySelectorAll('tbody tr')).toHaveLength(3)
  })

  it('la fila elegida se marca con aria-current, no con aria-selected', () => {
    const { container } = render(ListaApps, { apps: [app('a'), app('b')], servidor: 'vps', seleccionada: 'b' })
    expect(container.querySelector('[aria-selected]')).toBeNull()
    const filas = [...container.querySelectorAll('tbody tr')]
    expect(filas[1]!.getAttribute('aria-current')).toBe('true')
    expect(filas[0]!.hasAttribute('aria-current')).toBe(false)
  })

  it('la cuenta de las que no se ven cae en una región viva que ya existía', async () => {
    const { container, getByRole } = render(ListaApps, { apps: [app('tienda'), app('api')], servidor: 'vps' })
    const region = container.querySelector('.ocultas')!
    expect(region.getAttribute('role')).toBe('status')
    expect(region.textContent!.trim()).toBe('')
    await fireEvent.input(getByRole('searchbox'), { target: { value: 'tienda' } })
    // El MISMO nodo, con el texto nuevo: así es como se anuncia.
    expect(container.querySelector('.ocultas')).toBe(region)
    expect(region.textContent).toContain('1 de 2')
  })
})

// ── El estado se oye, no sólo se ve ─────────────────────────────────────────

describe('el chip de estado', () => {
  it.each([
    [{ served: true, service: null }, 'no aplica'],
    [{ served: true, service: 'active' }, 'activo'],
  ])('tiene rol de imagen y un nombre con palabra (%#)', (o, palabra) => {
    const { container } = render(ChipEstado, { estado: estado(o as Partial<Estado>) })
    const chip = container.querySelector('.chip')!
    expect(chip.getAttribute('role')).toBe('img')
    expect(chip.getAttribute('aria-label')).toContain(palabra)
  })
})

describe('la gráfica de barras', () => {
  const datos = [{ x: '10:00', y: 4 }, { x: '11:00', y: null }, { x: '12:00', y: 9 }]

  it('las cifras se alcanzan con el teclado, y el hueco dice «no se sabe»', () => {
    const { container } = render(Barras, { datos, etiqueta: 'Peticiones por hora' })
    const tabla = container.querySelector('details table')!
    expect(container.querySelector('details summary')).not.toBeNull()
    const celdas = [...tabla.querySelectorAll('tbody td')].map((c) => c.textContent?.trim())
    expect(celdas).toEqual(['10:00', '4', '11:00', 'no se sabe', '12:00', '9'])
  })

  it('el nombre se dice una vez, y el resumen dice el pico y los huecos', () => {
    const { container } = render(Barras, { datos, etiqueta: 'Peticiones por hora' })
    expect(container.querySelector('figcaption')).toBeNull()
    const svg = container.querySelector('svg')!
    const resumen = container.querySelector(`#${CSS.escape(svg.getAttribute('aria-describedby')!)}`)!
    expect(resumen.textContent).toContain('Máximo: 9 a las 12:00')
    expect(resumen.textContent).toContain('Una hora no se sabe')
  })
})

describe('instalar Orbit: lo que no se oía', () => {
  const REQ = {
    git: true, root: false, sudo_sin_contrasena: false, ya_instalado: false,
    sistema: 'ubuntu-24.04', puede: false, impedimentos: [], avisos: [], pasos: ['sudo bash install.sh'],
  }
  const props = { alias: 'produccion', alInstalar: vi.fn(), alCancelar: vi.fn(), alCerrar: vi.fn() }

  it('un requisito que falta dice «falta», no lo afirma', () => {
    const { container } = render(InstalarOrbit, { ...props, requisitos: REQ })
    const items = [...container.querySelectorAll('.chequeo li')].map(leible)
    expect(items[0]).toMatch(/^Cumple: git/)
    expect(items[1]).toMatch(/^Falta: sudo sin contraseña/)
  })

  it('cada hito dice si está hecho, en curso o pendiente', () => {
    const { container } = render(InstalarOrbit, {
      ...props, instalando: true, salida: '1/13  Uno\n2/13  Dos\n3/13  Tres',
    })
    const hitos = [...container.querySelectorAll('.hito')]
    expect(leible(hitos[0]!)).toMatch(/hecho$/)
    expect(leible(hitos[2]!)).toMatch(/en curso$/)
    expect(leible(hitos[3]!)).toMatch(/pendiente$/)
    expect(hitos[2]!.getAttribute('aria-current')).toBe('step')
  })

  it('el final se anuncia en una región que ya estaba', async () => {
    const r = render(InstalarOrbit, { ...props, instalando: true, salida: '1/13  Uno' })
    const region = r.container.querySelector('[role="status"]')!
    await r.rerender({ ...props, instalando: false, resultado: { codigo: 0, salida: '', version: '1.5.1' } })
    expect(r.container.querySelector('[role="status"]')).toBe(region)
    expect(region.textContent).toContain('Orbit 1.5.1 está instalado')
  })
})

// ── La primera vez: un servidor al que no se llega ──────────────────────────

describe('qué hacer cuando ssh no llega', () => {
  it.each([
    ['no he podido lanzar ssh: program not found', 'sin-ssh', /Cliente OpenSSH/],
    ['Permission denied (publickey).', 'clave-rechazada', /ssh-add/],
    ['Could not open a connection to your authentication agent.', 'sin-agente', /ssh-agent/],
    ['ssh: Could not resolve hostname srv.ejemplo: Name or service not known', 'no-resuelve', /IP/],
    ['ssh: connect to host 10.0.0.5 port 22: Connection timed out', 'no-contesta', /cortafuegos/],
    ['ssh: connect to host 10.0.0.5 port 22: Connection refused', 'rechaza', /puerto/],
    ['@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@', 'clave-cambiada', /ssh-keygen -R/],
    ['Host key verification failed.', 'host-desconocido', /ssh vps/],
    ['kex_exchange_identification: Connection closed by remote host', 'general', /ssh vps/],
  ])('«%s» → %s', (texto, caso, paso) => {
    const c = consejoSsh(texto, 'vps', true)
    expect(c.caso).toBe(caso)
    expect(c.hacer).toMatch(paso)
  })

  it('fuera de Windows no manda a la Configuración de Windows', () => {
    expect(consejoSsh('no he podido lanzar ssh: No such file', 'vps', false).hacer).toContain('openssh-client')
  })
})

describe('el alta de servidores', () => {
  const props = {
    alias: [{ alias: 'vps', hostname: '10.0.0.5', usuario: 'root', puerto: 22, salto: null, identidad: null }],
    propios: [], comprobando: null,
    alComprobar: vi.fn(), alUsar: vi.fn(), alAnadir: vi.fn(), alOlvidar: vi.fn(), alInstalar: vi.fn(),
  }
  const noSeLlega = (motivo: string): Saludo => ({
    clase: 'no-se-llega', version: null, contrato: null, motivo,
    puede_operar: false, puede_leer: false, orden_de_instalacion: '',
  })

  it('el veredicto se anuncia en la región de su fila, que ya existía, y dice de quién es', async () => {
    const r = render(AltaServidores, { ...props, saludos: {} })
    const region = r.container.querySelector('.veredicto')!
    expect(region.getAttribute('role')).toBe('status')
    await r.rerender({ ...props, saludos: { vps: noSeLlega('Permission denied (publickey).') } })
    expect(r.container.querySelector('.veredicto')).toBe(region)
    expect(region.textContent).toContain('vps')
    expect(region.textContent).toContain('No se llega')
  })

  it('«No se llega» da un paso siguiente y deja el stderr plegado, como texto', () => {
    const { container } = render(AltaServidores, {
      ...props, saludos: { vps: noSeLlega('Permission denied (publickey).<b>x</b>') },
    })
    const detalle = container.querySelector('.detalle')!
    expect(detalle.querySelector('.hacer')?.textContent).toContain('ssh-add')
    const crudo = detalle.querySelector('details pre')!
    expect(crudo.closest('details')!.open).toBe(false)
    expect(crudo.textContent).toContain('<b>x</b>')
    expect(crudo.querySelector('b')).toBeNull()
  })
})

describe('la pantalla de fallo', () => {
  it('sin ssh en Windows dice cómo instalarlo, y la frase de arriba es nuestra', () => {
    const { container } = render(Fallo, {
      error: { clase: 'no-se-pudo-lanzar', mensaje: 'no he podido lanzar ssh: program not found' },
      alias: 'vps',
    })
    const titulo = container.querySelector('.mensaje')!.textContent!
    expect(titulo).toMatch(/^[A-ZÁÉÍÓÚ]/)
    expect(titulo).not.toContain('program not found')
    expect(container.querySelector('details pre')?.textContent).toContain('program not found')
  })

  it('ya no manda a la persona fuera: «no es cosa de Orbit Desktop» no está', () => {
    const { container } = render(Fallo, {
      error: { clase: 'no-llego', mensaje: 'no he llegado al servidor: Connection reset' },
      alias: 'vps',
    })
    expect(container.textContent).not.toContain('no es cosa de Orbit Desktop')
    expect(container.textContent).toContain('ssh vps')
  })

  it('un mensaje del núcleo va como detalle, debajo de una frase en castellano', () => {
    const { container } = render(Fallo, {
      error: { clase: 'orbit-no-esta', mensaje: 'no hay un orbit ejecutable en /usr/local/bin/orbit' },
      alias: 'vps',
    })
    expect(container.querySelector('.mensaje')?.textContent).toBe('No encuentro Orbit en vps.')
    expect(container.querySelector('.cita')?.textContent).toContain('no hay un orbit ejecutable')
  })
})

describe('la pantalla de fallo: las clases nuevas', () => {
  const pinta = (clase: string, mensaje: string, detalle: string | null = null) =>
    render(Fallo, { error: { clase, mensaje, detalle }, alias: 'vps' }).container

  it('«forma» manda a comparar versiones y enseña la orden como detalle', () => {
    const c = pinta('forma', 'status --json: falta .apps[0].state')
    expect(c.querySelector('.que-hacer')?.textContent).toContain('orbit version')
    expect(c.querySelector('.cita')?.textContent).toContain('status --json')
  })

  it('«forma» de un alias que no existe no habla de versiones', () => {
    const c = pinta('forma', '«x» no es ninguno de tus servidores')
    expect(c.querySelector('.mensaje')?.textContent).toContain('no está entre tus servidores')
    expect(c.textContent).not.toContain('orbit version')
  })

  it('«al-pintar» pide el detalle para un issue', () => {
    const c = pinta('al-pintar', 'TypeError: x is undefined', 'at Vista.svelte:12')
    expect(c.querySelector('.que-hacer')?.textContent).toContain('issue')
    expect(c.querySelector('details summary')?.textContent).toBe('El detalle')
  })

  it('«tarde» no invita a repetir a ciegas una orden que cambia algo', () => {
    const c = pinta('tarde', 'el servidor no ha contestado en 30s; no sé si el despliegue llegó a hacerse')
    expect(c.querySelector('.que-hacer')?.textContent).toContain('mira cómo ha quedado antes de repetirla')
  })
})

// ── Los formularios ─────────────────────────────────────────────────────────

describe('añadir un servidor con el teclado', () => {
  const montar = () => render(AnadirServidor, { yaUsados: [], alGuardar: vi.fn(), alCerrar: vi.fn() })

  it('al abrirse, el foco está en el primer campo', async () => {
    const { container } = montar()
    await tick()
    expect(document.activeElement).toBe(container.querySelector('#ns-host'))
  })

  it('enviar vacío lleva el foco al primer campo que falla y avisa de cuántos', async () => {
    const { container } = montar()
    ;(container.querySelector('#ns-usuario') as HTMLInputElement).focus()
    await fireEvent.input(container.querySelector('#ns-usuario')!, { target: { value: '' } })
    await fireEvent.submit(container.querySelector('form')!)
    await espera()
    const host = container.querySelector('#ns-host') as HTMLInputElement
    expect(document.activeElement).toBe(host)
    expect(host.getAttribute('aria-invalid')).toBe('true')
    expect(container.querySelector('#' + host.getAttribute('aria-describedby'))?.textContent).toContain('Falta la dirección')
    expect(container.querySelector('.sr[role="status"]')?.textContent).toMatch(/Hay \d campos por corregir/)
  })

  it('el formulario tiene nombre, y el alias propuesto se explica en castellano', async () => {
    const { container, getByRole } = montar()
    expect(getByRole('form', { name: 'Añadir un servidor' })).toBeTruthy()
    await fireEvent.input(container.querySelector('#ns-host')!, { target: { value: 'srv.ejemplo.com' } })
    expect(container.textContent).toContain('Propuesto a partir de la dirección')
    expect(container.textContent).not.toContain('Salido de')
  })
})

describe('el asistente de web nueva con el teclado', () => {
  const props = { servidor: 'vps', alCrear: vi.fn(), alCerrar: vi.fn(), alResolver: vi.fn() }

  it('al abrirse, el foco está en su título', async () => {
    const { container } = render(AsistenteNueva, props)
    await tick()
    expect(document.activeElement).toBe(container.querySelector('h3'))
  })

  it('el paso en curso lleva aria-current="step"', () => {
    const { container } = render(AsistenteNueva, props)
    const actual = container.querySelectorAll('[aria-current="step"]')
    expect(actual).toHaveLength(1)
    expect(actual[0]!.textContent).toContain('Origen')
  })

  it('«Siguiente» a medias lleva el foco al primer campo que falla, con su error enlazado', async () => {
    const { container, getByRole } = render(AsistenteNueva, props)
    await fireEvent.click(getByRole('button', { name: 'Siguiente' }))
    await espera()
    const foco = document.activeElement as HTMLInputElement
    expect(foco.tagName).toBe('INPUT')
    expect(foco.getAttribute('aria-invalid')).toBe('true')
    const ids = foco.getAttribute('aria-describedby')!.split(' ')
    const dice = ids.map((i) => container.querySelector('#' + CSS.escape(i))?.textContent ?? '').join(' ')
    expect(dice).toContain('Falta el repositorio')
  })

  it('al cambiar de paso, el foco va al título del paso nuevo y no al <body>', async () => {
    const b = { ...borradorNuevo(), repo: 'usuario/tienda', nombre: 'tienda', dominio: 'tienda.ejemplo.com' }
    const { container, getByRole } = render(AsistenteNueva, { ...props, inicial: b, pasoInicial: 'Detección' })
    await fireEvent.click(getByRole('button', { name: 'Siguiente' }))
    await espera()
    expect(document.activeElement).toBe(container.querySelector('.paso-titulo'))
    expect(document.activeElement?.textContent).toContain('Dominio')
  })
})

// ── Los secretos ────────────────────────────────────────────────────────────

describe('el entorno', () => {
  it('una respuesta que llega después de perder el foco no se pinta', async () => {
    let soltar!: (v: string) => void
    const { container } = render(Entorno, {
      entorno: { schema: 1, app: 'api', keys: ['DB_PASSWORD'] },
      app: 'api', servidor: 'alfa',
      pedirValor: () => new Promise<string>((r) => (soltar = r)),
    })
    await fireEvent.click(container.querySelector('button')!)
    window.dispatchEvent(new Event('blur'))
    soltar('hunter2')
    await espera()
    expect(container.textContent).not.toContain('hunter2')
    // Y el botón vuelve: no se queda «pidiendo» para siempre.
    expect((container.querySelector('button') as HTMLButtonElement).disabled).toBe(false)
  })

  it('una clave repetida se enseña dos veces, y revelar una no revela la otra', async () => {
    const { container } = render(Entorno, {
      entorno: { schema: 1, app: 'api', keys: ['DB_URL', 'SECRET', 'DB_URL'] },
      app: 'api', servidor: 'alfa', pedirValor: async () => 'x',
    })
    expect(container.querySelectorAll('li.fila')).toHaveLength(3)
    expect(container.querySelectorAll('.repetida')).toHaveLength(2)
    await fireEvent.click(container.querySelectorAll('button')[0]!)
    await espera()
    expect(container.querySelectorAll('.valor')).toHaveLength(1)
  })
})
