// Por dónde va la instalación.
//
// La primera versión de esta pantalla iba a enseñar una ruedecita y un «esto
// tarda un rato», porque di por hecho que el instalador no decía nada útil.
// Dice bastante: **cada una de sus trece secciones se anuncia a sí misma** con
// un `N/13` delante.
//
//     1/13  Actualizando el sistema base
//     7/13  Instalando PostgreSQL
//     13/13  Instalando la herramienta 'orbit'
//
// O sea que aquí sí se puede dar progreso de verdad —«por el 7 de 13»— sin
// inventarse un porcentaje. Lo que **no** se puede es dar tiempo: los pasos
// duran cosas muy distintas y el instalador no promete ninguna, así que no hay
// ninguna estimación honesta que dar y no se da.
//
// Y el número sale del propio texto, no de una lista copiada aquí: si algún día
// el instalador tiene catorce pasos, esto dirá catorce sin tocarlo.

/** Un paso del instalador. */
export interface Hito {
  /** El número tal cual lo dijo él. */
  id: number
  texto: string
  estado: 'pendiente' | 'haciendo' | 'hecho'
}

export interface Marcha {
  pasos: Hito[]
  /** Cuál va ahora. `null` antes del primero y después del último. */
  actual: number | null
  /** Cuántos ha anunciado en total. Sale del texto: `13` de `7/13`. */
  total: number
  /** Cuántos ha terminado — o sea, cuántos han sido superados por otro. */
  hechos: number
}

/** Los títulos, para poder pintar los que aún no han salido.
 *
 *  Es una copia de los del instalador y **puede quedarse vieja**, así que sólo
 *  se usa para lo que no importa: rellenar el nombre de un paso que todavía no
 *  se ha anunciado. En cuanto el servidor anuncia uno, manda su texto. */
const TITULOS: Record<number, string> = {
  1: 'Actualizando el sistema base',
  2: 'Zona horaria, swap y límites del kernel',
  3: 'Creando el usuario de despliegue',
  4: 'Instalando Node.js LTS + pnpm',
  5: 'Instalando y endureciendo nginx',
  6: 'Restaurando IPs reales de Cloudflare',
  7: 'Instalando PostgreSQL',
  8: 'Instalando PHP (FPM) + Composer',
  9: 'Instalando Python + herramientas',
  10: 'Instalando Certbot',
  11: 'Instalando GitHub CLI',
  12: 'Firewall, fail2ban y actualizaciones',
  13: "Instalando la herramienta 'orbit'",
}

const POR_DEFECTO = 13

/** Quita los códigos de color ANSI.
 *
 *  El instalador los emite sólo con un terminal detrás —`[[ -t 1 ]]`— y aquí no
 *  lo hay, así que en la práctica no vienen. Se limpian igualmente: costaría un
 *  fallo raro y silencioso el día que alguien lance esto por una vía que sí
 *  tenga tty, y el paso saldría sin reconocer. */
function sinColor(s: string): string {
  // eslint-disable-next-line no-control-regex
  return s.replace(/\[[0-9;]*m/g, '')
}

export function hitos(salida: string): Marcha {
  let actual: number | null = null
  let total = POR_DEFECTO
  const vistos = new Map<number, string>()

  for (const cruda of salida.split('\n')) {
    const l = sinColor(cruda).trim()
    // `N/13  Título`, con el glifo de sección delante y espacios variables.
    const m = l.match(/(?:^|\s)(\d{1,2})\/(\d{1,2})\s+(.+)$/)
    if (!m) continue
    const n = Number(m[1])
    const t = Number(m[2])
    if (!Number.isFinite(n) || !Number.isFinite(t) || n < 1 || n > t) continue
    total = t
    // Manda el texto del servidor sobre la copia de aquí.
    vistos.set(n, m[3]!.trim())
    actual = n
  }

  const pasos: Hito[] = []
  for (let i = 1; i <= total; i++) {
    pasos.push({
      id: i,
      texto: vistos.get(i) ?? TITULOS[i] ?? `Paso ${i}`,
      // Sólo se da por hecho un paso cuando **otro posterior** se ha
      // anunciado. El que corre no está hecho, y decir que sí sería adelantarse
      // a un `apt` que puede estar a punto de fallar.
      estado: actual === null ? 'pendiente' : i < actual ? 'hecho' : i === actual ? 'haciendo' : 'pendiente',
    })
  }

  return {
    pasos,
    actual,
    total,
    hechos: actual === null ? 0 : actual - 1,
  }
}
