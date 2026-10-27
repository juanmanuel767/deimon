import sys


def mover(n, origen, destino, auxiliar, pasos):
    if n == 0:
        return
    mover(n - 1, origen, auxiliar, destino, pasos)
    pasos.append(f"mover disco {n} de {origen} a {destino}")
    mover(n - 1, auxiliar, destino, origen, pasos)


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    n = int(sys.stdin.read().split()[0])
    pasos = []
    mover(n, "A", "C", "B", pasos)
    for paso in pasos:
        print(paso)
    print(f"total: {len(pasos)}")


main()
