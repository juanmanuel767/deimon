import sys
from collections import Counter


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    cuenta = Counter()
    for token in data.split():
        palabra = token.lower().strip(".,;:!?")
        if palabra:
            cuenta[palabra] += 1
    orden = sorted(cuenta.items(), key=lambda kv: (-kv[1], kv[0]))
    for palabra, veces in orden[:3]:
        print(f"{palabra}: {veces}")


main()
