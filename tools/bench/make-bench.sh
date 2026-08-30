#!/usr/bin/env bash
# Monta un servidor Orbit de mentira con N apps, para medir el contrato sin un VPS.
#
# Por qué existe: ARCHITECTURE §13.6d de Orbit dice que «la latencia del
# contrato es la de la interfaz», y da cifras para 40 apps. Un cliente que se
# fíe de esas cifras sin reproducirlas está construyendo sobre una afirmación.
# Este banco las reproduce en un minuto y en cualquier máquina.
#
# No toca nada del sistema: parametriza ETC_DIR sobre una copia del script y
# neutraliza la auto-elevación de la línea 241. El 'orbit' original no se
# modifica — se comprueba con 'git status' en su repositorio.
#
#   uso:  tools/bench/make-bench.sh <ruta al repo de orbit> [n apps] [destino]
set -Eeuo pipefail

ORBIT_REPO="${1:?uso: make-bench.sh <ruta al repo de orbit> [n] [destino]}"
N="${2:-40}"
DEST="${3:-$PWD/.bench}"

SRC="$ORBIT_REPO/orbit"
[[ -r "$SRC" ]] || { echo "no encuentro $SRC" >&2; exit 1; }

rm -rf "$DEST"
mkdir -p "$DEST"/{etc/apps,srv,log}

# Tres sustituciones, y ninguna cambia una decisión del script:
#   · ETC_DIR y LOG_FILE pasan a aceptar el entorno, como ya hacen APPS_DIR y
#     los demás. Es la diferencia entre poder probar y no poder.
#   · La auto-elevación se apaga. En un banco no hay root que ganar.
sed -e 's|^ETC_DIR="/etc/orbit"|ETC_DIR="${ETC_DIR:-/etc/orbit}"|' \
    -e 's|^LOG_FILE="/var/log/orbit/orbit.log"|LOG_FILE="${LOG_FILE:-/var/log/orbit/orbit.log}"|' \
    -e 's|^if \[\[ \$EUID -ne 0 \]\]; then|if false; then|' \
    "$SRC" > "$DEST/orbit"
chmod +x "$DEST/orbit"

cat > "$DEST/etc/orbit.conf" <<CONF
DEPLOY_USER='deploy'
APPS_DIR='$DEST/srv'
CONF

# Cuatro tipos a propósito: 'static' y 'php' no tienen puerto ni servicio, así
# que son las que ejercitan los null del contrato. Una tanda de apps todas
# iguales mide una cosa que no existe.
i=1
while (( i <= N )); do
  n="$(printf '%03d' "$i")"
  case $(( i % 4 )) in
    0) t=node;   port=$(( 3000 + i )) ;;
    1) t=static; port="" ;;
    2) t=next;   port=$(( 3000 + i )) ;;
    3) t=php;    port="" ;;
  esac
  cat > "$DEST/etc/apps/app$n.conf" <<CONF
A_NAME='app$n'
A_TYPE='$t'
A_DOMAIN='app$n.ejemplo.com'
A_ALIASES='www.app$n.ejemplo.com'
A_REPO='usuario/app$n'
A_BRANCH='main'
A_PORT='$port'
A_BUILD='pnpm build'
A_START='pnpm start'
A_AUTODEPLOY='no'
A_QUEUE='no'
A_LASTDEPLOY='20260805-041230 abc123def456'
CONF
  mkdir -p "$DEST/srv/app$n/releases/20260805-041230" "$DEST/srv/app$n/shared"
  ln -sfn "$DEST/srv/app$n/releases/20260805-041230" "$DEST/srv/app$n/current"
  i=$(( i + 1 ))
done

echo "Banco listo en $DEST con $N apps."
echo
echo "  export ETC_DIR=$DEST/etc LOG_FILE=$DEST/log/orbit.log APPS_DIR=$DEST/srv"
echo "  $DEST/orbit list --json"
