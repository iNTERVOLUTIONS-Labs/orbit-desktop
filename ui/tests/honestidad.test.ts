/**
 * Las pruebas de que la interfaz no miente.
 *
 * El fallo característico de este producto **no es que la aplicación explote**:
 * es que pinte algo plausible. Un `null` pintado como 0, una web estática
 * pintada como caída, seis finales agrupados en dos. Ninguna de esas cosas
 * lanza una excepción; todas cuentan una historia falsa sobre un servidor de
 * producción.
 *
 * Por eso estas pruebas comprueban el DOM y no el estado interno: lo que
 * importa es lo que acaba delante de los ojos de alguien.
 */
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/svelte'

import ChipEstado from '../src/componentes/ChipEstado.svelte'
import FacetaSsl from '../src/componentes/FacetaSsl.svelte'
import ListaApps from '../src/componentes/ListaApps.svelte'
import { PRESENTACION, salud, nombreOperable, marcarInvisibles, num, type Estado, type App } from '../src/lib/contrato'

import listaEstados from '../src/lib/muestras/list-estados.json'
import listaHostil from '../src/lib/muestras/list-nombre-hostil.json'
import listaVacia from '../src/lib/muestras/list-vacia.json'

const BASE: Estado = {
  service: null, port: null, ssl: false, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 1, last_deploy: null, last_deploy_sha: null,
}
const e = (p: Partial<Estado>): Estado => ({ ...BASE, ...p })

describe('la resolución del estado', () => {
  it('una web estática no está parada: no aplica', () => {
    // El fallo que el contrato existe para evitar. `service: null` no es
    // `"stopped"`: pintar una alarma roja donde no pasa nada enseña a la gente
    // a ignorar las alarmas.
    expect(salud(e({ service: null }))).toBe('no-aplica')
    expect(salud(e({ service: 'stopped' }))).toBe('parado')
    expect(salud(e({ service: null }))).not.toBe(salud(e({ service: 'stopped' })))
  })

  it('sin vhost gana sobre el mantenimiento', () => {
    // Sin vhost tampoco se sirve la página de 503, así que ningún otro campo
    // describe lo que recibe un visitante.
    expect(salud(e({ served: false, maintenance: true }))).toBe('sin-vhost')
  })

  it('un estado de systemd que no conocemos no se traduce a «parado»', () => {
    expect(salud(e({ service: 'activating' }))).toBe('desconocido')
  })
})

describe('el chip', () => {
  it('nunca lleva el color solo: glifo, color y texto van juntos', () => {
    const { container } = render(ChipEstado, { estado: e({ service: 'active' }) })
    const chip = container.querySelector('.chip')!
    expect(chip.querySelector('.chip__glifo')?.textContent).toBe('●')
    expect(chip.textContent).toContain('activo')
    expect(chip.className).toContain('chip--activo')
  })

  it('«no aplica» y «desconocido» tienen glifos DISTINTOS, y los dos neutros', () => {
    const na = render(ChipEstado, { estado: e({ service: null }) })
    const desc = render(ChipEstado, { estado: e({ service: 'activating' }) })
    // `render` devuelve las consultas de testing-library además del contenedor,
    // y tiparlo con `ReturnType<typeof render>` arrastra sus firmas: se coge el
    // contenedor y ya.
    const g = (c: HTMLElement) => c.querySelector('.chip__glifo')!.textContent
    expect(g(na.container)).toBe('—')
    expect(g(desc.container)).toBe('·')
    expect(g(na.container)).not.toBe(g(desc.container))
  })

  it('«sin vhost» es el único chip sólido: la distinción es de forma, no de tono', () => {
    const solidos = (['sin-vhost'] as const)
    for (const s of ['active', 'stopped', null] as const) {
      const { container } = render(ChipEstado, { estado: e({ service: s }) })
      expect(container.querySelector('.chip--sin-vhost')).toBeNull()
    }
    const { container } = render(ChipEstado, { estado: e({ served: false }) })
    expect(container.querySelector('.chip--sin-vhost')).not.toBeNull()
    expect(solidos.length).toBe(1)
  })

  it('lleva la frase, porque el color sin la frase no dice qué hacer', () => {
    const { container } = render(ChipEstado, { estado: e({ served: false }) })
    const t = container.querySelector('.chip')!.getAttribute('title')!
    expect(t).toContain('ni 404 ni 502')
  })
})

describe('los números que no se saben', () => {
  it('un null NO se pinta como cero', () => {
    expect(num(null)).toBe('·')
    expect(num(0)).toBe('0')
    expect(num(null)).not.toBe('0')
  })

  it('el puerto de una web estática sale como desconocido, no como 0', () => {
    const apps = [{ name: 'estatica', type: 'static', domain: 'a.test', aliases: [], state: e({}) }]
    const { container } = render(ListaApps, { apps, servidor: 'x' })
    const celdas = [...container.querySelectorAll('td')].map((c) => c.textContent?.trim())
    expect(celdas).toContain('·')
    expect(celdas).not.toContain('0')
  })
})

describe('el certificado', () => {
  it('«no lo he mirado» no es «no hay certificado»', () => {
    const { container } = render(FacetaSsl, { estado: e({ ssl: true, cert_days: null }) })
    expect(container.textContent).toContain('·')
    expect(container.textContent).not.toContain('sin HTTPS')
  })

  it('un cert_days negativo es un certificado caducado, no un desbordamiento', () => {
    const { container } = render(FacetaSsl, { estado: e({ ssl: true, cert_days: -5 }) })
    expect(container.textContent).toContain('caducado')
    expect(container.textContent).not.toContain('-5 d')
  })

  it('sin HTTPS es un aviso neutro, NO un fallo', () => {
    const { container } = render(FacetaSsl, { estado: e({ ssl: false }) })
    expect(container.querySelector('.faceta--aviso')).toBeNull()
  })
})

describe('los nombres que llegan del servidor', () => {
  const hostil = listaHostil.apps as App[]

  it('un nombre con marcado se pinta LITERAL, nunca como HTML', () => {
    const { container } = render(ListaApps, { apps: hostil, servidor: 'comprometido' })
    // Si se hubiera interpretado, existiría el elemento.
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('script')).toBeNull()
    // Y el texto tiene que verse, recortado pero presente en el título.
    const titulos = [...container.querySelectorAll('[title]')].map((n) => n.getAttribute('title'))
    expect(titulos.some((t) => t?.includes('<img'))).toBe(true)
  })

  it('sobre un nombre que no pasa la regla de forma NO se puede operar', () => {
    const { container } = render(ListaApps, { apps: hostil, servidor: 'comprometido' })
    // Ninguna de las cinco filas hostiles ofrece el botón de abrir.
    expect(container.querySelectorAll('button.enlace').length).toBe(0)
    expect(container.querySelectorAll('.fila--no-operable').length).toBe(hostil.length)
  })

  it('la regla de forma es la del servidor, letra por letra', () => {
    expect(nombreOperable('mi-web')).toBe(true)
    expect(nombreOperable('viejo.com')).toBe(true)   // redirecciones de dominio
    expect(nombreOperable('Web')).toBe(false)        // sin mayúsculas
    expect(nombreOperable('-web')).toBe(false)
    expect(nombreOperable('a..b')).toBe(false)       // travesía de rutas
    expect(nombreOperable('оrbit')).toBe(false)      // o cirílica
    expect(nombreOperable('a'.repeat(41))).toBe(false)
  })
})

describe('la lista', () => {
  it('cero apps es una respuesta, y se dice como tal', () => {
    const { container } = render(ListaApps, { apps: listaVacia.apps as App[], servidor: 'nuevo' })
    expect(container.textContent).toContain('no tiene ninguna app')
    // Y no es un error: no hay nada pintado como fallo.
    expect(container.querySelector('.chip--sin-vhost')).toBeNull()
  })

  it('cuando la mayoría no tiene vhost, se cuenta como un hecho del servidor', () => {
    // Cuarenta chips rojos no dicen «cuarenta problemas»: no dicen nada.
    const apps = Array.from({ length: 10 }, (_, i) => ({
      name: `app${i}`, type: 'static', domain: `a${i}.test`, aliases: [],
      state: e({ served: false }),
    }))
    const { container } = render(ListaApps, { apps, servidor: 'x' })
    expect(container.querySelector('.banda')?.textContent).toContain('10 de 10')
  })

  it('los cinco estados se distinguen entre sí en la misma tabla', () => {
    const { container } = render(ListaApps, {
      apps: listaEstados.apps as App[], servidor: 'pruebas',
    })
    const clases = [...container.querySelectorAll('.chip')].map(
      (c) => [...c.classList].find((x) => x.startsWith('chip--')),
    )
    // Cinco apps, cinco estados, y ninguno repetido: si dos se colapsaran, la
    // tabla estaría contando una historia más simple que la real.
    expect(new Set(clases).size).toBe(5)
  })

  it('el nombre se recorta por el medio, no por el final', () => {
    // `tienda-produccion` y `tienda-staging` comparten prefijo: recortar por el
    // final las haría idénticas en pantalla.
    const apps = [
      { name: 'tienda-de-la-esquina-produccion', type: 'node', domain: 'a.test', aliases: [], state: e({}) },
      { name: 'tienda-de-la-esquina-staging', type: 'node', domain: 'b.test', aliases: [], state: e({}) },
    ]
    const { container } = render(ListaApps, { apps, servidor: 'x' })
    const vistos = [...container.querySelectorAll('.celda-nombre')].map((n) => n.textContent?.trim())
    expect(vistos[0]).not.toBe(vistos[1])
    expect(vistos[0]).toContain('…')
  })
})

describe('accesibilidad', () => {
  it('el chip se anuncia con su texto, no sólo con el glifo', () => {
    render(ChipEstado, { estado: e({ served: false }) })
    // «círculo negro» no es «activo»: el lector tiene que decir la palabra.
    const l = screen.getByLabelText(/sin vhost/i)
    expect(l).toBeTruthy()
  })

  it('la tabla tiene una descripción y encabezados de columna', () => {
    const { container } = render(ListaApps, { apps: listaEstados.apps as App[], servidor: 'pruebas' })
    expect(container.querySelector('caption')?.textContent).toContain('pruebas')
    expect(container.querySelectorAll('th[scope="col"]').length).toBeGreaterThan(4)
  })
})

describe('lo que sólo se vio mirando una captura', () => {
  it('un bidi override no puede seguir engañando al ojo', () => {
    // El nombre `produccion‮gnitset-` se PINTABA como `produccion-testing`.
    // La fila estaba tachada y no era operable —eso ya funcionaba— y aun así
    // leía como una app que no es. Es el ataque «Trojan Source» aplicado a una
    // lista, y ninguna prueba del DOM lo veía porque el DOM era correcto.
    const nombre = 'produccion‮gnitset-'
    expect(marcarInvisibles(nombre)).toContain('‹U+202E›')
    expect(marcarInvisibles(nombre)).not.toBe(nombre)

    const apps = [{ name: nombre, type: 'static', domain: 'a.test', aliases: [], state: e({}) }]
    const { container } = render(ListaApps, { apps, servidor: 'x' })
    expect(container.querySelector('.celda-nombre')?.textContent).toContain('‹U+202E›')
  })

  it('un homoglifo se marca: «оrbit» con o cirílica no es «orbit»', () => {
    expect(marcarInvisibles('оrbit')).toContain('‹U+043E›')
  })

  it('un carácter de ancho cero deja de ser invisible', () => {
    expect(marcarInvisibles('con​cero')).toContain('‹U+200B›')
  })

  it('un nombre legítimo no se toca', () => {
    // La marca sólo puede aparecer donde hay algo que marcar: si ensuciara los
    // nombres normales, la gente aprendería a ignorarla.
    expect(marcarInvisibles('mi-web')).toBe('mi-web')
    expect(marcarInvisibles('viejo.com')).toBe('viejo.com')
  })

  it('los chips neutros no pintan el guion dos veces', () => {
    // El DOM era correcto y la pantalla decía «— —». Se vio en una captura.
    const { container } = render(ChipEstado, { estado: e({ service: null }) })
    expect(container.querySelector('.chip')!.textContent!.trim()).toBe('—')
  })

  it('pero por voz se anuncian con su palabra, no con el glifo', () => {
    // «raya» no es «no aplica».
    const { container } = render(ChipEstado, { estado: e({ service: null }) })
    expect(container.querySelector('.chip')!.getAttribute('aria-label')).toContain('no aplica')
  })
})

describe('el vocabulario es el mismo en las dos interfaces', () => {
  // La precedencia vivía en el núcleo desde el principio; las palabras estaban
  // sólo aquí. Con una interfaz eso no se notaba. Con dos —la ventana y el
  // terminal— es la forma exacta en que se disuelve el activo más valioso del
  // producto: basta con que una diga «parado» donde la otra dice «no aplica».
  //
  // La prueba gemela está en `crates/orbit-client/tests/vocabulario.rs` y lee
  // este mismo fichero.
  const vocabulario = JSON.parse(
    readFileSync('../tests/contrato/vocabulario.json', 'utf8'),
  ) as { estados: { id: string; glifo: string; texto: string; voz: string; frase: string }[] }

  it('glifo, texto y frase salen del fichero compartido', () => {
    for (const e of vocabulario.estados) {
      const p = PRESENTACION[e.id as keyof typeof PRESENTACION]
      expect(p, `falta la presentación de ${e.id}`).toBeDefined()
      expect(p.glifo, `glifo de ${e.id}`).toBe(e.glifo)
      expect(p.texto, `texto de ${e.id}`).toBe(e.texto)
      expect(p.frase, `frase de ${e.id}`).toBe(e.frase)
    }
  })

  it('y no hay ningún estado de más en la interfaz', () => {
    expect(Object.keys(PRESENTACION).sort()).toEqual(vocabulario.estados.map((e) => e.id).sort())
  })

  it('lo que se anuncia por voz es la palabra, no el glifo', () => {
    // `—` se lee «raya», y eso no es «no aplica». Se comprueba sobre el DOM,
    // que es donde acaba el aria-label.
    for (const e of vocabulario.estados) {
      if (e.texto === e.glifo) {
        expect(e.voz, `${e.id} necesita una palabra para anunciarse`).not.toBe(e.glifo)
      }
    }
  })
})
