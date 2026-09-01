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
  import Pasada from './componentes/Pasada.svelte'
  import Comparar from './componentes/Comparar.svelte'
  import OrbitJson from './componentes/OrbitJson.svelte'
  import HojaDeComando from './componentes/HojaDeComando.svelte'
  import Desenlace from './componentes/Desenlace.svelte'
  import AsistenteNueva from './componentes/AsistenteNueva.svelte'
  import LoDetectado from './componentes/LoDetectado.svelte'
  import { borradorNuevo, clasificar, PASOS, type Borrador } from './lib/asistente'
  import type { AppInfo, Info } from './lib/contrato'
  import infoMuestra from './lib/muestras/info.json'
  import { leerProgreso } from './lib/despliegue'
  import type { Despliegue as Obj, Lote } from './lib/contrato'
  import loteMuestra from './lib/muestras/deploy-all.json'
  import falloMuestra from './lib/muestras/deploy-fallido.json'
  import okMuestra from './lib/muestras/deploy-ok.json'
  import { leerLog, type Doctor } from './lib/contrato'
  import doctorMuestra from './lib/muestras/doctor.json'
  import logCrudo from './lib/muestras/logs.ndjson?raw'
  import type { App, Estado, Lista } from './lib/contrato'
  import listaHostil from './lib/muestras/list-nombre-hostil.json'
  import listaSana from './lib/muestras/list.json'

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

  const infoBase = (p: Partial<Estado>, rel = ['20260830-120000']): AppInfo => ({
    name: 'tienda', path: '/srv/apps/tienda', config: {},
    state: {
      service: 'active', port: 3001, ssl: true, cert_days: 80,
      maintenance: false, served: true, autodeploy: false, queue: false,
      releases: 1, last_deploy: '20260830-120000', last_deploy_sha: 'abc',
      ...p,
    },
    releases: rel,
  })
  // Un borrador a medio llenar, para que cada paso se pueda mirar con datos
  // dentro. El de la detección va con un ajuste puesto a propósito: es el estado
  // que cuesta ver, porque el normal es que esté vacío.
  const BORRADOR: Borrador = {
    ...borradorNuevo(),
    repo: 'usuario/tienda',
    nombre: 'tienda',
    dominio: 'tienda.ejemplo.com',
    alias: 'www.tienda.ejemplo.com',
    ajustes: {
      ...borradorNuevo().ajustes,
      carpeta: 'apps/web',
      build: { modo: 'vacia' },
    },
  }

  const DNS = {
    coincide: { del_dominio: ['10.0.0.5'], del_servidor: ['10.0.0.5'], coinciden: true },
    no: { del_dominio: ['1.2.3.4'], del_servidor: ['10.0.0.5'], coinciden: false },
    sinSaber: { del_dominio: [], del_servidor: ['10.0.0.5'], coinciden: null },
  }

  // El progreso de una pasada a mitad: una hecha, otra compilando y una tercera
  // que ni ha empezado. Es el estado que hay que mirar, porque el de antes y el
  // de después son fáciles.
  // Dos servidores que se parecen y no son iguales: mismo nombre de app con
  // otro commit, el autodespliegue puesto sólo en uno, y una app de más en cada
  // lado. Es el caso que hay que poder leer de un vistazo.
  // Una app con todo lo que el descriptor puede traer: un monorepo, cosas que
  // la app escribe en marcha, y una especificación de variables que ya existía.
  // Es el caso que hay que poder mirar, porque el mínimo se lee solo.
  const INFO_RICA: AppInfo = {
    name: 'tienda',
    path: '/srv/apps/tienda',
    config: {
      type: 'laravel',
      appdir: 'apps/web',
      build: 'composer install --no-dev && npm ci && npm run build',
      start: '',
      outdir: '',
      docroot: 'public',
      spa: '',
      php: 'yes',
      shared: 'storage/app storage/logs public/uploads',
      env_file: '.env',
      env_spec: [
        'APP_KEY\tgenerate\t32\tsecret\tClave de cifrado de Laravel',
        'DB_PASSWORD\tprompt\tContraseña de la base de datos\tsecret\t',
        'APP_ENV\tprompt\tEntorno (production, staging…)\tplain\t',
      ].join('\n'),
    },
    state: (infoMuestra as Info).app.state,
    releases: ['20260830-120000'],
  }

  const LADO_A: App[] = (listaSana as Lista).apps.slice(0, 3)
  const LADO_B: App[] = [
    { ...LADO_A[0]!, state: { ...LADO_A[0]!.state, last_deploy_sha: 'f00ba7c0ffee', autodeploy: true } },
    { ...LADO_A[1]! },
    { ...LADO_A[2]!, name: 'solo-en-staging', domain: 'pruebas.ejemplo.com' },
  ]

  const CRUDO_PASADA = [
    '{"event":"app","app":"tienda","status":"start","elapsed_s":0}',
    '{"event":"step","app":"tienda","step":"build","status":"start","elapsed_s":4}',
    '{"event":"app","app":"tienda","status":"deployed","elapsed_s":97}',
    '{"event":"app","app":"blog","status":"start","elapsed_s":97}',
    '{"event":"step","app":"blog","step":"code","status":"ok","elapsed_s":2}',
    '{"event":"step","app":"blog","step":"build","status":"start","elapsed_s":3}',
    '{"event":"app","app":"api","status":"unreachable","elapsed_s":98}',
  ].join('\n')

  const DESENLACES = [
    clasificar('tienda', infoBase({}), true),
    clasificar('tienda', infoBase({ ssl: false }), true),
    clasificar('tienda', infoBase({}, []), true),
    clasificar('tienda', infoBase({ service: 'failed' }), true),
    clasificar('tienda', infoBase({ served: false }), true),
    clasificar('tienda', null),
    clasificar('tienda', infoBase({}), false),
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

  <h2>El orbit.json de una app que ya funciona</h2>
  <p class="nota">
    <code>orbit init</code> escribe este fichero <strong>volviendo a
    detectar</strong> sobre el repositorio, o sea haciendo exactamente lo mismo
    que se equivocó la primera vez. Esto no detecta nada: lee el descriptor de
    una app que está desplegada y sirviendo, incluidos los campos que alguien
    arregló a mano.
  </p>
  <p class="nota">
    El bloque <code>env</code> lleva <strong>nombres y no valores</strong>, y no
    por prudencia: el contrato sólo deja pasar los nombres. Y una especificación
    que ya existía se reproduce en vez de aplanarse — donde había
    <code>generate</code> no puede quedar <code>prompt</code>, porque eso cambia
    el significado en silencio.
  </p>
  <div class="panel">
    <OrbitJson
      app="tienda"
      info={INFO_RICA}
      entorno={{ schema: 1, app: 'tienda', keys: ['APP_KEY', 'DB_PASSWORD', 'APP_ENV', 'MAIL_FROM'] }}
    />
  </div>

  <h3>Cuando el descriptor no dice el tipo</h3>
  <p class="nota">
    <strong>Sin <code>type</code>, Orbit ignora el fichero entero</strong> y
    vuelve a detectar como si no existiera. El fichero se ve igual de bien, se
    sube tal cual, y el hueco aparece tres despliegues más tarde — así que se
    dice aquí.
  </p>
  <div class="panel">
    <OrbitJson
      app="tienda"
      info={{ ...INFO_RICA, config: { ...INFO_RICA.config, type: '' } }}
      entorno={null}
    />
  </div>

  <h2>Comparar dos servidores</h2>
  <p class="nota">
    La regla que sostiene esta pantalla es la del cliente multiservidor: la clave
    es <code>servidor:app</code> y <strong>nunca la app sola</strong>, porque
    «tienda» existe en tres servidores y son tres apps distintas. Aquí se ponen
    dos a la misma altura a propósito, así que el alias va escrito en la cabecera
    de cada columna y no se quita nunca — el color es el refuerzo, no la señal.
  </p>
  <div class="panel">
    <Comparar
      a="produccion"
      b="staging"
      appsA={LADO_A}
      appsB={LADO_B}
      candidatos={['staging']}
      alElegir={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h3>Cuando el otro no contesta</h3>
  <p class="nota">
    <strong>Media comparación no es una comparación.</strong> Si se enseñara la
    lista del que sí contestó con los huecos del otro en blanco, todas sus apps
    saldrían como «sólo en produccion» — y eso invita a crearlas otra vez en un
    servidor donde puede que ya existan. Es el mismo error que confundir «al día»
    con «sin contacto», con peores consecuencias.
  </p>
  <div class="panel">
    <Comparar
      a="produccion"
      b="staging"
      appsA={LADO_A}
      appsB={null}
      fallo={{ clase: 'no-llego', mensaje: 'ssh: connect to host staging port 22: Connection timed out' }}
      candidatos={['staging']}
      alElegir={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h2>La pasada por todas las apps</h2>
  <p class="nota">
    <code>deploy --all</code> significa <strong>dos cosas distintas</strong>
    según lleve <code>--if-changed</code> o no: con él le pregunta al remoto de
    cada app y despliega sólo lo que se ha movido; sin él recompila todas, hayan
    cambiado o no. Con cuarenta apps eso son cuarenta builds y cuarenta releases
    nuevas de un código idéntico, así que en la interfaz son
    <strong>dos entradas y nunca una casilla</strong> — la misma decisión que en
    la pantalla de retirar, y por el mismo motivo: la opción que está al lado se
    elige sin leerla.
  </p>
  <h3>Antes</h3>
  <div class="panel">
    <Pasada
      servidor="vps-ovh"
      apps={(listaSana as Lista).apps.slice(0, 12)}
      alLanzar={() => {}}
      alCancelar={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h3>A mitad</h3>
  <p class="nota">
    La lista <strong>es</strong> el progreso, y por eso no hay barra: el servidor
    salta las apps sin repositorio y eso no sale en <code>list</code>, así que
    una fracción sería un denominador inventado. Y «parar» no deshace nada — lo
    dice donde está el botón, no después.
  </p>
  <div class="panel">
    <Pasada
      servidor="vps-ovh"
      apps={(listaSana as Lista).apps.slice(0, 12)}
      crudo={CRUDO_PASADA}
      corriendo={true}
      modo="si-cambia"
      alLanzar={() => {}}
      alCancelar={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h3>Después de preguntar a cada remoto</h3>
  <p class="nota">
    Los seis recuentos, cada uno en su celda. <strong>Agruparlos está
    prohibido</strong>: confundir «al día» con «sin contacto» costó un fallo
    real, un remoto caído anunciado como «nada que hacer» cada cinco minutos
    durante días.
  </p>
  <div class="panel">
    <Pasada
      servidor="vps-ovh"
      apps={(listaSana as Lista).apps.slice(0, 12)}
      resultado={loteMuestra as Lote}
      modo="si-cambia"
      alLanzar={() => {}}
      alCancelar={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h3>Y después de NO preguntar a ninguno</h3>
  <p class="nota">
    Un cero que <strong>no podía ser otra cosa</strong> no es la misma
    información que un cero que sí podía. Sin <code>--if-changed</code> los
    finales baratos son cero por construcción, y dejarlos ahí sin decirlo
    invitaría a leerlos como «he mirado y no había nada».
  </p>
  <div class="panel">
    <Pasada
      servidor="vps-ovh"
      apps={(listaSana as Lista).apps.slice(0, 12)}
      resultado={{ ...(loteMuestra as Lote), total: 4, deployed: 2, failed: 2,
                   unchanged: 0, unreachable: 0, gone: 0, skipped: 0, ok: false,
                   apps: (loteMuestra as Lote).apps.filter(
                     (a) => a.status === 'deployed' || a.status === 'failed') }}
      modo="todo"
      alLanzar={() => {}}
      alCancelar={() => {}}
      alCerrar={() => {}}
    />
  </div>

  <h2>El lote, con sus seis finales</h2>
  <p class="nota">
    Seis y no dos. Confundir «al día» con «no he podido preguntar» costó un fallo
    real: un remoto caído anunciado como «nada que hacer» cada cinco minutos,
    durante días. Agruparlos está prohibido.
  </p>
  <div class="panel"><LoteVista lote={loteMuestra as Lote} servidor="vps-ovh" /></div>

  <h2>El asistente de web nueva</h2>
  <p class="nota">
    Cinco pasos, y el segundo es el que el informe de diseño pedía como
    «enseña lo detectado junto a su prueba». <strong>Ahí no se puede</strong>:
    <code>detect_stack</code> lee un directorio, y el directorio no existe hasta
    que <code>orbit new</code> ha clonado. No hay ninguna orden que mire un
    repositorio remoto y diga qué stack es. Así que ese paso no promete una
    conclusión: ofrece adelantarse a ella, y lo detectado se enseña después, ya
    como hecho.
  </p>
  {#each PASOS as p (p)}
    <h3>{PASOS.indexOf(p) + 1} · {p}</h3>
    <div class="panel">
      <AsistenteNueva
        servidor="vps-ovh"
        pasoInicial={p}
        inicial={BORRADOR}
        correoEnElServidor={false}
        resolucion={p === 'Dominio' ? DNS.no : null}
        alResolver={() => {}}
        alCrear={() => {}}
        alCerrar={() => {}}
      />
    </div>
  {/each}

  <h3>El DNS, en sus tres respuestas</h3>
  <p class="nota">
    «No coincide» y «no lo sé» <strong>no son lo mismo</strong>, y la tercera
    columna es la que se pinta mal en casi todas las interfaces: sin una de las
    dos listas no hay comparación que hacer, y decir «no apunta aquí» sería
    afirmar algo que no se sabe.
  </p>
  {#each Object.entries(DNS) as [cual, r] (cual)}
    <div class="panel">
      <AsistenteNueva
        servidor="vps-ovh"
        pasoInicial="Dominio"
        inicial={BORRADOR}
        resolucion={r}
        alResolver={() => {}}
        alCrear={() => {}}
        alCerrar={() => {}}
      />
    </div>
  {/each}

  <h3>Lo que Orbit acabó detectando</h3>
  <p class="nota">
    Esto sí sale del contrato: <code>config</code> es el descriptor tal cual. Un
    campo vacío lleva su etiqueta y nunca un guion — un guion se lee como «no lo
    sé», y el descriptor sabe perfectamente que ahí no hay nada.
  </p>
  <div class="panel">
    <LoDetectado
      app="tienda"
      config={{ type: 'next', appdir: 'apps/web', build: '', start: 'pnpm start', outdir: '', port: '3007' }}
    />
  </div>

  <h2>Los siete finales de una web nueva</h2>
  <p class="nota">
    <code>orbit new</code> <strong>no tiene <code>--json</code></strong>: sólo
    devuelve prosa y un código de salida. Así que la interfaz no la interpreta —
    le vuelve a preguntar al servidor con <code>orbit info --json</code>, que sí
    tiene contrato. Son 86&nbsp;ms sobre un comando de tres minutos, y es la
    única forma que no depende del idioma del servidor.
  </p>
  <p class="nota">
    Y son siete, no dos. <strong>Cinco son parciales</strong>: algo existe y algo
    falta. Tratar «salió bien» y «falló» como los dos únicos finales deja a
    alguien con una app a medias y sin saberlo.
  </p>
  {#each DESENLACES as d (d.final)}
    <h3>{d.final} · {d.titulo}</h3>
    <Desenlace {d} app="tienda" />
  {/each}

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
