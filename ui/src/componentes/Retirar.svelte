<script lang="ts">
  import HojaDeComando from './HojaDeComando.svelte'
  import type { AppInfo, Entorno } from '../lib/contrato'

  let {
    app,
    servidor,
    info,
    entorno,
    alRetirar,
    alCerrar,
  }: {
    app: string
    servidor: string
    /** El inventario se pide **en ese momento**, no se recuerda: decirle a
     *  alguien que va a perder 3 releases cuando tiene 12 es peor que no
     *  decírselo. */
    info: AppInfo | null
    entorno: Entorno | null
    alRetirar: (borrarDatos: boolean) => void
    alCerrar: () => void
  } = $props()

  // Dos operaciones, no una casilla. Una casilla junto a un botón se marca sin
  // leerla, y ésta se lee o no se lee nunca.
  let hoja = $state<'no' | 'retirar' | 'borrar'>('no')
</script>

<section class="retirar">
  <h3>Retirar «{app}» de {servidor}</h3>

  <div class="opcion">
    <button type="button" class="boton" onclick={() => (hoja = 'retirar')}>
      Retirar del servidor
    </button>
    <p class="que">
      Quita el vhost, el servicio y el descriptor. <strong>No borra nada de
      <code>/srv/apps/{app}</code></strong>, así que se rehace con
      <code>orbit new</code> en un minuto.
    </p>
  </div>

  <details class="peligrosa">
    <!-- En un submenú y no al lado del otro: la separación es la traducción a
         interfaz de la decisión que Orbit ya tomó al separar `--purge` de `-y`. -->
    <summary>Y borrar también sus datos</summary>
    <div class="opcion">
      <button type="button" class="boton boton--peligroso" onclick={() => (hoja = 'borrar')}>
        Retirar y borrar los datos
      </button>
      <p class="que">
        Además borra <code>/srv/apps/{app}</code>: el <code>.env</code>, todas las
        releases y <strong>las subidas de tus usuarios</strong>. Eso no vuelve.
      </p>
    </div>
  </details>

  {#if hoja === 'retirar'}
    <HojaDeComando
      titulo="Retirar «{app}»"
      {servidor}
      orden="orbit remove {app} -y"
      consecuencia="Quita el vhost, para y borra la unidad, quita el pool de php-fpm y borra el descriptor. Los datos se quedan donde están: si te arrepientes, se rehace con «orbit new» en un minuto."
      verbo="Retirar"
      alConfirmar={() => { hoja = 'no'; alRetirar(false) }}
      alCancelar={() => (hoja = 'no')}
    />
  {:else if hoja === 'borrar'}
    <HojaDeComando
      titulo="Retirar «{app}» y BORRAR sus datos"
      {servidor}
      orden="orbit remove {app} -y --purge"
      consecuencia={inventario(info, entorno, app)}
      verbo="Borrar"
      peligrosa={true}
      confirmarEscribiendo={app}
      alConfirmar={() => { hoja = 'no'; alRetirar(true) }}
      alCancelar={() => (hoja = 'no')}
    />
  {/if}

  <button type="button" class="cerrar" onclick={alCerrar}>Volver</button>
</section>

<script lang="ts" module>
  import type { AppInfo as AI, Entorno as En } from '../lib/contrato'

  /**
   * El inventario de lo que se va a perder, **de esta app y de este momento**.
   *
   * No un texto genérico: «esto borrará 5 releases y 12 variables» para una
   * acción concreta. Un aviso genérico se lee una vez y se ignora siempre; uno
   * con los números de lo que hay delante, no.
   *
   * Las variables se cuentan por su NOMBRE. Nunca se enseña un valor aquí,
   * aunque sea para despedirse de él.
   */
  export function inventario(info: AI | null, entorno: En | null, app: string): string {
    const trozos: string[] = []
    if (info) {
      trozos.push(`${info.releases.length} ${info.releases.length === 1 ? 'release' : 'releases'}`)
      if (info.state.last_deploy) trozos.push(`el último despliegue (${info.state.last_deploy})`)
    }
    if (entorno) {
      trozos.push(`${entorno.keys.length} ${entorno.keys.length === 1 ? 'variable' : 'variables'} de entorno`)
    }
    const lista = trozos.length > 0 ? `Se pierden ${trozos.join(', ')}, ` : 'Se pierde '
    return (
      lista +
      `el .env y todo lo que tus usuarios hayan subido a /srv/apps/${app}. ` +
      'Quitarla de nginx se deshace volviéndola a crear; esto no. ' +
      'Si tienes una copia con «orbit backup», se puede restaurar; si no, no.'
    )
  }
</script>

<style>
  .retirar { padding: var(--e-5); }
  h3 { margin: 0 0 var(--e-4); font-size: 15px; color: var(--fg); }
  .opcion { display: grid; gap: var(--e-2); margin-bottom: var(--e-4); }
  .boton {
    justify-self: start;
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-2);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 13px;
    color: var(--fg); cursor: pointer;
  }
  .boton:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .que { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 72ch; }
  .peligrosa summary {
    cursor: pointer; font-size: 13px; color: var(--fg-muted);
    padding: var(--e-2) 0; user-select: none;
  }
  .peligrosa summary:hover { color: var(--fg); }
  .cerrar {
    background: none; border: 0; padding: 0; margin-top: var(--e-4);
    font: inherit; font-size: 13px; color: var(--accent-text); cursor: pointer;
  }
  code { font-family: var(--mono); font-size: 12px; }
</style>
