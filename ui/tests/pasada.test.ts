/**
 * La pasada por todas las apps.
 *
 * Es la orden más cara que este cliente puede lanzar —cuarenta builds— y la que
 * más fácil es describir mal, porque `deploy --all` significa dos cosas
 * distintas según lleve `--if-changed` o no. Casi todas las pruebas de aquí van
 * sobre esa diferencia y sobre lo que la pantalla puede prometer en cada caso.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'

import Pasada from '../src/componentes/Pasada.svelte'
import { finalesPosibles, leerPasada } from '../src/lib/despliegue'
import type { App, Estado, Lote } from '../src/lib/contrato'

const BASE: Estado = {
  service: 'active', port: 3001, ssl: true, cert_days: 80,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 3, last_deploy: '20260830-120000', last_deploy_sha: 'abc',
}
const app = (name: string, type = 'node'): App => ({
  name, type, domain: `${name}.ejemplo.com`, aliases: [], state: { ...BASE },
})

const LOTE: Lote = {
  schema: 1,
  apps: [
    { app: 'tienda', status: 'deployed', error: null, result: null },
    { app: 'blog', status: 'unchanged', error: null, result: null },
  ],
  total: 2, deployed: 1, failed: 0, unchanged: 1, unreachable: 0, gone: 0, skipped: 0,
  ok: true, duration_s: 84,
}

function montar(props: Record<string, unknown> = {}) {
  const alLanzar = vi.fn()
  const alCancelar = vi.fn()
  const alCerrar = vi.fn()
  const r = render(Pasada, {
    servidor: 'vps-ovh',
    apps: [app('tienda'), app('blog'), app('viejo', 'redirect')],
    alLanzar, alCancelar, alCerrar,
    ...props,
  })
  return { ...r, alLanzar, alCancelar, alCerrar }
}

// ── El lector del progreso ──────────────────────────────────────────────────

describe('leer el progreso de una pasada', () => {
  it('junta los dos niveles de suceso, que llegan por el mismo canal', () => {
    // `{"event":"app"}` marca el principio y el final de cada app;
    // `{"event":"step"}` los seis pasos de dentro. Los dos traen `app`, que es
    // el campo que Orbit añadió justo para poder atribuirlos.
    const p = leerPasada(`
{"event":"app","app":"tienda","status":"start","elapsed_s":0}
{"event":"step","app":"tienda","step":"code","status":"start","elapsed_s":0}
{"event":"step","app":"tienda","step":"build","status":"start","elapsed_s":4}
{"event":"app","app":"tienda","status":"deployed","elapsed_s":97}
{"event":"app","app":"blog","status":"start","elapsed_s":97}
{"event":"step","app":"blog","step":"code","status":"start","elapsed_s":0}
`)
    expect(p.apps.map((a) => a.app)).toEqual(['tienda', 'blog'])
    expect(p.apps[0]!.final).toBe('deployed')
    expect(p.terminadas).toBe(1)
    expect(p.enCurso).toBe('blog')
    expect(p.apps[1]!.paso).toBe('code')
  })

  it('el reloj de la pasada NO retrocede con el de una app', () => {
    // Son dos relojes distintos por el mismo canal: los sucesos `step` cuentan
    // desde que empezó **esa app** (DEP_T0) y los `app` desde que empezó la
    // pasada (DALL_T0). Mezclarlos hacía que el reloj saltara hacia atrás en
    // cada app nueva.
    const p = leerPasada(`
{"event":"app","app":"tienda","status":"deployed","elapsed_s":300}
{"event":"app","app":"blog","status":"start","elapsed_s":300}
{"event":"step","app":"blog","step":"code","status":"start","elapsed_s":2}
`)
    expect(p.transcurrido).toBe(300)
  })

  it('las apps salen en el orden en que el servidor las toca', () => {
    // Ordenarlas alfabéticamente escondería cuál va ahora, que es lo único que
    // se quiere mirar mientras corre.
    const p = leerPasada(`
{"event":"app","app":"zeta","status":"deployed","elapsed_s":10}
{"event":"app","app":"alfa","status":"start","elapsed_s":10}
`)
    expect(p.apps.map((a) => a.app)).toEqual(['zeta', 'alfa'])
  })

  it('una línea rota no tumba la pasada', () => {
    const p = leerPasada(`
{"event":"app","app":"tienda","status":"deployed","elapsed_s":10}
{"event":"app","app":"blog","stat
{"event":"app","app":"blog","status":"failed","elapsed_s":20}
`)
    expect(p.rotas).toBe(1)
    expect(p.terminadas).toBe(2)
  })

  it('la prosa que va por el mismo canal no cuenta como línea rota', () => {
    const p = leerPasada(`
  ✔ tienda
{"event":"app","app":"tienda","status":"deployed","elapsed_s":10}
  Desplegando 2 aplicaciones
`)
    expect(p.rotas).toBe(0)
    expect(p.terminadas).toBe(1)
  })

  it('un final que no conocemos llega a la pantalla, no se descarta', () => {
    // Un analizador que sólo admita seis valores es un cliente que se rompe el
    // día que Orbit añada el séptimo.
    const p = leerPasada('{"event":"app","app":"x","status":"aplazada","elapsed_s":1}')
    expect(p.apps[0]!.final).toBe('aplazada')
  })

  it('un suceso sin app se ignora: un paso sin dueño no vale para nada', () => {
    const p = leerPasada('{"event":"step","step":"build","status":"start","elapsed_s":1}')
    expect(p.apps).toHaveLength(0)
  })

  it('si el canal llega cortado por arriba, un paso basta para saber cuál va', () => {
    // Abrir la pantalla a mitad no puede dejarla sin saber qué está corriendo.
    const p = leerPasada('{"event":"step","app":"tienda","step":"build","status":"start","elapsed_s":9}')
    expect(p.enCurso).toBe('tienda')
  })
})

// ── Qué finales son posibles ────────────────────────────────────────────────

describe('qué puede salir de cada modo', () => {
  it('sin preguntar al remoto sólo hay dos finales posibles', () => {
    // Los cuatro baratos salen de preguntarle al remoto de cada app, y sin
    // `--if-changed` no se le pregunta a nadie.
    expect(finalesPosibles(false)).toEqual(['deployed', 'failed'])
  })

  it('«saltadas» no sale nunca, porque sólo lo produce el autodespliegue', () => {
    expect(finalesPosibles(true)).not.toContain('skipped')
    expect(finalesPosibles(false)).not.toContain('skipped')
  })
})

// ── La pantalla ─────────────────────────────────────────────────────────────

describe('antes de lanzarla', () => {
  it('dice a cuántas va a tocar, y descuenta las redirecciones', () => {
    const { container } = montar()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('3 apps')
    expect(t).toContain('1 es una redirección')
  })

  it('y no da por sabido lo que no sabe', () => {
    // `list --json` trae el tipo pero NO el repositorio, y el servidor también
    // salta las apps sin repositorio. Dar el número redondo sería afirmar algo
    // que no se ha mirado.
    const { container } = montar()
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('no lo dice')
    expect(t).toContain('alguna menos')
  })

  it('son dos entradas y no una casilla', () => {
    // La misma decisión que en la pantalla de retirar: la opción que está al
    // lado se elige sin leerla, y aquí la de al lado son cuarenta builds.
    const { container } = montar()
    expect(container.querySelector('details')).not.toBeNull()
    expect(container.querySelectorAll('input[type=checkbox]')).toHaveLength(0)
  })

  it('la barata es la principal', () => {
    const { container, alLanzar } = montar()
    ;(container.querySelector('.primaria') as HTMLButtonElement).click()
    expect(alLanzar).toHaveBeenCalledWith(true)
  })

  it('la cara dice cuánto cuesta antes de ofrecerse', () => {
    const { container } = montar()
    const t = (container.querySelector('details')?.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('2 releases nuevas')
    expect(t).toContain('código idéntico')
    // Y avisa de que en ese modo cuatro finales no pueden salir.
    expect(t).toContain('sólo puede terminar')
  })

  it('la cara está detrás del submenú, no al lado', () => {
    const { container, alLanzar } = montar()
    ;(container.querySelector('details .boton') as HTMLButtonElement).click()
    expect(alLanzar).toHaveBeenCalledWith(false)
  })
})

describe('mientras corre', () => {
  const crudo = [
    '{"event":"app","app":"tienda","status":"deployed","elapsed_s":90}',
    '{"event":"app","app":"blog","status":"start","elapsed_s":90}',
    '{"event":"step","app":"blog","step":"build","status":"start","elapsed_s":3}',
  ].join('\n')

  it('enseña cuál va y en qué paso', () => {
    const { container } = montar({ corriendo: true, crudo, modo: 'si-cambia' })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('1 app hecha')
    expect(t).toContain('blog')
    expect(t).toContain('build')
  })

  it('la lista es el progreso: no hay barra con un denominador inventado', () => {
    // El servidor salta las apps sin repositorio, así que el total no se sabe.
    // Una fracción sería un denominador inventado.
    const { container } = montar({ corriendo: true, crudo, modo: 'si-cambia' })
    expect(container.querySelector('progress')).toBeNull()
    expect(container.querySelectorAll('.curso li')).toHaveLength(2)
  })

  it('parar NO deshace, y lo dice donde está el botón', () => {
    // La palabra «cancelar» sugiere deshacer, y aquí no se deshace nada: lo que
    // para es el bucle.
    const { container } = montar({ corriendo: true, crudo, modo: 'si-cambia' })
    const t = (container.querySelector('.parar')?.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('no deshace lo ya desplegado')
  })

  it('mientras corre no se puede cerrar por error', () => {
    const { container } = montar({ corriendo: true, crudo })
    expect(container.querySelector('.cerrar')).toBeNull()
  })
})

describe('al terminar', () => {
  it('enseña los seis recuentos, sin agrupar', () => {
    const { container } = montar({ resultado: LOTE, modo: 'si-cambia' })
    expect(container.querySelectorAll('.recuentos .cuenta')).toHaveLength(6)
  })

  it('un cero que no PODÍA ser otra cosa se explica', () => {
    // Los cuatro baratos son cero por construcción cuando no se preguntó a
    // ningún remoto. Dejarlos ahí sin decirlo invita a leerlos como «he mirado
    // y no había nada».
    const { container } = montar({ resultado: { ...LOTE, unchanged: 0 }, modo: 'todo' })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('no se le preguntó a ningún remoto')
    // Con el nombre que el producto le da en todas partes: «al día», no «sin
    // cambios». Dos nombres para el mismo concepto es exactamente lo que este
    // vocabulario existe para evitar.
    expect(t).toContain('al día')
  })

  it('y en el modo barato no se explica lo que sí podía salir', () => {
    const { container } = montar({ resultado: LOTE, modo: 'si-cambia' })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).not.toContain('no se le preguntó a ningún remoto')
    // Pero sí se explica «saltadas», que no puede salir en ninguno de los dos.
    expect(t).toContain('sólo lo produce el autodespliegue')
  })
})

describe('una predicción no pisa una medida', () => {
  // La frase «estos recuentos son cero porque no se preguntó» los afirmaba sin
  // mirarlos. Salió de una captura: en la galería valían 1 y el texto seguía
  // diciendo que eran cero — la pantalla contradiciendo a su propia tabla.
  //
  // Si el servidor devuelve un final que yo daba por imposible, quien manda es
  // el servidor. Ese día el recuento es justo lo que hay que mirar.
  it('un final «imposible» que viene con contenido NO se explica como cero', () => {
    const { container } = montar({
      resultado: { ...LOTE, unchanged: 3, unreachable: 0, gone: 0 },
      modo: 'todo',
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    // Los que sí son cero se siguen explicando…
    expect(t).toContain('sin contacto')
    // …y el que no lo es, no.
    expect(t).not.toMatch(/recuentos de[^.]*al día/)
    // Pero su cifra sigue ahí, que es lo que importa.
    expect(container.querySelector('.lote--unchanged .n')?.textContent).toBe('3')
  })

  it('y si ninguno es cero, no hay nada que explicar', () => {
    const { container } = montar({
      resultado: { ...LOTE, unchanged: 1, unreachable: 1, gone: 1, skipped: 1 },
      modo: 'todo',
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).not.toContain('no se le preguntó a ningún remoto')
    expect(t).not.toContain('sólo lo produce el autodespliegue')
  })
})
