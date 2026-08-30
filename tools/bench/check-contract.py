#!/usr/bin/env python3
"""Comprueba que el contrato sigue siendo el que documentamos.

Dos comprobaciones baratas que cazan una subida de contrato el mismo día. Los
campos se añaden y nunca se renombran —es la promesa de Orbit—, así que si
'schema' se mueve queremos enterarnos aquí y no por un usuario.

La segunda no es sobre el contrato sino sobre una decisión de diseño que
depende de él: 'status --json' trae hoy el array de apps completo e idéntico al
de 'list --json', y por eso la portada del cliente cuesta una llamada y no dos.
Si eso deja de ser cierto, la portada pasa de 389 ms a 695 y hay que enterarse.

    uso:  tools/bench/check-contract.py <ruta del banco>
"""
import json
import os
import subprocess
import sys

ESPERADO_SCHEMA = 1
ESPERADO_CONTRATO = 1


def orbit(bench, *args):
    entorno = dict(os.environ)
    entorno.update({
        "ETC_DIR": os.path.join(bench, "etc"),
        "APPS_DIR": os.path.join(bench, "srv"),
        "LOG_FILE": os.path.join(bench, "log", "orbit.log"),
    })
    r = subprocess.run([os.path.join(bench, "orbit"), *args],
                       capture_output=True, env=entorno)
    if r.returncode != 0:
        raise SystemExit("orbit %s salió con %d:\n%s"
                         % (" ".join(args), r.returncode,
                            r.stderr.decode("utf-8", "replace")))
    # Por stdout va un solo objeto y nada más. Si hay basura delante, es un
    # fallo y no algo que recortar: buscar la primera llave es exactamente cómo
    # se cuela un objeto ajeno delante del legítimo.
    return json.loads(r.stdout)


def main():
    bench = sys.argv[1] if len(sys.argv) > 1 else ".bench"
    fallos = []

    v = orbit(bench, "version", "--json")
    if v.get("schema") != ESPERADO_SCHEMA:
        fallos.append("el schema ha cambiado: %r" % v.get("schema"))
    if v.get("contract") != ESPERADO_CONTRATO:
        fallos.append("el contrato ha cambiado: %r" % v.get("contract"))
    print("orbit %s · schema %s · contrato %s"
          % (v.get("version"), v.get("schema"), v.get("contract")))

    lista = orbit(bench, "list", "--json")
    estado = orbit(bench, "status", "--json")
    if "apps" not in estado:
        fallos.append("'status --json' ya no trae el array de apps: "
                      "la portada del cliente pasa a costar dos llamadas")
    elif estado["apps"] != lista["apps"]:
        fallos.append("'status --json' y 'list --json' ya no coinciden: "
                      "la portada no puede seguir alimentándose de una sola llamada")
    else:
        print("status --json alimenta la portada de una sola llamada "
              "(%d apps, idénticas a list --json)" % len(estado["apps"]))

    if fallos:
        for f in fallos:
            print("  ✗ %s" % f, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
