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

    # Lo que el cliente lee desde Orbit 1.4.3 y 1.4.7. Si un campo deja de
    # salir, la interfaz no se rompe —lo trata como un servidor viejo— pero
    # deja de enseñar la congelación, la CI o el certificado de origen SIN
    # avisar, y eso hay que saberlo aquí y no por un usuario.
    esperados = ("cert_type", "checks", "locked", "lock")
    for a in lista["apps"]:
        falta = [k for k in esperados if k not in a["state"]]
        if falta:
            fallos.append("la app %s no trae %s en state" % (a["name"], ", ".join(falta)))
            break
    else:
        print("list --json trae %s" % ", ".join(esperados))

    # Los once recuentos de la pasada. Se congelan todas las apps del banco con
    # el testigo que lee Orbit (/etc/orbit/apps/<app>.lock): una congelada se
    # salta antes de nada, así que la pasada no despliega ni pregunta a ningún
    # remoto, y el objeto que sale es el de verdad.
    apps_dir = os.path.join(bench, "etc", "apps")
    testigos = []
    for a in lista["apps"]:
        t = os.path.join(apps_dir, a["name"] + ".lock")
        with open(t, "w") as f:
            f.write("at=2026-11-05T10:00:00+00:00\nby=banco\nreason=check-contract\n")
        testigos.append(t)
    try:
        lote = orbit(bench, "deploy", "--all", "--if-changed", "--json")
    finally:
        for t in testigos:
            os.remove(t)
    finales = ("deployed", "failed", "unchanged", "unreachable", "gone", "skipped",
               "locked", "ignored", "ci_pending", "ci_failed", "ci_unknown")
    falta = [k for k in finales if not isinstance(lote.get(k), int)]
    if falta:
        fallos.append("'deploy --all --json' no trae los recuentos %s" % ", ".join(falta))
    elif lote["locked"] != len(lista["apps"]) or sum(lote[k] for k in finales) != lote["total"]:
        fallos.append("los once recuentos no reconstruyen el total de la pasada")
    else:
        print("deploy --all --json cuenta los once finales (%d congeladas)" % lote["locked"])

    if fallos:
        for f in fallos:
            print("  ✗ %s" % f, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
