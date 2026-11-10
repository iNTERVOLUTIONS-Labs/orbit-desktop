/**
 * La forma de las respuestas, mirada al entrar.
 *
 * El puente devolvía lo que el servidor escribió con un `as T` encima, y un
 * tipo de TypeScript no comprueba nada en ejecución: `ok:"false"` se pintaba
 * «Desplegada», un `name:42` dejaba la lista en «Cargando…» y un campo que
 * faltaba salía como «undefined %». Estas pruebas fijan las tres reglas:
 *
 *  · lo que no se conoce se ignora sin ruido;
 *  · lo esperado que falta o trae otro tipo es un error de la respuesta, de
 *    clase `forma`, que dice qué comando y dónde — nunca un valor por defecto;
 *  · las muestras y las fixtures del servidor falso pasan, y la salida real de
 *    Orbit 1.5.1 también: si no, el lector está mal, no ellas.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const invocado = vi.hoisted(() => ({ respuesta: undefined as unknown, nombre: '' }))
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (nombre: string) => {
    invocado.nombre = nombre
    return invocado.respuesta
  }),
}))

import {
  leerApp, leerColas, leerComprobacionDeEntorno, leerCopias, leerDespliegue, leerDoctor, leerEntorno,
  leerInfo, leerLista, leerLog, leerLote, leerMetricasEnvueltas, leerMonitor, leerPortada,
  leerRedirecciones, leerTareas, leerTraficos, leerVerificacion, leerVersion, leerVigilancia,
  leerBasesDeDatos, sabe, semver, seQuedaCorto, validar, type ErrorDeForma, type Lector,
} from '../src/lib/contrato'
import * as puente from '../src/lib/puente'

const RAIZ = join(import.meta.dirname, '..', '..')
const MUESTRAS = join(RAIZ, 'ui', 'src', 'lib', 'muestras')
const FIXTURES = join(RAIZ, 'tests', 'fakeserver', 'fixtures')

/** Lo que lanza `validar`, o `null` si no lanza. */
function fallo(comando: string, l: Lector<unknown>, v: unknown): ErrorDeForma | null {
  try {
    validar(comando, l, v)
    return null
  } catch (e) {
    return e as ErrorDeForma
  }
}

const ESTADO = {
  service: 'running', port: 3001, ssl: true, cert_days: null, maintenance: false, served: true,
  autodeploy: false, queue: false, releases: 1, last_deploy: null, last_deploy_sha: null,
}
const APP = { name: 'tienda', type: 'node', domain: 'tienda.test', aliases: [], state: ESTADO }
const HOST = {
  hostname: 'h', ip: '1.2.3.4', uptime_seconds: 10, load: [0.1, 0.2, 0.3],
  memory_kb: { total: 1, used: 1, available: 0 }, disk_kb: { mount: '/', total: 1, used: 1, use_percent: 1 },
}
const DESPLIEGUE = {
  schema: 1, app: 'tienda', ok: true, release: '20261110-1', previous: null,
  commit: { sha: null, subject: null, ref: null }, rolled_back: false, recovered: false,
  duration_s: 3, failed_step: null, error: null,
}

/** La salida de `_list_json` de Orbit 1.5.1, la de la auditoría del contrato. */
const REAL_1_5_1 = '{"schema":1,"apps":[{"name":"web","type":"node","domain":"web.example.com","aliases":[],"state":{"service":"running","port":3001,"ssl":true,"cert_days":null,"cert_type":"letsencrypt","maintenance":false,"served":true,"autodeploy":false,"queue":false,"checks":false,"locked":false,"lock":null,"releases":1,"last_deploy":"2026-11-01T00:00:00+00:00","last_deploy_sha":"abc1234"}}]}'

describe('las muestras y las fixtures dicen lo mismo que el contrato', () => {
  // Qué lector le toca a cada fichero, por su nombre. Uno que no esté aquí hace
  // fallar la prueba: una fixture nueva tiene que decir qué es.
  function lectorDe(f: string): Lector<unknown> {
    if (f.startsWith('backup-list')) return leerCopias
    if (f.startsWith('backup-verify')) return leerVerificacion
    if (f.startsWith('cron')) return leerTareas
    if (f.startsWith('deploy-all')) return leerLote
    if (f.startsWith('deploy-')) return leerDespliegue
    if (f.startsWith('doctor')) return leerDoctor
    if (f.startsWith('env-check')) return leerComprobacionDeEntorno
    if (f === 'env.json') return leerEntorno
    if (f.startsWith('info')) return leerInfo
    if (f.startsWith('list') || f === 'schema2.json') return leerLista
    if (f.startsWith('metrics')) return leerMetricasEnvueltas
    if (f.startsWith('status')) return leerPortada
    if (f.startsWith('top')) return leerMonitor
    if (f.startsWith('traffic')) return leerTraficos
    if (f.startsWith('version')) return leerVersion
    if (f.startsWith('db-list')) return leerBasesDeDatos
    if (f.startsWith('queue-status')) return leerColas
    if (f.startsWith('redirect-list')) return leerRedirecciones
    if (f.startsWith('watch-status')) return leerVigilancia
    throw new Error(`no sé qué respuesta es ${f}`)
  }

  for (const [donde, dir] of [['muestras', MUESTRAS], ['fixtures', FIXTURES]] as const) {
    for (const f of readdirSync(dir).filter((x) => x.endsWith('.json'))) {
      it(`${donde}/${f}`, () => {
        const v = JSON.parse(readFileSync(join(dir, f), 'utf8'))
        expect(fallo(f, lectorDe(f), v)).toBeNull()
      })
    }
    it(`${donde}/logs.ndjson no tiene ninguna línea rota`, () => {
      const l = leerLog(readFileSync(join(dir, 'logs.ndjson'), 'utf8'))
      expect(l.rotas).toBe(0)
      expect(l.meta).not.toBeNull()
    })
  }

  it('la salida real de Orbit 1.5.1 pasa, con sus campos nuevos', () => {
    const l = validar('list', leerLista, JSON.parse(REAL_1_5_1))
    expect(l.apps[0]!.state.locked).toBe(false)
    expect(l.apps[0]!.state.lock).toBeNull()
    expect(l.apps[0]!.state.cert_type).toBe('letsencrypt')
  })

  it('las fixtures dicen «running», que es lo que escribe Orbit', () => {
    for (const dir of [MUESTRAS, FIXTURES]) {
      for (const f of readdirSync(dir).filter((x) => x.endsWith('.json'))) {
        expect(readFileSync(join(dir, f), 'utf8'), f).not.toContain('"service":"active"')
      }
    }
  })
})

describe('lo que no se conoce se ignora; lo que no está, no está', () => {
  it('un campo nuevo no rompe nada y no se cuela en el valor', () => {
    const a = validar('list', leerApp, { ...APP, futuro: 1, state: { ...ESTADO, campo_del_futuro: [1] } })
    expect(a).not.toHaveProperty('futuro')
    expect(a.state).not.toHaveProperty('campo_del_futuro')
  })

  it('lo de después de la 1.3.6 que no viene se queda sin venir: no es «false»', () => {
    const a = validar('list', leerApp, APP)
    expect('locked' in a.state).toBe(false)
    expect('checks' in a.state).toBe(false)
    expect('cert_type' in a.state).toBe(false)
  })

  it('un campo que no viene no sale del prototipo de Object', () => {
    const f = fallo('version', leerVersion, { schema: 1, contract: 1 })
    expect(f?.mensaje).toContain('falta version')
    // «constructor» existe en cualquier objeto; aquí no es un campo.
    expect(fallo('info', leerInfo, { schema: 1, app: { constructor: 'x' } })?.mensaje).toContain('falta app.name')
  })

  it('una clave «__proto__» del descriptor se conserva como clave', () => {
    const i = validar('info', leerInfo, JSON.parse(
      `{"schema":1,"app":{"name":"t","path":"/srv/t","config":{"__proto__":"x","type":"static"},"state":${JSON.stringify(ESTADO)},"releases":[]}}`,
    ))
    expect(Object.keys(i.app.config)).toEqual(['__proto__', 'type'])
    expect(i.app.config['__proto__']).toBe('x')
  })
})

describe('lo que viene mal es un error de la respuesta, y dice dónde', () => {
  it('ok:"false" no es un despliegue bueno: es una respuesta rota', () => {
    const f = fallo('deploy', leerDespliegue, { ...DESPLIEGUE, ok: 'false' })
    expect(f?.clase).toBe('forma')
    expect(f?.mensaje).toBe('«deploy»: ok debería ser verdadero o falso y es texto')
  })

  it('served:"false" tampoco pasa por activo', () => {
    expect(fallo('status', leerApp, { ...APP, state: { ...ESTADO, served: 'false' } })?.mensaje)
      .toBe('«status»: state.served debería ser verdadero o falso y es texto')
  })

  it('un nombre numérico dice qué app y qué campo', () => {
    const f = fallo('status', leerPortada, { schema: 1, host: HOST, services: [], apps: [APP, APP, APP, { ...APP, name: 42 }] })
    expect(f?.mensaje).toBe('«status»: apps[3].name debería ser texto y es un número')
    expect(f?.detalle).toBe('42')
  })

  it('sin apps, con apps que no son una lista, sin state o con el dominio a null', () => {
    const base = { schema: 1, host: HOST, services: [] }
    expect(fallo('status', leerPortada, base)?.mensaje).toBe('«status»: falta apps, que debería ser una lista')
    expect(fallo('status', leerPortada, { ...base, apps: { length: 2 } })?.mensaje)
      .toBe('«status»: apps debería ser una lista y es un objeto')
    const { state: _, ...sinState } = APP
    expect(fallo('status', leerPortada, { ...base, apps: [sinState] })?.mensaje)
      .toBe('«status»: falta apps[0].state, que debería ser un objeto')
    expect(fallo('status', leerPortada, { ...base, apps: [{ ...APP, domain: null }] })?.mensaje)
      .toBe('«status»: apps[0].domain debería ser texto y es null')
  })

  it('el host: cargas con null dentro, o sin memoria', () => {
    expect(fallo('status', leerPortada, { schema: 1, host: { ...HOST, load: [null, null, null] }, services: [], apps: [] })?.mensaje)
      .toBe('«status»: host.load[0] debería ser un número y es null')
    const { memory_kb: _, ...sinMemoria } = HOST
    expect(fallo('status', leerPortada, { schema: 1, host: sinMemoria, services: [], apps: [] })?.mensaje)
      .toBe('«status»: falta host.memory_kb, que debería ser un objeto')
  })

  it('un campo que admite null no admite que falte', () => {
    const { cert_days: _, ...sinDias } = ESTADO
    expect(fallo('info', leerApp, { ...APP, state: sinDias })?.mensaje)
      .toBe('«info»: falta state.cert_days, que debería ser un número o null')
  })

  it('el monitor sin cpu_percent, las copias sin keep_days, el tamaño que no es número', () => {
    const fila = { name: 't', type: 'node', domain: 'd', port: 1, service: 'running', memory_bytes: 1, requests_last_minute: 1, requests_capped: false }
    expect(fallo('top', leerMonitor, { schema: 1, apps: [fila] })?.mensaje)
      .toBe('«top»: falta apps[0].cpu_percent, que debería ser un número o null')
    expect(fallo('backup list', leerCopias, { schema: 1, dir: '/b', backups: [], total: 0, bytes: 0 })?.mensaje)
      .toBe('«backup list»: falta keep_days, que debería ser un número o null')
    expect(fallo('backup list', leerCopias, {
      schema: 1, dir: '/b', total: 1, bytes: 0, keep_days: 7,
      backups: [{ file: '/b/x', app: null, kind: 'config', created: 'x', size_bytes: '12', verified: null }],
    })?.mensaje).toBe('«backup list»: backups[0].size_bytes debería ser un número o null y es texto')
  })

  it('una versión que no es texto', () => {
    expect(fallo('version', leerVersion, { schema: 1, version: 1.5, contract: 1 })?.mensaje)
      .toBe('«version»: version debería ser texto y es un número')
  })

  it('un nivel del diagnóstico que no existe', () => {
    expect(fallo('doctor', leerDoctor, {
      schema: 1, summary: { ok: 0, warn: 0, error: 0 },
      checks: [{ id: 'x', level: 'grave', message: 'm', fix: null, fixable: false }],
    })?.mensaje).toBe('«doctor»: checks[0].level debería ser uno de «ok», «info», «warn», «error» y es texto')
  })

  it('nada en absoluto', () => {
    expect(fallo('version', leerVersion, undefined)?.mensaje)
      .toBe('«version»: no ha llegado ninguna respuesta, y debería ser un objeto')
  })

  it('el detalle es corto: es para reconocerlo, no para leerlo', () => {
    const f = fallo('env list', leerEntorno, { schema: 1, app: 't', keys: 'x'.repeat(1000) })
    expect(f!.detalle!.length).toBeLessThanOrEqual(201)
  })
})

describe('el log', () => {
  it('una línea que es JSON pero no un suceso se cuenta como rota, sin tumbar el resto', () => {
    const l = leerLog(['null', '42', '[]', '{"event":"line","ts":null,"stream":"access","text":"GET /"}'].join('\n'))
    expect(l.rotas).toBe(3)
    expect(l.lineas.length).toBe(1)
  })

  it('una cabecera mal formada no decide nada: se cuenta y se queda sin cabecera', () => {
    const l = leerLog('{"schema":1,"event":"meta","app":"t","source":"journal","unit":null,"since":null,"follow":"no","lines":5}')
    expect(l.meta).toBeNull()
    expect(l.rotas).toBe(1)
  })

  it('un final con «truncated» que no es booleano no se lee como cierto', () => {
    const l = leerLog('{"event":"end","lines":3,"truncated":"true"}')
    expect(l.truncado).toBe(false)
    expect(l.rotas).toBe(1)
  })
})

describe('la versión', () => {
  it('lo que no es texto no es una versión, y no revienta', () => {
    expect(semver(1.5)).toBeNull()
    expect(semver({})).toBeNull()
    expect(semver(null)).toBeNull()
    expect(sabe(1.5, 'contratoCompleto')).toBe(false)
    // Desconocida no esconde nada.
    expect(seQuedaCorto(1.5, 'contratoCompleto')).toBe(false)
  })

  it('una «v» delante se lee', () => {
    expect(semver('v1.5.1')).toEqual([1, 5, 1])
    expect(sabe('v1.4.3', 'congelar')).toBe(true)
  })
})

describe('el puente mira la forma en los dos caminos', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  })
  function conPuente(respuesta: unknown) {
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}
    invocado.respuesta = respuesta
  }

  it('sin envoltorio, cada muestra pasa por su lector', async () => {
    for (const a of await puente.servidoresDelConfig()) {
      const p = await puente.portada(a.alias)
      expect(p.host).toBeDefined()
    }
    await expect(puente.detalle('produccion', 'tienda')).resolves.toMatchObject({ name: 'tienda' })
    await expect(puente.detalle('vps-ovh', 'app001')).resolves.toMatchObject({ name: 'app001' })
    await expect(puente.copias('x')).resolves.toBeTruthy()
    await expect(puente.verificarCopias('x')).resolves.toBeTruthy()
    await expect(puente.redirecciones('x')).resolves.toBeTruthy()
    await expect(puente.basesDeDatos('x')).resolves.toBeTruthy()
    await expect(puente.vigilancia('x')).resolves.toBeTruthy()
    await expect(puente.colas('x')).resolves.toBeTruthy()
    await expect(puente.diagnostico('x')).resolves.toBeTruthy()
    await expect(puente.monitor('x')).resolves.toBeTruthy()
    await expect(puente.trafico('x', 'app001')).resolves.toBeTruthy()
    await expect(puente.metricas('x', 'app001')).resolves.toBeTruthy()
    await expect(puente.entorno('x', 'app001')).resolves.toMatchObject({ app: 'app001' })
    await expect(puente.comprobarEntorno('x', 'tienda')).resolves.toMatchObject({ app: 'tienda' })
    await expect(puente.tareas('x', 'tienda')).resolves.toMatchObject({ app: 'tienda' })
    await expect(puente.version('produccion')).resolves.toMatchObject({ version: '1.4.8' })
    expect((await puente.log('x', 'app001')).rotas).toBe(0)
  })

  it('un alias que se llama como algo de Object no es una muestra', async () => {
    await expect(puente.portada('constructor')).rejects.toMatchObject({ clase: 'sin-muestra' })
  })

  it('con envoltorio, un despliegue con ok:"false" es un error de forma y no un despliegue', async () => {
    conPuente({ ...DESPLIEGUE, ok: 'false' })
    await expect(puente.desplegar('s', 'tienda')).rejects.toMatchObject({
      clase: 'forma', mensaje: '«deploy»: ok debería ser verdadero o falso y es texto',
    })
    expect(invocado.nombre).toBe('desplegar')
  })

  it('con envoltorio, la portada sin apps no es una portada vacía', async () => {
    conPuente({ schema: 1, host: HOST, services: [] })
    await expect(puente.portada('s')).rejects.toMatchObject({ clase: 'forma' })
  })

  it('con envoltorio, una versión numérica no llega a la ventana', async () => {
    conPuente({ schema: 1, version: 1.5, contract: 1 })
    await expect(puente.version('s')).rejects.toMatchObject({ clase: 'forma' })
  })

  it('con envoltorio, lo del núcleo también se mira: un «cancelar» que no es booleano', async () => {
    conPuente('true')
    await expect(puente.cancelar('s', 't')).rejects.toMatchObject({ clase: 'forma' })
    conPuente({ salida: 'ok', codigo: '0' })
    await expect(puente.retirar('s', 't')).rejects.toMatchObject({
      clase: 'forma', mensaje: '«remove»: codigo debería ser un número y es texto',
    })
  })

  it('con envoltorio, una respuesta buena llega sin lo que no se conoce', async () => {
    conPuente({ ...DESPLIEGUE, algo_nuevo: true })
    const d = await puente.desplegar('s', 'tienda')
    expect(d.ok).toBe(true)
    expect(d).not.toHaveProperty('algo_nuevo')
  })

  it('con envoltorio, el log que no es texto', async () => {
    conPuente({ no: 'es texto' })
    await expect(puente.log('s', 't')).rejects.toMatchObject({ clase: 'forma' })
  })
})
