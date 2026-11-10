#!/bin/sh
# Imprime su argv separado por bytes nulos: el único separador que no puede ir
# dentro de un argumento.
for a in "$@"; do printf '%s\0' "$a"; done
