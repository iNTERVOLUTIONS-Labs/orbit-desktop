# Escapador de argumentos para una orden remota por SSH.
#
# La propiedad que tiene que sostener: para cualquier lista de cadenas,
#   argv  ->  build(argv)  ->  shell remoto  ->  argv
# es la identidad. Se comprueba con prop_test.py contra bash, dash, zsh,
# busybox ash y fish (si está instalado), porque el shell de login del usuario
# remoto no lo elegimos nosotros.
#
# csh y tcsh NO están soportados: allí un salto de línea dentro de '...' acaba
# la línea y lo que sigue se lee como otra. Ver el porqué en
# crates/orbit-client/src/shquote.rs.
#
# El conjunto "seguro" es deliberadamente estrecho, y esa estrechez costó un
# fallo real: con '=' dentro del conjunto, zsh expande las palabras que
# empiezan por '=' (opción EQUALS: '=ls' se sustituye por la ruta de 'ls') y
# la prueba de propiedad devolvía "zsh:1: Y not found" para el argumento
# '=Y'. bash, dash y busybox pasaban los 2.529 casos. Es exactamente el modo
# de fallo que hace falta cubrir: correcto en el shell donde se desarrolla,
# roto en el que usa el usuario.
#
# La lección, y por eso el conjunto se queda como está: cada carácter que se
# añade aquí es una regla de expansión de cuatro shells que hay que conocer.
# Entrecomillar de más no cuesta nada; entrecomillar de menos es T-03.
_SAFE = frozenset(
    "abcdefghijklmnopqrstuvwxyz"
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "0123456789"
    "_./-"
)


def shquote(s):
    """Entrecomilla una cadena para el shell de login remoto."""
    if not isinstance(s, str):
        raise TypeError("shquote espera str")
    if "\x00" in s:
        # Un byte nulo no puede viajar en un argv. Fingir que sí es peor que
        # fallar: se rechaza en vez de escaparse.
        raise ValueError("un argumento no puede contener un byte nulo")
    if s == "":
        return "''"
    if all(c in _SAFE for c in s):
        return s
    # Ninguna barra invertida dentro de comillas simples: fish lee \' y \\
    # como escapes ahí dentro, y con la forma clásica '\'' la entrada
    # "\'; id #" ejecutaba `id`. Cada ' y cada \ va fuera, con su barra, que
    # todos los shells probados leen como el carácter literal.
    out, abierta = [], False
    for c in s:
        if c in "'\\":
            if abierta:
                out.append("'")
                abierta = False
            out.append("\\" + c)
        else:
            if not abierta:
                out.append("'")
                abierta = True
            out.append(c)
    if abierta:
        out.append("'")
    return "".join(out)


def build(argv):
    """Serializa un argv en la cadena que se le entrega al shell remoto."""
    if isinstance(argv, str):
        raise TypeError("build espera una lista de argumentos, no una cadena")
    if not argv:
        raise ValueError("un comando remoto necesita al menos un argumento")
    return " ".join(shquote(a) for a in argv)
