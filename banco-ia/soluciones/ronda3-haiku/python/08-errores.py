import re
import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    suma = 0
    for n, linea in enumerate(data.splitlines(), 1):
        if re.fullmatch(r"-?[0-9]+", linea):
            suma += int(linea)
        else:
            print(f"linea {n}: no es un numero: {linea}")
    print(f"suma: {suma}")


main()
