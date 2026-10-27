import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    filas = int(tokens[0])
    columnas = int(tokens[1])
    numeros = list(map(int, tokens[2:2 + filas * columnas]))
    m = [numeros[i * columnas:(i + 1) * columnas] for i in range(filas)]

    resultado = []
    arriba, abajo = 0, filas - 1
    izq, der = 0, columnas - 1
    while arriba <= abajo and izq <= der:
        for j in range(izq, der + 1):
            resultado.append(m[arriba][j])
        arriba += 1
        for i in range(arriba, abajo + 1):
            resultado.append(m[i][der])
        der -= 1
        if arriba <= abajo:
            for j in range(der, izq - 1, -1):
                resultado.append(m[abajo][j])
            abajo -= 1
        if izq <= der:
            for i in range(abajo, arriba - 1, -1):
                resultado.append(m[i][izq])
            izq += 1

    print(" ".join(str(x) for x in resultado))


main()
