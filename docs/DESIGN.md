# Sistema de diseño y experiencia de uso

> Cómo se ve y cómo se comporta Orbit Desktop: quién lo usa y para qué, el mapa
> de pantallas, la paleta y sus tokens, los estados difíciles —que son el grueso
> del producto—, el movimiento, el despliegue en vivo y la accesibilidad.
>
> La paleta no está inventada: sale de los SVG de la marca de Orbit
> (`assets/banner.svg`, `logo.svg`, `logo-mark.svg`, `favicon.svg`), y los
> contrastes están **calculados** con la fórmula de luminancia de WCAG, no
> estimados.
>
> Las decisiones que lo enmarcan están en **[ARCHITECTURE.md](ARCHITECTURE.md)**;
> los datos que alimentan cada pantalla, en **[CONTRACT.md](CONTRACT.md)**; las
> latencias con las que se calculan los umbrales, en **[CLIENT.md](CLIENT.md)**;
> y lo que no se puede pintar en pantalla, en
> **[THREAT-MODEL.md](THREAT-MODEL.md)**.

---


> Documento de la especialidad de interfaz. Fija el usuario, la arquitectura de
> información, el sistema de diseño en tokens, el comportamiento de los estados que no son
> el estado feliz, el movimiento, la accesibilidad y la recomendación de stack visual. No
> propone arquitectura de proceso, ni modelo de amenazas, ni plan de QA: esos van en sus
> documentos y los escribe quien corresponde.
>
> **Ronda 2.** Reescrito contra tres cosas que en la ronda 1 no existían: cinco hallazgos
> **medidos** ejecutando Orbit de verdad, la auditoría del contrato del arquitecto —que
> corrige varias formas que yo daba por buenas— y el modelo de amenazas de QA. Lo que ha
> cambiado, sección por sección: **§1.4-1.5** latencias reales y el hallazgo que retira
> 306 ms de la portada; **§2.6** el asistente de web nueva rehecho con sus siete finales;
> **§2.8** el diagnóstico, que pierde un botón que invocaba un comando inexistente;
> **§2.9** multiservidor operativo, contestado; **§2.10** `exec`; **§3.8** la tabla decidida
> con cuarenta aplicaciones de verdad; **§4.1** los umbrales de carga recalculados;
> **§4.11-4.12**; **§6.8** notificaciones del sistema; **§7.8-7.9** el diálogo destructivo y
> los anuncios en vivo; **§8** el stack; y **§9**, la puntuación nueva.
> Lo que **no** ha cambiado es lo que §9.4 declara fijado: la paleta, la tabla de estados,
> los glifos neutros y las cuatro prohibiciones.
>
> **Ronda 3.** Sólo **§8**, y sólo porque el arquitecto y yo cedimos en direcciones
> cruzadas —yo retiré Svelte y acepté React, él retiró React y aceptó Svelte— y eso deja dos
> documentos que se contradicen. La coordinación arbitra **Svelte 5 con CSS de tokens**, por
> un criterio que no es el rendimiento y que §8.1 recoge entero. Lo que se rehace de verdad
> es **§8.4**, porque Recharts era de React: vuelven las cuatro gráficas a SVG propio, y el
> riesgo de calendario que yo había retirado a medias vuelve y se dice. **§9** ajusta las
> notas: UX se queda en 87 con dos movimientos que se anulan, y usabilidad baja a 88.

---

## 0. La premisa que condiciona todo lo demás

Orbit Desktop no tiene API. Tiene un contrato de línea de comandos que viaja por SSH,
y **cada pantalla de esta aplicación es literalmente un `orbit … --json` ejecutado en
un servidor que está a unas decenas de milisegundos de red y a entre 72 milisegundos y
varios segundos de trabajo — medido: 72 ms el comando más barato, 389 ms la portada con
cuarenta aplicaciones, segundos el diagnóstico y el tráfico**. Eso no es un detalle de implementación que la interfaz pueda esconder: es la
materia prima con la que se diseña. ARCHITECTURE §13.6d lo dice sin adornos —«la
latencia del contrato *es* la de la interfaz, sumada a la de la red y no escondida
detrás de ella»— y de ahí sale la mitad de las decisiones de este documento.

La segunda premisa es de vocabulario, y es la que más se nota al usar la aplicación.
El contrato de Orbit se tomó la molestia de distinguir **«no aplica» de «está caído»**:
el `service` de una web estática es `null` y no `"stopped"`, el puerto es `null` y no
`0`, la CPU de la primera lectura es `null` y no `0`. La frase que lo justifica está
en §13.1 y merece quedar aquí porque es, tal cual, un requisito de diseño visual:

> «Confundir "no aplica" con "está caída" pinta una alarma roja donde no pasa nada, y
> eso enseña a la gente a ignorar las alarmas.»

Una interfaz que pinte un punto rojo en la columna ESTADO de una web estática ha tirado
por la borda un trabajo que ya estaba hecho en el servidor. Así que este sistema de
diseño tiene, antes que una paleta, una **tabla de estados con cinco valores y dos
glifos neutros distintos**, y se comprueba antes que ninguna otra cosa.

Y la tercera: **la interfaz nunca escribe en `/etc/nginx`, `/etc/orbit` ni systemd.
Solo invoca `orbit`.** Esto, que parece una restricción de backend, tiene una
consecuencia visual enorme y buena: **toda acción de esta aplicación se puede expresar
como una orden**. La aprovecho a fondo en §2.4 con la *hoja de comando*, que es la
pieza de confianza del producto y algo que un panel web de los que Orbit rechaza no
podría enseñar nunca.

---

## 1. El usuario y el trabajo real

### 1.1 Quién es

Una persona que desarrolla y publica webs pequeñas, sola o en un estudio de dos o tres.
Tiene un VPS de entre 5 y 20 euros al mes con más recursos de los que usa, y entre una
y cuarenta aplicaciones encima: la web de su estudio, tres o cuatro landings de
clientes, un par de Next, un Laravel de una tienda pequeña, dos Astro estáticos, el
staging de alguno de ellos y un par de cosas que puso hace año y medio y no vuelve a
tocar hasta que se caen.

No tiene guardia. No tiene equipo de operaciones. No mira Grafana. **No quiere aprender
otra herramienta**: si Orbit Desktop le exige un modelo mental nuevo, vuelve al
terminal, donde ya sabe moverse, y esta aplicación pierde su única razón de existir,
que es ser más rápida que escribir `ssh mi-vps orbit list`.

Es importante lo que **no** es: no es un administrador de sistemas que gestiona
infraestructura ajena, no es un equipo de plataforma, y no es alguien que vaya a vivir
con esta aplicación abierta ocho horas. La abre, mira, actúa y la cierra.

### 1.2 Los dos modos de uso, que son dos productos distintos

Hay que separarlos desde el primer boceto, porque tiran en direcciones opuestas.

**Modo rutina.** El 95 % de las aperturas. Dura entre veinte segundos y dos minutos. La
pregunta es siempre la misma —*¿está todo bien?*— y la respuesta deseada es *sí*. A
veces va seguida de un despliegue que se espera que salga bien. El usuario está
tranquilo, tiene otra ventana delante, y esto es una interrupción que él mismo se ha
provocado. Lo que necesita: **contestar en un vistazo y desaparecer**.

**Modo incidente.** El 5 % restante. Empieza con una llamada, un mensaje de un cliente
o un correo del vigilante. Dura entre diez y cuarenta minutos, con el pulso alto y
tomando decisiones con consecuencias. Lo que necesita: **llegar del síntoma a la causa
en tres clics y poder deshacer**.

De aquí sale la decisión de arquitectura de información más importante del documento, y
la escribo antes que el mapa de pantallas porque el mapa se deriva de ella:

> **La portada tiene que contestar el modo rutina en menos de un segundo de lectura
> humana, y ser a la vez la puerta más corta al modo incidente.** No es un panel de
> métricas agregadas. Es la lista de aplicaciones con su estado, ordenada de forma que
> lo que va mal está arriba.

Un *dashboard* con tarjetas de «12 apps · 3 despliegues hoy · 41 % de disco» se
descarta explícitamente. Con tres aplicaciones es ruido decorativo; con cuarenta es un
muro que hay que leer entero para saber que no pasa nada; y ninguna de las ocho tareas
de la sección siguiente empieza ahí. **La lista es el panel.**

### 1.3 El día, contado

Nueve de la mañana. Hace `git push` a `main` de la landing de un cliente. Tiene
autodespliegue puesto en esa app, así que abre Orbit Desktop para ver si el temporizador
ya lo ha cogido, y si no, lo lanza a mano. *(T2)*

Diez y media. El cliente le pide cambiar el correo de destino del formulario de
contacto. Eso es una variable de entorno. La cambia y reinicia. *(T6)*

Una del mediodía. Abre por costumbre y mira la lista. Todo verde. Cierra. *(T1)*

Cuatro y cuarto. Le escriben: «la tienda no carga». Mira: la app está en verde pero el
tráfico se ha desplomado. Abre los logs y ve 502. Mira el último despliegue: falló hace
dos horas en el paso `service` y se volvió atrás solo, pero la versión anterior también
está mal porque la migración de base de datos ya se había aplicado. *(T3, T4)*

Seis. Un correo de Orbit: certificado de un dominio a nueve días. Lo renueva. *(T7)*

Jueves. Un cliente nuevo. Repositorio, dominio, HTTPS. *(T5)*

Fin de mes. «¿Cuánta gente entra en la web?» Necesita un número que pueda mandar por
correo sin mentir. *(T8)*

### 1.4 Las ocho tareas, y qué comando las alimenta

Esta tabla es el contrato entre el usuario y el diseño. Si una pantalla no sirve a una
de estas ocho, sobra; si una de estas ocho necesita más de tres pasos, la pantalla está
mal.

| # | Tarea | Comandos | Frecuencia | Coste en el servidor |
|---|---|---|---|---|
| **T1** | ¿Está todo bien? | **`status --json`** (trae también las apps) | varias/día | **389 ms** medido, 40 apps |
| **T2** | Publicar el cambio | `deploy <app> --json --progress` | 1-10/día | 20 s – 4 min |
| **T3** | Se ha roto: ¿qué y desde cuándo? | `logs`, `info --json`, `watch status --json`, `doctor --json` | semanal | `info` **86 ms**; logs continuo; doctor segundos |
| **T4** | Volver atrás ya | `rollback <app> <release>` | mensual | 3-8 s |
| **T5** | Web nueva | `new --repo … --domain … --yes` | mensual | 1-4 min |
| **T6** | Tocar la configuración | `env set/get/unset`, `port`, `redirect`, `ssl`, `maintenance` | semanal | < 2 s salvo `ssl` |
| **T7** | Enterarme de lo que nadie me ha dicho | `doctor --json`, y `status --json` (`cert_days`, `served`) | semanal | doctor: segundos, hace `dig` por dominio |
| **T8** | ¿Cuánta gente entra? | `traffic [app] --json` | mensual | segundos a decenas, según ventana |

Las cifras de la columna de coste **están medidas**, no estimadas: banco de 40
aplicaciones de cuatro tipos, ejecutando `orbit` v1.3.6 de verdad. Medianas:
`version --json` **72 ms**, `info --json` **86 ms**, `list --json` **306 ms**,
`status --json` **389 ms**. Reproducidas de forma independiente sobre el mismo banco:
0,06 s / 0,30 s / 0,38 s. Coinciden.

Y una novena que sólo existe a partir del segundo servidor, y que es de otra
naturaleza porque es un error, no una tarea:

| **T9** | ¿En qué servidor estaba esto? / no equivocarme de servidor | — | siempre | 0 |

T9 no se resuelve con una pantalla, se resuelve con **contexto permanente**: el
servidor activo tiene que estar visible en todo momento y ser imposible de confundir.
Es el motivo de que en §2.3 haya un rail permanente en vez de un desplegable.

### 1.5 Las dos reglas de latencia, y el hallazgo que retira 306 ms de la portada

> **Regla 1 · Una pantalla, como mucho una llamada bloqueante.** Si una vista necesita
> dos comandos, uno es el principal —el que decide si la pantalla existe— y el otro se
> pide después y rellena su hueco cuando llega.

El detalle de una aplicación necesita `info --json` (86 ms medidos) para existir, y
`metrics --json` y `traffic --json` para completarse. Si se esperara a los tres, la
pantalla tardaría lo que tarde el más lento. Con la regla, aparece en 86 ms con dos
huecos que se rellenan solos.

> **Regla 2 · Antes de pedir dos cosas, comprobar si una de ellas ya trae la otra.**

Y aquí está el hallazgo que más latencia retira de todo el producto, y que no cuesta
nada porque es sólo elegir bien qué comando se pide: **`orbit status --json` devuelve el
array `apps` completo y byte a byte idéntico al de `orbit list --json`.** Verificado
comparando los dos objetos: 40 elementos cada uno, `==` verdadero.

La portada —cabecera del servidor más lista de aplicaciones— parecía necesitar
`status --json` **y** `list --json`: 389 + 306 = **695 ms**. Necesita sólo el primero:
**389 ms**, y de ahí se alimentan las dos cachés. Es un **44 % menos de latencia en la
carga que forma la primera impresión del producto**, y sale de leer el emisor en vez de
suponer.

`list --json` sigue existiendo para el caso en que se quiera refrescar sólo la lista sin
volver a pagar los `free`, `df` y `systemctl` del host: es 83 ms más barato.

El caso contrario también está en el contrato y hay que respetarlo: `orbit top --json`
**tarda un segundo largo a propósito**, porque el porcentaje de CPU es la diferencia
entre dos lecturas y la primera no existe. Esa espera no se puede optimizar, sólo se
puede explicar; en §4.1 y §4.7 se explica.

---

## 2. Arquitectura de información

### 2.1 Tres niveles, y ni uno más

```
Servidor  ──▶  Aplicación  ──▶  Acción
(SSH)          (app de orbit)   (comando)
```

Todo lo que hay en el producto cuelga de esos tres niveles. Las cosas que son del
servidor y no de una app —estado del host, doctor, vigilancia, colas, bases de datos,
copias— cuelgan del primero. Todo lo demás cuelga del segundo. Un cuarto nivel
(«entornos», «proyectos», «equipos») sería inventar un concepto que Orbit no tiene, y
la primera vez que la interfaz inventa un concepto que el servidor desconoce, empiezan
las dos verdades contra las que avisa §13.4.

### 2.2 La rejilla

```
┌────┬───────────────────────────┬──────────────────────────────────────────────┐
│ ▣  │  ⌕ buscar          ⌘K     │  criticabits                        ▸ acciones│
│vps │ ─────────────────────────  │ ─────────────────────────────────────────────│
│    │  ● criticabits    next    │  Resumen  Despliegues  Logs  Tráfico  Entorno │
│ ▣  │  — estropealo   static    │  Redirecciones  Copias  Avanzado              │
│ovh │  ▲ brokenufo      next    │ ─────────────────────────────────────────────│
│    │  ✕ tienda      laravel    │                                               │
│ ▣  │  ⊘ api-vieja       node   │   (contenido de la pestaña)                   │
│lab │                           │                                               │
│    │ ─────────────────────────  │                                               │
│ +  │  SERVIDOR                 │                                               │
│    │  Estado · Doctor 2▲       │                                               │
│    │  Vigilancia · Colas       │                                               │
│    │  Bases de datos · Copias  │                                               │
├────┴───────────────────────────┴──────────────────────────────────────────────┤
│ ● vps-ovh · orbit 1.3.6 · contrato 1 · leído hace 4 s · 41 ms          ⏸  ⟳   │
└───────────────────────────────────────────────────────────────────────────────┘
  56px          280–360px (redim.)              resto
```

Tres zonas y una barra de estado. La zona 1 es el rail de servidores; la 2 es la lista
de aplicaciones más los enlaces de servidor; la 3 es el panel principal.

**Por qué el rail y no un desplegable.** Es la decisión que más discusión merece porque
cuesta 56 píxeles permanentes. La justifica T9: el error caro de un producto
multiservidor no es tardar en cambiar de servidor, es **actuar en el equivocado**. Un
desplegable esconde el contexto exactamente cuando se olvida —después de tres minutos
mirando otra cosa—, y el precio del olvido aquí es un `orbit deploy` o un
`orbit remove` en la máquina que no era. Con un rail, el servidor activo es un bloque
de color permanente en la periferia visual, y además el rail puede llevar el estado de
cada servidor (verde, ámbar, rojo, gris de desconectado) sin que nadie tenga que abrir
nada. Descartado además un selector arriba a lo Vercel: con uno o dos servidores es
igual, con cinco es peor, y no aporta el estado.

**Por qué la lista de apps vive en la columna y no en el panel principal.** Porque el
modo incidente es «mirar una app sin perder de vista las demás». Si la lista fuera una
pantalla y el detalle otra, cada comparación entre dos aplicaciones costaría dos
navegaciones. En el modo rutina la columna es la pantalla: se abre la app, se lee la
columna, se cierra.

La columna es redimensionable entre 280 y 360 px y se puede colapsar a 0 con `⌘\`. Con
40 aplicaciones y densidad compacta caben 28 filas sin desplazar en una ventana de
900 px de alto; con densidad cómoda, 21. Eso fija la densidad como una preferencia real
y no como un capricho (§3.7).

### 2.3 Vocabulario de superficies

La regla que evita que esto se convierta en un caos de ventanitas. Cinco superficies y
un criterio duro para cada una.

**Pantalla.** Ocupa el panel principal, tiene ruta interna, entra en el historial de
navegación (`⌘[` y `⌘]`) y sobrevive a cambiar de aplicación en la columna. Es para lo
que se mira más de diez segundos o para lo que se abandona y se vuelve. Todo lo que se
lee —logs, tráfico, monitor, doctor, despliegue en vivo— es pantalla.

**Panel lateral (inspector).** Se abre por la derecha, entre 380 y 520 px, **encima de
la pantalla pero sin taparla ni bloquearla**. Es para el detalle de *una fila* cuando
la lista de la que sale sigue importando: un *check* del doctor, un sujeto de la
vigilancia, una redirección, una release en el historial. Se cierra con `Esc` y con
clic fuera. No atrapa el foco, porque no bloquea nada.

**Modal.** Bloquea. Y por eso la regla es dura: **si un modal no contiene una decisión
con consecuencia, no debe ser un modal.** En este producto sólo hay tres clases de
modal: el asistente de web nueva (§2.6), la hoja de comando de una acción destructiva,
y los avisos de seguridad que no se pueden pasar por alto (host key cambiada). Un modal
para «editar el nombre visible del servidor» sería un error de diseño.

**Hoja de comando.** Una variante de modal específica de este producto, y probablemente
su pieza más característica. Antes de ejecutar cualquier acción que cambie algo,
enseña **la orden literal**, monoespaciada, seleccionable y copiable:

```
┌─ Volver atrás ────────────────────────────────────────────────┐
│                                                               │
│  Se va a ejecutar en vps-ovh:                                 │
│                                                               │
│   orbit rollback criticabits 20260805-041230                  │
│                                                               │
│  Reinicia el servicio y recarga nginx. La web deja de         │
│  responder entre uno y dos segundos.                          │
│                                                               │
│  ☐ No volver a preguntar para rollback en este servidor       │
│                                                               │
│                        [ Cancelar ]  [ Volver atrás ]         │
└───────────────────────────────────────────────────────────────┘
```

Existe por tres motivos, y los tres son de producto y no de estética. Uno: es la prueba
visible de que la aplicación **sólo invoca `orbit`**, que es la promesa que la deja
existir. Dos: enseña la CLI mientras se usa el ratón, y el usuario objetivo ya sabe
terminal —el día que necesite hacerlo por SSH sabrá cómo—. Y tres: convierte el «¿qué
va a hacer exactamente este botón?» en una lectura de tres segundos, que es lo que
convierte un panel en algo en lo que se confía. Se puede desactivar por acción y por
servidor, salvo para `remove --purge`, que no se puede desactivar nunca.

**Toast.** Esquina inferior derecha, 4 s, apilable hasta 3. Sólo para confirmar acciones
que ya terminaron, fueron rápidas y no tienen pantalla propia: «Variable guardada»,
«Servicio reiniciado», «Copiado». Nunca para errores que requieran leer: un error que
desaparece solo es un error que se pierde.

**Barra de estado.** Permanente, 28 px. Servidor conectado, versión de `orbit`, versión
de contrato, antigüedad del último dato, latencia del último comando, y dos controles:
pausar el refresco automático y refrescar ya. La antigüedad del dato en la barra no es
decoración: es lo que permite que la aplicación no mienta cuando el servidor deja de
contestar (§4.3).

### 2.4 Mapa de pantallas

```
Orbit Desktop
│
├── Conexiones                                    [pantalla]  · sin comando
│   ├── Añadir servidor (host, usuario, ~/.ssh/config, ProxyJump, probar)
│   └── Detalle de servidor (versión, contrato, latencia, apps)
│
├── ‹servidor›
│   │
│   ├── Aplicaciones                              [portada]   · status --json ①
│   │   ├── fila → panel lateral de resumen rápido           · (del mismo objeto)
│   │   └── Nueva web…                            [modal]     · new --yes  §2.6
│   │
│   ├── ‹app›                                     [pantalla]  · info --json
│   │   ├── Resumen        (config + state + salud + acciones)
│   │   ├── Despliegues    (historial + metrics)              · metrics --json
│   │   │   └── release → panel lateral (rollback)
│   │   ├── Logs                                  [pantalla]  · logs [--since]
│   │   ├── Tráfico                               [pantalla]  · traffic --json
│   │   ├── Entorno        (SOLO nombres)                     · env list --json
│   │   │   └── revelar valor                     [hoja]      · env get
│   │   ├── Redirecciones                                     · redirect list --json
│   │   ├── Copias                                            · backup
│   │   └── Avanzado       (puerto, clonar, aislar, eliminar) [hojas]
│   │
│   ├── Despliegue en vivo                        [pantalla]  · deploy --json --progress
│   ├── Despliegue de todas                       [pantalla]  · deploy --all --json --progress
│   │
│   ├── Monitor                                   [pantalla]  · top --json (bucle)
│   ├── Estado del servidor                       [pantalla]  · (del mismo ①)
│   ├── Diagnóstico                               [pantalla]  · doctor --json  §2.8
│   │   └── check → orden copiable, sin botón      ② no hay --fix automatizable
│   ├── Vigilancia                                [pantalla]  · watch status --json
│   ├── Colas                                     [pantalla]  · queue status --json
│   ├── Bases de datos                            [pantalla]  · db list --json
│   └── Ejecutar un comando                       [pantalla]  · exec  §2.10
│
├── Servidores (con más de 3)                     [portada]   · status --json × visibles
│
└── Preferencias                                  [pantalla]  · sin comando
    (idioma, tema, densidad, movimiento, confirmaciones, atajos,
     modo presentación, notificaciones)

① `status --json` trae el array `apps` completo e idéntico al de `list --json`:
   una llamada de 389 ms alimenta la portada y la pantalla de estado. §1.5
② `orbit doctor --fix --json --yes` no existe: muere con «no sé qué es --yes».
   Verificado ejecutándolo. §2.8
```

Cinco decisiones de este mapa merecen defensa.

**El despliegue en vivo es pantalla, no modal.** Dura de veinte segundos a cuatro
minutos. Un modal de cuatro minutos es una aplicación secuestrada, y el usuario tiene
cosas que hacer mientras compila: mirar los logs de otra app, preparar el siguiente
despliegue, leer el tráfico. Siendo pantalla, **se puede abandonar sin cancelar**: el
despliegue sigue, aparece un indicador vivo en el rail y en la fila de la app, y se
vuelve con un clic. Se desarrolla entero en §6.

**El diagnóstico es pantalla, y sus *checks* se leen en la propia lista.** En la
ronda 1 los puse en un panel lateral con un botón de arreglar. El botón no puede
existir —`doctor --fix --json --yes` está roto en el servidor, §2.8— y sin botón el
panel lateral sobra: lo que queda de cada *check* es un mensaje y una orden para
copiar, y eso cabe en la propia fila. Un panel lateral que sólo sirviera para repetir
lo que ya se ve es un clic de peaje.

**El entorno no es un editor, es una lista de nombres con un acto deliberado detrás.**
El contrato devuelve sólo los nombres (§13.2), y la interfaz no va a intentar rodearlo.
Cada fila es un nombre y un botón «revelar», que abre una hoja de comando enseñando
`orbit env get <app> <CLAVE>` y, al aceptar, muestra el valor durante 30 segundos con
un botón de copiar, sin quedárselo en memoria más allá de eso, sin escribirlo en
ningún registro y sin que aparezca nunca en una captura de la pantalla principal. La
lista, por defecto, no revela nada. Es más lento que un `.env` editable en una caja de
texto, y esa lentitud es el producto.

**El terminal deja de ser un panel acoplado y pasa a ser dos cosas separadas.** En la
ronda 1 era una superficie acoplada al pie que servía a la vez para `logs` y para
`exec`, y era un error de categoría: leer un log es una lectura y ejecutar un comando
es una acción con privilegios de root. Ahora **Logs es una pestaña de la app** (lectura,
con filtros y ventanas de tiempo) y **`exec` es una pantalla propia y visiblemente
distinta** (§2.10). La regla de fondo no cambia: **si la aplicación enseña un terminal
para contar algo que el contrato ya sabe decir estructurado, está renunciando**.

**La portada de servidores sólo aparece a partir del cuarto servidor.** Con uno, dos o
tres, el rail ya los enseña todos a la vez y una pantalla intermedia sería un clic
inventado. §2.9.

### 2.5 El detalle de app: pestañas, no acordeón

Ocho secciones. Un acordeón las apilaría en una columna infinita donde la posición de
cada cosa depende de lo que esté abierto arriba, y eso rompe la memoria muscular, que
es justo lo que hace rápido a un usuario recurrente. Con pestañas, «Logs» está siempre
en el mismo píxel. Además la pestaña activa se recuerda **por app**: quien entra a la
tienda a mirar logs, la próxima vez que entre a la tienda entra en logs.

Las pestañas se cargan perezosamente: sólo la visible pide su comando. Cambiar de
pestaña con datos ya cargados no vuelve a pedirlos si tienen menos de 30 segundos.

### 2.6 El asistente de web nueva, rehecho

En la ronda 1 esto eran cuatro pasos y una pantalla de resultado, y yo mismo lo señalé
como lo más frágil del documento. Leer `cmd_new` de verdad confirma por qué, y da lo que
faltaba para arreglarlo.

**Los cuatro hechos que cambian el diseño:**

1. **`orbit new` hace el primer despliegue dentro**, y no hay bandera para saltárselo.
   Así que el comando **puede tardar tres minutos** y devolver `1` con la aplicación ya
   creada, registrada y con vhost. El propio Orbit lo explica en prosa
   (`_new_undeployed`) distinguiendo tres casos según haya release activa y según
   responda el puerto: excelente para una persona, ilegible para un cliente.
2. **`new` no tiene `--json`.** Ni el comando, ni su resultado, ni sus errores. Lo único
   que devuelve es prosa y un código de salida.
3. **`--yes` no es «que sí a todo».** No crea la base de datos, no abre el editor del
   `.env`, y **sí emite el certificado** — salvo que no haya `LETSENCRYPT_EMAIL`, en cuyo
   caso avisa, **sigue, y no falla**. Ese caso está documentado en §13.5 como inevitable:
   Let's Encrypt necesita un email y sin terminal no hay a quién pedírselo.
4. **El nombre de la app tiene una forma exacta**: `^[a-z0-9][a-z0-9._-]{0,39}$` y sin
   `..`. Cuarenta caracteres, minúsculas. El validador del formulario es ése,
   literalmente, y valida **mientras se escribe**, no al enviar.

De 1 y 2 sale la decisión que sostiene todo lo demás:

> **La interfaz no interpreta la salida de `orbit new`. Le vuelve a preguntar al
> servidor.** Al terminar —salga con 0 o con 1— se ejecuta `orbit info <nombre> --json`,
> que sí tiene contrato, y el estado real se lee de ahí. Es una llamada de 86 ms sobre un
> comando que ha tardado tres minutos, y es la única forma que **no depende del idioma
> del servidor**. Analizar `_new_undeployed` sería atarse a unas frases en castellano que
> pueden cambiar en cualquier versión.

#### 2.6.1 Los cinco pasos

```
1. Origen      repo (usuario/repo o URL https), rama, nombre
2. Detección   qué se le adelanta a la detección — vacío en el caso normal
3. Dominio     dominio, alias, y a dónde apunta ya ese nombre
4. Extras      certificado (con su correo) y base de datos
5. Repaso      la orden literal, y lo que va a pasar, antes de ejecutar
```

El paso 2 va antes que el dominio a propósito: es una pregunta sobre el
repositorio, y va pegada a la pantalla que pregunta por el repositorio.

> **Corrección de la ronda 2.** El paso 2 se describía aquí como «qué ha
> detectado Orbit, enseñado como resultado editable», y **eso no se puede
> hacer**. `detect_stack` recibe un directorio (`orbit:2862`) y `orbit init`
> hay que ejecutarlo dentro del proyecto (`orbit:6803`): no existe ninguna
> orden que mire un repositorio remoto y diga qué stack es, y el clon ocurre
> **dentro** de `orbit new`. Antes de ejecutar no hay conclusión que enseñar,
> sólo una promesa — y una promesa puesta junto a un botón «cambiar» se lee
> como un hecho.
>
> Lo que sí se puede, y es lo que hace la pantalla, son dos cosas distintas:
> **adelantarse** a la detección antes (§2.6.2) y **enseñar lo detectado**
> después (§2.6.2b), donde ya es un dato del contrato.

La misma ronda encontró otras dos cosas que este diseño daba por hechas:

**El autodespliegue no es una bandera de `new`.** Es `cmd_autodeploy`
(`orbit:10463`), una orden aparte que se activa cuando la web ya existe y
sirve. Estaba en el paso 4 de la ronda 1, y ofrecerlo ahí daría a entender que
`orbit new` lo configura. Se ha quitado del formulario, y en su sitio hay una
línea que dice por qué no está: quien lo busque merece saber dónde está, no que
la pregunta desaparezca.

**`orbit new` habla por stdout, no por stderr.** `_ui_route` (`orbit:447`) deja
`UI_FD=1` salvo con `--json`, y `new` es la única orden larga que no lo lleva.
El cliente servía stderr línea a línea en todas: correcto para el resto del
catálogo y mudo justo en la que tarda tres minutos. Está en §4.5 de
[CONTRACT.md](CONTRACT.md).

#### 2.6.2 Adelantarse a la detección, que es el paso 2

Orbit detecta unos veinticinco *stacks* leyendo el `package.json`, los ficheros
del repositorio y los adaptadores. Acierta mucho, y aun así se equivoca en tres
casos que no son culpa de la detección: un monorepo donde la app está en un
subdirectorio, un Astro o un SvelteKit donde el adaptador decide si sale un
sitio estático o un servidor, y un proyecto que arranca con un script propio.

En los tres, **quien lo tiene ya lo sabe**. Ésa es la pregunta que se puede
hacer antes de clonar, y es la que hace la pantalla:

```
┌─ Detección ─────────────────────────────────────────────────────────┐
│                                                                     │
│  Orbit detecta el tipo de proyecto AL CLONARLO, así que todavía no   │
│  puedo enseñarte qué va a detectar. Casi siempre acierta y este      │
│  paso se deja en blanco.                                            │
│                                                                     │
│  ▸ Ya sé que se va a equivocar                                      │
│                                                                     │
│  ⓘ Si la detección no encaja nunca —y en un monorepo raro no encaja  │
│    nunca—, ejecuta `orbit init` en tu repositorio y sube el          │
│    orbit.json. A partir de entonces manda el fichero.                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

Plegado, y **vacío es la respuesta correcta a casi todos los repositorios**. Un
formulario que insinúa que hay que rellenarlo consigue que se rellene mal.

Tres reglas del bloque de dentro, que son las de la ronda 1 que sí sobreviven:

**`--appdir` no es un campo más y no se pinta como tal.** Cambiar la carpeta
redirige la detección entera: el tipo y el build se leen contra otro
directorio. Va la primera, con su propia explicación, y no en la lista junto a
«build» y «arranque» — ponerla ahí daría a entender que es un ajuste al lado de
los otros, y es el que invalida a los otros.

**«Ninguno» es una respuesta y tiene que poder escribirse.** `--build ''`
significa «esta app no se compila», y es distinto de no decir nada. En la
interfaz son tres estados visibles y no un campo de texto: *lo detecta Orbit*
(gris), *un valor* (editable) y *sin build* (ámbar). Un campo de texto vacío no
puede significar las dos primeras cosas a la vez.

> Esto casi se pierde en el cableado, y de la peor forma. El tri-estado cruzaba
> el puente como `undefined` / `null` / cadena, dando por hecho que serde
> distinguiría un campo ausente de un `null` explícito en un
> `Option<Option<String>>`. **No los distingue**: los colapsa en `None`. El
> resultado habría sido que el repaso enseñara `--build ''` y el servidor
> recibiera una orden sin esa bandera — enseñar una cosa y ejecutar otra, en la
> pantalla cuyo único argumento es que eso no pase. Ahora la etiqueta se
> escribe (`AnulacionDeLaInterfaz`) y hay una prueba que lo fija.

**Y la salida de emergencia: `orbit init`.** Cuando la detección se equivoca de
forma sistemática, la respuesta correcta no es pelearse con este formulario cada
vez, es que la configuración viaje con el código. Al pie del bloque, el enlace.
Es la única recomendación del producto que le dice al usuario que se vaya de la
interfaz, y está bien que exista: `orbit init` es el reverso exacto de lo que lee
el despliegue, y el descriptor manda sobre la detección.

#### 2.6.2b Lo detectado, enseñado cuando ya es un hecho

Después de crear se lee `info --json`, que es la llamada que ya se hacía para
clasificar el final. Su `config` **es el descriptor tal cual**, así que el tipo,
la carpeta, el build, el arranque, la salida y el puerto están ahí, escritos por
el servidor. Esa es la pantalla que la ronda 1 quería, sólo que en el único
momento en que sus datos existen.

```
┌─ Lo que Orbit ha detectado en «tienda» ─────────────────────────────┐
│                                                                     │
│   Tipo      next                                                    │
│   Carpeta   apps/web                                                │
│   Build     está vacío                                              │
│   Arranque  pnpm start                                              │
│   Puerto    3007                                                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

Dos reglas:

**Un campo vacío lleva su etiqueta, nunca un guion.** El descriptor son cadenas:
`""` quiere decir que ahí no hay nada, y el servidor lo sabe. Un guion se lee
como «no lo sé», y es la misma regla con la que la portada no pinta un `null`
como un cero.

**No se inventa el porqué.** La ronda 1 pedía «tipo `next` **porque**
`package.json` tiene `"next": "15.1.0"`», y tenía razón en que una conclusión
con su prueba es verificable de un vistazo. Pero **el descriptor guarda el
resultado de la detección, no las pruebas que llevaron a él**, y un motivo
verosímil inventado es peor que ninguno: se comprueba igual de fácil y es
mentira. Lo que sí se enseña es qué hacer si no encaja, que es lo que se iba a
buscar con el porqué.

#### 2.6.2c El dominio, y el final F7 convertido en aviso

El paso 3 pregunta a dónde apunta ya el nombre, y lo compara con a dónde va el
`ssh`. Es la única pregunta del asistente que no sale del contrato, y existe
porque **una web publicada cuyo dominio no apunta al servidor se ve, desde
dentro, exactamente igual que una que sí**: es el final F7, y enterarse después
de tres minutos de build es enterarse tarde.

Tres cosas que la pantalla dice tal cual, porque son ciertas y porque callarlas
convertiría un aviso en una afirmación:

- **Lo resuelve esta máquina**, con su caché y su `/etc/hosts`. Puede no ser lo
  que ve el resto del mundo, sobre todo con un registro recién cambiado.
- **«No lo sé» no es «no coincide».** Sin una de las dos listas no hay
  comparación que hacer, y se pinta con el color de lo que no se sabe — el mismo
  con el que la portada pinta un certificado sin comprobar.
- **Nunca impide seguir.** Un servidor detrás de un proxy o de un CDN da
  direcciones distintas y no está roto.

#### 2.6.3 Los finales, que son siete y no uno

Esto es lo que faltaba en la ronda 1. `orbit new` no tiene dos finales; tiene siete, y
cinco de ellos son parciales. La pantalla de resultado los distingue con **el mismo
vocabulario de estados de §3.4**, y cada uno dice tres cosas: **qué existe, qué falta, y
qué se puede deshacer**.

| # | Final | Cómo se detecta (`info --json` después) | Qué existe | Qué falta | Deshacer |
|---|---|---|---|---|---|
| **F1** | **Publicada** | `served:true`, `releases≥1`, `ssl:true` | todo | — | `remove` |
| **F2** | **Publicada sin certificado** | `served:true`, `releases≥1`, `ssl:false` | todo salvo HTTPS | el certificado | emitirlo: `orbit ssl <app>` |
| **F3** | **Creada, build fallido** | app existe, `releases:0` | vhost, unidad, config, clon | una release que servir | reintentar: `deploy`; o `remove -y` |
| **F4** | **Creada, no responde** | `releases≥1`, `served:true`, `service` no activo | release publicada | el proceso arranca y se cae | logs; `deploy`; `remove -y` |
| **F5** | **Creada sin vhost** | `served:false` | config y release | nginx | `doctor` lo regenera |
| **F6** | **No se pudo clonar** | `info` no encuentra la app | nada | todo | nada que deshacer |
| **F7** | **Dominio sin DNS** | app publicada, `dig` no resuelve | todo | el DNS del usuario | nada nuestro |

Los cuatro que merecen su propio texto, porque son los que ocurren de verdad:

**F2 · Aplicación creada, certificado pendiente.** Es el final que el propio Orbit
documenta como inevitable, y la razón por la que `orbit new` avisa y sigue en vez de morir
a mitad: «dejarte la app creada pero el comando en error es la peor combinación». La
interfaz tiene que honrar esa decisión y **no pintarlo como un fallo**. Ni verde ni rojo:
estado propio, ámbar, con una sola acción delante y el motivo real:

> **La web ya está publicada en `http://tienda.com`.** Falta el certificado: Let's Encrypt
> pide un correo de contacto y no había ninguno configurado.
> `[ Emitir el certificado ]` → pide el correo, ejecuta `orbit ssl tienda`.

Y como efecto secundario que hay que enseñar antes y no después: el correo que se pide
aquí **se guarda en la configuración global del servidor**, no en la de la app. La próxima
web que se cree ya no lo pedirá. Decirlo cuando ocurre evita el «¿por qué ya no me lo
pregunta?» de dentro de tres meses.

**F3 · Aplicación creada, build fallido.** La app existe, tiene vhost, tiene unidad y
tiene el clon; lo que no tiene es una release que servir. Un visitante recibe un 502.
Se pinta como error, y la primera línea dice **lo que existe**, porque es lo que el
usuario no se espera:

> **La aplicación se ha creado pero no ha llegado a compilar.** `tienda` está registrada
> en el servidor, con su dominio y su configuración; no hay ninguna versión publicada, así
> que el dominio devuelve 502.

Tres acciones, en este orden: **ver el log del build** (primaria; el final del log, no el
principio), **reintentar el despliegue** (`orbit deploy tienda --json --progress`, con lo
que ya está creado — no se vuelve a pasar por el asistente), y **retirar** (`orbit remove
tienda -y`, sin `--purge`: es reversible y no hay datos que perder, la app no ha llegado a
servir nada).

**F6 · No se pudo clonar.** Es el único final donde no hay nada que deshacer, y por eso es
el más fácil de tratar bien. La causa casi siempre es una de dos, y se distinguen por el
`fatal:` de git:

| Lo que dice git | Lo que se enseña |
|---|---|
| `could not read Username` / `Authentication failed` | El repositorio es privado y este servidor no tiene credenciales. Se resuelve con `orbit github` **en el servidor**, y la interfaz lo dice sin fingir que puede hacerlo por ti: hace falta un navegador y una cuenta. |
| `Repository not found` | O no existe, o es privado y el token no lo alcanza. Se enseñan las dos hipótesis, porque git no las distingue. |
| `Could not resolve host` | El servidor no llega a GitHub. Es un problema de red del servidor, no del repositorio. |

Y la regla de presentación que ya viene del análisis del contrato: **se enseña el primer
`fatal:`, no la última línea de git**, que en el error más común es «and the repository
exists.» — un trozo suelto del párrafo de ayuda.

**F7 · Dominio sin DNS.** No es un fallo de Orbit ni de la interfaz, y la aplicación está
perfectamente creada. Se detecta antes, en el paso 3, con una comprobación de DNS en vivo
mientras se escribe el dominio, y se avisa **sin bloquear**: se puede crear igual, porque
mucha gente crea la app primero y apunta el DNS después. Si al terminar sigue sin
resolver, la pantalla de resultado lo dice y explica la consecuencia concreta: **el
certificado no se va a poder emitir por HTTP y, con Cloudflare, sí por DNS-01**.

#### 2.6.4 Cómo se ve mientras corre

`orbit new` no publica progreso: no tiene `--json` ni `--progress`. Así que la barra
determinista está prohibida (§5.4) y lo que se pinta es un **contador con los pasos
conocidos y sin porcentaje**, marcando el que está en curso por lo último que se ha visto
en stderr:

```
  Creando tienda en vps-ovh                                     1 min 12 s

  ✓ validar el nombre y el dominio
  ✓ clonar el repositorio
  ✓ escribir la configuración
  ◐ primer despliegue                            (esto es lo que tarda)
  ○ certificado

  orbit new --repo https://github.com/yo/tienda.git --domain tienda.com \
            --email yo@tienda.com --db --yes
```

La orden completa se queda visible durante toda la ejecución. Cuesta tres líneas y hace
que la espera de tres minutos sea una espera con contenido.

**Y una regla que sale de medir el comportamiento sin terminal:** toda orden que construya
la interfaz **lleva el nombre de la app explícito, siempre**, aunque parezca redundante.
Sin TTY, un `orbit restart` sin app no aborta: **elige la primera por orden alfabético,
en silencio, y sale con código 0**. Probado. Sólo `info --json`, `deploy --json` y
`rollback` se protegen. Eso convierte «se me olvidó pasar el nombre» en «he reiniciado la
web de otro cliente y nadie me lo ha dicho», y es de los errores que no se descubren hasta
que alguien llama.

### 2.7 La paleta de comandos

`⌘K`. Busca a la vez aplicaciones, servidores, pantallas y acciones. Es la ruta de
teclado para todo, y es también la respuesta al usuario que viene del terminal: escribir
`deploy tie` y pulsar Enter tiene que ser más rápido que `ssh mi-vps orbit deploy tienda`.
Las acciones destructivas aparecen en la paleta pero **siempre pasan por su hoja de
comando**: la paleta acelera llegar, no saltarse.

### 2.8 El diagnóstico, sin botón de arreglar

En la ronda 1 esta pantalla tenía un botón que ejecutaba
`orbit doctor --fix --json --yes`. **Ese comando no existe.** Está documentado en dos
sitios del repositorio de Orbit y ejecutado dice:

```
$ orbit doctor --fix --json --yes
  ✗ orbit doctor: no sé qué es «--yes». Sólo acepta --fix y --json.
$ orbit --yes version --json
  ✗ Comando desconocido: --yes
```

`--yes` no es una bandera global, y `cmd_doctor` rechaza todo lo que no sea `--fix`. Y sin
`--yes`, `doctor --fix --json` se niega a correr porque sin terminal no puede preguntar.
**Las dos ramas cierran la puerta y no hay llave.** No es una carencia del contrato: es un
fallo del servidor que ninguna prueba cazó porque las pruebas llaman a `cmd_doctor` como
función, saltándose el `main()` donde vive la bandera.

Así que el botón se cae, y con él se cae el diseño que tenía. Lo que queda es mejor de lo
que parece, porque **el contrato ya traía la pieza correcta**: `fixable`, que —dice el
propio contrato— existe «para que un cliente enseñe el botón SOLO cuando hace algo».

#### La pantalla honesta

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  Diagnóstico · vps-ovh                        leído hace 12 s   [ Volver a     │
│                                                                   comprobar ]  │
│  ✕ 2 errores    ▲ 5 avisos    ● 31 correctos                                   │
│  ─────────────────────────────────────────────────────────────────────────────│
│                                                                                │
│  ✕  vhost-tienda                                              [ copiar orden ] │
│     tienda.com no tiene vhost: nginx cierra la conexión                        │
│     ┌─────────────────────────────────────────────────────────────┐            │
│     │ sudo orbit doctor --fix                                     │            │
│     └─────────────────────────────────────────────────────────────┘            │
│     Ejecútalo en tu terminal. Orbit preguntará una vez y lo aplicará.          │
│                                                                                │
│  ✕  ports                                                     [ copiar orden ] │
│     Puertos internos duplicados: 3005 (api, api-staging)                       │
│                                                                                │
│  ▲  cert:tienda.com                                                            │
│     El certificado de tienda.com caduca en 9 días                              │
│     Se renueva solo. Orbit no lo fuerza a propósito: certbot tiene límites     │
│     de peticiones.                                            (sin arreglo)    │
│                                                                                │
│  ▲  hugo                                                                       │
│     hugo no está instalado, y lo necesitan: blog                               │
│     Instalar programas en tu servidor no es un diagnóstico.   (sin arreglo)    │
│  ─────────────────────────────────────────────────────────────────────────────│
│  ⓘ  Esta versión de Orbit (1.3.6) no puede aplicar los arreglos desde un       │
│     cliente. Ver por qué.                                                      │
└───────────────────────────────────────────────────────────────────────────────┘
```

Cinco decisiones:

**1. Tres presentaciones distintas, según `fixable` y `fix`.** Es la traducción literal de
lo que el contrato distingue, y es lo que evita que el usuario haga clic en la nada:

| `fixable` | `fix` | Qué se pinta |
|---|---|---|
| `true` | texto | La orden en un bloque copiable, con botón **«copiar orden»** y la frase de qué hará |
| `false` | texto | El texto de `fix` como explicación, marcado **(sin arreglo automático)** |
| `false` | `null` | Sólo el mensaje. Ni bloque, ni botón, ni hueco donde debería haber uno |

Esta tabla es también la respuesta a un error de diseño que se comete solo: pintar un
botón deshabilitado. Un botón gris invita a averiguar por qué está gris, y aquí la
respuesta es «porque instalar `hugo` en tu servidor no es trabajo de un diagnóstico»,
que es una frase, no un estado de un control.

**2. Copiar la orden, no ejecutarla, y decir por qué.** El bloque de la orden lleva
`sudo orbit doctor --fix` —el comando de verdad, el que sí funciona con una persona
delante— y un botón que lo pone en el portapapeles. Debajo, la banda informativa explica
en una línea que esta versión de Orbit no permite aplicarlo desde un cliente, con un
enlace a la explicación larga.

Esto es incómodo y es lo correcto. La alternativa —pedir un TTY, lanzar
`orbit doctor --fix` y **escribir `s\n` en su entrada estándar**— funciona, y se descarta
para la primera versión por una razón que no es técnica: sería **la única acción del
producto que responde una pregunta en nombre del usuario**, contra un comando que se ha
negado explícitamente a que se le responda sin terminal. Un cliente que aprende a
contestar «sí» a preguntas que no ha leído es exactamente lo que §13.5 llama «fingir un
terminal, la peor solución de todas». Si el arreglo del servidor tardara, se reconsidera
—y entonces se hace visible: la interfaz enseñaría la pregunta y la respuesta que va a
dar, y pediría confirmación por cada *check*.

**3. La banda de abajo cambia sola el día que Orbit lo arregle.** El arreglo en el
servidor son tres líneas —reconocer `--yes` en el `case` de `cmd_doctor`— y va a llegar.
Así que la interfaz **no cablea la carencia**: pregunta por `version --json`, y la
capacidad «arreglo automático» es una función de la versión de Orbit del servidor.

```
if (servidor.orbitVersion >= VERSION_CON_FIX)  → botón «Arreglar» + hoja de comando
else                                           → bloque copiable + banda informativa
```

El día que el usuario actualice su servidor, la pantalla **gana un botón sin que la
aplicación cambie**, y con un aviso de una vez: «Este servidor ya puede aplicar los
arreglos.» Es el mismo mecanismo del contrato incompatible de §4.4, aplicado a una
capacidad concreta en vez de a un número de contrato.

**4. El doctor no se ejecuta solo, nunca.** Cuesta segundos —hace `nginx -t` y un `dig`
por dominio— y su resultado no cambia sin que alguien lo provoque. Se pide al entrar en la
pantalla y con el botón «volver a comprobar», y su caché no caduca por tiempo: caduca
cuando algo la invalida. Un diagnóstico que se dispara solo cada minuto es un diagnóstico
que nadie lee y un VPS que hace `dig` cuarenta veces por hora.

**5. Y el orden importa.** Los *checks* llegan en el orden en que Orbit los emite, que es
el orden en que los comprueba. La pantalla **no los reordena por gravedad**, los agrupa:
errores arriba, avisos después, correctos plegados en una sola línea («31 correctos ▸»).
Plegar los correctos no es esconderlos: un diagnóstico con treinta y una líneas verdes
entrena a desplazarse sin leer, y entonces también se pasa por encima de los dos rojos.

### 2.9 Multiservidor operativo: qué se contesta que sí, y qué se contesta que no

En la ronda 1 dejé esto abierto. Se cierra aquí, con dos respuestas y un porqué para cada
una.

#### «Desplegar esto en los tres servidores» → **no, y con alternativa**

La petición es razonable y la respuesta es que no, por un motivo que no es de esfuerzo:
**un botón que despliega en N servidores es un plano de control**, y la única razón por la
que este producto puede existir es que no lo es. En cuanto haya un objeto «esta app en
todas partes», hay un estado que el cliente mantiene y que ningún servidor conoce: qué
servidores forman el grupo, qué versión debería estar en cada uno, qué se hace cuando dos
divergen. Eso es exactamente el `orbit remote add` con estado que la v2.0 se prohibió a sí
misma, y hacerlo en el cliente no lo hace menos cierto: lo hace menos visible.

Hay además un motivo práctico que pesa igual. Desplegar en tres servidores tiene tres
resultados posibles y **catorce combinaciones** de éxito parcial, y ninguna interfaz
sencilla las cuenta bien. El producto que resulta no es «desplegar en tres»: es «gestionar
la divergencia entre tres», que es otro producto.

**Lo que sí se ofrece, que cubre el 90 % del caso real:** el usuario que tiene la misma
app en producción y en *staging* quiere, casi siempre, desplegar **primero en uno, mirar,
y luego en el otro**. Para eso no hace falta un grupo: hace falta que el segundo despliegue
cueste dos teclas. Y cuesta dos teclas, porque `⌘K → deploy tienda` ya existe y porque la
pantalla de resultado de un despliegue ofrece, al terminar bien, **«desplegar el mismo
commit en otro servidor»** — una acción que nombra el commit (`a1b2c3d`), nombra el
servidor destino, y **abre otra pantalla de despliegue**, no lo hace en silencio. Un
despliegue, una pantalla, un servidor nombrado. La secuencia es del usuario.

#### «Todas mis apps de todos los servidores en una lista» → **no como lista; sí como portada de servidores**

Una tabla global de trescientas aplicaciones exige consultar los cuarenta servidores para
existir, y consultarlos otra vez para refrescarse. Es el plano de control con otro nombre,
y encima es una tabla que nadie puede leer.

Lo que sí hay, y coincide con lo que decide el arquitecto, es una **portada de servidores**
cuando hay más de tres:

```
┌────────────────────┐ ┌────────────────────┐ ┌────────────────────┐
│ ● vps-ovh   [prod] │ │ ▲ hetzner          │ │ ⊘ lab              │
│   orbit 1.3.6      │ │   orbit 1.3.6      │ │   no responde      │
│   12 apps          │ │   6 apps           │ │   desde las 10:32  │
│   todo servido     │ │   1 sin vhost      │ │   [ reintentar ]   │
│   1 cert a 9 días  │ │                    │ │   última lectura   │
│                    │ │                    │ │   hace 6 min       │
└────────────────────┘ └────────────────────┘ └────────────────────┘
```

Cada tarjeta es **una** llamada (`status --json`, que trae host y apps: §1.5), y **sólo se
consultan las tarjetas visibles**. Lo que enseña cada una está elegido, no acumulado:
nombre y color, estado, versión de Orbit, número de apps, **cuántas tienen `served:false`**
—el dato más accionable de todo el contrato— y el certificado más próximo a caducar. Eso
es lo que contesta «¿tengo que entrar en este servidor?», que es la única pregunta que una
portada multiservidor debe contestar.

Lo que **no** hace: sumar CPU de varios servidores, pintar un total de apps caídas, ni
refrescarse sola. El refresco es un botón, con el semáforo del arquitecto y un contador
honesto: «Comprobando 12 de 40».

#### Y la mitigación que de verdad importa: no equivocarse de servidor

Esto no es una funcionalidad, es una amenaza con precedente. La suite de pruebas del
propio Orbit, ejecutada como root en un servidor que tenía una app llamada `tienda`,
**borró el vhost de la app de producción**. 32 suites en verde, 2.512 comprobaciones,
0 fallos, y una web muerta. No fue un fallo de seguridad: fue una operación correcta
contra el objetivo equivocado.

Cinco medidas, y ninguna es un cartel de aviso:

1. **`servidor:app` es la unidad de identidad en toda la interfaz.** Nunca `tienda`:
   siempre `vps-ovh : tienda`. En los títulos de diálogo, en los toasts, en el registro
   local, en las notificaciones del sistema (§6.8) y en los anuncios del lector de
   pantalla (§7.8). El nombre de la app **no identifica nada por sí solo**, porque
   `tienda` existe en tres servidores.
2. **Color por servidor, elegido por el usuario, aplicado a la periferia.** El rail, el
   borde superior de la ventana y el borde de todo diálogo destructivo llevan el color del
   servidor activo. Es información periférica, que es donde se detectan los errores de
   contexto: no hay que leerla para que funcione.
3. **La marca de producción es de otra categoría.** Un servidor marcado como producción
   lleva un distintivo permanente y **exige la confirmación reforzada** en las tres
   operaciones peligrosas, aunque el usuario haya desactivado las confirmaciones para el
   resto.
4. **Ninguna operación destructiva se ejecuta contra «el servidor actual» implícito.** La
   hoja de comando nombra el servidor en el título, no en la letra pequeña, y el destino no
   se puede cambiar desde dentro del diálogo: para cambiar de servidor hay que cerrarlo.
5. **Ninguna orden se construye sin el nombre de la app.** Es la regla de §2.6.4, y aquí
   se ve por qué importa el doble: sin TTY, `orbit restart` sin app reinicia la primera por
   orden alfabético y sale con 0.

### 2.10 `exec`: dos modos, y ninguna shell embebida

Lo dejé señalado en la ronda 1 como el punto donde la interfaz trataba con demasiada
normalidad una shell con privilegios. El informe de QA lo diseña mejor de lo que yo lo
tenía, y me alineo con él en todo salvo en un matiz de presentación. Lo que se adopta:

**Es una pantalla propia con los cuatro datos siempre visibles**, no una cajita en la
esquina del detalle de la app: *qué app, en qué servidor, como qué usuario, con el `.env`
cargado*. Esos cuatro, en la cabecera, permanentes.

**Dos modos explícitos, y ésta es la parte que yo no tenía.** `orbit exec` aplica una regla
que no se ve: si recibe **un solo argumento** y ese argumento contiene un espacio o un
metacarácter, lo ejecuta con `bash -lc`; en cualquier otro caso ejecuta el `argv` tal cual.
Es decir que `exec web "ls -la"` y `exec web ls -la` **no son lo mismo**. Una interfaz que
aplicara esa heurística por su cuenta produciría un campo de texto donde el `&&` a veces
se ejecuta y a veces se pasa como argumento literal, sin que el usuario pueda predecir
cuál. Así que los dos modos se hacen visibles:

```
  ( ) Comando   argumentos separados, sin shell        ls  -la  /srv
  (•) Shell     lo interpreta bash -lc                 cd public && du -sh *
```

Y debajo, siempre, **la orden literal ya escapada** que se va a ejecutar. No una
aproximación: la cadena exacta. Es lo que convierte «confío en la interfaz» en «he leído lo
que va a pasar», y es verificable por el usuario sin creerse nada.

**Sin confirmación por comando**, porque sería inutilizable y enseñaría a pulsar sin leer.
Lo que sí hay: confirmación reforzada la primera vez de la sesión en un servidor marcado
como producción, y una lista corta de patrones que paran (`rm -rf` con ruta absoluta,
`drop database`, `mkfs`, `dd of=/dev/`). Esa lista **se documenta como lo que es**: no
impide nada —hay mil formas de escribir un `rm`— y sirve para el error de dedos a las tres
de la mañana, que es el caso real. Presentarla como una protección sería mentir.

**El aviso del `.env`, una vez por sesión.** `orbit exec` carga el `.env` con `set -a`, así
que lo que se ejecute ahí ve **todos los secretos de la app en su entorno**. Es cierto, la
gente no lo sabe, y es la clase de dato que cambia lo que alguien escribe en esa caja.

**El histórico vive en memoria y no toca el disco**, salvo que el usuario active lo
contrario y lea lo que implica. La gente escribe
`psql "postgresql://usuario:contraseña@…"` en esa caja: es la misma decisión de `bash` con
`HISTCONTROL=ignorespace`, con el valor por defecto al revés porque aquí la proporción de
comandos con secretos es mucho más alta.

**La salida se pinta como texto plano, siempre**, con tope de tamaño en pantalla y volcado
a fichero si se pide. Es salida arbitraria de un proceso arbitrario: puede traer
secuencias ANSI, bytes nulos o cinco megas en una línea.

**Y la shell interactiva no se ofrece.** `orbit exec <app>` sin comando abre `bash`, y un
cliente sin terminal no puede con eso. En su lugar hay un botón que **copia al portapapeles
la orden `ssh` completa** para pegarla en un terminal de verdad. Es más honesto que emular
media terminal.

**Mi único matiz, y es de presentación, no de fondo.** El informe de QA describe esto como
una pantalla más; yo mantengo que **la pantalla de `exec` debe verse distinta de las
demás**: superficie hundida (`--surface-sunken`), tipografía monoespaciada desde la
cabecera, y el color del servidor en un borde de 2 px en los cuatro lados en vez de sólo
arriba. No es decoración. En un producto donde todo lo demás es un formulario que invoca un
comando acotado, ésta es la única superficie donde el usuario escribe algo que el producto
no entiende y no puede acotar, y esa diferencia tiene que ser visible antes de leer nada.
Es el mismo argumento por el que `served:false` es el único chip sólido: **la forma
comunica la categoría**.

---

## 3. Sistema de diseño

### 3.1 De dónde sale la paleta

No la he inventado. Sale de leer `assets/banner.svg`, `logo.svg`, `logo-mark.svg` y
`favicon.svg` del repositorio de Orbit, que ya tienen una marca coherente:

| Uso en la marca | HEX |
|---|---|
| Cian de acento (órbita, degradado) | `#5EE7E7` |
| Cian del subtítulo del banner | `#6FE3E3` |
| Violeta del degradado (fin) | `#7C6CF0` |
| Núcleo, claro → oscuro | `#B9FBFB` → `#3AC7D8` |
| Fondo del logotipo | `#0B1020` |
| Fondo del banner (degradado) | `#080C18` → `#0D1428` → `#10182F` |
| Texto principal | `#EDF1FB` / `#E8ECF8` |
| Texto secundario | `#9FB3D9` / `#8A9AC0` / `#7C8BB0` |
| Texto terciario | `#5A6B92` |
| Borde | `#33456B` |
| Tipografía | Inter, 700 con `letter-spacing` alto en el logotipo |

De aquí sale la primera decisión estructural, y no es cosmética:

> **El tema oscuro es el canónico. El claro es una traducción, no un espejo.**

La marca de Orbit es oscura: el logotipo lleva su propio círculo `#0B1020` detrás, el
banner es un cielo nocturno con estrellas, y el producto es una herramienta de terminal.
Si se invirtiera literalmente para el tema claro, el cian `#5EE7E7` sobre blanco daría
**1,5:1** de contraste, que no es texto, es una sugerencia de texto. Así que el tema
claro tiene **sus propios valores de acento**, más oscuros y más saturados, elegidos
para cumplir contraste, y conserva el *tono* de la marca sin conservar el valor.

### 3.2 Primitivas

```css
:root {
  /* ── Marca (extraídas de assets/*.svg) ───────────────────────────── */
  --o-cyan-100: #B9FBFB;   /* núcleo claro */
  --o-cyan-300: #6FE3E3;
  --o-cyan-400: #5EE7E7;   /* acento canónico, sólo sobre oscuro */
  --o-cyan-500: #3AC7D8;
  --o-cyan-700: #10707E;   /* derivada: acento sobre claro */
  --o-cyan-800: #0B5561;   /* derivada: texto de acento sobre claro */

  --o-violet-300: #BDB5F8;
  --o-violet-400: #9C90F4; /* derivada: violeta legible sobre oscuro */
  --o-violet-500: #7C6CF0; /* marca; relleno y degradados, no texto */
  --o-violet-600: #5A48D6;
  --o-violet-700: #4B3BC4; /* derivada: violeta legible sobre claro */

  /* ── Tinta y papel ───────────────────────────────────────────────── */
  --o-ink-900: #080C18;    /* banner, fondo más profundo */
  --o-ink-850: #0B1020;    /* fondo canónico oscuro */
  --o-ink-800: #0D1428;
  --o-ink-750: #10182F;    /* superficie 1 */
  --o-ink-700: #16203C;    /* superficie 2 (elevación) */
  --o-ink-650: #1E2A4A;    /* separadores en oscuro */
  --o-slate-600: #253355;
  --o-slate-550: #33456B;  /* borde de marca */
  --o-slate-500: #4A5878;
  --o-slate-450: #55679A;  /* borde de control en oscuro */
  --o-slate-400: #5A6B92;
  --o-slate-350: #64729A;
  --o-slate-300: #7C8BB0;
  --o-slate-250: #7E8AA8;  /* borde de control en claro */
  --o-slate-200: #8A9AC0;
  --o-slate-150: #9FB3D9;
  --o-slate-100: #C6D0E4;
  --o-paper-200: #E3E8F2;  /* separadores en claro */
  --o-paper-100: #EDF1FB;
  --o-paper-50:  #F5F8FF;  /* fondo canónico claro */
  --o-white:     #FFFFFF;

  /* ── Estado ──────────────────────────────────────────────────────── */
  --o-green-400: #4ADE9E;  /* activo, sobre oscuro */
  --o-green-700: #0E6E49;  /* activo, sobre claro */
  --o-amber-400: #F5B544;  /* atención, sobre oscuro */
  --o-amber-700: #8A5406;  /* atención, sobre claro */
  --o-red-400:   #FF7A70;  /* fallo, sobre oscuro */
  --o-red-600:   #C63127;
  --o-red-700:   #A32720;  /* fallo, sobre claro */
}
```

Tres notas sobre por qué hay valores «derivados» y no sólo los de la marca.

`#7C6CF0`, el violeta de la marca, **da 4,74:1 sobre `#0B1020` y 4,03:1 sobre la
superficie elevada `#16203C`**. Es decir: pasa el mínimo AA en el fondo base y lo falla
en cuanto se pone dentro de una tarjeta. Un color que cumple o no según dónde caiga no
sirve como token de texto. Se queda para rellenos, degradados y el logotipo, y para
texto sobre oscuro se usa `#9C90F4` (6,95:1 sobre base, 5,90:1 sobre superficie 2).

`#5EE7E7` sobre blanco es inutilizable como texto, pero **sobre oscuro es
excelente: 12,68:1**. Es un color de tema oscuro, no un color de marca universal, y
tratarlo como universal es el error clásico de trasladar una identidad hecha para un
banner a una interfaz que se lee ocho horas.

Los verdes y ámbares «obvios» no llegan. `#12885A` da **4,21:1** sobre `#F5F8FF` y
`#A66508` da **4,40:1**: los dos fallan AA de cuerpo por poco, que es la peor forma de
fallar porque parece que va bien. Bajados a `#0E6E49` (5,90:1) y `#8A5406` (5,89:1)
cumplen con margen.

### 3.3 Semánticos, con los dos temas

```css
/* ── Tema claro (canónico del bloque base) ─────────────────────────── */
:root {
  --bg:            var(--o-paper-50);
  --surface:       var(--o-white);
  --surface-2:     var(--o-paper-50);
  --surface-sunken: #EDF1F8;
  --border:        var(--o-paper-200);      /* separadores, decorativos */
  --border-strong: var(--o-slate-250);      /* bordes de control, 3,45:1 */
  --fg:            var(--o-ink-850);        /* 17,81:1 */
  --fg-muted:      #4A5878;                 /*  6,68:1 */
  --fg-faint:      var(--o-slate-350);      /*  4,47:1 — sólo ≥16 px */
  --accent:        var(--o-cyan-700);       /*  5,43:1 */
  --accent-text:   var(--o-cyan-800);       /*  7,95:1 */
  --accent-fill:   var(--o-cyan-700);
  --on-accent:     var(--o-white);
  --focus:         var(--o-cyan-800);       /*  8,45:1 */

  --st-ok:      var(--o-green-700);         /*  5,90:1 */
  --st-warn:    var(--o-amber-700);         /*  5,89:1 */
  --st-error:   var(--o-red-700);           /*  6,89:1 */
  --st-unknown: var(--o-slate-350);         /*  4,47:1 */
  --st-na:      var(--o-slate-350);
  --on-solid:   var(--o-white);
  --shadow-1: 0 1px 2px rgba(11,16,32,.06), 0 1px 3px rgba(11,16,32,.08);
  --shadow-2: 0 4px 12px rgba(11,16,32,.10), 0 1px 3px rgba(11,16,32,.06);
  --shadow-3: 0 12px 32px rgba(11,16,32,.16), 0 2px 8px rgba(11,16,32,.08);
}

/* ── Tema oscuro: por preferencia del sistema, salvo override a claro ─ */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { /* @import de --dark */ }
}
/* ── …y por elección explícita, que gana en los dos sentidos ────────── */
:root[data-theme="dark"] { /* mismo bloque */ }

/* El bloque, escrito una vez y aplicado en los dos sitios: */
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) {
  --bg:            var(--o-ink-850);
  --surface:       var(--o-ink-750);
  --surface-2:     var(--o-ink-700);
  --surface-sunken: var(--o-ink-900);
  --border:        var(--o-ink-650);
  --border-strong: var(--o-slate-450);      /* 3,18:1 sobre superficie */
  --fg:            var(--o-paper-100);      /* 16,75:1 sobre fondo */
  --fg-muted:      var(--o-slate-150);      /*  8,95:1 */
  --fg-faint:      var(--o-slate-300);      /*  5,57:1 */
  --accent:        var(--o-cyan-400);       /* 12,68:1 */
  --accent-text:   var(--o-cyan-400);
  --accent-fill:   var(--o-cyan-400);
  --on-accent:     #06121A;                 /* 12,68:1 sobre el cian */
  --focus:         var(--o-cyan-400);       /* 11,78:1 sobre superficie */

  --st-ok:      var(--o-green-400);          /* 11,03:1 */
  --st-warn:    var(--o-amber-400);          /* 10,44:1 */
  --st-error:   var(--o-red-400);            /*  7,46:1 */
  --st-unknown: var(--o-slate-300);          /*  5,57:1 */
  --st-na:      var(--o-slate-300);
  --on-solid:   var(--o-ink-850);            /*  7,46:1 sobre el rojo */
  /* En oscuro la sombra casi no se ve: la elevación la hace la superficie. */
  --shadow-1: none;
  --shadow-2: 0 8px 24px rgba(0,0,0,.45);
  --shadow-3: 0 20px 48px rgba(0,0,0,.60);
}}
:root[data-theme="dark"] { /* idéntico al bloque anterior */ }
```

Sobre el `override`: se hace con `:root:not([data-theme="light"])` dentro del *media
query* y `:root[data-theme="dark"]` fuera, de modo que **la elección explícita gana en
los dos sentidos**. Un usuario en un sistema oscuro que quiere la aplicación en claro
lo consigue, y al revés también. Es el bug clásico de los temas hechos sólo con *media
query*: el botón de tema sólo funciona en una dirección.

Los tintes de los chips se calculan con `color-mix()` en vez de mantener seis colores
más a mano:

```css
.chip { background: color-mix(in srgb, var(--chip) 12%, var(--surface)); }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) .chip {
  background: color-mix(in srgb, var(--chip) 18%, var(--surface));
}}
```

Los porcentajes no son iguales a propósito: sobre oscuro hace falta más tinte para que
el chip se despegue del fondo, y sobre claro menos para que el texto siga cumpliendo.

### 3.4 Los estados: la parte que más importa

El contrato da los datos crudos. La interfaz tiene que resolverlos a **un** estado
visible, y la regla de resolución tiene que estar escrita, ser determinista y ser la
misma en la lista, en el detalle, en el monitor y en el rail. Si la lista y el detalle
resuelven distinto, el producto ha perdido.

**Orden de precedencia** (el primero que se cumple, gana):

```
1. served === false          →  SIN VHOST      ⊘   sólido rojo
2. maintenance === true      →  MANTENIMIENTO  ▲   ámbar
3. service === null          →  —              —   neutro          (no aplica)
4. service === "active"      →  ACTIVO         ●   verde
5. service === (otro)        →  PARADO         ✕   rojo
```

Y las facetas que **no** son el estado, y por eso van en columnas propias y nunca
alteran el chip principal:

```
ssl === false                →  «sin HTTPS»    neutro con aviso, no rojo
cert_days < 10               →  «caduca en 9 d» ámbar
cert_days === null           →  «·»            desconocido
cpu_percent === null         →  «·»            desconocido
requests_capped === true     →  «5000+»
autodeploy / queue           →  distintivos neutros, informativos
```

La tabla completa, con lo que se pinta y —más importante— **qué frase se enseña**,
porque el color sin la frase no dice nada:

| Estado | Glifo | Color | Texto | Frase de detalle |
|---|---|---|---|---|
| Activo | `●` | `--st-ok` | `activo` | El servicio responde. |
| Parado | `✕` | `--st-error` | `parado` | El servicio existe y no está corriendo. |
| No aplica | `—` | `--st-na` | `—` | Web estática: no hay ningún proceso que arrancar. **Esto no es un fallo.** |
| Mantenimiento | `▲` | `--st-warn` | `mantenimiento` | nginx devuelve 503 con tu página de «volvemos enseguida». |
| Sin vhost | `⊘` | `--st-error` **sólido** | `sin vhost` | nginx no tiene el vhost. La conexión se cierra: ni 404 ni 502. |
| Desconocido | `·` | `--st-unknown` | `·` | No se sabe todavía. |

Tres reglas que salen de esta tabla y que son innegociables:

**El color nunca es el único portador.** Cada estado lleva glifo, color y texto. Tres
motivos: el 8 % de los hombres tiene alguna deficiencia de visión del color; las
capturas de pantalla que la gente pega en un *issue* a veces son en escala de grises; y
el usuario mira esta lista de reojo, en periferia, donde el color se pierde antes que
la forma. Es también el mínimo de WCAG 1.4.1, pero se hace por lo primero, no por lo
segundo.

**«No aplica» y «desconocido» tienen glifos distintos, y los dos son neutros.** `—` y
`·`. Los tomo prestados de `orbit top`, que ya usa exactamente esos dos caracteres para
exactamente esas dos cosas. Que la interfaz gráfica y la de terminal coincidan en el
glifo no es nostalgia: es que la misma persona usa las dos, a veces el mismo día, y
aprender dos vocabularios para lo mismo es un impuesto gratuito.

**`served:false` es el único estado con chip sólido.** Todos los demás son chips
teñidos: fondo al 12-18 % y texto del color. `sin vhost` es fondo sólido y texto
invertido (`#0B1020` sobre `#FF7A70`: **7,46:1**; `#FFFFFF` sobre `#A32720`: **7,32:1**).
La distinción es de **forma**, no de tono, y por tanto sobrevive al daltonismo y al
blanco y negro. Se lo gana porque es el estado que USAGE llama «lo primero que hay que
mirar»: sin vhost, ningún otro campo del estado describe lo que recibe el visitante, ni
siquiera `maintenance`, porque sin vhost tampoco se sirve la página de 503.

### 3.5 Contraste, medido

Todos los números de esta sección están calculados con la fórmula de luminancia
relativa de WCAG 2.1 sobre los HEX exactos de §3.2, no estimados a ojo. Los reproduzco
porque una cifra dentro de un documento no la comprueba nadie, y ya se sabe cómo acaba
eso.

**Tema oscuro, sobre el fondo `#0B1020` / superficie `#10182F` / superficie 2 `#16203C`:**

| Token | HEX | fondo | superficie | superficie 2 | Veredicto |
|---|---|---|---|---|---|
| `--fg` | `#EDF1FB` | 16,75 | 15,56 | 14,22 | AAA |
| `--fg-muted` | `#9FB3D9` | 8,95 | 8,31 | 7,60 | AAA |
| `--fg-faint` | `#7C8BB0` | 5,57 | 5,17 | 4,73 | AA cuerpo |
| `--accent` | `#5EE7E7` | 12,68 | 11,78 | 10,77 | AAA |
| `--st-ok` | `#4ADE9E` | 11,03 | 10,25 | 9,37 | AAA |
| `--st-warn` | `#F5B544` | 10,44 | 9,70 | 8,86 | AAA |
| `--st-error` | `#FF7A70` | 7,46 | 6,93 | 6,33 | AAA |
| `--st-unknown` | `#7C8BB0` | 5,57 | 5,17 | 4,73 | AA cuerpo |
| violeta texto | `#9C90F4` | 6,95 | 6,46 | 5,90 | AA+ |
| violeta marca | `#7C6CF0` | 4,74 | 4,41 | 4,03 | **descartado como texto** |
| `#5A6B92` | | 3,56 | 3,31 | 3,03 | sólo no-texto |
| `#33456B` | | 1,99 | 1,85 | 1,69 | sólo decorativo |

**Tema claro, sobre `#F5F8FF` / `#FFFFFF`:**

| Token | HEX | fondo | superficie | Veredicto |
|---|---|---|---|---|
| `--fg` | `#0B1020` | 17,81 | 18,93 | AAA |
| `--fg-muted` | `#4A5878` | 6,68 | 7,10 | AAA |
| `--fg-faint` | `#64729A` | 4,47 | 4,76 | AA sólo ≥16 px |
| `--accent` | `#10707E` | 5,43 | 5,77 | AA |
| `--accent-text` | `#0B5561` | 7,95 | 8,45 | AAA |
| `--st-ok` | `#0E6E49` | 5,90 | 6,28 | AA+ |
| `--st-warn` | `#8A5406` | 5,89 | 6,27 | AA+ |
| `--st-error` | `#A32720` | 6,89 | 7,32 | AAA |
| `--st-unknown` | `#64729A` | 4,47 | 4,76 | AA con reserva |
| violeta | `#4B3BC4` | 7,26 | 7,72 | AAA |
| verde «obvio» | `#12885A` | 4,21 | 4,47 | **falla AA, descartado** |
| ámbar «obvio» | `#A66508` | 4,40 | 4,68 | **falla AA, descartado** |

**Texto sobre chip teñido** —el caso que más se olvida, porque se comprueba el color
sobre el fondo y no sobre el chip que lleva encima—:

| | Chip resultante | Texto sobre chip |
|---|---|---|
| Verde, oscuro, 18 % sobre `#10182F` | `#1A3C43` | **6,91** |
| Ámbar, oscuro | `#393433` | **6,76** |
| Rojo, oscuro | `#3B2A3B` | **5,23** |
| Cian, oscuro | `#1E3D50` | **7,65** |
| Neutro, oscuro | `#2A344E` | **5,84** |
| Verde, claro, 12 % sobre blanco | `#E2EEE9` | **5,27** |
| Ámbar, claro | `#F1EAE1` | **5,25** |
| Rojo, claro | `#F4E5E4` | **5,99** |
| Neutro, claro | `#E9EBEF` | **5,95** |

**Componentes y foco** (WCAG 1.4.11 pide 3:1 en no-texto que identifica un control):

| Elemento | Ratio | ¿Cumple? |
|---|---|---|
| Borde de control, oscuro `#55679A` / `#10182F` | 3,18 | sí |
| Borde de control, claro `#7E8AA8` / `#FFFFFF` | 3,45 | sí |
| Anillo de foco, oscuro `#5EE7E7` / `#10182F` | 11,78 | sí, con mucho |
| Anillo de foco, claro `#0B5561` / `#FFFFFF` | 8,45 | sí |
| Botón primario `#06121A` sobre `#5EE7E7` | 12,68 | sí |
| Chip sólido sin-vhost, oscuro | 7,46 | sí |
| Chip sólido sin-vhost, claro | 7,32 | sí |
| Punto de estado verde, oscuro | 10,25 | sí |
| Punto de estado verde, claro | 6,28 | sí |

**Y lo que no llega, dicho en voz alta.** Los separadores de tabla (`#1E2A4A` sobre
`#10182F`, **1,24**; `#E3E8F2` sobre blanco, **1,23**) no cumplen 3:1 y no tienen que
cumplirlo: 1.4.11 se aplica a lo que *identifica* un componente, y una línea que separa
dos filas no identifica nada —quitarla no impide operar la tabla—. Los fondos de chip
respecto de la superficie están entre **1,19 y 1,54**, y tampoco: el chip no es el
identificador, lo son el texto y el glifo que van dentro, y esos cumplen. Digo los dos
casos porque un documento de accesibilidad que sólo enseña las cifras que salen bien no
es un documento de accesibilidad.

### 3.6 Tipografía

```css
:root {
  --font-ui:   "Inter", "Inter var", system-ui, -apple-system,
               "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, "SF Mono", "Cascadia Mono",
               Menlo, Consolas, monospace;

  --text-2xs: 0.6875rem; /* 11 px — sólo etiquetas de eje y unidades */
  --text-xs:  0.75rem;   /* 12 px — metadatos, chips, barra de estado */
  --text-sm:  0.8125rem; /* 13 px — tablas densas */
  --text-md:  0.875rem;  /* 14 px — cuerpo por defecto */
  --text-lg:  1rem;      /* 16 px — cuerpo de lectura, prosa de ayuda */
  --text-xl:  1.25rem;   /* 20 px — títulos de pantalla */
  --text-2xl: 1.75rem;   /* 28 px — cifras protagonistas (traffic, metrics) */
  --text-3xl: 2.5rem;    /* 40 px — sólo la cifra grande del despliegue */

  --lh-tight: 1.25;  --lh-normal: 1.5;  --lh-loose: 1.65;
  --ls-caps: 0.06em; /* versalitas de cabecera de tabla */
}
```

Inter porque es la del banner y del logotipo: la marca ya la eligió. Con
`system-ui` detrás, para que la primera ventana no espere a una fuente.

**La regla de la monoespaciada es una regla semántica, no estética:** *todo lo que viene
textualmente del servidor va en monoespaciada*. Nombres de aplicación, dominios,
nombres de release, SHAs, rutas, claves de entorno, órdenes, y la salida cruda de
cualquier error. Todo lo que escribe la interfaz —etiquetas, explicaciones, botones—
va en Inter. El resultado es que de un vistazo se distingue **qué es un dato del
servidor y qué es una opinión de la interfaz**, que es exactamente la distinción que un
producto así necesita fijar. Y hay un motivo práctico añadido: un nombre de release como
`20260805-041230` en proporcional es un churro ilegible, y en monoespaciada se lee por
grupos.

`font-variant-numeric: tabular-nums` en absolutamente todas las tablas y en todo número
que se refresque. Sin eso, la columna de CPU del monitor **baila horizontalmente cada
dos segundos** al pasar de `9,8%` a `12,4%`, y ese temblor es lo que hace que una tabla
que se refresca resulte desagradable sin que nadie sepa decir por qué.

Cabeceras de tabla en `--text-xs`, mayúsculas, `letter-spacing: .06em`, color
`--fg-faint`. Es la misma jerarquía que usa `orbit list` en el terminal, por la misma
razón de aprendizaje compartido.

### 3.7 Espaciado, radios, elevación, densidad

```css
:root {
  --sp-1: 4px;  --sp-2: 8px;  --sp-3: 12px; --sp-4: 16px;
  --sp-5: 20px; --sp-6: 24px; --sp-8: 32px; --sp-10: 40px; --sp-12: 48px;

  --r-xs: 3px;   /* chips, distintivos */
  --r-sm: 5px;   /* botones, campos */
  --r-md: 8px;   /* tarjetas, paneles */
  --r-lg: 12px;  /* modales */
  --r-full: 999px;

  --row-comfortable: 40px;
  --row-compact:     30px;
  --hit-min: 24px;    /* WCAG 2.2 · 2.5.8 */
  --hit-target: 32px; /* objetivo real de la aplicación */
}
```

Escala de 4 px, sin excepciones. Radios pequeños a propósito: esto es una herramienta de
sistemas, no una aplicación de consumo, y un radio de 16 px en una tabla de
infraestructura comunica algo que no es verdad. El único radio grande es el del modal,
porque un modal sí es un objeto flotante.

**La elevación se hace distinta en cada tema, y hay que decirlo porque es la
equivocación más habitual al portar un tema.** En claro, la elevación son sombras:
`--shadow-1/2/3`. En oscuro, **las sombras no se ven** —una sombra negra sobre un fondo
casi negro no es nada— así que la elevación se hace con **superficie más clara + borde
de 1 px**: `#0B1020` → `#10182F` → `#16203C`. Un panel oscuro que se eleva con sombra y
no con superficie parece plano y el usuario no sabe qué está encima de qué. Sólo el
modal conserva sombra en oscuro, y no para elevar sino para separarse del velo.

**Dos densidades, y no es un capricho.** 40 aplicaciones × 40 px = 1.600 px de lista, o
sea el doble de la altura útil de una ventana normal. Con la densidad compacta (30 px)
caben 28 filas en 900 px de alto. La preferencia es global, se recuerda, y —esto es lo
importante— **los objetivos de puntero no encogen con la fila**: los botones de acción
mantienen su caja de 24 px mínimo mediante `padding` negativo y área extendida, aunque
el glifo mida 14. Encoger la fila no puede significar encoger el objetivo.

### 3.8 La tabla, medida con cuarenta aplicaciones de verdad

En la ronda 1 dije que la densidad compacta estaba sin probar con cuarenta aplicaciones.
No hay usuarios que observar, pero sí hay un banco de cuarenta apps de cuatro tipos
corriendo `orbit` de verdad, y mirarlo cambia cuatro decisiones.

**Lo que dice el servidor sobre la forma de los datos:**

| Dato | Restricción real | De dónde sale |
|---|---|---|
| Nombre de app | `^[a-z0-9][a-z0-9._-]{0,39}$`, sin `..` | `_app_name_ok`. **Máximo 40 caracteres**, minúsculas, dígitos, `.`, `-`, `_` |
| Dominio | **sin límite** | Sólo se comprueba la forma, no la longitud |
| Alias | lista, típicamente 1 (`www.`) | En el banco, 40 de 40 tienen exactamente uno |
| Tipo | 9 valores | `static next node go bun deno php laravel python` |
| Peso del JSON | **311 bytes por app** | 12.452 bytes para 40 apps |

**Y lo que hace la tabla del propio Orbit**, que es el punto de partida honesto:
`printf "  %-16s %-8s %-30s %-7s %-9s %s"`. Dieciséis columnas para el nombre, ocho para
el tipo, treinta para el dominio. Pero `printf` con `%-16s` **rellena y no recorta**. Con
un nombre de 37 caracteres y un dominio de 48 —los dos perfectamente legales— la fila pasa
de 79 a **118 caracteres** y las columnas se desalinean por completo. Medido, no supuesto.

Eso no es una crítica a `orbit`: en un terminal la fila larga simplemente se ve larga y no
pasa nada. En una ventana con anchura fija sí pasa, y hay que decidir qué se recorta.

#### 3.8.1 La columna de aplicaciones: no cabe el dominio, y está bien

La lista vive en la columna de 280-360 px. Con 320 px y 12 px de margen a cada lado quedan
**296 px útiles**. A 13 px de JetBrains Mono el avance es de 0,6 em, o sea **7,8 px por
carácter**. El reparto:

```
[ ● 14px ][ 8 ][ nombre — flexible ][ 8 ][ tipo 6ch = 47px ]
  glifo               219 px ≈ 28 caracteres
```

Veintiocho caracteres. Los nombres reales de los ejemplos del propio repositorio son
`criticabits` (11), `estropealo` (10) y `brokenufo` (9); el banco usa cinco. **Veintiocho
cubre con holgura lo que la gente escribe y se queda a doce del máximo legal.**

De ahí sale la primera decisión, que no habría tomado a ojo: **el dominio no va en la
columna.** No cabe —`www.tienda-de-suplementos-deportivos.example.com` son 48 caracteres,
375 px— y meterlo obligaría a recortar el nombre, que es el identificador. El dominio vive
en el resumen rápido (`Espacio`), en el detalle y en el filtro, que **busca por nombre y
por dominio a la vez**. La tabla ancha con dominio existe, y es la del monitor.

**El recorte, cuando hace falta, es por el medio y no por el final.** Un nombre de 34
caracteres se pinta `tienda-de-suplement…s-2026`, no `tienda-de-suplementos-depo…`. El
motivo es que los nombres de este dominio se distinguen por el **sufijo**: `tienda-prod` y
`tienda-staging` comparten los ocho primeros caracteres, y una lista donde las dos filas
ponen `tienda-…` es una lista que no identifica nada. Lo mismo con los dominios en la tabla
ancha: `www.tienda…example.com` conserva el TLD, que es donde está la mitad del
significado.

#### 3.8.2 La densidad, con la aritmética delante

Ventana de 900 px de alto: 44 px de cabecera de la columna, 28 px de barra de estado, y
unos 60 px de los enlaces de servidor al pie. Quedan **≈ 768 px** para filas.

| Densidad | Fila | Filas visibles | De 40 apps |
|---|---|---|---|
| Cómoda | 40 px | **19** | 48 % |
| Compacta | 30 px | **25** | 62 % |

Ninguna de las dos enseña cuarenta aplicaciones sin desplazar, y perseguirlo sería una
mala meta: para que cupieran haría falta una fila de 19 px, donde no cabe un objetivo de
puntero de 24 px (§7.6). **Así que la densidad no se elige para que quepa todo, se elige
para reducir el desplazamiento**, y compacta gana un 30 % de filas visibles a cambio de
diez píxeles que el objetivo de puntero recupera con área extendida.

La decisión real es otra, y es de orden: **como no caben, lo que importa es qué cinco
filas están arriba.** El orden por defecto pone primero lo que requiere acción —
`sin vhost`, luego `parado`, luego `mantenimiento`, luego certificados a menos de diez
días— y después el resto por nombre. Con ese orden, las cuarenta aplicaciones sanas de un
servidor sano son cuarenta filas que nadie necesita leer, y las dos rotas están siempre en
los primeros 80 píxeles.

#### 3.8.3 Lo que enseñó el banco, y que no habría visto en una maqueta

En el banco, las cuarenta aplicaciones salen con **`served: false`**, porque no hay nginx
detrás. La lista renderizada es **una pared de cuarenta chips sólidos rojos**. Y ahí está
el hallazgo:

> **Cuando todas las filas comparten el mismo estado de alarma, el chip deja de
> discriminar y se convierte en fondo.** Cuarenta chips rojos no dicen «cuarenta
> problemas»: no dicen nada, porque no hay contraste entre filas. Y peor: la fila que sí
> sea distinta —la única app que además está parada— se pierde dentro del rojo.

Esto no es una hipótesis de accesibilidad, es lo que se ve al mirar la salida real. La
respuesta:

> **Si más del 70 % de las aplicaciones comparten un mismo estado que no es «activo», eso
> deja de ser N incidentes y pasa a ser uno.** La lista lo saca de las filas y lo pone en
> una banda arriba, y los chips de esas filas bajan a la variante tenue.

```
┌───────────────────────────────────────────────────────────────────┐
│ ⊘  Las 40 aplicaciones están sin vhost                            │
│    Esto no son 40 incidentes: es uno. nginx no está sirviendo     │
│    ninguna. Empieza por el diagnóstico.        [ Ir al diagnóstico ]│
└───────────────────────────────────────────────────────────────────┘
```

Es el mismo razonamiento que Orbit aplica a los avisos del vigilante —se avisa de la
transición, no del estado, «si una app se cae a las 3 de la mañana recibes un mensaje, no
300»— trasladado a una lista. Un aviso que aparece cuarenta veces se lee cero.

Dos observaciones más del mismo vistazo:

**La mitad de las apps tienen `service: null` y `port: null`.** Veinte de cuarenta, porque
veinte son estáticas o PHP. O sea que **el glifo `—` no es un caso raro: es la mitad de la
tabla.** Eso confirma la decisión de §3.4 de darle un tratamiento propio y neutro, y añade
una: la columna PUERTO, con veinte guiones de cuarenta, no se gana un sitio en la columna
estrecha. Va en el detalle.

**Y a 311 bytes por aplicación, cuarenta apps son 12 KB.** No hace falta virtualizar nada:
cuarenta filas en el DOM no son un problema para ningún navegador de esta década.
La virtualización se activa **a partir de 200 filas**, que en la práctica no ocurre en una
sola lista —ocurriría en una lista global multiservidor, que es justamente lo que §2.9
descarta—. Poner virtualización desde el principio complica el desplazamiento, la búsqueda
del navegador y el orden de foco para resolver un problema que estos datos dicen que no
existe.

#### 3.8.4 Nombres hostiles

Los nombres vienen del servidor, y el servidor los acepta con la forma de `_app_name_ok`.
Esa forma es restrictiva —minúsculas ASCII, dígitos y tres signos— pero **el cliente no
puede suponer que todo lo que llega la cumple**: puede hablar con un servidor comprometido
o con una versión futura. Tres reglas de renderizado:

1. **El nombre se pinta literalmente, nunca interpretado.** Un nombre que sea
   `</script><img src=x onerror=…>` se ve tal cual, como texto.
2. **Los caracteres de control y las marcas de dirección se aíslan y se marcan.** Un nombre
   con `U+202E` (anulación de derecha a izquierda) puede hacer que `gpj.exe` se lea
   `exe.jpg`. Se pinta con aislamiento bidireccional y con un distintivo `⚠ nombre inusual`
   junto a la fila.
3. **Un nombre que no cumple `_app_name_ok` marca la app como no operable**: se enseña, se
   marca, y sus botones de acción están desactivados con el motivo. Enseñarla y dejar
   actuar sobre ella sería peor que no enseñarla, y no enseñarla sería esconder algo que
   está pasando en el servidor.

---

## 4. Los estados difíciles

Esta sección es el 70 % del producto y suele ser el 5 % del esfuerzo de diseño. La
razón es que el estado feliz se diseña mirando una maqueta y los demás sólo aparecen
usando la cosa contra un servidor de verdad, un martes, con la red mal. Así que se
diseñan aquí, por escrito, antes.

### 4.1 Cargando, con los números reales

En la ronda 1 esta sección eran umbrales inventados sobre latencias estimadas. Ahora hay
medidas y hay una decisión de transporte, así que se rehace entera.

**Lo que se mide en el servidor** (banco de 40 apps, `orbit` v1.3.6, medianas):

| Comando | Servidor | Nota |
|---|---|---|
| `version --json` | **72 ms** | No lee ninguna app. Es el **suelo**: 13.720 líneas de Bash que se parsean en cada invocación. |
| `info <app> --json` | **86 ms** | Una app. 14 ms por encima del suelo. |
| `list --json` | **306 ms** | 40 apps. |
| `status --json` | **389 ms** | 40 apps **más** el host. Sólo un 27 % más que `list`, y trae estrictamente más. |
| `top --json` | **~1.000 ms** | Fijos, y no se pueden bajar: es la espera deliberada entre las dos lecturas de CPU. |
| `doctor --json` | segundos | Hace `nginx -t` y un `dig` por dominio. |
| `traffic --json` | segundos a decenas | Descomprime logs rotados. |

**Lo que cuesta el transporte.** El arquitecto decide `ssh -o ControlMaster=auto` con
`ControlPersist=120`, y eso parte la latencia en dos regímenes que hay que diseñar por
separado porque **no se parecen en nada**:

| | Primera llamada a un servidor | Llamadas siguientes (socket vivo) |
|---|---|---|
| Saludo SSH | 150-350 ms (y 400-700 con `ProxyJump`) | **~12 ms** |
| Suelo de `orbit` | 72 ms | 72 ms |
| Trabajo del comando | según la tabla | según la tabla |

Y de ahí salen **las cifras de extremo a extremo con las que se diseña**, que son las
que el usuario percibe:

| Pantalla | Comando | Primera vez | En caliente |
|---|---|---|---|
| Portada (servidor + apps) | `status --json` | **560-740 ms** | **~400 ms** |
| Detalle de app | `info --json` | 240-440 ms | **~100 ms** |
| Cambiar de app | `info --json` | — | **~100 ms** |
| Monitor, cada muestra | `top --json` | ~1,2 s | **~1,01 s** |
| Diagnóstico | `doctor --json` | 2-6 s | 2-6 s |
| Tráfico, 7 días | `traffic --json` | 1-20 s | 1-20 s |

**El régimen frío existe una vez por servidor y por sesión.** Después,
`ControlPersist=120` mantiene el socket dos minutos tras la última llamada, y el usuario
que está trabajando no vuelve a pagarlo. Esto cambia el diseño: **la única pantalla que
merece un tratamiento de "primera vez" es la portada al conectar**, y lo merece de
verdad, porque es la única que puede tardar tres cuartos de segundo.

**Los cuatro regímenes, ahora con los umbrales atados a esas cifras:**

| Umbral | Qué entra ahí | Qué se hace |
|---|---|---|
| **< 150 ms** | `info` en caliente (~100 ms), cambiar de app, cambiar de pestaña ya cargada | **Nada.** Ningún indicador, ninguna atenuación. Un parpadeo de 90 ms es peor que la espera. |
| **150 ms – 1,2 s** | portada en caliente (~400 ms), portada en frío (~700 ms), `info` en frío, una muestra de `top` | El dato anterior se conserva a `opacity: .55`, no interactivo, con una barra indeterminada de 2 px arriba del panel. |
| **> 1,2 s** | `doctor`, `traffic`, el alta de un servidor nuevo | Además, el control que lo disparó pasa a ocupado y la barra de estado enseña **la orden literal y el cronómetro**: `orbit doctor --json · vps-ovh · 2,4 s`. |
| **Sin dato previo** | primera carga de cualquier pantalla | Esqueleto **con la forma real** —5 filas con las columnas correctas—, no bloques grises. |

Dos afinados que salen de las medidas y que no habría sabido escribir sin ellas:

**La portada en caliente cae en el segundo tramo por poco (400 ms), y eso es exactamente
lo que había que evitar.** Cuatrocientos milisegundos es el peor sitio: se nota y no
justifica ceremonia. La respuesta no es un indicador más bonito, es **no volver a
pedirla**: `staleTime` de 15 s sobre `status`, y `refetchOnWindowFocus`. Volver a la
aplicación después de un rato pide una vez; moverse dentro de ella, ninguna. En la
práctica, el usuario que abre y mira paga 400 ms una vez cada quince segundos, y todo lo
demás es instantáneo.

**El monitor no entra en ningún régimen de carga, porque su espera es su dato.** Cada
muestra cuesta 1,01 s y el ciclo real es de unos 3 s (2 s de intervalo más el segundo que
tarda la medición). Es la única espera del producto cuya duración se conoce de antemano,
así que se pinta una barra exacta de 1 s con el motivo escrito al lado: «midiendo la
segunda lectura de CPU». Una espera que se explica deja de ser sospechosa.

**Lo que se quita respecto de la ronda 1.** El paliativo del *prefetch* de `info` al posar
el puntero 300 ms sobre una fila **sobra y se retira**. Estaba pensado para esconder una
latencia de 80 ms que resultó ser de ~100 ms en caliente, o sea por debajo del umbral de
percepción; y a cambio disparaba una llamada SSH por cada fila que el puntero cruzase al
recorrer una lista de cuarenta. Era optimizar lo que ya era instantáneo pagándolo con
tráfico que el usuario no pidió. **La precarga que sí se mantiene** es la del alta y la
reconexión: `version` y `status` en paralelo al conectar, que es donde está el régimen
frío de verdad.

**El reloj del dato**, sin cambios respecto de la ronda 1 y ahora mejor justificado: toda
superficie lleva su antigüedad (`hace 4 s`), y a partir de 60 segundos el reloj pasa de
`--fg-faint` a `--st-warn`. Con `staleTime` de 15 s y sin ningún sondeo de fondo (§6.8),
un dato de más de un minuto significa que el usuario no ha tocado nada o que el servidor
no contesta, y las dos cosas merecen que se note.

**Cancelar sigue significando matar el proceso remoto**, no ignorar la respuesta. Con las
cifras delante esto importa más de lo que parecía: un `traffic --since 90d` puede tener a
un VPS de 5 € descomprimiendo logs durante medio minuto, y dejarlo corriendo porque el
usuario cambió de pantalla es cobrarle a su servidor el coste de nuestra interfaz.

**Y una consecuencia incómoda que hay que decir:** en Windows, OpenSSH **no implementa
`ControlMaster`**. Todas las cifras de la columna «en caliente» dejan de existir y se
paga el saludo completo en cada llamada: la portada pasa de 400 ms a 700-1.000 ms, y el
detalle de app de 100 ms a 400-700 ms. Eso mueve dos pantallas enteras de un régimen a
otro. La interfaz no lo esconde: en Windows el umbral de atenuación baja de 150 ms a
100 ms, los `staleTime` se duplican, y la barra de estado dice **«sin multiplexado»** con
una explicación de una línea. Diseñar Windows como si fuera Linux y dejar que el usuario
concluya que la aplicación es lenta sería el resultado peor.

### 4.2 Vacío: hay cinco vacíos y no se parecen

Un solo componente de «no hay nada» sería un error. Estos son los cinco, con lo que
distingue a cada uno:

**Sin servidores configurados.** Es la primera pantalla de la vida de la aplicación.
Es la única con tono de bienvenida. Enseña lo que hace falta —un host, un usuario, una
clave que ya existe— y dice explícitamente que **no se instala nada en el servidor**,
porque es lo que la gente pregunta primero.

**Servidor sin aplicaciones.** `list --json` devuelve `{"apps":[]}` y eso significa lo
que dice. Ilustración discreta y un botón único: «Publicar tu primera web». Es el único
vacío alegre.

**Filtro sin resultados.** Distinto del anterior aunque la lista esté igual de vacía:
aquí hay 40 aplicaciones y el usuario ha escrito algo que no encaja. La acción es
«limpiar el filtro», y se dice cuántas hay en total. Confundir estos dos vacíos hace
que alguien crea que se le han borrado las apps.

**La lista está vacía porque el comando falló.** Esto **no es un vacío, es un error**, y
el contrato garantiza que se puede distinguir. USAGE es explícito: «una colección vacía
significa "no hay", nunca "no he podido preguntar"; si Orbit no puede obtener el dato,
aborta: error por stderr, código distinto de cero y nada por la salida normal». Así que
la interfaz mira el código de salida **antes** de mirar el JSON, y si es distinto de
cero pinta el error, nunca el vacío. Es una garantía regalada por el servidor y sería
un desperdicio no usarla.

**Aplicación sin releases.** Recién creada y aún sin desplegar. `releases: []` y
`last_deploy: null`. No es un error ni un vacío triste: es un estado legítimo de una
app nueva, y la acción es desplegar.

### 4.3 SSH caído: «error de conexión» no ayuda a nadie

Un mensaje genérico convierte un problema de treinta segundos en una tarde. La
taxonomía real, cada una con su causa, su mensaje y su acción:

| Síntoma (stderr) | Qué pasa | Acción primaria |
|---|---|---|
| `Could not resolve hostname` | El nombre no existe o el DNS local falla | Editar el host |
| `Connection refused` | Hay máquina, no hay sshd en ese puerto | Revisar puerto (¿22?) |
| `Connection timed out` | Cortafuegos o máquina apagada | Reintentar / ver proveedor |
| `Permission denied (publickey)` | La clave no la acepta el servidor | Ver qué clave se ofreció |
| `Enter passphrase for key` | La clave está cifrada y el agente no la tiene | **«Añadir al ssh-agent»** |
| `REMOTE HOST IDENTIFICATION HAS CHANGED` | Clave de host distinta | Aviso de seguridad, aparte |
| `ProxyJump` falla antes | El salto intermedio no responde | Nombrar el salto que falló |
| `sudo: a password is required` | Es permisos, **no** es SSH | §4.5 |

Cuatro reglas transversales:

**El stderr crudo siempre está, a un clic.** Un desplegable «Ver la salida» con el texto
tal cual, monoespaciado y seleccionable. La primera acción de cualquiera ante un error
es copiarlo para buscarlo o pegarlo en un *issue*, y una interfaz que se lo impide está
estorbando.

**La aplicación no se vacía cuando cae la conexión.** Se queda con lo último que sabía,
con el reloj del dato en ámbar y una banda que dice desde cuándo. Vaciar la pantalla
castiga al usuario por un fallo de red y además destruye información que seguía siendo
útil para diagnosticar.

**El rail lo marca.** El servidor pasa a gris con un punto rojo, y el resto de
servidores siguen funcionando. Es la ventaja regalada del multiservidor sin plano de
control: que uno caiga no afecta a los demás.

**Reconexión con reintento espaciado** —1 s, 2 s, 4 s, 8 s, hasta 30 s— con el próximo
intento visible («reintento en 6 s») y un botón para forzarlo ya. Un reintento
silencioso hace que la gente cierre y abra la aplicación por si acaso.

**Y el caso de la clave de host cambiada tiene su propio modal**, es el único error que
bloquea, y **no lleva un botón fácil de aceptar**. Lleva las dos huellas —la esperada y
la recibida—, el recordatorio de que esto es lo que se ve cuando alguien se interpone, y
la orden `ssh-keygen -R <host>` para copiar y ejecutar a mano si el usuario está seguro
de que reinstaló el servidor. La interfaz no toca `known_hosts`. Que resolverlo cueste
un minuto es exactamente el punto.

### 4.4 Contrato incompatible

`orbit version --json` devuelve `{"version":"1.3.6","contract":1}` y son dos cosas
distintas a propósito: Orbit sube de versión sin que el contrato cambie, y un cliente
que las confundiera se negaría a hablar con un servidor perfectamente compatible.

| Caso | Qué se hace |
|---|---|
| `contract` == el conocido | Normal. |
| `contract` > el conocido | **Funciona, con banda ámbar.** Los campos se añaden, nunca se renombran: todo lo que el cliente sabe leer sigue estando. Se avisa de que el servidor sabe cosas que esta versión no enseña y se ofrece actualizar. |
| `contract` < el conocido | **Funciona, con capacidades recortadas.** Se ocultan —no se deshabilitan con un icono triste— las pantallas cuyo comando no existe todavía en ese servidor. |
| `contract` ≥ el conocido + 2 | Modo sólo lectura con aviso más fuerte: hay margen para que algo haya cambiado de verdad. |
| Sin `orbit` / no ejecutable | Pantalla propia: «Este servidor no tiene Orbit», con la orden de instalación copiable. |
| Salida que no es JSON | Casi siempre un `.bashrc` que imprime algo. Se enseña el texto recibido y se dice esa hipótesis en concreto, porque acierta el 90 % de las veces. |

**Banda persistente, nunca modal.** Un modal impediría leer todo lo que sí funciona,
que es casi todo. La banda va anclada arriba del panel principal, es descartable por
sesión y vuelve al reconectar.

### 4.5 Permiso denegado

`orbit` se auto-eleva a root. Por SSH se entra normalmente como un usuario con `sudo` o
como root, y cuando `sudo` pide contraseña por *tty* el comando muere de una forma que
parece un fallo de Orbit y no lo es.

El mensaje nombra las tres cosas que hacen falta para arreglarlo —**usuario, host y qué
falta exactamente**— en vez de decir «error de permisos»:

> `dave@vps-ovh` no puede elevar a root sin contraseña, y por SSH no hay dónde
> escribirla. Opciones: entrar como `root`, o dar a este usuario una regla de `sudoers`
> sin contraseña para `/usr/local/bin/orbit`.

Y debajo, con el mismo tono con el que lo dice el ROADMAP, la honestidad que
corresponde: una regla de `sudoers` limitada a `orbit` **sigue siendo equivalente a
root**, porque `orbit exec` existe. Prometer lo contrario en una interfaz sería vender
una seguridad que no hay.

### 4.6 `served:false`: el estado con nombre propio

Es el estado más importante del producto y el que ninguna interfaz genérica pintaría
bien, porque en cualquier otro panel «la app está corriendo» sería una buena noticia.
Aquí puede ser irrelevante: hay proceso, hay puerto, hay certificado, y **nginx no
tiene el vhost**, así que la petición cae en el servidor por defecto y el visitante
recibe la conexión cerrada. Ni 404 ni 502. `curl` dice `000`.

En la lista: chip sólido `sin vhost`, y un borde izquierdo de 3 px del mismo color en
toda la fila. La ordenación por defecto lo pone arriba del todo, por encima incluso de
las apps paradas.

En el detalle, una banda que explica el mecanismo en vez de dar un código:

```
┌───────────────────────────────────────────────────────────────────┐
│ ⊘  nginx no está sirviendo este dominio                           │
│                                                                   │
│    No existe el vhost de tienda.com (ni el fichero ni el enlace). │
│    Una petición a ese dominio la atiende el servidor por defecto  │
│    y se corta: el visitante no recibe ni un 404 ni un 502.        │
│    La página de mantenimiento tampoco se sirve.                   │
│                                                                   │
│    Se arregla con `sudo orbit doctor --fix` en el servidor.       │
│    [ copiar la orden ]        [ Ir al diagnóstico ]               │
└───────────────────────────────────────────────────────────────────┘
```

Y la regla que traduce literalmente lo que dice USAGE —«ningún otro campo del estado
describe lo que está recibiendo»—:

> **Con `served:false`, el resto de facetas del estado se atenúan al 55 % y llevan un
> tooltip que dice por qué.** El servicio puede estar activo y el certificado ser
> válido, y ninguna de las dos cosas le sirve de nada a un visitante. Enseñarlas con el
> mismo peso que en una app sana sería contradecir la banda que hay justo encima.

### 4.7 `cpu_percent: null`, y por qué no es cero

La CPU es la diferencia entre dos lecturas de `CPUUsageNSec`. La primera vez no hay
diferencia. Así que **no hay porcentaje**, y el contrato lo dice con `null` en vez de
inventar un `0`, porque un cero es una afirmación.

En la tabla: `·` en `--st-unknown`, con tooltip «Todavía no se sabe: el porcentaje es la
diferencia entre dos lecturas y ésta es la primera». En el monitor en vivo la segunda
lectura llega sola al fotograma siguiente, así que el `·` dura dos segundos y
desaparece; en una foto suelta se queda.

En la gráfica de la barra lateral —el *sparkline* de CPU— **un `null` es un hueco, no un
cero**. La línea se corta, se pinta un tramo punteado del ancho de la muestra que falta
y se continúa. Dibujarlo como cero pintaría una caída a plomo y una recuperación que no
han ocurrido nunca, y ésa es exactamente la clase de mentira que el contrato se
esforzó en no decir. Es la razón principal por la que en §8.4 no uso una librería de
gráficas: casi todas interpolan sobre el `null` o lo tratan como cero.

Y la distinción que hay que mantener a toda costa, con dos glifos distintos:

| | Glifo | Significa |
|---|---|---|
| `cpu_percent: null` | `·` | **No se sabe.** Puede haber CPU, no la hemos medido. |
| `service: null`, MEM de una estática | `—` | **No aplica.** No hay proceso. No hay nada que medir. |

Los mismos dos caracteres que usa `orbit top`.

### 4.8 `requests_capped: true`

Las peticiones se cuentan sobre las últimas 5.000 líneas del log, no sobre el fichero
entero, porque esto se refresca cada dos segundos y el log de una web con tráfico son
cientos de megas. Cuando un minuto llena el tope, el número sale con un `+`.

El diseño es una sola regla y es de contención: **el `+` va del mismo tamaño y del mismo
color que el número**. `5000+`, no `5000⁺` en gris pequeño. La tentación de hacerlo
discreto es fuerte y es exactamente el error: un número corto sin avisar se lee como
«hay poco tráfico», que es lo contrario de lo que está pasando. El tooltip explica el
tope y nombra `TOP_LOG_LINES`, para quien quiera subirlo.

Si además el log no lleva marca de tiempo —servidores anteriores a agosto de 2026—, la
columna entera es `·` y hay una nota de una línea con la solución (`orbit nginx-rebuild`)
y su matiz honesto: las líneas ya escritas siguen sin fecha, no se pueden inventar.

### 4.9 `complete: false` en tráfico

Se pidieron 30 días y logrotate se lleva los logs a los 14. La respuesta es más pequeña
que la pregunta, y el contrato lo publica.

```
┌──────────────────────────────────────────────────────────────────┐
│ ⚠  La ventana está recortada                                     │
│    Pediste 30 días. El log cubre 13 días y 4 horas, desde el 17  │
│    de agosto. Lo que ves abajo es esa ventana, no 30 días.       │
│                                        [ Ver 14 días en su lugar ]│
└──────────────────────────────────────────────────────────────────┘
```

Dos alternativas descartadas, y las dos son formas de mentir:

- **Pintar 30 días con 17 vacíos.** Miente por omisión: la gráfica sugiere que hubo
  diecisiete días sin tráfico.
- **Pintar 13 días y llamarlos «los últimos 30».** Miente por silencio, que es peor,
  porque el número parece una respuesta.

El eje temporal se etiqueta con la ventana **real**, y la agrupación se dice: por horas
hasta dos días, por días a partir de ahí, y la etiqueta lo indica. Una barra que unas
veces es una hora y otras un día, sin avisar, es peor que no dibujarla.

**Y `automated` nunca se suma al número grande.** En un VPS con IP pública buena parte
del tráfico son escáneres buscando `/.git/config`. El diseño lo pone en dos series
visualmente distintas y una jerarquía inequívoca:

```
   Peticiones                       Automáticas
      371                          13 526  (97 %)
   ▔▔▔▔▔▔▔▔▔▔▔▔▔▔                  ▔▔▔▔▔▔▔▔▔▔▔▔▔▔
   --text-3xl, --fg               --text-lg, --fg-muted, trama diagonal
```

En la gráfica por horas, dos series apiladas donde la automática lleva **trama
diagonal además de color**, para que la distinción sobreviva a una captura en blanco y
negro. Y un pie que dice, cada vez, lo que Orbit dice cada vez: **son IPs, no personas**.
Sin cookies no hay forma de distinguir dos pestañas de dos visitantes. Es la frase que
hace que el número que el usuario le manda a su cliente sea defendible.

### 4.10 Los seis finales de `deploy --all`

Confundir dos de ellos ya costó un fallo real: «sin cambios» y «no he podido preguntar»
valían lo mismo, y un remoto caído se anunciaba como «nada que hacer» cada cinco
minutos. El contrato existe para que un cliente no lo repita, así que aquí van los seis
con color, glifo, texto y acción, y **ningún par comparte los tres primeros**:

| Final | Glifo | Color | Texto | Acción primaria |
|---|---|---|---|---|
| `deployed` | `✓` | `--st-ok` | desplegada | ver el despliegue |
| `failed` | `✕` | `--st-error` | ha fallado | ver el error (paso + log) |
| `unchanged` | `=` | `--st-unknown` | sin cambios | — |
| `unreachable` | `⚠` | `--st-warn` | **sin contacto con el remoto** | ver el `fatal:` de git |
| `gone` | `⌀` | `--st-warn` (borde `--st-error`) | la rama ya no existe | revisar la rama configurada |
| `skipped` | `⇥` | `--st-unknown` (borde ámbar) | saltada: este commit ya rompió | ver el fallo anterior |

Y la regla de la cabecera:

> **Nunca un recuento agregado sin los seis desglosados al lado.** Está prohibido
> «10 correctas / 2 fallidas».

```
  12 aplicaciones · 8 desplegadas · 2 sin cambios · 1 sin contacto · 1 fallida
  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░▒▒▒░░░  1 min 47 s
```

`unchanged` y `unreachable` no pueden compartir ni color, ni glifo, ni celda de
recuento, ni orden en la lista. Es la única restricción de este documento que viene de
un bug con nombre, y por eso es la que menos se puede relajar.

`ok` del objeto de lote sigue **la misma regla que el código de salida** —ni `failed`,
ni `unreachable`, ni `gone`—, y la interfaz usa ese campo y no recalcula el suyo, para
que un usuario que mire la pantalla y un script que mire el código de salida no puedan
discrepar nunca.

### 4.11 La respuesta que no corresponde a la pregunta

Un estado que no había contemplado y que es barato de tratar: **comprobar que lo que ha
llegado contesta a lo que se preguntó.** El contrato lo permite porque el campo `app`
viaja en `info`, `env list`, `deploy`, `metrics` y `traffic`.

| Caso | Qué se hace | Qué no se hace |
|---|---|---|
| Se pidió `info tienda` y vuelve `"app":"blog"` | Error: «la respuesta no es de lo que se preguntó» | Pintarla como si fuera de `tienda` |
| Se pidió `list` y vuelve la forma de `status` | Error de forma | Leer los campos que casualmente coincidan |
| Llega un `{"event":"step","app":"otra"}` en el progreso | Se ignora o se marca; **no** se atribuye a la app actual | Sumarlo a la barra |
| Basura en stdout antes del JSON | Error | Recortar hasta la primera llave |
| Dos objetos JSON en stdout | Error | Quedarse con uno de los dos |

Es la clase de comprobación que sale gratis hasta el día que no. Y visualmente tiene una
regla propia, porque el error es raro y desconcertante: **no se pinta como un error de
red**. Se pinta con su propio texto —«el servidor ha contestado otra cosa»— y con las dos
salidas literales, la pedida y la recibida, porque quien vea esto va a querer entender qué
pasó y no va a poder si sólo le decimos que algo falló.

### 4.12 El modo presentación

No es un estado difícil, es una tecla que los evita todos a la vez. Un interruptor visible
—no enterrado en preferencias— que **oculta de golpe valores revelados, dominios,
hostnames e IPs** y los sustituye por marcadores de la misma anchura, para que la interfaz
no cambie de forma. Se activa antes de compartir pantalla en una videollamada, o antes de
hacer una captura para un *issue*.

Vale la pena porque los nombres de dominio de una agencia **son** su cartera de clientes, y
porque el escenario en que se filtran no es un ataque: es enseñarle a alguien cómo funciona
esto. Cuesta un interruptor y una clase de CSS.

Con el modo activo, la barra de estado lo dice permanentemente. Un modo que oculta datos y
no se anuncia es un modo en el que alguien acaba leyendo mal la pantalla.

---

## 5. Movimiento y animación

### 5.1 El presupuesto

```css
:root {
  --dur-instant: 90ms;   /* realimentación de pulsación */
  --dur-fast:   120ms;   /* fundidos, cambio de pestaña */
  --dur-base:   180ms;   /* aparición de elementos pequeños */
  --dur-slow:   260ms;   /* paneles, cambios de estado */
  --dur-panel:  220ms;   /* inspector lateral */
  --dur-modal:  200ms;

  --ease-out:  cubic-bezier(.20, 0, 0, 1);      /* entradas: arranca rápido, posa */
  --ease-in:   cubic-bezier(.40, 0, 1, 1);      /* salidas: se va sin ceremonia */
  --ease-both: cubic-bezier(.40, 0, .20, 1);
}
```

Regla dura: **nada por encima de 260 ms**, con una sola excepción, el barrido del
logotipo al abrir la aplicación, que dura 500 ms y ocurre **una vez por sesión**. Esa
excepción y su límite los copio de `orbit`, que ya tomó exactamente esta decisión:
ninguna animación pasa de medio segundo, y la del menú sólo se dibuja en la primera
vuelta, porque «volver del diagnóstico por décima vez y tener que esperar otra vez
convertiría un detalle en un peaje».

### 5.2 Qué se anima y qué comunica cada cosa

Ninguna animación entra en el producto sin una frase en esta columna de la derecha.

| Elemento | Duración / curva | Qué comunica |
|---|---|---|
| Panel lateral | 220 ms, deslizar 24 px + fundir, `--ease-out` | «Sigue habiendo una lista detrás; esto está encima, no en lugar de». |
| Modal | 200 ms, escala 0,97→1 + velo | «Esto bloquea; hay que resolverlo». |
| Cambio de pestaña | 120 ms **sólo de opacidad**, 0 ms de posición | La posición es información; deslizar el contenido rompería la memoria muscular. |
| Chip que cambia de estado | 260 ms de color + un pulso **único** de borde | «Esto ha cambiado ahora mismo, no estaba así». |
| Fila nueva en la lista | 180 ms de altura + fundido | «Ha aparecido algo, no te lo has perdido». |
| Fila que desaparece | 120 ms, `--ease-in` | Se va sin pedir atención. |
| Barra de progreso del despliegue | interpolación continua, monótona | El único movimiento que es **dato**, no decoración. |
| Toast | 160 ms entra / 120 ms sale | Transitorio y sin importancia. |
| Rail: cambio de servidor | 180 ms de indicador, contenido sin transición | El indicador se mueve, el contenido cambia de golpe: son otros datos. |
| Barrido del logotipo | 500 ms, una vez por sesión | Identidad. Y una sola vez. |

### 5.3 Qué NO se anima, y por qué

Esta lista importa más que la anterior.

**Los números que se refrescan cada dos segundos.** CPU, memoria, peticiones por
minuto: cambian de golpe. Un número con *tween* de 300 ms que se refresca cada 2.000 ms
pasa el **15 % del tiempo mostrando una cifra que no es ninguna de las dos**. En una
aplicación de monitorización eso no es pulido, es ruido con aspecto de precisión.

**El reordenado de la tabla del monitor.** Y va más lejos: **la tabla del monitor no se
reordena sola**. Si el usuario la ordena por CPU y las filas se reordenan cada dos
segundos, hacer clic en algo se convierte en un juego de puntería. Lo que se hace en su
lugar: el orden se congela mientras el puntero está encima de la tabla, y si el orden
«real» ha cambiado aparece un botón discreto de «reordenar» con el número de cambios
pendientes. Es la misma solución que usan los gestores de tareas que se dejan usar.

**Las gráficas de tráfico al cambiar de ventana.** Se redibujan, no se transicionan.
Pasar de 24 h a 7 d no es que los mismos datos se muevan: son otros datos. Animar la
transición sugiere una continuidad que no existe.

**Los errores.** Aparecen ya, sin fundido. Un error con `fade-in` de 200 ms es un error
que llega tarde, y además invita a que el usuario haga clic en lo que había debajo.

**El esqueleto de carga.** Nada de barrido de brillo. Es una convención heredada de la
web y aquí sólo añadiría movimiento a una pantalla donde ya hay una barra de progreso
diciendo lo mismo mejor.

### 5.4 El *spinner* infinito está prohibido

Un despliegue tarda entre veinte segundos y cuatro minutos. Una rueda girando durante
cuatro minutos no informa de nada y, peor, no distingue «esto va bien y tarda» de «esto
se ha colgado». La regla:

> **Si no se sabe cuánto falta, no se finge que se sabe; pero siempre se dice qué se
> está haciendo y cuánto lleva.** Un contador de segundos que sube es más honesto y más
> tranquilizador que cualquier animación.

Concretamente, `build · 1 min 12 s` transmite dos cosas que ninguna rueda transmite: que
el paso es el build (y el build es el que tarda) y que sigue vivo. Si el contador se
para, se ve inmediatamente.

Barra determinista **sólo donde hay denominador**:

| Superficie | ¿Hay denominador? | Qué se pinta |
|---|---|---|
| `deploy` | Sí: 6 pasos conocidos, con pesos de `metrics` | Barra ponderada (§6.2) |
| `deploy --all` | Sí: N apps conocidas de antemano | Barra por apps + barra interna |
| `doctor` | No: no publica progreso | Contador + «esto tarda 2-4 s, hace `nginx -t` y consultas de DNS» |
| `traffic` largo | No | Contador + la ventana pedida |
| `new` | **No**: no tiene `--json` ni `--progress` | Contador + los cinco pasos conocidos, **sin porcentaje**, marcando el que va por lo último visto en stderr. §2.6.4 |
| `top` (primera lectura) | Sí, y es fijo: ~1 s | Barra de 1 s, con el motivo escrito |

Ese último caso es un pequeño hallazgo: la espera de `top --json` es la única de todo
el producto cuya duración se conoce de antemano, porque es una espera deliberada del
servidor para tomar la segunda muestra de CPU. Se puede pintar una barra exacta y
además explicar por qué existe. Una espera que se explica deja de ser una espera
sospechosa.

### 5.5 `prefers-reduced-motion`

No es apagar todo indiscriminadamente; es distinguir lo que es decoración de lo que es
dato.

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: .01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: .01ms !important;
    scroll-behavior: auto !important;
  }
  /* Excepción: la barra de progreso es información, no adorno. */
  .progress__fill { transition-duration: var(--dur-slow) !important; }
}
```

Qué cambia exactamente:

- **Transform y opacity a 0 ms.** Paneles y modales aparecen, no entran.
- **La barra de progreso sigue avanzando.** Es dato. Congelarla sería quitar
  información a quien pidió que no le movieran la pantalla, que no es lo que pidió.
- **El pulso del chip de estado** se sustituye por un borde de 2 px que permanece
  3 segundos y luego desaparece sin transición. El cambio se sigue viendo; no parpadea.
- **El barrido del logotipo** no se dibuja.
- **El desplazamiento automático de la consola de despliegue** pasa de suave a
  instantáneo, y sigue funcionando.

Y, como en Orbit, **un ajuste propio en Preferencias** que apaga el movimiento aunque el
sistema no lo pida. Es el espejo de `UI_ANIM="no"` de `orbit.conf`: quien ya tomó esa
decisión en el servidor espera poder tomarla también aquí, y hay gente que no configura
la preferencia del sistema porque afecta a todo lo demás.

---

## 6. El despliegue en vivo

Es la pantalla estrella y la única que tiene un usuario mirándola fijamente durante
minutos. Es también donde el contrato da más de lo habitual: NDJSON por stderr con
`--progress`, y un objeto final que contesta también cuando falla.

### 6.1 Por qué es una pantalla, y por qué se puede abandonar

Ya se dijo en §2.4: un modal de cuatro minutos secuestra la aplicación. Siendo pantalla,
el despliegue **sobrevive a que el usuario se vaya a otra parte**. Mientras corre:

- La fila de la app en la columna lleva un indicador vivo (un anillo de progreso de
  12 px, no un *spinner*).
- El servidor en el rail lleva un contador de trabajos activos.
- Volver es un clic, y `⌘⇧D` va directo al despliegue en curso.
- **Se pueden lanzar varios**, en apps distintas y en servidores distintos. Cada uno es
  un proceso SSH independiente; nada en el servidor los coordina, ni falta.

Cerrar la ventana **no cancela el despliegue** —el proceso remoto sigue— y eso se dice
al cerrar si hay uno vivo. Cancelar de verdad es matar el proceso, y eso se ofrece con
su advertencia: un despliegue interrumpido a mitad del build no deja nada roto (la
release nueva se descarta y `current` no se ha movido), pero interrumpido en `service` o
`nginx` sí puede dejar trabajo a medias. Se dice cuál es el caso según el paso en curso.

### 6.2 Anatomía

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  ← Aplicaciones          tienda · vps-ovh                                     │
│                                                                               │
│  Desplegando  tienda                                          1 min 47 s      │
│  a1b2c3d  «arregla el cálculo del IVA en el carrito»          main            │
│                                                                               │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░  68 %     │
│                                                                               │
│   ✓ code       3 s      actualizar el clon de git                             │
│   ✓ release    2 s      copiar a releases/20260830-114302                     │
│   ◐ build   1 m 38 s    compilar                          ← estás aquí        │
│   ○ activate            mover el symlink current                              │
│   ○ service             reiniciar y esperar al health check                   │
│   ○ nginx               recargar nginx                                        │
│                                                                               │
│  ┌─ Salida ────────────────────────────────── [ crudo ] [ ⤓ ] [ ↧ seguir ] ─┐ │
│  │ 11:43:05  code      empieza                                             │ │
│  │ 11:43:08  code      hecho en 3 s                                        │ │
│  │ 11:43:08  release   empieza                                             │ │
│  │ 11:43:10  release   hecho en 2 s                                        │ │
│  │ 11:43:10  build     empieza                                             │ │
│  │ …                                                                        │ │
│  └──────────────────────────────────────────────────────────────────────────┘ │
│                                                                               │
│                                        [ Cancelar ]      [ Ver los logs ]     │
└───────────────────────────────────────────────────────────────────────────────┘
```

Cuatro decisiones dentro de esta pantalla:

**Los seis pasos se enseñan desde el principio, con los que faltan en gris.** El
contrato los tiene fijos —`code`, `release`, `build`, `activate`, `service`, `nginx`— y
enseñarlos todos convierte una espera opaca en un recorrido con final visible. Es la
diferencia entre «está pasando algo» y «faltan tres cosas».

**Cada paso lleva su propio cronómetro, y se conserva al terminar.** El usuario aprende
en tres despliegues cuánto tarda su build, y a partir del cuarto sabe si algo va raro
sin que nadie se lo diga. Es observabilidad regalada.

**La barra está ponderada y no es lineal.** Seis pasos no valen un sexto cada uno:
`build` es típicamente el 70-85 % del tiempo. Los pesos salen de `orbit metrics <app>
--json`, que da `build_median_s` y la duración total: peso del build =
`build_median_s / mediana_total`, el resto repartido proporcionalmente a lo observado.
Si hay menos de seis builds, `build_trend_s` es `null` y **las métricas no bastan**;
entonces se usan pesos por defecto (`code` 5 %, `release` 5 %, `build` 70 %, `activate`
3 %, `service` 12 %, `nginx` 5 %) y la barra se pinta con trama sutil para decir que es
una estimación sin histórico. Que el servidor se calle la tendencia con menos de seis
builds es una decisión suya que la interfaz respeta en vez de rellenar.

**Y la barra tiene su propio reloj.** `elapsed_s` viene en segundos enteros, y los
eventos llegan cuando llegan: si la barra sólo se moviera con cada evento, estaría
quieta 90 segundos durante el build y luego daría un salto. Así que dentro de un paso
la barra avanza sola con `requestAnimationFrame`, aproximándose asintóticamente al final
de ese paso sin llegar nunca a él, y salta al 100 % del paso cuando llega el `ok`. Con
una regla: **monótona creciente, nunca retrocede**. Una barra que retrocede destruye
más confianza que cualquier error.

### 6.3 Cómo se lee el NDJSON, y cómo no se rompe

```
{"event":"step","app":"tienda","step":"build","status":"start","elapsed_s":5}
{"event":"step","app":"tienda","step":"build","status":"ok","elapsed_s":103}
{"event":"app","app":"tienda","status":"deployed","elapsed_s":107}
```

Tres reglas de robustez, todas derivadas de que **los campos se añaden, nunca se
renombran**:

1. **Un `event` desconocido no rompe nada.** Se ignora para la barra y aparece en la
   vista cruda. Un cliente que se caiga porque el servidor añadió un evento es un
   cliente que rompe cada vez que Orbit mejora.
2. **Un `step` desconocido se añade a la lista al final** en vez de descartarse, y la
   barra se recalcula. Si algún día hay un séptimo paso, la pantalla lo enseña sin
   actualizar el cliente.
3. **`app` se usa siempre para atribuir**, incluso en un despliegue de una sola app.
   Es el campo que se añadió justo para que un lote pueda mezclar niveles por el mismo
   canal, y usarlo desde el principio evita tener dos analizadores.

La vista cruda enseña las líneas tal cual, sin traducir, con un botón de copiar todo.
Es lo que se pega en un *issue*.

### 6.4 El final: un objeto, cuatro finales visuales

El objeto de `deploy` no tiene un campo «resultado» con cuatro valores; tiene `ok`,
`rolled_back` y `recovered`, y la combinación da cuatro finales que **se ven distintos
porque son distintos**:

**1 · `ok:true`, todo lo demás falso.** Verde. Una línea: «Desplegada en 1 min 51 s.»
Release nueva, commit, y un enlace a la web. La pantalla se puede cerrar sin leer nada
más, que es lo que se hace el 90 % de las veces.

**2 · `ok:true`, `recovered:true`.** Verde con una nota ámbar. Copy:

> Salió bien **al segundo intento**. Orbit reconoció el fallo del build, lo arregló y
> reintentó una vez. El arreglo queda apuntado en la configuración de la app, así que
> el próximo despliegue sale a la primera.

Esto no es un error y no puede pintarse rojo. Pero tampoco puede pintarse exactamente
igual que un éxito limpio, porque entonces se pierde una señal útil: cuatro `recovered`
seguidos son un patrón que alguien debería mirar. Verde con nota es el punto medio, y
es la traducción exacta de por qué el contrato tiene ese campo.

**3 · `ok:false`, `rolled_back:true`.** Rojo, **y la primera línea no habla del fallo**:

> El despliegue ha fallado en el paso `service`. **Tu web sigue en pie**, sirviendo la
> versión anterior `20260805-041230`.

Es lo único que el usuario necesita saber en el primer segundo, y ponerlo debajo del
volcado del error sería enterrar la buena noticia. Debajo, el error y las acciones.

**4 · `ok:false`, `rolled_back:false`.** Rojo, y aquí sí hay que mirar. `failed_step`
dice dónde, y de eso depende **qué se enseña abierto**:

| `failed_step` | Qué se enseña expandido | Acción primaria |
|---|---|---|
| `code` | El primer `fatal:` de git, no la última línea | Ver la configuración del repo |
| `release` | Espacio en disco y permisos | Ir al Doctor |
| `build` | **El final del log del build, no el principio** | Ver el log completo |
| `activate` | Estado del symlink `current` | Ir al Doctor |
| `service` | Journal del servicio, últimas 40 líneas | **Volver a `previous`** |
| `nginx` | Salida de `nginx -t` | `doctor --fix` |

El caso de `build` merece énfasis: **se abre el final del log, no el principio**. Un
error de compilación está en las últimas veinte líneas, y una consola que se abre por
arriba obliga a desplazarse mil líneas hasta el sitio donde está la respuesta. Es un
detalle de tres líneas de código que cambia por completo la sensación de la pantalla.

### 6.5 El rollback

`previous` viene en el objeto precisamente para poder ofrecerlo sin una segunda llamada.
La regla de cuándo aparece el botón:

```
mostrar «Volver a previous»  ⟺  previous !== null  &&  rolled_back === false
```

Si `rolled_back` ya es `true`, la vuelta atrás **ya ocurrió**: ofrecerla otra vez sería
ofrecer lo que acaba de pasar, y el usuario haría clic pensando que hace falta.

El botón no es primario. La acción primaria en un fallo es casi siempre **entender**, y
el rollback está un escalón por debajo, con la misma prominencia que «ver el log». Y va
detrás de una hoja de comando que enseña `orbit rollback tienda 20260805-041230` y que
avisa de lo que cuesta: reinicia el servicio y recarga nginx, con uno o dos segundos sin
respuesta.

Descartado: rollback de un clic, sin confirmación, como acción principal del error.
Motivo doble. Uno, es una acción con corte de servicio. Y dos, en buena parte de los
casos **no es lo que se quiere**: si el build falló, `current` ni se movió y no hay nada
que deshacer; si falló el health check, a veces lo correcto es arreglar el código y
volver a desplegar, no dejar la versión vieja puesta y olvidarse. Un botón grande y
rojo empujaría hacia la opción que parece más segura y a menudo no lo es.

### 6.6 `rolled_back` frente a `recovered`, dicho a la cara

Son las dos cosas que un panel tiene que poder enseñar distinto de un despliegue normal,
y sólo una es mala. La distinción se sostiene con tres piezas y no con una:

| | `recovered` | `rolled_back` |
|---|---|---|
| Glifo | `↻✓` reintento con éxito | `↩` vuelta a la anterior |
| Color | verde con nota ámbar | rojo |
| Frase | «arreglado y reintentado, salió» | «salió mal, se volvió atrás» |
| En la historia | punto verde con anillo ámbar | punto rojo con flecha |

Y donde de verdad rinden es en la **línea de tiempo de despliegues** de la pestaña
Despliegues, que se lee de un vistazo:

```
  ●───●───◍───●───●───◍───◍───◍───⊗───●
                              ↑ tres recuperados seguidos
```

Tres o cuatro `recovered` seguidos son un problema que ningún despliegue individual
señala, porque todos terminaron con `ok:true`. La línea de tiempo lo hace visible, y
encima de ella hay una nota cuando ocurre: «Los últimos 3 despliegues necesitaron
recuperación del build. Mira `A_PNPM_ALLOW` y `A_NODE_HEAP` en la configuración.»

### 6.7 El lote

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  Desplegando todas · vps-ovh                                       2 min 12 s │
│                                                                               │
│  12 aplicaciones · 8 desplegadas · 2 sin cambios · 1 sin contacto · 1 fallida │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒░░░░░░░░░░░░░░░  10 / 12   │
│                                                                               │
│  ✓ criticabits    desplegada    1 m 04 s   20260830-114302                    │
│  = estropealo     sin cambios       2 s                                       │
│  ⚠ vieja-web      sin contacto      4 s   fatal: could not read Username…  ▸  │
│  ✕ tienda         ha fallado    1 m 38 s   falló en build                 ▾  │
│  │    ┌──────────────────────────────────────────────────────────────┐        │
│  │    │ …                                                            │        │
│  │    │ ERR_PNPM_OUTDATED_LOCKFILE  Cannot install with frozen…       │        │
│  │    └──────────────────────────────────────────────────────────────┘        │
│  ⇥ blog           saltada           0 s   este commit ya rompió el build      │
│  ◐ api            compilando     0 m 41 s  ▓▓▓▓▓▓▓░░░░░░                      │
│  ○ landing        en cola                                                     │
└───────────────────────────────────────────────────────────────────────────────┘
```

Dos comportamientos que importan al terminar:

**Nada se colapsa solo, y lo que falló se queda abierto.** Las `deployed` se colapsan a
una línea; las `failed` conservan su consola desplegada. Una pantalla de resultados que
lo colapsa todo obliga a volver a abrir exactamente lo que se quería ver.

**El orden final no es el de ejecución.** Al acabar, la lista se reordena poniendo
arriba `failed`, luego `unreachable`, luego `gone`, luego `skipped`, y abajo `deployed`
y `unchanged`. Durante la pasada, en cambio, el orden **no** se toca: reordenar mientras
corre haría bailar la lista y es exactamente lo que §5.3 prohíbe.

### 6.8 Cuando el usuario no está mirando: notificaciones del sistema

Es la debilidad nº 6 de la ronda 1 y sale directamente de esta pantalla: un despliegue de
cuatro minutos con la aplicación en segundo plano necesita avisar al terminar, o el usuario
vuelve cada treinta segundos a mirar, que es peor que esperar.

**El permiso se pide cuando hace falta y no al arrancar.** No en el primer arranque, donde
no significa nada y se deniega por reflejo: **la primera vez que un despliegue supera los
45 segundos con la ventana sin foco**. En ese momento la pregunta tiene contexto —«¿te
aviso cuando termine `vps-ovh : tienda`?»— y se contesta con conocimiento. Si se deniega,
no se vuelve a pedir nunca; queda un ajuste en Preferencias para quien cambie de idea.

**Qué se notifica, que son cuatro cosas y sólo cuatro:**

| Suceso | Prioridad | Cuándo |
|---|---|---|
| Un despliegue termina | normal si `ok`, alta si no | Sólo si la ventana no tiene el foco **y** llevaba más de 45 s |
| Un lote (`--all`) termina | normal / alta si hay `failed` o `unreachable` | Igual |
| El vigilante ha marcado algo caído | alta | Sólo si el usuario lo ha activado explícitamente |
| El actualizador tiene una versión nueva | baja, sin sonido | Una vez |

Lo que **no** se notifica, por si a alguien le tienta: que un servidor haya vuelto a
responder, que una lectura de `list` haya cambiado, que un certificado esté a treinta días,
ni nada que el usuario no haya provocado. La regla es la misma que aplica `orbit watch`:
**se avisa de la transición, no del estado**, y sólo de las transiciones que alguien pidió.

**Qué pone dentro, y qué no.** Aquí hay una restricción que no es de diseño sino de
filtración: las notificaciones aparecen **en la pantalla de bloqueo y en la barra de
notificaciones**, es decir, delante de cualquiera que pase por delante del portátil, y en
muchos sistemas se guardan en un histórico.

> **Regla: una notificación no lleva nunca un nombre de dominio, ni una IP, ni una ruta,
> ni un mensaje de error del servidor.**

Los nombres de dominio de una agencia son su cartera de clientes, y una pantalla de bloqueo
en un café es un sitio de lectura pública. Lo que sí lleva es lo que hace falta para
decidir si hay que ir corriendo:

```
  ✓  Orbit Desktop
     vps-ovh : tienda — desplegada en 1 min 51 s

  ✕  Orbit Desktop
     vps-ovh : tienda — el despliegue ha fallado en «compilar».
     La web sigue sirviendo la versión anterior.
```

El identificador es `servidor : app` —la unidad de identidad de §2.9— y no el dominio. Y
en el caso fallido, la segunda frase es la que importa y va dentro: saber que la web sigue
en pie cambia si el usuario abre el portátil ahora o después de cenar. El error concreto
no va: eso está en la pantalla.

**Y hay un ajuste, apagado por defecto, que sustituye el nombre por «una aplicación».**
Para quien trabaje en abierto y prefiera que ni siquiera el nombre salga. Con él, la
notificación dice `vps-ovh — un despliegue ha fallado`. Es menos útil y es su decisión.

**Tres despliegues a la vez no son tres notificaciones.** Se agrupan por servidor y por
ventana de 10 segundos, y se sustituyen en vez de apilarse: si dos terminan casi a la vez,
una sola dice `vps-ovh — 2 despliegues terminados, 1 con error`. Tres notificaciones
seguidas del mismo producto entrenan a descartarlas sin leer, y entonces se descarta
también la que traía el fallo. Con más de tres, una sola: `vps-ovh — 4 despliegues
terminados, 1 con error`.

**El vigilante con la aplicación cerrada: no.** Es tentador y se descarta, con argumento.
Para avisar de que una app se ha caído habría que **sondear el servidor periódicamente en
segundo plano**, y eso rompe dos cosas a la vez: el presupuesto de sondeo del cliente
—que es cero salvo donde el usuario está mirando— y la promesa de que si Orbit Desktop está
abierto y nadie hace nada, el servidor no se entera. Un cliente que despierta cada cinco
minutos para preguntar a cuarenta servidores es un demonio con ventana.

Y además **el problema ya está resuelto en el sitio correcto**: `orbit watch` avisa por
Telegram, Discord, correo o webhook, desde el servidor, funcione o no el portátil, esté o
no abierta esta aplicación. Un aviso que depende de que tu portátil esté encendido no es un
aviso. Así que la interfaz **no compite con eso: lo configura y lo enseña**. La pantalla de
vigilancia tiene, cuando `notify` no está configurado, una banda que lo dice con el
argumento delante:

> Orbit puede avisarte por Telegram o correo desde el servidor, aunque este portátil esté
> apagado. Es la única forma de enterarte de noche. `orbit notify setup`

Lo único que sí hace la aplicación mientras está abierta: **al recuperar el foco, si
`watch status --json` dice que hay sujetos caídos que no lo estaban en la última lectura,
lo enseña**. Eso no es sondeo, es mirar cuando el usuario ya ha vuelto.

**Un clic en la notificación abre la pantalla exacta**, no la portada: el despliegue que
terminó, con su resultado. Una notificación que deja al usuario buscando dónde estaba lo
que se le acaba de contar es media notificación.

---

## 7. Accesibilidad

El usuario de este producto viene del terminal, donde todo se hace con teclado y donde
el contenido es texto. Una interfaz gráfica que sólo se deje usar con ratón sería, para
él, una regresión. Así que aquí la accesibilidad no es un requisito legal añadido al
final: es la continuidad con la herramienta que sustituye.

### 7.1 Teclado completo

> **Ninguna acción del producto es exclusiva de ratón.** Ninguna. Incluido el menú
> contextual de una fila, que responde a `⇧F10` y a la tecla de menú.

| Atajo | Acción |
|---|---|
| `⌘K` / `Ctrl+K` | Paleta de comandos: apps, servidores, pantallas, acciones |
| `⌘1`…`⌘9` | Cambiar de servidor |
| `⌘\` | Colapsar / mostrar la columna de aplicaciones |
| `/` | Enfocar el filtro de la lista |
| `↑ ↓` / `j k` | Moverse por la lista |
| `Enter` | Abrir la aplicación enfocada |
| `Espacio` | Panel lateral de resumen rápido, sin salir de la lista |
| `D` | Desplegar (con su hoja de comando) |
| `L` | Logs |
| `T` | Tráfico |
| `⌘R` | Refrescar la pantalla actual, invalidando su caché |
| `P` | Pausar / reanudar el refresco automático |
| `⌘⇧D` | Ir al despliegue en curso |
| `⌘⇧S` | Leer en voz alta el resumen del servidor (§7.9) |
| `⌘⇧P` | Modo presentación: ocultar dominios, IPs y valores (§4.12) |
| `⌘[` `⌘]` | Atrás / adelante en el historial de pantallas |
| `Esc` | Cerrar panel lateral, modal o filtro (en ese orden) |
| `⌃\`` | Abrir la pantalla de ejecutar un comando (§2.10) |

Sobre las teclas de una sola letra: **sólo están activas cuando el foco está en la lista
o en una tabla, nunca cuando está en un campo de texto**. Es el error clásico de los
atajos de una letra y hace que la gente los desactive. Y se ofrecen las dos familias —
flechas y `j`/`k`— porque el usuario que viene de `vim` espera lo segundo y quien no
viene de ahí espera lo primero; sostener las dos cuesta cuatro líneas.

### 7.2 Foco

```css
:focus-visible {
  outline: 2px solid var(--focus);
  outline-offset: 2px;
  border-radius: var(--r-sm);
}
```

`--focus` es `#5EE7E7` en oscuro (**11,78:1** sobre superficie) y `#0B5561` en claro
(**8,45:1**). Nunca `outline: none` sin sustituto, y el anillo se dibuja **fuera** del
elemento con `outline-offset` para que no lo tape un borde vecino ni lo recorte un
`overflow: hidden` mal puesto —esto último es la causa número uno de anillos de foco
invisibles en tablas con desplazamiento, y se comprueba explícitamente en la tabla del
monitor y en la consola del despliegue.

`:focus-visible` en lugar de `:focus`, para no dibujar el anillo al hacer clic con el
ratón. Dos excepciones donde el foco se ve **siempre**: dentro de un modal y dentro de la
consola del despliegue, porque en los dos casos hay que saber dónde va a ir el `Enter`.

**Trampa de foco** en modales, con retorno al elemento que lo abrió al cerrar. El panel
lateral **no** atrapa el foco —no bloquea nada—, pero se inserta en el orden de
tabulación justo después del elemento que lo abrió, que es lo que hace que se pueda usar
sin ratón sin tener que tabular por media pantalla.

### 7.3 Tablas

Se usan `<table>`, `<thead>`, `<th scope="col">` y `<caption>` de verdad. No una rejilla
de `<div>` con roles ARIA. El motivo es práctico: una tabla nativa le da a NVDA,
VoiceOver y Orca la navegación por celdas, el anuncio de cabecera al cambiar de columna
y el modo tabla, todo gratis y bien probado; una rejilla con ARIA reimplementa eso peor
y se rompe con cada actualización de lector.

```html
<table aria-describedby="apps-nota">
  <caption class="sr-only">40 aplicaciones en vps-ovh, ordenadas por estado</caption>
  <thead>
    <tr>
      <th scope="col" aria-sort="descending">
        <button type="button">Estado</button>
      </th>
      <th scope="col">Aplicación</th>
      …
```

Cuatro reglas:

- **`aria-sort` en la columna ordenada**, y la cabecera ordenable es un `<button>`
  dentro del `<th>`, no un `<th>` con `onclick`.
- **La fila entera no es un botón.** Meter un `role="button"` en el `<tr>` rompe la
  semántica de tabla. El nombre de la app es un enlace; la fila responde a `Enter`
  porque la celda del nombre es la que lleva el foco.
- **El chip de estado lleva texto real**, y el glifo va `aria-hidden`:
  `<span class="chip chip--unserved"><span aria-hidden="true">⊘</span> sin vhost</span>`.
  Un lector lee «sin vhost», no «círculo tachado».
- **`<caption>` con el recuento**, para que quien navegue con lector sepa a qué entra
  antes de recorrerla.

### 7.4 El log en vivo, `aria-live`, y el problema de verdad

Éste es el punto donde la mayoría de los paneles de despliegue se vuelven inutilizables
con lector de pantalla, y no por descuido: un `aria-live="polite"` en un contenedor que
recibe cuarenta líneas por segundo encola cuarenta anuncios por segundo, y el lector se
pasa los siguientes cuatro minutos leyendo salida de compilación mientras el usuario no
puede hacer nada. Es peor que no tener nada.

La solución son tres piezas, y ninguna sirve sola:

**1. El contenedor del log no es una región viva.** Es `role="log"` con
`aria-live="off"`, navegable, leíble a voluntad. Y tiene un botón explícito —«Anunciar
la salida»— que lo pone en `polite` para quien lo quiera, con la advertencia de lo que
implica.

**2. Un resumidor aparte, que es lo que sí habla.** Una región visualmente oculta,
`aria-live="polite" aria-atomic="true"`, que publica **sólo transiciones**:

```
«Paso 3 de 6, compilar, empezado.»
«Compilar, terminado en 1 minuto 38 segundos.»
«Paso 4 de 6, mover el symlink, empezado.»
…
«Despliegue de tienda terminado con error en el paso compilar.»
```

Entre seis y diez anuncios por despliegue en vez de seiscientos. Y llevan el ordinal
(«3 de 6») porque sin él no hay forma de saber cuánto falta sin explorar la pantalla.

**3. `assertive` sólo para el final, y sólo si fue mal.** Un despliegue correcto se
anuncia `polite`; uno fallido interrumpe, porque interrumpe la tarea del usuario de
todas formas.

### 7.5 El panel que se refresca cada dos segundos

Mismo problema, peor: cuarenta filas × treinta refrescos por minuto. Cuatro decisiones:

**La tabla del monitor es `aria-live="off"`. Punto.** No hay versión matizada.

**Existe «seguir esta aplicación».** Se elige una fila y sólo esa publica cambios, y
**sólo cuando cruza un umbral**: se para, arranca, la CPU pasa de 80 %, la memoria
crece un 50 % sobre la media de la sesión. No cada muestra. Un panel que anuncia «CPU
2,4 %… CPU 2,5 %… CPU 2,3 %» no informa, aturde.

**El refresco automático se puede parar, con un botón visible.** No escondido en
preferencias, no sólo con atajo: un control en la barra de estado, siempre presente.
Y esto no es sólo para lectores de pantalla —una pantalla que se mueve sola es hostil
también para quien tiene dificultades motoras o de atención, y molesta a cualquiera que
esté intentando leer una fila concreta.

**Anuncio de contexto al entrar en el monitor**, una vez:

> «Monitor de vps-ovh. Doce aplicaciones. Actualizándose cada dos segundos. Pulsa P
> para pausar.»

Y una equivalencia que se aplica a las cuatro gráficas del producto: cada una lleva
detrás una **tabla de datos equivalente**, oculta visualmente, referenciada con
`aria-describedby`, y con un botón de «ver como tabla» que la enseña también para quien
sencillamente prefiera los números.

### 7.6 Objetivos de puntero

Esto es una aplicación de escritorio, así que la regla operativa es la de WCAG 2.2
(criterio 2.5.8, *Target Size (Minimum)*): **24 × 24 CSS px como mínimo**, con **32 × 32
como objetivo real** y 8 px de separación entre objetivos adyacentes.

Dos consecuencias concretas:

- Los iconos de acción de una fila tienen caja de 32 × 32 aunque el glifo mida 14 o 16.
- En **densidad compacta**, la fila baja a 30 px pero el objetivo **no baja de 24**: el
  botón conserva su área con `padding` y, donde no cabe, se extiende con un
  pseudo-elemento. La regla escrita: *encoger la fila nunca puede significar encoger el
  objetivo*.

Y una excepción legítima que la propia norma contempla: las celdas de una tabla densa
no son objetivos independientes cuando toda la fila es activable —la fila entera es el
objetivo, y mide 280 × 30.

### 7.7 Lo demás que se comprueba

- **Zoom al 200 %** sin pérdida de contenido ni de funcionalidad (1.4.4), y reflujo
  comprobado en una ventana equivalente a 1.280 × 720 escalada.
- **`prefers-contrast: more`**: los bordes de control suben a 4,5:1, los chips ganan un
  borde de 1 px del color de su estado, y los separadores decorativos suben a 3:1.
- **Nada depende sólo del color** (1.4.1). Ya cubierto por los glifos de §3.4, pero se
  verifica con una captura en escala de grises de la lista y del lote, que son las dos
  pantallas donde más colores conviven.
- **Idioma**: `lang="es"` / `lang="en"` en la raíz, cambiable sin reiniciar — y el
  atributo cambia también en los fragmentos que vengan del servidor en otro idioma, que es
  lo que hace que un lector de pantalla no pronuncie un mensaje español con fonética
  inglesa. La regla de qué se traduce y qué no está en §8.8.
- **Todo error crudo va en un `<pre>` seleccionable con botón de copiar.** Es
  accesibilidad práctica: la primera acción de cualquiera ante un error es copiarlo.

### 7.8 El diálogo destructivo, en el orden en que se lee

Un lector de pantalla, al abrirse un diálogo, anuncia su nombre accesible y luego recorre
el contenido. Si el primer anuncio es «Botón: Eliminar», el diálogo es una trampa: quien no
ve la pantalla ha oído la acción antes que el objeto sobre el que actúa.

**El orden de anuncio se fija, y es el mismo que el orden del DOM:**

```
1. servidor y app         «Retirar y borrar los datos. vps-ovh, dos puntos, tienda.»
2. qué se pierde          «Se borrarán 5 versiones, 12 variables de entorno y
                           4,2 gigabytes en shared. Esto no se puede deshacer.»
3. qué se puede recuperar «Hay una copia del 28 de agosto. Se puede restaurar
                           con orbit restore.»
4. el campo de confirmar  «Escribe tienda para confirmar. Campo de texto, vacío.»
5. los botones            «Cancelar, botón. Retirar y borrar, botón, desactivado.»
```

Cuatro reglas que sostienen ese orden:

**El nombre accesible del diálogo lleva ya el servidor y la app.**
`aria-labelledby` apunta a un título que dice `vps-ovh : tienda`, no «¿Estás seguro?».
La primera cosa que se oye es el objetivo. Y es la misma unidad de identidad de §2.9: el
nombre de la app no identifica nada por sí solo, porque `tienda` existe en tres servidores.

**El foco inicial no está en ningún botón.** Va al cuerpo del diálogo (o al campo de
confirmación cuando lo hay), nunca a «Cancelar» y muchísimo menos a la acción. Un `Enter`
reflejo al abrirse un diálogo no puede ejecutar nada.

**El botón destructivo está desactivado hasta que el nombre coincide**, y su estado
desactivado se anuncia con el motivo (`aria-describedby` → «escribe el nombre de la app
para activarlo»). Un botón gris sin explicación obliga a explorar la pantalla para
averiguar qué falta.

**Y la fricción se gasta donde el daño es irreversible, no en todas partes.** Escribir el
nombre a mano se pide **sólo** para `remove --purge`. Para el rollback, no: es reversible,
y pedir el nombre en las dos cosas hace que escribir el nombre deje de significar algo.
Esto no es una concesión a la comodidad: es que **la fricción es un recurso escaso**, y
gastarla en lo reversible la agota justo donde hacía falta.

Y una consecuencia visual del mismo orden, para quien sí ve: el diálogo destructivo lleva
el color del servidor en un borde de 3 px, y el inventario de lo que se pierde va **encima**
de los botones, con el mismo peso tipográfico que el título. Lo que se lee primero es lo
mismo que lo que se oye primero.

### 7.9 Anunciar los cambios de estado sin volverse ruidoso

§7.4 y §7.5 resolvieron el log del despliegue y la tabla del monitor. Queda el caso
general: **la lista de aplicaciones, que puede cambiar de estado mientras se mira.**

El problema tiene dos mitades y una regla distinta para cada una.

**Mitad 1 · Los cambios que el usuario ha provocado se anuncian siempre, y de inmediato.**
Si acabo de pulsar «reiniciar», que el servicio pase a activo es la respuesta a mi acción y
tiene que llegar aunque el foco esté en otra parte. Va por la región `polite` con el
patrón completo: `vps-ovh : tienda, activo.`

**Mitad 2 · Los cambios que llegan solos se anuncian por excepción, no por cambio.** Un
refresco de `status` puede traer cinco diferencias respecto de la lectura anterior, y
anunciar cinco frases cada quince segundos hace que la aplicación no se pueda usar. La
regla:

> **Sólo se anuncia lo que cruza hacia peor, agrupado, y como mucho una vez por minuto.**

En concreto, se anuncia cuando una aplicación entra en `sin vhost`, en `parado` o en
`mantenimiento`, y **no** se anuncia cuando sale de ellos (eso es una buena noticia que
puede esperar a que alguien mire), ni cuando cambia la CPU, ni cuando cambia el número de
releases, ni cuando un certificado pasa de 11 a 10 días.

Y se agrupa igual que las notificaciones: dos cambios en la misma lectura son una frase,
no dos. `vps-ovh: 2 aplicaciones han pasado a parado, tienda y api.` Con más de tres, se
cuenta y no se enumera: `vps-ovh: 6 aplicaciones han pasado a sin vhost.` — que es, además,
la misma situación que dispara la banda de §3.8.3, y las dos dicen lo mismo por canales
distintos, que es como debe ser.

**Tres controles que el usuario tiene sobre esto**, y los tres son visibles:

1. **Pausar el refresco** (`P`, y un botón en la barra de estado). Para todo, incluidos los
   anuncios.
2. **Silenciar los anuncios automáticos** sin pausar el refresco, para quien prefiera mirar
   a que le cuenten.
3. **Un resumen bajo demanda**: `⌘⇧S` lee el estado completo del servidor en una frase —
   `vps-ovh, 40 aplicaciones: 38 activas, 1 parada, 1 sin vhost` —, que es lo que hace
   falta al entrar y que evita tener que recorrer cuarenta filas para saber si hay algo que
   mirar. Es la versión hablada de lo que la banda de recuento hace visualmente, y es
   probablemente el atajo más útil de todo el producto para quien navega con lector.

Y una regla que gobierna las tres: **ninguna región viva tiene `assertive` salvo el fallo
de un despliegue.** `assertive` interrumpe lo que se esté leyendo, y sólo se gana ese
derecho lo que interrumpe la tarea del usuario de todas formas.

---

## 8. Recomendación de tecnología, desde el lado de la interfaz

Contexto que condiciona todo: es una aplicación de escritorio que se abre varias veces
al día para sesiones de menos de dos minutos, así que **el arranque en frío es una
característica de producto**. Y la carga de trabajo de la interfaz es muy concreta:
tablas, texto, estado y cuatro gráficas pequeñas, con una pantalla —el monitor— que
reconcilia cuarenta filas cada dos segundos. Nada de 3D, nada de lienzos pesados, nada
de listas de cien mil elementos.

**Esta sección se reescribe por segunda vez, y conviene contar por qué.** En la ronda 1
recomendé Svelte 5 y descarté React. En la ronda 2, con la auditoría del contrato delante,
retiré esa recomendación y acepté React, porque **el argumento que la sostenía era falso**.
El arquitecto hizo el movimiento simétrico: retiró React y aceptó Svelte. Dos cesiones
cruzadas no son un acuerdo, son un documento que se contradice consigo mismo, así que la
coordinación ha arbitrado: **Svelte 5 con CSS de tokens**. Recojo aquí el razonamiento
completo, incluida la parte que me quita la razón y la parte que me la da, porque una
decisión que sólo se anuncia se reabre en tres meses.

### 8.1 Framework de UI → **Svelte 5**, por un criterio que no es el rendimiento

Lo primero es lo que **no** decide, y hay que escribirlo porque es lo que los dos habíamos
usado para decidir:

**El rendimiento no decide, y mi argumento de la ronda 1 era falso.** Sostuve que «el
monitor reconcilia cuarenta filas cada dos segundos y ahí React obliga a memoizar a mano».
Con los datos delante eso no describe este producto: cuarenta filas son **12 KB** (311 bytes
por app, medido), el monitor **sólo sondea con la pantalla visible y la ventana enfocada**,
y su ciclo real es de unos 3,2 s y no de 2, porque cada muestra cuesta un segundo en el
servidor. El arquitecto lo cerró midiéndolo: **reconciliar los ~320 nodos de esa tabla
cuesta unos 2 ms cada 3,2 s.** Eso es ruido. Lo dejo escrito con el número porque una
decisión defendida con una cifra equivocada se reabre sola, y porque el error era mío.

**El peso tampoco decide, y ésa sí la mantengo.** Bajo Tauri el WebView es del sistema: no
se empaqueta un motor, y la diferencia de arranque entre 45 KB y 200 KB de JavaScript
comprimido existe pero no es la que separa usar de no usar.

**Y cuando los dos diferenciadores objetivos se anulan, lo que queda no es un empate.**
Queda el criterio con el que este proyecto ya ha decidido dos veces, y en las dos pagó una
incomodidad concreta a cambio de una superficie más pequeña:

| Decisión | Lo que se pagó | Lo que se compró |
|---|---|---|
| Tauri en vez de Electron | Dos lenguajes en el repositorio | Ni Node ni Chromium en el proceso que sostiene las credenciales SSH |
| El `ssh` del sistema en vez de una librería | Analizar texto de error en vez de tener excepciones tipadas | Cero código de criptografía propio; `ProxyJump`, `Match` e `IdentityAgent` gratis |
| **Svelte en vez de React** | **Un mercado de contratación mucho más pequeño** | **Un árbol de dependencias de terceros mucho menor en ese mismo proceso** |

Elegir React aquí sería **la primera vez que el proyecto rompe su propio patrón**, y lo
rompería justo en la dimensión que el informe de seguridad puntúa peor de todas: la cadena
de suministro. No es el tamaño del bundle lo que importa —200 KB comprimidos no cambian el
arranque de forma perceptible— es **cuántos paquetes de terceros se ejecutan en el proceso
que tiene acceso a las claves SSH del usuario**. Es literalmente el mismo argumento con el
que se descartó Electron, y no puede valer allí y no valer aquí.

Hay además una coherencia de discurso que este producto no puede permitirse perder: Orbit
rechaza el panel web porque sería «una shell de root expuesta a internet», y cita un CVE.
Un cliente que responde a eso arrastrando el árbol transitivo más grande disponible en el
mismo proceso que las credenciales estaría contradiciendo con su `package.json` lo que
afirma en su README.

**El coste se acepta y se escribe, porque es real.** El mercado de gente que sabe React es
mucho mayor que el de Svelte, y para un equipo pequeño eso es **un riesgo de mantenimiento a
cinco años**, no una molestia. Lo que lo hace pagable es dónde vive la lógica: el
`OrbitClient`, el analizador del contrato, la caché y los tipos están en `crates/`, fuera de
la interfaz. **Un cambio de framework no toca nada de eso.** O sea que ésta es una decisión
**reversible** —se reescribe la capa de vistas y el resto sigue— mientras que las de
transporte y de contrato no lo son. Cuando hay que elegir dónde equivocarse, se elige lo
reversible.

**Mis tres condiciones de la ronda 2 se mantienen íntegras**, y no por inercia: son buenas
con cualquier framework y ninguna depende de cuál sea.

1. **Presupuesto de 8 ms por refresco con 40 filas, medido en los tres WebView y en CI.**
   Que hoy sean 2 ms no exime de medirlo: lo que se mide es que siga siendo así después de
   cien commits.
2. **Sin virtualización por debajo de 200 filas** (§3.8.3). Activarla desde el principio
   complica el desplazamiento, el orden de foco y la búsqueda del navegador para resolver un
   problema que los datos dicen que no existe.
3. **El arranque en frío se mide y se publica en CI.** Es una característica de producto, y
   una característica que no se mide se degrada sola.

**Y una reserva honesta sobre la capa de datos.** El modelo de caché de §4.1 —`staleTime`
por naturaleza del dato, deduplicado de peticiones en vuelo, invalidación por prefijo— se
apoya en TanStack Query. La versión de Svelte, `@tanstack/svelte-query`, es del mismo
mantenedor y de la misma base, pero **está menos rodada que la de React**: menos usuarios,
menos preguntas contestadas, menos ejemplos. Eso es un riesgo, no un empate, y lo digo
porque el argumento de la cadena de suministro se gana pagando esto y hay que saber lo que
se paga. La mitigación es acotarlo: las cuatro cosas que este producto usa de esa librería
son las cuatro más básicas, y si alguna diera problemas se sustituye por un store propio
sin tocar ninguna pantalla.

**Lo que sigue descartado, y por qué:** **React** (cadena de suministro, con todo lo
anterior); **SolidJS** (misma tesis que Svelte, ecosistema aún más fino, y ningún argumento
que lo prefiera); **Vue** (correcto y sin ventaja aquí); **componentes web a pelo** (se
acaba escribiendo un framework peor, y la parte que sale mal siempre es la reactividad).

### 8.2 Estilos → **CSS puro con tokens**, y `estado.css` como pieza central

El arbitraje cierra también esto, y en la dirección que yo defendía. Recojo lo que dije en
la ronda 2 y lo endurezco, porque sin Tailwind el argumento no es más débil: es más fuerte.

**La regla no cambia y sigue siendo lo importante:**

> **El estado semántico vive en un solo fichero.** Los chips, la fila de la lista, los seis
> finales del lote y las bandas de error están en `estado.css`, con clases que se llaman
> como el concepto: `.chip--sin-vhost`, `.fila--desatendida`, `.lote--unreachable`.

El motivo es el mismo que hizo que §3.4 tenga una tabla de precedencia escrita: **la
distinción entre `served:false`, `service:null` y `service:"stopped"` es el activo más
valioso de este producto** —es lo que impide pintar una alarma roja donde no pasa nada— y es
exactamente lo que se disuelve cuando el estado se convierte en
`bg-red-500/10 text-red-400 border-red-500/30` repetido en cuatro componentes. El día que
alguien añada una quinta copia con un matiz distinto, la lista y el detalle dejan de decir
lo mismo, y nadie lo nota hasta que un usuario ve un incidente donde no lo hay.

En la ronda 2 usé ese argumento para sacar el estado de Tailwind. Con el arbitraje, **el
argumento es el motivo por el que Tailwind no entra**, no sólo el motivo por el que el
estado sale de él: si el sistema son sesenta tokens y veinticinco componentes, un motor de
utilidades es infraestructura para un problema que no tenemos, y su único efecto neto sería
crear un segundo vocabulario de color al lado del que ya está definido en §3.2. Dos
vocabularios de color en el mismo producto son dos verdades sobre qué es rojo.

Y hay un beneficio de legibilidad que en una herramienta de sistemas pesa: **el DOM se
lee**. `class="app-row is-unserved"` dice algo cuando alguien abre el inspector para
entender por qué una fila está en rojo. Una tira de veinte utilidades no.

**La regla se comprueba, porque una regla que no se comprueba no es una regla.** Una regla
de lint que **falla el build** si aparece un color literal —`#`, `rgb(`, `hsl(`— fuera de la
capa de tokens, o si una clase de estado se define fuera de `estado.css`. Diez líneas de
configuración, y es lo que hace que la decisión sobreviva a la semana veinte.

Todo lo demás de §3 queda tal cual y ahora sin intermediario: los tokens de §3.2 son
variables CSS en `:root`, los dos temas de §3.3 son los cuatro bloques que ya están
escritos, y los tintes de chip salen de `color-mix()`. El CSS por componente lo da Svelte
de serie con su `<style>` de ámbito, que es exactamente el alcance que hace falta y no
requiere ninguna herramienta añadida.

### 8.3 Componentes → **Melt UI para las primitivas, propio para lo demás**

Vuelve la propuesta de la ronda 1, que es la equivalente para Svelte de lo que Radix es
para React: las mismas cinco piezas sin estilo donde la accesibilidad es difícil y aburrida
y donde equivocarse es caro —diálogo, menú, tooltip, popover y *combobox*—, con la gestión
de foco, `aria-expanded`, el cierre por `Esc` y las colisiones de posicionamiento ya
resueltas.

Todo lo demás sigue siendo propio, y por el mismo motivo de siempre: **tabla, chip de
estado, barra de progreso, consola de eventos y hoja de comando son las piezas que llevan
las decisiones de este documento**. Ninguna librería trae una tabla que sepa de
`served:false` ni una barra de progreso que no retroceda.

El apunte sobre §7.8 se mantiene igual, sólo cambia el nombre: el diálogo de Melt UI
etiqueta con `aria-labelledby` apuntando a su título —lo que hace falta, siempre que el
título sea `vps-ovh : tienda` y no «¿Estás seguro?»— y permite redirigir el foco inicial
fuera de los botones. Las dos cosas hay que hacerlas a mano; la librería lo permite, no lo
regala.

### 8.4 Gráficas → **SVG propio para las cuatro, con `d3-scale` y `d3-shape`**

Recharts es de React y se cae con el arbitraje. Así que hay que rehacer esta decisión, y la
tentación es sustituir una librería por otra. No lo hago, y el motivo obliga a mirar qué
compraba de verdad la concesión de la ronda 2.

**Lo que compré con Recharts no fue «una librería»: fue madurez.** Delegué dos de las
cuatro gráficas porque Recharts hace bien el trabajo aburrido —ejes, leyenda, *tooltip*,
responsividad— y porque lleva años haciéndolo, con lo que el riesgo que me quitaba de
encima era real. El equivalente en Svelte no tiene esa propiedad: **LayerChart** es la
opción más completa y es mucho más joven y con mucha menos comunidad, así que delegarle las
gráficas no retiraría riesgo de calendario, lo cambiaría por riesgo de dependencia —que es
peor, porque el de calendario se ve venir y el de dependencia aparece el día que hay que
arreglar algo y no hay respuesta escrita en ningún sitio. **Sin madurez no hay nada a lo que
conceder.**

**Y `LayerCake`, que es la otra forma de plantearlo, no llega a la cuenta.** No es una
librería de componentes sino un andamio: da el contenedor responsivo, las escalas y un
contexto de coordenadas, y las marcas las dibujas tú. Es honesto y es poco. Pero eso es
`d3-scale` más un `ResizeObserver` más un contexto de Svelte, que son unas treinta líneas
propias, y meter una dependencia para no escribir treinta líneas es exactamente la clase de
decisión que engorda el árbol que §8.1 acaba de decidir cuidar.

**Así que las cuatro se dibujan en SVG, con `d3-scale` y `d3-shape` como únicas
dependencias**, y el argumento decisivo de la ronda 1 vuelve a ser el argumento decisivo:

> **Las cuatro tienen que saber pintar lo que no se sabe**, y las librerías genéricas o
> interpolan por encima del `null` —inventando un valor— o lo tratan como cero —inventando
> una caída—. Las dos cosas son exactamente la mentira que el contrato de Orbit se ha
> esforzado en no decir.

Los cuatro casos concretos, que son los mismos que iba a fijar por prueba con Recharts y
que ahora se cumplen por construcción:

| Caso | Qué exige el dato | Qué hace una librería genérica |
|---|---|---|
| `cpu_percent: null` | **Hueco**: la línea se corta y se pinta un tramo punteado | Interpola, o dibuja una caída a cero |
| Hora sin tráfico | **Cero real**, que ocupa su sitio en el eje | Se salta el punto y encoge el hueco: dos picos separados por un día parecen seguidos |
| `complete: false` | **El eje es más corto que lo pedido**, y hay que poder decirlo | Pinta el rango pedido con vacío dentro |
| `build_trend_s: null` (< 6 builds) | **No se dibuja tendencia**; hay una nota que dice por qué | Dibuja una línea plana, que significa «igual» |

Y con SVG se obtiene gratis lo que en un lienzo hay que reconstruir: escala con el zoom del
sistema sin pixelarse, `<title>` por elemento, selección de texto en las etiquetas, y la
tabla de datos equivalente de §7.5 sin duplicar la lógica.

**El riesgo de calendario vuelve a subir, y lo digo en vez de taparlo.** Era mi debilidad
nº 7 de la ronda 1, la retiré a medias en la ronda 2 al delegar dos gráficas, y con el
arbitraje vuelve. El tamaño honesto: las dos gráficas de pantalla completa —barras por hora
con dos series apiladas, y línea de duración de build con la mediana— son **entre tres y
cuatro días cada una** contando el *tooltip*, los ejes y la tabla accesible; el *sparkline*
del monitor y la barra apilada del lote son sesenta líneas entre las dos. O sea **algo más
de una semana de proyecto**, frente a unos tres días con una librería madura. Esa semana es
el precio del arbitraje en mi columna, y es un precio, no un detalle.

Tres cosas lo acotan, y por eso sigue siendo pagable:

1. **Las dos grandes comparten un solo esqueleto.** Un componente con ejes, rejilla,
   *tooltip* y contenedor responsivo, y dos capas de marcas encima. Es una implementación de
   ejes, no dos.
2. **Los cuatro comportamientos de la tabla de arriba se escriben como pruebas el primer
   día**, no al final. Con Recharts eran pruebas para vigilar a un tercero; aquí son
   pruebas para vigilarnos a nosotros, que es lo mismo dentro de un año.
3. **Y la degradación ya existe como requisito.** §7.5 obliga a que cada gráfica lleve
   detrás una tabla de datos equivalente. O sea que si el calendario aprieta, **la gráfica
   de tráfico se puede aplazar y quedarse en su tabla**, que es una pantalla peor pero
   completa y accesible, no un hueco. Un riesgo con un plan de repliegue escrito es un
   riesgo acotado.

**Lo que sigue descartado, sin cambios:** Chart.js y ECharts sobre lienzo (no accesibles,
no seleccionables, y el hueco hay que pelearlo igual), Recharts y visx (atados a React), y
Observable Plot (pensado para exploración de datos, no para un panel que se refresca).

### 8.5 Animación → **CSS y `Element.animate()`. Ninguna librería.**

Sin cambios de fondo respecto de las dos rondas anteriores, porque nunca dependió del
framework. El presupuesto entero de §5 son transiciones de `opacity` y `transform` y una
barra que se interpola con `requestAnimationFrame` y la regla de que es monótona creciente.
GSAP y Motion One son excelentes y no resuelven ningún problema que tengamos: serían 15-30
KB y, sobre todo, una tentación de animar de más, que aquí es un riesgo real —§5.3 es más
larga que §5.2 por algo.

Y un aviso propio de Svelte, que sustituye al que en la ronda 2 era sobre Radix: **Svelte
trae transiciones incorporadas (`transition:`, `animate:`) y son cómodas hasta el punto de
ser un peligro.** La regla se mantiene tal cual: **una animación entra sólo si tiene una
frase en la columna «qué comunica» de §5.2**, y las transiciones incorporadas no eximen de
esa columna. Para lo que sí entra, se prefiere CSS con los atributos de estado que exponen
los primitivos de Melt UI, porque así el movimiento vive en el mismo sitio que el resto del
sistema visual y respeta `prefers-reduced-motion` con la misma regla de §5.5 sin código
aparte.

### 8.6 Terminal → **xterm.js**, y sólo donde hace falta

Es la única dependencia grande que se acepta, y se acepta porque el problema es
genuinamente difícil: ANSI de verdad (Orbit colorea su salida), reflujo al redimensionar,
selección por bloques, búsqueda, y rendimiento con decenas de miles de líneas.
Escribirlo sería un proyecto en sí mismo.

Dos matices de uso:

- **`orbit logs` no es una sesión interactiva**, es texto coloreado que llega. Se abre
  en modo sólo lectura, sin `stdin`, con los complementos `fit`, `search` y `web-links`.
  Sobre él, la interfaz añade lo que un terminal no da: filtro por nivel, resaltado de
  códigos HTTP, y los atajos de `--since` como botones (`30m`, `2h`, `hoy`, `ayer`).
- **`orbit exec` tampoco necesita PTY, y ése es el punto.** En la ronda 1 escribí que sí,
  y era una consecuencia de haber dado por buena la shell interactiva embebida. §2.10 la
  descarta: la interfaz ejecuta un comando y recibe su salida, y para una shell de verdad
  copia la orden `ssh` al portapapeles. Sin PTY, `xterm.js` aquí es un visor de sólo
  lectura igual que en logs, no un emulador de terminal completo, y eso quita de encima
  todo el problema de tamaños de ventana, señales y `stdin`.
  **Y la salida de `exec` no se interpreta como ANSI: se pinta como texto plano.** Es
  salida arbitraria de un proceso arbitrario —puede traer secuencias de escape, bytes
  nulos o cinco megas en una línea— y un visor que la interpreta es un visor que un
  proceso remoto puede usar para dibujar lo que quiera en nuestra ventana.

**Descartado: un `<pre>` con un analizador de ANSI propio.** Se rompe con la primera
secuencia rara, no tiene selección decente y hay que reimplementar la búsqueda.
**Descartado: hterm**, menos mantenido.

Y una decisión de producto que va con esto: **el terminal no es la interfaz por defecto
de nada**. Si la aplicación enseña un terminal para contar algo que el contrato ya sabe
decir estructurado, está renunciando a ser una interfaz.

### 8.7 Envoltorio de escritorio → **Tauri v2**, decidido, y lo que fija para el CSS

En la ronda 1 dije que no era mi decisión pero que tenía consecuencias visuales. Ya está
decidida —**Tauri v2**— y coincide con lo que yo habría pedido, así que aquí sólo apunto lo
que fija para mi lado.

**Lo que gano.** Arranque por debajo del segundo, que era mi condición: la ventana no puede
tardar más en pintarse que el primer `status --json` (389 ms de servidor más el saludo SSH),
porque entonces el usuario ve una ventana en blanco esperándose a sí misma antes de esperar
al servidor, y concluye que la aplicación es lenta por un motivo que no tiene nada que ver
con el servidor.

**Lo que me cuesta, y es lo que hay que escribir para que nadie lo descubra tarde: son tres
motores de WebView distintos.** WebKitGTK en Linux, WKWebView en macOS, WebView2 en Windows.
Eso fija el suelo de CSS de todo el §3:

| Función | ¿Se puede usar? |
|---|---|
| Variables CSS, `flex`, `grid` | Sí, sin reservas |
| `color-mix()` | Sí — de ahí los tintes de chip de §3.3 |
| `:has()` | Sí, pero sin abusar: es el que más divergencias da |
| `subgrid` | Con cuidado y con reserva |
| **`light-dark()`** | **No.** Por eso §3.3 escribe los dos temas a mano |
| `@container` | Sí, y es la forma correcta de que la columna de 280-360 px reaccione |
| `text-wrap: balance` | Sí, pero es decoración: nada depende de ello |

Y la consecuencia de proceso: **capturas de pantalla en CI en las tres plataformas**, sobre
las cuatro pantallas donde una divergencia se paga —lista, despliegue en vivo, diagnóstico y
monitor—, en los dos temas. Sin eso, la divergencia se descubre en el ordenador de un
usuario.

Un apunte sobre §7.6: los objetivos de puntero de 24 px se miden en píxeles CSS, y los tres
sistemas escalan distinto. La comprobación de tamaño de objetivo se hace **a 100 %, a 125 %
y a 200 %** de escala del sistema, porque el redondeo a 125 % es donde una caja de 24 px
acaba midiendo 23.

### 8.8 Tipos e idiomas

**Los tipos se generan desde Rust con `ts-rs`, y retiro lo que dije en la ronda 1.**
Propuse escribirlos a mano «porque escribirlos obliga a leerlos», y ese argumento es bueno
para el primer día y malo para todos los demás: dos listas de campos mantenidas por
separado divergen, que es literalmente la lección de `ORBIT_APP_FIELDS` en el propio Orbit
—tres sitios enumeraban los campos de una app y el que se olvidaba dejaba el valor de la
app anterior—. El trabajo de leer el contrato ya está hecho, y está en la auditoría.

Lo que sí sigue siendo mío y no se genera: **los `null` con nombre propio.** El tipo dirá
`service: string | null`, y encima habrá un puñado de funciones que traducen esos `null` al
vocabulario de §3.4 —`estadoDeApp()`, `cpuDesconocida()`, `ventanaRecortada()`— escritas
**una vez** y usadas en la lista, en el detalle, en el monitor y en el rail. La precedencia
de §3.4 vive en una función, no en cuatro componentes. Si vive en cuatro, en seis meses hay
cuatro precedencias.

Y un matiz sobre `config`: son **37 campos, no 31**, y todos son cadenas porque en el
fichero lo son. Un tipo escrito a partir de la documentación se habría quedado corto en
seis, que es otra razón para generarlo del emisor.

**i18n**: catálogos planos `es` / `en` y ninguna librería. En la ronda 2 acepté `i18next`
porque venía en la lista del arquitecto; con el criterio de §8.1 sobre la mesa, no se
sostiene: son **dos idiomas y cadenas sin plurales complicados**, y lo que hace falta —una
función de búsqueda con interpolación y un store reactivo con el idioma activo— son unas
cuarenta líneas. Añadir un árbol de dependencias para eso es justo lo que acabamos de
decidir no hacer. El formateo lo hace el navegador: `Intl.NumberFormat`,
`Intl.RelativeTimeFormat` y `Intl.DateTimeFormat`. Formatos que hay que fijar y
que son la mitad de los errores de una interfaz de sistemas: bytes en base 1024 con
sufijo corto (`184M`), duraciones humanas por encima de 60 s (`1 min 38 s`), porcentajes
con una decimal y coma decimal en español, y fechas de release siempre en el nombre
literal de la release (`20260805-041230`) con la fecha legible al lado, nunca en su
lugar.

Y la regla de traducción que no cambia y que importa más que la librería: **los textos que
vienen del servidor no se traducen.** Si `orbit` habla español y el cliente está en inglés,
el error sale en español, etiquetado como venido del servidor. Traducir a medias un mensaje
ajeno produce frases que no existen en ninguna documentación y que no se pueden buscar.

---

## 9. Puntuación, rondas 2 y 3

Sigo puntuando la propuesta y no el producto. En la ronda 1 me puse **78 en UX y 82 en
usabilidad**, con ocho debilidades y una tabla de qué haría falta para subirlas. La ronda 2
fue a por seis de esas ocho, con evidencia medida en vez de con más razonamiento. La
ronda 3 sólo tocó §8, y por tanto sólo puede mover la nota por el lado del stack.

| | R1 | R2 | **R3** | Qué la mueve |
|---|---|---|---|---|
| **UX** | 78 | 87 | **87** | R2: el asistente rehecho con siete finales, el diagnóstico honesto, multiservidor contestado, notificaciones, la tabla decidida con datos. R3: dos movimientos que se anulan, §9.1b |
| **Usabilidad** | 82 | 89 | **88** | R2: latencia cerrada con cifras reales y 306 ms retirados de la portada, el diálogo destructivo con orden de lectura, los anuncios acotados, `exec` predecible. R3: **−1**, §9.1b |

### 9.1 Qué se cerró en la ronda 2, y con qué

**A · Latencia (era la debilidad nº 1).** Cerrada. Ya no es «gestionada»: hay medianas
medidas sobre 40 apps (72 / 86 / 306 / 389 ms), hay una decisión de transporte
(`ControlMaster=auto`, ~12 ms por canal en caliente frente a 150-350 ms de saludo), y los
umbrales de §4.1 salen de sumar las dos cosas en vez de de mi intuición. Y hay una ganancia
que no esperaba: **`status --json` trae el array `apps` completo**, así que la portada baja
de 695 ms a 389 ms sin escribir una línea de código, sólo eligiendo bien qué se pide.
También se retiró un paliativo que sobraba (el *prefetch* al pasar el puntero), que es la
otra mitad de cerrar bien una debilidad. **+6 usabilidad.**

**B · El asistente de web nueva (era la nº 2).** Cerrado. De cuatro pasos y una pantalla
de resultado a cinco pasos y **siete finales**, con la regla que lo sostiene: la interfaz
no analiza la prosa de `orbit new`, le vuelve a preguntar al servidor con `info --json`.
Y el caso que faltaba —la detección de *stack* equivocándose, que es el caso normal— tiene
ahora una pantalla propia que enseña cada conclusión con su porqué y una salida de
emergencia hacia `orbit init`. **+5 UX.**

**C · El diagnóstico (no estaba en mi lista; lo trajo la evidencia).** El botón de arreglar
de la ronda 1 invocaba un comando que **no existe**. Rehecho: tres presentaciones según
`fixable` y `fix`, la orden para copiar, y la capacidad enganchada a la versión de Orbit
del servidor, de modo que la pantalla gana el botón el día que el PR aterrice sin que la
aplicación cambie. Vale más de lo que parece: era una funcionalidad fantasma que habría
llegado hasta la implementación. **+2 UX, +1 usabilidad.**

**D · Notificaciones (era la nº 6).** Cerrada, incluida la parte que no es obvia: qué **no**
se pone en una notificación, porque la pantalla de bloqueo es un sitio de lectura pública
y los dominios de una agencia son su cartera de clientes. Y la decisión de no sondear en
segundo plano para el vigilante, con el argumento de que el problema ya está resuelto en el
sitio correcto —`orbit notify`, desde el servidor, funcione o no el portátil—. **+3 UX.**

**E · Multiservidor operativo (era la nº 5).** Contestado, y con dos noes razonados en vez
de con una funcionalidad: no hay despliegue en grupo y no hay lista global, porque las dos
son un plano de control con otro nombre. Lo que sí hay: portada de servidores a partir del
cuarto, «desplegar este commit en otro servidor» como acción explícita tras un despliegue
correcto, y cinco medidas contra el accidente del servidor equivocado, que tiene precedente
real en el propio repositorio. **+3 UX.**

**F · La tabla con 40 apps (era la nº 5 de la tabla de acciones).** Decidida con datos:
`_app_name_ok` da el máximo de 40 caracteres, el `printf` de `orbit list` no recorta y una
fila con nombre y dominio legales pasa de 79 a 118 caracteres, y 311 bytes por app dicen que
no hace falta virtualizar. Y salió un hallazgo que no habría tenido de otra forma: con las
cuarenta apps en `served:false`, **la lista es una pared de rojo donde el chip deja de
discriminar**, y de ahí la regla del 70 % y la banda de resumen. **+2 UX, +1 usabilidad.**

**G · `exec` (era la nº 4).** Alineado con el diseño de QA, que es mejor que el mío: dos
modos visibles en vez de la heurística invisible del argumento único, orden literal ya
escapada antes de ejecutar, histórico en memoria, sin shell interactiva embebida. Mantengo
un matiz propio: la pantalla debe **verse** distinta de las demás, por el mismo argumento
por el que `served:false` es el único chip sólido. **+2 usabilidad.**

**H · Accesibilidad.** Añadidos el orden de lectura del diálogo destructivo —servidor y app
antes que el botón, foco inicial fuera de los botones— y la política de anuncios en vivo:
sólo lo que cruza hacia peor, agrupado, como mucho una vez por minuto, con tres controles
visibles y un resumen bajo demanda con `⌘⇧S`. **+2 usabilidad.**

### 9.1b Lo que mueve el arbitraje del stack, en las dos direcciones

La coordinación ha arbitrado **Svelte 5 con CSS de tokens** después de que el arquitecto y
yo cediéramos en direcciones cruzadas. La decisión es correcta por su propio criterio —la
cadena de suministro, que es el mismo con el que este proyecto ya eligió Tauri y el `ssh`
del sistema— y ese criterio no es el mío: yo puntúo experiencia y usabilidad. Así que la
contabilidad honesta es que **me quita algo y me da algo**, y las dos cosas hay que
escribirlas:

**+1 UX · `estado.css` queda mejor protegido sin Tailwind.** En la ronda 2 tuve que
defender una frontera dentro de un sistema de utilidades, con una regla de lint que vigilaba
que el estado no se disolviera en `bg-red-500/10` repetido. Sin Tailwind, esa frontera no
existe porque no hay un segundo vocabulario de color contra el que defenderla: hay uno, el
de §3.2, y el estado semántico es la única forma de escribirlo. El activo más valioso del
producto —la distinción entre `served:false`, `service:null` y `stopped`— está más seguro,
y eso es UX y no gusto personal.

**−1 UX · Las gráficas vuelven a ser un riesgo de calendario entero.** Mi concesión de la
ronda 2 —delegar dos de cuatro a Recharts— retiraba la mitad de mi propia debilidad nº 7.
Recharts es de React y se cae. He decidido SVG propio para las cuatro con argumento (§8.4),
y el precio es honesto: **algo más de una semana de proyecto frente a unos tres días** con
una librería madura. Lo acoto con un esqueleto compartido, con las cuatro pruebas de
comportamiento escritas el primer día y con un plan de repliegue que ya existe como
requisito —la tabla accesible de §7.5—, pero acotado no es inexistente.

**−1 usabilidad · `@tanstack/svelte-query` está menos rodada.** Todo el modelo de carga de
§4.1 —`staleTime` por naturaleza del dato, deduplicado, invalidación por prefijo— se apoya
en esa librería, y la versión de Svelte tiene menos usuarios y menos respuestas escritas
que la de React. Es la pieza de la que depende que la latencia se sienta como se ha
diseñado, así que el riesgo cae justo en mi columna. No lo compenso con nada, y por eso la
nota baja.

Neto: **UX 87 (sin cambio, con dos movimientos que se anulan) y usabilidad 88 (−1)**. Que
la nota baje un punto no es una objeción al arbitraje: es lo que pasa cuando se optimiza un
criterio y el coste cae en otro. Decir que no ha costado nada sería el tipo de conformidad
que hace inútil una puntuación.

### 9.2 Lo que sigue abierto, y por qué no llega más arriba

Trece puntos de UX y once de usabilidad siguen sin estar, y sé de dónde salen.

**1. No hay observación de usuarios, y no la va a haber.** Es la que más pesa y es la que no
se puede compensar con rigor: cada decisión de este documento es razonamiento, y el
razonamiento acierta menos de lo que cree. Concretamente, tengo tres apuestas que sólo se
resuelven mirando a alguien: si la hoja de comando encanta veinte veces y estorba a las
cien; si el rail de servidores merece sus 56 píxeles con un solo servidor, que es el caso
del 80 % de los usuarios; y si el orden por gravedad de la lista desorienta a quien tenía
memorizada la posición de sus apps. **Techo estructural: −6 UX, −4 usabilidad.** Ninguna
propuesta sin observación debería puntuarse por encima de 90.

**2. `orbit new` sigue sin contrato, y eso tiene un coste que he tapado, no eliminado.**
La solución de volver a preguntar con `info --json` es la mejor disponible y sigue siendo
una inferencia: la interfaz deduce el final por el estado resultante, no por lo que el
comando dijo. Hay un caso que no distingue bien —una app creada cuyo despliegue falló y otra
creada cuyo despliegue ni se intentó se ven igual desde `info`— y lo resuelvo enseñando las
dos hipótesis. Es honesto y es peor que un `new --json`. **−2 UX.**

**3. `orbit logs` no tiene contrato tampoco, y la pantalla de logs es la segunda más
visitada de cualquier gestor de despliegues.** Está diseñada como una superficie de texto
con filtros encima, y eso es todo lo que se puede hacer sin estructura. Una pantalla que
sabe qué es una línea de error, qué es un código HTTP y qué es un reinicio necesitaría que
el servidor lo dijera, o un analizador propio por cada formato —journal, nginx access,
nginx error—, que es un analizador de texto de los que §13.1 avisa. No lo he resuelto y no
creo que se deba resolver por este lado. **−2 UX.**

**4. El diagnóstico depende de un arreglo que no controlo.** La pantalla es honesta hoy y
está preparada para mañana, pero mientras el PR no aterrice, la acción principal de una
pantalla de diagnóstico es **copiar un comando y salir de la aplicación**. Es lo correcto y
no es bueno. **−2 usabilidad.**

**5. Windows es un producto peor y lo he documentado, no arreglado.** Sin `ControlMaster`,
la portada pasa de 400 ms a cerca del segundo y el detalle de app de 100 a 500. La interfaz
lo dice y ajusta sus umbrales, que es lo máximo que puede hacer desde mi lado, pero un
tercio de los usuarios potenciales tendrá una aplicación mensurablemente más lenta.
**−2 usabilidad.**

**6. Las gráficas son un riesgo de calendario entero otra vez.** Con el arbitraje de la
ronda 3 se caen las dos que había delegado, y las cuatro vuelven a ser código propio. El
tamaño está contado en §8.4 —una semana larga frente a tres días— y el repliegue está
escrito, pero es la partida del proyecto que más probable es que se recorte si el
calendario aprieta, y sigue siendo la que yo mismo señalé en la ronda 1. **−2 UX.**

**6b. Y una nueva de la ronda 3: la capa de caché es la menos rodada del stack.** §4.1 es
la sección de la que más depende que el producto se sienta rápido, y descansa entera en
`@tanstack/svelte-query`. Está acotado —son las cuatro funciones más básicas de esa
librería y se pueden sustituir por un store propio sin tocar pantallas— y sigue siendo el
único sitio donde una decisión de stack puede degradar una decisión de experiencia.
**−1 usabilidad.**

**7. El modo incidente sigue sin cronometrarse.** Dije que la portada debe contestar el modo
rutina en menos de un segundo y ser la puerta más corta al modo incidente. Lo primero ahora
tiene número (389 ms de servidor). Lo segundo no: no he contado los clics de «me llaman
porque la tienda no carga» hasta «sé qué pasa», ni he comprobado que las tres rutas posibles
—logs, diagnóstico, último despliegue— converjan. Es un ejercicio de una tarde y no lo he
hecho. **−2 UX.**

**8. Y una nueva, que ha aparecido al cerrar E:** he contestado que no al despliegue en
grupo con un argumento que creo bueno, y no he diseñado qué pasa cuando el usuario lo pide
igualmente. La respuesta «usa la paleta dos veces» es cierta y es la clase de respuesta que
hace que alguien escriba un script y se salte la interfaz. **−1 UX, −1 usabilidad.**

### 9.3 Qué haría falta para pasar de 87/88

| Acción | UX | Usab. | Nota |
|---|---|---|---|
| Cinco sesiones observadas con usuarios reales | **+5** | **+4** | Descartado por el encargo. Es el techo. |
| Cronometrar el modo incidente en un prototipo navegable | +2 | +1 | Una tarde |
| PR a `orbit` para `doctor --fix --json --yes` | — | +2 | Tres líneas en el servidor |
| Proponer `orbit new --json` en el roadmap de Orbit | +2 | — | Cierra la debilidad 2 de verdad |
| Diseñar el despliegue secuencial multi-servidor guiado | +1 | +1 | Sin grupo persistente |
| Medir el arranque y el refresco en los tres WebView | — | +1 | CI |
| Escribir las cuatro gráficas la primera semana, no la última | +2 | — | Retira la debilidad 6 quitándola del camino crítico |

Sin observación de usuarios, el techo realista de esta propuesta está en **92 / 94**, y
llegar ahí es trabajo de detalle sobre lo que ya está decidido. Digo el techo porque una
propuesta que se puntúa 98 sin haber sido usada por nadie no se está evaluando, se está
promocionando.

### 9.4 Lo que este documento fija y lo que deja abierto

**Fijado en la ronda 1 y no tocado en la ronda 2** (es lo mejor del documento y cambiarlo
requiere argumento escrito): la paleta y sus tokens; la tabla de estados y su orden de
precedencia; los dos glifos neutros `—` y `·`; la regla de que el color nunca es el único
portador; la prohibición de agrupar los seis finales del lote; la prohibición del *spinner*
infinito; que el entorno sólo enseña nombres; y que la interfaz sólo invoca `orbit` y lo
enseña antes de hacerlo.

**Fijado en la ronda 3:** Svelte 5 con CSS de tokens, Melt UI para las cinco primitivas y
SVG propio para las cuatro gráficas; que el estado semántico vive en `estado.css` y que un
color literal fuera de la capa de tokens **falla el build**; y las tres condiciones que
sobreviven a cualquier framework —8 ms por refresco con 40 filas medidos en los tres
WebView, sin virtualización por debajo de 200 filas, y el arranque en frío publicado en CI.

**Fijado en la ronda 2:** la portada se alimenta de `status --json` y de nada más; los
umbrales de carga de §4.1 y su versión de Windows; los siete finales de `orbit new` y la
regla de volver a preguntar en vez de analizar prosa; que el diagnóstico no tiene botón
mientras el servidor no lo permita; que no hay despliegue en grupo ni lista global; que
`servidor : app` es la unidad de identidad en toda la interfaz, incluidas las notificaciones
y los anuncios de lector; la regla del 70 % de la lista; el orden de lectura del diálogo
destructivo; y que las utilidades de color no salen de `estado.css`, comprobado por lint.

**Abierto, y con dueño:** la pantalla de logs (limitada por el contrato, no por el diseño);
qué se hace cuando alguien pide despliegue en grupo por tercera vez; el cronometraje del
modo incidente; y si la hoja de comando debe poder desactivarse globalmente o sólo por
acción, que sigue sin poder contestarse sin ver a alguien usarla cien veces.
