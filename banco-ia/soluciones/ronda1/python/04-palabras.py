import sys
from collections import Counter

conteo = Counter()
for palabra in sys.stdin.read().split():
    palabra = palabra.lower().strip(".,;:!?")
    if palabra:
        conteo[palabra] += 1

ordenadas = sorted(conteo.items(), key=lambda par: (-par[1], par[0]))
for palabra, veces in ordenadas[:3]:
    print(f"{palabra}: {veces}")
