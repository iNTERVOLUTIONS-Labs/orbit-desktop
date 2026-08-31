/**
 * El formulario de web nueva.
 *
 * Las pruebas de aquí son casi todas sobre lo que la pantalla **no** dice, y no
 * es casualidad: el asistente se juega su valor en tres momentos concretos en
 * los que es fácil prometer de más.
 *
 *   · El paso de la detección, que no puede enseñar lo detectado porque todavía
 *     no se ha clonado nada.
 *   · El aviso del certificado, que evita el final F2 y que no puede afirmar
 *     que el servidor no tiene correo cuando no se ha mirado.
 *   · El repaso, que enseña la orden literal — y que enseñe una y se ejecute
 *     otra sería el peor fallo posible de una pantalla cuyo único argumento es
 *     «mira lo que va a pasar antes de que pase».
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'
import { tick } from 'svelte'

import AsistenteNueva from '../src/componentes/AsistenteNueva.svelte'
import LoDetectado from '../src/componentes/LoDetectado.svelte'
import { borradorNuevo, type Borrador, type Paso } from '../src/lib/asistente'

function lleno(cambios: Partial<Borrador> = {}): Borrador {
  return {
    ...borradorNuevo(),
    repo: 'usuario/tienda',
    nombre: 'tienda',
    dominio: 'tienda.ejemplo.com',
    ...cambios,
  }
}

function montar(paso: Paso = 'Origen', inicial?: Borrador, extra: Record<string, unknown> = {}) {
  const alCrear = vi.fn()
  const alCerrar = vi.fn()
  const alResolver = vi.fn()
  const r = render(AsistenteNueva, {
    servidor: 'vps-ovh',
    pasoInicial: paso,
    inicial,
    alCrear,
    alCerrar,
    alResolver,
    ...extra,
  })
  return { ...r, alCrear, alCerrar, alResolver }
}

describe('los cinco pasos', () => {
  it('el raíl dice cuántos quedan, que es lo que distingue un formulario de un pozo', () => {
    const { container } = montar()
    const tramos = container.querySelectorAll('.tramo')
    expect(tramos.length).toBe(5)
    expect(container.textContent).toContain('Detección')
    expect(container.textContent).toContain('Repaso')
  })

  it('no se puede saltar hacia delante desde el raíl', () => {
    // Saltarse una validación no es navegar. Los pasos de delante están
    // deshabilitados hasta que se llega a ellos.
    const { container } = montar('Origen')
    const botones = [...container.querySelectorAll('.tramo button')] as HTMLButtonElement[]
    expect(botones[0]!.disabled).toBe(false)
    expect(botones.slice(1).every((x) => x.disabled)).toBe(true)
  })

  it('«Siguiente» no se puede pulsar con el paso a medias', async () => {
    const { container } = montar('Origen')
    const siguiente = container.querySelector('.primaria') as HTMLButtonElement
    expect(siguiente.disabled).toBe(true)
  })

  it('y sí con el paso resuelto', () => {
    const { container } = montar('Origen', lleno())
    expect((container.querySelector('.primaria') as HTMLButtonElement).disabled).toBe(false)
  })

  it('el primer paso no se queja de lo que aún no se ha preguntado', () => {
    const { container } = montar('Origen')
    const p = container.querySelector('.problema')?.textContent ?? ''
    expect(p).toContain('repositorio')
    expect(p).not.toContain('dominio')
  })
})

describe('el paso de la detección', () => {
  it('no enseña lo detectado, porque todavía no se ha detectado nada', () => {
    // `detect_stack` lee un directorio, y el directorio no existe hasta que
    // `orbit new` ha clonado. Enseñar aquí un «Tipo: next» sería enseñar una
    // promesa, y una promesa junto a una casilla se lee como un hecho.
    const { container } = montar('Detección', lleno())
    expect(container.textContent).toContain('al clonarlo')
    expect(container.textContent).toContain('todavía no puedo enseñarte')
  })

  it('está vacío por defecto y no obliga a nada', () => {
    const { container } = montar('Detección', lleno())
    expect(container.querySelector('.problema')).toBeNull()
    expect((container.querySelector('.primaria') as HTMLButtonElement).disabled).toBe(false)
  })

  it('los ajustes van plegados: el caso normal es no tocarlos', () => {
    const { container } = montar('Detección', lleno())
    const d = container.querySelector('details') as HTMLDetailsElement
    expect(d.open).toBe(false)
  })

  it('pero se abren solos si el borrador ya los trae', () => {
    // Volver a un paso no puede esconder lo que ya se rellenó: un ajuste puesto
    // detrás de un desplegable cerrado es un ajuste que se aplica sin verse.
    const b = lleno()
    b.ajustes.carpeta = 'apps/web'
    const { container } = montar('Detección', b)
    expect((container.querySelector('details') as HTMLDetailsElement).open).toBe(true)
  })

  it('la carpeta se presenta como lo que invalida a los demás, no como un campo más', () => {
    // Cambiarla redirige la detección entera. Ponerla en la misma lista que
    // «build» y «arranque» daría a entender que es un ajuste al lado de los
    // otros, y es el que hace que los otros se lean contra otro directorio.
    const { container } = montar('Detección', lleno())
    // Normalizado: el texto va partido por el <strong> y por el sangrado, y una
    // prueba que dependa de dónde parte la línea se rompe al reformatear.
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Cambia dónde mira todo lo demás')
  })

  it('«sin build» y «lo detecta Orbit» son dos estados visibles distintos', async () => {
    // Un campo de texto en blanco no puede significar las dos cosas.
    const { container } = montar('Detección', lleno())
    expect(container.textContent).toContain('lo detecta Orbit')

    const cambiar = [...container.querySelectorAll('.tri button')] as HTMLButtonElement[]
    cambiar[0]!.click() // → valor
    await tick()
    expect(container.querySelectorAll('.tri input').length).toBe(1)

    cambiar[0]!.click() // → vacía
    await tick()
    expect(container.textContent).toContain('sin build')
    expect(container.querySelector('.anulacion--vacia')).not.toBeNull()
  })

  it('ofrece la salida de emergencia, que es irse de la interfaz', () => {
    // La única recomendación del producto que manda al usuario fuera, y está
    // bien que exista: el descriptor manda sobre la detección.
    const { container } = montar('Detección', lleno())
    expect(container.textContent).toContain('orbit init')
    expect(container.textContent).toContain('orbit.json')
  })
})

describe('el dominio', () => {
  it('avisa antes de crear cuando el DNS no apunta aquí', () => {
    // Es el final F7 convertido en aviso previo. Una web publicada cuyo dominio
    // no apunta al servidor se ve, desde dentro, igual que una que sí.
    const { container } = montar('Dominio', lleno(), {
      resolucion: { del_dominio: ['1.2.3.4'], del_servidor: ['10.0.0.5'], coinciden: false },
    })
    expect(container.textContent).toContain('no apunta a este servidor')
    expect(container.textContent).toContain('1.2.3.4')
  })

  it('pero no impide seguir: un proxy delante da direcciones distintas y no está roto', () => {
    const { container } = montar('Dominio', lleno(), {
      resolucion: { del_dominio: ['1.2.3.4'], del_servidor: ['10.0.0.5'], coinciden: false },
    })
    expect((container.querySelector('.primaria') as HTMLButtonElement).disabled).toBe(false)
  })

  it('«no lo sé» NO se pinta como «no coincide»', () => {
    // La misma regla que el certificado sin comprobar de la portada: son dos
    // cosas distintas, y decir la segunda sería afirmar algo que no se sabe.
    const { container } = montar('Dominio', lleno(), {
      resolucion: { del_dominio: [], del_servidor: ['10.0.0.5'], coinciden: null },
    })
    expect(container.textContent).toContain('no quiere decir que esté mal')
    expect(container.textContent).not.toContain('no apunta a este servidor')
    expect(container.querySelector('.aviso-dns--sin-mirar')).not.toBeNull()
  })

  it('dice que lo resuelve esta máquina, no el mundo', () => {
    const { container } = montar('Dominio', lleno(), {
      resolucion: { del_dominio: ['10.0.0.5'], del_servidor: ['10.0.0.5'], coinciden: true },
    })
    expect(container.textContent).toContain('Lo resuelve esta máquina')
  })
})

describe('los extras', () => {
  it('el correo del certificado se pide antes, que es cuando sale gratis', () => {
    const { container } = montar('Extras', lleno(), { correoEnElServidor: false })
    expect(container.textContent).toContain("Let's Encrypt")
  })

  it('y dice dónde se guarda, que es el servidor y no la app', () => {
    // Evita el «¿por qué ya no me lo pregunta?» de dentro de tres meses.
    const { container } = montar('Extras', lleno(), { correoEnElServidor: false })
    expect(container.textContent).toContain('no en la de esta web')
  })

  it('con el correo puesto no queda nada que avisar', async () => {
    const { container } = montar('Extras', lleno({ correo: 'a@ejemplo.com' }), {
      correoEnElServidor: false,
    })
    expect(container.querySelector('.aviso-cert')).toBeNull()
  })

  it('la base de datos NO viene marcada', () => {
    // «--yes» es «acepta lo que está por defecto», y el valor por defecto de
    // esta pregunta es «no».
    const { container } = montar('Extras', lleno())
    const casillas = [...container.querySelectorAll('input[type=checkbox]')] as HTMLInputElement[]
    expect(casillas.find((c) => !c.checked)).toBeDefined()
  })

  it('no ofrece el autodespliegue, porque «new» no lo hace', () => {
    // Ofrecerlo aquí daría a entender que la orden lo configura. Es una orden
    // aparte y se activa cuando la web ya existe.
    const { container } = montar('Extras', lleno())
    expect(container.querySelector('.salida')?.textContent).toContain('orden aparte')
  })
})

describe('el repaso', () => {
  it('enseña la orden literal, no un resumen de la orden', () => {
    const { container } = montar('Repaso', lleno())
    const o = container.querySelector('.orden')?.textContent ?? ''
    expect(o).toContain('orbit new --yes')
    expect(o).toContain('--repo usuario/tienda')
    expect(o).toContain('--domain tienda.ejemplo.com')
  })

  it('un argumento vacío se VE en la orden', () => {
    // `--build ''` es una instrucción, no un hueco, y si no se ve no se puede
    // comprobar antes de pulsar.
    const b = lleno()
    b.ajustes.build = { modo: 'vacia' }
    const { container } = montar('Repaso', b)
    expect(container.querySelector('.orden')?.textContent).toContain("--build ''")
  })

  it('dice que puede terminar a medias, antes de empezar', () => {
    // `orbit new` despliega por dentro y puede salir con 1 con la app creada.
    // Que eso sorprenda al terminar es peor que decirlo al empezar.
    const { container } = montar('Repaso', lleno())
    expect(container.textContent).toContain('puede terminar a medias')
  })

  it('explica qué es «--yes» donde se ve el «--yes»', () => {
    const { container } = montar('Repaso', lleno())
    expect(container.textContent).toContain('no es «que sí a todo»')
  })

  it('crear devuelve el borrador entero a quien lo pidió', () => {
    const { container, alCrear } = montar('Repaso', lleno())
    ;(container.querySelector('.primaria') as HTMLButtonElement).click()
    expect(alCrear).toHaveBeenCalledOnce()
    expect(alCrear.mock.calls[0]![0].nombre).toBe('tienda')
  })
})

describe('lo que Orbit acabó detectando', () => {
  it('sale del descriptor, después de crear', () => {
    const { container } = render(LoDetectado, {
      app: 'tienda',
      config: { type: 'next', build: 'pnpm build', start: 'pnpm start' },
    })
    expect(container.textContent).toContain('next')
    expect(container.textContent).toContain('pnpm build')
  })

  it('un campo vacío lleva su etiqueta, nunca un guion', () => {
    // Un guion se lee como «no lo sé», y el descriptor sabe perfectamente que
    // ahí no hay nada. Es la misma regla con la que un null no se pinta como 0.
    const { container } = render(LoDetectado, { app: 'tienda', config: { build: '' } })
    expect(container.textContent).toContain('está vacío')
    expect(container.querySelector('dd')?.textContent).not.toContain('—')
  })

  it('no se inventa un porqué que el descriptor no guarda', () => {
    // El diseño lo pedía —«next porque package.json trae next»— y el descriptor
    // guarda el resultado de la detección, no las pruebas. Un motivo verosímil
    // inventado es peor que ninguno.
    const { container } = render(LoDetectado, { app: 'tienda', config: { type: 'next' } })
    expect(container.textContent).not.toContain('porque')
  })

  it('lo que el servidor no manda no aparece', () => {
    const { container } = render(LoDetectado, { app: 'tienda', config: { type: 'static' } })
    expect(container.querySelectorAll('dl > div').length).toBe(1)
  })
})

describe('los controles deshabilitados se ven deshabilitados', () => {
  it('«Atrás» en el primer paso no invita a pulsarlo', () => {
    // Un control deshabilitado que se ve igual que uno activo es peor que no
    // tenerlo: invita a pulsarlo y no contesta. Salió de mirar una captura con
    // el DOM ya en verde, que es de donde salen todos los de esta clase.
    const { container } = montar('Origen', lleno())
    const atras = [...container.querySelectorAll('footer button')].find(
      (x) => x.textContent?.trim() === 'Atrás',
    ) as HTMLButtonElement
    expect(atras.disabled).toBe(true)
    expect(getComputedStyle(atras).opacity).not.toBe('1')
  })
})
