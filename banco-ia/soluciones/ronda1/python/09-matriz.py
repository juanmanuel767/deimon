import sys

datos = sys.stdin.read().split()
filas, columnas = int(datos[0]), int(datos[1])
valores = list(map(int, datos[2:2 + filas * columnas]))
matriz = [valores[i * columnas:(i + 1) * columnas] for i in range(filas)]

resultado = []
arriba, abajo, izquierda, derecha = 0, filas - 1, 0, columnas - 1
while arriba <= abajo and izquierda <= derecha:
    for c in range(izquierda, derecha + 1):
        resultado.append(matriz[arriba][c])
    arriba += 1
    for f in range(arriba, abajo + 1):
        resultado.append(matriz[f][derecha])
    derecha -= 1
    if arriba <= abajo:
        for c in range(derecha, izquierda - 1, -1):
            resultado.append(matriz[abajo][c])
        abajo -= 1
    if izquierda <= derecha:
        for f in range(abajo, arriba - 1, -1):
            resultado.append(matriz[f][izquierda])
        izquierda += 1

print(" ".join(map(str, resultado)))
