<script lang="ts">
  import { loDetectado } from '../lib/asistente'

  let { config, app }: { config: Record<string, string>; app: string } = $props()

  const filas = $derived(loDetectado(config))
</script>

<!--
  Lo que Orbit acabó detectando, enseñado DESPUÉS de crear y no antes.

  El informe de diseño lo pedía como paso 2 del formulario, con la conclusión al
  lado de su prueba —«tipo next porque package.json trae "next": "15.1.0"»—, y
  ahí no se puede: la detección lee el repositorio ya clonado, y clonar ocurre
  dentro de `orbit new`. Antes de ejecutar no hay conclusión que enseñar, sólo
  una promesa.

  Aquí sí es un hecho, y sale del contrato: `config` es el descriptor tal cual,
  o sea lo que el servidor escribió. Esta pantalla no interpreta nada.

  Lo que NO se puede enseñar, y por eso no se finge, es el «porqué» de cada
  conclusión: el descriptor guarda el resultado de la detección, no las pruebas
  que la llevaron a él. Enseñar «porque package.json tiene next» sería
  inventarse un motivo verosímil, que es peor que no dar ninguno.
-->
<section class="detectado">
  <p class="titulo">Lo que Orbit ha detectado en «{app}»</p>

  <dl>
    {#each filas as f (f.campo)}
      <div>
        <dt>{f.campo}</dt>
        <dd>
          {#if f.vacio}
            <!-- Vacío A PROPÓSITO, y por eso lleva su etiqueta y no un guion:
                 un guion se lee como «no lo sé», y el descriptor sabe
                 perfectamente que ahí no hay nada. Es la misma regla con la que
                 la portada no pinta un `null` como un cero. -->
            <span class="vacio">está vacío</span>
          {:else}
            <code>{f.valor}</code>
          {/if}
        </dd>
      </div>
    {/each}
  </dl>

  <p class="nota">
    Si algo de esto no es lo que esperabas, la web ya está creada: cambiarlo
    ahora es retirarla con <code>orbit remove {app} -y</code> y volver a crearla
    diciendo el tipo por delante. Si el proyecto se va a desplegar muchas veces,
    sale más a cuenta <code>orbit init</code> en el repositorio: el
    <code>orbit.json</code> manda sobre la detección y viaja con el código.
  </p>
</section>

<style>
  .detectado { padding: var(--e-4); border: 1px solid var(--border); border-radius: var(--r-3); }
  .titulo { margin: 0 0 var(--e-3); font-size: 13px; font-weight: 600; color: var(--fg); }
  dl { display: grid; gap: var(--e-2); margin: 0; }
  dl > div { display: grid; grid-template-columns: 140px 1fr; gap: var(--e-3); align-items: baseline; }
  dt { font-size: 12px; color: var(--fg-faint); }
  dd { margin: 0; font-size: 13px; color: var(--fg); min-width: 0; }
  code { font-family: var(--mono); font-size: 12px; word-break: break-all; }
  .vacio { font-style: italic; color: var(--fg-faint); font-size: 12px; }
  .nota { margin: var(--e-4) 0 0; font-size: 12px; color: var(--fg-muted); max-width: 68ch; }
</style>
