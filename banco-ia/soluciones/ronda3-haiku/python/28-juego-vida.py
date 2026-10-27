import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    filas = int(tokens[0])
    columnas = int(tokens[1])
    generaciones = int(tokens[2])
    cuadricula = [[1 if c == "#" else 0 for c in fila] for fila in tokens[3:3 + filas]]

    for _ in range(generaciones):
        nueva = [[0] * columnas for _ in range(filas)]
        for i in range(filas):
            for j in range(columnas):
                vecinas = 0
                for di in (-1, 0, 1):
                    for dj in (-1, 0, 1):
                        if di or dj:
                            ni, nj = i + di, j + dj
                            if 0 <= ni < filas and 0 <= nj < columnas:
                                vecinas += cuadricula[ni][nj]
                if cuadricula[i][j]:
                    nueva[i][j] = 1 if vecinas in (2, 3) else 0
                else:
                    nueva[i][j] = 1 if vecinas == 3 else 0
        cuadricula = nueva

    for fila in cuadricula:
        print("".join("#" if c else "." for c in fila))


main()
