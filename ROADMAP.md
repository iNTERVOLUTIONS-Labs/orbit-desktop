# Roadmap de Orbit Desktop

Leyenda: ✅ hecho · 🚧 en curso · 📋 planificado · 💭 en debate

El criterio que ordena todo lo de abajo: **cada fase termina en algo que se puede
enseñar y usar.** Ninguna fase es «la arquitectura». Esa ya está escrita, y está
en [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Fase 0 · La biblia 🚧

Escribir las decisiones antes que el código, que es lo que hizo Orbit con su
§13 y por lo que este repositorio existe con esa forma.

- ✅ **Auditoría del contrato `--json` ejecutando el script**, no leyéndolo. De
  ahí salieron cosas que llevaban documentadas y rotas: `orbit doctor --fix
  --json --yes` no existe, y sin TTY un comando sin app no aborta sino que elige
  la primera por orden alfabético y sale con 0
- ✅ **Elección de stack** con las cinco alternativas descartadas por su motivo, y
  la subdecisión que de verdad manda: quién interpreta `~/.ssh/config`
- ✅ **Sistema de diseño** con la paleta real de la marca sacada de los SVG de
  Orbit y los contrastes **calculados**, no estimados
- ✅ **Modelo de amenazas** con un actor que casi nadie modela: el servidor
  gestionado que devuelve lo que quiera al cliente que tiene las llaves de los
  otros cuatro
- ✅ **Plan de pruebas** con el catálogo de respuestas patológicas, que es la
  parte que caza los bugs de verdad
- ✅ **El banco de 40 apps**, para que las cifras de latencia sean medidas
- ✅ **La prueba de propiedad del escapado** contra cuatro shells. Encontró un
  fallo real en su primera ejecución, y sólo en zsh

## Fase 1 · Ver 📋

Un servidor, leído de `~/.ssh/config` por su alias. El transporte con
`ControlMaster`. `version`, `status`, `list`, `info`. Dos pantallas.

**No entra ni un botón que cambie nada.** Ninguna mutación, ni multiservidor, ni
logs, ni despliegue, ni actualizador.

Se puede empezar por aquí porque `status --json` trae el array de apps
**completo e idéntico** al de `list --json` —comprobado comparando los dos
objetos—, así que una sola llamada pinta la pantalla que ya da valor el primer
día: quién está caída, quién está en mantenimiento y, lo que ninguna otra
herramienta enseña, **quién está registrada pero sin vhost**, que es el fallo que
no ve ni `nginx -t` ni `orbit list`.

**Terminado cuando:** contra un VPS de verdad la lista carga en menos de 500 ms
con la conexión caliente, y contra el servidor falso se ven en pantalla las ocho
clasificaciones de estado.

## Fase 2 · Desplegar 📋

`deploy <app> --json --progress` con la barra de pasos y la estimación sacada de
`build_median_s`. `deploy --all` con los **seis** finales pintados separados.
`rollback`, `restart/start/stop`, `maintenance`.

Es el núcleo. Y `deploy --all` es la pantalla que justifica el proyecto entero:
confundir «no hay cambios» con «no he podido preguntar» costó un fallo real en
la versión en prosa de Orbit —un remoto caído se anunciaba como «nada que hacer»
cada cinco minutos— y el contrato tiene seis finales justamente para que un
cliente no pueda repetirlo. **Agruparlos en «correctas / fallidas» está
prohibido.**

## Fase 3 · Operar 📋

Logs en ventana y en vivo. `doctor` con botón **sólo** donde `fixable` lo
permita. Variables de entorno, con los nombres a la vista y cada valor tras un
gesto deliberado. `top` refrescándose. Métricas, tráfico, y la terminal de
`exec`.

**Y aquí se abre el PR contra `orbit`**, que es el trabajo más rentable de todo
el proyecto y no es trabajo de este repositorio: `--json` en `logs` y en
`backup list`, y el arreglo de `--yes` en `doctor --fix --json`. Tres huecos del
contrato desaparecen y el cliente se queda sin una sola línea que parsee texto.
Es el momento correcto para abrirlo: ya se sabe exactamente qué falta porque se
ha intentado usar.

## Fase 4 · Administrar 📋

Multiservidor completo con importación de `~/.ssh/config`. El asistente de web
nueva. Retirar una app, con la confirmación reforzada que el servidor ya no da:
`orbit remove -y --purge` **no pregunta nada**, porque la pregunta vivía en el
terminal que este cliente sustituye. Certificados, redirecciones, bases de datos,
copias, colas y vigilancia.

## Fase 5 · Distribuir 📋

Firma en las tres plataformas, notarización, y el actualizador con verificación
de firma y la clave pública empotrada en el binario.

**Va al final, con una excepción.** Firmar y notarizar es un trabajo que no se
paraleliza y que no aporta nada mientras no haya producto; hacerlo con el
producto terminado permite además probar una actualización de verdad en vez de
con una app vacía. La excepción: **las cuentas de firma se piden en la fase 1**,
porque si Apple o Azure tardan, tardan.

## Fase 6 y más allá 💭

- 💭 **Comparar dos servidores** — qué tiene uno que no tenga el otro
- 💭 **Acciones en lote sobre varias apps**
- 💭 **Gráficas históricas**, que exigirían que el cliente guarde algo. Es una
  decisión aparte y no pequeña: hoy el cliente no persiste ningún dato del
  servidor, y eso es media hoja del modelo de amenazas
- 💭 **Integración con el `orbit.json` de `orbit init`**
- 💭 **Una TUI** que reutilice el mismo núcleo. Es gratis si el núcleo vive en un
  crate aparte de la interfaz, y por eso vive así

---

## Fuera de alcance

Para que nadie pierda el tiempo proponiéndolo.

- **Una versión web.** Es la razón de ser del proyecto. Orbit descartó el panel
  web porque `orbit` se auto-eleva a root y `orbit exec` ejecuta cualquier cosa;
  poner esta aplicación en un puerto reconstruye exactamente lo que se evitó.
- **Una versión móvil.** Lo mismo, en un aparato que se pierde. El escenario «me
  escriben a las once y no tengo el portátil» es real y **ya tiene respuesta en
  Orbit**: el panel HTML estático de solo lectura, regenerado por el temporizador
  de `watch` y protegido con Cloudflare Access. Esta aplicación no lo sustituye.
- **Escribir en `/etc/nginx`, `/etc/orbit` o systemd.** Nunca, por nada. Si falta
  un comando, el PR va al repositorio de Orbit.
- **Sacar datos con `orbit exec`** para tapar un hueco del contrato. El día que
  se haga, el cliente deja de hablar el contrato y pasa a hablar Bash contra un
  servidor cuyo layout puede cambiar.
- **Guardar contraseñas, frases de paso o claves privadas.** En ningún soporte,
  ni cifrado. Cifrar con una clave que también está en el disco es una ofuscación
  con nombre de cifrado.
- **Telemetría con nombres de dominio, de app, hostnames, IPs o rutas.** Los
  dominios de un cliente de agencia son su cartera de clientes.
- **Gestionar servidores que no tengan Orbit.** Hay herramientas mejores.
- **Un agente en el servidor.** Ni para acelerar, ni para cachear, ni para nada.
  El servidor no gana un byte de estado, y esa es la característica.

---

## Cómo influir en esto

Abre un issue describiendo **el problema que tienes**, no la solución que
imaginas. Y lee [CONTRIBUTING.md](CONTRIBUTING.md) antes de escribir código: hay
cinco reglas que no se discuten en la revisión de un PR.
