import sys


VALORES = [
    (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
    (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
    (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
]


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for token in data.split():
        n = int(token)
        partes = []
        for valor, letras in VALORES:
            while n >= valor:
                partes.append(letras)
                n -= valor
        print("".join(partes))


main()
