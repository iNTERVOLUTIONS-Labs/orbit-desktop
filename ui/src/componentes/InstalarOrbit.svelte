<script lang="ts">
  import type { Instalacion, Requisitos } from '../lib/puente'
  import { hitos } from '../lib/instalacion'

  let {
    alias,
    requisitos = null,
    mirando = false,
    instalando = false,
    salida = '',
    resultado = null,
    alInstalar,
    alCancelar,
    alCerrar,
  }: {
    alias: string
    /** `null` mientras no se ha mirado. No es «no se puede»: son dos cosas
     *  distintas y la segunda es una afirmación. */
    requisitos?: Requisitos | null
    mirando?: boolean
    instalando?: boolean
    salida?: string
    resultado?: Instalacion | null
    alInstalar: () => void
    alCancelar: () => void
    alCerrar: () => void
  } = $props()

  const lineas = $derived(salida.split('\n').filter((l) => l.trim() !== ''))
  const marcha = $derived(hitos(salida))

  let visor: HTMLPreElement | undefined = $state()
  // Sigue el final mientras llegan líneas. Sin esto hay que perseguir la barra
  // durante diez minutos; con esto y sin comprobar nada, no se puede subir a
  // leer lo que pasó hace un rato. Así que sólo se sigue si ya estabas abajo.
  $effect(() => {
    void lineas.length
    if (!visor) return
    const alFinal = visor.scrollHeight - visor.scrollTop - visor.clientHeight < 40
    if (alFinal) visor.scrollTop = visor.scrollHeight
  })
</script>

<section class="instalar">
  <header>
    <h3>Instalar Orbit en {alias}</h3>
    {#if !instalando}
      <button type="button" class="cerrar" onclick={alCerrar}>Cerrar</button>
    {/if}
  </header>

  {#if resultado}
    <!-- Terminado. El veredicto NO sale del código de salida ni de la prosa:
         sale de haberle preguntado a `orbit version --json` después. Un
         instalador que termina en 1 puede haber dejado Orbit funcionando, y uno
         que termina en 0 no lo demuestra. -->
    <div class="final" class:final--bien={resultado.version} class:final--mal={!resultado.version}>
      <div class="marca" aria-hidden="true">{resultado.version ? '✓' : '✕'}</div>
      <div>
        {#if resultado.version}
          <p class="titulo">Orbit {resultado.version} está instalado</p>
          <p class="que">
            Lo dice él: al terminar le he preguntado con <code>orbit version</code>.
            Ya puedes abrir el servidor y crear tu primera web.
          </p>
        {:else}
          <p class="titulo">No ha quedado instalado</p>
          <p class="que">
            Le he preguntado al terminar y no contesta. Abajo está lo que dijo el
            instalador; el final suele decir por dónde se rompió.
          </p>
        {/if}
      </div>
    </div>
  {:else if instalando}
    <p class="que">
      Instalando nginx, Node, PostgreSQL, PHP y Certbot. <strong>Tarda entre
      cinco y diez minutos</strong> y no se puede acortar.
    </p>

    <!-- Los hitos. No es una barra: el instalador no dice cuánto le queda, así
         que un porcentaje sería un número inventado. Lo que sí se puede decir es
         por dónde va, y eso se saca de sus propios títulos. -->
    <ol class="hitos">
      {#each marcha.pasos as p (p.id)}
        <li class="hito" class:hito--hecho={p.estado === 'hecho'} class:hito--ahora={p.estado === 'haciendo'}>
          <span class="punto" aria-hidden="true"></span>
          <span class="nombre">{p.texto}</span>
        </li>
      {/each}
    </ol>

    <pre class="visor" bind:this={visor} aria-label="Salida del instalador">{lineas.join('\n') ||
        'esperando al servidor…'}</pre>

    <div class="parar">
      <button type="button" class="secundario" onclick={alCancelar}>Parar</button>
      <p class="pista">
        Para el proceso; <strong>no deshace lo ya instalado</strong>. Si lo paras
        a mitad, el servidor se queda con parte de los paquetes puestos y hace
        falta volver a lanzarlo.
      </p>
    </div>
  {:else if mirando}
    <p class="tenue">mirando qué hay en {alias}…</p>
  {:else if requisitos}
    <ul class="chequeo">
      <li class:si={requisitos.git} class:no={!requisitos.git}>
        <span aria-hidden="true">{requisitos.git ? '✓' : '✕'}</span> git
      </li>
      <li class:si={requisitos.root || requisitos.sudo_sin_contrasena}
          class:no={!(requisitos.root || requisitos.sudo_sin_contrasena)}>
        <span aria-hidden="true">{requisitos.root || requisitos.sudo_sin_contrasena ? '✓' : '✕'}</span>
        {requisitos.root ? 'entras como root' : 'sudo sin contraseña'}
      </li>
      {#if requisitos.sistema}
        <li class="si"><span aria-hidden="true">·</span> {requisitos.sistema}</li>
      {/if}
    </ul>

    {#each requisitos.impedimentos as i (i.clase)}
      <div class="impedimento">
        <p class="titulo">{i.que}</p>
        {#if i.arreglo}
          <p class="que">En el servidor:</p>
          <pre class="orden">{i.arreglo}</pre>
        {/if}
      </div>
    {/each}

    {#each requisitos.avisos as a (a)}
      <p class="aviso">{a}</p>
    {/each}

    <!-- La secuencia literal, siempre y también cuando no se puede instalar
         desde aquí: quien no pueda la copia y la ejecuta él.

         Y es la de verdad. La versión anterior de esta pantalla mandaba copiar
         un `curl … | sudo bash` que NO funciona: install.sh lee el fichero
         `orbit` que tiene al lado y por una tubería no hay ninguno. -->
    <p class="et">Lo que voy a ejecutar</p>
    <pre class="orden secuencia">{requisitos.pasos.join('\n')}</pre>

    <div class="acciones">
      <button type="button" class="primario" disabled={!requisitos.puede} onclick={alInstalar}>
        {requisitos.ya_instalado ? 'Reinstalar' : 'Instalar Orbit'}
      </button>
      {#if !requisitos.puede}
        <p class="pista">
          Desde aquí no se puede todavía. Arregla lo de arriba y vuelve a
          mirar, o ejecuta esos pasos a mano en el servidor.
        </p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .instalar { display: grid; gap: var(--e-4); max-width: 76ch; }
  header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }
  .cerrar {
    background: none; border: 0; padding: 0; font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer; text-decoration: underline;
  }
  .que { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .pista { margin: 0; font-size: 12px; color: var(--fg-faint); max-width: 68ch; }
  .tenue { margin: 0; font-size: 13px; color: var(--fg-faint); }
  .et { margin: 0; font-size: 12px; color: var(--fg-muted); }

  .chequeo { list-style: none; margin: 0; padding: 0; display: flex; gap: var(--e-4); flex-wrap: wrap; }
  .chequeo li { font-size: 13px; display: inline-flex; align-items: center; gap: var(--e-1); }
  .chequeo .si { color: var(--chequeo-si, var(--fg-muted)); }
  .chequeo .no { color: var(--chequeo-no, var(--fg-muted)); }

  .impedimento { display: grid; gap: var(--e-2); }
  .impedimento .titulo { margin: 0; font-size: 13px; color: var(--chequeo-no, var(--fg)); font-weight: 600; }
  .aviso { margin: 0; font-size: 12px; color: var(--aviso-instalar, var(--fg-muted)); max-width: 68ch; }

  .orden {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    white-space: pre-wrap; word-break: normal; overflow-wrap: anywhere;
    user-select: all;
  }

  .acciones { display: grid; gap: var(--e-2); justify-items: start; }
  button {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-4);
    font: inherit; font-size: 14px; font-weight: 600; cursor: pointer;
    transition: transform var(--t-rapido) var(--e-suave), filter var(--t-rapido) var(--e-suave);
  }
  .primario { background: var(--accent-fill); color: var(--on-accent); border: 0; }
  .secundario { background: none; border: 1px solid var(--border-strong); color: var(--fg-muted); }
  button:hover:not(:disabled) { filter: brightness(1.08); }
  button:active:not(:disabled) { transform: translateY(1px); }
  button:disabled { opacity: .45; cursor: default; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }

  /* Los hitos. La línea que los une se rellena según avanzan, que es la única
     señal de progreso honesta que hay aquí: el instalador no dice cuánto le
     queda. */
  .hitos { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--e-3); }
  .hito {
    display: grid; grid-template-columns: 14px 1fr; gap: var(--e-3);
    align-items: center; font-size: 13px; color: var(--fg-faint);
    transition: color var(--t-normal) var(--e-suave);
  }
  .punto {
    width: 10px; height: 10px; border-radius: 50%;
    border: 2px solid var(--border-strong); justify-self: center;
    transition: background var(--t-normal) var(--e-entrada),
                border-color var(--t-normal) var(--e-entrada),
                transform var(--t-normal) var(--e-entrada);
  }
  .hito--hecho { color: var(--fg-muted); }
  .hito--hecho .punto { background: var(--hito-hecho, var(--fg-muted)); border-color: var(--hito-hecho, var(--fg-muted)); }
  .hito--ahora { color: var(--fg); font-weight: 600; }
  .hito--ahora .punto {
    border-color: var(--hito-ahora, var(--accent));
    transform: scale(1.25);
    /* Late mientras dura. Es lo que distingue «va por aquí» de «se ha quedado
       aquí», y en un proceso de diez minutos esa diferencia es la pregunta
       entera de quien mira. */
    animation: late 1.6s var(--e-suave) infinite;
  }
  @keyframes late {
    0%, 100% { box-shadow: 0 0 0 0 var(--velo); }
    50% { box-shadow: 0 0 0 6px transparent; }
  }

  .visor {
    margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 11.5px; line-height: 1.55; color: var(--fg-muted);
    white-space: pre-wrap; word-break: break-word;
    max-height: 18rem; overflow-y: auto;
  }

  .parar { display: grid; gap: var(--e-2); }
  .parar button { justify-self: start; font-size: 13px; }

  .final {
    display: grid; grid-template-columns: auto 1fr; gap: var(--e-3);
    align-items: start; padding: var(--e-4);
    border: 1px solid var(--final-borde, var(--border)); border-radius: var(--r-3);
    animation: aterriza var(--t-lento) var(--e-entrada);
  }
  .final .titulo { margin: 0 0 var(--e-1); font-size: 15px; font-weight: 600; color: var(--final-borde, var(--fg)); }
  .marca {
    width: 28px; height: 28px; border-radius: 50%;
    display: grid; place-items: center;
    color: var(--on-solid); background: var(--final-borde, var(--fg-muted));
    font-size: 15px; font-weight: 700;
  }
  @keyframes aterriza {
    from { opacity: 0; transform: translateY(8px) scale(.99); }
    to { opacity: 1; transform: none; }
  }

  code { font-family: var(--mono); font-size: 12px; }
</style>
