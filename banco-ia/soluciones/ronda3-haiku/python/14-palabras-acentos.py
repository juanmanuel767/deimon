import re
import sys
from collections import Counter


PATRON = re.compile(r"[a-zA-ZáéíóúüñÁÉÍÓÚÜÑ]+")


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    palabras = [p.lower() for p in PATRON.findall(data)]
    cuenta = Counter(palabras)
    print(f"total: {len(palabras)}")
    print(f"distintas: {len(cuenta)}")
    orden = sorted(cuenta.items(), key=lambda kv: (-kv[1], kv[0]))
    for palabra, veces in orden[:5]:
        print(f"{palabra}: {veces}")


main()
