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

function montar(saludos: Record<string, Saludo | null> = {}) {
  const alComprobar = vi.fn()
  const alUsar = vi.fn()
  const r = render(AltaServidores, {
    alias: ALIAS, saludos, comprobando: null, alComprobar, alUsar,
  })
  return { ...r, alComprobar, alUsar }
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
    expect(container.querySelector('.intro')?.textContent).toContain('no habla con ninguno')
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
  it('se ofrece la orden para COPIAR, no un botón de instalar', () => {
    // Instalar Orbit desde aquí sería la primera vez que este cliente escribe
    // en el servidor algo que no es una invocación de `orbit`, y la regla nº 1
    // no admite un «pero es el instalador».
    const { container } = montar({
      'vps-ovh': s({ clase: 'no-instalado', puede_operar: false, puede_leer: false, motivo: 'no hay orbit ahí' }),
    })
    expect(container.querySelector('.instalar')?.textContent).toContain('install.sh')
    const botones = [...container.querySelectorAll('button')].map((b) => b.textContent?.trim())
    expect(botones).not.toContain('instalar')
    expect(botones).not.toContain('Instalar')
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
