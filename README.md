<p align="center">
  <strong>Orbit Desktop</strong><br>
  La interfaz con ratón de <a href="https://github.com/iNTERVOLUTIONS-Labs/orbit">Orbit</a>,
  que corre en tu portátil y no en tu servidor.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/licencia-MIT-5EE7E7?style=flat-square" alt="MIT">
  <img src="https://img.shields.io/badge/estado-dise%C3%B1o-7C6CF0?style=flat-square" alt="En diseño">
  <img src="https://img.shields.io/badge/contrato-orbit%20--json%20v1-3AC7D8?style=flat-square" alt="Contrato 1">
</p>

---

> **Estado: en diseño.** Todavía no hay aplicación. Lo que hay es la biblia
> técnica: la arquitectura, el contrato, el sistema de diseño, el modelo de
> amenazas y el plan de pruebas, escritos antes que el código y a propósito.
> Empieza por **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**.

Orbit despliega tus webs en tu VPS con un comando, y [se niega a tener un panel
web](https://github.com/iNTERVOLUTIONS-Labs/orbit/blob/main/docs/ARCHITECTURE.md).
El motivo está escrito en su arquitectura y es bueno: `orbit` se auto-eleva a
root y `orbit exec` ejecuta comandos arbitrarios, así que un panel encima de eso
no sería un panel, sería una shell de root expuesta a internet — la clase de
producto más atacada del hosting.

Orbit Desktop es la otra respuesta a la misma pregunta. Una aplicación que corre
**en tu portátil**, entra por **SSH con tus claves de siempre** y ejecuta
`orbit … --json`. El servidor no gana un proceso, ni un puerto, ni un byte de
estado: tiene exactamente el mismo estatus que tu terminal. Por eso puede existir
cuando un panel no puede.

## Las cinco reglas que no se rompen

Vienen del repositorio de Orbit, están escritas allí antes que este código, y
aquí se dan por cerradas.

1. **La interfaz nunca escribe en `/etc/nginx`, `/etc/orbit` ni systemd. Sólo
   invoca `orbit`.** El día que genere un vhost por su cuenta habrá dos verdades
   sobre cómo se despliega. Hay además un motivo de seguridad: el `.conf` de una
   app se carga con `.` desde Bash, así que es **código que se ejecuta como
   root** — escribir ahí desde una ventana no es editar datos.
2. **El servidor no gana nada.** Ni un proceso, ni un puerto, ni un fichero. Si
   Orbit Desktop desaparece mañana, el servidor no se entera.
3. **Habla SSH, no una API propia.** Tus claves, tu `ssh-agent`, tu
   `~/.ssh/config` con su `ProxyJump`. No inventamos autenticación y no
   reimplementamos un cliente SSH: delegamos en el `ssh` del sistema, que es lo
   que hacen VS Code Remote-SSH y `git`.
4. **Los secretos no cruzan el contrato.** `orbit env list` da los **nombres** de
   las variables. Un valor se pide de uno en uno y a propósito, porque un panel
   que enseña el `.env` entero filtra la contraseña de la base de datos en la
   primera captura que alguien pegue en un issue.
5. **Lo que no se sabe no se pinta como un valor.** Un `null` no es un cero.
   «No he podido preguntar» no es «no hay cambios». Un cero es una afirmación.

## Qué resuelve, y qué no

**Resuelve** mirar y operar varios servidores de Orbit sin acordarse de la
sintaxis, ver un despliegue mientras ocurre en vez de leer un bloque de texto al
final, y darse cuenta de que una web lleva dos días sin servirse antes de que lo
diga un cliente.

**No resuelve** —y no lo va a intentar— administrar un servidor que no tenga
Orbit, sustituir al terminal para lo que el terminal hace mejor, ni existir en el
móvil: una shell de root en un teléfono es exactamente lo que este proyecto
existe para no construir.

## Multiservidor, gratis

Un cliente que habla SSH con varios servidores **es** el `orbit remote add` de la
v2.0 de Orbit, sin plano de control y sin demonio. Sale de la arquitectura, no de
una función que haya que escribir.

## La documentación

| | |
|---|---|
| **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** | La biblia. Empieza aquí: qué se construye, con qué, y por qué se descartó lo demás |
| **[docs/CONTRACT.md](docs/CONTRACT.md)** | El contrato `--json` tal y como **es**, auditado ejecutando el script y no leyéndolo |
| **[docs/CLIENT.md](docs/CLIENT.md)** | Stack, transporte SSH, modelo de datos, caché y distribución |
| **[docs/DESIGN.md](docs/DESIGN.md)** | Sistema de diseño, mapa de pantallas, los estados difíciles y el despliegue en vivo |
| **[docs/THREAT-MODEL.md](docs/THREAT-MODEL.md)** | Modelo de amenazas y las reglas duras del cliente |
| **[docs/QA.md](docs/QA.md)** | Plan de pruebas, catálogo de respuestas patológicas y criterios de aceptación |
| **[ROADMAP.md](ROADMAP.md)** | Las fases, y lo que queda fuera |
| **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** | Cómo se monta el entorno y cómo se prueba sin un VPS |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | Cómo se trabaja aquí |

## Lo que ya se puede ejecutar

Todavía no hay aplicación, pero sí hay dos herramientas, y las dos existen
porque una cifra que nadie ha medido no es una cifra.

**El banco de 40 apps.** Monta un Orbit de mentira y mide la latencia del
contrato, que es la latencia de la interfaz:

```bash
tools/bench/make-bench.sh ../orbit 40
tools/bench/measure.sh
```

**La prueba de propiedad del escapado**, que es la pieza de la que depende que un
nombre de app no se convierta en un comando:

```bash
python3 tests/escaping/prop_test.py 20260830 2500
```

Corre contra `bash`, `dash`, `zsh` y `busybox ash`. En su primera ejecución
encontró un fallo real que sólo se daba en zsh; está contado en
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Licencia

MIT, como Orbit.

## Créditos

Hecho por **[Intervolutions](https://intervolutions.com)**.
