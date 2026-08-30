<script lang="ts">
  import { PRESENTACION, salud, type Estado } from '../lib/contrato'

  let { estado }: { estado: Estado } = $props()

  const s = $derived(salud(estado))
  const p = $derived(PRESENTACION[s])

  // Lo que se anuncia por voz nunca es un glifo: `—` se lee «raya» y eso no es
  // «no aplica». Los dos estados neutros tienen su palabra aquí.
  const ETIQUETA: Record<string, string> = {
    'no-aplica': 'no aplica',
    desconocido: 'no se sabe',
  }
  const etiqueta = $derived(ETIQUETA[s] ?? p.texto)
</script>

<!--
  Glifo + color + texto, siempre los tres. El color no es nunca el único
  portador, y el `title` lleva la frase completa: el color sin la frase no dice
  qué hacer.

  `aria-label` incluye el texto porque el glifo solo no se lee bien en un lector
  de pantalla — «círculo negro» no es «activo».
-->
<span class="chip chip--{s}" title={p.frase} aria-label="{etiqueta}. {p.frase}">
  <span class="chip__glifo" aria-hidden="true">{p.glifo}</span>
  <!--
    En los dos estados neutros el glifo ES el texto —`—` y `·`—, así que
    pintarlos los dos salía duplicado: la fila decía «— —». Lo vi en una
    captura, no en una prueba: el DOM era correcto y la pantalla, absurda.

    El `aria-label` sí lleva la palabra completa, porque «raya» no le dice nada
    a nadie: lo que hay que anunciar es «no aplica».
  -->
  {#if p.texto !== p.glifo}
    <span class="chip__texto">{p.texto}</span>
  {/if}
</span>
