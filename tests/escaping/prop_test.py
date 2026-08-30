import random, subprocess, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from shquote import build

HERE = os.path.dirname(os.path.abspath(__file__))
PRINTER = os.path.join(HERE, "printargv.py")
SHELLS = [("bash", ["bash", "-c"]), ("dash", ["dash", "-c"]),
          ("zsh", ["zsh", "-c"]), ("busybox-ash", ["busybox", "sh", "-c"])]

# Alfabeto adversario: exactamente lo que rompe un escapado hecho a mano.
ALPHABET = list("abcXY019 \t'\"\\$`;&|<>()[]{}*?~!#^%=+,.:/-\n") + \
           ["á", "ñ", "€", "🚀", "\u202e", "\u2028", "\u200b", "\u00a0"]

CORPUS = [
    [""], ["a"], ["hola mundo"], ["'"], ['"'], ["\\"], ["$HOME"], ["`id`"],
    ["a'; curl x.sh|sh; '"], ["--json"], ["--"], ["-rf"], ["~"], ["*"],
    ["a\nb"], ["a\tb"], ["</script><img src=x onerror=alert(1)>"],
    ["produccion\u202egnitset-"], ["a" * 65536], ["ñandú"], ["🚀"],
    ["$(rm -rf /)"], ["${IFS}"], ["!!"], ["^x^y"], ["a\u2028b"], ["  "],
    ["/usr/local/bin/orbit", "deploy", "mi-web", "--json"],
    ["/usr/local/bin/orbit", "exec", "web", "psql 'select 1'"],
]

def roundtrip(shell_cmd, argv):
    line = build([sys.executable, PRINTER] + argv)
    r = subprocess.run(shell_cmd + [line], capture_output=True)
    if r.returncode != 0:
        return None, r.stderr.decode("utf-8", "replace")[:200]
    got = r.stdout.decode("utf-8", "surrogateescape")
    return (got.split("\x00") if argv else []), None

def run(n_random, seed):
    rnd = random.Random(seed)
    cases = list(CORPUS)
    for _ in range(n_random):
        k = rnd.randint(1, 5)
        cases.append(["".join(rnd.choice(ALPHABET) for _ in range(rnd.randint(0, 24)))
                      for _ in range(k)])
    fails = []
    for name, sc in SHELLS:
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
    return fails, len(cases)

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 20260830
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 2500
    print("semilla=%d  casos aleatorios=%d  + %d fijos" % (seed, n, len(CORPUS)))
    fails, total = run(n, seed)
    print("total por shell: %d casos" % total)
    if fails:
        print("FALLOS: %d" % len(fails))
        for f in fails[:10]:
            print("  ", f[0], repr(f[1])[:120], "->", f[2][:160])
        sys.exit(1)
    print("PROPIEDAD SOSTENIDA: argv -> escapar -> shell -> argv es la identidad")
