<script lang="ts">
  // Una faceta NO es el estado. Va en su propia columna y nunca altera el chip
  // principal: una web sin HTTPS no está «rota», y pintarla de rojo junto a una
  // que sí lo está enseña a ignorar el rojo.
  let {
    texto,
    tono = 'normal',
    titulo = '',
  }: { texto: string; tono?: 'normal' | 'aviso' | 'desconocida'; titulo?: string } = $props()
</script>

<span
  class="faceta"
  class:faceta--aviso={tono === 'aviso'}
  class:faceta--desconocida={tono === 'desconocida'}
  title={titulo}
>{texto}</span>
