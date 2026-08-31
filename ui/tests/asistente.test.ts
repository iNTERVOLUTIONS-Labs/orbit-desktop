/**
 * El asistente de web nueva: siete finales, y cinco de ellos parciales.
 *
 * La decisión que sostiene todo esto, y que sale de dos hechos del servidor:
 * `orbit new` **no tiene `--json`** —sólo prosa y un código de salida— y su
 * resumen distingue tres casos en castellano.
 *
 * > La interfaz no interpreta esa prosa. **Le vuelve a preguntar al servidor.**
 *
 * Al terminar se ejecuta `orbit info --json`, que sí tiene contrato. Son 86 ms
 * medidos sobre un comando de tres minutos, y es la única forma que no depende
 * del idioma del servidor.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'

import Desenlace from '../src/componentes/Desenlace.svelte'
import { clasificar, nombreValido, ordenDeNew } from '../src/lib/asistente'
import type { AppInfo, Estado } from '../src/lib/contrato'

const BASE: Estado = {
  service: 'active', port: 3001, ssl: true, cert_days: 80,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 1, last_deploy: '20260830-120000', last_deploy_sha: 'abc',
}
const info = (p: Partial<Estado>, releases = ['20260830-120000']): AppInfo => ({
  name: 'tienda', path: '/srv/apps/tienda', config: {},
  state: { ...BASE, ...p }, releases,
})

describe('los siete finales', () => {
  it('F1 · publicada del todo', () => {
    const d = clasificar('tienda', info({}), true)
    expect(d.final).toBe('F1')
    expect(d.tono).toBe('bien')
  })

  it('F2 · publicada sin certificado NO es un fallo', () => {
    // Es el final que el propio Orbit documenta como inevitable —Let's Encrypt
    // necesita un email y sin terminal no hay a quién pedírselo— y la razón por
    // la que `orbit new` avisa y sigue en vez de morir a mitad: «dejarte la app
    // creada pero el comando en error es la peor combinación». Pintarlo rojo
    // contradiría esa decisión.
    const d = clasificar('tienda', info({ ssl: false }), true)
    expect(d.final).toBe('F2')
    expect(d.tono).toBe('atencion')
    expect(d.tono).not.toBe('error')
    expect(d.existe).toContain('ya está publicada')
    expect(d.accion?.orden).toBe('orbit ssl tienda')
  })

  it('F3 · creada pero sin compilar, y lo primero que se dice es lo que EXISTE', () => {
    // Quien ve «ha fallado» asume que no hay nada, y lo que hay es una app
    // registrada con su dominio y su configuración. Y un 502.
    const d = clasificar('tienda', info({}, []), true)
    expect(d.final).toBe('F3')
    expect(d.existe).toContain('está registrada')
    expect(d.falta).toContain('502')
    // Se retira sin borrar: no ha llegado a servir nada, así que no hay datos
    // que perder. Se comprueba la ORDEN y no la frase — la frase dice «sin
    // --purge», que es correcto y contiene la cadena.
    expect(d.deshacer).toContain('orbit remove tienda -y')
    expect(d.deshacer).not.toContain('remove tienda -y --purge')
  })

  it('F4 · publicada pero el proceso no se mantiene', () => {
    const d = clasificar('tienda', info({ service: 'failed' }), true)
    expect(d.final).toBe('F4')
    expect(d.accion?.orden).toContain('logs')
  })

  it('F5 · sin vhost gana sobre todo lo demás', () => {
    // Sin vhost la conexión se cierra: ningún otro campo describe lo que recibe
    // un visitante, así que este final manda aunque falten otras cosas.
    const d = clasificar('tienda', info({ served: false, ssl: false }, []), true)
    expect(d.final).toBe('F5')
    expect(d.falta).toContain('ni 404 ni 502')
  })

  it('F6 · no se creó nada, y no hay nada que deshacer', () => {
    const d = clasificar('tienda', null)
    expect(d.final).toBe('F6')
    expect(d.deshacer).toContain('Nada que deshacer')
    expect(d.accion).toBeNull()
  })

  it('F7 · publicada pero el dominio no apunta aquí', () => {
    // Una web perfectamente publicada cuyo DNS no resuelve se ve exactamente
    // igual desde dentro: por eso esto no sale del contrato y se comprueba
    // aparte.
    const d = clasificar('tienda', info({}), false)
    expect(d.final).toBe('F7')
    expect(d.deshacer).toContain('no aquí')
  })

  it('los siete son distintos entre sí', () => {
    const casos = [
      clasificar('t', info({}), true),
      clasificar('t', info({ ssl: false }), true),
      clasificar('t', info({}, []), true),
      clasificar('t', info({ service: 'failed' }), true),
      clasificar('t', info({ served: false }), true),
      clasificar('t', null),
      clasificar('t', info({}), false),
    ]
    expect(new Set(casos.map((c) => c.final)).size).toBe(7)
  })

  it('cinco de los siete son PARCIALES: algo existe y algo falta', () => {
    // Es lo que faltaba en la primera versión del asistente, y lo que lo hacía
    // frágil: tratar «salió bien» y «falló» como los dos únicos finales deja a
    // alguien con una app a medias y sin saberlo.
    const parciales = [
      clasificar('t', info({ ssl: false }), true),
      clasificar('t', info({}, []), true),
      clasificar('t', info({ service: 'failed' }), true),
      clasificar('t', info({ served: false }), true),
      clasificar('t', info({}), false),
    ]
    for (const p of parciales) {
      expect(p.existe.length, `${p.final} tiene que decir qué existe`).toBeGreaterThan(0)
      expect(p.falta.length, `${p.final} tiene que decir qué falta`).toBeGreaterThan(0)
    }
  })
})

describe('la pantalla del final', () => {
  it('dice las tres cosas: qué existe, qué falta y qué se deshace', () => {
    const d = clasificar('tienda', info({}, []), true)
    const { container } = render(Desenlace, { d, app: 'tienda' })
    const dts = [...container.querySelectorAll('dt')].map((n) => n.textContent)
    expect(dts).toEqual(['Qué existe', 'Qué falta', 'Qué se deshace'])
  })

  it('«sin certificado» no se pinta como error', () => {
    const d = clasificar('tienda', info({ ssl: false }), true)
    const { container } = render(Desenlace, { d, app: 'tienda' })
    expect(container.querySelector('.desenlace--atencion')).not.toBeNull()
    expect(container.querySelector('.desenlace--error')).toBeNull()
  })

  it('avisa de dónde se guarda el correo, antes y no después', () => {
    // Evita el «¿por qué ya no me lo pregunta?» de dentro de tres meses.
    const d = clasificar('tienda', info({ ssl: false }), true)
    const { container } = render(Desenlace, { d, app: 'tienda' })
    expect(container.querySelector('.nota')?.textContent).toContain('configuración del servidor')
  })

  it('la orden se ve junto a la acción', () => {
    // La prueba de que esto sólo invoca `orbit`, también aquí.
    const d = clasificar('tienda', info({ ssl: false }), true)
    const { container } = render(Desenlace, { d, app: 'tienda' })
    expect(container.querySelector('code')?.textContent).toBe('orbit ssl tienda')
  })
})

describe('el nombre se valida mientras se escribe', () => {
  it('acepta lo que el servidor acepta', () => {
    for (const n of ['mi-web', 'viejo.com', 'a', 'a'.repeat(40), 'app_2']) {
      expect(nombreValido(n), n).toBeNull()
    }
  })

  it('y rechaza lo que el servidor rechaza, diciendo por qué', () => {
    // Un «nombre no válido» a secas obliga a adivinar cuál de las cinco reglas
    // se ha roto.
    expect(nombreValido('Web')).toContain('minúscula')
    expect(nombreValido('-web')).toContain('empezar')
    expect(nombreValido('a..b')).toContain('dos puntos')
    expect(nombreValido('a'.repeat(41))).toContain('40')
    expect(nombreValido('mi web')).toContain('Sólo minúsculas')
  })

  it('vacío no es un error todavía', () => {
    // Quejarse antes de que alguien haya escrito nada es ruido.
    expect(nombreValido('')).toBeNull()
  })
})

describe('la orden que se va a ejecutar', () => {
  it('se puede enseñar entera antes de lanzarla', () => {
    const o = ordenDeNew({
      nombre: 'tienda', repo: 'usuario/tienda', rama: 'main', dominio: 'tienda.com',
    })
    expect(o).toBe('orbit new --yes --repo usuario/tienda --branch main --name tienda --domain tienda.com')
  })

  it('«--yes» no es «que sí a todo»', () => {
    // Es «acepta lo que está por defecto». No crea la base de datos y no abre el
    // editor del .env, porque esas dos preguntas tienen «no» por defecto.
    const o = ordenDeNew({
      nombre: 'x', repo: 'u/x', rama: 'main', dominio: 'x.com',
    })
    expect(o).not.toContain('--db')
    expect(o).not.toContain('--purge')
  })
})
