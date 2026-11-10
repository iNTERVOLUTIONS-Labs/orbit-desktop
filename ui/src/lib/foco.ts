/**
 * Devuelve el foco al sitio desde el que se abrió un diálogo.
 *
 * Fuera del ciclo en el que se cierra, y no dentro: mientras el diálogo está
 * abierto, lo de detrás lleva `inert`, y Svelte lo quita en la misma pasada en
 * la que desmonta el diálogo. Un `focus()` sobre un elemento todavía inerte no
 * hace nada y no avisa, así que la primera versión dejaba el foco en `<body>`
 * al cerrar cualquier hoja: un lector de pantalla volvía a empezar la página
 * desde arriba. jsdom no implementa `inert`, y por eso ninguna prueba lo vio.
 *
 * Si el elemento ya no existe —el botón «Desplegar» se cambia por
 * «Desplegando…» al confirmar—, al título de la página. Si el «de antes» era el
 * propio `<body>` —una hoja abierta desde la paleta, que ya se había ido—, eso
 * no es un sitio: también al título.
 *
 * Y sólo si el foco se ha perdido de verdad. Si mientras tanto se ha abierto
 * otro diálogo, o la vista nueva ya se ha enfocado sola —el asistente pone el
 * foco en su paso—, no se le quita.
 */
export function devolverFoco(previo: HTMLElement | null): void {
  setTimeout(() => {
    if (document.querySelector('[role="dialog"]')) return
    if (!perdido()) return
    const util = previo && previo !== document.body && previo !== document.documentElement ? previo : null
    if (util && util.isConnected && !util.closest('[inert]')) {
      util.focus()
      if (document.activeElement === util) return
    }
    document.querySelector<HTMLElement>('main h1')?.focus()
  }, 0)
}

/** Nadie tiene el foco: está en el `<body>` o en ninguna parte. */
export function perdido(): boolean {
  const a = document.activeElement
  return a === null || a === document.body || a === document.documentElement
}

/** Lleva el foco a `destino` después de la pasada en curso, si se ha perdido.
 *  Para lo que desaparece con el foco dentro: un botón que se quita al
 *  pulsarlo, la consola que se cierra al terminar un despliegue. */
export function recogerFoco(destino: () => HTMLElement | null | undefined): void {
  setTimeout(() => {
    if (!perdido()) return
    destino()?.focus()
  }, 0)
}
