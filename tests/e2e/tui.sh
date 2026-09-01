#!/usr/bin/env bash
# ============================================================================
#  El abanico del cliente de terminal, contra un sshd de verdad.
#
#  Es lo único que este programa hace y que `ssh servidor orbit list` no puede:
#  preguntarle a varios servidores a la vez. Así que es lo único que hay que
#  probar de punta a punta, y hay que probarlo con **más de un servidor** —con
#  uno, el abanico no existe y la prueba pasaría sin comprobar nada.
#
#  Los tres casos que tiene que distinguir, y que son tres y no dos:
#
#    · un servidor con apps
#    · un servidor sin ninguna app          → «sin apps todavía»
#    · un servidor que no contesta          → «no contesta», NUNCA cero apps
#
#  El tercero es el que importa. Confundirlo con el segundo es el fallo que ya
#  costó que un remoto caído se anunciara como «nada que hacer» durante días, y
#  en una tabla de diez servidores es mucho más fácil de cometer: basta con
#  dejar la fila en blanco.
#
#    uso:  tests/e2e/tui.sh <directorio del banco>
# ============================================================================
set -Eeuo pipefail

DIR="${1:?uso: tui.sh <directorio del banco>}"
RAIZ="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"
BIN="${ORBIT_TUI:-$RAIZ/target/debug/orbit-desktop}"
# El «orbit» del otro lado es el servidor falso del banco, no el de /usr/local.
ORB="$DIR/fakeserver/orbit"
PUERTO="${PUERTO:-2222}"

[[ -x "$BIN" ]] || { echo "no encuentro el binario en $BIN — cargo build -p orbit-tui" >&2; exit 1; }
[[ -d "$DIR" ]] || { echo "no hay banco en $DIR — tests/e2e/montar-sshd.sh $DIR" >&2; exit 1; }

# Un HOME propio con su ~/.ssh/config, que es de donde salen los servidores. No
# se toca el del usuario: la misma regla que el banco de sshd.
# El fichero se le pasa con `-F` y no con HOME: `ssh` resuelve el `~` con la
# entrada de passwd y no con la variable, así que un HOME de mentira no le dice
# nada. Es exactamente para esto para lo que existe `-F`.
CFG="$DIR/config-tui"

# Tres alias. Los dos primeros apuntan al mismo sshd —lo que se prueba es el
# abanico, no que haya dos máquinas— y el tercero a un puerto donde no hay
# nadie, que es como se consigue un servidor mudo sin apagar nada.
cat > "$CFG" <<EOF
Host con-apps sin-apps
  HostName 127.0.0.1
  Port $PUERTO
  User $USER
  IdentityFile $DIR/cliente
  UserKnownHostsFile $DIR/known_hosts
  StrictHostKeyChecking yes
  BatchMode yes
  SendEnv ORBIT_FAKE_CASE

Host mudo
  HostName 127.0.0.1
  Port 1
  User $USER
  IdentityFile $DIR/cliente
  UserKnownHostsFile $DIR/known_hosts
  BatchMode yes
  ConnectTimeout 2
EOF
chmod 0600 "$CFG"

fallos=0
comprobar() { # comprobar <qué> <esperado> <texto>
  if grep -qF -- "$2" <<<"$3"; then
    printf '  ok    %s\n' "$1"
  else
    printf '  FALLA %s\n        esperaba encontrar: %s\n' "$1" "$2"
    fallos=$((fallos + 1))
  fi
}
comprobar_no() { # lo contrario, para lo que NO se puede decir
  if grep -qF -- "$2" <<<"$3"; then
    printf '  FALLA %s\n        no debería aparecer: %s\n' "$1" "$2"
    fallos=$((fallos + 1))
  else
    printf '  ok    %s\n' "$1"
  fi
}

echo "El abanico, con tres servidores"

# `sin-apps` contesta la lista vacía; los otros dos, el caso sano. El caso se
# elige por variable de entorno, que el sshd del banco deja cruzar.
salida="$(NO_COLOR=1 ORBIT_FAKE_CASE=sano "$BIN" -F "$CFG" --orbit "$ORB" estado con-apps mudo 2>&1)" || rc=$?
rc="${rc:-0}"

comprobar "el servidor que contesta trae sus apps"   "con-apps"      "$salida"
comprobar "el que no contesta lo DICE"               "no contesta"   "$salida"
comprobar_no "y no sale como cero apps"              "mudo  0"       "$salida"
comprobar "el motivo va con él, sin interpretarlo"   "mudo"          "$salida"
comprobar "y el recuento de mudos va aparte"         "1 servidor no contesta. Lo que tengan no se sabe." "$salida"

# Un servidor mudo no es un error del programa: la respuesta es correcta.
if [[ "$rc" == "3" ]]; then
  printf '  ok    un servidor mudo sale con 3, que no es el 1 de «no he podido ni empezar»\n'
else
  printf '  FALLA el código de salida con un servidor mudo era %s y esperaba 3\n' "$rc"
  fallos=$((fallos + 1))
fi

echo
echo "Un servidor sin ninguna app"
vacio="$(NO_COLOR=1 ORBIT_FAKE_CASE=sin-apps "$BIN" -F "$CFG" --orbit "$ORB" estado sin-apps 2>&1)" || true
comprobar "«sin apps» es una respuesta y se dice"    "sin apps todavía" "$vacio"
comprobar_no "y NO se confunde con no contestar"     "no contesta"      "$vacio"

echo
echo "El vocabulario es el del núcleo"
# Las palabras salen de `tests/contrato/vocabulario.json` a través del núcleo, y
# no de una copia en este programa. Se comprueba una de cada clase.
estados="$(NO_COLOR=1 ORBIT_FAKE_CASE=estados "$BIN" -F "$CFG" --orbit "$ORB" estado con-apps 2>&1)" || true
for palabra in "sin vhost" "mantenimiento" "activo"; do
  comprobar "«$palabra» sale tal cual del vocabulario" "$palabra" "$estados"
done
# Y el defecto que la ventana ya cazó en una captura, y que este programa
# cometió otra vez a la primera: en los dos estados neutros el texto ES el
# glifo, así que pintarlos los dos deja una fila que dice «— —».
comprobar_no "el glifo no se pinta dos veces" "— —" "$estados"
comprobar_no "ni el del estado desconocido"   "· ·" "$estados"

echo
echo "Enumerar no es visitar"
lista="$(NO_COLOR=1 "$BIN" -F "$CFG" --orbit "$ORB" servidores 2>&1)"
comprobar "los tres alias salen del fichero"  "con-apps" "$lista"
comprobar "y se dice que no se ha hablado con ninguno" "no habla con ninguno" "$lista"

echo
if (( fallos == 0 )); then
  echo "Todo en verde."
else
  echo "$fallos comprobaciones han fallado."
fi
exit $(( fallos > 0 ? 1 : 0 ))
