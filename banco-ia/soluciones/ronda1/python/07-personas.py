import sys
from decimal import Decimal, ROUND_HALF_UP

personas = []
for linea in sys.stdin.read().splitlines():
    if not linea.strip():
        continue
    nombre, edad = linea.split(",")
    personas.append((nombre.strip(), int(edad)))

personas.sort(key=lambda p: (-p[1], p[0]))
for nombre, edad in personas:
    print(f"{nombre} ({edad})")

if personas:
    media = Decimal(sum(edad for _, edad in personas)) / Decimal(len(personas))
    media = media.quantize(Decimal("0.1"), rounding=ROUND_HALF_UP)
else:
    media = Decimal("0.0")
print(f"media: {media}")
