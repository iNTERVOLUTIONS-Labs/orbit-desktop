/**
 * Las dos operaciones que cambian algo de verdad.
 *
 * Y una frase que hay que tener delante todo el rato, porque cambia el diseño
 * de la primera: **`orbit remove -y --purge` no pregunta absolutamente nada**.
 * El «escribe el nombre» de Orbit sólo ocurre sin `-y`, y `--purge`
 * cortocircuita la segunda pregunta. Como el cliente tiene que pasar `-y`, aquí
 * no hay red debajo: si esta pantalla se equivoca, no hay una segunda pregunta
 * en el servidor que la pare.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'
import { tick } from 'svelte'

import Retirar, { inventario } from '../src/componentes/Retirar.svelte'
import Revertir, { consecuencia } from '../src/componentes/Revertir.svelte'
import type { AppInfo, Entorno } from '../src/lib/contrato'

const INFO: AppInfo = {
  name: 'tienda', path: '/srv/apps/tienda',
  config: {},
  state: {
    service: 'active', port: 3001, ssl: true, cert_days: 60,
    maintenance: false, served: true, autodeploy: false, queue: false,
    releases: 5, last_deploy: '20260805-041230', last_deploy_sha: 'abc',
  },
  releases: ['20260830-120000', '20260829-031500', '20260828-101500'],
}
const ENV: Entorno = { schema: 1, app: 'tienda', keys: ['DB_PASSWORD', 'APP_KEY'] }

describe('retirar', () => {
  function montar() {
    const alRetirar = vi.fn()
    const r = render(Retirar, {
      app: 'tienda', servidor: 'vps-ovh', info: INFO, entorno: ENV,
      alRetirar, alCerrar: () => {},
    })
    return { ...r, alRetirar }
  }

  it('son dos operaciones, no una casilla', () => {
    // Una casilla junto a un botón se marca sin leerla, y ésta se lee o no se
    // lee nunca.
    const { container } = montar()
    expect(container.querySelector('input[type="checkbox"]')).toBeNull()
    expect(container.querySelectorAll('.boton').length).toBe(2)
  })

  it('la que borra datos está en un submenú, no al lado de la otra', () => {
    const { container } = montar()
    const det = container.querySelector('details')!
    expect(det.querySelector('.boton--peligroso')).not.toBeNull()
    // Y la reversible NO está dentro.
    expect(det.querySelectorAll('.boton').length).toBe(1)
  })

  it('retirar sin borrar no pide escribir el nombre', async () => {
    // Es reversible. Pedir el nombre para las dos hace que escribir el nombre
    // deje de significar nada.
    const { container } = montar()
    ;(container.querySelectorAll('.boton')[0] as HTMLButtonElement).click()
    await tick()
    expect(document.querySelector('.hoja input')).toBeNull()
    expect(document.querySelector('.orden')!.textContent).toBe('orbit remove tienda -y')
  })

  it('borrar los datos SÍ lo pide, siempre', async () => {
    // Orbit ya lo hace en su rama interactiva, y el cliente no puede ser más
    // permisivo que el terminal al que sustituye.
    const { container } = montar()
    ;(container.querySelector('.boton--peligroso') as HTMLButtonElement).click()
    await tick()
    expect(document.querySelector('.hoja input')).not.toBeNull()
    expect(document.querySelector<HTMLButtonElement>('.confirmar')!.disabled).toBe(true)
    expect(document.querySelector('.orden')!.textContent).toBe('orbit remove tienda -y --purge')
  })

  it('el inventario es de ESTA app y de este momento, no un texto genérico', () => {
    // «esto borrará 5 releases y 12 variables» para una acción concreta. Un
    // aviso genérico se lee una vez y se ignora siempre.
    const t = inventario(INFO, ENV, 'tienda')
    expect(t).toContain('3 releases')
    expect(t).toContain('2 variables')
    expect(t).toContain('20260805-041230')
  })

  it('el inventario cuenta las variables, nunca enseña una', () => {
    // Ni siquiera para despedirse de ellas.
    const t = inventario(INFO, ENV, 'tienda')
    expect(t).not.toContain('DB_PASSWORD')
    expect(t).not.toContain('APP_KEY')
  })

  it('dice qué se deshace y qué no', () => {
    const t = inventario(INFO, ENV, 'tienda')
    expect(t).toContain('se deshace')
    expect(t).toContain('esto no')
    // Y ofrece la única mitigación real que existe para un borrado irreversible.
    expect(t).toContain('orbit backup')
  })

  it('no hay ningún «no volver a preguntar»', () => {
    const { container } = montar()
    expect(container.textContent).not.toMatch(/no volver a preguntar|recordar/i)
  })
})

describe('revertir', () => {
  function montar(info = INFO) {
    const alRevertir = vi.fn()
    const r = render(Revertir, { info, servidor: 'vps-ovh', alRevertir })
    return { ...r, alRevertir }
  }

  it('la release activa se marca y NO se ofrece', () => {
    // Volver a ella reiniciaría el servicio y recargaría nginx para dejarlo
    // todo exactamente igual.
    const { container } = montar()
    const activa = container.querySelector('.release--activa')!
    expect(activa.textContent).toContain('sirviendo ahora')
    expect(activa.querySelector('button')).toBeNull()
  })

  it('la release se ELIGE de una lista, nunca se escribe', () => {
    const { container } = montar()
    expect(container.querySelector('input')).toBeNull()
    expect(container.querySelectorAll('.release button').length).toBe(2)
  })

  it('no pide escribir el nombre: la fricción se gasta donde no hay vuelta atrás', async () => {
    // Pedirla aquí enseña a teclear nombres sin leer, y entonces se teclea
    // también en el borrado.
    const { container } = montar()
    ;(container.querySelector('.release button') as HTMLButtonElement).click()
    await tick()
    expect(document.querySelector('.hoja input')).toBeNull()
  })

  it('dice cuántos despliegues se retrocede, no sólo la marca de tiempo', () => {
    // «Vas a volver 3 despliegues atrás» es información; «vas a volver a
    // 20260805-041230» no lo es.
    expect(consecuencia(2, false)).toContain('2 despliegues atrás')
    expect(consecuencia(1, false)).toContain('un despliegue atrás')
  })

  it('el aviso de las migraciones va SIEMPRE', () => {
    // El cliente no puede saber si hubo una; lo que puede es no dejar que se
    // olvide. El código vuelve atrás y los datos no.
    expect(consecuencia(1, false)).toContain('migración')
    expect(consecuencia(9, true)).toContain('migración')
  })

  it('el del autodespliegue sólo cuando está puesto, y con qué hacer', () => {
    // Es el error que más veces se comete: se revierte a las tres de la mañana
    // y a las 3:05 el temporizador vuelve a poner la versión rota.
    expect(consecuencia(1, true)).toContain('autodespliegue')
    expect(consecuencia(1, true)).toContain('Desactívalo antes')
    expect(consecuencia(1, false)).not.toContain('autodespliegue')
  })

  it('con una sola release lo dice, en vez de enseñar una lista vacía', () => {
    const { container } = montar({ ...INFO, releases: ['20260830-120000'] })
    expect(container.textContent).toContain('no hay a dónde volver')
  })
})
