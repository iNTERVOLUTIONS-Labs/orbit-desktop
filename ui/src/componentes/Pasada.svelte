<script lang="ts">
  import LoteVista from './Lote.svelte'
  import { finalesPosibles, leerPasada, FINALES_DEL_LOTE } from '../lib/despliegue'
  import type { App, Lote } from '../lib/contrato'

  let {
    servidor,
    apps,
    crudo = '',
    resultado = null,
    corriendo = false,
    modo = null,
    alLanzar,
    alCancelar,
    alCerrar,
  }: {
    servidor: string
    /** La portada, para poder decir a cuántas va a tocar **antes** de tocarlas. */
    apps: App[]
    /** Las líneas de progreso tal como llegaron. */
    crudo?: string
    resultado?: Lote | null
    corriendo?: boolean
    /** Con cuál de las dos se lanzó. Hace falta después, para saber qué finales
     *  eran posibles. */
    modo?: 'si-cambia' | 'todo' | null
    alLanzar: (soloSiCambia: boolean) => void
    alCancelar: () => void
    alCerrar: () => void
  } = $props()

  // Una redirección no se despliega: `deployable_apps` la salta por su tipo, y
  // eso sí lo dice `list --json`.
  const redirecciones = $derived(apps.filter((a) => a.type === 'redirect').length)
  const candidatas = $derived(apps.length - redirecciones)

  const pasada = $derived(leerPasada(crudo))
  const posibles = $derived(finalesPosibles(modo !== 'todo'))
  // Los que no podían salir **por no haber preguntado al remoto**. No se ocultan
  // del recuento —un cero es una respuesta— pero sí se dice por qué son cero,
  // que es distinto de que no haya pasado nada.
  //
  // «Saltadas» queda fuera de esta lista aunque tampoco pueda salir: su motivo
  // es otro —sólo lo produce el autodespliegue— y darle el motivo equivocado
  // sería peor que no darle ninguno.
  //
  // Y se comprueba contra el recuento en vez de afirmarlo: la frase decía «son
  // cero» sin mirarlos, así que si el servidor devolviera uno de estos con
  // contenido, la pantalla estaría contradiciendo a su propia tabla. Aquí quien
  // manda es el dato; esta lista es una predicción, y una predicción no pisa una
  // medida. Si sale uno que «no podía salir», el recuento se queda solo — que es
  // exactamente lo que hay que mirar ese día.
  const porNoPreguntar = $derived(
    FINALES_DEL_LOTE.filter(
      (f) =>
        f.id !== 'skipped' &&
        !posibles.includes(f.id) &&
        ((resultado as unknown as Record<string, number> | null)?.[f.id] ?? 0) === 0,
    ).map((f) => f.texto),
  )

  function dur(s: number): string {
    return s < 60 ? `${s} s` : `${Math.floor(s / 60)} min ${s % 60} s`
  }
</script>

<section class="pasada">
  <header>
    <h3>Desplegar todo en {servidor}</h3>
    {#if !corriendo}
      <button type="button" class="cerrar" onclick={alCerrar}>Cerrar</button>
    {/if}
  </header>

  {#if resultado}
    <LoteVista lote={resultado} {servidor} />

    {#if porNoPreguntar.length > 0}
      <!--
        Un cero que no podía ser otra cosa NO es la misma información que un cero
        que podía serlo. Los cuatro finales baratos salen de preguntarle al
        remoto de cada app, y sin `--if-changed` no se le pregunta a nadie: sus
        recuentos son cero por construcción. Dejarlos ahí sin decirlo invitaría a
        leerlos como «he mirado y no había nada».
      -->
      <p class="matiz">
        En esta pasada {porNoPreguntar.length === 1 ? 'el recuento de' : 'los recuentos de'}
        <strong>{porNoPreguntar.join(', ')}</strong>
        {porNoPreguntar.length === 1 ? 'es cero' : 'son cero'} porque no se le
        preguntó a ningún remoto: se recompiló todo sin mirar si había cambiado.
      </p>
    {/if}
    {#if resultado.skipped === 0}
      <p class="matiz">
        <strong>Saltadas</strong> sólo lo produce el autodespliegue, cuando un
        commit ya rompió el build y espera al siguiente. Una pasada lanzada a
        mano lo reintenta siempre, y por eso ese recuento es cero.
      </p>
    {/if}
  {:else if corriendo}
    <p class="marcador" role="status">
      {pasada.terminadas}
      {pasada.terminadas === 1 ? 'app hecha' : 'apps hechas'}
      {#if pasada.enCurso}
        · desplegando <strong>{pasada.enCurso}</strong>
        {#if pasada.apps.find((a) => a.app === pasada.enCurso)?.paso}
          <span class="paso">{pasada.apps.find((a) => a.app === pasada.enCurso)?.paso}</span>
        {/if}
      {/if}
      {#if pasada.transcurrido > 0}<span class="reloj">{dur(pasada.transcurrido)}</span>{/if}
    </p>

    <!-- La lista ES el progreso, y por eso no hay barra: una pasada no tiene un
         total fiable —el servidor salta las apps sin repositorio, y eso no sale
         en `list`— así que una fracción sería un denominador inventado. Lo que
         de verdad se quiere mirar es cuál va y cuál ha fallado. -->
    {#if pasada.apps.length > 0}
      <ul class="curso">
        {#each pasada.apps as a (a.app)}
          <li>
            <span class="nombre">{a.app}</span>
            {#if a.final}
              <span class="chip lote--{a.final}">{a.final}</span>
            {:else}
              <span class="haciendo">{a.paso ?? 'empezando'}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="tenue">esperando al servidor…</p>
    {/if}

    <div class="parar">
      <button type="button" onclick={alCancelar}>Parar la pasada</button>
      <!--
        La palabra «cancelar» sugiere deshacer, y aquí no se deshace nada. Lo
        que para es el bucle: las apps que ya se desplegaron siguen
        desplegadas. Decirlo aquí y no después es la diferencia entre una
        decisión y una sorpresa.
      -->
      <p class="ayuda">
        Para el bucle; <strong>no deshace lo ya desplegado</strong>. Las apps que
        ya terminaron se quedan como están, y la que esté a mitad puede quedarse
        a mitad.
      </p>
    </div>
  {:else}
    <p class="cuantas">
      Este servidor tiene <strong>{apps.length}</strong>
      {apps.length === 1 ? 'app' : 'apps'}{#if redirecciones > 0}, de las que
        {redirecciones}
        {redirecciones === 1 ? 'es una redirección y no se despliega' : 'son redirecciones y no se despliegan'}{/if}.
    </p>
    <!--
      El número de arriba es el que se sabe, y no es exactamente el que se va a
      tocar: el servidor también salta las apps sin repositorio configurado, y
      eso `list --json` no lo dice. Dar el número redondo sería dar por sabido
      algo que no se ha mirado.
    -->
    <p class="ayuda">
      El servidor también salta las que no tengan repositorio configurado, y eso
      <code>list</code> no lo dice: puede que toque alguna menos de
      {candidatas}.
    </p>

    <div class="opcion">
      <button type="button" class="primaria" onclick={() => alLanzar(true)}>
        Desplegar lo que haya cambiado
      </button>
      <p class="que">
        Le pregunta al remoto de cada app si hay algo nuevo y despliega
        <strong>sólo las que se han movido</strong>. Preguntar cuesta poco; es lo
        que se hace a diario. Las demás salen como «al día», que no es lo mismo
        que «sin contacto» — y las dos se ven aparte, porque confundirlas ya
        costó un fallo real.
      </p>
    </div>

    <!--
      En un submenú y no al lado de la otra. Es la misma decisión que en la
      pantalla de retirar: dos operaciones distintas no se ponen juntas, porque
      la que está al lado se elige sin leerla. Y aquí la de al lado son
      cuarenta builds.
    -->
    <details class="cara">
      <summary>Y recompilarlo todo, haya cambiado o no</summary>
      <div class="opcion">
        <button type="button" class="boton" onclick={() => alLanzar(false)}>
          Recompilar {candidatas > 0 ? candidatas : 'todas'}
          {candidatas === 1 ? 'app' : 'apps'}
        </button>
        <p class="que">
          Vuelve a compilar y publicar <strong>todas</strong>, incluidas las que
          no han cambiado: {candidatas === 1 ? 'una release nueva' : `${candidatas} releases nuevas`}
          de un código idéntico. Existe para cuando lo que ha cambiado no está en
          el repositorio —la versión de Node, un <code>.env</code> compartido, una
          librería del sistema— y no hay otra forma de decirlo.
        </p>
        <p class="que">
          Y no le pregunta a ningún remoto, así que <strong>sólo puede terminar
          en «desplegadas» o «fallidas»</strong>: los otros cuatro finales salen
          de preguntar.
        </p>
      </div>
    </details>
  {/if}
</section>

<style>
  .pasada { display: grid; gap: var(--e-4); max-width: 84ch; }
  header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }
  .cerrar {
    background: none; border: 0; padding: 0; font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer; text-decoration: underline;
  }

  .cuantas { margin: 0; font-size: 13px; color: var(--fg); }
  .ayuda, .matiz { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 76ch; }
  .tenue { margin: 0; font-size: 12px; color: var(--fg-faint); }

  .opcion { display: grid; gap: var(--e-2); }
  .que { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 76ch; }
  .primaria, .boton {
    justify-self: start;
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
  }
  .primaria { background: var(--accent-fill); color: var(--on-accent); border: 0; }
  .boton { background: none; border: 1px solid var(--border-strong); color: var(--fg); }
  .primaria:focus-visible, .boton:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .cara { display: grid; gap: var(--e-3); }
  summary { font-size: 13px; color: var(--fg-muted); cursor: pointer; }

  .marcador { margin: 0; font-size: 13px; color: var(--fg); }
  .paso { font-family: var(--mono); font-size: 12px; color: var(--fg-muted); margin-left: var(--e-2); }
  .reloj { float: right; font-family: var(--mono); font-size: 12px; color: var(--fg-faint); }

  .curso { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .curso li {
    display: grid; grid-template-columns: 1fr auto; gap: var(--e-3);
    align-items: baseline; padding: var(--e-1) 0;
    border-bottom: 1px solid var(--border);
  }
  .nombre { font-size: 13px; color: var(--fg); }
  .haciendo { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .chip {
    font-size: 11px; font-weight: 600; color: var(--chip, var(--fg-muted));
    font-family: var(--mono);
  }

  .parar { display: grid; gap: var(--e-2); }
  .parar button {
    justify-self: start;
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .parar button:hover { color: var(--fg); }
  .parar button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  code { font-family: var(--mono); font-size: 12px; }
</style>
