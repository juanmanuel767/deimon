import sys


def insertar(arbol, ruta, valor):
    nodo = arbol
    claves = ruta.split(".")
    for clave in claves[:-1]:
        if not isinstance(nodo.get(clave), dict):
            nodo[clave] = {}
        nodo = nodo[clave]
    nodo[claves[-1]] = valor


def escribir(nodo, nivel, salida):
    sangria = "  " * nivel
    for clave in sorted(nodo):
        valor = nodo[clave]
        if isinstance(valor, dict):
            salida.append(f"{sangria}{clave}:")
            escribir(valor, nivel + 1, salida)
        else:
            salida.append(f"{sangria}{clave}: {valor}")


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    arbol = {}
    for linea in data.splitlines():
        if "=" not in linea:
            continue
        ruta, valor = linea.split("=", 1)
        insertar(arbol, ruta, valor)
    salida = []
    escribir(arbol, 0, salida)
    for linea in salida:
        print(linea)


main()
