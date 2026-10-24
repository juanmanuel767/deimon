import re
import sys

suma = 0
for numero_linea, linea in enumerate(sys.stdin.read().splitlines(), start=1):
    texto = linea.strip()
    if re.fullmatch(r"[+-]?[0-9]+", texto):
        suma += int(texto)
    else:
        print(f"linea {numero_linea}: no es un numero: {linea}")
print(f"suma: {suma}")
