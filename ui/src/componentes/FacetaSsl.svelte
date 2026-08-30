<script lang="ts">
  import Faceta from './Faceta.svelte'
  import type { Estado } from '../lib/contrato'

  let { estado }: { estado: Estado } = $props()
</script>

<!--
  Tres casos y ninguno se puede colapsar en otro:

    · sin certificado        → aviso neutro, NO rojo. No está rota.
    · cert_days === null     → «·». `list` no lo calcula, y «no lo he mirado»
                               no es «no hay certificado».
    · cert_days negativo     → caducado, y es un dato REAL, no un desbordamiento
-->
{#if !estado.ssl}
  <Faceta texto="sin HTTPS" titulo="No tiene certificado. Se emite con «orbit ssl»." />
{:else if estado.cert_days === null}
  <Faceta texto="·" tono="desconocida" titulo="No se han mirado los días que le quedan al certificado. Los da «orbit info»." />
{:else if estado.cert_days < 0}
  <Faceta texto="caducado" tono="aviso" titulo="El certificado caducó hace {Math.abs(estado.cert_days)} días." />
{:else if estado.cert_days < 10}
  <Faceta texto="caduca en {estado.cert_days} d" tono="aviso" titulo="Renueva con «orbit ssl»." />
{:else}
  <Faceta texto="HTTPS" titulo="Certificado válido {estado.cert_days} días más." />
{/if}
