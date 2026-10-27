import sys
from decimal import Decimal


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    por_producto = {}
    total = Decimal("0")
    for linea in data.splitlines():
        linea = linea.strip()
        if not linea:
            continue
        producto, cantidad, precio = linea.split(";")
        importe = int(cantidad) * Decimal(precio)
        por_producto[producto] = por_producto.get(producto, Decimal("0")) + importe
        total += importe
    for producto in sorted(por_producto):
        print(f"{producto}: {por_producto[producto]:.2f}")
    print(f"TOTAL: {total:.2f}")


main()
