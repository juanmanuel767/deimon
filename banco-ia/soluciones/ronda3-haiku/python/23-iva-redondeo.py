import sys
from decimal import Decimal, ROUND_HALF_UP

CENTIMO = Decimal("0.01")


def euros(d):
    return str(d.quantize(CENTIMO, rounding=ROUND_HALF_UP))


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    suma = Decimal("0")
    for linea in data.splitlines():
        partes = linea.split()
        if len(partes) < 2:
            continue
        base = Decimal(partes[0])
        tipo = int(partes[1])
        iva = (base * tipo / 100).quantize(CENTIMO, rounding=ROUND_HALF_UP)
        total = base + iva
        suma += total
        print(f"{euros(base)} {tipo}% -> iva {euros(iva)} total {euros(total)}")
    print(f"suma de totales: {euros(suma)}")


main()
