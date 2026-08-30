<script lang="ts">
  import ChipEstado from './componentes/ChipEstado.svelte'
  import FacetaSsl from './componentes/FacetaSsl.svelte'
  import Fallo from './componentes/Fallo.svelte'
  import Esqueleto from './componentes/Esqueleto.svelte'
  import ListaApps from './componentes/ListaApps.svelte'
  import Diagnostico from './componentes/Diagnostico.svelte'
  import VisorLog from './componentes/VisorLog.svelte'
  import Despliegue from './componentes/Despliegue.svelte'
  import LoteVista from './componentes/Lote.svelte'
  import HojaDeComando from './componentes/HojaDeComando.svelte'
  import { leerProgreso } from './lib/despliegue'
  import type { Despliegue as Obj, Lote } from './lib/contrato'
  import loteMuestra from './lib/muestras/deploy-all.json'
  import falloMuestra from './lib/muestras/deploy-fallido.json'
  import okMuestra from './lib/muestras/deploy-ok.json'
  import { leerLog, type Doctor } from './lib/contrato'
  import doctorMuestra from './lib/muestras/doctor.json'
  import logCrudo from './lib/muestras/logs.ndjson?raw'
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

  const LOG = leerLog(logCrudo)
  // El mismo diagnóstico, con y sin la capacidad de arreglar. Es la diferencia
  // que el PR a Orbit desbloqueó, y verla lado a lado es lo que dice si el «sin
  // botón» sigue siendo útil.
  const DIAG = doctorMuestra as Doctor

  const PROG = leerProgreso([
    '{"event":"step","app":"tienda","step":"code","status":"start","elapsed_s":0}',
    '{"event":"step","app":"tienda","step":"code","status":"ok","elapsed_s":3}',
    '{"event":"step","app":"tienda","step":"release","status":"start","elapsed_s":3}',
    '{"event":"step","app":"tienda","step":"release","status":"ok","elapsed_s":5}',
    '{"event":"step","app":"tienda","step":"build","status":"start","elapsed_s":5}',
    '{"event":"step","app":"tienda","step":"build","status":"ok","elapsed_s":103}',
    '{"event":"step","app":"tienda","step":"activate","status":"start","elapsed_s":103}',
  ].join('\n'))
  const OK = okMuestra as Obj
  const RECUPERADO: Obj = { ...OK, recovered: true, duration_s: 214 }
  const REVERTIDO: Obj = {
    ...OK, ok: false, rolled_back: true, release: null,
    failed_step: 'service', duration_s: 64,
    error: 'la app no responde al health check en 30 s',
  }
  const ROTO = falloMuestra as Obj

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

  <h2>El diagnóstico</h2>
  <p class="nota">
    El botón sólo aparece donde <code>fixable</code> lo permite. Uno que no hace
    nada es peor que ninguno: invita a averiguar por qué no se puede pulsar, y
    la respuesta es una frase que se podía haber leído sin el botón.
  </p>
  <div class="panel">
    <Diagnostico doctor={DIAG} servidor="vps-ovh" alArreglar={() => {}} />
  </div>
  <p class="nota">
    Y contra un servidor que no puede aplicarlo sin terminal: en vez de un botón
    gris, la orden para copiar.
  </p>
  <div class="panel">
    <Diagnostico doctor={DIAG} servidor="vps-ovh" alArreglar={null} />
  </div>

  <h2>El log</h2>
  <p class="nota">
    Se puede separar el log de acceso del de error. Es la primera pregunta de
    cualquiera que mira un log de nginx, y la salida en prosa no la contesta
    porque <code>tail</code> mezcla los dos ficheros sin decir cuál es cuál.
  </p>
  <div class="panel">
    <VisorLog log={LOG} app="app001" />
  </div>

  <h2>El despliegue, en marcha</h2>
  <p class="nota">
    La barra está ponderada: seis pasos no valen un sexto cada uno, y el build
    es el 70-85&nbsp;% del tiempo. Y es monótona creciente — una barra que
    retrocede destruye más confianza que cualquier error.
  </p>
  <div class="panel"><Despliegue app="tienda" servidor="vps-ovh" progreso={PROG} /></div>

  <h2>Los cuatro finales</h2>
  <p class="nota">
    El objeto no tiene un campo «resultado» con cuatro valores: tiene
    <code>ok</code>, <code>rolled_back</code> y <code>recovered</code>, y su
    combinación da cuatro finales que se ven distintos porque lo son.
  </p>
  {#each [['Bien', OK], ['Al segundo intento', RECUPERADO], ['Falló y volvió atrás', REVERTIDO], ['Falló', ROTO]] as [titulo, r] (titulo)}
    <h3>{titulo}</h3>
    <div class="panel">
      <Despliegue app="tienda" servidor="vps-ovh" progreso={leerProgreso('')} resultado={r as Obj} />
    </div>
  {/each}

  <h2>El lote, con sus seis finales</h2>
  <p class="nota">
    Seis y no dos. Confundir «al día» con «no he podido preguntar» costó un fallo
    real: un remoto caído anunciado como «nada que hacer» cada cinco minutos,
    durante días. Agruparlos está prohibido.
  </p>
  <div class="panel"><LoteVista lote={loteMuestra as Lote} servidor="vps-ovh" /></div>

  <h2>La hoja de comando</h2>
  <p class="nota">
    Antes de cambiar nada, la orden literal. Existe por tres motivos y ninguno es
    estético: es la <strong>prueba visible</strong> de que esto sólo invoca
    <code>orbit</code> —la promesa que lo deja existir—; enseña la CLI mientras se
    usa el ratón; y convierte «¿qué va a pasar?» en algo que se lee.
  </p>
  <div class="panel hoja-demo">
    <HojaDeComando
      titulo="Volver atrás"
      servidor="vps-ovh"
      orden="orbit rollback tienda 20260805-041230"
      consecuencia="Reinicia el servicio y recarga nginx. La web deja de responder entre uno y dos segundos. Y si la app tiene el autodespliegue puesto, el siguiente ciclo del temporizador volverá a poner la versión de la que estás saliendo."
      verbo="Volver atrás"
      alConfirmar={() => {}}
      alCancelar={() => {}}
    />
  </div>
  <p class="nota">
    Y cuando el daño es irreversible, hay que escribir el nombre. La fricción es
    un recurso escaso: se gasta donde no hay vuelta atrás y en ningún otro sitio,
    porque pedirla para todo enseña a teclear sin leer.
  </p>
  <div class="panel hoja-demo">
    <HojaDeComando
      titulo="Retirar y borrar los datos"
      servidor="vps-ovh"
      orden="orbit remove tienda -y --purge"
      consecuencia="Quita la app de nginx y de systemd —eso se deshace— y BORRA /srv/apps/tienda: el .env, todas las releases y las subidas de tus usuarios. Eso no vuelve."
      verbo="Borrar"
      peligrosa={true}
      confirmarEscribiendo="tienda"
      alConfirmar={() => {}}
      alCancelar={() => {}}
    />
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
  /* La hoja es un modal a pantalla completa: en la galería se contiene para
     poder verla junto a lo demás. */
  .hoja-demo { position: relative; min-height: 380px; overflow: hidden; }
  .hoja-demo :global(.velo) { position: absolute; }
</style>
