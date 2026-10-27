import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    total = 0
    for linea in data.splitlines():
        linea = linea.strip()
        if linea:
            total += int(linea)
    print(total)


main()
