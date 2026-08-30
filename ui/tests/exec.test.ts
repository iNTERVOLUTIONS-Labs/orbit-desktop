/**
 * `exec`: la puerta trasera, y la pantalla más delicada del producto.
 *
 * El daño no está acotado. `orbit remove --purge` hace una cosa mala conocida;
 * `orbit exec` hace lo que le digas, con el entorno de la app, en un servidor de
 * producción.
 */
import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/svelte'
import { tick } from 'svelte'

import Exec from '../src/componentes/Exec.svelte'
import { parecePeligroso } from '../src/lib/contrato'

function montar(salida = { orden: 'orbit exec web ls', stdout: 'a\nb\n', stderr: '', codigo: 0 }) {
  const correr = vi.fn(async () => salida)
  const r = render(Exec, { app: 'web', servidor: 'vps-ovh', usuario: 'orbit-web', correr })
  return { ...r, correr }
}

async function escribir(container: HTMLElement, t: string) {
  const i = container.querySelector('input')!
  i.value = t
  i.dispatchEvent(new Event('input', { bubbles: true }))
  await tick()
}

describe('los cuatro datos siempre visibles', () => {
  it('dice dónde, en qué app, como quién y con qué entorno', () => {
    // Nada de eso se deduce mirando una caja de texto, y lo que se ejecuta aquí
    // corre en un servidor de producción.
    const { container } = montar()
    const c = container.querySelector('.cabecera')!.textContent!
    expect(c).toContain('web')
    expect(c).toContain('vps-ovh')
    expect(c).toContain('orbit-web')
    expect(c).toContain('.env')
  })

  it('advierte de los secretos una vez, no en cada comando', async () => {
    // Un aviso que sale siempre se aprende a ignorar, y entonces se ignoran
    // también los que importan.
    const { container } = montar()
    expect(container.querySelector('.secretos')).not.toBeNull()
    await escribir(container, 'ls')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(container.querySelector('.secretos')).toBeNull()
  })
})

describe('los dos modos, explícitos', () => {
  it('por defecto NO pasa por un shell', () => {
    // Es el modo que no sorprende: un `&&` es un argumento literal.
    const { container } = montar()
    const b = container.querySelectorAll('.modos button')
    expect(b[0]!.className).toContain('activo')
    expect(b[0]!.textContent).toContain('comando')
  })

  it('y se ve la diferencia antes de ejecutar', async () => {
    // Si la interfaz aplicara la heurística del servidor en silencio, quien
    // escribe no podría predecir cuándo su `&&` se ejecuta y cuándo se pasa
    // como texto. Una herramienta de depuración que no es predecible tampoco
    // sirve.
    const { container } = montar()
    await escribir(container, 'ls -la')
    const comoComando = container.querySelector('.previa')!.textContent
    ;(container.querySelectorAll('.modos button')[1] as HTMLButtonElement).click()
    await tick()
    const comoShell = container.querySelector('.previa')!.textContent
    expect(comoComando).not.toBe(comoShell)
  })

  it('el modo elegido llega al que ejecuta', async () => {
    const { container, correr } = montar()
    await escribir(container, 'ls -la')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(correr).toHaveBeenCalledWith(false, ['ls', '-la'])
  })
})

describe('la orden se enseña antes de ejecutarla', () => {
  it('aparece en cuanto se escribe algo', async () => {
    // Es lo que convierte «confío en la interfaz» en «he leído lo que va a
    // pasar», y es la mitigación que el usuario puede verificar por sí mismo.
    const { container } = montar()
    expect(container.querySelector('.previa')).toBeNull()
    await escribir(container, 'php artisan migrate')
    expect(container.querySelector('.previa')!.textContent).toContain('orbit exec web')
  })
})

describe('la lista de patrones', () => {
  it('para el error de dedos, y se dice que sólo hace eso', async () => {
    const { container, correr } = montar()
    await escribir(container, 'rm -rf /srv/apps/web')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick()
    // La primera pulsación NO ejecuta: avisa.
    expect(correr).not.toHaveBeenCalled()
    const aviso = container.querySelector('.peligro')!.textContent!
    expect(aviso).toContain('borra recursivamente')
    // Y se dice que es pedagógica, para que nadie la confunda con una defensa.
    expect(aviso).toContain('no protege de nada')
  })

  it('la segunda pulsación sí ejecuta: no se pregunta dos veces', async () => {
    // Preguntar otra vez tras confirmar enseña a pulsar dos veces sin leer
    // ninguna.
    const { container, correr } = montar()
    await escribir(container, 'drop database tienda')
    const b = container.querySelector('.lanzar') as HTMLButtonElement
    b.click(); await tick()
    b.click(); await tick(); await tick(); await tick()
    expect(correr).toHaveBeenCalledOnce()
  })

  it('un comando normal no pregunta nada', async () => {
    const { container, correr } = montar()
    await escribir(container, 'php artisan migrate')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(correr).toHaveBeenCalledOnce()
    expect(container.querySelector('.peligro')).toBeNull()
  })

  it('se salta sin proponérselo, y por eso es pedagógica', () => {
    // Hay mil formas de escribir un rm. Documentarlo así evita que alguien
    // construya encima suponiendo que protege.
    expect(parecePeligroso('rm -rf /srv')).not.toBeNull()
    expect(parecePeligroso('cd /srv && rm -rf apps')).toBeNull()
  })
})

describe('la salida', () => {
  it('se pinta como texto plano, nunca interpretada', async () => {
    // Es salida de un proceso arbitrario: puede traer ANSI, bytes nulos o megas
    // en una línea.
    const { container } = montar({
      orden: 'orbit exec web x', codigo: 0,
      stdout: '<img src=x onerror=alert(1)>\n', stderr: '',
    })
    await escribir(container, 'x')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('.chorro')!.textContent).toContain('<img')
  })

  it('un código distinto de cero se dice, y no es un fallo del transporte', async () => {
    // Un comando que sale con error es un comando que salió con error, y su
    // salida es lo que hay que enseñar.
    const { container } = montar({
      orden: 'orbit exec web falso', codigo: 127,
      stdout: '', stderr: 'command not found\n',
    })
    await escribir(container, 'falso')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(container.querySelector('.codigo--mal')?.textContent).toContain('127')
    expect(container.querySelector('.chorro--err')?.textContent).toContain('not found')
  })
})

describe('el histórico', () => {
  it('vive en memoria y no toca el disco', async () => {
    // Aquí se escriben cadenas de conexión con contraseña dentro más a menudo
    // de lo que parece. `bash` tomó esta decisión con HISTCONTROL; aquí el
    // valor por defecto va al revés, porque el ratio es mucho más alto.
    const { container } = montar()
    await escribir(container, 'ls')
    ;(container.querySelector('.lanzar') as HTMLButtonElement).click()
    await tick(); await tick(); await tick()
    expect(container.querySelector('.historico')).not.toBeNull()
    expect(container.textContent).toContain('Sólo en memoria')
  })
})

describe('la shell interactiva', () => {
  it('no se ofrece: se ofrece la orden para un terminal de verdad', () => {
    // `orbit exec app` sin comando abre un bash, y un cliente sin terminal no
    // puede con eso. Fingir medio terminal es la peor solución de todas.
    const { container } = montar()
    expect(container.querySelector('.copiar')?.textContent).toContain('terminal de verdad')
  })
})
