import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    inventario = {}
    for linea in data.splitlines():
        partes = linea.split()
        if not partes:
            continue
        orden = partes[0]
        if orden == "poner":
            producto, n = partes[1], int(partes[2])
            inventario[producto] = inventario.get(producto, 0) + n
        elif orden == "quitar":
            producto, n = partes[1], int(partes[2])
            actual = inventario.get(producto, 0)
            if actual >= n:
                inventario[producto] = actual - n
            else:
                print(f"error: no hay {n} de {producto}")
        elif orden == "ver":
            con_unidades = [(p, u) for p, u in sorted(inventario.items()) if u > 0]
            if not con_unidades:
                print("vacio")
            for producto, unidades in con_unidades:
                print(f"{producto}={unidades}")


main()
