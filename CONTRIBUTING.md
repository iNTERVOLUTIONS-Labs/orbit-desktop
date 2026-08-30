# Contribuir a Orbit Desktop

Las contribuciones son bienvenidas. Este documento dice lo que hay que saber
antes de escribir la primera línea, y está ordenado por lo que más veces se
olvida.

## Antes que nada: lee las cinco reglas

Están en el [README](README.md) y desarrolladas en
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). No son estilo, son la frontera del
producto. Un PR que las rompa se rechaza aunque el código sea bueno, y el
rechazo no es negociable en la revisión: si crees que una regla está mal, el sitio
para discutirlo es un issue, no un PR.

La que más veces se va a intentar romper, y por eso va primero:

> **La interfaz nunca escribe en `/etc/nginx`, `/etc/orbit` ni systemd. Sólo
> invoca `orbit`.**

Siempre habrá una tarea que sería más fácil escribiendo un fichero por SSH que
esperando a que `orbit` gane un comando. Hacerlo crea dos verdades sobre cómo se
despliega una web, y a partir de ahí el cliente y el servidor discrepan en
silencio. Si falta un comando, el PR va **al repositorio de Orbit**.

## El contrato manda

Todo lo que el cliente sabe del servidor viene de `orbit … --json`. Tres
consecuencias prácticas:

- **Ningún dato se saca con `orbit exec`.** Ni para leer un fichero, ni para
  contar releases, ni para tapar un hueco del contrato. El día que se haga, el
  cliente deja de hablar el contrato y pasa a hablar Bash contra un servidor
  cuyo layout puede cambiar.
- **Los campos se añaden, nunca se renombran** — es la promesa de Orbit, y
  nuestra mitad es cumplirla: un campo desconocido se ignora sin ruido; un campo
  esperado que falte o venga con otro tipo es un **error de la respuesta**, no un
  valor por defecto.
- **Cuando el contrato y la documentación de Orbit discrepan, manda el
  comportamiento.** Ya ha pasado: `orbit doctor --fix --json --yes` está
  documentado y no existe. Se comprueba ejecutando, no leyendo.

## Reglas de código que no se discuten en la revisión

Estas están automatizadas a propósito. Una regla que depende de que un revisor se
acuerde no sobrevive al commit 400.

1. **Ningún comando remoto se construye concatenando cadenas.** Toda orden es una
   lista de argumentos y se serializa en un único módulo. Sólo ese módulo puede
   invocar `ssh`; el resto del árbol falla en CI si lo menciona.
2. **Todo argumento pasa por el escapador.** Y el escapador tiene una prueba de
   propiedad contra cuatro shells que **tiene que seguir en verde**. Si la tocas,
   córrela: `python3 tests/escaping/prop_test.py`.
3. **Ningún dato del servidor se inserta como marcado.** Nada de `innerHTML` ni
   equivalentes sobre datos del contrato. Un nombre de app puede traer
   `</script>` — el servidor produce JSON válido, no HTML seguro, y hace bien.
4. **Toda orden lleva el nombre de la app explícito.** Sin TTY, `orbit info` sin
   app no aborta: elige la primera por orden alfabético y sale con 0. Con
   `restart` eso es reiniciar la app equivocada sin que nada lo diga.
5. **`orbit` se invoca por ruta absoluta**, nunca por `PATH`.
6. **Un dato que no se ha podido obtener no se pinta como un valor.**

## Antes de mandar un PR

- La prueba de propiedad del escapado en verde, con la semilla en la salida.
- Ningún secreto en disco: el barrido de `tests/` no encuentra nada.
- Sin violaciones de accesibilidad de nivel *serious* o *critical*.
- Si tocas algo que se mide, **mídelo antes y después** y pon las dos cifras. Una
  cifra dentro de un comentario no la comprueba nadie, y Orbit ya se llevó ese
  disgusto: dos números de su propia documentación resultaron falsos y se cayeron
  al medir de verdad.

## Idioma

El producto habla **español e inglés**, como Orbit. La documentación se escribe
en español. Los mensajes de commit, en español, en imperativo y sin adornos.

Y el estilo de la documentación, que es el de Orbit y conviene respetarlo: cada
decisión dice **por qué** se tomó y **qué se descartó**. Un documento que sólo
lista lo que hay no le sirve a quien tenga que cambiarlo dentro de dos años.

## Lo que no se acepta

- Una versión web de esto. Es la razón de ser del proyecto.
- Una versión móvil. Una shell de root en un teléfono.
- Telemetría que contenga nombres de dominio, de app, hostnames, IPs o rutas. Los
  dominios de un cliente de agencia **son** su cartera de clientes.
- Guardar contraseñas, frases de paso o claves privadas. En ningún soporte, ni
  cifrado. Lo que necesite una credencial la pide al `ssh-agent`.
