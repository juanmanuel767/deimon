#!/usr/bin/env python3
"""Crea banco-ia/tareas/: cada tarea tiene enunciado.md, entrada.txt y esperado.txt.
La salida esperada la calcula aquí una solución de referencia, para que no haya erratas a mano.
Uso: python3 herramientas/crear_tareas.py (desde banco-ia/)."""
import os
from collections import Counter, OrderedDict

TAREAS = []


def tarea(nombre, enunciado, entrada):
    def registrar(funcion):
        TAREAS.append((nombre, enunciado.strip() + "\n", entrada, funcion))
        return funcion
    return registrar


COMUN = """
Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas; la última termina en salto
de línea) y escribe exactamente la salida pedida, cada línea terminada en salto de línea, sin
espacios de más.
"""


@tarea("01-suma", """
# Suma de números

Cada línea de la entrada tiene un número entero (puede ser negativo). Escribe una sola línea
con la suma de todos.
""", "5\n-3\n10\n0\n28\n")
def _(lineas):
    return [str(sum(int(l) for l in lineas))]


@tarea("02-fizzbuzz", """
# FizzBuzz con límite

La entrada es una línea con un entero N (1 ≤ N ≤ 100). Para cada i de 1 a N escribe una línea:
"FizzBuzz" si i es múltiplo de 3 y de 5, "Fizz" si solo de 3, "Buzz" si solo de 5, y si no,
el propio número.
""", "16\n")
def _(lineas):
    out = []
    for i in range(1, int(lineas[0]) + 1):
        out.append("FizzBuzz" if i % 15 == 0 else "Fizz" if i % 3 == 0 else "Buzz" if i % 5 == 0 else str(i))
    return out


@tarea("03-parentesis", """
# Paréntesis equilibrados

Cada línea es un texto con (), [] y {} mezclados con otras letras. Para cada línea escribe
"si" si todos los símbolos de apertura se cierran en el orden correcto, y "no" si no.
Las demás letras se ignoran. Una línea sin símbolos está equilibrada.
""", "(a[b]{c})\n([)]\n((\nhola\n{[()()]}\n)(\n")
def _(lineas):
    out = []
    pares = {")": "(", "]": "[", "}": "{"}
    for l in lineas:
        pila, bien = [], True
        for c in l:
            if c in "([{":
                pila.append(c)
            elif c in ")]}":
                if not pila or pila.pop() != pares[c]:
                    bien = False
                    break
        out.append("si" if bien and not pila else "no")
    return out


@tarea("04-palabras", """
# Palabras más frecuentes

La entrada es un texto en varias líneas. Las palabras son trozos separados por espacios; antes
de contarlas pásalas a minúsculas y quítales los signos . , ; : ! ? del principio y del final
(si se queda vacía, no cuenta). Escribe las 3 palabras más repetidas, una por línea, con el
formato "palabra: veces", de más a menos veces; si empatan, en orden alfabético.
""", "El perro y el gato.\nEl gato duerme; el PERRO no.\nY el gato, ¿qué?\n")
def _(lineas):
    c = Counter()
    for l in lineas:
        for p in l.split(" "):
            p = p.lower().strip(".,;:!?")
            if p:
                c[p] += 1
    orden = sorted(c.items(), key=lambda kv: (-kv[1], kv[0]))[:3]
    return [f"{k}: {v}" for k, v in orden]


@tarea("05-romanos", """
# Números romanos

Cada línea tiene un entero entre 1 y 3999. Escribe su número romano (I, V, X, L, C, D, M, con
las restas IV, IX, XL, XC, CD y CM).
""", "1\n4\n9\n14\n40\n90\n400\n1994\n2024\n3999\n")
def _(lineas):
    valores = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
               (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
    out = []
    for l in lineas:
        n, r = int(l), ""
        for v, s in valores:
            while n >= v:
                r += s
                n -= v
        out.append(r)
    return out


@tarea("06-ventas", """
# Ventas por producto

Cada línea es una venta: "producto;cantidad;precio_unitario" (cantidad entera, precio con punto
decimal). Escribe una línea por producto, en orden alfabético, con el formato
"producto: total" donde total es la suma de cantidad × precio con exactamente 2 decimales
(redondeo normal). Al final, una línea "TOTAL: x" con la suma de todo, también con 2 decimales.
""", "pan;3;1.20\nleche;2;0.95\npan;1;1.20\nhuevos;12;0.25\nleche;1;0.99\n")
def _(lineas):
    tot = {}
    for l in lineas:
        p, c, pr = l.split(";")
        tot[p] = tot.get(p, 0) + int(c) * float(pr)
    out = [f"{p}: {tot[p]:.2f}" for p in sorted(tot)]
    out.append(f"TOTAL: {sum(tot.values()):.2f}")
    return out


@tarea("07-personas", """
# Ordenar personas

Cada línea es "nombre,edad". Escribe las personas ordenadas por edad de mayor a menor; si tienen
la misma edad, por nombre en orden alfabético. Formato de cada línea: "nombre (edad)".
Después, una línea "media: x" con la media de las edades con 1 decimal.
""", "Lucia,34\nPedro,28\nAna,34\nZoe,19\nBruno,28\n")
def _(lineas):
    ps = [(l.split(",")[0], int(l.split(",")[1])) for l in lineas]
    out = [f"{n} ({e})" for n, e in sorted(ps, key=lambda p: (-p[1], p[0]))]
    out.append(f"media: {sum(e for _, e in ps) / len(ps):.1f}")
    return out


@tarea("08-errores", """
# Números con errores

Cada línea debería ser un número entero. Suma los que lo sean. Para cada línea que no sea un
entero válido escribe "linea N: no es un numero: TEXTO" (N empieza en 1 y TEXTO es la línea tal
cual). Al final escribe "suma: S".
""", "10\nveinte\n-5\n3.5\n1e3\n7\n")
def _(lineas):
    out, s = [], 0
    for i, l in enumerate(lineas, 1):
        try:
            if l.strip() != l or l == "":
                raise ValueError
            s += int(l)
        except ValueError:
            out.append(f"linea {i}: no es un numero: {l}")
    out.append(f"suma: {s}")
    return out


@tarea("09-matriz", """
# Matriz en espiral

La primera línea tiene dos enteros "F C" (filas y columnas, de 1 a 10). Siguen F líneas con C
enteros separados por un espacio. Escribe en una sola línea, separados por un espacio, los
números recorridos en espiral: primera fila de izquierda a derecha, después hacia abajo por la
última columna, hacia la izquierda por la última fila, hacia arriba, y así hacia dentro.
""", "3 4\n1 2 3 4\n5 6 7 8\n9 10 11 12\n")
def _(lineas):
    f, c = map(int, lineas[0].split())
    m = [list(map(int, l.split())) for l in lineas[1:1 + f]]
    out = []
    while m:
        out += m.pop(0)
        m = [list(x) for x in zip(*m)][::-1] if m else []
    return [" ".join(map(str, out))]


@tarea("10-inventario", """
# Inventario con órdenes

Cada línea es una orden sobre un inventario que empieza vacío:
- "poner PRODUCTO N": suma N unidades (N entero positivo).
- "quitar PRODUCTO N": resta N unidades; si no hay bastantes, no cambia nada y escribe
  "error: no hay N de PRODUCTO".
- "ver": escribe una línea por producto con unidades mayores que 0, en orden alfabético, con el
  formato "PRODUCTO=N"; si no hay ninguno, escribe "vacio".
""", "poner tornillo 10\nponer clavo 5\nquitar tornillo 3\nver\nquitar clavo 9\nquitar clavo 5\nver\nquitar tuerca 1\nponer arandela 2\nver\n")
def _(lineas):
    inv, out = {}, []
    for l in lineas:
        p = l.split(" ")
        if p[0] == "poner":
            inv[p[1]] = inv.get(p[1], 0) + int(p[2])
        elif p[0] == "quitar":
            n = int(p[2])
            if inv.get(p[1], 0) < n:
                out.append(f"error: no hay {n} de {p[1]}")
            else:
                inv[p[1]] -= n
        elif p[0] == "ver":
            vivos = [k for k in sorted(inv) if inv[k] > 0]
            out += [f"{k}={inv[k]}" for k in vivos] or ["vacio"]
    return out


def main():
    raiz = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tareas")
    for nombre, enunciado, entrada, funcion in TAREAS:
        carpeta = os.path.join(raiz, nombre)
        os.makedirs(carpeta, exist_ok=True)
        with open(os.path.join(carpeta, "enunciado.md"), "w") as f:
            f.write(enunciado + COMUN)
        with open(os.path.join(carpeta, "entrada.txt"), "w") as f:
            f.write(entrada)
        lineas = entrada.split("\n")[:-1]
        with open(os.path.join(carpeta, "esperado.txt"), "w") as f:
            f.write("".join(l + "\n" for l in funcion(lineas)))
    print(f"{len(TAREAS)} tareas en {os.path.normpath(raiz)}")


if __name__ == "__main__":
    main()
