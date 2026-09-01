/**
 * Que el fichero generado lo lea **Orbit**, no yo.
 *
 * El resto de las pruebas de `orbit.json` comprueban que el objeto tiene las
 * claves que creo que tiene que tener. Eso no prueba lo que hay que probar: lo
 * que decide si este fichero sirve es `_read_descriptor`, que lo abre con `jq` y
 * tiene **tres formas de ignorarlo en silencio**. Un fichero que parece correcto
 * y que el despliegue descarta es exactamente el peor resultado, porque se sube
 * al repositorio y el hueco aparece tres despliegues más tarde.
 *
 * Así que aquí se ejecutan **las expresiones literales del Orbit real** contra
 * el fichero generado, y se comprueba qué valores saldrían al otro lado.
 *
 * Si no hay `jq`, la prueba **falla**, no se salta. Es la lección de `make
 * test-strict` de Orbit: una suite que se salta una comprobación y sale en verde
 * es peor que una roja, porque enseña que el verde no quiere decir nada.
 */
import { execFileSync } from 'node:child_process'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

import { generar } from '../src/lib/orbitjson'
import type { AppInfo, Estado } from '../src/lib/contrato'

const ESTADO: Estado = {
  service: 'active', port: 3001, ssl: true, cert_days: null,
  maintenance: false, served: true, autodeploy: false, queue: false,
  releases: 3, last_deploy: '20260830-120000', last_deploy_sha: 'abc',
}

const RICA: AppInfo = {
  name: 'tienda',
  path: '/srv/apps/tienda',
  config: {
    type: 'laravel',
    appdir: 'apps/web',
    build: 'composer install --no-dev && npm ci && npm run build',
    docroot: 'public',
    php: 'yes',
    shared: 'storage/app storage/logs public/uploads',
    env_spec: [
      'APP_KEY\tgenerate\t32\tsecret\tClave de cifrado',
      'DB_PASSWORD\tprompt\tContraseña de la base de datos\tsecret\t',
      'DEBUG\tskip\t\tplain\t',
    ].join('\n'),
  },
  state: ESTADO,
  releases: ['20260830-120000'],
}

function fichero(info: AppInfo, nombres: string[] | null): string {
  const dir = mkdtempSync(join(tmpdir(), 'orbitjson-'))
  const f = join(dir, 'orbit.json')
  writeFileSync(f, generar(info, nombres).texto)
  return f
}

/** `_d` del Orbit real, línea 1813. */
function _d(f: string, clave: string): string {
  return execFileSync(
    'jq',
    ['-r', '--arg', 'k', clave, 'if has($k) then (.[$k] | tostring) else "" end', f],
    { encoding: 'utf8' },
  ).trimEnd()
}

describe('el fichero generado pasa por el lector de Orbit', () => {
  it('hay jq: sin él esta prueba no comprueba nada y no puede salir en verde', () => {
    // La lección de `make test-strict`: una suite que se salta un requisito y
    // sale en verde enseña que el verde no quiere decir nada.
    expect(() => execFileSync('jq', ['--version'])).not.toThrow()
  })

  it('es JSON válido, que es la primera criba', () => {
    // `jq -e . fichero` — línea 1811. Si falla, Orbit avisa y lo ignora entero.
    const f = fichero(RICA, ['APP_KEY', 'DB_PASSWORD', 'DEBUG'])
    expect(() => execFileSync('jq', ['-e', '.', f], { stdio: 'pipe' })).not.toThrow()
  })

  it('«type» llega, que es lo que decide si el fichero cuenta o no', () => {
    const f = fichero(RICA, [])
    expect(_d(f, 'type')).toBe('laravel')
  })

  it('las cadenas llegan enteras, con sus «&&» dentro', () => {
    // `build` pasa por `tostring`, no por un shell. Un `&&` dentro es texto.
    const f = fichero(RICA, [])
    expect(_d(f, 'build')).toBe('composer install --no-dev && npm ci && npm run build')
    expect(_d(f, 'appdir')).toBe('apps/web')
    expect(_d(f, 'docroot')).toBe('public')
  })

  it('las booleanas llegan como «true», que es lo que Orbit compara', () => {
    // `[[ "$v" == "true" || "$v" == "yes" ]]` — línea 1829. Un `true` de JSON
    // pasa por `tostring` y sale como la cadena «true».
    const f = fichero(RICA, [])
    expect(_d(f, 'php')).toBe('true')
  })

  it('«shared» sale como lista, con la expresión que la lee', () => {
    // `jq -r 'if has("shared") then (.shared[]?) else empty end'` — línea 1836.
    const f = fichero(RICA, [])
    const v = execFileSync(
      'jq',
      ['-r', 'if has("shared") then (.shared[]?) else empty end', f],
      { encoding: 'utf8' },
    )
      .trim()
      .split('\n')
    expect(v).toEqual(['storage/app', 'storage/logs', 'public/uploads'])
  })

  it('el bloque env se lee con la expresión de «_read_env_block», modo a modo', () => {
    // Es la expresión literal de la línea 1857, la que produce el TSV que Orbit
    // guarda en el descriptor. Si el fichero generado no la satisface, la
    // especificación se pierde en silencio.
    const f = fichero(RICA, ['APP_KEY', 'DB_PASSWORD', 'DEBUG', 'MAIL_FROM'])
    const tsv = execFileSync(
      'jq',
      [
        '-r',
        `(.env.vars // {}) | to_entries[] |
         [ .key,
           (if .value.generate then "generate" elif .value.prompt then "prompt" else "skip" end),
           (.value.generate // .value.prompt // ""),
           (if .value.secret then "secret" else "plain" end),
           (.value.desc // "")
         ] | @tsv`,
        f,
      ],
      { encoding: 'utf8' },
      // Sólo el salto final, y NO `trimEnd()`: el último campo de la última
      // fila puede estar vacío —una variable sin descripción— y `@tsv` no pone
      // tabulador detrás, así que `trimEnd()` se comía el campo y la fila salía
      // con cuatro columnas en vez de cinco. Es lo mismo que hace el servidor,
      // donde `$( )` recorta saltos y no tabuladores.
    ).replace(/\n$/, '')

    const filas = tsv.split('\n').map((l) => l.split('\t'))
    expect(filas).toEqual([
      ['APP_KEY', 'generate', '32', 'secret', 'Clave de cifrado'],
      ['DB_PASSWORD', 'prompt', 'Contraseña de la base de datos', 'secret', ''],
      // «skip» sobrevive al viaje de ida y vuelta: sin `generate` ni `prompt`,
      // Orbit vuelve a deducirlo.
      ['DEBUG', 'skip', '', 'plain', ''],
      ['MAIL_FROM', 'prompt', 'Valor de MAIL_FROM', 'plain', ''],
    ])
  })

  it('«env.file» por defecto no se escribe, y Orbit pone «.env» igual', () => {
    // `.env.file // ".env"` — línea 1855.
    const f = fichero(RICA, [])
    const v = execFileSync('jq', ['-r', '.env.file // ".env"', f], { encoding: 'utf8' }).trim()
    expect(v).toBe('.env')
  })

  it('un fichero sin bloque env no rompe la lectura del bloque env', () => {
    // `jq -e 'has("env")'` sale distinto de cero y Orbit vuelve sin tocar nada.
    const f = fichero({ ...RICA, config: { type: 'static' } }, null)
    let tiene = true
    try {
      execFileSync('jq', ['-e', 'has("env")', f], { stdio: 'pipe' })
    } catch {
      tiene = false
    }
    expect(tiene).toBe(false)
  })

  it('las claves que NO se emiten no llegan como una cadena vacía por accidente', () => {
    // `_d` devuelve "" tanto para la clave ausente como para una clave con
    // valor vacío, y Orbit trata las dos igual (`[[ -n "$v" ]]`). O sea que un
    // hueco de verdad y una clave puesta a "" son indistinguibles al otro lado:
    // por eso aquí no se emite nunca una clave vacía.
    const f = fichero({ ...RICA, config: { type: 'static', build: '', start: '' } }, [])
    expect(_d(f, 'build')).toBe('')
    expect(_d(f, 'start')).toBe('')
    const claves = execFileSync('jq', ['-r', 'keys[]', f], { encoding: 'utf8' }).trim().split('\n')
    expect(claves).not.toContain('build')
    expect(claves).not.toContain('start')
  })
})
