<script lang="ts">
  import ChipEstado from './componentes/ChipEstado.svelte'
  import FacetaSsl from './componentes/FacetaSsl.svelte'
  import Fallo from './componentes/Fallo.svelte'
  import Esqueleto from './componentes/Esqueleto.svelte'
  import ListaApps from './componentes/ListaApps.svelte'
  import type { App, Estado } from './lib/contrato'
  import listaHostil from './lib/muestras/list-nombre-hostil.json'

  const BASE: Estado = {
    service: null, port: null, ssl: false, cert_days: null,
    maintenance: false, served: true, autodeploy: false, queue: false,
    releases: 1, last_deploy: null, last_deploy_sha: null,
  }
  const e = (p: Partial<Estado>): Estado => ({ ...BASE, ...p })

  const ESTADOS: Array<[string, Estado]> = [
    ['activo', e({ service: 'active' })],
    ['parado', e({ service: 'stopped' })],
    ['no aplica (estática)', e({ service: null })],
    ['mantenimiento', e({ service: 'active', maintenance: true })],
    ['sin vhost', e({ served: false })],
    ['desconocido', e({ service: 'activating' })],
  ]

  const SSL: Array<[string, Estado]> = [
    ['sin certificado', e({ ssl: false })],
    ['no se ha mirado', e({ ssl: true, cert_days: null })],
    ['caducado', e({ ssl: true, cert_days: -5 })],
    ['caduca pronto', e({ ssl: true, cert_days: 6 })],
    ['válido', e({ ssl: true, cert_days: 74 })],
  ]

  const FALLOS = [
    {
      titulo: 'La clave del host ha cambiado',
      error: {
        clase: 'clave-de-host-cambiada',
        mensaje: 'la clave de este servidor ha cambiado',
        detalle:
          'The fingerprint for the ED25519 key sent by the remote host is\n' +
          'SHA256:pws/4rfCdfT+hvZDfEQqw74EPGjsVR/skGpH1RUHDTA.\n' +
          'Offending ED25519 key in ~/.ssh/known_hosts:14',
      },
    },
    {
      titulo: 'Orbit no está donde se esperaba',
      error: {
        clase: 'orbit-no-esta',
        mensaje: 'no hay un orbit ejecutable en /usr/local/bin/orbit',
        detalle: 'bash: orbit: command not found',
      },
    },
    {
      titulo: 'sudo pide contraseña',
      error: {
        clase: 'sudo-pide-clave',
        mensaje: 'ese usuario necesita contraseña para sudo, y aquí no hay terminal donde escribirla',
        detalle: null,
      },
    },
    {
      titulo: 'Basura antes del objeto',
      error: {
        clase: 'respuesta-sucia',
        mensaje: 'por stdout no ha venido un solo objeto JSON: hay algo antes del objeto',
        detalle: 'Last login: Fri Aug 29 14:02:11 2026\n{"schema":1,"apps":[]}',
      },
    },
  ]
</script>

<main>
  <h1>Galería</h1>
  <p class="intro">
    Los estados difíciles, que son el grueso del producto. Esta página no forma
    parte de la aplicación: es una entrada aparte, para poder mirar cada estado
    sin tener que provocarlo. El estado feliz se diseña solo; los demás, no.
  </p>

  <h2>Estados de una app</h2>
  <p class="nota">
    Glifo, color y texto, siempre los tres. El color nunca es el único portador:
    el 8&nbsp;% de los hombres tiene alguna deficiencia de visión del color, las
    capturas que se pegan en un issue a veces son en gris, y esta lista se mira
    de reojo, donde la forma sobrevive y el color no.
  </p>
  <div class="rejilla">
    {#each ESTADOS as [nombre, est] (nombre)}
      <div class="celda"><ChipEstado estado={est} /><span class="etiqueta">{nombre}</span></div>
    {/each}
  </div>

  <h2>El certificado, que no es el estado</h2>
  <p class="nota">
    Va en su propia columna y nunca altera el chip principal. Una web sin HTTPS
    no está «rota», y pintarla de rojo junto a una que sí lo está enseña a
    ignorar el rojo.
  </p>
  <div class="rejilla">
    {#each SSL as [nombre, est] (nombre)}
      <div class="celda"><FacetaSsl estado={est} /><span class="etiqueta">{nombre}</span></div>
    {/each}
  </div>

  <h2>Nombres que llegan del servidor</h2>
  <p class="nota">
    Ninguno se puede operar, y los caracteres que engañan al ojo se marcan con
    su punto de código. No se borran: borrarlos sería «arreglar» el nombre, y un
    nombre arreglado ya no identifica a nadie.
  </p>
  <div class="panel">
    <ListaApps apps={listaHostil.apps as App[]} servidor="comprometido" />
  </div>

  <h2>Cargando</h2>
  <p class="nota">
    Un esqueleto y no un spinner: un spinner dice «espera» y no dice cuánto; el
    esqueleto dice <em>qué</em> va a aparecer.
  </p>
  <div class="panel"><Esqueleto filas={4} /></div>

  <h2>Fallos</h2>
  <p class="nota">
    Cada clase de fallo dice qué hacer. «Error al conectar» no le dice a nadie si
    es su clave, su red o su servidor.
  </p>
  {#each FALLOS as f (f.titulo)}
    <h3>{f.titulo}</h3>
    <Fallo error={f.error} alias="vps-ovh" />
  {/each}
</main>

<style>
  main { max-width: 900px; margin: 0 auto; padding: var(--e-6) var(--e-5) var(--e-7); }
  h1 { font-size: 22px; margin: 0 0 var(--e-3); color: var(--fg); }
  h2 { font-size: 15px; margin: var(--e-7) 0 var(--e-2); color: var(--fg); }
  h3 { font-size: 13px; margin: var(--e-5) 0 var(--e-2); color: var(--fg-muted); font-weight: 600; }
  .intro, .nota { color: var(--fg-muted); font-size: 13px; margin: 0 0 var(--e-4); max-width: 70ch; line-height: 1.6; }
  .rejilla { display: flex; flex-wrap: wrap; gap: var(--e-5); align-items: center; }
  .celda { display: flex; flex-direction: column; gap: var(--e-2); align-items: flex-start; }
  .etiqueta { font-size: 11px; color: var(--fg-faint); }
  .panel {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--r-3); padding: var(--e-4);
  }
</style>
