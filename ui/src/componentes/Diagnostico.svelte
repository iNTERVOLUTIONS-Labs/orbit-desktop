<script lang="ts">
  import { PESO_NIVEL, type Comprobacion, type Doctor } from '../lib/contrato'

  let {
    doctor,
    servidor,
    alArreglar = null,
    arreglando = false,
  }: {
    doctor: Doctor
    servidor: string
    /** `null` mientras el servidor no sepa arreglar sin preguntar. */
    alArreglar?: (() => void) | null
    arreglando?: boolean
  } = $props()

  const ordenadas = $derived(
    [...doctor.checks].sort((a, b) => PESO_NIVEL[a.level] - PESO_NIVEL[b.level]),
  )
  // Cuántas arreglaría el servidor por su cuenta. Es lo que decide si tiene
  // sentido ofrecer nada, y se cuenta de `fixable` y no del nivel: hay avisos
  // que se arreglan solos y errores que no.
  const arreglables = $derived(doctor.checks.filter((c) => c.fixable).length)

  const GLIFO: Record<Comprobacion['level'], string> = {
    ok: '●', info: '·', warn: '▲', error: '✕',
  }
</script>

<div class="resumen">
  <p>
    <strong>{doctor.summary.error}</strong> {doctor.summary.error === 1 ? 'error' : 'errores'},
    <strong>{doctor.summary.warn}</strong> {doctor.summary.warn === 1 ? 'aviso' : 'avisos'},
    {doctor.summary.ok} bien.
  </p>

  {#if arreglables > 0}
    {#if alArreglar}
      <button type="button" class="arreglar" disabled={arreglando} onclick={alArreglar}>
        {arreglando ? 'Arreglando…' : `Arreglar ${arreglables}`}
      </button>
    {:else}
      <!--
        El botón no aparece cuando el servidor no puede hacerlo sin preguntar, y
        en su lugar va la orden para copiar. Un botón deshabilitado sería peor:
        invita a averiguar por qué no se puede pulsar, y la respuesta es una
        frase que se puede leer directamente.
      -->
      <p class="nota">
        Este servidor no puede aplicarlo sin una terminal delante. Desde la tuya:
        <code>ssh {servidor} orbit doctor --fix</code>
      </p>
    {/if}
  {/if}
</div>

<ul class="lista">
  {#each ordenadas as c (c.id)}
    <li class="fila diag--{c.level}">
      <span class="glifo" aria-hidden="true">{GLIFO[c.level]}</span>
      <div class="cuerpo">
        <p class="mensaje">{c.message}</p>
        {#if c.fix}
          <!--
            El texto del arreglo se enseña SIEMPRE que exista, lo aplique o no
            el servidor. Que Orbit no pueda hacerlo solo no significa que no se
            sepa qué hay que hacer, y ocultarlo dejaría a alguien con un
            problema y sin la frase que lo resuelve.
          -->
          <p class="arreglo">
            {c.fix}
            {#if !c.fixable}
              <span class="a-mano">· a mano</span>
            {/if}
          </p>
        {/if}
      </div>
      <span class="id">{c.id}</span>
    </li>
  {/each}
</ul>

<style>
  .resumen {
    display: flex; align-items: center; justify-content: space-between;
    gap: var(--e-4); margin-bottom: var(--e-4);
  }
  .resumen p { margin: 0; font-size: 13px; color: var(--fg); }
  .nota { color: var(--fg-muted); font-size: 12px; max-width: 52ch; }
  .nota code { font-family: var(--mono); }
  .arreglar {
    background: var(--accent-fill); color: var(--on-accent);
    border: 0; border-radius: var(--r-2);
    padding: var(--e-2) var(--e-4);
    font: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
    white-space: nowrap;
  }
  .arreglar:disabled { opacity: .6; cursor: progress; }
  .arreglar:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .lista { list-style: none; margin: 0; padding: 0; }
  .fila {
    display: grid; grid-template-columns: 16px 1fr auto;
    gap: var(--e-3); align-items: baseline;
    padding: var(--e-2) 0; border-top: 1px solid var(--border);
  }
  .glifo { font-family: var(--mono); font-size: 11px; }
  .cuerpo { min-width: 0; }
  .mensaje { margin: 0; font-size: 13px; color: var(--fg); }
  .arreglo { margin: 2px 0 0; font-size: 12px; color: var(--fg-muted); font-family: var(--mono); }
  /* Sin partirse: cuando cae al final de una línea larga, saltaba solo y
     parecía el principio de una frase nueva. */
  .a-mano { font-family: var(--fuente); font-style: italic; white-space: nowrap; }
  .id { font-size: 11px; color: var(--fg-faint); font-family: var(--mono); }
</style>
