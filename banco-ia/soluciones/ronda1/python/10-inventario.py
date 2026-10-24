import sys

inventario = {}
for linea in sys.stdin.read().splitlines():
    partes = linea.split()
    if not partes:
        continue
    orden = partes[0]
    if orden == "ver":
        disponibles = sorted((p, n) for p, n in inventario.items() if n > 0)
        if not disponibles:
            print("vacio")
        for producto, unidades in disponibles:
            print(f"{producto}={unidades}")
    elif orden in ("poner", "quitar"):
        producto = " ".join(partes[1:-1])
        cantidad = int(partes[-1])
        actual = inventario.get(producto, 0)
        if orden == "poner":
            inventario[producto] = actual + cantidad
        elif actual >= cantidad:
            inventario[producto] = actual - cantidad
        else:
            print(f"error: no hay {cantidad} de {producto}")
