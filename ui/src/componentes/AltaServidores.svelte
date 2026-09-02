<script lang="ts">
  import type { AliasSsh, ServidorPropio } from '../lib/puente'
  import type { Saludo } from '../lib/contrato'

  let {
    alias,
    propios,
    saludos,
    comprobando,
    alComprobar,
    alUsar,
    alAnadir,
    alOlvidar,
    alInstalar,
  }: {
    /** Los `Host` del `~/.ssh/config`. Se leen y no se tocan. */
    alias: AliasSsh[]
    /** Los añadidos a mano, que se guardan aquí. */
    propios: ServidorPropio[]
    /** Lo que contestó cada uno. Sin comprobar es `undefined`, que es distinto
     *  de «no contestó»: enumerar no es visitar. */
    saludos: Record<string, Saludo | null>
    comprobando: string | null
    alComprobar: (a: string) => void
    alUsar: (a: string) => void
    alAnadir: () => void
    alOlvidar: (a: string) => void
    alInstalar: (a: string) => void
  } = $props()

  const TEXTO: Record<string, { titulo: string; que: string }> = {
    ok: { titulo: 'Listo', que: 'Habla el contrato que este cliente entiende.' },
    'mas-nuevo': {
      titulo: 'Más nuevo',
      que: 'Habla un contrato más nuevo. Se puede mirar, no operar: lo que no se entiende no se toca, y no se adivina.',
    },
    'sin-contrato': {
      titulo: 'Orbit demasiado antiguo',
      que: 'Este Orbit es anterior al contrato --json. No es un fallo: es un servidor sano y viejo. Reinstalar lo actualiza.',
    },
    'no-instalado': {
      titulo: 'Sin Orbit',
      que: 'No hay ningún Orbit en este servidor todavía.',
    },
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

  interface Fila {
    alias: string
    donde: string
    /** De dónde salió. Se dice siempre: son dos sitios distintos y se
     *  desapuntan de forma distinta. */
    fuente: 'config' | 'propio'
  }

  const filas = $derived<Fila[]>([
    ...propios.map((p) => ({
      alias: p.alias,
      donde: `${p.usuario}@${p.host}${p.puerto !== 22 ? `:${p.puerto}` : ''}`,
      fuente: 'propio' as const,
    })),
    ...alias.map((a) => ({
      alias: a.alias,
      donde: a.hostname
        ? `${a.usuario ? `${a.usuario}@` : ''}${a.hostname}${a.puerto && a.puerto !== 22 ? `:${a.puerto}` : ''}${a.salto ? ` · por ${a.salto}` : ''}`
        : 'lo resuelve tu ~/.ssh/config',
      fuente: 'config' as const,
    })),
  ])
</script>

{#if filas.length === 0}
  <!--
    La primera pantalla de verdad, y la que estaba rota: la aplicación sacaba los
    servidores SÓLO del ~/.ssh/config, así que quien no tuviera ese fichero
    —mucha gente, y en Windows casi nadie lo tiene— abría una lista vacía sin
    ninguna salida. Un producto que no se puede empezar a usar no es un producto.
  -->
  <div class="bienvenida">
    <div class="orbe" aria-hidden="true"></div>
    <h2>Añade tu primer servidor</h2>
    <p>
      Un servidor con Ubuntu o Debian al que llegues por SSH. Si todavía no tiene
      Orbit, se instala desde aquí.
    </p>
    <button type="button" class="primario grande" onclick={alAnadir}>Añadir un servidor</button>
    <p class="tenue">
      Si tienes un <code>~/.ssh/config</code> con servidores dentro, saldrán aquí
      solos.
    </p>
  </div>
{:else}
  <div class="cabecera">
    <p class="intro">
      Preguntar a uno es un gesto aparte: <strong>abrir esta pantalla no habla
      con ninguno</strong>, porque no puede significar abrir cuarenta sesiones
      SSH.
    </p>
    <button type="button" class="primario" onclick={alAnadir}>Añadir</button>
  </div>

  <ul class="lista">
    {#each filas as f, i (f.alias)}
      {@const s = saludos[f.alias]}
      <li class="fila" style="--retraso: {Math.min(i, 8) * 28}ms">
        <div class="quien">
          <span class="alias">{f.alias}</span>
          <span class="donde">
            {f.donde}
            {#if f.fuente === 'propio'}<span class="marca">añadido a mano</span>{/if}
          </span>
        </div>

        <div class="veredicto">
          {#if comprobando === f.alias}
            <span class="tenue"><span class="giro" aria-hidden="true"></span> preguntando…</span>
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
          <button type="button" onclick={() => alComprobar(f.alias)}>comprobar</button>
          {#if s?.puede_leer}
            <button type="button" class="usar" onclick={() => alUsar(f.alias)}>abrir</button>
          {/if}
          {#if f.fuente === 'propio'}
            <button type="button" onclick={() => alOlvidar(f.alias)}>quitar</button>
          {/if}
        </div>

        {#if s && s.clase !== 'ok'}
          <div class="detalle">
            <p>
              {TEXTO[s.clase]?.que}
              {#if s.motivo}<span class="motivo">{s.motivo}</span>{/if}
            </p>
            {#if s.clase === 'no-instalado' || s.clase === 'sin-contrato'}
              <!--
                Antes aquí había un comando para copiar y una explicación de por
                qué no había botón. El comando NO funcionaba —`curl … | sudo
                bash` muere porque install.sh necesita el fichero `orbit` a su
                lado— y la explicación defendía una regla que se sostenía mal: lo
                que iba a ejecutar quien copiara era exactamente lo mismo que
                ejecuta este botón.
              -->
              <button type="button" class="primario" onclick={() => alInstalar(f.alias)}>
                {s.clase === 'sin-contrato' ? 'Actualizar Orbit' : 'Instalar Orbit'}
              </button>
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .bienvenida {
    display: grid; justify-items: center; text-align: center;
    gap: var(--e-3); padding: var(--e-5) var(--e-4);
    max-width: 46ch; margin: 0 auto;
    animation: aterriza var(--t-lento) var(--e-entrada);
  }
  .bienvenida h2 { margin: 0; font-size: 20px; font-weight: 600; color: var(--fg); }
  .bienvenida p { margin: 0; font-size: 14px; color: var(--fg-muted); }
  .bienvenida .tenue { font-size: 12px; color: var(--fg-faint); }

  /* El orbe. Es lo único decorativo de toda la aplicación y está aquí a
     propósito: es la pantalla vacía, no hay ningún dato que enseñar, y un vacío
     con vida se lee como «va a pasar algo» en vez de como «esto está roto». En
     cuanto hay un servidor desaparece para siempre. */
  .orbe {
    width: 72px; height: 72px; border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, var(--accent), transparent 68%);
    border: 1px solid var(--border-strong);
    animation: respira 4.5s var(--e-suave) infinite;
    margin-bottom: var(--e-2);
  }
  @keyframes respira {
    0%, 100% { transform: scale(1); opacity: .85; }
    50% { transform: scale(1.06); opacity: 1; }
  }
  @keyframes aterriza {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: none; }
  }

  .cabecera {
    display: flex; align-items: start; justify-content: space-between;
    gap: var(--e-4); margin-bottom: var(--e-4);
  }
  .intro { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }

  .lista { list-style: none; margin: 0; padding: 0; }
  .fila {
    display: grid; grid-template-columns: 1fr auto auto;
    gap: var(--e-3); align-items: center;
    padding: var(--e-3) 0; border-top: 1px solid var(--border);
    /* Escalonadas, y con tope: ocho filas de retraso creciente se leen como una
       lista que aparece; cuarenta se leen como una lista que va lenta. */
    animation: entra-fila var(--t-normal) var(--e-entrada) both;
    animation-delay: var(--retraso, 0ms);
  }
  @keyframes entra-fila {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: none; }
  }

  .quien { min-width: 0; }
  .alias { display: block; font-size: 14px; color: var(--fg); font-weight: 600; }
  .donde {
    display: block; font-family: var(--mono); font-size: 11px; color: var(--fg-faint);
  }
  .marca {
    font-family: var(--fuente); margin-left: var(--e-2);
    padding: 1px var(--e-2); border-radius: var(--r-1);
    background: var(--surface-sunken); font-size: 10px;
  }

  .veredicto { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .saludo { font-size: 12px; font-weight: 600; color: var(--saludo, var(--fg-muted)); }
  .version { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .tenue { font-size: 12px; color: var(--fg-faint); display: inline-flex; align-items: center; gap: var(--e-1); }

  .acciones { display: flex; gap: var(--e-2); }
  .acciones button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: 3px var(--e-2); font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer;
    transition: color var(--t-rapido) var(--e-suave), border-color var(--t-rapido) var(--e-suave);
  }
  .acciones button:hover { color: var(--fg); border-color: var(--accent); }
  .acciones button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .usar { color: var(--fg) !important; }

  .detalle {
    grid-column: 1 / -1; display: grid; gap: var(--e-3);
    justify-items: start; margin-top: var(--e-2);
    animation: entra-fila var(--t-normal) var(--e-entrada);
  }
  .detalle p { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 72ch; }
  .motivo { display: block; font-family: var(--mono); margin-top: 2px; color: var(--fg-faint); }

  .primario {
    background: var(--accent-fill); color: var(--on-accent); border: 0;
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
    transition: transform var(--t-rapido) var(--e-suave), filter var(--t-rapido) var(--e-suave);
  }
  .grande { padding: var(--e-3) var(--e-5); font-size: 15px; }
  .primario:hover { filter: brightness(1.08); }
  .primario:active { transform: translateY(1px); }
  .primario:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }

  .giro {
    width: 10px; height: 10px; border-radius: 50%;
    border: 2px solid currentColor; border-top-color: transparent;
    animation: gira 700ms linear infinite;
  }
  @keyframes gira { to { transform: rotate(360deg); } }

  code { font-family: var(--mono); font-size: 12px; }
</style>
