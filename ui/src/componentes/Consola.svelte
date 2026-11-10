<script lang="ts">
  // La consola en vivo: la salida cruda de una orden larga, tal cual llega.
  //
  // Las líneas NO pasan por la plantilla. Llegan a cientos por segundo y cada
  // vuelta de la reactividad por línea —copiar el texto, volver a pintarlo,
  // forzar un reflujo para bajar al final— era lo que hacía que diez mil líneas
  // tardaran diez minutos. Aquí se apunta qué ha llegado y, una vez por
  // fotograma, se añaden las filas nuevas, se quitan las que sobran por arriba
  // y se baja al final. Cada fila es un elemento con `textContent`: nada del
  // servidor se inserta como HTML.
  import { untrack } from 'svelte'
  import type { Salida } from '../lib/consola'

  let {
    salida,
    vivo = true,
    etiqueta = 'Salida del servidor',
    vacio = 'esperando al servidor…',
    tramo = 2000,
  }: {
    /** De dónde salen las líneas. Cambiarla por otra empieza de cero. */
    salida: Salida
    /** Mientras la orden corre: se sigue el final y la caja es más baja. */
    vivo?: boolean
    etiqueta?: string
    /** Lo que se enseña mientras no ha llegado nada. */
    vacio?: string
    /** Cuántas filas hay en la página a la vez. Más arriba quedan guardadas y
     *  se piden con «ver más», como en el visor de log. */
    tramo?: number
  } = $props()

  let caja = $state<HTMLPreElement | null>(null)
  // Sigue el final mientras nadie suba a leer: si alguien sube, se queda donde
  // está, que es lo que hace cualquier terminal.
  let seguir = $state(true)
  // Lo único reactivo, y se escribe una vez por fotograma como mucho.
  let arriba = $state(0)
  let perdidas = $state(0)
  let cuantas = 2000

  // Lo pintado, fuera de la reactividad: es contabilidad, no algo que se vea.
  let pintadas = 0
  let hasta = 0
  let vezPintada = -1
  let marco = 0
  let conMarcador = false

  function fila(t: string): HTMLSpanElement {
    const e = document.createElement('span')
    e.className = 'l'
    e.textContent = t
    return e
  }

  function marcador(c: HTMLPreElement) {
    c.replaceChildren()
    const e = document.createElement('span')
    e.className = 'l vacia'
    e.textContent = vacio
    c.append(e)
    conMarcador = true
    pintadas = 0
  }

  function rehacer(c: HTMLPreElement) {
    const lineas = salida.ultimas(cuantas)
    if (lineas.length === 0) return marcador(c)
    const f = document.createDocumentFragment()
    for (const l of lineas) f.append(fila(l))
    c.replaceChildren(f)
    conMarcador = false
    pintadas = lineas.length
  }

  function pintar() {
    marco = 0
    const c = caja
    if (!c) return
    const nuevas = salida.total - hasta
    if (salida.vez !== vezPintada || nuevas > cuantas || nuevas > salida.guardadas) {
      rehacer(c)
    } else if (nuevas > 0) {
      const f = document.createDocumentFragment()
      for (const l of salida.ultimas(nuevas)) f.append(fila(l))
      if (conMarcador) {
        c.replaceChildren()
        conMarcador = false
      }
      c.append(f)
      pintadas += nuevas
      while (pintadas > cuantas && c.firstElementChild) {
        c.firstElementChild.remove()
        pintadas -= 1
      }
    }
    hasta = salida.total
    vezPintada = salida.vez
    const a = Math.max(0, salida.guardadas - pintadas)
    if (a !== arriba) arriba = a
    if (salida.perdidas !== perdidas) perdidas = salida.perdidas
    // Un solo reflujo por fotograma, no uno por línea.
    if (vivo && seguir) c.scrollTop = c.scrollHeight
  }

  function programar() {
    if (marco) return
    marco = typeof requestAnimationFrame === 'function'
      ? requestAnimationFrame(pintar)
      : (setTimeout(pintar, 16) as unknown as number)
  }

  function anularMarco() {
    if (!marco) return
    if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(marco)
    clearTimeout(marco)
    marco = 0
  }

  $effect(() => {
    const s = salida
    if (!caja) return
    // Otra salida: se empieza de cero, y lo que ya tenga se pinta ahora y no
    // en el fotograma siguiente.
    const t = tramo
    // Sin seguir lo que lee `pintar` (`seguir`, `arriba`…): este efecto es la
    // suscripción, y rehacerla en cada desplazamiento repintaría todo.
    untrack(() => {
      vezPintada = -1
      hasta = 0
      cuantas = t
      pintar()
    })
    const baja = s.alCambiar(programar)
    return () => {
      baja()
      anularMarco()
    }
  })

  function alDesplazar() {
    if (!caja) return
    seguir = caja.scrollHeight - caja.scrollTop - caja.clientHeight < 24
  }

  function irAlFinal() {
    // El botón desaparece al pulsarlo: el foco va a la salida, no al <body>.
    // En el acto y no después: la salida no está inerte y no hay que esperar.
    caja?.focus()
    seguir = true
    // Al volver al final sobran las filas que se pidieron con «ver más».
    cuantas = tramo
    vezPintada = -1
    anularMarco()
    pintar()
  }

  /** Las de más arriba, por delante y sin mover lo que se está leyendo. */
  function verMas() {
    const c = caja
    if (!c) return
    const g = salida.guardadas
    const desde = Math.max(0, g - pintadas - tramo)
    const viejas = salida.entre(desde, g - pintadas)
    if (viejas.length === 0) return
    const alto = c.scrollHeight
    const f = document.createDocumentFragment()
    for (const l of viejas) f.append(fila(l))
    c.prepend(f)
    pintadas += viejas.length
    cuantas = Math.max(cuantas, pintadas)
    c.scrollTop += c.scrollHeight - alto
    arriba = Math.max(0, g - pintadas)
    // Con la última tanda el botón se va: el foco, a la salida.
    if (arriba === 0) caja?.focus()
  }
</script>

<div class="consola">
  {#if arriba > 0 || perdidas > 0}
    <p class="arriba">
      {#if arriba > 0}
        <button type="button" class="mas" onclick={verMas}>
          {arriba} más arriba · ver {Math.min(tramo, arriba)} más
        </button>
      {/if}
      {#if perdidas > 0}
        <!-- Se dice: una salida a la que le falta el principio no puede
             parecer completa. -->
        <span class="perdidas">
          {perdidas === 1 ? 'La primera línea ya no se guarda' : `Las primeras ${perdidas} líneas ya no se guardan`}.
        </span>
      {/if}
    </p>
  {/if}
  <div class="envoltorio">
    <!-- `role="log"` con `aria-live="off"`: cientos de líneas por segundo
         leídas en voz alta inutilizan la pantalla. Lo que se anuncia son las
         transiciones, y de eso se encarga quien usa la consola. Con foco para
         poder desplazarla con el teclado. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre
      class="crudo"
      class:crudo--vivo={vivo}
      role="log"
      aria-live="off"
      aria-label={etiqueta}
      tabindex="0"
      bind:this={caja}
      onscroll={alDesplazar}
    ></pre>
    {#if vivo && !seguir}
      <button type="button" class="seguir" aria-label="Ir al final y seguir la salida" onclick={irAlFinal}>
        <span aria-hidden="true">↓</span> seguir
      </button>
    {/if}
  </div>
</div>

<style>
  .consola { margin-top: var(--e-4); }
  .envoltorio { position: relative; }
  .crudo {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; white-space: pre-wrap; word-break: break-word;
    color: var(--fg-muted);
    max-height: 300px; overflow: auto;
  }
  .crudo--vivo { max-height: 260px; min-height: 120px; }
  .crudo:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  /* Una fila por línea: así quitar las de arriba es quitar elementos, y no
     reescribir un texto de megas. La vacía conserva su alto. */
  .crudo :global(.l) { display: block; min-height: 1lh; }
  .crudo :global(.vacia) { color: var(--fg-faint); }
  .arriba { margin: 0 0 var(--e-2); display: flex; gap: var(--e-3); flex-wrap: wrap; font-size: 12px; }
  .mas {
    background: none; border: 0; padding: 0;
    font: inherit; color: var(--accent-text); cursor: pointer;
  }
  .mas:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .perdidas { color: var(--fg-faint); }
  .seguir {
    position: absolute; right: var(--e-3); bottom: var(--e-3);
    background: var(--surface); border: 1px solid var(--border-strong); border-radius: 999px;
    padding: 2px var(--e-3); font: inherit; font-size: 11px; color: var(--fg); cursor: pointer;
    box-shadow: var(--shadow-1);
  }
  .seguir:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
