import sys
from datetime import date

DIAS = ["lunes", "martes", "miercoles", "jueves", "viernes", "sabado", "domingo"]


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        fecha = linea.strip()
        if not fecha:
            continue
        try:
            anio, mes, dia = map(int, fecha.split("-"))
            nombre = DIAS[date(anio, mes, dia).weekday()]
            print(f"{fecha}: {nombre}")
        except ValueError:
            print(f"{fecha}: invalida")


main()
