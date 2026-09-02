/**
 * Añadir un servidor e instalar Orbit en él.
 *
 * Estas dos pantallas existen porque la aplicación **no se podía empezar a
 * usar**: los servidores salían sólo del `~/.ssh/config`, y quien no tuviera ese
 * fichero —en Windows casi nadie— abría una lista vacía sin ninguna salida.
 *
 * Y la de instalar rompe a propósito una regla que el proyecto defendía: «el
 * cliente no escribe en el servidor nada que no sea una invocación de `orbit`».
 * Lo que hay que probar aquí es lo que la hace aceptable — que se enseña qué se
 * va a ejecutar, que se comprueba antes de tocar nada, y que al terminar no se
 * cree ni al código de salida ni a la prosa.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'

import AnadirServidor from '../src/componentes/AnadirServidor.svelte'
import InstalarOrbit from '../src/componentes/InstalarOrbit.svelte'
import { hitos } from '../src/lib/instalacion'
import { aliasValido, hostValido, usuarioValido } from '../src/lib/servidores'

// ── Las reglas de forma ─────────────────────────────────────────────────────

describe('las reglas de un servidor propio', () => {
  it('el guion inicial fuera: se lo comería el analizador de opciones de ssh', () => {
    expect(aliasValido('-oProxyCommand=lo-que-sea')).toBe(false)
    expect(hostValido('-oProxyCommand=lo-que-sea')).toBe(false)
    expect(usuarioValido('-x')).toBe(false)
  })

  it('y los comodines también: convertirían el alias en un patrón', () => {
    // `prod-*` en un ssh_config no es un nombre, es una regla que casa con
    // otros hosts.
    expect(aliasValido('prod-*')).toBe(false)
    expect(aliasValido('prod?')).toBe(false)
    expect(aliasValido('produccion')).toBe(true)
  })

  it('una IPv6 es un host válido', () => {
    expect(hostValido('2001:db8::1')).toBe(true)
    expect(hostValido('srv.ejemplo.com')).toBe(true)
    expect(hostValido('con espacio')).toBe(false)
  })
})

// ── El formulario ───────────────────────────────────────────────────────────

function montarAlta(props: Record<string, unknown> = {}) {
  const alGuardar = vi.fn()
  const alCerrar = vi.fn()
  const r = render(AnadirServidor, { yaUsados: [], alGuardar, alCerrar, ...props })
  return { ...r, alGuardar, alCerrar }
}

const escribir = (c: HTMLElement, id: string, v: string) => {
  const i = c.querySelector(`#${id}`) as HTMLInputElement
  i.value = v
  i.dispatchEvent(new Event('input', { bubbles: true }))
  return i
}

describe('añadir un servidor', () => {
  it('no se puede guardar vacío', () => {
    const { container } = montarAlta()
    expect((container.querySelector('.primario') as HTMLButtonElement).disabled).toBe(true)
  })

  it('con lo mínimo, sí', async () => {
    const { container, alGuardar } = montarAlta()
    escribir(container, 'ns-host', '203.0.113.10')
    await Promise.resolve()
    const b = container.querySelector('.primario') as HTMLButtonElement
    expect(b.disabled).toBe(false)
    b.click()
    expect(alGuardar).toHaveBeenCalled()
    expect(alGuardar.mock.calls[0]![0].usuario).toBe('root')
    expect(alGuardar.mock.calls[0]![0].puerto).toBe(22)
  })

  it('un alias que ya existe se rechaza MIENTRAS se escribe, y se dice por qué', async () => {
    // Es el accidente más caro de un cliente multiservidor: si el alias choca
    // con un Host del ~/.ssh/config, `ssh` resuelve el del fichero y la
    // aplicación enseña un servidor mientras habla con otro.
    const { container } = montarAlta({ yaUsados: ['vps-ovh'] })
    escribir(container, 'ns-host', '203.0.113.10')
    escribir(container, 'ns-alias', 'vps-ovh')
    await Promise.resolve()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('ssh usaría ése')
    expect((container.querySelector('.primario') as HTMLButtonElement).disabled).toBe(true)
  })

  it('el alias se propone a partir de la dirección', async () => {
    const { container } = montarAlta()
    escribir(container, 'ns-host', 'produccion.ejemplo.com')
    await Promise.resolve()
    expect((container.querySelector('#ns-alias') as HTMLInputElement).value).toBe('produccion')
  })

  it('y deja de proponerse en cuanto alguien lo toca', async () => {
    const { container } = montarAlta()
    escribir(container, 'ns-host', 'uno.ejemplo.com')
    escribir(container, 'ns-alias', 'mio')
    escribir(container, 'ns-host', 'dos.ejemplo.com')
    await Promise.resolve()
    expect((container.querySelector('#ns-alias') as HTMLInputElement).value).toBe('mio')
  })

  it('la clave va plegada, y dice que se guarda la RUTA', () => {
    // Alguien está a punto de escribir algo sobre una clave privada en un
    // formulario: la duda hay que resolverla donde aparece.
    const { container } = montarAlta()
    expect((container.querySelector('details') as HTMLDetailsElement).open).toBe(false)
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Se guarda la ruta, nunca la clave ni su frase de paso')
  })

  it('dice qué se guarda y qué no', () => {
    const { container } = montarAlta()
    const t = (container.querySelector('.nota')?.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Nada de lo que diga el servidor')
    expect(t).toContain('ninguna contraseña')
  })
})

// ── Los hitos del instalador ────────────────────────────────────────────────

describe('por dónde va la instalación', () => {
  it('el número sale del propio instalador, no de una lista de aquí', () => {
    // Cada sección se anuncia con un `N/13` delante. Si algún día son catorce,
    // esto dirá catorce sin tocarlo.
    const m = hitos('  ▸ 7/13  Instalando PostgreSQL')
    expect(m.total).toBe(13)
    expect(m.actual).toBe(7)
    expect(m.hechos).toBe(6)
  })

  it('el que corre NO está hecho', () => {
    // Darlo por hecho sería adelantarse a un `apt` que puede estar a punto de
    // fallar.
    const m = hitos('3/13  Creando el usuario')
    expect(m.pasos[2]!.estado).toBe('haciendo')
    expect(m.pasos[1]!.estado).toBe('hecho')
    expect(m.pasos[3]!.estado).toBe('pendiente')
  })

  it('manda el texto del servidor sobre la copia local', () => {
    const m = hitos('7/13  Instalando PostgreSQL 16 desde el repo oficial')
    expect(m.pasos[6]!.texto).toBe('Instalando PostgreSQL 16 desde el repo oficial')
  })

  it('un total distinto del esperado se respeta', () => {
    const m = hitos('2/20  Otra cosa')
    expect(m.total).toBe(20)
    expect(m.pasos).toHaveLength(20)
  })

  it('antes del primer paso no se da nada por empezado', () => {
    const m = hitos('Bienvenido a Ubuntu\nLeyendo listas de paquetes...')
    expect(m.actual).toBeNull()
    expect(m.hechos).toBe(0)
    expect(m.pasos.every((p) => p.estado === 'pendiente')).toBe(true)
  })

  it('la prosa que no tiene esa forma se ignora sin romper nada', () => {
    const m = hitos('Setting up libc6:amd64 (2.39-0ubuntu8) ...\n4/13  Instalando Node')
    expect(m.actual).toBe(4)
  })
})

// ── La pantalla de instalar ─────────────────────────────────────────────────

const REQ = {
  git: true, root: true, sudo_sin_contrasena: false, ya_instalado: false,
  sistema: 'ubuntu-24.04', puede: true, impedimentos: [], avisos: [],
  pasos: ['rm -rf /tmp/orbit-instalacion', 'git clone --depth 1 https://…/orbit.git /tmp/orbit-instalacion', 'cd /tmp/orbit-instalacion && sudo bash install.sh'],
}

function montarInstalar(props: Record<string, unknown> = {}) {
  const alInstalar = vi.fn()
  const alCancelar = vi.fn()
  const alCerrar = vi.fn()
  const r = render(InstalarOrbit, {
    alias: 'produccion', alInstalar, alCancelar, alCerrar, ...props,
  })
  return { ...r, alInstalar, alCancelar, alCerrar }
}

describe('instalar Orbit', () => {
  it('enseña la secuencia literal antes de ejecutarla', () => {
    const { container } = montarInstalar({ requisitos: REQ })
    const o = container.querySelector('.secuencia')?.textContent ?? ''
    expect(o).toContain('git clone')
    expect(o).toContain('sudo bash install.sh')
  })

  it('y NO es el `curl | sudo bash` que no funcionaba', () => {
    // install.sh lee el fichero `orbit` que tiene al lado y muere si no está.
    // Por una tubería no hay ninguno.
    const { container } = montarInstalar({ requisitos: REQ })
    expect(container.textContent).not.toContain('curl')
  })

  it('sin poder elevarse, el botón no se puede pulsar y se dice qué falta', () => {
    const req = {
      ...REQ, root: false, sudo_sin_contrasena: false, puede: false,
      impedimentos: [{ clase: 'sudo', que: 'Ese usuario necesita contraseña para sudo.', arreglo: null }],
    }
    const { container } = montarInstalar({ requisitos: req })
    expect((container.querySelector('.primario') as HTMLButtonElement).disabled).toBe(true)
    expect(container.textContent).toContain('necesita contraseña')
  })

  it('y la secuencia se enseña igualmente, para copiarla', () => {
    // Quien no pueda instalar desde aquí sigue pudiendo hacerlo a mano.
    const req = { ...REQ, puede: false, impedimentos: [{ clase: 'git', que: 'No hay git.', arreglo: 'apt-get install -y git' }] }
    const { container } = montarInstalar({ requisitos: req })
    // `.secuencia` y no `.orden` a secas: el arreglo de un impedimento también
    // es una orden, y son dos cosas distintas — una la ejecuta la aplicación y
    // la otra tiene que ejecutarla una persona.
    expect(container.querySelector('.secuencia')?.textContent).toContain('install.sh')
    expect(container.querySelector('.impedimento .orden')?.textContent).toContain('git')
  })

  it('un Orbit que ya está dice «reinstalar», no «instalar»', () => {
    const { container } = montarInstalar({ requisitos: { ...REQ, ya_instalado: true } })
    expect(container.querySelector('.primario')?.textContent?.trim()).toBe('Reinstalar')
  })

  it('mientras corre avisa de que tarda, y de que parar no deshace', () => {
    const { container } = montarInstalar({ instalando: true, salida: '2/13  Zona horaria' })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('cinco y diez minutos')
    expect(t).toContain('no deshace lo ya instalado')
  })

  it('mientras corre no se puede cerrar por error', () => {
    const { container } = montarInstalar({ instalando: true })
    expect(container.querySelector('.cerrar')).toBeNull()
  })

  it('el veredicto sale de preguntarle al servidor, no del código de salida', () => {
    // Un instalador que termina en 1 puede haber dejado Orbit funcionando, y uno
    // que termina en 0 no lo demuestra. Lo único que cuenta es que `orbit
    // version` conteste.
    const { container } = montarInstalar({
      resultado: { codigo: 1, salida: 'algo se torció al final', version: '1.3.6' },
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('Orbit 1.3.6 está instalado')
    expect(t).toContain('Lo dice él')
  })

  it('y un código 0 sin versión NO se cuenta como instalado', () => {
    const { container } = montarInstalar({
      resultado: { codigo: 0, salida: 'todo bien, dice él', version: null },
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('No ha quedado instalado')
    expect(t).toContain('no contesta')
  })
})

describe('un formulario vacío no está mal: está vacío', () => {
  // Salió de mirar la galería: el formulario aparecía en rojo nada más abrirlo,
  // con «Falta la dirección» y «Ponle un nombre» sin que nadie hubiera escrito
  // nada. Eso se lee como «esto está roto» justo en la pantalla donde alguien
  // empieza a usar la aplicación. Es la misma regla que `nombreValido('')`, que
  // devuelve null a propósito.
  it('no se queja antes de que hayan escrito nada', () => {
    const { container } = montarAlta()
    expect(container.querySelectorAll('.mal')).toHaveLength(0)
    expect(container.querySelectorAll('[aria-invalid="true"]')).toHaveLength(0)
  })

  it('pero sí al salir del campo', async () => {
    const { container } = montarAlta()
    const i = container.querySelector('#ns-host') as HTMLInputElement
    i.dispatchEvent(new Event('blur', { bubbles: true }))
    await Promise.resolve()
    expect(container.textContent).toContain('Falta la dirección')
  })

  it('y al intentar enviar se dice todo lo que falta de una vez', async () => {
    const { container, alGuardar } = montarAlta()
    ;(container.querySelector('form') as HTMLFormElement).dispatchEvent(
      new Event('submit', { bubbles: true, cancelable: true }),
    )
    await Promise.resolve()
    expect(container.querySelectorAll('.mal').length).toBeGreaterThan(1)
    expect(alGuardar).not.toHaveBeenCalled()
  })

  it('el choque de alias SÍ se dice mientras se escribe', async () => {
    // No es «te falta algo», es «esto no puede ser»: callarlo hasta el final
    // deja teclear un nombre entero para tirarlo después.
    const { container } = montarAlta({ yaUsados: ['vps-ovh'] })
    escribir(container, 'ns-alias', 'vps-ovh')
    await Promise.resolve()
    expect(container.querySelector('.mal')?.textContent).toContain('ssh usaría ése')
  })
})
