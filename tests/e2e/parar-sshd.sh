#!/usr/bin/env bash
# Para el sshd del banco. Se llama siempre, también cuando la tanda falla: un
# sshd huérfano escuchando en 2222 es lo que hace que la siguiente tanda falle
# por un motivo que no tiene nada que ver.
set -uo pipefail
DIR="${1:?uso: parar-sshd.sh <directorio>}"
[[ -f "$DIR/sshd.pid" ]] && sudo kill "$(cat "$DIR/sshd.pid")" 2>/dev/null
exit 0
