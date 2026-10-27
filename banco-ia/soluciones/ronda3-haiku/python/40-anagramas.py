import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    palabras = set()
    for linea in data.splitlines():
        palabra = linea.strip()
        if palabra:
            palabras.add(palabra)

    grupos = {}
    for palabra in palabras:
        clave = "".join(sorted(palabra))
        grupos.setdefault(clave, []).append(palabra)

    validos = [sorted(g) for g in grupos.values() if len(g) >= 2]
    if not validos:
        print("ninguno")
        return
    validos.sort(key=lambda g: g[0])
    for g in validos:
        print(" ".join(g))


main()
