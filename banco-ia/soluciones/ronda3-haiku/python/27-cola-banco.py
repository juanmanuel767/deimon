import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    cajas = int(tokens[0])
    resto = tokens[1:]
    libre = [0] * cajas
    espera_total = 0
    termina = 0
    for k in range(0, len(resto) - 2, 3):
        llegada = int(resto[k])
        duracion = int(resto[k + 1])
        nombre = resto[k + 2]
        caja = min(range(cajas), key=lambda i: (libre[i], i))
        empieza = max(llegada, libre[caja])
        libre[caja] = empieza + duracion
        espera = empieza - llegada
        espera_total += espera
        termina = max(termina, empieza + duracion)
        print(f"{nombre}: caja {caja + 1} empieza {empieza} espera {espera}")
    print(f"espera total: {espera_total}")
    print(f"termina: {termina}")


main()
