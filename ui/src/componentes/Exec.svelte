<script lang="ts">
  import { parecePeligroso, type SalidaDeExec } from '../lib/contrato'

  let {
    app,
    servidor,
    usuario = 'el usuario de la app',
    correr,
  }: {
    app: string
    servidor: string
    usuario?: string
    correr: (shell: boolean, argumentos: string[]) => Promise<SalidaDeExec>
  } = $props()

  let texto = $state('')
  // El modo por defecto es el que NO sorprende: argumentos separados, sin
  // shell. Quien quiera que su `&&` sea un operador lo dice.
  let shell = $state(false)
  let corriendo = $state(false)
  let salida = $state<SalidaDeExec | null>(null)
  let aviso = $state<string | null>(null)
  /** Se advierte del `.env` **una vez por sesión**. Un aviso que sale siempre se
   *  aprende a ignorar, y entonces se ignoran también los que importan. */
  let visto = $state(false)

  // El histórico vive en memoria y **no toca el disco**. La gente escribe
  // `psql "postgresql://usuario:contraseña@…"` en esta caja, y `bash` ya tomó
  // esta decisión con HISTCONTROL — sólo que aquí el valor por defecto va al
  // revés, porque el ratio de comandos con secretos es mucho más alto.
  let historico = $state<string[]>([])

  const argumentos = $derived(
    shell ? [texto] : texto.split(/\s+/).filter((x) => x.length > 0),
  )
  const peligro = $derived(parecePeligroso(texto))
  const listo = $derived(texto.trim().length > 0 && !corriendo)

  async function lanzar() {
    if (!listo) return
    if (peligro && aviso !== texto) {
      // Una sola vez por comando: si volviera a preguntar tras confirmar, se
      // aprendería a pulsar dos veces sin leer ninguna.
      aviso = texto
      return
    }
    aviso = null
    corriendo = true
    visto = true
    try {
      salida = await correr(shell, argumentos)
      if (historico[0] !== texto) historico = [texto, ...historico].slice(0, 30)
    } finally {
      corriendo = false
    }
  }

  function teclado(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) lanzar()
  }
</script>

<!--
  Los cuatro datos, siempre visibles. No es un adorno: lo que se ejecuta aquí
  corre en un servidor de producción, como un usuario concreto y con el .env
  cargado, y nada de eso se deduce mirando una caja de texto.
-->
<p class="cabecera">
  Ejecuta un comando dentro de <code>{app}</code> en <strong>{servidor}</strong>,
  como <code>{usuario}</code>, con el <code>.env</code> cargado.
</p>

{#if !visto}
  <!-- Cierto, y la gente no lo sabe. Una vez por sesión. -->
  <p class="secretos">
    Lo que ejecutes aquí <strong>ve todos los secretos de esta app</strong>: Orbit
    carga su <code>.env</code> en el entorno antes de correr nada.
  </p>
{/if}

<div class="modos" role="group" aria-label="Cómo se interpreta lo que escribes">
  <!--
    La regla del argumento único, hecha explícita. Orbit decide por dentro según
    si le llega un argumento o varios; aplicar esa heurística en silencio haría
    imposible predecir cuándo un `&&` se ejecuta y cuándo se pasa como texto, y
    una herramienta de depuración que no es predecible tampoco sirve.
  -->
  <button type="button" class:activo={!shell} onclick={() => (shell = false)}>
    comando
    <span class="que">argumentos separados, sin shell</span>
  </button>
  <button type="button" class:activo={shell} onclick={() => (shell = true)}>
    shell
    <span class="que">lo interpreta <code>bash -lc</code></span>
  </button>
</div>

<label class="entrada">
  <span class="sr">Comando</span>
  <input
    type="text"
    bind:value={texto}
    onkeydown={teclado}
    placeholder={shell ? 'php artisan migrate --force && php artisan cache:clear' : 'php artisan migrate --force'}
    autocomplete="off"
    spellcheck="false"
  />
</label>

{#if texto.trim()}
  <!-- La orden exacta, ya escapada, ANTES de ejecutarla. -->
  <pre class="previa">orbit exec {app} {shell ? JSON.stringify(texto) : argumentos.join(' ')}</pre>
{/if}

{#if aviso === texto && peligro}
  <p class="peligro" role="alert">
    Esto <strong>{peligro}</strong>. Pulsa otra vez si es lo que quieres.
    <span class="matiz">
      Esta comprobación es una lista corta de patrones: para el error de dedos,
      no protege de nada.
    </span>
  </p>
{/if}

<div class="acciones">
  <button type="button" class="lanzar" disabled={!listo} onclick={lanzar}>
    {corriendo ? 'Ejecutando…' : 'Ejecutar'}
  </button>
  <!--
    Sin shell interactiva embebida. `orbit exec app` sin comando abre un bash, y
    un cliente sin terminal no puede con eso: fingir medio terminal es la peor
    solución de todas. Lo que se ofrece es la orden para pegarla en uno de
    verdad.
  -->
  <button
    type="button"
    class="copiar"
    onclick={() => navigator.clipboard?.writeText(`ssh -t ${servidor} orbit exec ${app}`)}
  >Copiar la orden para una terminal de verdad</button>
</div>

{#if salida}
  <div class="salida">
    <p class="orden">Se ejecutó: <code>{salida.orden}</code></p>
    <!--
      Texto plano, siempre. Es la salida de un proceso arbitrario: puede traer
      secuencias ANSI, bytes nulos o megas en una línea. Nunca se interpreta.
    -->
    {#if salida.stdout}<pre class="chorro">{salida.stdout}</pre>{/if}
    {#if salida.stderr}<pre class="chorro chorro--err">{salida.stderr}</pre>{/if}
    <p class="codigo" class:codigo--mal={salida.codigo !== 0}>
      Salió con {salida.codigo}.
    </p>
  </div>
{/if}

{#if historico.length > 0}
  <h3>Antes</h3>
  <ul class="historico">
    {#each historico as h, i (i)}
      <li><button type="button" onclick={() => (texto = h)}><code>{h}</code></button></li>
    {/each}
  </ul>
  <p class="matiz">
    Sólo en memoria: al cerrar se va. Aquí se escriben cadenas de conexión con
    contraseña dentro más a menudo de lo que parece.
  </p>
{/if}

<style>
  .cabecera { margin: 0 0 var(--e-3); font-size: 13px; color: var(--fg); max-width: 72ch; }
  .secretos {
    margin: 0 0 var(--e-4); padding: var(--e-3);
    background: var(--surface-2); border-radius: var(--r-2);
    font-size: 12px; color: var(--fg-muted); max-width: 72ch;
  }
  .modos { display: flex; gap: var(--e-2); margin-bottom: var(--e-3); }
  .modos button {
    text-align: left; background: none; border: 1px solid var(--border);
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; color: var(--fg-muted); cursor: pointer;
  }
  .modos button.activo { border-color: var(--border-strong); color: var(--fg); }
  .modos .que { display: block; font-size: 11px; color: var(--fg-faint); }
  .entrada input {
    width: 100%; padding: var(--e-2) var(--e-3);
    border: 1px solid var(--border-strong); border-radius: var(--r-2);
    background: var(--surface-2); color: var(--fg);
    font-family: var(--mono); font-size: 13px;
  }
  .previa {
    margin: var(--e-2) 0 0; padding: var(--e-2) var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg-muted);
    white-space: pre-wrap; word-break: break-all; user-select: all;
  }
  .peligro { margin: var(--e-3) 0 0; font-size: 13px; color: var(--fg); max-width: 72ch; }
  .matiz { display: block; font-size: 11px; color: var(--fg-faint); margin-top: 2px; }
  .acciones { display: flex; gap: var(--e-2); margin-top: var(--e-4); flex-wrap: wrap; }
  .lanzar, .copiar {
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; cursor: pointer;
  }
  .lanzar { background: var(--accent-fill); color: var(--on-accent); border: 0; font-weight: 600; }
  .lanzar:disabled { opacity: .45; cursor: not-allowed; }
  .copiar { background: none; color: var(--fg); border: 1px solid var(--border-strong); }
  .salida { margin-top: var(--e-5); }
  .orden { margin: 0 0 var(--e-2); font-size: 12px; color: var(--fg-faint); }
  .chorro {
    margin: 0 0 var(--e-2); padding: var(--e-3);
    background: var(--surface-sunken); border-radius: var(--r-2);
    font-family: var(--mono); font-size: 12px; color: var(--fg);
    white-space: pre-wrap; word-break: break-word;
    max-height: 40vh; overflow: auto;
  }
  .codigo { margin: 0; font-size: 12px; color: var(--fg-muted); font-family: var(--mono); }
  h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .04em;
       color: var(--fg-faint); margin: var(--e-5) 0 var(--e-2); }
  .historico { list-style: none; margin: 0; padding: 0; }
  .historico button {
    background: none; border: 0; padding: 2px 0; font: inherit;
    color: var(--fg-muted); cursor: pointer; text-align: left;
  }
  .historico button:hover { color: var(--fg); }
  code { font-family: var(--mono); font-size: 12px; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
