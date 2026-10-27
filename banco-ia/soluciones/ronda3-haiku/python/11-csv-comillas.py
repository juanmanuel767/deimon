import sys


def separar(linea):
    campos = []
    i = 0
    n = len(linea)
    while True:
        if i < n and linea[i] == "'":
            j = linea.index("'", i + 1)
            campos.append(linea[i + 1:j])
            i = j + 1
        else:
            j = linea.find(",", i)
            if j == -1:
                j = n
            campos.append(linea[i:j])
            i = j
        if i < n and linea[i] == ",":
            i += 1
            continue
        break
    return campos


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        campos = separar(linea)
        valores = "".join(f"[{c}]" for c in campos)
        print(f"{len(campos)} campos: {valores}")


main()
