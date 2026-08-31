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
import { readFileSync } from 'node:fs'
import {
  argvDeNew, avisoDelCertificado, borradorNuevo, clasificar, correoValido, dominioValido,
  listaDeAlias, loDetectado, nombreDesdeRepo, nombreValido, ordenDeNew, PASOS,
  problemasDe, ramaValida, repoValido, sinAjustes,
  type Borrador,
} from '../src/lib/asistente'
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

// ── El formulario ───────────────────────────────────────────────────────────

/** Un borrador que pasa todos los pasos, para partir de ahí y romper uno. */
function bueno(cambios: Partial<Borrador> = {}): Borrador {
  return {
    ...borradorNuevo(),
    repo: 'usuario/tienda',
    rama: 'main',
    nombre: 'tienda',
    dominio: 'tienda.ejemplo.com',
    ...cambios,
  }
}

describe('las reglas de forma', () => {
  it('un dominio sin punto no puede tener certificado, así que no vale', () => {
    // Publicar en «localhost» dejaría una web que nunca podrá tener HTTPS, que
    // es justo el final parcial que este asistente existe para evitar.
    expect(dominioValido('localhost')).toBe(false)
    expect(dominioValido('tienda.ejemplo.com')).toBe(true)
    expect(dominioValido('-mal.ejemplo.com')).toBe(false)
    expect(dominioValido('mal-.ejemplo.com')).toBe(false)
    expect(dominioValido('guion_bajo.ejemplo.com')).toBe(false)
  })

  it('una rama que empieza por guion no sale de aquí', () => {
    // No es una cuestión de forma: «--branch --purge» se lo come el analizador
    // de opciones del otro lado antes de que nadie mire si la rama existe.
    expect(ramaValida('--purge')).toBe(false)
    expect(ramaValida('main')).toBe(true)
    expect(ramaValida('feat/algo')).toBe(true)
    expect(ramaValida('con espacio')).toBe(false)
    expect(ramaValida('a..b')).toBe(false)
  })

  it('un «git@…» no se acepta, porque no se puede comprobar desde aquí', () => {
    expect(repoValido('git@github.com:usuario/repo.git')).toBe(false)
    expect(repoValido('usuario/repo')).toBe(true)
    expect(repoValido('https://github.com/usuario/repo')).toBe(true)
    expect(repoValido('usuario/repo/de/mas')).toBe(false)
    expect(repoValido('solorepo')).toBe(false)
  })

  it('el correo se valida flojo a propósito', () => {
    // Quien valida de verdad es Let\'s Encrypt. Una expresión estricta aquí sólo
    // consigue rechazar direcciones que existen.
    expect(correoValido('avisos+orbit@ejemplo.com')).toBe(true)
    expect(correoValido('sin-arroba')).toBe(false)
    expect(correoValido('@ejemplo.com')).toBe(false)
  })
})

describe('el nombre que se propone', () => {
  it('sale del repositorio, para no teclear dos veces lo mismo', () => {
    expect(nombreDesdeRepo('usuario/mi-tienda')).toBe('mi-tienda')
    expect(nombreDesdeRepo('https://github.com/usuario/Mi-Tienda.git')).toBe('mi-tienda')
  })

  it('si de ahí no sale un nombre válido, no se propone ninguno', () => {
    // Proponer un nombre que el servidor va a rechazar es peor que no proponer:
    // el error llega igual, pero después de haberlo dado por bueno.
    expect(nombreDesdeRepo('usuario/---')).toBe('')
    expect(nombreValido(nombreDesdeRepo('usuario/Ünïcödé'))).toBeNull()
  })
})

describe('los cinco pasos', () => {
  it('son cinco, y la detección va antes del dominio', () => {
    expect(PASOS).toEqual(['Origen', 'Detección', 'Dominio', 'Extras', 'Repaso'])
  })

  it('cada paso se queja sólo de lo suyo, no de lo que aún no se ha preguntado', () => {
    // Un borrador recién abierto no tiene dominio, y decirlo en el primer paso
    // sería quejarse de una pantalla que todavía no se ha visto.
    const b = borradorNuevo()
    expect(problemasDe('Origen', b).join(' ')).toContain('repositorio')
    expect(problemasDe('Origen', b).join(' ')).not.toContain('dominio')
  })

  it('el paso de la detección no pide nada, porque el caso normal es no tocarlo', () => {
    expect(problemasDe('Detección', bueno())).toEqual([])
    expect(sinAjustes(borradorNuevo().ajustes)).toBe(true)
  })

  it('la carpeta es relativa al repositorio y no puede salirse de él', () => {
    const b = bueno()
    b.ajustes.carpeta = '/etc'
    expect(problemasDe('Detección', b).join(' ')).toContain('no empieza por')
    b.ajustes.carpeta = '../otro'
    expect(problemasDe('Detección', b).join(' ')).toContain('salir del repositorio')
  })

  it('un alias mal escrito se señala por su nombre', () => {
    const b = bueno({ alias: 'www.tienda.ejemplo.com, mal_.ejemplo.com' })
    expect(listaDeAlias(b)).toHaveLength(2)
    expect(problemasDe('Dominio', b).join(' ')).toContain('mal_.ejemplo.com')
  })

  it('con todo bien, ningún paso se queja', () => {
    for (const p of PASOS) expect(problemasDe(p, bueno())).toEqual([])
  })
})

describe('el aviso del certificado', () => {
  it('avisa antes de crear, que es el único momento en que sale gratis', () => {
    // Es el final F2 —publicada sin certificado— y es evitable escribiendo un
    // correo aquí en lugar de emitirlo a mano después.
    const a = avisoDelCertificado(bueno(), false)
    expect(a).toContain("Let's Encrypt")
  })

  it('«no lo he mirado» NO se pinta como «no hay»', () => {
    // Son dos cosas distintas, y decir la segunda sería afirmar algo que no se
    // sabe. Es la misma regla que el certificado de la portada.
    const sinMirar = avisoDelCertificado(bueno(), null)
    const sinCorreo = avisoDelCertificado(bueno(), false)
    expect(sinMirar).not.toBeNull()
    expect(sinMirar).not.toBe(sinCorreo)
    expect(sinMirar).toContain('No he mirado')
  })

  it('con correo escrito, o sin HTTPS, no hay nada que avisar', () => {
    expect(avisoDelCertificado(bueno({ correo: 'a@ejemplo.com' }), false)).toBeNull()
    expect(avisoDelCertificado(bueno({ https: false }), false)).toBeNull()
    expect(avisoDelCertificado(bueno(), true)).toBeNull()
  })
})

// ── La orden ────────────────────────────────────────────────────────────────

describe('la orden que se va a ejecutar', () => {
  // El mismo fichero que lee `crates/orbit-client/tests/orden_de_new.rs`. La
  // orden se construye dos veces —allí la que se ejecuta, aquí la que se
  // enseña— y una pantalla cuyo único argumento es «mira lo que va a pasar
  // antes de que pase» no puede enseñar una cosa y ejecutar otra.
  const contrato = JSON.parse(readFileSync('../tests/contrato/orden-de-new.json', 'utf8'))

  const desde = (e: Record<string, any>): Borrador => ({
    repo: e.repo,
    rama: e.rama,
    nombre: e.nombre,
    dominio: e.dominio,
    alias: (e.alias ?? []).join(', '),
    correo: e.correo ?? '',
    baseDeDatos: e.base_de_datos,
    https: e.https,
    ajustes: {
      carpeta: e.ajustes.carpeta ?? '',
      tipo: e.ajustes.tipo ?? '',
      build: leerAnulacion(e.ajustes.build),
      arranque: leerAnulacion(e.ajustes.arranque),
      outdir: leerAnulacion(e.ajustes.outdir),
    },
  })

  function leerAnulacion(v: unknown) {
    if (v === undefined || v === null) return { modo: 'detectar' } as const
    if (Array.isArray(v)) return { modo: 'vacia' } as const
    return { modo: 'valor', valor: String(v) } as const
  }

  it('el caso completo produce el argv del fichero compartido', () => {
    expect(argvDeNew(desde(contrato.entrada))).toEqual(contrato.argv)
  })

  it('y el caso normal, que no lleva ni una bandera de más', () => {
    expect(argvDeNew(desde(contrato.minimo.entrada))).toEqual(contrato.minimo.argv)
  })

  it('se puede enseñar entera antes de lanzarla', () => {
    expect(ordenDeNew(bueno())).toBe(
      "orbit new --yes --repo usuario/tienda --branch main --name tienda " +
        "--domain tienda.ejemplo.com --aliases ''",
    )
  })

  it('los alias se teclean con comas y se mandan con espacios', () => {
    // Se escriben separados por comas porque es lo cómodo de teclear, y se
    // mandan como los lee el servidor: `for a in $A_ALIASES`. Con comas
    // llegarían como un solo alias con una coma dentro, y eso viaja hasta el
    // `-d` de certbot.
    const v = argvDeNew(bueno({ alias: 'www.tienda.ejemplo.com, tienda.es' }))
    expect(v[v.indexOf('--aliases') + 1]).toBe('www.tienda.ejemplo.com tienda.es')
  })

  it('«--yes» no es «que sí a todo»', () => {
    // Es «acepta lo que está por defecto». No crea la base de datos y no abre el
    // editor del .env, porque esas dos preguntas tienen «no» por defecto.
    const o = ordenDeNew(bueno())
    expect(o).not.toContain('--db')
    expect(o).not.toContain('--purge')
  })

  it('«sin build» y «no digas nada» son dos órdenes distintas', () => {
    // Un campo de texto vacío no puede significar las dos cosas: «--build \'\'»
    // le dice al servidor que esta app no se compila, y callarse le deja
    // detectarlo. Son respuestas distintas y tienen que verse distintas.
    const callado = bueno()
    const sinBuild = bueno()
    sinBuild.ajustes.build = { modo: 'vacia' }

    expect(argvDeNew(callado)).not.toContain('--build')
    expect(argvDeNew(sinBuild)).toContain('--build')
    expect(ordenDeNew(sinBuild)).toContain("--build ''")
  })

  it('la lista de alias vacía se manda igualmente', () => {
    // Para el servidor «ninguno, y lo digo yo» y «no he dicho nada» son dos
    // casos, y con el segundo se inventa un «www.» que nadie ha pedido.
    const v = argvDeNew(bueno())
    expect(v[v.indexOf('--aliases') + 1]).toBe('')
  })

  it('renunciar al certificado se ve en la orden', () => {
    expect(argvDeNew(bueno())).not.toContain('--no-ssl')
    expect(argvDeNew(bueno({ https: false }))).toContain('--no-ssl')
  })
})

// ── Lo detectado, que se lee después y no antes ─────────────────────────────

describe('lo que Orbit acabó detectando', () => {
  it('sale del descriptor, sin interpretar nada', () => {
    const d = loDetectado({ type: 'next', build: 'pnpm build', start: 'pnpm start', appdir: '' })
    expect(d.map((x) => x.campo)).toEqual(['Tipo', 'Carpeta', 'Build', 'Arranque'])
    expect(d[0]!.valor).toBe('next')
  })

  it('un campo vacío se marca como vacío, no se pinta como desconocido', () => {
    // El descriptor son cadenas: «» quiere decir que ahí no hay nada, y eso se
    // enseña con su etiqueta —«sin build»—, nunca con un guion, que se leería
    // como «no lo sé». Es la misma regla que los números que no se saben.
    const d = loDetectado({ build: '' })
    expect(d[0]!.vacio).toBe(true)
  })

  it('lo que el servidor no manda no se inventa', () => {
    expect(loDetectado({ type: 'static' }).map((x) => x.campo)).toEqual(['Tipo'])
  })
})
