#!/usr/bin/env python3
"""Comprueba que el banco de tareas está sano:
  - cada tarea tiene al menos 4 casos, y cada caso tiene .entrada y .esperado;
  - el caso 1 (entrada y salida) aparece en enunciado.md;
  - lo que hay en disco coincide con lo que calcula la referencia en Python (crear_tareas.py);
  - la solución de referencia en Deimon (soluciones/referencia/deimon/) da la salida de Python
    en todos los casos.
Uso: python3 herramientas/comprobar_tareas.py   (desde banco-ia/). Código de salida 1 si algo falla.
La variable DEIMON elige el compilador (por defecto ../compilador/target/release/deimon)."""
import os
import subprocess
import sys

AQUI = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, AQUI)
import crear_tareas  # noqa: E402

RAIZ = os.path.normpath(os.path.join(AQUI, ".."))
DEIMON = os.environ.get("DEIMON") or os.path.normpath(os.path.join(RAIZ, "..", "compilador", "target", "release", "deimon"))


def leer(ruta):
    with open(ruta, encoding="utf-8") as f:
        return f.read()


def main():
    fallos = []
    for nombre, _enunciado, casos, funcion in crear_tareas.TAREAS:
        carpeta = os.path.join(RAIZ, "tareas", nombre)
        if len(casos) < 4:
            fallos.append(f"{nombre}: solo {len(casos)} casos (mínimo 4)")
        try:
            enunciado = leer(os.path.join(carpeta, "enunciado.md"))
        except OSError:
            fallos.append(f"{nombre}: falta enunciado.md (ejecuta crear_tareas.py)")
            continue
        ent1 = casos[0]
        sal1 = "".join(l + "\n" for l in funcion(ent1.split("\n")[:-1]))
        if "```\n" + ent1 + "```" not in enunciado and ent1 != "":
            fallos.append(f"{nombre}: la entrada del caso 1 no aparece en el enunciado")
        if "```\n" + sal1 + "```" not in enunciado and sal1 != "":
            fallos.append(f"{nombre}: la salida del caso 1 no aparece en el enunciado")
        for k, entrada in enumerate(casos, 1):
            rec = os.path.join(carpeta, "casos", f"{k}.entrada")
            res = os.path.join(carpeta, "casos", f"{k}.esperado")
            esperado = "".join(l + "\n" for l in funcion(entrada.split("\n")[:-1]))
            if not os.path.isfile(rec) or not os.path.isfile(res):
                fallos.append(f"{nombre}: caso {k} sin archivos (ejecuta crear_tareas.py)")
                continue
            if leer(rec) != entrada or leer(res) != esperado:
                fallos.append(f"{nombre}: caso {k} en disco no coincide con la referencia de Python")
            sol = os.path.join(RAIZ, "soluciones", "referencia", "deimon", nombre + ".dmn")
            if not os.path.isfile(sol):
                if k == 1:
                    fallos.append(f"{nombre}: falta la referencia en Deimon")
                continue
            try:
                r = subprocess.run([DEIMON, "ejecutar", sol], input=entrada, capture_output=True,
                                   text=True, timeout=30)
            except subprocess.TimeoutExpired:
                fallos.append(f"{nombre}: caso {k}: la referencia de Deimon tarda demasiado")
                continue
            if r.stdout != esperado:
                fallos.append(f"{nombre}: caso {k}: la referencia de Deimon no coincide con la de Python"
                              + (f" ({r.stderr.strip().splitlines()[0]})" if r.stderr.strip() else ""))
    total = len(crear_tareas.TAREAS)
    if fallos:
        print("\n".join(fallos))
        print(f"FALLOS: {len(fallos)} en {total} tareas")
        sys.exit(1)
    print(f"bien: {total} tareas, todas con >=4 casos, caso 1 en el enunciado y Deimon = Python en todos")


if __name__ == "__main__":
    main()
