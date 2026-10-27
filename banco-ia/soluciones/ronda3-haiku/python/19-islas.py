import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    filas = int(tokens[0])
    columnas = int(tokens[1])
    rejilla = tokens[2:2 + filas]

    visitado = set()
    islas = 0
    mayor = 0
    for i in range(filas):
        for j in range(columnas):
            if rejilla[i][j] != "#" or (i, j) in visitado:
                continue
            islas += 1
            tamano = 0
            visitado.add((i, j))
            pila = [(i, j)]
            while pila:
                a, b = pila.pop()
                tamano += 1
                for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    ni, nj = a + di, b + dj
                    if (0 <= ni < filas and 0 <= nj < columnas
                            and rejilla[ni][nj] == "#" and (ni, nj) not in visitado):
                        visitado.add((ni, nj))
                        pila.append((ni, nj))
            mayor = max(mayor, tamano)

    print(f"islas: {islas}")
    print(f"mayor: {mayor}")


main()
