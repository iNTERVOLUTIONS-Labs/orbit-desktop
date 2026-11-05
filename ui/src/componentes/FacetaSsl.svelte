<script lang="ts">
  import Faceta from './Faceta.svelte'
  import type { Estado } from '../lib/contrato'

  let { estado }: { estado: Estado } = $props()

  // El certificado de origen de Cloudflare (1.4.7) sólo lo acepta Cloudflare:
  // quien entre a la IP del servidor sin pasar por su proxy ve un error de
  // certificado. No es un aviso —en Full (strict) es lo correcto— pero sí es
  // algo que hay que poder ver sin abrir el detalle, porque cambia qué pasa el
  // día que alguien apague el proxy. `undefined` (un servidor anterior) y
  // `letsencrypt` se pintan igual que siempre.
  const origen = $derived(estado.cert_type === 'origin')
  const de = $derived(origen ? 'Certificado de origen de Cloudflare: sólo vale detrás de su proxy. ' : '')
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
  {#if origen}
    <Faceta texto="origen CF" titulo="{de}No se han mirado los días que le quedan. Los da «orbit info»." />
  {:else}
    <Faceta texto="·" tono="desconocida" titulo="No se han mirado los días que le quedan al certificado. Los da «orbit info»." />
  {/if}
{:else if estado.cert_days < 0}
  <Faceta texto="caducado" tono="aviso" titulo="{de}El certificado caducó hace {Math.abs(estado.cert_days)} días." />
{:else if estado.cert_days < 10}
  <Faceta
    texto="caduca en {estado.cert_days} d"
    tono="aviso"
    titulo={origen ? `${de}No lo renueva certbot: pide otro con «orbit ssl --origin».` : 'Renueva con «orbit ssl».'}
  />
{:else}
  <Faceta texto={origen ? 'HTTPS · origen CF' : 'HTTPS'} titulo="{de}Certificado válido {estado.cert_days} días más." />
{/if}
