#!/usr/bin/env bash
# ============================================================================
#  Un sshd de verdad, para probar lo que un doble local no puede ver.
#
#  El plan de pruebas es tajante y tiene razón: **lo que sólo existe dentro de
#  un shell remoto no lo ve un doble local.** Ejecutar el binario falso en local
#  cubre el parser, los null y los seis finales en milisegundos, y no cubre nada
#  de esto:
#
#    · el escapado de argumentos de punta a punta, atravesando sshd y el shell
#      de login de verdad — que es donde se rompe
#    · known_hosts y la política de primer contacto
#    · ControlMaster: si el multiplexado funciona, y cuánto ahorra de verdad
#    · la separación de stdout y stderr sobre un canal SSH
#    · los códigos de salida de ssh frente a los de orbit, que son distintos y
#      confundirlos marca un servidor entero como caído
#
#  Levanta un sshd en un puerto alto con su propia configuración, sus propias
#  claves y su propio known_hosts. No toca nada del sistema: ni el sshd de la
#  máquina, ni el ~/.ssh del usuario.
#
#    uso:  tests/e2e/montar-sshd.sh <directorio>   -> imprime el puerto
# ============================================================================
set -Eeuo pipefail

DIR="${1:?uso: montar-sshd.sh <directorio>}"
PUERTO="${PUERTO:-2222}"

rm -rf "$DIR"; mkdir -p "$DIR"
chmod 0700 "$DIR"

# Claves propias. La del host se regenera en cada tanda a propósito: así la
# prueba de «la clave del host ha cambiado» tiene con qué cambiarla.
ssh-keygen -q -t ed25519 -N '' -f "$DIR/host_key" -C 'banco de pruebas'
ssh-keygen -q -t ed25519 -N '' -f "$DIR/cliente"   -C 'banco de pruebas'
cp "$DIR/cliente.pub" "$DIR/authorized_keys"
chmod 0600 "$DIR/authorized_keys" "$DIR/host_key" "$DIR/cliente"

# El 'orbit' del otro lado es el servidor falso. Se copia en vez de enlazarse
# para que la ruta absoluta del cliente apunte a algo suyo.
cp -r "$(dirname "$(readlink -f "$0")")/../fakeserver" "$DIR/fakeserver"
chmod +x "$DIR/fakeserver/orbit"

cat > "$DIR/sshd_config" <<EOF
Port $PUERTO
ListenAddress 127.0.0.1
HostKey $DIR/host_key
PidFile $DIR/sshd.pid
AuthorizedKeysFile $DIR/authorized_keys
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
UsePAM no
PrintMotd no
StrictModes no
# VERBOSE y no ERROR: este log sólo se lee cuando algo ha fallado, y con ERROR
# se callaba justo lo que hacía falta. Un rechazo de autenticación —«account is
# locked», que es lo que pasa en un runner de CI donde la cuenta no tiene
# contraseña— se registra a nivel INFO, así que con ERROR el fichero salía
# vacío y el fallo parecía no tener causa.
LogLevel VERBOSE
# El entorno tiene que cruzar para poder pedirle un caso patológico concreto al
# servidor falso. En un servidor de verdad esto NO está, y por eso el cliente
# no se apoya en el entorno para nada: para el idioma usa 'orbit --lang'.
AcceptEnv ORBIT_FAKE_*
EOF

# Lo que a una máquina limpia le falta, y que no se ve porque el log de sshd va
# a un fichero: sin '/run/sshd' arranca y muere con «Missing privilege
# separation directory», exit 255 y **ni una línea por stderr**. Es la clase de
# fallo que en CI parece que el script no ha hecho nada.
command -v /usr/sbin/sshd >/dev/null || {
  echo "no hay /usr/sbin/sshd: instala openssh-server" >&2; exit 1; }
sudo install -d -m 0755 /run/sshd

# Y la configuración se valida antes de arrancar: 'sshd -t' dice qué línea está
# mal, y arrancar a ciegas sólo dice que no arrancó.
sudo /usr/sbin/sshd -t -f "$DIR/sshd_config" || {
  echo "la configuración del banco no es válida" >&2; exit 1; }

sudo /usr/sbin/sshd -f "$DIR/sshd_config" -E "$DIR/sshd.log"

# Esperar a que escuche de verdad, no dormir un número mágico: preguntarle a
# algo si existe no es preguntarle si funciona.
for _ in $(seq 1 50); do
  if ssh -q -o BatchMode=yes -o StrictHostKeyChecking=no \
         -o UserKnownHostsFile=/dev/null -i "$DIR/cliente" \
         -p "$PUERTO" "$(id -un)@127.0.0.1" true 2>/dev/null; then
    echo "$PUERTO"; exit 0
  fi
  sleep 0.1
done

# Y si no llega, se enseña el log en vez de nombrarlo: un mensaje que dice
# «mira este fichero» en un CI donde nadie va a mirarlo no dice nada.
echo "el sshd de pruebas no ha llegado a escuchar. Su log:" >&2
sudo cat "$DIR/sshd.log" >&2 2>/dev/null || echo "(sin log)" >&2
echo "--- último intento de conexión ---" >&2
ssh -v -o BatchMode=yes -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null -i "$DIR/cliente" \
    -p "$PUERTO" "$(id -un)@127.0.0.1" true 2>&1 | tail -20 >&2
exit 1
