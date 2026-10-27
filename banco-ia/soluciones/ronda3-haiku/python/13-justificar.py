import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    lineas = sys.stdin.read().splitlines()
    if not lineas:
        return
    ancho = int(lineas[0].strip())
    palabras = " ".join(lineas[1:]).split()

    grupos = []
    actual = []
    largo = 0
    for p in palabras:
        if not actual:
            actual = [p]
            largo = len(p)
        elif largo + 1 + len(p) <= ancho:
            actual.append(p)
            largo += 1 + len(p)
        else:
            grupos.append(actual)
            actual = [p]
            largo = len(p)
    if actual:
        grupos.append(actual)

    for k, grupo in enumerate(grupos):
        es_ultima = k == len(grupos) - 1
        if es_ultima or len(grupo) == 1:
            print(" ".join(grupo))
            continue
        huecos = len(grupo) - 1
        espacios = ancho - sum(len(p) for p in grupo)
        base, extra = divmod(espacios, huecos)
        partes = []
        for i, p in enumerate(grupo):
            partes.append(p)
            if i < huecos:
                partes.append(" " * (base + (1 if i < extra else 0)))
        print("".join(partes))


main()
