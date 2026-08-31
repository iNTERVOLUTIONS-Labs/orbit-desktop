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

/** Lo que el asistente va a ejecutar, construido para poder enseñarlo antes.
 *
 *  `--yes` **no es «que sí a todo»**: es «acepta lo que está por defecto». No
 *  crea la base de datos y no abre el editor del `.env`, porque esas dos
 *  preguntas tienen «no» por defecto. */
export function ordenDeNew(d: {
  nombre: string
  repo: string
  rama: string
  dominio: string
  tipo?: string
}): string {
  const p = [
    'orbit new --yes',
    `--repo ${d.repo}`,
    `--branch ${d.rama}`,
    `--name ${d.nombre}`,
    `--domain ${d.dominio}`,
  ]
  if (d.tipo) p.push(`--type ${d.tipo}`)
  return p.join(' ')
}
