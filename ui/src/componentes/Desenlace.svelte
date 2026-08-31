<script lang="ts">
  import type { Desenlace } from '../lib/asistente'

  let { d, app }: { d: Desenlace; app: string } = $props()
</script>

<!--
  Cada final dice tres cosas: qué existe, qué falta y qué se deshace. Y en los
  parciales **«qué existe» va primero**, porque es lo que quien mira no se
  espera: alguien que ve «ha fallado» asume que no hay nada, y lo que hay es una
  app registrada con su dominio y su configuración.
-->
<section class="desenlace desenlace--{d.tono}" role="status">
  <p class="titulo">{d.titulo}</p>

  <dl>
    <div><dt>Qué existe</dt><dd>{d.existe}</dd></div>
    {#if d.falta}
      <div><dt>Qué falta</dt><dd>{d.falta}</dd></div>
    {/if}
    <div><dt>Qué se deshace</dt><dd>{d.deshacer}</dd></div>
  </dl>

  {#if d.accion}
    <div class="accion">
      <button type="button" class="primaria">{d.accion.texto}</button>
      <!-- La orden, siempre a la vista: es la prueba de que esto sólo invoca
           `orbit`, también aquí. -->
      <code>{d.accion.orden}</code>
    </div>
  {/if}

  {#if d.final === 'F2'}
    <!-- Un efecto secundario que hay que enseñar ANTES y no después: el correo
         se guarda en la configuración global del servidor, no en la de la app.
         Decirlo cuando ocurre evita el «¿por qué ya no me lo pregunta?» de
         dentro de tres meses. -->
    <p class="nota">
      El correo que pide se guarda en la configuración del servidor, no en la de
      «{app}»: la próxima web que crees aquí ya no te lo preguntará.
    </p>
  {/if}
</section>

<style>
  .desenlace {
    padding: var(--e-4); border-radius: var(--r-3);
    border: 1px solid var(--border);
  }
  .titulo { margin: 0 0 var(--e-3); font-size: 15px; font-weight: 600; color: var(--desenlace, var(--fg)); }
  dl { display: grid; gap: var(--e-3); margin: 0; }
  dl > div { display: grid; grid-template-columns: 140px 1fr; gap: var(--e-3); align-items: baseline; }
  dt { font-size: 12px; color: var(--fg-faint); }
  dd { margin: 0; font-size: 13px; color: var(--fg); max-width: 68ch; }
  .accion { display: flex; align-items: center; gap: var(--e-3); margin-top: var(--e-4); flex-wrap: wrap; }
  .primaria {
    background: var(--accent-fill); color: var(--on-accent); border: 0;
    border-radius: var(--r-2); padding: var(--e-2) var(--e-3);
    font: inherit; font-size: 13px; font-weight: 600; cursor: pointer;
  }
  .primaria:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  code { font-family: var(--mono); font-size: 12px; color: var(--fg-muted); }
  .nota { margin: var(--e-4) 0 0; font-size: 12px; color: var(--fg-muted); max-width: 68ch; }
</style>
