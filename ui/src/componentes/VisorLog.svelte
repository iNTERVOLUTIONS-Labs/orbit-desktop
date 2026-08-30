<script lang="ts">
  import type { Log, Canal } from '../lib/contrato'

  let { log, app }: { log: Log; app: string } = $props()

  // El filtro por canal es la razón de ser de esta pantalla: la salida en prosa
  // mezcla el log de acceso con el de error sin decir cuál es cuál, y
  // distinguirlos es la primera pregunta de cualquiera que mira un log de
  // nginx. Aquí no sólo se distinguen: se pueden separar.
  let solo = $state<Canal | 'todo'>('todo')

  const visibles = $derived(
    solo === 'todo' ? log.lineas : log.lineas.filter((l) => l.stream === solo),
  )
  const hayCanal = (c: Canal) => log.lineas.some((l) => l.stream === c)

  // La marca de tiempo se pinta corta, pero el título lleva la original entera:
  // el huso importa cuando se compara con otro servidor, y recortarlo sin
  // dejarlo a mano sería tirar el dato.
  function hora(ts: string | null): string {
    if (ts === null) return '·'
    const m = ts.match(/T(\d{2}:\d{2}:\d{2})/)
    return m ? m[1]! : ts
  }
</script>

<div class="barra">
  <div class="filtros" role="group" aria-label="Filtrar por origen">
    <button type="button" class:activo={solo === 'todo'} onclick={() => (solo = 'todo')}>
      todo <span class="cuenta">{log.lineas.length}</span>
    </button>
    {#each (['journal', 'access', 'error'] as Canal[]).filter(hayCanal) as c (c)}
      <button type="button" class:activo={solo === c} onclick={() => (solo = c)}>
        {c} <span class="cuenta">{log.lineas.filter((l) => l.stream === c).length}</span>
      </button>
    {/each}
  </div>

  {#if log.meta?.follow}
    <span class="vivo">en vivo</span>
  {/if}
</div>

{#if log.truncado}
  <!-- Un número corto sin avisar se lee como «hay poco log», que suele ser lo
       contrario de lo que pasa. -->
  <p class="aviso">
    Se ha llegado al tope de líneas. Hay más log del que se ve: pide más con
    <code>--lines</code> o acota con una ventana de tiempo.
  </p>
{/if}

{#if log.rotas > 0}
  <!-- Se enseña. Si se callara, un log a medias se leería como uno completo. -->
  <p class="aviso">
    {log.rotas} {log.rotas === 1 ? 'línea no se ha entendido' : 'líneas no se han entendido'}
    y no se muestran.
  </p>
{/if}

{#if log.lineas.length === 0}
  <p class="vacio">
    «{app}» no tiene líneas en esta ventana. No es un error: puede que aún no
    haya recibido ninguna visita.
  </p>
{:else}
  <!-- `role="log"` con `aria-live="off"`: un log a cuarenta líneas por segundo
       leído en voz alta inutiliza la pantalla. Lo que se anuncia son las
       transiciones, no cada línea, y de eso se encarga otro sitio. -->
  <ol class="log" role="log" aria-live="off" aria-label="Log de {app}">
    {#each visibles as l, i (i)}
      <li class="linea log--{l.stream}">
        <time class="ts" title={l.ts ?? 'esta línea no lleva marca de tiempo'}>{hora(l.ts)}</time>
        <span class="canal" aria-hidden="true">{l.stream === 'error' ? '✕' : '·'}</span>
        <span class="texto">{l.text}</span>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .barra { display: flex; align-items: center; justify-content: space-between; gap: var(--e-3); margin-bottom: var(--e-3); }
  .filtros { display: flex; gap: var(--e-1); }
  .filtros button {
    background: none; border: 1px solid transparent; border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .filtros button:hover { color: var(--fg); }
  .filtros button.activo { border-color: var(--border-strong); color: var(--fg); }
  .filtros button:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .cuenta { font-family: var(--mono); font-size: 11px; opacity: .7; }
  .vivo { font-size: 11px; color: var(--fg-faint); }
  .aviso { margin: 0 0 var(--e-3); font-size: 12px; color: var(--fg-muted); }
  .aviso code { font-family: var(--mono); }
  .vacio { color: var(--fg-muted); font-size: 13px; }
  .log {
    list-style: none; margin: 0; padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; line-height: 1.7;
    max-height: 52vh; overflow: auto;
  }
  .linea { display: grid; grid-template-columns: 68px 12px 1fr; gap: var(--e-2); }
  .ts { color: var(--fg-faint); }
  .texto { white-space: pre-wrap; word-break: break-word; color: var(--fg); }
</style>
