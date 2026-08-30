/**
 * El log y el diagnóstico: las dos pantallas que el contrato terminado
 * desbloqueó.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'

import Diagnostico from '../src/componentes/Diagnostico.svelte'
import VisorLog from '../src/componentes/VisorLog.svelte'
import { leerLog, type Doctor } from '../src/lib/contrato'

const FLUJO = [
  '{"schema":1,"event":"meta","app":"web","source":"nginx","unit":null,"since":null,"follow":false,"lines":80}',
  '{"event":"line","ts":"2026-08-29T14:02:11+02:00","stream":"access","text":"GET / 200"}',
  '{"event":"line","ts":null,"stream":"access","text":"sin marca"}',
  '{"event":"line","ts":"2026-08-29T14:03:01","stream":"error","text":"open() failed"}',
  '{"event":"end","lines":3,"truncated":false}',
].join('\n')

describe('el flujo NDJSON', () => {
  it('se lee entero y distingue el acceso del error', () => {
    const l = leerLog(FLUJO)
    expect(l.rotas).toBe(0)
    expect(l.meta?.schema).toBe(1)
    expect(l.lineas.map((x) => x.stream)).toEqual(['access', 'access', 'error'])
  })

  it('una línea rota no deja el log en blanco', () => {
    // Un byte mal puesto no puede convertir un log en una pantalla vacía, y
    // quien mira un log suele estar mirándolo porque algo va mal.
    const roto = FLUJO.replace('"text":"sin marca"}', '"text":"sin marc')
    const l = leerLog(roto)
    expect(l.rotas).toBe(1)
    expect(l.lineas.length).toBe(2)
  })

  it('un suceso que no conocemos se ignora sin ruido', () => {
    const l = leerLog(FLUJO + '\n{"event":"delfuturo","x":1}')
    expect(l.rotas).toBe(0)
    expect(l.lineas.length).toBe(3)
  })

  it('sin marca de tiempo es null, y no la hora de ahora', () => {
    const l = leerLog(FLUJO)
    expect(l.lineas[1]!.ts).toBeNull()
  })
})

describe('el visor de log', () => {
  it('deja separar el acceso del error', () => {
    // Es la razón de ser de la pantalla: la prosa no lo contesta.
    const { container } = render(VisorLog, { log: leerLog(FLUJO), app: 'web' })
    const botones = [...container.querySelectorAll('.filtros button')].map((b) =>
      b.textContent?.trim().split(/\s+/)[0],
    )
    expect(botones).toContain('access')
    expect(botones).toContain('error')
  })

  it('una línea sin marca de tiempo se pinta como desconocida, no en blanco', () => {
    const { container } = render(VisorLog, { log: leerLog(FLUJO), app: 'web' })
    const tiempos = [...container.querySelectorAll('.ts')].map((t) => t.textContent)
    expect(tiempos).toContain('·')
  })

  it('las líneas rotas se dicen, no se callan', () => {
    // Si se callaran, un log a medias se leería como uno completo.
    const l = leerLog(FLUJO.replace('"text":"sin marca"}', '"text":"sin marc'))
    const { container } = render(VisorLog, { log: l, app: 'web' })
    expect(container.textContent).toContain('no se ha entendido')
  })

  it('el tope se anuncia', () => {
    const l = leerLog(FLUJO.replace('"truncated":false', '"truncated":true'))
    const { container } = render(VisorLog, { log: l, app: 'web' })
    expect(container.textContent).toContain('tope de líneas')
  })

  it('cero líneas no es un error', () => {
    const l = leerLog('{"schema":1,"event":"meta","app":"web","source":"nginx","unit":null,"since":null,"follow":false,"lines":80}')
    const { container } = render(VisorLog, { log: l, app: 'web' })
    expect(container.textContent).toContain('No es un error')
  })

  it('el log no se anuncia línea a línea por voz', () => {
    // Un log a cuarenta líneas por segundo leído en voz alta inutiliza la
    // pantalla para quien usa un lector.
    const { container } = render(VisorLog, { log: leerLog(FLUJO), app: 'web' })
    expect(container.querySelector('[role="log"]')?.getAttribute('aria-live')).toBe('off')
  })
})

const DIAG: Doctor = {
  schema: 1,
  checks: [
    { id: 'pnpm', level: 'warn', message: 'pnpm no está instalado', fix: 'npm i -g pnpm', fixable: false },
    { id: 'vhost', level: 'error', message: 'a la app «web» le falta el vhost', fix: 'se regenera del descriptor', fixable: true },
    { id: 'nginx', level: 'ok', message: 'nginx OK', fix: null, fixable: false },
  ],
  summary: { ok: 1, warn: 1, error: 1 },
}

describe('el diagnóstico', () => {
  it('el botón cuenta sólo lo que el servidor arreglaría solo', () => {
    const { container } = render(Diagnostico, {
      doctor: DIAG, servidor: 'vps', alArreglar: () => {},
    })
    // Uno de los tres es `fixable`, aunque hay dos problemas.
    expect(container.querySelector('.arreglar')?.textContent?.trim()).toBe('Arreglar 1')
  })

  it('sin capacidad de arreglar no hay botón gris, hay una orden', () => {
    // Un botón deshabilitado invita a averiguar por qué no se puede pulsar, y
    // la respuesta es una frase que se podía haber leído sin el botón.
    const { container } = render(Diagnostico, {
      doctor: DIAG, servidor: 'vps', alArreglar: null,
    })
    expect(container.querySelector('.arreglar')).toBeNull()
    expect(container.textContent).toContain('orbit doctor --fix')
  })

  it('el texto del arreglo se enseña aunque no sea aplicable', () => {
    // Que Orbit no pueda hacerlo solo no significa que no se sepa qué hacer, y
    // ocultarlo dejaría a alguien con un problema y sin la frase que lo resuelve.
    const { container } = render(Diagnostico, { doctor: DIAG, servidor: 'vps' })
    expect(container.textContent).toContain('npm i -g pnpm')
    expect(container.textContent).toContain('a mano')
  })

  it('primero lo roto, luego lo que avisa, y al final lo que está bien', () => {
    // Ordenar por el orden en que Orbit comprueba pondría un «ok» entre dos
    // errores.
    const { container } = render(Diagnostico, { doctor: DIAG, servidor: 'vps' })
    const ids = [...container.querySelectorAll('.id')].map((n) => n.textContent)
    expect(ids).toEqual(['vhost', 'pnpm', 'nginx'])
  })
})
