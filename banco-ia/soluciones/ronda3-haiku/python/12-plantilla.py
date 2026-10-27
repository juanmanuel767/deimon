import re
import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    lineas = sys.stdin.read().splitlines()
    plantilla = lineas[0] if lineas else ""
    valores = {}
    for linea in lineas[1:]:
        if "=" in linea:
            clave, valor = linea.split("=", 1)
            valores[clave] = valor

    salida = []
    i = 0
    while i < len(plantilla):
        if plantilla[i] == "{":
            cierre = plantilla.find("}", i + 1)
            if cierre != -1:
                nombre = plantilla[i + 1:cierre]
                if re.fullmatch(r"[A-Za-z0-9_]+", nombre):
                    if nombre in valores:
                        salida.append(valores[nombre])
                    else:
                        salida.append(plantilla[i:cierre + 1])
                    i = cierre + 1
                    continue
        salida.append(plantilla[i])
        i += 1

    print("".join(salida))


main()
