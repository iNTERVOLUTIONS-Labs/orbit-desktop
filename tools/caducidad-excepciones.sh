#!/usr/bin/env bash
# Una excepción sin caducidad se convierte en permanente en tres meses, y a
# partir de ahí el escaneo de dependencias sigue saliendo en verde y ha dejado
# de mirar. Esto es lo que impide que pase: si la fecha declarada en
# `.cargo/audit.toml` ya pasó, la tanda falla y hay que volver a pensarlo.
#
# Va aparte y en cinco líneas a propósito: una política que sólo está escrita en
# un documento es una intención.
set -Eeuo pipefail

F="${1:-.cargo/audit.toml}"
[[ -r "$F" ]] || { echo "no hay $F: nada que caducar"; exit 0; }

FECHA="$(grep -oE '^# CADUCA: [0-9]{4}-[0-9]{2}-[0-9]{2}' "$F" | awk '{print $3}')"
[[ -n "$FECHA" ]] || {
  echo "$F tiene excepciones y no declara «# CADUCA: AAAA-MM-DD»." >&2
  echo "Una excepción sin fecha es una excepción permanente." >&2
  exit 1
}

HOY="$(date -u +%F)"
if [[ "$HOY" > "$FECHA" ]]; then
  echo "Las excepciones de la auditoría caducaron el $FECHA (hoy es $HOY)." >&2
  echo "Toca mirar si siguen haciendo falta, no ampliar la fecha por inercia." >&2
  exit 1
fi
echo "Excepciones de auditoría vigentes hasta $FECHA."
