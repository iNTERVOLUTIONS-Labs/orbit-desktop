/**
 * El despliegue: la pantalla estrella y la que justifica el proyecto.
 *
 * Lo que se comprueba aquí no es que se pinte, es que **no mienta**: una barra
 * que retrocede, un final que se confunde con otro, o seis recuentos agrupados
 * en dos son formas de contar una historia falsa sobre un servidor de
 * producción, y ninguna lanza una excepción.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'

import Despliegue from '../src/componentes/Despliegue.svelte'
import LoteVista from '../src/componentes/Lote.svelte'
import { finalDe, leerProgreso, ofreceRollback, pesos, PASOS, recuentos } from '../src/lib/despliegue'
import type { Despliegue as Obj, Lote } from '../src/lib/contrato'

import lote from '../src/lib/muestras/deploy-all.json'
import fallido from '../src/lib/muestras/deploy-fallido.json'
import correcto from '../src/lib/muestras/deploy-ok.json'

const FLUJO = [
  '{"event":"step","app":"web","step":"code","status":"start","elapsed_s":0}',
  '{"event":"step","app":"web","step":"code","status":"ok","elapsed_s":3}',
  '{"event":"step","app":"web","step":"release","status":"start","elapsed_s":3}',
  '{"event":"step","app":"web","step":"release","status":"ok","elapsed_s":5}',
  '{"event":"step","app":"web","step":"build","status":"start","elapsed_s":5}',
].join('\n')

describe('la barra', () => {
  it('no es lineal: el build pesa lo que dura', () => {
    // Seis pasos no valen un sexto cada uno. Una barra lineal se quedaría
    // clavada en el 33 % durante dos minutos y luego correría hasta el final.
    const p = pesos(null, null)
    expect(p[2]).toBeGreaterThan(0.5)
    expect(p.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 3)
  })

  it('con histórico, los pesos salen de lo medido', () => {
    const p = pesos(90, 120)
    expect(p[2]).toBeCloseTo(0.75, 2)
    expect(p.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 3)
  })

  it('un histórico absurdo no la desbarata', () => {
    expect(pesos(500, 10)).toEqual(pesos(null, null))
    expect(pesos(0, 0)).toEqual(pesos(null, null))
  })

  it('nunca retrocede', () => {
    // Una barra que retrocede destruye más confianza que cualquier error.
    const a = leerProgreso(FLUJO).avance
    expect(leerProgreso('', { anterior: a }).avance).toBe(a)
    expect(leerProgreso(FLUJO, { anterior: 0.95 }).avance).toBeGreaterThanOrEqual(0.95)
  })

  it('no llega al final antes de tiempo', () => {
    const casi = PASOS.slice(0, 5)
      .map((p) => `{"event":"step","app":"w","step":"${p.id}","status":"ok","elapsed_s":1}`)
      .join('\n')
    expect(leerProgreso(casi).avance).toBeLessThan(1)
  })
})

describe('el flujo de progreso', () => {
  it('cada paso conserva su cronómetro', () => {
    // En tres despliegues alguien aprende cuánto tarda su build, y al cuarto
    // sabe si algo va raro sin que nadie se lo diga.
    const p = leerProgreso(FLUJO)
    expect(p.pasos.find((x) => x.id === 'code')!.segundos).toBe(3)
    expect(p.pasos.find((x) => x.id === 'release')!.segundos).toBe(2)
    expect(p.pasos.find((x) => x.id === 'build')!.estado).toBe('haciendo')
  })

  it('un paso que no conocemos se AÑADE, no se descarta', () => {
    // Si algún día hay un séptimo paso, la pantalla lo enseña sin actualizar el
    // cliente.
    const p = leerProgreso(FLUJO + '\n{"event":"step","app":"web","step":"septimo","status":"start","elapsed_s":9}')
    expect(p.pasos.map((x) => x.id)).toContain('septimo')
  })

  it('un evento que no conocemos no rompe nada', () => {
    // Un cliente que se caiga porque el servidor añadió un evento es un cliente
    // que rompe cada vez que Orbit mejora.
    const p = leerProgreso(FLUJO + '\n{"event":"delfuturo","app":"web"}')
    expect(p.rotas).toBe(0)
    expect(p.pasos.find((x) => x.id === 'code')!.estado).toBe('hecho')
  })

  it('la prosa de stderr no cuenta como línea rota', () => {
    const p = leerProgreso('  ✔ Compilando\n' + FLUJO)
    expect(p.rotas).toBe(0)
  })

  it('una línea rota no tumba el despliegue', () => {
    const p = leerProgreso(FLUJO + '\n{"event":"step","app":')
    expect(p.rotas).toBe(1)
    expect(p.pasos.find((x) => x.id === 'code')!.estado).toBe('hecho')
  })

  it('los pasos se atribuyen por app, también con una sola', () => {
    // Es el campo que existe para que un lote mezcle niveles por el mismo
    // canal, y usarlo desde el principio evita tener dos analizadores.
    const mezclado = FLUJO + '\n{"event":"step","app":"otra","step":"nginx","status":"ok","elapsed_s":7}'
    const p = leerProgreso(mezclado, { app: 'web' })
    expect(p.pasos.find((x) => x.id === 'nginx')!.estado).toBe('pendiente')
  })
})

describe('los cuatro finales', () => {
  const obj = (p: Partial<Obj>): Obj => ({ ...(correcto as Obj), ...p })

  it('se distinguen los cuatro', () => {
    expect(finalDe(obj({ ok: true }))).toBe('bien')
    expect(finalDe(obj({ ok: true, recovered: true }))).toBe('recuperado')
    expect(finalDe(obj({ ok: false, rolled_back: true }))).toBe('revertido')
    expect(finalDe(obj({ ok: false, rolled_back: false }))).toBe('roto')
  })

  it('«recuperado» no es un error y no se pinta rojo', () => {
    // Pero tampoco igual que un éxito limpio: cuatro seguidos son un patrón que
    // alguien debería mirar.
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''),
      resultado: obj({ ok: true, recovered: true }),
    })
    expect(container.querySelector('.final--recuperado')).not.toBeNull()
    expect(container.querySelector('.final--roto')).toBeNull()
  })

  it('cuando revierte, lo PRIMERO que se dice es que la web sigue en pie', () => {
    // Ponerlo debajo del volcado del error sería enterrar la buena noticia.
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''),
      resultado: obj({ ok: false, rolled_back: true, previous: '20260805-041230', failed_step: 'service' }),
    })
    expect(container.querySelector('.titular')!.textContent).toContain('sigue en pie')
  })

  it('no se ofrece volver atrás si ya se volvió', () => {
    // Ofrecerlo sería ofrecer lo que acaba de pasar, y alguien haría clic
    // pensando que hace falta.
    expect(ofreceRollback(obj({ ok: false, rolled_back: true, previous: 'x' }))).toBe(false)
    expect(ofreceRollback(obj({ ok: false, rolled_back: false, previous: 'x' }))).toBe(true)
    // Ni si no hay a dónde volver.
    expect(ofreceRollback(obj({ ok: false, rolled_back: false, previous: null }))).toBe(false)
  })

  it('volver atrás no es la acción primaria', () => {
    // En un fallo, lo primero es entender. Y muchas veces volver atrás ni
    // siquiera es lo que se quiere: si el build falló, `current` no se movió.
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''),
      resultado: fallido as Obj,
    })
    const volver = [...container.querySelectorAll('button')]
      .find((b) => b.textContent?.includes('Volver a'))!
    expect(volver.className).toContain('secundaria')
  })

  it('un fallo dice dónde mirar según el paso que se rompió', () => {
    const { container } = render(Despliegue, {
      app: 'web', servidor: 'vps', progreso: leerProgreso(''),
      resultado: fallido as Obj,
    })
    // `fallido` se rompió en `build`, y lo que hay que mirar es el FINAL del
    // log, no el principio: el error está en las últimas veinte líneas.
    expect(container.textContent).toContain('FINAL del log')
  })
})

describe('el lote', () => {
  const l = lote as Lote

  it('los seis finales se cuentan por separado', () => {
    const c = recuentos(l)
    expect(c.length).toBe(6)
    expect(c.map((x) => x.id)).toEqual([
      'deployed', 'failed', 'unchanged', 'unreachable', 'gone', 'skipped',
    ])
  })

  it('«sin contacto» y «al día» NO son lo mismo', () => {
    // Es el fallo real que este contrato existe para que no se repita: un
    // remoto caído anunciado como «nada que hacer» cada cinco minutos.
    const c = recuentos(l)
    const mudo = c.find((x) => x.id === 'unreachable')!
    const dia = c.find((x) => x.id === 'unchanged')!
    expect(mudo.texto).not.toBe(dia.texto)
    expect(mudo.glifo).not.toBe(dia.glifo)
    expect(mudo.frase).toContain('NO es lo mismo')
  })

  it('la pantalla enseña las seis celdas, también las que están a cero', () => {
    // Quitar de la fila un recuento a cero haría que su aparición pasara
    // desapercibida justo el día que importa.
    const vacio: Lote = {
      ...l, apps: [], total: 0, deployed: 0, failed: 0,
      unchanged: 0, unreachable: 0, gone: 0, skipped: 0, ok: true,
    }
    const { container } = render(LoteVista, { lote: vacio, servidor: 'vps' })
    expect(container.querySelectorAll('.cuenta').length).toBe(6)
  })

  it('no hay ninguna celda que agrupe', () => {
    const { container } = render(LoteVista, { lote: l, servidor: 'vps' })
    const textos = [...container.querySelectorAll('.etiqueta')].map((n) => n.textContent?.trim())
    expect(textos.some((t) => /correctas|fallidas y/i.test(t ?? ''))).toBe(false)
    expect(container.querySelectorAll('.cuenta').length).toBe(6)
  })

  it('cada app dice qué le pasó, y las que no traen objeto dan su motivo', () => {
    const { container } = render(LoteVista, { lote: l, servidor: 'vps' })
    // La muda no tiene `result` y su motivo va en `error`: un null es una
    // respuesta, un objeto a medias no.
    expect(container.textContent).toContain('could not read Username')
    // Y la revertida dice lo primero que hace falta saber.
    expect(container.textContent).toContain('su web sigue en pie')
  })
})

describe('lo que sólo se vio mirando una captura', () => {
  it('los seis finales no comparten glifo', () => {
    // `failed` y `gone` compartían el `✕` Y el color: dos celdas idénticas
    // salvo por la palabra, que es justo lo que la regla de «el color nunca va
    // solo» existe para evitar.
    const glifos = recuentos(lote as Lote).map((c) => c.glifo)
    expect(new Set(glifos).size).toBe(6)
  })

  it('el veredicto se declina, sin «app(s)»', () => {
    const uno: Lote = { ...(lote as Lote), failed: 1, unreachable: 0, gone: 0, ok: false }
    const { container } = render(LoteVista, { lote: uno, servidor: 'vps' })
    expect(container.textContent).toContain('una app que necesita')
    expect(container.textContent).not.toContain('app(s)')
  })
})
