import sys
from collections import deque


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    pila = []
    cola = deque()
    for linea in data.splitlines():
        partes = linea.split()
        if not partes:
            continue
        orden = partes[0]
        if orden == "apilar":
            pila.append(partes[1])
        elif orden == "desapilar":
            if pila:
                print(f"pila->{pila.pop()}")
            else:
                print("error: pila vacia")
        elif orden == "encolar":
            cola.append(partes[1])
        elif orden == "desencolar":
            if cola:
                print(f"cola->{cola.popleft()}")
            else:
                print("error: cola vacia")
        elif orden == "mostrar":
            print("pila: " + (" ".join(pila) if pila else "vacia"))
            print("cola: " + (" ".join(cola) if cola else "vacia"))


main()
