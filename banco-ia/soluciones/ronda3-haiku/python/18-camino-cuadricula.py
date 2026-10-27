import sys
from collections import deque


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    filas = int(tokens[0])
    columnas = int(tokens[1])
    rejilla = tokens[2:2 + filas]

    inicio = fin = None
    for i in range(filas):
        for j in range(columnas):
            if rejilla[i][j] == "S":
                inicio = (i, j)
            elif rejilla[i][j] == "E":
                fin = (i, j)

    distancia = {inicio: 0}
    cola = deque([inicio])
    while cola:
        i, j = cola.popleft()
        if (i, j) == fin:
            break
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ni, nj = i + di, j + dj
            if (0 <= ni < filas and 0 <= nj < columnas
                    and rejilla[ni][nj] != "#" and (ni, nj) not in distancia):
                distancia[(ni, nj)] = distancia[(i, j)] + 1
                cola.append((ni, nj))

    if fin in distancia:
        print(distancia[fin])
    else:
        print("sin camino")


main()
