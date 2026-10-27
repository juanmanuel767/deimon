import sys

DIRECCIONES = ["norte", "este", "sur", "oeste"]
DX = [0, 1, 0, -1]
DY = [1, 0, -1, 0]


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        ordenes = linea.strip()
        if not ordenes:
            continue
        x = y = 0
        d = 0
        maximo = 0
        for c in ordenes:
            if c == "A":
                x += DX[d]
                y += DY[d]
                maximo = max(maximo, x * x + y * y)
            elif c == "G":
                d = (d + 1) % 4
            elif c == "I":
                d = (d + 3) % 4
        print(f"{x},{y} {DIRECCIONES[d]} max={maximo}")


main()
