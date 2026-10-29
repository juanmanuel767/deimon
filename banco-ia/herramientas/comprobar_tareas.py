#!/usr/bin/env python3
"""Comprueba que el banco de tareas está sano:
  - cada tarea tiene al menos 4 casos, y cada caso tiene .entrada y .esperado;
  - el caso 1 (entrada y salida) aparece en enunciado.md;
  - lo que hay en disco coincide con lo que calcula la referencia en Python (crear_tareas.py);
  - la solución de referencia en Deimon (soluciones/referencia/deimon/) da la salida de Python
    en todos los casos;
  - tareas web (51-55): casos .peticiones.json/.esperado.json en disco = los del generador (tareas_web.py), ≥4 casos,
    el caso 1 dentro del enunciado, y las DOS referencias (Python y Deimon) pasan todos los casos con el arnés
    medir_web.py; salvo las que hoy Deimon no puede hacer (IMPOSIBLE_EN_DEIMON).
Uso: python3 herramientas/comprobar_tareas.py   (desde banco-ia/). Código de salida 1 si algo falla.
La variable DEIMON elige el compilador (por defecto ../compilador/target/release/deimon)."""
import json
import os
import subprocess
import sys

AQUI = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, AQUI)
import crear_tareas  # noqa: E402
import medir_web  # noqa: E402
import tareas_web  # noqa: E402

RAIZ = os.path.normpath(os.path.join(AQUI, ".."))
DEIMON = os.environ.get("DEIMON") or os.path.normpath(os.path.join(RAIZ, "..", "compilador", "target", "release", "deimon"))


# Tareas web que la referencia de Deimon no puede resolver con el compilador actual (motivo y ejemplo mínimo).
# Si algún día Deimon puede, hay que escribir su referencia y quitarla de aquí (se avisa si pasa).
IMPOSIBLE_EN_DEIMON = {}


def leer(ruta):
    with open(ruta, encoding="utf-8") as f:
        return f.read()


def comprobar_webs(fallos):
    for nombre, enunciado, casos in tareas_web.WEBS:
        carpeta = os.path.join(RAIZ, "tareas", nombre)
        if len(casos) < 4:
            fallos.append(f"{nombre}: solo {len(casos)} casos (mínimo 4)")
        try:
            en_disco = leer(os.path.join(carpeta, "enunciado.md"))
        except OSError:
            fallos.append(f"{nombre}: falta enunciado.md (ejecuta crear_tareas.py)")
            continue
        if en_disco != crear_tareas.texto_enunciado_web(nombre, enunciado, casos):
            fallos.append(f"{nombre}: enunciado.md en disco no coincide con el generador")
        if tareas_web.texto_ejemplo(*casos[0]) not in en_disco:
            fallos.append(f"{nombre}: el caso 1 no aparece en el enunciado")
        for k, (peticiones, esperado) in enumerate(casos, 1):
            if len(peticiones) != len(esperado):
                fallos.append(f"{nombre}: caso {k}: distinto número de peticiones y respuestas")
            for sufijo, datos in (("peticiones", peticiones), ("esperado", esperado)):
                try:
                    with open(os.path.join(carpeta, "casos", f"{k}.{sufijo}.json"), encoding="utf-8") as f:
                        if json.load(f) != datos:
                            fallos.append(f"{nombre}: caso {k}.{sufijo}.json en disco no coincide con el generador")
                except (OSError, ValueError):
                    fallos.append(f"{nombre}: falta o está roto casos/{k}.{sufijo}.json (ejecuta crear_tareas.py)")
        referencias = [("python", os.path.join(RAIZ, "soluciones", "referencia", "python", nombre + ".py")),
                       ("deimon", os.path.join(RAIZ, "soluciones", "referencia", "deimon", nombre + ".dmn"))]
        for lenguaje, archivo in referencias:
            existe = os.path.isfile(archivo)
            if lenguaje == "deimon" and nombre in IMPOSIBLE_EN_DEIMON:
                if existe:
                    fallos.append(f"{nombre}: ya hay referencia en Deimon; quita la tarea de IMPOSIBLE_EN_DEIMON")
                continue
            if not existe:
                fallos.append(f"{nombre}: falta la referencia en {lenguaje}")
                continue
            detalles = []
            c, f, _o = medir_web.medir_tarea(lenguaje, archivo, carpeta, detalles.append)
            if f:
                fallos.append(f"{nombre}: la referencia de {lenguaje} falla {f}/{c} casos" + "".join("\n" + d for d in detalles))


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
    comprobar_webs(fallos)
    total = len(crear_tareas.TAREAS) + len(tareas_web.WEBS)
    if fallos:
        print("\n".join(fallos))
        print(f"FALLOS: {len(fallos)} en {total} tareas")
        sys.exit(1)
    imposibles = f" (imposibles hoy: {', '.join(sorted(IMPOSIBLE_EN_DEIMON))})" if IMPOSIBLE_EN_DEIMON else ""
    print(f"bien: {total} tareas, todas con >=4 casos y caso 1 en el enunciado; Deimon = Python en todos los casos de 01-50; "
          f"webs 51-55: referencia Python pasa las 5 y la de Deimon las {len(tareas_web.WEBS) - len(IMPOSIBLE_EN_DEIMON)}{imposibles}")


if __name__ == "__main__":
    main()
