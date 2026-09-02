/**
 * El alta de servidores.
 *
 * Sin esta clasificación, quien añade un servidor ve «error» y no sabe si es
 * **su clave, su red o su servidor**. Es la diferencia entre un producto y una
 * demo.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'

import AltaServidores from '../src/componentes/AltaServidores.svelte'
import type { Saludo } from '../src/lib/contrato'
import type { AliasSsh } from '../src/lib/puente'

const ALIAS: AliasSsh[] = [
  { alias: 'vps-ovh', hostname: '10.0.0.5', usuario: 'root', puerto: 22, salto: null, identidad: null },
  { alias: 'interno', hostname: '10.1.0.9', usuario: 'dave', puerto: 2222, salto: 'bastion', identidad: null },
]
const s = (p: Partial<Saludo>): Saludo => ({
  clase: 'ok', version: '1.3.6', contrato: 1, motivo: null,
  puede_operar: true, puede_leer: true,
  orden_de_instalacion: 'curl -fsSL https://…/install.sh | sudo bash',
  ...p,
})

function montar(saludos: Record<string, Saludo | null> = {}, extra: Record<string, unknown> = {}) {
  const alComprobar = vi.fn()
  const alUsar = vi.fn()
  const alAnadir = vi.fn()
  const alOlvidar = vi.fn()
  const alInstalar = vi.fn()
  const r = render(AltaServidores, {
    alias: ALIAS, propios: [], saludos, comprobando: null,
    alComprobar, alUsar, alAnadir, alOlvidar, alInstalar,
    ...extra,
  })
  return { ...r, alComprobar, alUsar, alAnadir, alOlvidar, alInstalar }
}

describe('enumerar no es visitar', () => {
  it('de entrada nadie está comprobado', () => {
    // Abrir esta pantalla no puede significar abrir cuarenta sesiones SSH.
    const { container } = montar()
    expect(container.querySelectorAll('.tenue').length).toBe(2)
    expect(container.textContent).toContain('sin comprobar')
  })

  it('y se dice en voz alta', () => {
    const { container } = montar()
    const t = (container.querySelector('.intro')?.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('no habla con ninguno')
  })

  it('preguntar por uno es un gesto aparte', () => {
    const { container, alComprobar } = montar()
    ;(container.querySelector('.acciones button') as HTMLButtonElement).click()
    expect(alComprobar).toHaveBeenCalledWith('vps-ovh')
  })
})

describe('qué se puede hacer con cada servidor', () => {
  it('sólo se puede abrir el que se puede leer', () => {
    const { container } = montar({
      'vps-ovh': s({}),
      interno: s({ clase: 'no-instalado', puede_operar: false, puede_leer: false }),
    })
    expect(container.querySelectorAll('.usar').length).toBe(1)
  })

  it('uno más nuevo se puede mirar, no operar', () => {
    // Negarse sería la peor forma de romper algo que todavía funcionaba.
    const { container } = montar({
      'vps-ovh': s({ clase: 'mas-nuevo', contrato: 9, puede_operar: false, puede_leer: true }),
    })
    expect(container.querySelector('.usar')).not.toBeNull()
    expect(container.textContent).toContain('mirar, no operar')
  })

  it('uno anterior al contrato es viejo, NO roto', () => {
    // Cero funcionalidad, y aun así el mensaje no lo llama fallo: es un
    // servidor sano y viejo.
    const { container } = montar({
      'vps-ovh': s({ clase: 'sin-contrato', contrato: null, puede_operar: false, puede_leer: false }),
    })
    expect(container.textContent).toContain('No es un fallo')
    expect(container.querySelector('.usar')).toBeNull()
  })
})

describe('sin Orbit', () => {
  it('se ofrece instalarlo, con un botón', () => {
    // Esta prueba decía lo contrario. Afirmaba que aquí NO podía haber un botón
    // de instalar porque sería la primera vez que el cliente escribe en el
    // servidor algo que no es una invocación de `orbit`.
    //
    // Se revirtió por dos motivos. El razonable: lo que iba a ejecutar quien
    // copiara el comando era exactamente lo mismo que ejecuta este botón, así
    // que la diferencia no era de seguridad sino de quién teclea — y a cambio,
    // un cliente de escritorio para una herramienta de despliegue no podía
    // poner en marcha un servidor.
    //
    // Y el vergonzoso: **el comando que se mandaba copiar no funcionaba**. Era
    // un `curl … | sudo bash` que me inventé sin leer el README, y `install.sh`
    // muere si no encuentra el fichero `orbit` a su lado. Por una tubería no hay
    // ninguno. O sea que la regla que se estaba defendiendo protegía una
    // instrucción rota.
    const { container, alInstalar } = montar({
      'vps-ovh': s({ clase: 'no-instalado', puede_operar: false, puede_leer: false, motivo: 'no hay orbit ahí' }),
    })
    const boton = [...container.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Instalar Orbit',
    ) as HTMLButtonElement
    expect(boton).toBeDefined()
    boton.click()
    expect(alInstalar).toHaveBeenCalledWith('vps-ovh')
  })

  it('y a un Orbit viejo se le ofrece actualizar, que no es lo mismo', () => {
    // Un Orbit anterior al contrato es un servidor SANO y viejo, no uno roto.
    // El botón dice «actualizar» porque eso es lo que hace.
    const { container } = montar({
      'vps-ovh': s({ clase: 'sin-contrato', contrato: 0, puede_operar: false, puede_leer: false }),
    })
    const botones = [...container.querySelectorAll('button')].map((b) => b.textContent?.trim())
    expect(botones).toContain('Actualizar Orbit')
    expect(botones).not.toContain('Instalar Orbit')
  })

  it('el comando roto ya no está en ninguna parte', () => {
    // `curl … | sudo bash` no instala Orbit: install.sh necesita el fichero
    // `orbit` a su lado. Que no vuelva por copiar y pegar de un sitio viejo.
    const { container } = montar({
      'vps-ovh': s({ clase: 'no-instalado', puede_operar: false, puede_leer: false }),
    })
    const t = container.textContent ?? ''
    expect(t).not.toContain('curl')
    expect(t).not.toContain('| sudo bash')
  })
})

describe('cada caso dice qué hacer', () => {
  it('sudo pidiendo contraseña se distingue de «no llegué»', () => {
    // Confundirlos manda a quien lo mire a revisar su red cuando el problema es
    // su sudoers.
    const { container } = montar({
      'vps-ovh': s({ clase: 'sin-privilegios', puede_operar: false, puede_leer: false }),
    })
    expect(container.textContent).toContain('sudo sin contraseña')
    expect(container.textContent).not.toContain('No se llega')
  })

  it('la clave de host cambiada no ofrece continuar', () => {
    const { container } = montar({
      'vps-ovh': s({ clase: 'clave-de-host-cambiada', puede_operar: false, puede_leer: false }),
    })
    expect(container.querySelector('.usar')).toBeNull()
    expect(container.textContent).toContain('otra máquina')
  })

  it('los siete estados tienen su propio color', () => {
    const clases: Array<Saludo['clase']> = [
      'ok', 'mas-nuevo', 'sin-contrato', 'no-instalado',
      'sin-privilegios', 'no-se-llega', 'clave-de-host-cambiada',
    ]
    for (const c of clases) {
      const { container } = montar({ 'vps-ovh': s({ clase: c }) })
      expect(container.querySelector(`.saludo--${c}`), c).not.toBeNull()
    }
  })
})

describe('un salto se anuncia', () => {
  it('porque cambia lo que se puede prometer sobre la latencia', () => {
    // Por un bastión, el saludo se paga dos veces.
    const { container } = montar()
    expect(container.textContent).toContain('por bastion')
  })
})

describe('sin ningún servidor', () => {
  // El fallo que rompía la aplicación al abrirla por primera vez: los
  // servidores salían SÓLO del ~/.ssh/config, así que quien no tuviera ese
  // fichero —en Windows casi nadie— veía una lista vacía sin ninguna salida.
  it('no se queda en blanco: ofrece añadir uno', () => {
    const { container, alAnadir } = montar({}, { alias: [], propios: [] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Añade tu primer servidor')

    const boton = [...container.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Añadir un servidor',
    ) as HTMLButtonElement
    expect(boton).toBeDefined()
    boton.click()
    expect(alAnadir).toHaveBeenCalled()
  })

  it('y dice que el ~/.ssh/config es un extra, no un requisito', () => {
    const { container } = montar({}, { alias: [], propios: [] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('saldrán aquí solos')
  })
})

describe('los servidores añadidos a mano', () => {
  const PROPIO = {
    alias: 'produccion', host: '203.0.113.10', usuario: 'root',
    puerto: 22, clave: null, binario: null,
  }

  it('salen junto a los del fichero, y se distingue de dónde viene cada uno', () => {
    // Se desapuntan de forma distinta: uno se quita de aquí y el otro hay que
    // editarlo en el ~/.ssh/config. Confundirlos manda a alguien a buscar un
    // botón que no existe.
    const { container } = montar({}, { propios: [PROPIO] })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('produccion')
    expect(t).toContain('vps-ovh')
    expect(t).toContain('añadido a mano')
    expect(container.querySelectorAll('.marca')).toHaveLength(1)
  })

  it('sólo los propios se pueden quitar', () => {
    const { container, alOlvidar } = montar({}, { propios: [PROPIO] })
    const quitar = [...container.querySelectorAll('button')].filter(
      (b) => b.textContent?.trim() === 'quitar',
    )
    expect(quitar).toHaveLength(1)
    ;(quitar[0] as HTMLButtonElement).click()
    expect(alOlvidar).toHaveBeenCalledWith('produccion')
  })

  it('el puerto sólo se enseña cuando no es el de siempre', () => {
    const { container } = montar({}, { propios: [{ ...PROPIO, puerto: 2222 }] })
    expect(container.textContent).toContain('root@203.0.113.10:2222')
  })
})
