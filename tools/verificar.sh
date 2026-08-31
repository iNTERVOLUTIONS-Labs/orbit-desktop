#!/usr/bin/env bash
# Verifica todo, y **falla** si algo falla.
#
# Existe por un motivo concreto y vergonzoso: dos veces en esta misma sesión
# publiqué con clippy roto porque miré la última línea de su salida —o el número
# de errores impreso al lado de un «(0 = limpio)»— en vez del código de salida.
# Un resumen que hay que leer bien es un resumen que se lee mal, así que esto no
# imprime números que interpretar: o sale con 0, o dice qué se rompió.
#
#   uso:  tools/verificar.sh [--e2e]
set -Eeuo pipefail

fallos=()
paso() { # paso <nombre> <orden…>
  local n="$1"; shift
  if "$@" >/tmp/verificar.log 2>&1; then
    printf '  ok    %s\n' "$n"
  else
    printf '  FALLO %s\n' "$n"
    tail -25 /tmp/verificar.log | sed 's/^/        /'
    fallos+=("$n")
  fi
}

echo "Núcleo"
paso "formato"    cargo fmt --all -- --check
paso "clippy"     cargo clippy --all-targets --all-features -- -D warnings
paso "pruebas"    cargo test --all
paso "excepciones de auditoría" ./tools/caducidad-excepciones.sh

echo "Interfaz"
paso "tipos"      npx --prefix ui svelte-check --tsconfig ./ui/tsconfig.json --threshold error
paso "pruebas"    npm --prefix ui run test
paso "build"      npm --prefix ui run build

if [[ "${1:-}" == "--e2e" ]]; then
  echo "De punta a punta"
  paso "capturas"  npm --prefix ui run test:visual
fi

echo
if (( ${#fallos[@]} )); then
  printf 'FALLA: %s\n' "${fallos[*]}" >&2
  exit 1
fi
echo "Todo en verde."
