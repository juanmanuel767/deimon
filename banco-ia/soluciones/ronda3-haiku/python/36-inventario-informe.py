import sys


def euros(centimos):
    return f"{centimos // 100}.{centimos % 100:02d}"


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    productos = {}
    for linea in data.splitlines():
        partes = linea.split()
        if not partes:
            continue
        orden = partes[0]
        if orden == "alta":
            codigo, nombre = partes[1], partes[2]
            precio, stock = int(partes[3]), int(partes[4])
            if codigo in productos:
                print(f"error: codigo duplicado {codigo}")
            else:
                productos[codigo] = {"nombre": nombre, "precio": precio,
                                     "stock": stock, "ingresos": 0}
        elif orden == "venta":
            codigo, n = partes[1], int(partes[2])
            if codigo not in productos:
                print(f"error: codigo desconocido {codigo}")
                continue
            p = productos[codigo]
            if p["stock"] < n:
                print(f"error: stock insuficiente de {codigo} (hay {p['stock']}, piden {n})")
            else:
                p["stock"] -= n
                p["ingresos"] += n * p["precio"]
        elif orden == "reponer":
            codigo, n = partes[1], int(partes[2])
            if codigo not in productos:
                print(f"error: codigo desconocido {codigo}")
            else:
                productos[codigo]["stock"] += n
        elif orden == "informe":
            total = 0
            for codigo in sorted(productos):
                p = productos[codigo]
                print(f"{codigo} {p['nombre']} stock={p['stock']} ingresos={euros(p['ingresos'])}")
                total += p["ingresos"]
            print(f"ingresos totales: {euros(total)}")
            sin_stock = [c for c in sorted(productos) if productos[c]["stock"] == 0]
            print("sin stock: " + (", ".join(sin_stock) if sin_stock else "ninguno"))


main()
