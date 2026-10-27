import sys
from datetime import date, timedelta


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        partes = linea.split()
        if len(partes) < 2:
            continue
        anio, mes, dia = map(int, partes[0].split("-"))
        n = int(partes[1])
        resultado = date(anio, mes, dia) + timedelta(days=n)
        print(f"{resultado.year:04d}-{resultado.month:02d}-{resultado.day:02d}")


main()
