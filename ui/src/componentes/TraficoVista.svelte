<script lang="ts">
  import Barras from './Barras.svelte'
  import { num, type Trafico } from '../lib/contrato'

  let { trafico, servidor }: { trafico: Trafico; servidor: string } = $props()

  // Lo automático se resta para dar el de personas, y las dos cifras se
  // enseñan. En un VPS con IP pública buena parte del tráfico son escáneres
  // buscando `/.git/config`, y sumarlo convierte la analítica en un número que
  // no describe a nadie.
  const humano = $derived(
    trafico.requests === null || trafico.automated === null
      ? null
      : trafico.requests - trafico.automated,
  )
  const horas = $derived((trafico.hours ?? []).map((h) => ({ x: h.hour, y: h.requests })))
  const hayLatencia = $derived(trafico.latency_ms.lines > 0)
</script>

{#if !trafico.complete}
  <!--
    La ventana pedida excede lo que el log cubre. Se DICE, y no se devuelve un
    número más pequeño callándose: un total recortado sin avisar se lee como el
    total, y entonces alguien concluye que su web tiene menos visitas de las que
    tiene.
  -->
  <p class="aviso" role="status">
    El log no llega a cubrir <code>{trafico.since}</code>: lo que ves empieza donde
    empieza el log, no donde pediste. Los totales son de esa ventana más corta.
  </p>
{/if}

<div class="cifras">
  <div class="cifra">
    <span class="n">{num(humano)}</span>
    <span class="et">peticiones de personas</span>
  </div>
  <div class="cifra">
    <span class="n">{num(trafico.automated)}</span>
    <!-- Aparte y con su nombre: no es «ruido», es tráfico automático, y saber
         cuánto hay es un dato en sí mismo. -->
    <span class="et">automáticas</span>
  </div>
  <div class="cifra">
    <span class="n">{num(trafico.ips)}{#if trafico.ips_capped}<span class="tope">+</span>{/if}</span>
    <!-- IPs, no personas. Una misma persona con datos móviles son tres, y una
         oficina entera detrás de un NAT es una. -->
    <span class="et">IPs distintas</span>
  </div>
  <div class="cifra">
    <span class="n">{trafico.status['5xx'] ?? 0}</span>
    <span class="et">errores del servidor</span>
  </div>
</div>

{#if horas.length > 0}
  <h3>Por hora</h3>
  <Barras datos={horas} etiqueta="Peticiones por hora en {trafico.app}" />
{/if}

<h3>Latencia</h3>
{#if hayLatencia}
  <p class="latencia">
    p50 <strong>{trafico.latency_ms.p50} ms</strong> ·
    p95 <strong>{trafico.latency_ms.p95} ms</strong> ·
    máx <strong>{trafico.latency_ms.max} ms</strong>
    <span class="sobre">sobre {trafico.latency_ms.lines} peticiones</span>
  </p>
{:else}
  <!--
    Cero muestras. Los tres percentiles llegan `null` y **no se pintan**: un
    percentil sin muestras no es un cero, es nada. Y se dice por qué, porque el
    motivo tiene arreglo.
  -->
  <p class="sin-datos">
    No hay tiempos de respuesta en este log. El formato antiguo de nginx no los
    escribe; se arregla con <code>orbit nginx-rebuild</code>.
  </p>
{/if}

<h3>Rutas más pedidas</h3>
<ul class="rutas">
  {#each trafico.paths.slice(0, 8) as p (p.path)}
    <li><code>{p.path}</code><span class="n-ruta">{p.requests}</span></li>
  {/each}
</ul>

<p class="pie">
  Del log que nginx ya escribe: sin cookies, sin JavaScript y sin nada nuevo
  corriendo. En <strong>{servidor}</strong>.
</p>

<style>
  .aviso {
    margin: 0 0 var(--e-4); padding: var(--e-3);
    background: var(--surface-2); border-radius: var(--r-2);
    font-size: 12px; color: var(--fg-muted); max-width: 72ch;
  }
  .cifras { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--e-3); }
  .cifra { padding: var(--e-3); background: var(--surface-2); border-radius: var(--r-2); }
  .n { display: block; font-size: 22px; font-weight: 700; color: var(--fg); font-family: var(--mono); }
  .et { display: block; font-size: 11px; color: var(--fg-muted); margin-top: 2px; }
  .tope { color: var(--fg-faint); }
  h3 { font-size: 12px; text-transform: uppercase; letter-spacing: .04em;
       color: var(--fg-faint); margin: var(--e-5) 0 var(--e-2); }
  .latencia { margin: 0; font-size: 13px; color: var(--fg); }
  .sobre { color: var(--fg-faint); font-size: 12px; }
  .sin-datos { margin: 0; font-size: 13px; color: var(--fg-muted); max-width: 68ch; }
  .rutas { list-style: none; margin: 0; padding: 0; font-size: 13px; }
  .rutas li { display: flex; justify-content: space-between; gap: var(--e-3);
              padding: 3px 0; border-bottom: 1px solid var(--border); }
  .n-ruta { font-family: var(--mono); color: var(--fg-muted); }
  code { font-family: var(--mono); font-size: 12px; }
  .pie { margin: var(--e-5) 0 0; font-size: 12px; color: var(--fg-faint); max-width: 68ch; }
</style>
