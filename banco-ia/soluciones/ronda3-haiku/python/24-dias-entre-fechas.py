import sys
from datetime import date


def leer_fecha(texto):
    anio, mes, dia = map(int, texto.split("-"))
    return date(anio, mes, dia)


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        partes = linea.split()
        if len(partes) < 2:
            continue
        primera = leer_fecha(partes[0])
        segunda = leer_fecha(partes[1])
        print((segunda - primera).days)


main()
