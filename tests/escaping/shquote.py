# Escapador de argumentos para una orden remota por SSH.
#
# La propiedad que tiene que sostener: para cualquier lista de cadenas,
#   argv  ->  build(argv)  ->  shell remoto  ->  argv
# es la identidad. Se comprueba con prop_test.py contra bash, dash, zsh y
# busybox ash, porque el shell de login del usuario remoto no lo elegimos
# nosotros.
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
    """Entrecomilla una cadena para un shell POSIX."""
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
    return "'" + s.replace("'", "'\\''") + "'"


def build(argv):
    """Serializa un argv en la cadena que se le entrega al shell remoto."""
    if isinstance(argv, str):
        raise TypeError("build espera una lista de argumentos, no una cadena")
    if not argv:
        raise ValueError("un comando remoto necesita al menos un argumento")
    return " ".join(shquote(a) for a in argv)
