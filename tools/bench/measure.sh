#!/usr/bin/env bash
# Mide la latencia del contrato contra el banco. Imprime medianas, no medias:
# un build que una vez tardó 400 s no describe ningún despliegue real, y la
# misma lógica vale aquí (ROADMAP de Orbit, métricas de despliegue).
#
#   uso:  tools/bench/measure.sh [ruta del banco] [repeticiones]
set -Eeuo pipefail

BENCH="${1:-$PWD/.bench}"
REPS="${2:-7}"
[[ -x "$BENCH/orbit" ]] || { echo "no hay banco en $BENCH; corre make-bench.sh" >&2; exit 1; }

export ETC_DIR="$BENCH/etc" LOG_FILE="$BENCH/log/orbit.log" APPS_DIR="$BENCH/srv"

med() {
  local etiqueta="$1"; shift
  local -a t=()
  local i s e
  for (( i = 0; i < REPS; i++ )); do
    s=$(date +%s%N); "$@" >/dev/null 2>&1 || true; e=$(date +%s%N)
    t+=( $(( (e - s) / 1000000 )) )
  done
  printf '%-28s ' "$etiqueta"
  printf '%s\n' "${t[@]}" | sort -n | \
    awk '{a[NR]=$1} END {printf "%5s ms   (min %s / max %s, n=%s)\n", a[int((NR+1)/2)], a[1], a[NR], NR}'
}

apps=$(ls "$BENCH/etc/apps" | wc -l)
echo "Banco: $apps apps · $(nproc) vCPU · $REPS repeticiones · mediana"
echo
med "orbit version --json"  "$BENCH/orbit" version --json
med "orbit list --json"     "$BENCH/orbit" list --json
med "orbit list"            "$BENCH/orbit" list
med "orbit status --json"   "$BENCH/orbit" status --json
med "orbit info app001 --json" "$BENCH/orbit" info app001 --json
echo
echo "El suelo es 'version --json': no lee ninguna app, así que es lo que"
echo "cuesta parsear el script. Se paga en cada pantalla, antes de la red."
