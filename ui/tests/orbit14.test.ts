/**
 * Lo que trajeron Orbit 1.3.8 a 1.4.8, y lo que NO se puede romper al traerlo.
 *
 * Las respuestas son las del servidor falso del núcleo, sacadas del Orbit de
 * verdad ejecutado en el banco. Dos mitades, como en `crates/orbit-client/
 * tests/orbit14.rs`:
 *
 *  1. Lo nuevo se pinta: la congelación con su quién y su porqué, la CI, el
 *     certificado de origen, las once celdas de la pasada, las variables que
 *     faltan antes de desplegar y las tareas.
 *  2. El silencio de un servidor viejo no se convierte en un valor. Un 1.3.6
 *     no dice `locked`, y eso no es «no está congelada».
 */
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render } from '@testing-library/svelte'
import { readFileSync } from 'node:fs'
import { tick } from 'svelte'

import Lote from '../src/componentes/Lote.svelte'
import Pasada from '../src/componentes/Pasada.svelte'
import ListaApps from '../src/componentes/ListaApps.svelte'
import FacetaSsl from '../src/componentes/FacetaSsl.svelte'
import DetalleApp from '../src/componentes/DetalleApp.svelte'
import HojaDeDespliegue from '../src/componentes/HojaDeDespliegue.svelte'
import Despliegue from '../src/componentes/Despliegue.svelte'
import {
  argvDeCongelar, argvDeDescongelar, argvDeDespliegue, argvDeEsperarCi,
  faltanVariables, finalDe, finalesPosibles, leerProgreso, necesitanMirada,
  ordenLegible, problemaDelMotivo, problemaDelValor, recuentos, retenidas, OCULTO,
} from '../src/lib/despliegue'
import {
  obligatoriasSinValor, sabe, semver,
  type App, type AppInfo, type ComprobacionDeEntorno, type Despliegue as Obj,
  type Lista, type Lote as L, type Tareas,
} from '../src/lib/contrato'

import lista14 from '../src/lib/muestras/list-1.4.json'
import listaVieja from '../src/lib/muestras/list.json'
import info14 from '../src/lib/muestras/info-1.4.json'
import puertas from '../src/lib/muestras/deploy-all-puertas.json'
import loteViejo from '../src/lib/muestras/deploy-all.json'
import congelada from '../src/lib/muestras/deploy-congelada.json'
import faltan from '../src/lib/muestras/deploy-faltan-variables.json'
import comprobacion from '../src/lib/muestras/env-check.json'
import cron from '../src/lib/muestras/cron.json'

const apps14 = (lista14 as Lista).apps
const app = (n: string): App => apps14.find((a) => a.name === n)!
const texto = (c: HTMLElement) => (c.textContent ?? '').replace(/\s+/g, ' ')

describe('qué sabe hacer un servidor', () => {
  it('las variables por la entrada, desde la 1.4.9', () => {
    expect(sabe('1.4.8', 'entornoPorEntrada')).toBe(false)
    expect(sabe('1.4.9', 'entornoPorEntrada')).toBe(true)
  })

  it('sale de la versión de Orbit, por componentes y no como texto', () => {
    expect(sabe('1.3.6', 'congelar')).toBe(false)
    expect(sabe('1.4.3', 'congelar')).toBe(true)
    expect(sabe('1.4.8', 'tipoDeCertificado')).toBe(true)
    expect(sabe('1.4.6', 'tipoDeCertificado')).toBe(false)
    expect(sabe('1.3.8', 'entornoDeclarado')).toBe(true)
    expect(sabe('1.3.7', 'entornoDeclarado')).toBe(false)
    expect(sabe('1.10.0', 'tareas')).toBe(true)
    expect(sabe('2.0.0-rc1', 'congelar')).toBe(true)
  })

  it('una versión que no se entiende no ofrece nada', () => {
    expect(semver('desarrollo')).toBeNull()
    expect(sabe('desarrollo', 'congelar')).toBe(false)
    expect(sabe(null, 'congelar')).toBe(false)
  })
})

describe('las órdenes que se enseñan son las que se ejecutan', () => {
  // El mismo fichero contra el que se compara el catálogo de Rust.
  const f = JSON.parse(readFileSync('../tests/contrato/ordenes.json', 'utf8'))
  for (const c of f.casos) {
    it(c.nombre, () => {
      const o = c.orden
      const argv =
        o.tipo === 'desplegar' ? argvDeDespliegue(o.app, o.entorno, o.forzar, o.por_entrada ?? false)
        : o.tipo === 'congelar' ? argvDeCongelar(o.app, o.motivo)
        : o.tipo === 'descongelar' ? argvDeDescongelar(o.app)
        : argvDeEsperarCi(o.app, o.esperar)
      expect(argv).toEqual(c.argv)
      for (const e of o.entorno ?? []) {
        expect(argv.join(' ')).not.toContain(e.valor)
      }
    })
  }

  it('la orden legible deja ver la clave y nunca el valor', () => {
    const l = ordenLegible(argvDeDespliegue('tienda', [{ clave: 'DB_PASSWORD', valor: 'hunter2' }]))
    expect(l).toContain(`DB_PASSWORD=${OCULTO}`)
    expect(l).not.toContain('hunter2')
  })

  it('lo que el servidor rechazaría se dice junto al campo, sin repetir el valor', () => {
    expect(problemaDelValor('a\nb')).toContain('saltos de línea')
    expect(problemaDelValor("it's")).toContain('comilla simple')
    expect(problemaDelValor('p@ss w"ord;$(id)')).toBeNull()
    expect(problemaDelValor("it's")).not.toContain("it's")
    expect(problemaDelMotivo('x'.repeat(201))).toContain('200')
    expect(problemaDelMotivo('dos\nlíneas')).toContain('una línea')
    expect(problemaDelMotivo('á'.repeat(200))).toBeNull()
  })
})

describe('los finales de un despliegue que no son fallos', () => {
  it('una app congelada se niega antes de empezar, y eso no es «roto»', () => {
    expect(finalDe(congelada as Obj)).toBe('negado')
  })

  it('una obligatoria que falta para antes del build, y se dice cuál', () => {
    expect(finalDe(faltan as Obj)).toBe('faltan')
    // `env_missing` trae también la opcional (MAIL_FROM); obligatoria es la que
    // nombra el error, y sólo ésa para el despliegue.
    expect(faltanVariables(faltan as Obj)).toEqual(['DATABASE_URL'])
  })

  it('el nombre se busca entero: una DB declarada no nombra a DB_PASSWORD', () => {
    const d: Obj = {
      ...(faltan as Obj), env_missing: ['DB', 'DB_PASSWORD'],
      error: 'faltan variables obligatorias: DB_PASSWORD',
    }
    expect(faltanVariables(d)).toEqual(['DB_PASSWORD'])
  })

  it('una opcional pendiente NO convierte un fallo del build en «faltan»', () => {
    const roto: Obj = {
      ...(faltan as Obj), failed_step: 'build', error: 'el build ha fallado',
    }
    expect(finalDe(roto)).toBe('roto')
    // Ni en el paso de la release si el error no nombra ninguna: el rsync
    // también puede fallar ahí.
    const rsync: Obj = { ...(faltan as Obj), error: 'rsync: no space left on device' }
    expect(finalDe(rsync)).toBe('roto')
  })

  it('la pantalla del negado no dice «ha fallado» y la de «faltan» pide los valores', async () => {
    const p = leerProgreso('')
    const n = render(Despliegue, { app: 'tienda', servidor: 'vps', progreso: p, resultado: congelada as Obj })
    expect(texto(n.container)).toContain('No se ha desplegado')
    expect(texto(n.container)).not.toContain('ha fallado')
    n.unmount()

    const dar = vi.fn()
    const f = render(Despliegue, {
      app: 'tienda', servidor: 'vps', progreso: p, resultado: faltan as Obj, alDarVariables: dar,
    })
    expect(texto(f.container)).toContain('DATABASE_URL')
    expect(texto(f.container)).not.toContain('MAIL_FROM')
    // Lo que se ofrece es darlas, no un diagnóstico de disco que no tiene nada
    // que ver.
    expect(texto(f.container)).not.toContain('Ir al diagnóstico')
    const b = [...f.container.querySelectorAll('button')].find((x) => /Darlas/.test(x.textContent ?? ''))!
    await fireEvent.click(b)
    expect(dar).toHaveBeenCalledWith(['DATABASE_URL'])
  })
})

describe('la pasada con las puertas de la 1.4.3', () => {
  const l = puertas as L

  it('once recuentos separados, y la CI sin respuesta pone la pasada en rojo', () => {
    const c = recuentos(l)
    expect(c.every((x) => x.n !== null)).toBe(true)
    expect(c.reduce((s, x) => s + (x.n ?? 0), 0)).toBe(l.total)
    // La regla de `ok`: fallida, sin contacto, sin rama y CI sin respuesta.
    expect(necesitanMirada(l)).toBe(4)
    expect(retenidas(l)).toBe(3) // congelada, CI pendiente, CI en rojo
  })

  it('se pintan las once celdas con uno nuevo y las seis de siempre con uno viejo', () => {
    const nuevo = render(Lote, { lote: l, servidor: 'produccion' })
    expect(nuevo.container.querySelectorAll('.cuenta')).toHaveLength(11)
    for (const id of ['locked', 'ignored', 'ci_pending', 'ci_failed', 'ci_unknown']) {
      expect(nuevo.container.querySelector(`.cuenta[data-final="${id}"] .n`)?.textContent).toBe('1')
    }
    nuevo.unmount()
    const viejo = render(Lote, { lote: loteViejo as L, servidor: 'vps-ovh' })
    // Un 1.3.6 no cuenta las puertas: no hay celda, porque un cero diría que
    // ha mirado.
    expect(viejo.container.querySelectorAll('.cuenta')).toHaveLength(6)
  })

  it('las filas dicen el final con su palabra y el motivo de las que no se tocaron', () => {
    const { container } = render(Lote, { lote: l, servidor: 'produccion' })
    const t = texto(container)
    expect(t).toContain('congeladas')
    expect(t).toContain('black friday')
    expect(t).toContain('CI sin respuesta')
    expect(t).toContain('gh no tiene sesión')
    // Y las retenidas a propósito se dicen aparte: «ninguna falló» se leería
    // como «todo publicado».
    expect(t).toContain('a propósito')
  })

  it('sin --if-changed una app con la CI sí pregunta al remoto, y eso cambia lo posible', () => {
    // Un servidor viejo: sólo desplegadas o fallidas.
    expect(finalesPosibles(false)).toEqual(['deployed', 'failed'])
    // Uno con puertas: la congelada se salta siempre, y la CI pregunta.
    const p = finalesPosibles(false, true)
    expect(p).toContain('locked')
    expect(p).toContain('unreachable')
    expect(p).toContain('ci_unknown')
    // Lo que sigue sin poder salir: las dos respuestas a «¿ha cambiado?».
    expect(p).not.toContain('unchanged')
    expect(p).not.toContain('ignored')
  })

  it('antes de lanzar dice cuáles se van a saltar por congeladas', () => {
    const { container } = render(Pasada, {
      servidor: 'produccion', apps: apps14,
      alLanzar: () => {}, alCancelar: () => {}, alCerrar: () => {},
    })
    const t = texto(container)
    expect(t).toContain('2 están congeladas')
    expect(t).toContain('tienda, portal')
  })

  it('con un servidor viejo no dice nada de congeladas', () => {
    const { container } = render(Pasada, {
      servidor: 'vps-ovh', apps: (listaVieja as Lista).apps,
      alLanzar: () => {}, alCancelar: () => {}, alCerrar: () => {},
    })
    expect(texto(container)).not.toContain('congelada')
  })
})

describe('la lista', () => {
  it('marca las congeladas como faceta y deja el estado como está', () => {
    const { container } = render(ListaApps, { apps: apps14, servidor: 'produccion' })
    const marcas = [...container.querySelectorAll('.faceta')].filter((f) => f.textContent === 'congelada')
    expect(marcas).toHaveLength(2)
    // El chip de una congelada sigue siendo el de lo que se sirve.
    expect(container.querySelector('.chip--activo')).not.toBeNull()
  })

  it('un servidor viejo no tiene congeladas que pintar', () => {
    const { container } = render(ListaApps, { apps: (listaVieja as Lista).apps, servidor: 'vps-ovh' })
    expect(texto(container)).not.toContain('congelada')
  })

  it('el certificado de origen se ve sin abrir el detalle', () => {
    const { container } = render(FacetaSsl, { estado: app('api').state })
    expect(texto(container)).toContain('origen CF')
    expect(container.querySelector('.faceta')?.getAttribute('title')).toContain('detrás de su proxy')
  })
})

describe('el detalle', () => {
  const info = (n: string): AppInfo => ({ ...(info14 as { app: AppInfo }).app, name: n, state: { ...app(n).state, cert_days: 61 } })

  it('una congelada dice quién, cuándo y por qué, y ofrece descongelar', async () => {
    const descongelar = vi.fn()
    const { container } = render(DetalleApp, {
      app: app('tienda'), servidor: 'produccion', info: info('tienda'),
      puedeCongelar: true, alCongelar: vi.fn(), alDescongelar: descongelar,
    })
    const t = texto(container)
    expect(t).toContain('Despliegues congelados')
    expect(t).toContain('dabellan')
    expect(t).toContain('black friday')
    expect(t).toContain('Revertir sigue permitido')
    expect(t).toContain("Let's Encrypt")
    const b = [...container.querySelectorAll('button')].find((x) => /Descongelar/.test(x.textContent ?? ''))!
    await fireEvent.click(b)
    expect(descongelar).toHaveBeenCalled()
  })

  it('un touch a mano no dice quién, y no se inventa', () => {
    const { container } = render(DetalleApp, { app: app('portal'), servidor: 'produccion' })
    const t = texto(container)
    expect(t).toContain('Despliegues congelados')
    expect(t).not.toContain(' por ')
  })

  it('un servidor que no sabe congelar no ofrece congelar', () => {
    const vieja = (listaVieja as Lista).apps[0]!
    const { container } = render(DetalleApp, {
      app: vieja, servidor: 'vps-ovh', puedeCongelar: true, alCongelar: vi.fn(),
    })
    expect(texto(container)).not.toContain('Congelar')
  })

  it('congelar pide el motivo y no deja mandar uno que no cabe', async () => {
    const congelar = vi.fn()
    const { container } = render(DetalleApp, {
      app: app('blog'), servidor: 'produccion', puedeCongelar: true, alCongelar: congelar,
    })
    const abrir = [...container.querySelectorAll('button')].find((x) => x.textContent?.trim() === 'Congelar despliegues')!
    await fireEvent.click(abrir)
    const input = container.querySelector('input')!
    await fireEvent.input(input, { target: { value: 'x'.repeat(201) } })
    const ir = [...container.querySelectorAll('button')].find((x) => x.textContent?.trim() === 'Congelar…')!
    expect(ir.hasAttribute('disabled')).toBe(true)
    await fireEvent.input(input, { target: { value: 'migración' } })
    await tick()
    expect(ir.hasAttribute('disabled')).toBe(false)
    await fireEvent.click(ir)
    expect(congelar).toHaveBeenCalledWith('migración')
  })

  it('las tareas: un null es «nunca», y sin respuesta no se pinta una lista vacía', () => {
    const con = render(DetalleApp, { app: app('tienda'), servidor: 'produccion', tareas: cron as Tareas })
    expect(texto(con.container)).toContain('limpiar-sesiones')
    expect(texto(con.container)).toContain('nunca')
    con.unmount()
    const sin = render(DetalleApp, { app: app('tienda'), servidor: 'produccion', tareas: null })
    expect(texto(sin.container)).not.toContain('Tareas programadas')
  })

  it('un automático que no espera a la CI se dice, porque desde 1.4.9 no es lo normal', () => {
    const sola = { ...app('blog'), state: { ...app('blog').state, autodeploy: true, checks: false } }
    const { container } = render(DetalleApp, {
      app: sola, servidor: 'produccion', puedeEsperarCi: true, alEsperarCi: vi.fn(),
    })
    expect(texto(container)).toContain('Se despliega sola sin esperar a la CI')
    // Y con la espera puesta, no.
    const r = render(DetalleApp, { app: app('api'), servidor: 'produccion', puedeEsperarCi: true, alEsperarCi: vi.fn() })
    expect(texto(r.container)).not.toContain('sin esperar a la CI')
  })

  it('el origen de Cloudflare se explica en el detalle', () => {
    const { container } = render(DetalleApp, {
      app: app('api'), servidor: 'produccion', info: { ...info('api'), state: { ...app('api').state, cert_days: 5400 } },
    })
    expect(texto(container)).toContain('origen de Cloudflare')
    expect(texto(container)).toContain('espera a la CI')
  })
})

describe('la hoja del despliegue', () => {
  const c = comprobacion as ComprobacionDeEntorno

  it('la comprobación cuenta lo que pararía el despliegue', () => {
    expect(obligatoriasSinValor(c)).toEqual(['DATABASE_URL'])
  })

  it('pide la obligatoria antes de desplegar y no enseña su valor en la orden', async () => {
    const confirmar = vi.fn()
    const { container } = render(HojaDeDespliegue, {
      app: app('blog'), servidor: 'produccion', comprobacion: c, estadoComprobacion: 'hecho',
      alConfirmar: confirmar, alCancelar: () => {},
    })
    const ir = [...container.querySelectorAll('button')].find((x) => x.textContent?.trim() === 'Desplegar')!
    expect(ir.hasAttribute('disabled')).toBe(true)

    const campo = container.querySelector('input[type=password]') as HTMLInputElement
    await fireEvent.input(campo, { target: { value: "it's" } })
    await tick()
    expect(texto(container)).toContain('comilla simple')
    expect(ir.hasAttribute('disabled')).toBe(true)

    await fireEvent.input(campo, { target: { value: 'postgres://u:s3cr3t@h/db' } })
    await tick()
    expect(ir.hasAttribute('disabled')).toBe(false)
    const orden = container.querySelector('.orden')!.textContent!
    expect(orden).toContain(`DATABASE_URL=${OCULTO}`)
    expect(orden).not.toContain('s3cr3t')

    await fireEvent.click(ir)
    expect(confirmar).toHaveBeenCalledWith([{ clave: 'DATABASE_URL', valor: 'postgres://u:s3cr3t@h/db' }], false)
    // Y en cuanto se manda, se olvida.
    await tick()
    expect(campo.value).toBe('')
  })

  it('con Orbit 1.4.9 los valores no van en la orden: ni el valor ni la bandera --env', async () => {
    const { container } = render(HojaDeDespliegue, {
      app: app('blog'), servidor: 'produccion', comprobacion: c, estadoComprobacion: 'hecho',
      porEntrada: true, alConfirmar: () => {}, alCancelar: () => {},
    })
    const campo = container.querySelector('input[type=password]') as HTMLInputElement
    await fireEvent.input(campo, { target: { value: 's3cr3t' } })
    await tick()
    const orden = container.querySelector('.orden')!.textContent!
    expect(orden).toContain('--env-stdin')
    expect(orden).not.toContain('--env ')
    expect(orden).not.toContain('DATABASE_URL')
    expect(texto(container)).not.toContain('s3cr3t')
    expect(texto(container)).toContain('por la entrada del canal SSH')
    expect(texto(container)).not.toContain('lista de procesos del servidor, y si')
  })

  it('con un Orbit anterior se avisa de que el valor va en la orden', () => {
    const { container } = render(HojaDeDespliegue, {
      app: app('blog'), servidor: 'produccion', comprobacion: c, estadoComprobacion: 'hecho',
      alConfirmar: () => {}, alCancelar: () => {},
    })
    expect(texto(container)).toContain('anterior a la 1.4.9')
    expect(texto(container)).toContain('registro de sudo')
  })

  it('una congelada se despliega con su propio botón y con --force', async () => {
    const confirmar = vi.fn()
    const { container } = render(HojaDeDespliegue, {
      app: app('tienda'), servidor: 'produccion', alConfirmar: confirmar, alCancelar: () => {},
    })
    const t = texto(container)
    expect(t).toContain('Está congelada')
    expect(t).toContain('sigue congelada')
    expect(container.querySelector('.orden')!.textContent).toContain('--force')
    const ir = [...container.querySelectorAll('button')].find((x) => /Desplegar igualmente/.test(x.textContent ?? ''))!
    await fireEvent.click(ir)
    expect(confirmar).toHaveBeenCalledWith([], true)
  })

  it('«no he podido mirarlo» no es «no falta nada»', () => {
    const { container } = render(HojaDeDespliegue, {
      app: app('blog'), servidor: 'produccion', estadoComprobacion: 'no-se-pudo',
      alConfirmar: () => {}, alCancelar: () => {},
    })
    expect(texto(container)).toContain('No he podido preguntar')
  })

  it('lo que pidió un despliegue parado manda, aunque la release que sirve no lo declare', () => {
    const { container } = render(HojaDeDespliegue, {
      app: app('blog'), servidor: 'produccion', faltanDeAntes: ['STRIPE_KEY'],
      alConfirmar: () => {}, alCancelar: () => {},
    })
    expect(texto(container)).toContain('STRIPE_KEY')
    expect(container.querySelectorAll('input[type=password]')).toHaveLength(1)
  })
})
