import re
import sys
from itertools import groupby


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        if len(linea) < 2:
            continue
        orden = linea[0]
        texto = linea[2:]
        if orden == "C":
            partes = []
            for letra, grupo in groupby(texto):
                partes.append(f"{sum(1 for _ in grupo)}{letra}")
            print("".join(partes))
        elif orden == "D":
            partes = [letra * int(n) for n, letra in re.findall(r"([0-9]+)(.)", texto)]
            print("".join(partes))


main()
