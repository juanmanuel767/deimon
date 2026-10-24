import sys
from decimal import Decimal, ROUND_HALF_UP


def redondear(valor):
    return valor.quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)


totales = {}
for linea in sys.stdin.read().splitlines():
    if not linea.strip():
        continue
    producto, cantidad, precio = linea.split(";")
    producto = producto.strip()
    subtotal = int(cantidad) * Decimal(precio.strip())
    totales[producto] = totales.get(producto, Decimal("0")) + subtotal

total_general = Decimal("0")
for producto in sorted(totales):
    print(f"{producto}: {redondear(totales[producto])}")
    total_general += totales[producto]
print(f"TOTAL: {redondear(total_general)}")
