<script lang="ts">
  import { finalDe, ofreceRollback, QUE_MIRAR, type Progreso } from '../lib/despliegue'
  import type { Despliegue } from '../lib/contrato'

  let {
    app,
    servidor,
    progreso,
    resultado = null,
    crudo = '',
  }: {
    app: string
    servidor: string
    progreso: Progreso
    resultado?: Despliegue | null
    crudo?: string
  } = $props()

  let verCrudo = $state(false)
  const final = $derived(resultado ? finalDe(resultado) : null)

  function dur(s: number | null): string {
    if (s === null) return ''
    if (s < 60) return `${s} s`
    return `${Math.floor(s / 60)} min ${s % 60} s`
  }
  const GLIFO = { pendiente: '○', haciendo: '◐', hecho: '✓' } as const
</script>

<article class="despliegue">
  <header>
    <!-- Siempre `servidor : app`. El nombre de una app no identifica nada por
         sí solo: `tienda` existe en tres servidores. -->
    <p class="ruta"><span class="servidor">{servidor}</span> : <strong>{app}</strong></p>
    <p class="reloj">{dur(progreso.transcurrido)}</p>
  </header>

  {#if resultado === null}
    <p class="estado">Desplegando…</p>
  {/if}

  <!-- La barra está ponderada: seis pasos no valen un sexto cada uno, y el
       build es el 70-85 % del tiempo. Una lineal se quedaría clavada en el 33 %
       durante dos minutos y luego correría hasta el final. -->
  <div
    class="barra"
    class:barra--estimada={progreso.sinHistorico}
    role="progressbar"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(progreso.avance * 100)}
    aria-label="Progreso del despliegue de {app}"
  >
    <div class="relleno" style="width: {progreso.avance * 100}%"></div>
  </div>
  {#if progreso.sinHistorico && resultado === null}
    <!-- Orbit se calla la tendencia con menos de seis builds, porque dos datos
         no son una tendencia. La interfaz respeta ese silencio en vez de
         rellenarlo, y lo dice. -->
    <p class="nota">Sin histórico suficiente: la estimación es aproximada.</p>
  {/if}

  <ol class="pasos">
    {#each progreso.pasos as p (p.id)}
      <li class="paso paso--{p.estado}">
        <span class="glifo" aria-hidden="true">{GLIFO[p.estado] ?? '○'}</span>
        <span class="nombre">{p.id}</span>
        <!-- El cronómetro de cada paso se conserva al terminar: en tres
             despliegues alguien aprende cuánto tarda su build. -->
        <span class="tiempo">{p.segundos !== null ? dur(p.segundos) : ''}</span>
        <span class="que">{p.texto}</span>
      </li>
    {/each}
  </ol>

  {#if progreso.rotas > 0}
    <p class="nota">
      {progreso.rotas} {progreso.rotas === 1 ? 'línea de progreso no se ha entendido' : 'líneas de progreso no se han entendido'}.
      El despliegue sigue: una línea rota no lo tumba.
    </p>
  {/if}

  {#if resultado}
    <section class="final final--{final}" role="status">
      {#if final === 'bien'}
        <p class="titular">Desplegada en {dur(resultado.duration_s)}.</p>
        <p class="detalle">Release <code>{resultado.release}</code>.</p>
      {:else if final === 'recuperado'}
        <!-- Ni rojo ni exactamente igual que un éxito limpio: cuatro
             «recovered» seguidos son un patrón que alguien debería mirar. -->
        <p class="titular">Desplegada en {dur(resultado.duration_s)}, al segundo intento.</p>
        <p class="detalle">
          Orbit reconoció el fallo del build, lo arregló y reintentó una vez. El
          arreglo queda apuntado, así que el próximo sale a la primera.
        </p>
      {:else if final === 'revertido'}
        <!-- La primera línea NO habla del fallo. Es lo único que hace falta
             saber en el primer segundo, y ponerlo debajo del error sería
             enterrar la buena noticia. -->
        <p class="titular">Tu web sigue en pie.</p>
        <p class="detalle">
          El despliegue falló en <code>{resultado.failed_step}</code> y Orbit volvió a
          <code>{resultado.previous}</code>, que es lo que se está sirviendo.
        </p>
        {#if resultado.error}<pre>{resultado.error}</pre>{/if}
      {:else}
        <p class="titular">El despliegue ha fallado en <code>{resultado.failed_step}</code>.</p>
        {#if resultado.failed_step && QUE_MIRAR[resultado.failed_step]}
          <p class="detalle">{QUE_MIRAR[resultado.failed_step]!.mira}</p>
        {/if}
        {#if resultado.error}<pre>{resultado.error}</pre>{/if}
      {/if}

      <div class="acciones">
        {#if resultado.failed_step && QUE_MIRAR[resultado.failed_step]}
          <button type="button" class="primaria">{QUE_MIRAR[resultado.failed_step]!.accion}</button>
        {/if}
        {#if ofreceRollback(resultado)}
          <!-- No es la acción primaria. En un fallo, lo primero es ENTENDER; y
               en buena parte de los casos volver atrás ni siquiera es lo que se
               quiere: si el build falló, `current` no se movió y no hay nada a
               lo que volver. -->
          <button type="button" class="secundaria">
            Volver a {resultado.previous}
          </button>
        {/if}
        <button type="button" class="secundaria" onclick={() => (verCrudo = !verCrudo)}>
          {verCrudo ? 'Ocultar' : 'Ver'} la salida cruda
        </button>
      </div>
    </section>
  {/if}

  {#if verCrudo}
    <!-- Sin traducir y tal cual. Es lo que se pega en un issue. -->
    <pre class="crudo">{crudo}</pre>
  {/if}
</article>

<style>
  .despliegue { padding: var(--e-5); }
  header { display: flex; align-items: baseline; justify-content: space-between; }
  .ruta { margin: 0; font-size: 18px; color: var(--fg); }
  .servidor { color: var(--fg-muted); }
  .reloj { margin: 0; font-family: var(--mono); font-size: 13px; color: var(--fg-muted); }
  .estado { margin: var(--e-1) 0 var(--e-4); color: var(--fg-muted); font-size: 13px; }
  .barra {
    height: 8px; border-radius: 999px; overflow: hidden;
    background: var(--surface-sunken); margin-bottom: var(--e-2);
  }
  .relleno { height: 100%; background: var(--accent-fill); transition: width .25s linear; }
  .barra--estimada .relleno {
    background: repeating-linear-gradient(
      90deg, var(--accent-fill) 0 8px, color-mix(in srgb, var(--accent-fill) 55%, transparent) 8px 16px);
  }
  .nota { margin: 0 0 var(--e-4); font-size: 12px; color: var(--fg-faint); }
  .pasos { list-style: none; margin: var(--e-4) 0 0; padding: 0; }
  .paso {
    display: grid; grid-template-columns: 16px 76px 74px 1fr;
    gap: var(--e-2); align-items: baseline;
    padding: 3px 0; font-size: 13px;
  }
  .glifo { font-family: var(--mono); font-size: 11px; }
  .nombre { font-family: var(--mono); font-size: 12px; color: var(--fg); }
  .tiempo { font-family: var(--mono); font-size: 12px; color: var(--fg-faint); text-align: right; }
  .que { color: var(--fg-muted); }
  .paso--pendiente .nombre, .paso--pendiente .que { color: var(--fg-faint); }
  .final {
    margin-top: var(--e-5); padding: var(--e-4);
    border: 1px solid var(--border); border-radius: var(--r-3);
  }
  .titular { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }
  .detalle { margin: var(--e-2) 0 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  code { font-family: var(--mono); font-size: 12px; }
  pre {
    margin: var(--e-3) 0 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; white-space: pre-wrap; color: var(--fg-muted);
    max-height: 300px; overflow: auto;
  }
  .crudo { margin-top: var(--e-4); }
  .acciones { display: flex; gap: var(--e-2); margin-top: var(--e-4); flex-wrap: wrap; }
  .primaria, .secundaria {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; cursor: pointer;
  }
  .primaria { background: var(--accent-fill); color: var(--on-accent); border: 0; font-weight: 600; }
  .secundaria { background: none; color: var(--fg); border: 1px solid var(--border-strong); }
  .primaria:focus-visible, .secundaria:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
