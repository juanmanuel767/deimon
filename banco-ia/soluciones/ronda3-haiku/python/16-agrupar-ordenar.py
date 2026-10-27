import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    grupos = {}
    for linea in data.splitlines():
        linea = linea.strip()
        if not linea:
            continue
        departamento, nombre, salario = linea.split(",")
        grupos.setdefault(departamento, []).append((nombre, int(salario)))
    for departamento in sorted(grupos):
        miembros = grupos[departamento]
        total = sum(salario for _, salario in miembros)
        print(f"{departamento} ({len(miembros)} personas): total {total}")
        for nombre, salario in sorted(miembros, key=lambda m: (-m[1], m[0])):
            print(f"  {nombre}: {salario}")


main()
