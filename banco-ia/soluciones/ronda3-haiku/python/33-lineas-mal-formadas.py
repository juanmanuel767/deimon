import re
import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    ciudades = {}
    validas = 0
    descartadas = 0
    for n, linea in enumerate(data.splitlines(), 1):
        campos = linea.split(";")
        motivo = None
        if len(campos) != 3:
            motivo = "numero de campos incorrecto"
        else:
            nombre, edad, ciudad = campos
            if nombre == "":
                motivo = "nombre vacio"
            elif not (re.fullmatch(r"[0-9]+", edad) and int(edad) <= 120):
                motivo = "edad invalida"
            elif ciudad == "":
                motivo = "ciudad vacia"
        if motivo is not None:
            print(f"linea {n}: {motivo}")
            descartadas += 1
        else:
            validas += 1
            ciudades[ciudad] = ciudades.get(ciudad, 0) + 1
    print(f"validas: {validas}")
    print(f"descartadas: {descartadas}")
    for ciudad in sorted(ciudades):
        print(f"{ciudad}: {ciudades[ciudad]}")


main()
