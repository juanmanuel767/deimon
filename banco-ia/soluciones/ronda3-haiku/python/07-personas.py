import sys
from decimal import Decimal, ROUND_HALF_UP


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    personas = []
    for linea in data.splitlines():
        linea = linea.strip()
        if not linea:
            continue
        nombre, edad = linea.rsplit(",", 1)
        personas.append((nombre, int(edad)))
    personas.sort(key=lambda p: (-p[1], p[0]))
    for nombre, edad in personas:
        print(f"{nombre} ({edad})")
    suma = sum(edad for _, edad in personas)
    media = Decimal(suma) / Decimal(len(personas))
    print("media: " + str(media.quantize(Decimal("0.1"), rounding=ROUND_HALF_UP)))


main()
