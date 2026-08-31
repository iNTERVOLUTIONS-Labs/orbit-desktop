<script lang="ts">
  import type { AliasSsh } from '../lib/puente'
  import type { Saludo } from '../lib/contrato'

  let {
    alias,
    saludos,
    comprobando,
    alComprobar,
    alUsar,
  }: {
    alias: AliasSsh[]
    /** Lo que contestó cada uno. Sin comprobar es `undefined`, que es distinto
     *  de «no contestó»: enumerar no es visitar. */
    saludos: Record<string, Saludo | null>
    comprobando: string | null
    alComprobar: (a: string) => void
    alUsar: (a: string) => void
  } = $props()

  const TEXTO: Record<string, { titulo: string; que: string }> = {
    ok: { titulo: 'Listo', que: 'Habla el contrato que este cliente entiende.' },
    'mas-nuevo': {
      titulo: 'Más nuevo',
      que: 'Habla un contrato más nuevo. Se puede mirar, no operar: lo que no se entiende no se toca, y no se adivina.',
    },
    'sin-contrato': {
      titulo: 'Orbit demasiado antiguo',
      que: 'Este Orbit es anterior al contrato --json. No es un fallo: es un servidor sano y viejo. Actualízalo con install.sh.',
    },
    'no-instalado': { titulo: 'Sin Orbit', que: '' },
    'sin-privilegios': {
      titulo: 'Falta poder elevarse',
      que: 'Ese usuario necesita contraseña para sudo, y aquí no hay terminal donde escribirla. Entra como root, o dale sudo sin contraseña para /usr/local/bin/orbit.',
    },
    'no-se-llega': { titulo: 'No se llega', que: '' },
    'clave-de-host-cambiada': {
      titulo: 'La clave ha cambiado',
      que: 'Puede ser una reinstalación. También puede ser que estés hablando con otra máquina. No se sigue por aquí.',
    },
  }
</script>

<p class="intro">
  Los servidores salen de tu <code>~/.ssh/config</code>. Enumerarlos
  <strong>no habla con ninguno</strong>: abrir esta pantalla no puede significar
  abrir cuarenta sesiones SSH, así que preguntar por cada uno es un gesto aparte.
</p>

{#if alias.length === 0}
  <p class="vacio">
    No hay ningún <code>Host</code> en tu <code>~/.ssh/config</code>, o el fichero
    no existe. No es un error: mucha gente no lo tiene.
  </p>
{:else}
  <ul class="lista">
    {#each alias as a (a.alias)}
      {@const s = saludos[a.alias]}
      <li class="fila">
        <div class="quien">
          <span class="alias">{a.alias}</span>
          {#if a.hostname}
            <span class="donde">
              {a.usuario ? `${a.usuario}@` : ''}{a.hostname}{a.puerto && a.puerto !== 22 ? `:${a.puerto}` : ''}
              {#if a.salto}
                <!-- Un salto se anuncia porque cambia lo que se puede prometer
                     sobre la latencia: el saludo se paga dos veces. -->
                · por {a.salto}
              {/if}
            </span>
          {:else}
            <!-- Sin datos no se pinta «?@?». Un interrogante donde va un dato
                 se lee como un dato roto, y esto es simplemente que todavía no
                 se ha preguntado a `ssh -G`. -->
            <span class="donde donde--sin">lo resuelve tu ~/.ssh/config</span>
          {/if}
        </div>

        <div class="veredicto">
          {#if comprobando === a.alias}
            <span class="tenue">preguntando…</span>
          {:else if s === undefined}
            <span class="tenue">sin comprobar</span>
          {:else if s === null}
            <span class="tenue">·</span>
          {:else}
            <span class="saludo saludo--{s.clase}">{TEXTO[s.clase]?.titulo ?? s.clase}</span>
            {#if s.version}<span class="version">Orbit {s.version}</span>{/if}
          {/if}
        </div>

        <div class="acciones">
          <button type="button" onclick={() => alComprobar(a.alias)}>comprobar</button>
          {#if s?.puede_leer}
            <button type="button" class="usar" onclick={() => alUsar(a.alias)}>abrir</button>
          {/if}
        </div>

        {#if s && s.clase !== 'ok'}
          <p class="detalle">
            {TEXTO[s.clase]?.que}
            {#if s.motivo}<span class="motivo">{s.motivo}</span>{/if}
          </p>
          {#if s.clase === 'no-instalado'}
            <!--
              Para COPIAR, nunca un botón de «instalar». Instalar Orbit desde
              aquí sería la primera vez que este cliente escribe en el servidor
              algo que no es una invocación de `orbit`, y la regla nº 1 no admite
              un «pero es el instalador». Se copia el comando; lo ejecuta la
              persona.
            -->
            <pre class="instalar">{s.orden_de_instalacion}</pre>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .intro { margin: 0 0 var(--e-4); font-size: 13px; color: var(--fg-muted); max-width: 72ch; }
  .vacio { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .lista { list-style: none; margin: 0; padding: 0; }
  .fila {
    display: grid; grid-template-columns: 1fr auto auto;
    gap: var(--e-3); align-items: center;
    padding: var(--e-3) 0; border-top: 1px solid var(--border);
  }
  .quien { min-width: 0; }
  .alias { display: block; font-size: 13px; color: var(--fg); font-weight: 600; }
  .donde { display: block; font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .donde--sin { font-family: var(--fuente); font-style: italic; }
  .veredicto { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .saludo { font-size: 12px; font-weight: 600; color: var(--saludo, var(--fg-muted)); }
  .version { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .tenue { font-size: 12px; color: var(--fg-faint); }
  .acciones { display: flex; gap: var(--e-2); }
  .acciones button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 2px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
  }
  .acciones button:hover { color: var(--fg); }
  .acciones button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .usar { color: var(--fg) !important; }
  .detalle {
    grid-column: 1 / -1; margin: var(--e-2) 0 0;
    font-size: 12px; color: var(--fg-muted); max-width: 72ch;
  }
  .motivo { display: block; font-family: var(--mono); margin-top: 2px; color: var(--fg-faint); }
  .instalar {
    grid-column: 1 / -1; margin: var(--e-2) 0 0; padding: var(--e-2) var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    white-space: pre-wrap; word-break: break-all; user-select: all;
  }
  code { font-family: var(--mono); font-size: 12px; }
</style>
