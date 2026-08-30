<script lang="ts">
  import { bytes, num, salud, PRESENTACION, type Monitor } from '../lib/contrato'

  let {
    monitor,
    servidor,
    periodo,
  }: { monitor: Monitor; servidor: string; periodo: number } = $props()
</script>

<div class="barra">
  <p class="intro">
    {monitor.apps.length} apps en <strong>{servidor}</strong>.
  </p>
  <!--
    El periodo se ENSEÑA, y no es un adorno.

    Medido: `orbit top --json` cuesta ~2,1 s con 40 apps, y no es lentitud — la
    CPU es la diferencia entre dos lecturas del cgroup, así que una foto suelta
    tiene que esperar a la segunda a propósito. El plan de refrescar cada dos
    segundos era físicamente imposible, y venía de copiar el intervalo del panel
    en vivo de Orbit sin ver que ése reutiliza la muestra anterior.

    Así que el periodo se adapta a lo que de verdad tarda, y se dice: una
    pantalla que promete «en vivo» y refresca cada cinco segundos está mintiendo
    por omisión.
  -->
  <p class="periodo">cada {periodo} s</p>
</div>

<table class="tabla">
  <caption class="sr">Consumo de las apps de {servidor}</caption>
  <thead>
    <tr>
      <th scope="col">App</th>
      <th scope="col">Estado</th>
      <th scope="col" class="num">CPU</th>
      <th scope="col" class="num">Memoria</th>
      <th scope="col" class="num">Peticiones / min</th>
    </tr>
  </thead>
  <tbody>
    {#each monitor.apps as a (a.name)}
      <tr>
        <td class="nombre">{a.name}</td>
        <td class="tenue">
          {PRESENTACION[salud({
            service: a.service, port: a.port, ssl: false, cert_days: null,
            maintenance: false, served: true, autodeploy: false, queue: false,
            releases: null, last_deploy: null, last_deploy_sha: null,
          })].texto}
        </td>
        <!--
          `cpu_percent` es `null` la primera vez, y eso es «no se sabe».
          Pintarlo como 0 % sería inventar una afirmación: un cero dice que la
          app no está consumiendo, y eso no se sabe todavía.
        -->
        <td class="num mono">{a.cpu_percent === null ? '·' : `${a.cpu_percent} %`}</td>
        <td class="num mono">{bytes(a.memory_bytes)}</td>
        <td class="num mono">
          {num(a.requests_last_minute)}{#if a.requests_capped}<span
            class="tope"
            title="El minuto llenó el tope de líneas de log que se miran: hay más peticiones de las que se ven."
          >+</span>{/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .barra { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  .intro { margin: 0 0 var(--e-3); font-size: 13px; color: var(--fg-muted); }
  .periodo { margin: 0; font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabla th {
    text-align: left; font-size: 11px; font-weight: 600; letter-spacing: .04em;
    text-transform: uppercase; color: var(--fg-faint);
    padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border);
  }
  .tabla td { padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border); }
  .num { text-align: right; }
  .mono { font-family: var(--mono); }
  .nombre { font-family: var(--mono); font-size: 12px; }
  .tenue { color: var(--fg-muted); }
  .tope { color: var(--fg-faint); }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
