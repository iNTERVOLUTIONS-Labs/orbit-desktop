<script lang="ts">
  import { finalDe, recuentos } from '../lib/despliegue'
  import type { Lote } from '../lib/contrato'

  let { lote, servidor }: { lote: Lote; servidor: string } = $props()

  const cuentas = $derived(recuentos(lote))

  function dur(s: number): string {
    return s < 60 ? `${s} s` : `${Math.floor(s / 60)} min ${s % 60} s`
  }
</script>

<article class="lote">
  <header>
    <p class="ruta"><span class="servidor">{servidor}</span> · {lote.total} apps</p>
    <p class="reloj">{dur(lote.duration_s)}</p>
  </header>

  <!--
    Los SEIS recuentos, separados y con su propia celda.

    Existen seis y no dos porque confundir dos de ellos ya costó un fallo real:
    «al día» y «no he podido preguntar» valían lo mismo, así que un remoto caído
    se anunciaba como «nada que hacer» cada cinco minutos, durante días. El
    contrato los separa para que un cliente no pueda repetirlo, y **agruparlos
    en «correctas / fallidas» está prohibido**.

    Los que están a cero se pintan igual y en su sitio: que `unreachable` sea 0
    es una respuesta, y quitarlo de la fila haría que su aparición pasara
    desapercibida justo el día que importa.
  -->
  <ul class="recuentos">
    {#each cuentas as c (c.id)}
      <li class="cuenta lote--{c.id}" class:cuenta--cero={c.n === 0} title={c.frase}>
        <span class="n">{c.n}</span>
        <span class="etiqueta"><span class="glifo" aria-hidden="true">{c.glifo}</span> {c.texto}</span>
      </li>
    {/each}
  </ul>

  <!-- `ok` sigue la misma regla que el código de salida, para que quien mire el
       objeto y quien mire el rc no puedan discrepar nunca. -->
  <p class="veredicto" role="status">
    {#if lote.ok}
      Todas contestaron y ninguna falló.
    {:else}
      {@const n = lote.failed + lote.unreachable + lote.gone}
      {n === 1 ? 'Hay una app que necesita una mirada.' : `Hay ${n} apps que necesitan una mirada.`}
    {/if}
  </p>

  <table class="tabla">
    <caption class="sr">Resultado por app en {servidor}</caption>
    <thead>
      <tr><th scope="col">App</th><th scope="col">Final</th><th scope="col">Qué ha pasado</th></tr>
    </thead>
    <tbody>
      {#each lote.apps as a (a.app)}
        <tr>
          <td class="nombre">{a.app}</td>
          <td><span class="chip lote--{a.status}">{a.status}</span></td>
          <td class="que">
            {#if a.result}
              {#if finalDe(a.result) === 'revertido'}
                Falló en <code>{a.result.failed_step}</code> y volvió a
                <code>{a.result.previous}</code>: su web sigue en pie.
              {:else if finalDe(a.result) === 'recuperado'}
                Salió bien al segundo intento; Orbit arregló el build y reintentó.
              {:else if a.result.ok}
                Release <code>{a.result.release}</code>.
              {:else}
                Falló en <code>{a.result.failed_step}</code>. {a.result.error ?? ''}
              {/if}
            {:else if a.error}
              <!-- Sin objeto, el motivo va aquí: un null es una respuesta, un
                   objeto a medias no. -->
              {a.error}
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</article>

<style>
  .lote { padding: var(--e-5); }
  header { display: flex; align-items: baseline; justify-content: space-between; }
  .ruta { margin: 0; font-size: 18px; color: var(--fg); }
  .servidor { color: var(--fg-muted); }
  .reloj { margin: 0; font-family: var(--mono); font-size: 13px; color: var(--fg-muted); }
  .recuentos {
    list-style: none; margin: var(--e-4) 0 0; padding: 0;
    display: grid; grid-template-columns: repeat(6, 1fr); gap: var(--e-2);
  }
  .cuenta {
    padding: var(--e-3); border-radius: var(--r-2);
    background: color-mix(in srgb, var(--chip) var(--chip-tinte), var(--surface));
  }
  .cuenta--cero { opacity: .5; }
  .n { display: block; font-size: 22px; font-weight: 700; color: var(--chip); line-height: 1.1; }
  .etiqueta { display: block; font-size: 11px; color: var(--fg-muted); margin-top: 2px; }
  .glifo { font-family: var(--mono); }
  .veredicto { margin: var(--e-4) 0 var(--e-3); font-size: 13px; color: var(--fg); }
  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tabla th {
    text-align: left; font-size: 11px; font-weight: 600; letter-spacing: .04em;
    text-transform: uppercase; color: var(--fg-faint);
    padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border);
  }
  .tabla td { padding: var(--e-2) var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border); }
  .nombre { font-family: var(--mono); font-size: 12px; }
  .que { color: var(--fg-muted); }
  code { font-family: var(--mono); font-size: 12px; }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
