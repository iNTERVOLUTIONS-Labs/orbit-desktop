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

  `role="img"` y no un `aria-label` a secas: en un `<span>` sin rol, ARIA 1.2
  prohíbe el nombre y NVDA en modo exploración no lo lee, así que en los dos
  estados neutros —donde lo único que hay dentro es el glifo, oculto— el chip
  se quedaba mudo. Con el rol, el nombre es lo que se anuncia, y lleva la
  palabra y la frase: «círculo negro» no es «activo», y «raya» no es «no
  aplica».
-->
<span class="chip chip--{s}" role="img" title={p.frase} aria-label="{etiqueta}. {p.frase}">
  <span class="chip__glifo" aria-hidden="true">{p.glifo}</span>
  <!--
    En los dos estados neutros el glifo ES el texto —`—` y `·`—, así que
    pintarlos los dos salía duplicado: la fila decía «— —». Lo vi en una
    captura, no en una prueba: el DOM era correcto y la pantalla, absurda.

    El nombre accesible sí lleva la palabra completa, porque «raya» no le dice
    nada a nadie: lo que hay que anunciar es «no aplica».
  -->
  {#if p.texto !== p.glifo}
    <span class="chip__texto">{p.texto}</span>
  {/if}
</span>
