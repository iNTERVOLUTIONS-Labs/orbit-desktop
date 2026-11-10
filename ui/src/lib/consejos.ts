/**
 * Qué hacer cuando `ssh` no llega.
 *
 * Es la primera pantalla de quien empieza, y era un callejón: «No se llega» y,
 * debajo, el stderr de `ssh` en inglés —«Permission denied (publickey).»—, que
 * dice qué ha pasado a quien ya lo sabe y nada a quien no. Y el consejo que
 * quedaba, «si eso también falla, no es cosa de Orbit Desktop», mandaba a la
 * persona fuera justo cuando más ayuda necesitaba.
 *
 * Aquí se reconocen los fallos de siempre **leyendo el texto de ssh**. Es la
 * única excepción a la regla de decidir por la clase y no por el texto, y se
 * acepta por dos motivos: el texto no es nuestro —es el de OpenSSH, que no se
 * traduce y lleva veinte años diciendo lo mismo— y lo único que se decide con
 * él es un consejo. Nada se bloquea ni se permite por lo que diga: si no casa,
 * sale el consejo general, que también dice qué hacer.
 */

export interface Consejo {
  /** Un identificador estable, para las pruebas y para quien lo pinte. */
  caso: 'sin-ssh' | 'sin-agente' | 'clave-rechazada' | 'host-desconocido' | 'clave-cambiada'
    | 'no-resuelve' | 'no-contesta' | 'rechaza' | 'sin-red' | 'general'
  /** Qué ha pasado, en una frase y en castellano. */
  que: string
  /** El siguiente paso. Uno, concreto. */
  hacer: string
}

/** Windows no trae siempre el cliente de OpenSSH, y es donde más falla el
 *  primer arranque. Se pregunta al navegador porque es lo único que hay aquí:
 *  la interfaz no sabe en qué sistema corre de otra forma. */
export function enWindows(): boolean {
  const n = typeof navigator === 'undefined' ? null : navigator
  const plataforma = (n as { userAgentData?: { platform?: string } } | null)?.userAgentData?.platform ?? n?.platform ?? ''
  return /win/i.test(plataforma) || /windows/i.test(n?.userAgent ?? '')
}

/**
 * El consejo para un fallo de conexión.
 *
 * @param texto lo que dijeron `ssh` o el transporte, tal cual. Puede ser la
 *   frase del transporte —«no he podido lanzar ssh: …»— o el stderr entero.
 * @param alias el servidor, para poder escribir la orden exacta.
 * @param windows si se está en Windows; por defecto se pregunta.
 */
export function consejoSsh(texto: string, alias: string, windows = enWindows()): Consejo {
  const t = texto.toLowerCase()

  // Sin `ssh` en este ordenador. Lo dice el transporte —no llegó a lanzarlo— o
  // el sistema, con su frase de «no encuentro el programa».
  if (/no he podido lanzar ssh|program not found|cannot find the file|no se puede encontrar el archivo|el sistema no puede encontrar|not recognized as|command not found: ssh|ssh: not found/.test(t)) {
    return {
      caso: 'sin-ssh',
      que: 'Este ordenador no tiene el programa ssh, y sin él no se puede hablar con ningún servidor.',
      hacer: windows
        ? 'Instala el Cliente OpenSSH: Configuración → Sistema → Características opcionales → Agregar una característica → «Cliente OpenSSH». Después cierra Orbit Desktop y vuelve a abrirlo.'
        : 'Instala el cliente de OpenSSH (en Debian o Ubuntu, «sudo apt install openssh-client») y vuelve a abrir Orbit Desktop.',
    }
  }

  // La clave del servidor ha cambiado. Va antes que «permission denied»
  // porque ssh, en ese caso, a veces dice las dos cosas.
  if (/remote host identification has changed|host key for .* has changed|offending .* key/.test(t)) {
    return {
      caso: 'clave-cambiada',
      que: 'La clave de este servidor no es la que tu ordenador recuerda.',
      hacer: 'Comprueba la huella por otro canal —la consola de tu proveedor— y, si es legítima, quita la vieja con «ssh-keygen -R <dirección>» desde un terminal.',
    }
  }

  // Sin agente, o un agente que no tiene la clave cargada. Es el caso típico
  // de Windows: el servicio ssh-agent viene parado y deshabilitado.
  if (/could not open a connection to your authentication agent|error connecting to agent|agent refused operation|no such identity|load key .*: no such file|sign_and_send_pubkey: signing failed|communication with agent failed/.test(t)) {
    return {
      caso: 'sin-agente',
      que: 'ssh no encuentra tu clave: el agente no está en marcha o no la tiene cargada.',
      hacer: windows
        ? 'Inicia el servicio ssh-agent (Servicios → «OpenSSH Authentication Agent» → Automático → Iniciar) y añade tu clave con «ssh-add» en un terminal.'
        : 'Arranca el agente y añade tu clave con «ssh-add» en un terminal.',
    }
  }

  // El servidor no acepta ninguna clave. Con BatchMode no hay contraseña que
  // pedir, así que esto también sale cuando la clave existe pero tiene frase
  // de paso y no está en el agente.
  if (/permission denied \(publickey|permission denied, please try again|too many authentication failures|no more authentication methods/.test(t)) {
    return {
      caso: 'clave-rechazada',
      que: 'El servidor ha rechazado tus claves.',
      hacer: `Si tu clave tiene frase de paso, cárgala en el agente con «ssh-add»${windows ? ' (con el servicio ssh-agent iniciado)' : ''}. Si no, añade tu clave pública al ~/.ssh/authorized_keys del servidor —con «ssh-copy-id» o desde la consola de tu proveedor— o indica la clave correcta en «Usar una clave concreta».`,
    }
  }

  if (/host key verification failed/.test(t)) {
    return {
      caso: 'host-desconocido',
      que: 'Tu ordenador todavía no conoce la clave de este servidor, y no la acepto a ciegas.',
      hacer: `Conéctate una vez con «ssh ${alias}» desde un terminal, comprueba la huella que te enseña y acéptala. Después vuelve a comprobarlo aquí.`,
    }
  }

  if (/could not resolve hostname|name or service not known|nodename nor servname|no such host is known|no se puede encontrar el host|temporary failure in name resolution|no address associated/.test(t)) {
    return {
      caso: 'no-resuelve',
      que: 'No encuentro ese nombre: no hay ninguna dirección detrás.',
      hacer: 'Revisa que la dirección esté bien escrita, o usa la IP del servidor.',
    }
  }

  if (/connection refused|conexión rechazada|actively refused/.test(t)) {
    return {
      caso: 'rechaza',
      que: 'El servidor responde, pero no acepta conexiones SSH en ese puerto.',
      hacer: 'Comprueba el puerto (casi siempre el 22) y que el servicio SSH esté en marcha en el servidor.',
    }
  }

  if (/network is unreachable|no route to host|la red no es accesible|unreachable network/.test(t)) {
    return {
      caso: 'sin-red',
      que: 'Desde este ordenador no hay camino hasta ese servidor.',
      hacer: 'Comprueba tu conexión, y la VPN si el servidor sólo es accesible por ella.',
    }
  }

  if (/timed out|time out|tiempo de espera|no ha contestado/.test(t)) {
    return {
      caso: 'no-contesta',
      que: 'El servidor no contesta.',
      hacer: 'Comprueba que está encendido, que la dirección y el puerto son los correctos y que el cortafuegos deja pasar SSH.',
    }
  }

  return {
    caso: 'general',
    que: 'No he podido conectar por SSH.',
    hacer: `Prueba «ssh ${alias}» en un terminal: verás el mensaje entero de ssh, que suele decir qué falta. Si ahí entra y aquí no, el fallo es de Orbit Desktop.`,
  }
}
