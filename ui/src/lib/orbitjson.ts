// Generar el `orbit.json` de una app que **ya funciona**.
//
// `orbit init` escribe este fichero corriendo la detección sobre el repositorio,
// en el portátil de quien lo ejecuta. Eso tiene un problema que el propio Orbit
// documenta al despedirse de esa orden: la detección se equivoca en un monorepo,
// con un adaptador o con un arranque propio — y `orbit init` **se vuelve a
// equivocar igual**, porque hace exactamente lo mismo que se equivocó la primera
// vez.
//
// Aquí se hace al revés, y es la única ventaja que este cliente tiene sobre esa
// orden: se lee el descriptor de una app **que está desplegada y sirviendo**, o
// sea la configuración que de verdad funciona, incluidos los campos que alguien
// arregló a mano después de que la detección fallara. Eso no lo puede hacer un
// comando que corre sobre un directorio.
//
// Tres cosas que este módulo NO hace, y las tres son la misma regla:
//
//   1. **No escribe nada.** Ni en el servidor ni en el repositorio de nadie. Se
//      genera el texto y se copia; lo pega una persona. Es lo mismo que con la
//      orden de instalación en la pantalla de servidores.
//   2. **No inventa lo que el servidor no dice.** Una clave sin valor no se
//      emite, igual que hace `_ik` en el Orbit real.
//   3. **No lleva ni un valor del `.env`.** Puede llevar los **nombres**, porque
//      el bloque `env` de `orbit.json` es una *especificación* —qué variables
//      hacen falta y cómo obtenerlas— y no un almacén. Y aunque se quisiera no
//      se podría: el contrato sólo deja pasar los nombres.

import type { AppInfo } from './contrato'

/** Un registro del `env_spec` del descriptor.
 *
 *  Viaja dentro de una cadena del contrato, **en dos formatos**, según la
 *  versión de Orbit que escribió el descriptor:
 *
 *  · hasta la 1.3.7, TSV de cinco campos:
 *    `CLAVE<TAB>modo<TAB>argumento<TAB>secreto<TAB>descripción`;
 *  · desde la 1.3.8, siete campos separados por US (`\x1f`), con
 *    `obligatoria` y `por defecto` al final. El cambio de separador no es
 *    cosmético: con tabuladores, `read` funde dos seguidos y una descripción
 *    vacía corría los campos de detrás —una obligatoria salía como opcional—.
 *
 *  Un descriptor escrito por una versión y leído por otra es lo normal: el
 *  fichero no se reescribe hasta el siguiente despliegue. */
export interface Variable {
  clave: string
  modo: 'generate' | 'prompt' | 'skip'
  argumento: string
  secreta: boolean
  descripcion: string
  /** `"required": true`. Sólo existe en el formato de la 1.3.8: perderlo al
   *  regenerar el fichero convertiría una variable sin la que la app no
   *  arranca en una que se puede saltar. */
  obligatoria: boolean
  /** `"default"`. `null` es «no tiene». Un valor por defecto vacío y «no
   *  tiene» se escriben igual en el descriptor, y Orbit los trata igual. */
  porDefecto: string | null
}

/**
 * Lee el `env_spec` del descriptor.
 *
 * Existe para **no degradar** un `orbit.json` que ya existía. Si a esta app la
 * configuró un descriptor con `{"generate": 32}`, regenerar el fichero poniendo
 * `prompt` en su lugar cambiaría el significado en silencio: donde había «esto
 * se genera solo» quedaría «pregúntaselo a alguien». Perder eso sin decirlo es
 * peor que no ofrecer la orden.
 */
export function leerEspecificacion(spec: string): Variable[] {
  const fuera: Variable[] = []
  for (const linea of spec.split('\n')) {
    if (linea.trim() === '') continue
    if (linea.includes('\x1f')) {
      // El formato de la 1.3.8. Siete campos y ninguno puede llevar un US
      // dentro: el `gsub` del servidor los quita antes de unirlos.
      const [clave, modo, argumento, secreto, desc, req, def] = linea.split('\x1f')
      if (!clave) continue
      fuera.push({
        clave,
        modo: modo === 'generate' ? 'generate' : 'prompt',
        argumento: argumento ?? '',
        secreta: secreto === 'secret',
        descripcion: desc ?? '',
        obligatoria: req === 'required',
        porDefecto: def === undefined || def === '' ? null : def,
      })
      continue
    }
    const [clave, modo, argumento, secreto, ...resto] = linea.split('\t')
    if (!clave) continue
    fuera.push({
      clave,
      modo: modo === 'generate' || modo === 'skip' ? modo : 'prompt',
      argumento: argumento ?? '',
      secreta: secreto === 'secret',
      // La descripción puede llevar tabuladores dentro; se recompone.
      descripcion: resto.join('\t'),
      obligatoria: false,
      porDefecto: null,
    })
  }
  return fuera
}

/**
 * La regla de `_safe_relpath`, copiada letra por letra.
 *
 * Hace falta aquí por una asimetría del servidor que no es evidente: **el
 * descriptor puede contener rutas que el `orbit.json` no admite**. `--appdir` sí
 * se valida al crear la app, pero `--outdir` y `--docroot` no, y la detección
 * escribe lo que encuentra. O sea que una app perfectamente desplegada puede
 * tener un `outdir` que, puesto en un `orbit.json`, el despliegue **descartaría
 * con un aviso**.
 *
 * Emitir esa clave sería lo peor de las tres opciones: el fichero se ve bien, se
 * sube, y la carpeta que publica no es la que pone ahí. Así que no se emite y se
 * dice cuál era.
 */
export function rutaSegura(v: string): boolean {
  if (v === '') return false
  if (v === '.') return true
  if (v.startsWith('/') || v.includes('..')) return false
  return /^[A-Za-z0-9._][A-Za-z0-9._-]*(\/[A-Za-z0-9._][A-Za-z0-9._-]*)*$/.test(v)
}

/** Las tres claves que Orbit lee con `_dpath`, o sea pasándolas por
 *  `_safe_relpath`. Las demás son cadenas y valen tal cual. */
const RUTAS = new Set(['appdir', 'outdir', 'docroot'])

/** Las claves que Orbit lee del `orbit.json`, con la del descriptor de la que
 *  sale cada una.
 *
 *  La lista es exactamente la de `_read_descriptor`. Emitir una clave que Orbit
 *  no lee sería escribir en el repositorio de alguien un ajuste que no hace
 *  nada, y que dentro de seis meses alguien intentará cambiar. */
const CADENAS: [string, string][] = [
  ['type', 'type'],
  ['appdir', 'appdir'],
  ['build', 'build'],
  ['start', 'start'],
  ['outdir', 'outdir'],
  ['docroot', 'docroot'],
]

/** Las dos booleanas. En el descriptor son `"yes"` y en el fichero son `true`,
 *  y sólo se emiten cuando valen que sí — igual que `_ib`. */
const BOOLEANAS: [string, string][] = [
  ['spa', 'spa'],
  ['php', 'php'],
]

/**
 * La regla de `_csp_check` de Orbit 1.5.1, copiada regla por regla y en el
 * mismo orden, para que el motivo que se enseña sea el mismo que daría el
 * despliegue.
 *
 * El valor acaba entre comillas dobles dentro de un vhost que se enlaza antes
 * del `nginx -t`, así que Orbit **rechaza** en vez de arreglar: un despliegue
 * con una política mala se aborta con la versión anterior intacta. Decirlo
 * aquí, antes de que el fichero llegue al repositorio, ahorra ese despliegue.
 *
 * Devuelve `null` si vale, o el motivo. Los blancos de los extremos no cuentan,
 * como en `_csp_load`: «  » es no haber puesto nada, y eso vale (no se emite).
 */
export function problemaDeLaCsp(valor: string): string | null {
  // `_csp_load` recorta los blancos de los extremos antes de comprobar nada.
  const v = valor.replace(/^\s+/, '').replace(/\s+$/, '')
  if (v === '') return null
  // Bash cuenta caracteres con UTF-8 y bytes con `LANG=C`, y no se sabe con
  // cuál corre el servidor: se aplica la cuenta más estricta, la de bytes.
  if (new TextEncoder().encode(v).length > 4096) return 'pasa de 4096 caracteres'
  // `[[:cntrl:]]` con UTF-8 incluye también los de control C1 (U+0080–U+009F).
  if (/[\x00-\x1f\x7f-\x9f]/.test(v)) return 'lleva saltos de línea o caracteres de control'
  if (/["\\${}]/.test(v)) return 'no puede llevar comillas dobles, barras invertidas, «$», «{» ni «}»'
  if (v.endsWith(';')) return 'no puede acabar en «;»'
  return null
}

/** Lo que no sale del descriptor del servidor y lo pone quien genera el
 *  fichero. Hoy, la CSP de Orbit 1.5.1, que vive en el `orbit.json` de la
 *  release y que `info --json` no devuelve: no hay de dónde copiarla. */
export interface Anadidos {
  /** `"csp"`. Vacío o sólo blancos es «no pongas ninguna». */
  csp?: string
  /** `"csp_report_only": true`: la manda como `…-Report-Only`, para probarla
   *  sin bloquear nada. Sólo se emite con una `csp` que se emita. */
  cspSoloInforme?: boolean
}

export interface Generado {
  /** El fichero, listo para copiar. */
  texto: string
  /** Qué claves se han emitido, para poder enseñarlo sin leer el JSON. */
  claves: string[]
  /** Lo que no se ha podido poner, con el motivo. Va a la pantalla: un fichero
   *  incompleto que no dice qué le falta es peor que ninguno. */
  huecos: string[]
}

/**
 * Construye el fichero.
 *
 * `nombresDeEntorno` son los que devuelve `env list --json`, que **sólo da
 * nombres**. `null` quiere decir que no se han pedido, y entonces el bloque
 * `env` no se escribe: escribirlo vacío diría que esta app no necesita
 * variables, que es una afirmación y no un silencio.
 */
export function generar(
  info: AppInfo,
  nombresDeEntorno: string[] | null,
  anadidos: Anadidos = {},
): Generado {
  const c = info.config
  const obj: Record<string, unknown> = {}
  const huecos: string[] = []

  // `hasOwn`: las claves del descriptor las pone el servidor, y una que no
  // viene no puede salir del prototipo de Object.
  const val = (k: string): string => (Object.hasOwn(c, k) ? (c[k] ?? '') : '').trim()

  for (const [clave, campo] of CADENAS) {
    const v = val(campo)
    if (v === '') continue
    // `orbit init` no escribe `appdir` cuando es la raíz, y con razón: la clave
    // existiría sin decir nada.
    if (clave === 'appdir' && (v === '.' || v === './')) continue
    // Una ruta que el lector del fichero no admitiría no se emite: la
    // descartaría él, con un aviso, en mitad de un despliegue.
    if (RUTAS.has(clave) && !rutaSegura(v)) {
      huecos.push(
        `Falta «${clave}». El servidor lo tiene puesto como «${v}», y eso no es una ruta que Orbit acepte dentro de un orbit.json: la descartaría con un aviso en mitad del despliegue. Ponlo a mano si de verdad es lo que quieres.`,
      )
      continue
    }
    obj[clave] = v
  }

  for (const [clave, campo] of BOOLEANAS) {
    if (val(campo) === 'yes') obj[clave] = true
  }

  // Lo que la app escribe en marcha, y que por eso no puede vivir dentro de una
  // release —que se rehace entera en cada despliegue—. En el descriptor va
  // separado por espacios; en el fichero es una lista.
  const shared = val('shared')
  if (shared !== '') obj.shared = shared.split(/\s+/).filter((x) => x !== '')

  // El bloque `env`: nombres y cómo obtenerlos, nunca valores.
  const previas = leerEspecificacion(Object.hasOwn(c, 'env_spec') ? (c.env_spec ?? '') : '')
  if (previas.length > 0 || (nombresDeEntorno !== null && nombresDeEntorno.length > 0)) {
    const porClave = new Map(previas.map((v) => [v.clave, v]))
    // Las que ya tenían especificación mandan, y detrás las que existen en el
    // servidor y no estaban declaradas.
    const claves = [
      ...previas.map((v) => v.clave),
      ...(nombresDeEntorno ?? []).filter((n) => !porClave.has(n)),
    ]

    // Sin prototipo: los nombres son los del `.env` del servidor, y en un `{}`
    // normal `vars['__proto__'] = …` no crea la clave, cambia el prototipo.
    // `__proto__` es un nombre de variable válido para Orbit, y desaparecía del
    // fichero sin decir nada.
    const vars = Object.create(null) as Record<string, unknown>
    for (const k of claves) {
      const v = porClave.get(k)
      const entrada: Record<string, unknown> = {}
      if (v?.modo === 'generate') {
        entrada.generate = numeroSiCabe(v.argumento)
      } else if (v?.modo === 'skip') {
        // «skip» no tiene forma propia en el fichero: es lo que Orbit deduce de
        // una clave que no declara ni `generate` ni `prompt`. Se emite el objeto
        // igualmente para que la variable siga declarada.
      } else {
        entrada.prompt = v && v.argumento !== '' ? v.argumento : `Valor de ${k}`
      }
      if (v?.secreta) entrada.secret = true
      if (v && v.descripcion !== '') entrada.desc = v.descripcion
      // Lo que añadió la 1.3.8, que es justo lo que no se puede aplanar: una
      // obligatoria regenerada como opcional deja de parar el despliegue, y la
      // web arranca sin su clave de base de datos.
      if (v?.obligatoria) entrada.required = true
      if (v && v.porDefecto !== null) entrada.default = v.porDefecto
      vars[k] = entrada
    }

    const env: Record<string, unknown> = {}
    const fichero = val('env_file')
    if (fichero !== '' && fichero !== '.env') env.file = fichero
    env.vars = vars
    obj.env = env

    if (nombresDeEntorno === null) {
      huecos.push(
        'El bloque «env» sale de la especificación que ya tenía esta app. No he pedido la lista de variables del servidor, así que puede faltar alguna.',
      )
    }
  } else if (nombresDeEntorno === null) {
    huecos.push(
      'Falta el bloque «env», que dice qué variables necesita la app para arrancar. No he pedido su lista de nombres.',
    )
  }

  // La CSP (1.5.1). Va en el vhost de cualquier app que tenga uno —no en una
  // redirección, que no sirve nada—, y una mala aborta el despliegue entero,
  // así que una que Orbit rechazaría no se emite: se dice por qué.
  const csp = (anadidos.csp ?? '').replace(/^\s+/, '').replace(/\s+$/, '')
  if (csp !== '') {
    const motivo = problemaDeLaCsp(csp)
    if (val('type') === 'redirect') {
      huecos.push('Una redirección no sirve páginas: Orbit no le pone «csp» a su vhost, así que no la escribo.')
    } else if (motivo !== null) {
      huecos.push(
        `Falta «csp»: ${motivo}. Orbit la rechazaría y abortaría el despliegue con la versión anterior en producción.`,
      )
    } else {
      obj.csp = csp
      if (anadidos.cspSoloInforme) obj.csp_report_only = true
    }
  }

  // Lo que no se puede saber desde aquí, dicho una vez y con nombre.
  if (val('type') === '') {
    huecos.push(
      'El descriptor de esta app no dice el tipo, y **sin «type» Orbit ignora el fichero entero**. Escríbelo a mano antes de subirlo.',
    )
  }

  return {
    texto: JSON.stringify(obj, null, 2) + '\n',
    claves: Object.keys(obj),
    huecos,
  }
}

/** `generate` admite un número —la longitud del secreto— o una cadena. Se
 *  conserva como llegó: convertir «32» en `"32"` cambiaría lo que Orbit hace. */
function numeroSiCabe(s: string): number | string {
  return /^[0-9]+$/.test(s) ? Number(s) : s
}
