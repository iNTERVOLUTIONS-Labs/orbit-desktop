<script lang="ts">
  import HojaDeComando from './HojaDeComando.svelte'
  import type { AppInfo } from '../lib/contrato'

  let {
    info,
    servidor,
    alRevertir,
  }: {
    info: AppInfo
    servidor: string
    alRevertir: (release: string) => void
  } = $props()

  // La activa es la primera: `releases_desc` las da de la más nueva a la más
  // vieja. Ni se ofrece — Orbit devuelve un aviso amable si la eliges, pero un
  // cliente no debe ni presentarla: reiniciaría el servicio y recargaría nginx
  // para dejarlo todo exactamente igual.
  const activa = $derived(info.releases[0] ?? null)
  const elegibles = $derived(info.releases.slice(1))

  // No se escribe el nombre. Es reversible, y **la fricción es un recurso
  // escaso**: pedirla aquí enseña a teclear nombres sin leer, y entonces se
  // teclea también en el borrado, que es donde no hay vuelta atrás.
  let elegida = $state<string | null>(null)

  function legible(r: string): string {
    const m = r.match(/^(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})/)
    return m ? `${m[3]}/${m[2]}/${m[1]} a las ${m[4]}:${m[5]}` : r
  }
  /** «Vas a volver 3 despliegues atrás» es información; «vas a volver a
   *  20260805-041230» no lo es. */
  const cuantos = $derived(elegida ? info.releases.indexOf(elegida) : 0)
</script>

<section class="revertir">
  <h3>Volver a una release anterior</h3>

  {#if elegibles.length === 0}
    <p class="vacio">
      Sólo hay una release: no hay a dónde volver todavía.
    </p>
  {:else}
    <ul class="releases">
      <li class="release release--activa">
        <span class="cual">{activa}</span>
        <span class="cuando">{activa ? legible(activa) : ''}</span>
        <span class="marca">sirviendo ahora</span>
      </li>
      {#each elegibles as r (r)}
        <li class="release">
          <span class="cual">{r}</span>
          <span class="cuando">{legible(r)}</span>
          <button type="button" onclick={() => (elegida = r)}>volver aquí</button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if elegida}
    <HojaDeComando
      titulo="Volver a {elegida}"
      {servidor}
      orden="orbit rollback {info.name} {elegida}"
      consecuencia={consecuencia(cuantos, info.state.autodeploy)}
      verbo="Volver atrás"
      alConfirmar={() => { const r = elegida!; elegida = null; alRevertir(r) }}
      alCancelar={() => (elegida = null)}
    />
  {/if}
</section>

<script lang="ts" module>
  /**
   * Lo que cuesta volver atrás, dicho entero.
   *
   * Dos avisos, y ninguno es opcional:
   *
   *  · **Las migraciones van SIEMPRE**, se detecte una o no. El cliente no puede
   *    saber si el despliegue que se revierte traía una; lo que sí puede es no
   *    dejar que se olvide. El código vuelve atrás y los datos no, así que una
   *    app puede quedar apuntando a un esquema que ya no entiende.
   *  · **El autodespliegue**, cuando está puesto, porque es el error que más
   *    veces se comete: se revierte a las tres de la mañana y a las 3:05 el
   *    temporizador vuelve a poner la versión rota.
   */
  export function consecuencia(cuantos: number, autodespliegue: boolean): string {
    const cuantas =
      cuantos === 1
        ? 'Vuelves un despliegue atrás. '
        : cuantos > 1
          ? `Vuelves ${cuantos} despliegues atrás. `
          : ''
    const base =
      'Mueve el symlink, reinicia el servicio y recarga nginx: la web deja de ' +
      'responder entre uno y dos segundos. ' +
      'El código vuelve atrás, pero los datos NO: si ese despliegue traía una ' +
      'migración de base de datos, la app va a quedar apuntando a un esquema que ' +
      'no entiende.'
    const auto = autodespliegue
      ? ' Y esta app tiene el autodespliegue puesto: el siguiente ciclo del ' +
        'temporizador volverá a poner la versión de la que estás saliendo. ' +
        'Desactívalo antes si quieres que esto dure.'
      : ''
    return cuantas + base + auto
  }
</script>

<style>
  .revertir { padding: var(--e-5); }
  h3 { margin: 0 0 var(--e-4); font-size: 15px; color: var(--fg); }
  .vacio { margin: 0; font-size: 13px; color: var(--fg-muted); }
  .releases { list-style: none; margin: 0; padding: 0; }
  .release {
    display: grid; grid-template-columns: 180px 1fr auto;
    gap: var(--e-3); align-items: center;
    padding: var(--e-2) 0; border-top: 1px solid var(--border);
  }
  .cual { font-family: var(--mono); font-size: 12px; color: var(--cual, var(--fg)); }
  .cuando { font-size: 13px; color: var(--fg-muted); }
  .marca { font-size: 11px; color: var(--fg-faint); }
  .release button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .release button:hover { color: var(--fg); }
  .release button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
