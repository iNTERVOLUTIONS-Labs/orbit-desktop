/**
 * El `orbit.json` generado desde una app que ya funciona.
 *
 * Lo que se juega aquí no es que el fichero sea bonito: es que **Orbit lo lea**.
 * Su lector tiene tres formas de ignorarlo en silencio —sin `type` descarta el
 * fichero entero, sin `jq` no lo abre, y una ruta que se sale del repositorio la
 * salta— así que generar algo que se parezca a un orbit.json y no lo sea es el
 * peor resultado posible: se sube al repositorio y el hueco aparece tres
 * despliegues más tarde.
 */
import { describe, expect, it } from 'vitest'
import { render } from '@testing-library/svelte'

import OrbitJson from '../src/componentes/OrbitJson.svelte'
import { generar, leerEspecificacion, rutaSegura } from '../src/lib/orbitjson'
import type { AppInfo, Entorno, Estado } from '../src/lib/contrato'

const ESTADO: Estado = {
  service: 'active', port: 3001, ssl: true, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 3, last_deploy: '20260830-120000', last_deploy_sha: 'abc',
}

const info = (config: Record<string, string>): AppInfo => ({
  name: 'tienda',
  path: '/srv/apps/tienda',
  config: { type: 'next', build: 'pnpm build', start: 'pnpm start', ...config },
  state: ESTADO,
  releases: ['20260830-120000'],
})

const entorno = (keys: string[]): Entorno => ({ schema: 1, app: 'tienda', keys })

const leer = (g: { texto: string }) => JSON.parse(g.texto)

/** `generar` recibe los **nombres**; el componente recibe el objeto entero. */
const claves = (k: string[]) => k

describe('sólo las claves que Orbit lee', () => {
  it('emite las del descriptor y ninguna más', () => {
    // Emitir una clave que Orbit no lee sería escribir en el repositorio de
    // alguien un ajuste que no hace nada y que dentro de seis meses alguien
    // intentará cambiar.
    const o = leer(generar(info({ appdir: 'apps/web', outdir: 'dist' }), null))
    expect(Object.keys(o).sort()).toEqual(['appdir', 'build', 'outdir', 'start', 'type'])
  })

  it('lo que el servidor no dice no se inventa', () => {
    const o = leer(generar(info({ outdir: '', docroot: '' }), null))
    expect(o).not.toHaveProperty('outdir')
    expect(o).not.toHaveProperty('docroot')
  })

  it('«appdir» en la raíz no se escribe, igual que hace orbit init', () => {
    // La clave existiría sin decir nada.
    expect(leer(generar(info({ appdir: '.' }), null))).not.toHaveProperty('appdir')
    expect(leer(generar(info({ appdir: '' }), null))).not.toHaveProperty('appdir')
    expect(leer(generar(info({ appdir: 'apps/web' }), null)).appdir).toBe('apps/web')
  })

  it('las booleanas van como booleanas, y sólo cuando valen que sí', () => {
    // En el descriptor son «yes»; en el fichero son `true`. Y `_ib` sólo las
    // escribe cuando valen que sí.
    const si = leer(generar(info({ spa: 'yes', php: 'yes' }), null))
    expect(si.spa).toBe(true)
    expect(si.php).toBe(true)
    const no = leer(generar(info({ spa: 'no', php: '' }), null))
    expect(no).not.toHaveProperty('spa')
    expect(no).not.toHaveProperty('php')
  })

  it('«shared» pasa de cadena a lista, que es como lo lee el fichero', () => {
    const o = leer(generar(info({ shared: 'uploads storage/logs' }), null))
    expect(o.shared).toEqual(['uploads', 'storage/logs'])
  })
})

describe('sin «type» el fichero entero se ignora', () => {
  it('y se dice, porque el fichero se ve igual de bien', () => {
    // Es la última línea de `_read_descriptor`. Un orbit.json sin type se sube,
    // parece correcto y no hace absolutamente nada.
    const g = generar(info({ type: '' }), null)
    expect(g.huecos.join(' ')).toContain('ignora el fichero entero')
  })

  it('con type no sobra ese aviso', () => {
    expect(generar(info({}), []).huecos.join(' ')).not.toContain('ignora el fichero entero')
  })
})

describe('el bloque env: nombres, nunca valores', () => {
  it('sale de los nombres que da el contrato', () => {
    const o = leer(generar(info({}), claves(['DATABASE_URL', 'SECRET_KEY'])))
    expect(Object.keys(o.env.vars)).toEqual(['DATABASE_URL', 'SECRET_KEY'])
    expect(JSON.stringify(o)).not.toContain('postgres://')
  })

  it('«no los he pedido» no es «esta app no tiene variables»', () => {
    // Escribir `env.vars: {}` afirmaría que no necesita ninguna. Un silencio no
    // es una afirmación, así que el bloque no se escribe y se dice que falta.
    const g = generar(info({}), null)
    expect(leer(g)).not.toHaveProperty('env')
    expect(g.huecos.join(' ')).toContain('Falta el bloque «env»')
  })

  it('una lista vacía SÍ es una respuesta y no deja hueco', () => {
    const g = generar(info({}), [])
    expect(leer(g)).not.toHaveProperty('env')
    expect(g.huecos.join(' ')).not.toContain('Falta el bloque')
  })
})

describe('no degradar un orbit.json que ya existía', () => {
  // Si a esta app la configuró un descriptor con `{"generate": 32}`, regenerar
  // el fichero poniendo `prompt` cambiaría el significado en silencio: donde
  // había «esto se genera solo» quedaría «pregúntaselo a alguien».
  const SPEC = [
    'SECRET_KEY\tgenerate\t48\tsecret\tClave de firma de la sesión',
    'ADMIN_EMAIL\tprompt\tCorreo del administrador\tplain\t',
    'DEBUG\tskip\t\tplain\t',
  ].join('\n')

  it('lee la especificación del descriptor', () => {
    const v = leerEspecificacion(SPEC)
    expect(v).toHaveLength(3)
    expect(v[0]).toEqual({
      clave: 'SECRET_KEY',
      modo: 'generate',
      argumento: '48',
      secreta: true,
      descripcion: 'Clave de firma de la sesión',
    })
  })

  it('y la reproduce en vez de aplanarla a «prompt»', () => {
    const o = leer(generar(info({ env_spec: SPEC }), claves(['SECRET_KEY', 'ADMIN_EMAIL', 'DEBUG'])))
    expect(o.env.vars.SECRET_KEY).toEqual({
      generate: 48,
      secret: true,
      desc: 'Clave de firma de la sesión',
    })
    expect(o.env.vars.ADMIN_EMAIL).toEqual({ prompt: 'Correo del administrador' })
    // «skip» es lo que Orbit deduce de una clave sin `generate` ni `prompt`.
    expect(o.env.vars.DEBUG).toEqual({})
  })

  it('el argumento de «generate» sigue siendo un número', () => {
    // Convertir 32 en "32" cambiaría lo que Orbit hace con él.
    const o = leer(generar(info({ env_spec: 'K\tgenerate\t32\tplain\t' }), []))
    expect(o.env.vars.K.generate).toBe(32)
  })

  it('las variables que existen y no estaban declaradas se añaden detrás', () => {
    const o = leer(generar(info({ env_spec: 'A\tprompt\tdi A\tplain\t' }), claves(['A', 'B'])))
    expect(Object.keys(o.env.vars)).toEqual(['A', 'B'])
    expect(o.env.vars.B).toEqual({ prompt: 'Valor de B' })
  })

  it('«env.file» sólo se escribe si no es el de siempre', () => {
    expect(leer(generar(info({ env_file: '.env' }), []))).not.toHaveProperty('env')
    const o = leer(generar(info({ env_file: '.env.produccion', env_spec: 'A\tprompt\t\tplain\t' }), []))
    expect(o.env.file).toBe('.env.produccion')
  })
})

describe('la pantalla', () => {
  it('dice por qué esto existe habiendo «orbit init»', () => {
    // `orbit init` vuelve a detectar sobre el repositorio, o sea exactamente lo
    // que se equivocó la primera vez. Esto copia lo que ya funciona.
    const { container } = render(OrbitJson, { app: 'tienda', info: info({}), entorno: null })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('volviendo a detectar')
    expect(t).toContain('se equivoca otra vez igual')
  })

  it('avisa de las tres formas que tiene Orbit de ignorarlo en silencio', () => {
    const { container } = render(OrbitJson, { app: 'tienda', info: info({}), entorno: null })
    const t = (container.querySelector('.avisos')?.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('ignora el fichero entero')
    expect(t).toContain('jq')
    expect(t).toContain('no se corrige, se ignora')
  })

  it('y dice que el fichero se puede subir a un repositorio público', () => {
    // Alguien que mira un fichero que habla de sus variables de entorno espera
    // lo contrario, así que se dice donde lo está mirando.
    const { container } = render(OrbitJson, {
      app: 'tienda',
      info: info({}),
      entorno: entorno(['SECRET_KEY']),
    })
    const t = (container.textContent ?? '').replace(/\s+/g, ' ')
    expect(t).toContain('nombres, no valores')
    expect(t).toContain('repositorio público')
  })

  it('sin descriptor no se inventa un fichero', () => {
    const { container } = render(OrbitJson, { app: 'tienda', info: null, entorno: null })
    expect(container.querySelector('.fichero')).toBeNull()
  })
})

describe('una ruta que Orbit descartaría no se escribe', () => {
  // Asimetría del servidor que no es evidente: el DESCRIPTOR puede contener
  // rutas que el orbit.json no admite. `--appdir` se valida al crear la app,
  // pero `--outdir` y `--docroot` no, y la detección escribe lo que encuentra.
  // O sea que una app perfectamente desplegada puede tener un `outdir` que,
  // puesto en un orbit.json, el despliegue descartaría con un aviso.
  it('la regla es la de «_safe_relpath», letra por letra', () => {
    expect(rutaSegura('apps/web')).toBe(true)
    expect(rutaSegura('.')).toBe(true)
    expect(rutaSegura('/etc')).toBe(false)
    expect(rutaSegura('../fuera')).toBe(false)
    expect(rutaSegura('apps/../otro')).toBe(false)
    // Una carpeta llamada «-rf» pasa cualquier validación razonable de nombre y
    // convierte el `cd` del build en un puñado de opciones.
    expect(rutaSegura('-rf')).toBe(false)
    expect(rutaSegura('apps/-rf')).toBe(false)
    expect(rutaSegura('con espacio')).toBe(false)
  })

  it('no se emite, y se dice cuál era', () => {
    // Emitirla sería lo peor de las tres opciones: el fichero se ve bien, se
    // sube, y la carpeta que publica no es la que pone ahí.
    const g = generar(info({ outdir: '../fuera' }), [])
    expect(leer(g)).not.toHaveProperty('outdir')
    expect(g.huecos.join(' ')).toContain('../fuera')
    expect(g.huecos.join(' ')).toContain('la descartaría con un aviso')
  })

  it('y una ruta normal sí se emite', () => {
    expect(leer(generar(info({ outdir: 'dist' }), [])).outdir).toBe('dist')
  })
})
