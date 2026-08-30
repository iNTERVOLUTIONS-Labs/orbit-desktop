# Política de seguridad de Orbit Desktop

Propio y no un enlace al de Orbit, porque son dos superficies distintas: el
servidor no tiene actualizador ni interfaz gráfica, y el cliente no tiene nginx
ni certificados. Compartir el documento haría que ninguno de los dos fuera
exacto.

El análisis completo está en **[docs/THREAT-MODEL.md](docs/THREAT-MODEL.md)** y
cómo se verifica en **[docs/QA.md](docs/QA.md)**.

---

## Lo primero, porque cambia cómo hay que usar esto

> **Orbit Desktop tiene privilegios equivalentes a root en tus servidores. Por
> diseño, y eso no es un fallo: es la naturaleza del producto.**

`orbit` se auto-eleva a root —lo necesita para escribir en `/etc/nginx` y
gestionar systemd— y `orbit exec` ejecuta comandos arbitrarios. No existe un
nivel intermedio de privilegio que este cliente pueda pedir: o tienes root en ese
servidor, o no puedes usar Orbit Desktop contra él. Ni siquiera el grupo
`orbit-admin` que Orbit tiene en su roadmap cambia eso, y su propia
documentación lo dice: sigue siendo equivalente a root porque `orbit exec`
existe.

De ahí sale la frase que conviene tener delante antes de instalarlo en un
portátil que se lleva a todas partes:

> **Comprometer Orbit Desktop es comprometer todos tus servidores de producción.**
> No es un cliente de gestión. Es una llave maestra con interfaz gráfica.

La consecuencia agradable es que **la superficie de red es cero**: no abre
puertos, no escucha nada, no expone un endpoint. Todo sale por una conexión SSH
que inicias tú. Eso ahorra la categoría entera de vulnerabilidades que mató a
CentOS Web Panel (CVE-2025-48703, ejecución remota sin autenticar sobre unos
200.000 servidores).

La desagradable es que, sin una frontera de red que defender, las amenazas que
quedan —el portátil, el canal, el contrato, la cadena de suministro y la propia
pantalla— no se resuelven con un cortafuegos.

## Qué protegemos

- **Que un servidor comprometido no contamine a los demás a través del cliente.**
  Nada de lo que llega del contrato se interpreta como marcado ni entra en un
  comando sin validarse y escaparse.
- **Que un nombre de app no se convierta en un comando.** Toda orden remota se
  construye en un único módulo, como lista de argumentos, y pasa por un escapador
  con una prueba de propiedad contra `bash`, `dash`, `zsh` y `busybox ash`.
- **Que los secretos no se filtren por la pantalla.** El `.env` nunca se pinta
  entero; cada valor se pide de uno en uno y se vuelve a ocultar solo.
- **Que no se guarde nada que no tengas ya.** El cliente no almacena
  contraseñas, frases de paso, claves privadas ni tokens. En ningún soporte, ni
  cifrado.
- **Que una acción destructiva no ocurra por accidente.** `orbit remove -y
  --purge` **no pregunta nada** en el servidor, porque la pregunta vivía en el
  terminal que este cliente sustituye. Toda esa protección se traslada aquí.

## Qué NO protegemos

La parte más útil de una política de seguridad, y por eso va con el mismo peso
que la anterior.

- **Un equipo con el atacante dentro.** Si algo ejecuta código como tú y tu
  agente SSH está desbloqueado, ha ganado: puede lanzar `ssh` por su cuenta.
  Cualquier promesa de lo contrario es marketing. Lo que hacemos es no darle
  nada extra y dejar rastro.
- **Un servidor ya comprometido con root.** No podemos detectarlo, no lo
  intentamos y no vamos a fingir que sí.
- **El código que despliegas.** Igual que Orbit: no lo auditamos. Una dependencia
  maliciosa en tu repositorio se ejecuta en tu servidor.
- **Los secretos en reposo en el servidor.** Siguen en texto plano en
  `shared/.env`. Orbit Desktop no es una bóveda y no la sustituye.
- **Un compañero de equipo con acceso legítimo.** No hay control de acceso por
  rol, porque no hay ningún servidor donde ponerlo. Quien tiene la clave, puede.
- **A quien mire tu pantalla si decides enseñarla.** El modo presentación ayuda;
  no obliga.

## Cómo reportar una vulnerabilidad

**No abras un issue público.**

Escribe a **security@intervolutions.com**. Si el hallazgo es grave, pide primero
la clave pública y manda el detalle cifrado: un reporte de una vulnerabilidad
crítica enviado por correo en claro es una vulnerabilidad más.

- **Respuesta en 72 horas**, la misma promesa que da Orbit.
- Te mantenemos al corriente del arreglo y de cuándo se publica.
- **Crédito a tu nombre**, salvo que prefieras lo contrario.

Incluye, si puedes: qué versión, qué sistema operativo, los pasos para
reproducirlo y qué impacto crees que tiene. Si el fallo está en el escapado de
argumentos o en el transporte, **la semilla de la prueba de propiedad que lo
reproduce** vale más que cualquier descripción.

## Versiones con soporte

Mientras el proyecto esté en diseño no hay versiones publicadas y por tanto no
hay nada que soportar. Cuando las haya: la última menor, y arreglos de seguridad
para la anterior durante un plazo que se declarará aquí y no se moverá.

## Recomendaciones si administras producción con esto

- **Ponle frase de paso a tu clave SSH.** Sin ella, cualquier proceso que lea el
  fichero tiene root en tus servidores. Se arregla con `ssh-keygen -p`.
- **Usa el agente con caducidad**: `ssh-add -t 8h`. Una sesión que no expira es
  una llave que no se guarda.
- **No reenvíes el agente.** Orbit Desktop no lo necesita para nada, y hacerlo le
  da a un servidor la capacidad de usar tu clave mientras la sesión esté abierta.
- **Marca tus servidores de producción** en el cliente. El accidente más caro de
  un cliente multiservidor no es un ataque: es ejecutar lo correcto contra el
  servidor equivocado.
- **Deja el bloqueo por inactividad puesto.** No protege contra malware —nada lo
  hace a ese nivel— pero sí contra el portátil abierto encima de una mesa, que es
  el escenario real y frecuente.
- **Si administras servidores de terceros, deja la telemetría apagada.** Va
  apagada por defecto y nunca contiene nombres de dominio, de app, hostnames,
  IPs ni rutas; aun así, los dominios de tus clientes son su cartera y la decisión
  de no enviarlos debería ser tuya y consciente.
