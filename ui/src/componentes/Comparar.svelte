<script lang="ts">
  import { comparar, sha } from '../lib/comparar'
  import type { App } from '../lib/contrato'
  import type { ErrorDelPuente } from '../lib/puente'

  let {
    a,
    b,
    appsA,
    appsB = null,
    fallo = null,
    cargando = false,
    candidatos,
    alElegir,
    alCerrar,
  }: {
    /** El servidor desde el que se abrió. Sus apps ya están cargadas. */
    a: string
    /** Contra cuál se compara. `null` mientras no se ha elegido. */
    b: string | null
    appsA: App[]
    /** `null` es **no lo he preguntado o no contestó**, y no es una lista
     *  vacía: una lista vacía sería «no tiene ninguna app». */
    appsB?: App[] | null
    fallo?: ErrorDelPuente | null
    cargando?: boolean
    candidatos: string[]
    alElegir: (alias: string) => void
    alCerrar: () => void
  } = $props()

  // Sólo cuando hay las dos listas. Media comparación no es una comparación.
  const c = $derived(appsB === null ? null : comparar(appsA, appsB))
</script>

<section class="comparar">
  <header>
    <h3>Comparar {a} con {b ?? '…'}</h3>
    <button type="button" class="cerrar" onclick={alCerrar}>Cerrar</button>
  </header>

  {#if b === null}
    <p class="intro">
      Dos servidores, uno al lado del otro: qué hay en uno que no esté en el
      otro, y en qué se diferencian las que están en los dos.
    </p>
    {#if candidatos.length === 0}
      <p class="intro">
        Sólo hay un servidor en tu <code>~/.ssh/config</code>. Con uno no hay
        nada que comparar.
      </p>
    {:else}
      <ul class="elegir">
        {#each candidatos as x (x)}
          <li><button type="button" onclick={() => alElegir(x)}>{x}</button></li>
        {/each}
      </ul>
      <p class="ayuda">
        Se le pide la lista de apps a cada uno. Es una lectura: esta pantalla no
        cambia nada en ninguno de los dos.
      </p>
    {/if}
  {:else if cargando}
    <p class="tenue">preguntando a {b}…</p>
  {:else if fallo}
    <!--
      Media comparación no es una comparación, y esto es el corazón de la
      pantalla. Si se enseñara la lista de «a» con los huecos de «b» en blanco,
      todas sus apps saldrían como «sólo en {a}» — y eso invita a crearlas otra
      vez en un servidor donde puede que ya existan. Es el mismo error que
      confundir «al día» con «sin contacto», con peores consecuencias.
    -->
    <div class="sin-comparar" role="status">
      <p class="titulo">No he podido comparar</p>
      <p>
        <strong>{b}</strong> no ha contestado, así que no sé lo que tiene.
        <strong>Eso no quiere decir que no tenga nada</strong>, y por eso no
        enseño la lista de {a} con los huecos en blanco: saldría entera como
        «sólo en {a}», que sería falso.
      </p>
      <p class="motivo">{fallo.mensaje}</p>
    </div>
  {:else if c}
    <p class="resumen" role="status">
      {c.enLosDos.length}
      {c.enLosDos.length === 1 ? 'app en los dos' : 'apps en los dos'}{#if c.conDiferencias > 0},
        de las que <strong>{c.conDiferencias}</strong>
        {c.conDiferencias === 1 ? 'tiene diferencias' : 'tienen diferencias'}{/if}.
      {c.soloA.length} sólo en {a}, {c.soloB.length} sólo en {b}.
    </p>

    {#if c.conDiferencias > 0}
      <h4>Están en los dos, y no están igual</h4>
      <!--
        Cada celda lleva SIEMPRE el nombre del servidor encima, también cuando
        es obvio. La regla del cliente multiservidor es que la clave es
        `servidor:app` y nunca la app sola, y esta pantalla es justo la que pone
        dos servidores a la misma altura: si algún día alguien recorta esta
        cabecera «porque ya se sabe cuál es cuál», la pantalla pasa a ser una
        forma cómoda de equivocarse de máquina.
      -->
      <table class="tabla">
        <thead>
          <tr>
            <th scope="col">App</th>
            <th scope="col">Qué</th>
            <th scope="col" class="lado lado--a">{a}</th>
            <th scope="col" class="lado lado--b">{b}</th>
          </tr>
        </thead>
        <tbody>
          {#each c.enLosDos.filter((f) => f.diferencias.length > 0) as f (f.app)}
            {#each f.diferencias as d, i (d.campo)}
              <tr class:primera={i === 0}>
                <td class="nombre">{i === 0 ? f.app : ''}</td>
                <td class="campo" title={d.porque}>{d.etiqueta}</td>
                <td class="valor lado--a">
                  {d.campo === 'last_deploy_sha'
                    ? sha(f.a?.state.last_deploy_sha ?? null)
                    : d.a}
                </td>
                <td class="valor lado--b">
                  {d.campo === 'last_deploy_sha'
                    ? sha(f.b?.state.last_deploy_sha ?? null)
                    : d.b}
                </td>
              </tr>
            {/each}
            {#if f.nombreIgualDominioDistinto}
              <!--
                Se enseña la duda, no una conclusión. Que dos apps se llamen
                igual no las hace la misma app: puede ser el mismo proyecto en
                dos entornos, o dos proyectos distintos que coinciden de nombre.
                Quien mira lo sabe; esta pantalla no.
              -->
              <tr class="duda">
                <td></td>
                <td colspan="3">
                  Se llaman igual y sirven dominios distintos. Puede ser el mismo
                  proyecto en dos entornos, o dos proyectos que coinciden de
                  nombre: eso lo sabes tú, no yo.
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}

    <div class="listas">
      <section>
        <h4>Sólo en <span class="lado lado--a">{a}</span></h4>
        {#if c.soloA.length === 0}
          <p class="tenue">ninguna</p>
        {:else}
          <ul class="solo">
            {#each c.soloA as f (f.app)}
              <li><span class="nombre">{f.app}</span><span class="dom">{f.a?.domain}</span></li>
            {/each}
          </ul>
        {/if}
      </section>
      <section>
        <h4>Sólo en <span class="lado lado--b">{b}</span></h4>
        {#if c.soloB.length === 0}
          <p class="tenue">ninguna</p>
        {:else}
          <ul class="solo">
            {#each c.soloB as f (f.app)}
              <li><span class="nombre">{f.app}</span><span class="dom">{f.b?.domain}</span></li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>

    {#if c.conDiferencias === 0 && c.enLosDos.length > 0}
      <p class="tenue">
        Las que están en los dos están igual en todo lo que se compara.
      </p>
    {/if}

    <!--
      Lo que esta pantalla NO puede decir, dicho por ella misma. `list --json` no
      trae la rama ni el repositorio: dos apps en el mismo commit pueden venir de
      repositorios distintos, y esta comparación no lo vería. Callarlo dejaría a
      alguien creyendo que «sin diferencias» quiere decir «idénticas».
    -->
    <p class="ayuda">
      Se compara lo que dice <code>list</code>: el tipo, los dominios, el commit
      desplegado, el vhost, el certificado y el autodespliegue. <strong>La rama y
      el repositorio no salen ahí</strong>, así que dos apps sin diferencias
      podrían seguir viniendo de repositorios distintos. Y lo que cambia solo —el
      proceso, el puerto, el número de releases— no se compara a propósito: casi
      siempre difiere y no dice nada.
    </p>
  {/if}
</section>

<style>
  .comparar { display: grid; gap: var(--e-4); max-width: 92ch; }
  header { display: flex; align-items: baseline; justify-content: space-between; gap: var(--e-3); }
  h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--fg); }
  h4 { margin: 0 0 var(--e-2); font-size: 12px; font-weight: 600; color: var(--fg-muted); }
  .cerrar {
    background: none; border: 0; padding: 0; font: inherit; font-size: 12px;
    color: var(--fg-muted); cursor: pointer; text-decoration: underline;
  }

  .intro, .resumen { margin: 0; font-size: 13px; color: var(--fg); max-width: 76ch; }
  .ayuda { margin: 0; font-size: 12px; color: var(--fg-muted); max-width: 80ch; }
  .tenue { margin: 0; font-size: 12px; color: var(--fg-faint); }

  .elegir { list-style: none; margin: 0; padding: 0; display: flex; gap: var(--e-2); flex-wrap: wrap; }
  .elegir button {
    background: none; border: 1px solid var(--border-strong); border-radius: var(--r-1);
    padding: var(--e-2) var(--e-3); font: inherit; font-size: 13px;
    color: var(--fg); cursor: pointer;
  }
  .elegir button:hover { border-color: var(--accent); }
  .elegir button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }

  .sin-comparar { display: grid; gap: var(--e-2); }
  .sin-comparar p { margin: 0; font-size: 13px; color: var(--fg); max-width: 76ch; }
  .sin-comparar .titulo { font-weight: 600; }
  .motivo { font-family: var(--mono); font-size: 12px; color: var(--fg-faint) !important; }

  .tabla { width: 100%; border-collapse: collapse; font-size: 13px; }
  th {
    text-align: left; font-size: 11px; font-weight: 600; text-transform: uppercase;
    letter-spacing: .04em; color: var(--fg-faint);
    padding: 0 var(--e-3) var(--e-2) 0; border-bottom: 1px solid var(--border);
  }
  td { padding: var(--e-1) var(--e-3) var(--e-1) 0; color: var(--fg); vertical-align: baseline; }
  tr.primera td { padding-top: var(--e-3); border-top: 1px solid var(--border); }
  .nombre { font-weight: 600; }
  .campo { color: var(--fg-muted); font-size: 12px; }
  .valor { font-family: var(--mono); font-size: 12px; }
  .duda td { font-size: 12px; color: var(--fg-muted); padding-bottom: var(--e-2); }

  /* El lado se marca por color y además por posición y por nombre: la cabecera
     lleva el alias escrito en todas las columnas, siempre. */
  .lado--a { color: var(--comparado-a, var(--fg)); }
  .lado--b { color: var(--comparado-b, var(--fg)); }

  .listas { display: grid; grid-template-columns: 1fr 1fr; gap: var(--e-5); }
  .solo { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; font-size: 13px; }
  .solo li {
    display: grid; grid-template-columns: 1fr auto; gap: var(--e-3);
    align-items: baseline; padding: var(--e-1) 0; border-bottom: 1px solid var(--border);
  }
  .dom { font-family: var(--mono); font-size: 11px; color: var(--fg-faint); }
  code { font-family: var(--mono); font-size: 12px; }
</style>
