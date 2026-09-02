// Las reglas de forma de un servidor propio, del lado de la interfaz.
//
// Son las mismas que `crates/orbit-client/src/registro.rs` y están aquí por lo
// de siempre: validar **mientras se escribe** en vez de al enviar. Enterarse de
// que el nombre choca después de haber rellenado cuatro campos es gratuito y
// molesto.
//
// Aquí manda el `ssh` local y no el servidor de Orbit, así que las reglas no son
// las de un nombre de app: son las de un `Host` de `ssh_config`.

/** Un servidor a medio escribir. */
export interface Borrador {
  alias: string
  host: string
  usuario: string
  puerto: number
  /** La **ruta** de una clave. Vacío quiere decir «la que use ssh por su
   *  cuenta», que es lo normal con un agente cargado. */
  clave: string
}

/** El guion inicial fuera —un argumento que empieza por guion se lo come el
 *  analizador de opciones de `ssh`— y los comodines también: `prod-*` no sería
 *  un alias sino un patrón que casa con otros hosts. */
export function aliasValido(s: string): boolean {
  return s.length > 0 && s.length <= 64 && !s.startsWith('-') && /^[A-Za-z0-9._-]+$/.test(s)
}

/** Una IP —incluida una v6, con sus dos puntos— o un nombre. Laxo con los
 *  nombres, porque quien valida de verdad es el resolutor, y estricto con lo que
 *  rompería el `argv`. */
export function hostValido(s: string): boolean {
  return s.length > 0 && s.length <= 253 && !s.startsWith('-') && /^[A-Za-z0-9.:-]+$/.test(s)
}

/** La regla de POSIX para un usuario de sistema, con el guion inicial fuera. */
export function usuarioValido(s: string): boolean {
  return s.length > 0 && s.length <= 32 && !s.startsWith('-') && /^[A-Za-z0-9._-]+$/.test(s)
}
