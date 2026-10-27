import json
import sys


def euros(centimos):
    return f"{centimos // 100}.{centimos % 100:02d}"


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    documento = json.loads(sys.stdin.read())
    print(f"tienda: {documento['tienda']}")
    agotados = 0
    total = 0
    for producto in documento["productos"]:
        precio = producto["precio"]
        stock = producto["stock"]
        if stock > 0:
            valor = precio * stock
            total += valor
            print(f"{producto['nombre']}: {euros(precio)} x {stock} = {euros(valor)}")
        else:
            agotados += 1
    print(f"agotados: {agotados}")
    print(f"valor total: {euros(total)}")


main()
