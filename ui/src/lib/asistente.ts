// El asistente de web nueva, y sus siete finales.
//
// La decisión que sostiene todo lo demás, y que sale de dos hechos verificados
// del servidor:
//
//   1. `orbit new` **no tiene `--json`**. Ni el comando, ni su resultado, ni sus
//      errores: lo único que devuelve es prosa y un código de salida.
//   2. Su resumen final distingue tres casos en castellano, excelente para una
//      persona e ilegible para un cliente.
//
// De ahí:
//
// > **La interfaz no interpreta la salida de `orbit new`. Le vuelve a preguntar
// > al servidor.**
//
// Al terminar —salga con 0 o con 1— se ejecuta `orbit info <app> --json`, que sí
// tiene contrato, y el estado real se lee de ahí. Son 86 ms medidos sobre un
// comando que ha tardado tres minutos, y es la única forma que **no depende del
// idioma del servidor**: analizar la prosa sería atarse a unas frases que pueden
// cambiar en cualquier versión.

import type { AppInfo } from './contrato'

export type Final = 'F1' | 'F2' | 'F3' | 'F4' | 'F5' | 'F6' | 'F7'

export interface Desenlace {
  final: Final
  /** El vocabulario de estados de siempre. `bien` no es «éxito»: es «no hay
   *  nada que hacer». */
  tono: 'bien' | 'atencion' | 'error'
  titulo: string
  /** Qué existe. Va **primero** en los parciales, porque es lo que quien mira no
   *  se espera. */
  existe: string
  /** Qué falta. */
  falta: string
  /** Qué se puede deshacer, y con qué. */
  deshacer: string
  /** La acción principal, con la orden que ejecuta. */
  accion: { texto: string; orden: string } | null
}

/**
 * Clasifica el resultado preguntándole al servidor, no leyendo su prosa.
 *
 * `info` es `null` cuando `orbit info` no encuentra la app: eso es F6, y es el
 * único caso en que **no hay nada creado y nada que deshacer**.
 *
 * `dnsResuelve` es lo único que no sale del contrato: se comprueba aparte,
 * porque una web perfectamente publicada cuyo dominio no apunta al servidor se
 * ve exactamente igual desde dentro.
 */
export function clasificar(
  app: string,
  info: AppInfo | null,
  dnsResuelve: boolean | null = null,
): Desenlace {
  if (info === null) {
    return {
      final: 'F6',
      tono: 'error',
      titulo: 'No se ha podido crear',
      existe: 'Nada. El servidor no tiene ninguna app con ese nombre.',
      falta: 'Todo. Lo más probable es que no haya podido clonar el repositorio: revisa la URL, la rama y si «orbit github» está conectado.',
      deshacer: 'Nada que deshacer.',
      accion: null,
    }
  }

  const releases = info.releases.length
  const s = info.state

  // El orden importa y es el mismo que el de la tabla de estados: lo que impide
  // que la web se sirva gana sobre lo que sólo la degrada.
  if (!s.served) {
    return {
      final: 'F5',
      tono: 'error',
      titulo: 'Creada, pero nginx no la sirve',
      existe: `«${app}» está registrada, con su configuración y ${releases === 1 ? 'una release' : `${releases} releases`}.`,
      falta: 'El vhost de nginx. Sin él la conexión se cierra: ni 404 ni 502, el visitante no recibe nada.',
      deshacer: 'Se regenera del descriptor, sin decidir nada por ti.',
      accion: { texto: 'Ir al diagnóstico', orden: `orbit doctor --fix` },
    }
  }

  if (releases === 0) {
    // La app existe con vhost, unidad y clon; lo que no tiene es una release que
    // servir. Un visitante recibe un 502.
    return {
      final: 'F3',
      tono: 'error',
      titulo: 'Creada, pero no ha llegado a compilar',
      existe: `«${app}» está registrada en el servidor, con su dominio y su configuración.`,
      falta: 'Una versión publicada. Como no hay ninguna, el dominio devuelve 502.',
      deshacer: `Se retira con «orbit remove ${app} -y», sin --purge: no ha llegado a servir nada, así que no hay datos que perder.`,
      accion: { texto: 'Ver el log del build', orden: `orbit logs ${app}` },
    }
  }

  if (s.service !== null && s.service !== 'active') {
    return {
      final: 'F4',
      tono: 'error',
      titulo: 'Publicada, pero el proceso no se mantiene en pie',
      existe: `La release está publicada y nginx tiene su vhost.`,
      falta: 'El proceso arranca y se cae. El log dirá por qué; suele ser una variable de entorno que falta o un puerto ocupado.',
      deshacer: `Se puede reintentar el despliegue, o retirarla con «orbit remove ${app} -y».`,
      accion: { texto: 'Ver el log', orden: `orbit logs ${app}` },
    }
  }

  if (dnsResuelve === false) {
    return {
      final: 'F7',
      tono: 'atencion',
      titulo: 'Publicada, pero el dominio no apunta aquí',
      existe: `Todo lo del servidor: «${app}» está publicada y sirviendo.`,
      falta: `El DNS de ${info.state.last_deploy ? '' : ''}tu dominio. Desde fuera nadie llega todavía.`,
      deshacer: 'Nada nuestro: esto se arregla en tu proveedor de DNS, no aquí.',
      accion: null,
    }
  }

  if (!s.ssl) {
    // El final que el propio Orbit documenta como inevitable, y la razón por la
    // que `orbit new` avisa y sigue en vez de morir a mitad: dejar la app creada
    // y el comando en error es la peor combinación. La interfaz honra esa
    // decisión y **no lo pinta como un fallo**.
    return {
      final: 'F2',
      tono: 'atencion',
      titulo: 'Publicada, falta el certificado',
      existe: 'La web ya está publicada y sirviendo por HTTP.',
      falta: "El certificado. Let's Encrypt pide un correo de contacto y no había ninguno configurado en el servidor.",
      deshacer: 'Nada que deshacer: se emite y ya está.',
      accion: { texto: 'Emitir el certificado', orden: `orbit ssl ${app}` },
    }
  }

  return {
    final: 'F1',
    tono: 'bien',
    titulo: 'Publicada',
    existe: 'Todo: la release, el vhost y el certificado.',
    falta: '',
    deshacer: `Se retira con «orbit remove ${app} -y» si te arrepientes.`,
    accion: null,
  }
}

/** La regla de forma del servidor, para validar **mientras se escribe** y no al
 *  enviar: enterarse de que el nombre no vale después de tres pantallas es
 *  gratuito y molesto. */
export function nombreValido(s: string): string | null {
  if (s.length === 0) return null
  if (s.length > 40) return 'Como mucho 40 caracteres.'
  if (s.includes('..')) return 'No puede llevar dos puntos seguidos.'
  if (!/^[a-z0-9]/.test(s)) return 'Tiene que empezar por una letra minúscula o un número.'
  if (!/^[a-z0-9._-]*$/.test(s)) return 'Sólo minúsculas, números, punto, guion y guion bajo.'
  return null
}

/** Qué hacer con un campo que Orbit sabe detectar solo.
 *
 *  Son tres estados y no dos, y la diferencia no es cosmética: **`--build ''`
 *  significa «esta app no se compila», que no es lo mismo que no decir nada.**
 *  Un campo de texto vacío no puede querer decir las dos cosas, así que en la
 *  pantalla son dos estados visibles: *detectar* (gris) y *sin build*. */
export type Anulacion = { modo: 'detectar' } | { modo: 'vacia' } | { modo: 'valor'; valor: string }

export const DETECTAR: Anulacion = { modo: 'detectar' }

/** Lo que se le adelanta a la detección del servidor.
 *
 *  **Está vacío en el caso normal, y esa es la forma correcta de usarlo.** */
export interface Ajustes {
  carpeta: string
  tipo: string
  build: Anulacion
  arranque: Anulacion
  outdir: Anulacion
}

export function ajustesVacios(): Ajustes {
  return { carpeta: '', tipo: '', build: DETECTAR, arranque: DETECTAR, outdir: DETECTAR }
}

export function sinAjustes(a: Ajustes): boolean {
  return (
    a.carpeta.trim() === '' &&
    a.tipo.trim() === '' &&
    a.build.modo === 'detectar' &&
    a.arranque.modo === 'detectar' &&
    a.outdir.modo === 'detectar'
  )
}

/** El estado del formulario. */
export interface Borrador {
  repo: string
  rama: string
  nombre: string
  dominio: string
  alias: string
  correo: string
  baseDeDatos: boolean
  https: boolean
  ajustes: Ajustes
}

export function borradorNuevo(): Borrador {
  return {
    repo: '',
    // La rama por defecto se enseña rellenada en vez de vacía: un campo vacío
    // con un valor por defecto invisible es un valor por defecto que nadie
    // puede comprobar antes de que se aplique.
    rama: 'main',
    nombre: '',
    dominio: '',
    alias: '',
    correo: '',
    baseDeDatos: false,
    https: true,
    ajustes: ajustesVacios(),
  }
}

// ── Las reglas de forma, que son las del servidor ────────────────────────────
//
// Se copian del servidor en vez de inventarse, igual que `nombreValido`: quien
// manda sobre qué se acepta es el otro lado, y una regla más laxa aquí sólo
// consigue que el error llegue tres minutos más tarde y desde una máquina
// remota.

/** La regla de nginx, con una prohibición añadida a propósito: sin punto no hay
 *  certificado posible, y una web publicada que nunca podrá tener HTTPS es justo
 *  el final parcial que este asistente existe para evitar. */
export function dominioValido(s: string): boolean {
  if (s.length === 0 || s.length > 253) return false
  if (s.startsWith('.') || s.endsWith('.') || !s.includes('.')) return false
  return s
    .split('.')
    .every(
      (e) =>
        e.length > 0 &&
        e.length <= 63 &&
        !e.startsWith('-') &&
        !e.endsWith('-') &&
        /^[A-Za-z0-9-]+$/.test(e),
    )
}

/** Las prohibiciones de `git check-ref-format` que importan aquí. El guion
 *  inicial va aparte del resto: no es una cuestión de forma sino de quién
 *  interpreta el argumento — `--branch -X` se lo come el analizador de opciones
 *  del otro lado antes de que nadie mire si la rama existe. */
export function ramaValida(s: string): boolean {
  if (s.length === 0 || s.length > 255 || s.startsWith('-')) return false
  if (s.startsWith('/') || s.endsWith('/') || s.includes('//')) return false
  if (s.includes('..') || s.endsWith('.lock') || s.endsWith('.')) return false
  return !/[\x00-\x20~^:?*[\\\x7f]/.test(s)
}

/** O `usuario/repo`, que es lo que entiende `gh`, o una URL https. Lo que no se
 *  acepta es un `git@…`: eso autentica con la clave del servidor, y darlo por
 *  bueno aquí sería prometer algo que no se ha mirado. */
export function repoValido(s: string): boolean {
  if (s.length === 0 || s.length > 512 || s.startsWith('-')) return false
  if (/[\x00-\x20\x7f]/.test(s)) return false
  const url = s.startsWith('https://') ? s.slice(8) : s.startsWith('http://') ? s.slice(7) : null
  if (url !== null) return url.length > 0 && url.includes('/')
  const p = s.split('/')
  return p.length === 2 && p.every((x) => x.length > 0 && /^[A-Za-z0-9._-]+$/.test(x))
}

/** Deliberadamente laxo: quien valida de verdad es Let's Encrypt, y una
 *  expresión estricta aquí sólo consigue rechazar direcciones que existen. */
export function correoValido(s: string): boolean {
  const i = s.indexOf('@')
  return i > 0 && !s.startsWith('-') && dominioValido(s.slice(i + 1))
}

/** El nombre que se propone a partir del repositorio, para no hacer teclear dos
 *  veces lo mismo.
 *
 *  Es una **propuesta**, no una imposición: se rellena mientras nadie lo haya
 *  tocado y deja de tocarse en cuanto alguien escribe. Un campo que se reescribe
 *  solo después de haberlo editado es un campo que pelea contra quien lo usa.
 *
 *  Devuelve cadena vacía si de ahí no sale un nombre válido, porque proponer un
 *  nombre que el servidor va a rechazar es peor que no proponer ninguno. */
export function nombreDesdeRepo(repo: string): string {
  const ultimo = repo.replace(/\.git$/, '').split('/').filter(Boolean).pop() ?? ''
  const limpio = ultimo
    .toLowerCase()
    .replace(/[^a-z0-9._-]/g, '-')
    .replace(/\.\.+/g, '.')
    .replace(/^[^a-z0-9]+/, '')
    .slice(0, 40)
  return nombreValido(limpio) === null && limpio !== '' ? limpio : ''
}

// ── Los cinco pasos ─────────────────────────────────────────────────────────

export const PASOS = ['Origen', 'Detección', 'Dominio', 'Extras', 'Repaso'] as const
export type Paso = (typeof PASOS)[number]

/** Los alias, que se escriben en un campo separados por comas: escribirlos es
 *  raro, y una lista de filas para algo que casi nadie usa cuesta más de lo que
 *  ahorra. */
export function listaDeAlias(b: Borrador): string[] {
  return b.alias
    .split(',')
    .map((a) => a.trim())
    .filter((a) => a.length > 0)
}

/** Qué impide pasar de este paso. Vacío quiere decir que se puede seguir.
 *
 *  Se valida **por paso y mientras se escribe**, no al enviar: enterarse de que
 *  el nombre no vale después de tres pantallas es gratuito y molesto. Y no se
 *  valida hacia delante — decirle a alguien que el dominio está mal cuando aún
 *  no ha llegado a esa pantalla es ruido. */
export function problemasDe(paso: Paso, b: Borrador): string[] {
  const p: string[] = []
  switch (paso) {
    case 'Origen': {
      if (b.repo.trim() === '') p.push('Falta el repositorio.')
      else if (!repoValido(b.repo.trim()))
        p.push(
          'El repositorio se escribe «usuario/repo» o como una URL https. Un «git@…» autentica con la clave del servidor, y desde aquí no se puede comprobar que exista.',
        )
      if (b.rama.trim() === '') p.push('Falta la rama.')
      else if (!ramaValida(b.rama.trim())) p.push('Esa rama no tiene forma de rama de git.')
      if (b.nombre.trim() === '') p.push('Falta el nombre.')
      else {
        const n = nombreValido(b.nombre.trim())
        if (n) p.push(n)
      }
      break
    }
    case 'Detección': {
      // No hay nada obligatorio: este paso está vacío en el caso normal, y ésa
      // es la respuesta correcta a casi todos los repositorios.
      const t = b.ajustes.tipo.trim()
      if (t !== '' && !/^[a-z0-9_-]+$/.test(t))
        p.push('El tipo es una palabra en minúsculas, como «next» o «laravel».')
      if (b.ajustes.carpeta.trim().startsWith('/'))
        p.push('La carpeta es relativa a la raíz del repositorio, así que no empieza por «/».')
      if (b.ajustes.carpeta.includes('..')) p.push('La carpeta no puede salir del repositorio.')
      break
    }
    case 'Dominio': {
      if (b.dominio.trim() === '') p.push('Falta el dominio.')
      else if (!dominioValido(b.dominio.trim()))
        p.push(
          'Ese dominio no vale. Tiene que llevar al menos un punto: sin él no se le puede emitir un certificado.',
        )
      for (const a of listaDeAlias(b)) {
        if (!dominioValido(a)) p.push(`El alias «${a}» no tiene forma de dominio.`)
      }
      break
    }
    case 'Extras': {
      if (b.correo.trim() !== '' && !correoValido(b.correo.trim()))
        p.push('Ese correo no tiene forma de correo.')
      break
    }
    case 'Repaso':
      break
  }
  return p
}

/** El aviso que evita el final F2, y el motivo de que el correo esté en el
 *  formulario y no escondido en la configuración del servidor.
 *
 *  `correoEnElServidor` es `null` cuando no se ha mirado, y eso **no se pinta
 *  como «no hay»**: son dos cosas distintas y decir la segunda sería afirmar
 *  algo que no se sabe. */
export function avisoDelCertificado(
  b: Borrador,
  correoEnElServidor: boolean | null,
): string | null {
  if (!b.https) return null
  if (b.correo.trim() !== '') return null
  if (correoEnElServidor === true) return null
  if (correoEnElServidor === null)
    return 'No he mirado si este servidor tiene un correo de contacto configurado. Si no lo tiene, la web quedará publicada por HTTP y el certificado habrá que emitirlo después.'
  return "Let's Encrypt necesita un correo de contacto y este servidor no tiene ninguno configurado. Sin él la web quedará publicada por HTTP y «orbit new» avisará sin fallar: se puede arreglar después, pero cuesta menos escribirlo ahora."
}

/** Los argumentos exactos, sin el binario.
 *
 *  **Esto se compara contra `tests/contrato/orden-de-new.json`, que es el mismo
 *  fichero contra el que se compara el catálogo de Rust.** Aquí se construye
 *  para enseñarla en el repaso; allí se construye la que se ejecuta. Una
 *  pantalla cuyo único argumento es «mira lo que va a pasar antes de que pase»
 *  no puede enseñar una cosa y ejecutar otra, y sin nada que las ate eso se
 *  separa solo en el primer cambio.
 *
 *  `--yes` **no es «que sí a todo»**: es «acepta lo que está por defecto». No
 *  crea la base de datos y no abre el editor del `.env`, porque esas dos
 *  preguntas tienen «no» por defecto. Si aquí sale un `--db` es porque alguien
 *  ha marcado la casilla, no porque `--yes` lo arrastre. */
export function argvDeNew(b: Borrador): string[] {
  const v: string[] = ['new', '--yes']
  const par = (bandera: string, valor: string) => v.push(bandera, valor)

  par('--repo', b.repo.trim())
  par('--branch', b.rama.trim())
  par('--name', b.nombre.trim())
  par('--domain', b.dominio.trim())
  // Separados por ESPACIOS, que es como los lee el servidor (`for a in
  // $A_ALIASES`), no por comas — con comas llegan como un solo alias con una
  // coma dentro y eso viaja hasta el `-d` de certbot. Se escriben separados por
  // comas porque es lo cómodo de teclear; se mandan como los espera el otro
  // lado.
  //
  // La lista vacía se manda igualmente: para el servidor «sin alias, y lo digo
  // yo» y «no he dicho nada» son dos casos, y con el segundo se inventa un
  // «www.» que nadie ha pedido.
  par('--aliases', listaDeAlias(b).join(' '))

  if (b.correo.trim() !== '') par('--email', b.correo.trim())
  if (b.baseDeDatos) v.push('--db')
  if (!b.https) v.push('--no-ssl')

  const a = b.ajustes
  // La carpeta va antes que el resto porque no es un campo más: cambiarla
  // redirige la detección entera, y los otros se leen contra otro directorio.
  if (a.carpeta.trim() !== '') par('--appdir', a.carpeta.trim())
  if (a.tipo.trim() !== '') par('--type', a.tipo.trim())
  const anular = (x: Anulacion, bandera: string) => {
    if (x.modo === 'vacia') par(bandera, '')
    else if (x.modo === 'valor') par(bandera, x.valor)
  }
  anular(a.build, '--build')
  anular(a.arranque, '--start')
  anular(a.outdir, '--outdir')
  return v
}

/** La misma orden, para leerla.
 *
 *  Sólo para enseñar: lo que viaja son los argumentos de arriba, y el escapado
 *  de verdad lo hace el núcleo, que tiene su propia prueba de propiedad contra
 *  cuatro shells. Aquí se entrecomilla lo justo para que se lea bien y para que
 *  **un argumento vacío se vea**, que es el caso de `--build ''`. */
export function ordenDeNew(b: Borrador, binario = 'orbit'): string {
  return [binario, ...argvDeNew(b)]
    .map((t) => (t === '' || /[^A-Za-z0-9_./:@-]/.test(t) ? `'${t.split("'").join(`'\\''`)}'` : t))
    .join(' ')
}

// ── Lo que Orbit acabó detectando ───────────────────────────────────────────

/** Lo detectado, leído de `info --json` **después** de crear.
 *
 *  Va aquí y no en un paso del formulario por un hecho del servidor: **la
 *  detección no existe hasta que el repositorio está clonado**, y clonar ocurre
 *  dentro de `orbit new`. No hay ninguna orden que mire un repositorio remoto y
 *  diga qué stack es —`detect_stack` recibe un directorio, y `orbit init` hay
 *  que ejecutarlo dentro del proyecto—, así que enseñarlo antes habría sido
 *  enseñar una promesa.
 *
 *  Después sí es un hecho, y sale del contrato: `config` es el descriptor tal
 *  cual, así que esto no interpreta nada. Lee lo que el servidor escribió. */
export interface Detectado {
  campo: string
  valor: string
  /** `true` cuando el descriptor lo dejó vacío. Se pinta con su etiqueta —«sin
   *  build»— y nunca con un guion, que se leería como «no lo sé». */
  vacio: boolean
}

const CAMPOS: [string, string][] = [
  ['type', 'Tipo'],
  ['appdir', 'Carpeta'],
  ['build', 'Build'],
  ['start', 'Arranque'],
  ['outdir', 'Salida'],
  ['port', 'Puerto'],
]

export function loDetectado(config: Record<string, string>): Detectado[] {
  return CAMPOS.filter(([k]) => k in config).map(([k, etiqueta]) => ({
    campo: etiqueta,
    valor: config[k] ?? '',
    vacio: (config[k] ?? '') === '',
  }))
}
