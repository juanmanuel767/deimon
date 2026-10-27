#!/usr/bin/env python3
"""Crea banco-ia/tareas/. Cada tarea es una carpeta:

    tareas/NN-nombre/enunciado.md
    tareas/NN-nombre/casos/1.entrada, 1.esperado, 2.entrada, 2.esperado, ...

El caso 1 es el ejemplo visible (se copia dentro de enunciado.md); los demás son OCULTOS.
La salida esperada la calcula aquí la función de referencia en Python (nunca a mano).
Uso: python3 herramientas/crear_tareas.py (desde banco-ia/)."""
import os
import re
import shutil
import datetime
from collections import Counter

TAREAS = []


def tarea(nombre, enunciado, casos):
    """casos: lista de entradas (texto). La primera es el ejemplo visible."""
    def registrar(funcion):
        TAREAS.append((nombre, enunciado.strip() + "\n", casos, funcion))
        return funcion
    return registrar


COMUN = """
Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
"""


def euros(c):
    return f"{c // 100}.{c % 100:02d}"


def a_centimos(t):
    """'12.5' -> 1250, '12' -> 1200, '12.05' -> 1205 (como mucho 2 decimales)."""
    ent, _, dec = t.partition(".")
    return int(ent) * 100 + int((dec + "00")[:2])


def lcg(semilla):
    estado = [semilla]

    def siguiente(n):
        estado[0] = (estado[0] * 1103515245 + 12345) % 2147483648
        return (estado[0] >> 8) % n
    return siguiente


# ---------------------------------------------------------------- 01 a 10 (con casos ocultos)

@tarea("01-suma", """
# Suma de números

Cada línea de la entrada tiene un número entero (puede ser negativo). Escribe una sola línea
con la suma de todos. Si no hay ninguna línea, la suma es 0.
""", ["5\n-3\n10\n0\n28\n", "", "42\n", "-1\n-2\n-3\n", "1000000\n2000000\n-3000000\n7\n", "0\n0\n0\n"])
def _(lineas):
    return [str(sum(int(l) for l in lineas))]


@tarea("02-fizzbuzz", """
# FizzBuzz con límite

La entrada es una línea con un entero N (1 ≤ N ≤ 100). Para cada i de 1 a N escribe una línea:
"FizzBuzz" si i es múltiplo de 3 y de 5, "Fizz" si solo de 3, "Buzz" si solo de 5, y si no,
el propio número.
""", ["16\n", "1\n", "15\n", "100\n", "3\n", "30\n"])
def _(lineas):
    out = []
    for i in range(1, int(lineas[0]) + 1):
        out.append("FizzBuzz" if i % 15 == 0 else "Fizz" if i % 3 == 0 else "Buzz" if i % 5 == 0 else str(i))
    return out


@tarea("03-parentesis", """
# Paréntesis equilibrados

Cada línea es un texto con (), [] y {} mezclados con otras letras. Para cada línea escribe
"si" si todos los símbolos de apertura se cierran en el orden correcto, y "no" si no.
Las demás letras se ignoran. Una línea sin símbolos (también una línea vacía) está equilibrada.
""", ["(a[b]{c})\n([)]\n((\nhola\n{[()()]}\n)(\n", "", "\n", "((()))\n(()\n)\n]\n",
      "{[(])}\nsin símbolos ñandú\n[a]{b}(c)\n}{\n", "{{{{[[[[((((x))))]]]]}}}}\n(((((((((((\n"])
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

La entrada es un texto en varias líneas. Las palabras son trozos separados por espacios (puede
haber varios espacios seguidos); antes de contarlas pásalas a minúsculas (también las letras con
acento y la ñ) y quítales los signos . , ; : ! ? del principio y del final (si se queda vacía, no
cuenta; los demás signos, como ¿ o ¡, no se quitan). Escribe las 3 palabras más repetidas, una por
línea, con el formato "palabra: veces", de más a menos veces; si empatan, en orden alfabético.
Si hay menos de 3 palabras distintas, escribe solo las que haya.
""", ["El perro y el gato.\nEl gato duerme; el PERRO no.\nY el gato, ¿qué?\n", "", "Hola\n",
      "Árbol árbol ÁRBOL niño Niño, ñu ñu ñu\n", "...!!!\nhola ... ,hola; ¿hola?\n",
      "a  b   c a b a\nz y x z y x\nd\n", "uno dos tres cuatro cinco\n"])
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
""", ["1\n4\n9\n14\n40\n90\n400\n1994\n2024\n3999\n", "3\n8\n27\n44\n99\n444\n1000\n3888\n", "1\n",
      "3999\n", "2999\n1666\n", "", "49\n94\n149\n444\n949\n1444\n"])
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
decimal y como mucho 2 decimales). Escribe una línea por producto, en orden alfabético, con el
formato "producto: total" donde total es la suma de cantidad × precio con exactamente 2 decimales.
Al final, una línea "TOTAL: x" con la suma de todo, también con 2 decimales. Si no hay ventas,
solo se escribe "TOTAL: 0.00".
""", ["pan;3;1.20\nleche;2;0.95\npan;1;1.20\nhuevos;12;0.25\nleche;1;0.99\n", "", "pan;1;1\n",
      "x;1;0.10\nx;1;0.20\nx;1;0.30\n", "ñandú;3;19.99\nÁrbol;2;0.05\nzumo;1000;12.5\n",
      "a;0;5.00\nb;7;0.01\na;2;0.5\n"])
def _(lineas):
    tot = {}
    for l in lineas:
        p, c, pr = l.split(";")
        tot[p] = tot.get(p, 0) + int(c) * a_centimos(pr)
    out = [f"{p}: {euros(tot[p])}" for p in sorted(tot)]
    out.append(f"TOTAL: {euros(sum(tot.values()))}")
    return out


@tarea("07-personas", """
# Ordenar personas

Cada línea es "nombre,edad" (edad entera, no negativa). Escribe las personas ordenadas por edad de
mayor a menor; si tienen la misma edad, por nombre en orden alfabético. Formato de cada línea:
"nombre (edad)". Después, una línea "media: x" con la media de las edades con 1 decimal (si queda
justo en la mitad, se redondea hacia arriba: 28.25 → 28.3). La entrada tiene al menos una persona.
""", ["Lucia,34\nPedro,28\nAna,34\nZoe,19\nBruno,28\n", "Solo,0\n", "Ana,20\nBea,21\n",
      "Ñandú,40\nÁlvaro,40\nAlba,40\nzoe,40\n", "a,1\nb,2\nc,3\nd,4\n", "a,10\nb,11\nc,10\nd,10\ne,0\nf,0\ng,0\nh,1\n"])
def _(lineas):
    ps = [(l.split(",")[0], int(l.split(",")[1])) for l in lineas]
    out = [f"{n} ({e})" for n, e in sorted(ps, key=lambda p: (-p[1], p[0]))]
    n = len(ps)
    d = (sum(e for _, e in ps) * 20 + n) // (2 * n)
    out.append(f"media: {d // 10}.{d % 10}")
    return out


@tarea("08-errores", """
# Números con errores

Cada línea debería ser un número entero: un signo menos opcional seguido de una o más cifras 0-9,
sin espacios ni otros símbolos (por ejemplo "-5" y "007" valen; "+5", " 5", "3.5", "1e3", "-" y
la línea vacía no). Suma los que lo sean. Para cada línea que no sea un entero válido escribe
"linea N: no es un numero: TEXTO" (N empieza en 1 y TEXTO es la línea tal cual). Al final
escribe "suma: S".
""", ["10\nveinte\n-5\n3.5\n1e3\n7\n", "", "-0\n+5\n 5\n5 \n--3\n-\n007\n", "1\n2\n3\n",
      "abc\nñ\n12abc\n99999999\n-99999999\n", "x\n\ny\n"])
def _(lineas):
    out, s = [], 0
    for i, l in enumerate(lineas, 1):
        if re.fullmatch(r"-?[0-9]+", l):
            s += int(l)
        else:
            out.append(f"linea {i}: no es un numero: {l}")
    out.append(f"suma: {s}")
    return out


@tarea("09-matriz", """
# Matriz en espiral

La primera línea tiene dos enteros "F C" (filas y columnas, de 1 a 10). Siguen F líneas con C
enteros separados por un espacio. Escribe en una sola línea, separados por un espacio, los
números recorridos en espiral: primera fila de izquierda a derecha, después hacia abajo por la
última columna, hacia la izquierda por la última fila, hacia arriba, y así hacia dentro.
""", ["3 4\n1 2 3 4\n5 6 7 8\n9 10 11 12\n", "1 1\n7\n", "1 4\n1 2 3 4\n", "4 1\n1\n2\n3\n4\n",
      "4 4\n1 2 3 4\n5 6 7 8\n9 10 11 12\n13 14 15 16\n", "2 3\n-1 -2 -3\n4 5 6\n",
      "5 3\n1 2 3\n4 5 6\n7 8 9\n10 11 12\n13 14 15\n", "3 3\n1 2 3\n4 5 6\n7 8 9\n"])
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
- "quitar PRODUCTO N": resta N unidades; si no hay bastantes (o el producto no existe), no cambia
  nada y escribe "error: no hay N de PRODUCTO".
- "ver": escribe una línea por producto con unidades mayores que 0, en orden alfabético, con el
  formato "PRODUCTO=N"; si no hay ninguno, escribe "vacio".
""", ["poner tornillo 10\nponer clavo 5\nquitar tornillo 3\nver\nquitar clavo 9\nquitar clavo 5\nver\nquitar tuerca 1\nponer arandela 2\nver\n",
      "ver\n", "quitar x 1\nver\n", "poner a 1\nponer a 1\nquitar a 2\nver\nquitar a 1\n",
      "poner ñandú 3\nponer árbol 2\nponer Zeta 1\nponer abc 7\nver\n", "", "poner a 5\nquitar a 5\nver\nponer a 1\nver\n"])
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


# ---------------------------------------------------------------- 11 a 40 (nuevas)

@tarea("11-csv-comillas", """
# CSV con comillas simples

Cada línea (nunca vacía) es una fila CSV con los campos separados por comas. Un campo puede ir
entre comillas simples (') y entonces las comas de dentro no separan campos; las comillas no
forman parte del valor. Las comillas siempre abarcan el campo entero (no hay comillas dentro de
un campo sin comillas) y siempre están cerradas. Los espacios se conservan. Un campo puede estar
vacío (por ejemplo "a,,b" tiene 3 campos, y "a,b," también).

Para cada fila escribe "N campos: [campo1][campo2]..." con N el número de campos y cada valor
entre corchetes, todo pegado. (Siempre se escribe la palabra "campos".)
""", ["ana,'Madrid, España',30\nx,y\n", "a,,b\n", "a,b,\n", "'',x\n", "'a,b,c'\n", "solo\n",
      "ñu,'año, ñandú',  espacio ,'  '\n", ",\n", "'una,dos','tres,cuatro',cinco\n"])
def _(lineas):
    out = []
    for l in lineas:
        campos, cur, dentro = [], "", False
        for c in l:
            if c == "'":
                dentro = not dentro
            elif c == "," and not dentro:
                campos.append(cur)
                cur = ""
            else:
                cur += c
        campos.append(cur)
        out.append(f"{len(campos)} campos: " + "".join(f"[{c}]" for c in campos))
    return out


@tarea("12-plantilla", """
# Plantilla con marcadores

La primera línea es una plantilla de texto. Las demás líneas (puede no haber ninguna) son
definiciones "clave=valor": la clave es lo que hay antes del primer "=", y el valor es todo lo
que sigue (puede contener "=" y espacios; nunca está vacío). Si una clave se define varias veces,
vale la última.

En la plantilla, busca de izquierda a derecha cada "{". Si lo que sigue hasta la primera "}"
(sin incluirla) es un nombre válido, es decir, una o más letras ASCII sin acento, cifras o "_",
el marcador "{nombre}" se sustituye por su valor, o se deja tal cual (con las llaves) si esa
clave no está definida; y se sigue justo después de la "}". Si no es un nombre válido (o no hay
"}" después), la "{" se escribe tal cual y se sigue con el carácter siguiente. El texto
sustituido no se vuelve a examinar. Escribe una sola línea con el resultado.
""", ["Hola {nombre}, tienes {n} mensajes y {otro}.\nnombre=Ana\nn=3\n", "sin marcadores\n",
      "{a}{b}{a}\na=1\nb=dos palabras\na=3\n", "{{a}} { a} {} {a b} {a\na=X\n", "{x}\nx={y}\ny=no\n",
      "Año {año} y {ano}\nano=2024\naño=mal\n", "{a_1}={A}\na_1=k=v\nA=mayúscula\n", "}{a}{\na=1\n"])
def _(lineas):
    plantilla, defs = lineas[0], {}
    for l in lineas[1:]:
        k, _, v = l.partition("=")
        defs[k] = v
    out, i = "", 0
    while i < len(plantilla):
        c = plantilla[i]
        if c == "{":
            j = plantilla.find("}", i + 1)
            if j != -1 and re.fullmatch(r"[A-Za-z0-9_]+", plantilla[i + 1:j]):
                nombre = plantilla[i + 1:j]
                out += defs.get(nombre, "{" + nombre + "}")
                i = j + 1
                continue
        out += c
        i += 1
    return [out]


@tarea("13-justificar", """
# Justificar texto

La primera línea es un entero ANCHO. El resto de líneas es un texto: sus palabras (trozos
separados por uno o más espacios, de todas las líneas, en orden) se reparten en líneas de como
mucho ANCHO caracteres: en cada línea se ponen todas las palabras que quepan (separadas por un
espacio como mínimo) antes de pasar a la siguiente. Ninguna palabra es más larga que ANCHO.

Luego se justifica: en cada línea, salvo la última, que tenga 2 o más palabras, se reparten
espacios entre las palabras hasta que la línea mida exactamente ANCHO caracteres; todos los
huecos reciben los mismos espacios y, si sobran, un espacio más cada uno de los primeros huecos
(empezando por la izquierda). Una línea con una sola palabra y la última línea se dejan con las
palabras separadas por un solo espacio. Cuenta caracteres (á o ñ cuentan 1). Escribe las líneas
resultantes; si no hay palabras, no escribes nada.
""", ["20\nEl veloz murciélago hindú comía feliz cardillo y kiwi.\n", "5\n", "10\nuna\n", "10\nab cd ef gh ij kl\n",
      "12\nla  canción\n\ndel niño   y la acción número ñ\n", "7\nhola\nmundo bonito de verdad\n",
      "30\nSe repartirán los espacios sobrantes hacia la izquierda primero en cada línea justificada con cuidado\n",
      "4\nabcd abcd ab abcd a\n"])
def _(lineas):
    ancho = int(lineas[0])
    palabras = [p for l in lineas[1:] for p in l.split(" ") if p]
    filas, fila = [], []
    for p in palabras:
        if fila and len(" ".join(fila)) + 1 + len(p) > ancho:
            filas.append(fila)
            fila = []
        fila.append(p)
    if fila:
        filas.append(fila)
    out = []
    for k, f in enumerate(filas):
        if k == len(filas) - 1 or len(f) == 1:
            out.append(" ".join(f))
            continue
        huecos = len(f) - 1
        total = ancho - sum(len(p) for p in f)
        base, extra = divmod(total, huecos)
        s = ""
        for h, p in enumerate(f):
            s += p
            if h < huecos:
                s += " " * (base + (1 if h < extra else 0))
        out.append(s)
    return out


@tarea("14-palabras-acentos", """
# Contar palabras con acentos

La entrada es un texto en varias líneas. Una palabra es una secuencia máxima de letras, y las
letras son solo: a-z, A-Z, á é í ó ú ü ñ y sus mayúsculas Á É Í Ó Ú Ü Ñ. Cualquier otro carácter
(espacios, cifras, signos, ¿ ¡, guiones...) separa palabras. Pasa las palabras a minúsculas.

Escribe "total: N" (palabras en total, con repeticiones), "distintas: M" y luego las 5 palabras
más repetidas, una por línea, con el formato "palabra: veces", de más a menos veces y, si
empatan, en orden alfabético. Si hay menos de 5 palabras distintas, escribe solo las que haya.
""", ["¿Qué hace el niño? El niño come; la niña no come.\n", "", "123 -- ¡!\n", "Ñandú ñandú ÑANDÚ año AÑO\n",
      "uno-dos tres_cuatro cinco5seis\n", "a b c d e f g a b c d e a b c a b a\n",
      "Árbol\nárbol, ÁRBOL: árboles\nPingüino pingüino\n"])
def _(lineas):
    letras = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZáéíóúüñÁÉÍÓÚÜÑ")
    c, total = Counter(), 0
    for l in lineas:
        actual = ""
        for ch in l + " ":
            if ch in letras:
                actual += ch
            elif actual:
                c[actual.lower()] += 1
                total += 1
                actual = ""
    out = [f"total: {total}", f"distintas: {len(c)}"]
    for k, v in sorted(c.items(), key=lambda kv: (-kv[1], kv[0]))[:5]:
        out.append(f"{k}: {v}")
    return out


@tarea("15-mapas-anidados", """
# Mapas anidados

Cada línea es "ruta=valor". La ruta son una o más claves separadas por puntos (por ejemplo
"servidor.puerto"); el valor es todo lo que sigue al primer "=" (puede contener puntos, "=" y
espacios; nunca está vacío ni empieza o acaba en espacio). Las claves no están vacías ni
contienen "=" ni ".". Ninguna ruta es prefijo propio de otra (si "a.b" existe, no existe "a" ni
"a.b.c"). Si una ruta se repite, vale el último valor.

Construye el árbol de claves y escríbelo: cada clave en su propia línea, con 2 espacios de
sangría por nivel de profundidad (nivel 0 sin sangría), y las claves de cada nivel en orden
alfabético. Una clave con hijos se escribe "clave:"; una clave con valor, "clave: valor".
""", ["servidor.puerto=8080\nservidor.host=local host\nnombre=app\nservidor.tls.activo=si\n", "",
      "a=1\n", "a.b.c.d=profundo\na.b.c.e=otro\na.z=2\n", "x=1\nx=2\ny.k=a=b\ny.k=c.d=e\n",
      "ñ.á=1\nñ.a=2\nZ=3\na=4\n", "k=un valor con espacios\n"])
def _(lineas):
    arbol = {}
    for l in lineas:
        ruta, _, valor = l.partition("=")
        claves = ruta.split(".")
        nodo = arbol
        for k in claves[:-1]:
            nodo = nodo.setdefault(k, {})
        nodo[claves[-1]] = valor
    out = []

    def escribir(nodo, nivel):
        for k in sorted(nodo):
            if isinstance(nodo[k], dict):
                out.append("  " * nivel + k + ":")
                escribir(nodo[k], nivel + 1)
            else:
                out.append("  " * nivel + f"{k}: {nodo[k]}")
    escribir(arbol, 0)
    return out


@tarea("16-agrupar-ordenar", """
# Agrupar y ordenar por varios campos

Cada línea es "departamento,nombre,salario" (salario entero). Agrupa por departamento. Escribe
los departamentos en orden alfabético; cada uno con una línea "departamento (N personas): total T",
donde N es cuántas personas tiene (siempre se escribe la palabra "personas") y T la suma de sus
salarios; y debajo una línea por persona, con dos espacios de sangría y el formato
"nombre: salario", ordenadas por salario de mayor a menor y, si empatan, por nombre en orden
alfabético. Si una misma persona aparece varias veces, cada línea cuenta como una persona más.
""", ["ventas,Ana,1500\ncompras,Luis,1200\nventas,Bea,1800\ncompras,Carmen,1200\nventas,Dani,1500\n", "",
      "it,Solo,1\n", "b,x,5\na,y,5\nb,a,5\na,y,5\n", "ñ,Ñoño,100\nñ,Álex,100\nñ,zoe,100\nZ,u,0\n",
      "d1,a,1000000\nd1,b,-5\nd2,c,0\n"])
def _(lineas):
    grupos = {}
    for l in lineas:
        d, n, s = l.split(",")
        grupos.setdefault(d, []).append((n, int(s)))
    out = []
    for d in sorted(grupos):
        ps = grupos[d]
        out.append(f"{d} ({len(ps)} personas): total {sum(s for _, s in ps)}")
        for n, s in sorted(ps, key=lambda p: (-p[1], p[0])):
            out.append(f"  {n}: {s}")
    return out


@tarea("17-pila-cola", """
# Pila y cola

Hay una pila y una cola, las dos vacías al principio. Cada línea es una orden (los elementos X
son palabras sin espacios):
- "apilar X": pone X encima de la pila.
- "desapilar": quita el elemento de arriba de la pila y escribe "pila->X" (con X ese elemento);
  si la pila está vacía escribe "error: pila vacia".
- "encolar X": pone X al final de la cola.
- "desencolar": quita el primer elemento de la cola y escribe "cola->X"; si la cola está vacía
  escribe "error: cola vacia".
- "mostrar": escribe dos líneas: "pila: " seguido de los elementos desde el fondo hasta arriba
  separados por un espacio, y "cola: " seguido de los elementos desde el primero hasta el último
  separados por un espacio. Si está vacía, escribe "vacia" en lugar de los elementos
  (por ejemplo "pila: vacia").
""", ["apilar a\napilar b\nencolar 1\nencolar 2\nmostrar\ndesapilar\ndesencolar\nmostrar\n", "mostrar\n",
      "desapilar\ndesencolar\n", "", "apilar x\ndesapilar\ndesapilar\nmostrar\n",
      "encolar ñ\nencolar á\napilar ñ\napilar á\nmostrar\ndesencolar\ndesapilar\ndesencolar\ndesencolar\nmostrar\n"])
def _(lineas):
    pila, cola, out = [], [], []
    for l in lineas:
        p = l.split(" ")
        if p[0] == "apilar":
            pila.append(p[1])
        elif p[0] == "desapilar":
            out.append(f"pila->{pila.pop()}" if pila else "error: pila vacia")
        elif p[0] == "encolar":
            cola.append(p[1])
        elif p[0] == "desencolar":
            out.append(f"cola->{cola.pop(0)}" if cola else "error: cola vacia")
        elif p[0] == "mostrar":
            out.append("pila: " + (" ".join(pila) or "vacia"))
            out.append("cola: " + (" ".join(cola) or "vacia"))
    return out


def leer_cuadricula(lineas):
    f, c = map(int, lineas[0].split())
    return f, c, [lineas[1 + i] for i in range(f)]


@tarea("18-camino-cuadricula", """
# Camino más corto en una cuadrícula

La primera línea es "F C" (filas y columnas, de 1 a 12). Siguen F líneas de C caracteres:
"." casilla libre, "#" muro, "S" salida y "E" entrada (hay exactamente una de cada). Moviéndote
una casilla cada vez hacia arriba, abajo, izquierda o derecha (no en diagonal), sin pisar muros
ni salir de la cuadrícula, escribe el número mínimo de movimientos para ir de S a E. Si no se
puede llegar, escribe "sin camino".
""", ["3 4\nS..#\n.#..\n...E\n", "1 2\nSE\n", "1 3\nS#E\n", "3 3\nS#.\n.#.\n.#E\n",
      "5 5\nS....\n####.\n.....\n.####\n....E\n", "2 2\nE.\n.S\n",
      "4 6\nS.....\n.####.\n.#..#.\n.#.E#.\n"])
def _(lineas):
    f, c, g = leer_cuadricula(lineas)
    for i in range(f):
        for j in range(c):
            if g[i][j] == "S":
                s = (i, j)
            if g[i][j] == "E":
                e = (i, j)
    dist = {s: 0}
    cola = [s]
    while cola:
        i, j = cola.pop(0)
        if (i, j) == e:
            return [str(dist[e])]
        for di, dj in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            a, b = i + di, j + dj
            if 0 <= a < f and 0 <= b < c and g[a][b] != "#" and (a, b) not in dist:
                dist[(a, b)] = dist[(i, j)] + 1
                cola.append((a, b))
    return ["sin camino"]


@tarea("19-islas", """
# Contar islas

La primera línea es "F C" (filas y columnas, de 1 a 12). Siguen F líneas de C caracteres:
"#" tierra y "." agua. Una isla es un grupo de casillas de tierra conectadas por arriba, abajo,
izquierda o derecha (no en diagonal). Escribe dos líneas: "islas: N" (cuántas hay) y
"mayor: M" (cuántas casillas tiene la isla más grande, 0 si no hay ninguna).
""", ["4 5\n##...\n#..#.\n...##\n.#...\n", "1 1\n.\n", "1 1\n#\n", "2 2\n#.\n.#\n", "3 3\n###\n###\n###\n",
      "3 4\n....\n....\n....\n", "5 5\n#.#.#\n.....\n#.#.#\n.....\n#.#.#\n", "4 4\n#..#\n####\n#..#\n.##.\n"])
def _(lineas):
    f, c, g = leer_cuadricula(lineas)
    visto, n, mayor = set(), 0, 0
    for i in range(f):
        for j in range(c):
            if g[i][j] == "#" and (i, j) not in visto:
                n += 1
                visto.add((i, j))
                pila, tam = [(i, j)], 0
                while pila:
                    a, b = pila.pop()
                    tam += 1
                    for da, db in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                        x, y = a + da, b + db
                        if 0 <= x < f and 0 <= y < c and g[x][y] == "#" and (x, y) not in visto:
                            visto.add((x, y))
                            pila.append((x, y))
                mayor = max(mayor, tam)
    return [f"islas: {n}", f"mayor: {mayor}"]


@tarea("20-factorial-grande", """
# Factorial exacto

Cada línea tiene un entero N entre 0 y 60. Escribe N! (el factorial de N, con 0! = 1) con todas
sus cifras, exacto (sin notación científica ni redondeos).
""", ["5\n20\n25\n", "0\n1\n", "40\n", "60\n", "10\n15\n", "", "21\n22\n23\n"])
def _(lineas):
    import math
    return [str(math.factorial(int(l))) for l in lineas]


@tarea("21-fibonacci-grande", """
# Fibonacci exacto

Cada línea tiene un entero N entre 0 y 400. Escribe el término N de la sucesión de Fibonacci con
F(0) = 0, F(1) = 1 y F(N) = F(N-1) + F(N-2), con todas sus cifras, exacto.
""", ["10\n50\n90\n", "0\n1\n2\n", "150\n", "400\n", "93\n94\n", "", "200\n"])
def _(lineas):
    out = []
    for l in lineas:
        a, b = 0, 1
        for _ in range(int(l)):
            a, b = b, a + b
        out.append(str(a))
    return out


@tarea("22-producto-grande", """
# Producto de enteros grandes

Cada línea tiene dos enteros no negativos "A B", sin ceros a la izquierda salvo el propio 0
(hasta 40 cifras cada uno). Escribe A × B exacto, con todas sus cifras.
""", ["12 34\n99999999999 99999999999\n", "0 0\n", "0 123456789012345678901234567890\n", "1 1\n",
      "123456789012345678901234567890 987654321098765432109876543210\n",
      "9999999999999999999999999999999999999999 9999999999999999999999999999999999999999\n",
      "10000000000000000000 10000000000000000000\n", ""])
def _(lineas):
    return [str(int(l.split()[0]) * int(l.split()[1])) for l in lineas]


@tarea("23-iva-redondeo", """
# IVA con redondeo exacto

Cada línea es "base tipo": la base imponible en euros con exactamente 2 decimales (por ejemplo
"12.35") y el tipo de IVA, un entero de 0 a 100 (%). El IVA, en céntimos, es base × tipo / 100
redondeado al céntimo más cercano, y si queda justo en la mitad, hacia arriba. El total es la
base más el IVA.

Para cada línea escribe "BASE TIPO% -> iva X total Y" con BASE, X e Y en euros con exactamente
2 decimales (BASE tal como se escribe con 2 decimales). Al final escribe "suma de totales: Z"
con la suma de todos los totales (2 decimales); con la entrada vacía escribe solo
"suma de totales: 0.00".
""", ["12.35 21\n100.00 10\n0.50 4\n", "", "0.00 21\n", "0.05 10\n0.15 10\n0.25 10\n0.35 10\n", "99999.99 100\n",
      "1.01 50\n1.03 50\n2.50 0\n", "19.99 21\n7.77 7\n"])
def _(lineas):
    out, suma = [], 0
    for l in lineas:
        b, t = l.split()
        c = a_centimos(b)
        iva = (c * int(t) + 50) // 100
        suma += c + iva
        out.append(f"{euros(c)} {int(t)}% -> iva {euros(iva)} total {euros(c + iva)}")
    out.append(f"suma de totales: {euros(suma)}")
    return out


def _fecha(t):
    a, m, d = t.split("-")
    return datetime.date(int(a), int(m), int(d))


@tarea("24-dias-entre-fechas", """
# Días entre fechas

Cada línea tiene dos fechas válidas "AAAA-MM-DD AAAA-MM-DD" (años de 1600 a 2400, calendario
gregoriano: son bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que no lo son de
400). Escribe los días que hay desde la primera fecha hasta la segunda (negativo si la segunda
es anterior; 0 si son iguales).
""", ["2024-01-01 2024-12-31\n2000-02-28 2000-03-01\n", "", "2024-05-05 2024-05-05\n", "2024-03-01 2024-02-01\n",
      "1900-02-28 1900-03-01\n2100-02-28 2100-03-01\n", "1600-01-01 2400-12-31\n", "1999-12-31 2000-01-01\n",
      "2023-02-28 2023-03-01\n2024-02-28 2024-03-01\n"])
def _(lineas):
    return [str((_fecha(l.split()[1]) - _fecha(l.split()[0])).days) for l in lineas]


@tarea("25-dia-semana", """
# Día de la semana

Cada línea es una fecha "AAAA-MM-DD" (siempre con 4 cifras, guion, 2 cifras, guion y 2 cifras; el
año va de 1600 a 2400, calendario gregoriano: son bisiestos los años múltiplos de 4, salvo los
múltiplos de 100 que no lo son de 400). Si la fecha existe, escribe "FECHA: dia", con el día de
la semana en minúsculas y sin acentos: lunes, martes, miercoles, jueves, viernes, sabado o
domingo. Si no existe (mes 00 o mayor de 12, día 00 o más del que tiene ese mes), escribe
"FECHA: invalida". FECHA es la línea tal cual.
""", ["2024-02-29\n2023-02-29\n2000-01-01\n", "", "1900-02-29\n2000-02-29\n2100-02-29\n", "2024-00-10\n2024-13-01\n2024-04-31\n2024-04-30\n2024-01-00\n",
      "1600-01-01\n2400-12-31\n1970-01-01\n", "2026-10-26\n2026-10-25\n2026-10-27\n2026-10-28\n2026-10-29\n2026-10-30\n2026-10-31\n"])
def _(lineas):
    dias = ["lunes", "martes", "miercoles", "jueves", "viernes", "sabado", "domingo"]
    out = []
    for l in lineas:
        try:
            out.append(f"{l}: {dias[_fecha(l).weekday()]}")
        except ValueError:
            out.append(f"{l}: invalida")
    return out


@tarea("26-sumar-dias", """
# Sumar días a una fecha

Cada línea es "AAAA-MM-DD N": una fecha válida (años de 1600 a 2400, calendario gregoriano: son
bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que no lo son de 400) y un entero
N entre -100000 y 100000 (puede ser negativo). Escribe la fecha que resulta de sumarle N días,
también como "AAAA-MM-DD" (año con 4 cifras, mes y día con 2). El resultado siempre cae entre
los años 1000 y 9999.
""", ["2024-01-31 1\n2024-02-28 2\n2023-12-31 1\n", "", "2024-03-01 -1\n2023-03-01 -1\n", "2000-01-01 0\n",
      "2024-01-01 366\n2023-01-01 365\n", "1600-01-01 100000\n2400-12-31 -100000\n", "2100-02-28 1\n2000-02-28 1\n"])
def _(lineas):
    out = []
    for l in lineas:
        f, n = l.split()
        out.append((_fecha(f) + datetime.timedelta(days=int(n))).isoformat())
    return out


@tarea("27-cola-banco", """
# Cola de banco

La primera línea es K, el número de cajas (1 a 10). Siguen líneas "llegada duracion nombre": el
minuto en que llega un cliente, los minutos que dura su gestión (enteros, duración ≥ 1) y su
nombre (sin espacios). Los clientes vienen ordenados por llegada (puede haber empates, y entonces
se atienden en el orden de la entrada).

Se procesan los clientes en orden. Cada caja tiene un instante en que queda libre (al principio,
0). Para cada cliente se elige la caja que quede libre antes (menor instante libre; si empatan, la
de menor número, numeradas desde 1). El cliente empieza en el mayor entre su llegada y ese
instante, y la caja queda libre en "empieza + duracion". Espera = empieza − llegada.

Para cada cliente escribe "nombre: caja C empieza S espera W". Al final escribe
"espera total: T" (suma de esperas) y "termina: M" (el instante en que acaba el último en acabar;
0 si no hay clientes).
""", ["2\n0 5 ana\n1 3 bea\n2 4 carlos\n3 1 dora\n", "1\n", "1\n0 2 a\n0 2 b\n0 2 c\n", "3\n0 10 x\n0 10 y\n0 10 z\n0 1 w\n",
      "2\n5 1 a\n5 1 b\n5 1 c\n20 3 ñ\n", "10\n0 1 a\n", "2\n0 100 largo\n1 1 a\n2 1 b\n3 1 c\n150 1 d\n"])
def _(lineas):
    k = int(lineas[0])
    libres = [0] * k
    out, esp, fin = [], 0, 0
    for l in lineas[1:]:
        ll, du, no = l.split(" ")
        ll, du = int(ll), int(du)
        c = min(range(k), key=lambda i: (libres[i], i))
        ini = max(ll, libres[c])
        libres[c] = ini + du
        esp += ini - ll
        fin = max(fin, libres[c])
        out.append(f"{no}: caja {c + 1} empieza {ini} espera {ini - ll}")
    out += [f"espera total: {esp}", f"termina: {fin}"]
    return out


@tarea("28-juego-vida", """
# Juego de la vida

La primera línea es "F C G": filas, columnas (de 1 a 10) y número de generaciones (de 0 a 20).
Siguen F líneas de C caracteres: "#" célula viva y "." muerta. En cada generación, todas las
células cambian a la vez: una célula viva con 2 o 3 vecinas vivas sigue viva, una muerta con
exactamente 3 vecinas vivas nace, y las demás quedan o pasan a muertas. Las vecinas son las 8
casillas que rodean a la célula; fuera de la cuadrícula todo está muerto (no hay vuelta
alrededor de los bordes). Escribe la cuadrícula tras G generaciones, en F líneas.
""", ["5 5 2\n.....\n..#..\n..#..\n..#..\n.....\n", "1 1 5\n#\n", "2 2 3\n##\n##\n", "3 3 0\n#.#\n.#.\n#.#\n",
      "4 4 1\n.#..\n..#.\n###.\n....\n", "6 6 4\n.#....\n..#...\n###...\n......\n......\n......\n",
      "3 3 1\n###\n###\n###\n", "1 5 1\n###..\n"])
def _(lineas):
    f, c, g = map(int, lineas[0].split())
    m = [[ch == "#" for ch in lineas[1 + i]] for i in range(f)]
    for _ in range(g):
        n = [[False] * c for _ in range(f)]
        for i in range(f):
            for j in range(c):
                v = sum(m[a][b] for a in range(i - 1, i + 2) for b in range(j - 1, j + 2)
                        if 0 <= a < f and 0 <= b < c and (a, b) != (i, j))
                n[i][j] = v == 3 or (m[i][j] and v == 2)
        m = n
    return ["".join("#" if x else "." for x in fila) for fila in m]


@tarea("29-robot", """
# Robot en una cuadrícula

Cada línea (nunca vacía) es una secuencia de órdenes con las letras A, G e I, para un robot que
empieza en la posición (0, 0) mirando al norte. "A" avanza una casilla en la dirección en que mira,
"G" gira 90° a la derecha sin moverse e "I" gira 90° a la izquierda. El norte es y creciente,
el este x creciente, el sur y decreciente y el oeste x decreciente (mirando al norte, girar a la
derecha lleva al este). Cada línea es un robot nuevo, que parte otra vez de (0, 0) mirando al norte.

Para cada línea escribe "x,y direccion max=D": la posición final, la dirección final (norte, este,
sur u oeste) y D, el mayor valor de x×x + y×y entre todas las posiciones por las que pasó (la
inicial incluida).
""", ["AAGAAIA\nGGA\n", "AAAA\n", "A\n", "GAGAGAGA\n", "IAIAIAIA\nIIIIIIII\nGGGG\n",
      "AAAAAAAAAAGAAAAAAAAAAGGGAAAAAAAAAAAAAAAAAAAAAAAAIAAA\n", "GA\nIA\nGGA\nIIA\n"])
def _(lineas):
    nombres = ["norte", "este", "sur", "oeste"]
    dx, dy = [0, 1, 0, -1], [1, 0, -1, 0]
    out = []
    for l in lineas:
        x = y = d = 0
        mx = 0
        for c in l:
            if c == "A":
                x, y = x + dx[d], y + dy[d]
                mx = max(mx, x * x + y * y)
            elif c == "G":
                d = (d + 1) % 4
            elif c == "I":
                d = (d + 3) % 4
        out.append(f"{x},{y} {nombres[d]} max={mx}")
    return out


def _calcular(texto):
    pos = [0]
    t = texto.replace(" ", "")

    def expr():
        v = termino()
        while pos[0] < len(t) and t[pos[0]] in "+-":
            op = t[pos[0]]
            pos[0] += 1
            w = termino()
            v = v + w if op == "+" else v - w
        return v

    def termino():
        v = unario()
        while pos[0] < len(t) and t[pos[0]] in "*/":
            op = t[pos[0]]
            pos[0] += 1
            w = unario()
            if op == "*":
                v = v * w
            else:
                if w == 0:
                    raise ZeroDivisionError
                q = abs(v) // abs(w)
                v = q if (v < 0) == (w < 0) else -q
        return v

    def unario():
        if t[pos[0]] == "-":
            pos[0] += 1
            return -unario()
        if t[pos[0]] == "(":
            pos[0] += 1
            v = expr()
            pos[0] += 1
            return v
        ini = pos[0]
        while pos[0] < len(t) and t[pos[0]].isdigit():
            pos[0] += 1
        return int(t[ini:pos[0]])
    return expr()


@tarea("30-calculadora", """
# Calculadora con precedencia

Cada línea es una expresión con enteros sin signo (hasta 9 cifras), los operadores + - * /,
paréntesis y espacios en cualquier sitio entre símbolos. Está bien escrita. Reglas: * y / se
calculan antes que + y -; a igual prioridad, de izquierda a derecha; los paréntesis mandan; el
signo menos también puede ir delante de un número o de un paréntesis (por ejemplo "-3", "2 * -3",
"-(1+2)", "--4"), y se aplica antes que cualquier operador binario de esa expresión simple.
La división es entera redondeando hacia cero (7 / 2 = 3 y -7 / 2 = -3). Los resultados
intermedios nunca pasan de 10^15 en valor absoluto. Escribe el valor de cada expresión; si se
divide entre cero, escribe "error: division por cero".
""", ["2 + 3 * 4\n(2 + 3) * 4\n10 / 3\n-7 / 2\n", "", "1\n", "2 * -3\n-(1+2)\n--4\n", "100 / 10 / 5\n100 - 10 - 5\n2*3+4*5-6/2\n",
      "1 / 0\n5 + 3 / (2 - 2)\n0 / 5\n", "((((7))))\n  ( 1 + 2 ) * ( 3 + 4 )  \n", "-7 / -2\n7 / -2\n-1 - -1\n99999 * 99999\n",
      "8 / 2 * (2 + 2)\n"])
def _(lineas):
    out = []
    for l in lineas:
        try:
            out.append(str(_calcular(l)))
        except ZeroDivisionError:
            out.append("error: division por cero")
    return out


@tarea("31-json-tienda", """
# Leer un JSON pequeño

La entrada es un único documento JSON (puede ocupar varias líneas, con cualquier sangría) con esta
forma: {"tienda": TEXTO, "productos": [ {"nombre": TEXTO, "precio": ENTERO, "stock": ENTERO}, ... ]}.
El precio está en céntimos. Los objetos pueden traer las claves en cualquier orden y pueden tener
además otras claves (textos, números, listas u objetos) que se ignoran. Los textos no llevan
escapes con barra invertida.

Escribe "tienda: NOMBRE"; luego, por cada producto con stock mayor que 0, en el orden en que
aparecen, "nombre: precio x stock = valor", con precio y valor en euros con 2 decimales (valor =
precio × stock). Después "agotados: N" (productos con stock 0) y "valor total: T" (suma de los
valores, 2 decimales).
""", ['{"tienda": "Casa Ana", "productos": [{"nombre": "pan", "precio": 120, "stock": 10}, {"nombre": "leche", "precio": 95, "stock": 0}]}\n',
      '{"tienda":"Vacía","productos":[]}\n',
      '{\n  "productos": [\n    {"stock": 3, "nombre": "ñandú", "precio": 1999, "etiquetas": ["a", "b"]},\n    {"nombre": "x", "extra": {"k": 1}, "precio": 5, "stock": 1}\n  ],\n  "tienda": "Año Nuevo",\n  "version": 2\n}\n',
      '{"tienda": "T", "productos": [{"nombre": "a", "precio": 0, "stock": 5}, {"nombre": "b", "precio": 100000, "stock": 1000}]}\n',
      '{"tienda": "Solo agotados", "productos": [{"nombre": "p", "precio": 10, "stock": 0}, {"nombre": "q", "precio": 20, "stock": 0}]}\n'])
def _(lineas):
    import json
    d = json.loads("\n".join(lineas))
    out = [f"tienda: {d['tienda']}"]
    agot, total = 0, 0
    for p in d["productos"]:
        if p["stock"] > 0:
            v = p["precio"] * p["stock"]
            total += v
            out.append(f"{p['nombre']}: {euros(p['precio'])} x {p['stock']} = {euros(v)}")
        else:
            agot += 1
    out += [f"agotados: {agot}", f"valor total: {euros(total)}"]
    return out


@tarea("32-validar-formatos", """
# Validar formatos

Cada línea es "tipo valor" (el valor no tiene espacios). Los tipos y sus reglas:
- correo: exactamente una "@"; antes, uno o más caracteres de a-z, 0-9, ".", "_" o "-"; después,
  el dominio: partes separadas por puntos (al menos dos partes), cada una no vacía, con solo
  a-z, 0-9 o "-" y sin empezar ni acabar en "-"; la última parte tiene al menos 2 caracteres y
  solo letras a-z.
- telefono: exactamente 9 cifras y la primera es 6, 7, 8 o 9.
- fecha: "DD/MM/AAAA" (2 cifras, barra, 2 cifras, barra, 4 cifras), fecha que existe en el
  calendario gregoriano (son bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que
  no lo son de 400) y con año de 1900 a 2100.
- cp: código postal: exactamente 5 cifras y las dos primeras forman un número de 01 a 52.
Escribe por cada línea "tipo valor: valido" o "tipo valor: invalido" (la línea de entrada, dos
puntos y la palabra).
""", ["correo ana@ejemplo.es\ncorreo ana@@x.es\ntelefono 612345678\ncp 28001\nfecha 29/02/2024\n", "",
      "correo a@b.c\ncorreo a@b.co\ncorreo @b.co\ncorreo a@.co\ncorreo a@b..co\ncorreo a@-b.co\ncorreo a@b-.co\ncorreo a.b_c-d@x-y.z9.com\ncorreo A@b.co\ncorreo a@b.c0\ncorreo a@bco\ncorreo a@b@c.co\n",
      "telefono 512345678\ntelefono 91234567\ntelefono 9123456789\ntelefono 6123a5678\ntelefono 712345678\ntelefono 812345678\n",
      "fecha 29/02/1900\nfecha 29/02/2000\nfecha 31/04/2024\nfecha 00/01/2024\nfecha 01/13/2024\nfecha 1/1/2024\nfecha 01/01/1899\nfecha 31/12/2100\nfecha 01/01/2101\nfecha 01-01-2024\n",
      "cp 00999\ncp 01000\ncp 52999\ncp 53000\ncp 2800\ncp 280011\ncp 28a01\ncp 99999\n"])
def _(lineas):
    out = []
    for l in lineas:
        tipo, valor = l.split(" ")
        if tipo == "correo":
            ok = False
            if valor.count("@") == 1:
                loc, dom = valor.split("@")
                partes = dom.split(".")
                ok = (len(loc) > 0 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789._-" for c in loc)
                      and len(partes) >= 2
                      and all(p and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in p)
                              and p[0] != "-" and p[-1] != "-" for p in partes)
                      and len(partes[-1]) >= 2 and all(c in "abcdefghijklmnopqrstuvwxyz" for c in partes[-1]))
        elif tipo == "telefono":
            ok = len(valor) == 9 and all(c in "0123456789" for c in valor) and valor[0] in "6789"
        elif tipo == "fecha":
            ok = False
            if re.fullmatch(r"[0-9]{2}/[0-9]{2}/[0-9]{4}", valor):
                d, m, a = int(valor[0:2]), int(valor[3:5]), int(valor[6:10])
                try:
                    datetime.date(a, m, d)
                    ok = 1900 <= a <= 2100
                except ValueError:
                    ok = False
        else:
            ok = len(valor) == 5 and all(c in "0123456789" for c in valor) and 1 <= int(valor[:2]) <= 52
        out.append(f"{l}: {'valido' if ok else 'invalido'}")
    return out


@tarea("33-lineas-mal-formadas", """
# Líneas mal formadas

Cada línea debería ser "nombre;edad;ciudad". Se comprueba en este orden y la línea se descarta
por el primer fallo:
1. Si no tiene exactamente 3 campos (separados por ";"): "numero de campos incorrecto".
2. Si el nombre está vacío: "nombre vacio".
3. Si la edad no son solo cifras 0-9 (al menos una) o su valor pasa de 120: "edad invalida".
4. Si la ciudad está vacía: "ciudad vacia".
Una línea vacía cuenta como línea (y falla el punto 1). Por cada línea descartada escribe
"linea N: motivo" (N empieza en 1), en orden. Después escribe "validas: V" y "descartadas: D" y,
por último, por cada ciudad que aparezca en las líneas válidas, en orden alfabético,
"ciudad: cuantas" (cuántas líneas válidas tiene).
""", ["Ana;30;Madrid\nLuis;x;Sevilla\nBea;25;Madrid\n;40;Lima\nCarlos;50\n", "", "\n", "a;1;b;c\n",
      "u;007;Z\nv;120;Z\nw;121;Z\nx;-1;Z\ny; 5;Z\nz;;Z\n", "Ñu;3;Ávila\nÑu;4;Ávila\nÑu;5;Zaragoza\nÑu;6;Alcalá\n",
      "a;1;\n;;\n;\n;;;\na;1;c\n"])
def _(lineas):
    out, ciudades, validas = [], {}, 0
    for i, l in enumerate(lineas, 1):
        p = l.split(";")
        if len(p) != 3:
            m = "numero de campos incorrecto"
        elif p[0] == "":
            m = "nombre vacio"
        elif not re.fullmatch(r"[0-9]+", p[1]) or int(p[1]) > 120:
            m = "edad invalida"
        elif p[2] == "":
            m = "ciudad vacia"
        else:
            m = None
        if m:
            out.append(f"linea {i}: {m}")
        else:
            validas += 1
            ciudades[p[2]] = ciudades.get(p[2], 0) + 1
    out += [f"validas: {validas}", f"descartadas: {len(lineas) - validas}"]
    out += [f"{c}: {ciudades[c]}" for c in sorted(ciudades)]
    return out


@tarea("34-permutaciones", """
# Permutaciones en orden

La entrada es una línea con una palabra de 1 a 6 letras, todas distintas entre sí (pueden ser
minúsculas, mayúsculas, con acento o con ñ). Escribe todas las permutaciones de sus letras (cada
orden posible de las letras), una por línea, ordenadas alfabéticamente.
""", ["abc\n", "a\n", "cba\n", "ab\n", "añb\n", "bcde\n", "Ba\n", "dcbaz\n"])
def _(lineas):
    import itertools
    return sorted("".join(p) for p in itertools.permutations(lineas[0]))


@tarea("35-torres-hanoi", """
# Torres de Hanoi

La entrada es una línea con N, el número de discos (1 a 8). Los discos se numeran de 1 (el más
pequeño) a N (el más grande) y empiezan apilados en la varilla A; hay que llevarlos a la varilla C
usando la B, moviendo un disco cada vez y sin poner nunca uno grande sobre uno pequeño. Escribe la
solución con el menor número de movimientos (que es única), una línea por movimiento:
"mover disco D de X a Y". Al final escribe "total: T" con el número de movimientos.
""", ["2\n", "1\n", "3\n", "4\n", "5\n", "8\n"])
def _(lineas):
    out = []

    def h(n, a, c, b):
        if n == 0:
            return
        h(n - 1, a, b, c)
        out.append(f"mover disco {n} de {a} a {c}")
        h(n - 1, b, c, a)
    h(int(lineas[0]), "A", "C", "B")
    out.append(f"total: {len(out)}")
    return out


@tarea("36-inventario-informe", """
# Inventario con informe

Cada línea es una orden sobre un almacén vacío (CODIGO y NOMBRE son palabras sin espacios; los
precios están en céntimos):
- "alta CODIGO NOMBRE PRECIO STOCK": crea el producto. Si el código ya existe no cambia nada y
  escribe "error: codigo duplicado CODIGO".
- "venta CODIGO N": vende N unidades (N ≥ 1): resta N del stock y suma N × PRECIO a los ingresos
  de ese producto. Si el código no existe escribe "error: codigo desconocido CODIGO"; si no hay
  stock suficiente no cambia nada y escribe "error: stock insuficiente de CODIGO (hay H, piden N)".
- "reponer CODIGO N": suma N al stock; si el código no existe escribe "error: codigo desconocido CODIGO".
- "informe": escribe una línea por producto, en orden alfabético de código,
  "CODIGO NOMBRE stock=S ingresos=E" (E en euros con 2 decimales); luego "ingresos totales: E"
  y por último "sin stock: " seguido de los códigos con stock 0, en orden alfabético y separados
  por una coma y un espacio, o la palabra "ninguno" si no hay ninguno.
""", ["alta B2 tuerca 25 100\nalta A1 tornillo 10 5\nventa A1 3\nventa A1 9\nventa Z9 1\nreponer A1 10\ninforme\n",
      "informe\n", "alta a x 1 1\nalta a y 2 2\nventa a 1\ninforme\n", "",
      "alta c1 caja 1999 3\nventa c1 3\nalta c2 bolsa 5 0\ninforme\nreponer c2 2\nreponer c9 1\nventa c2 2\ninforme\n",
      "alta b ñu 100000 1000\nventa b 1000\nalta B Ñu 1 1\ninforme\n"])
def _(lineas):
    prod, out = {}, []
    for l in lineas:
        p = l.split(" ")
        if p[0] == "alta":
            if p[1] in prod:
                out.append(f"error: codigo duplicado {p[1]}")
            else:
                prod[p[1]] = {"nombre": p[2], "precio": int(p[3]), "stock": int(p[4]), "ing": 0}
        elif p[0] == "venta":
            n = int(p[2])
            if p[1] not in prod:
                out.append(f"error: codigo desconocido {p[1]}")
            elif prod[p[1]]["stock"] < n:
                out.append(f"error: stock insuficiente de {p[1]} (hay {prod[p[1]]['stock']}, piden {n})")
            else:
                prod[p[1]]["stock"] -= n
                prod[p[1]]["ing"] += n * prod[p[1]]["precio"]
        elif p[0] == "reponer":
            if p[1] not in prod:
                out.append(f"error: codigo desconocido {p[1]}")
            else:
                prod[p[1]]["stock"] += int(p[2])
        elif p[0] == "informe":
            for c in sorted(prod):
                d = prod[c]
                out.append(f"{c} {d['nombre']} stock={d['stock']} ingresos={euros(d['ing'])}")
            out.append(f"ingresos totales: {euros(sum(d['ing'] for d in prod.values()))}")
            sin = [c for c in sorted(prod) if prod[c]["stock"] == 0]
            out.append("sin stock: " + (", ".join(sin) if sin else "ninguno"))
    return out


@tarea("37-notas-alumnos", """
# Notas de alumnos con informe

Cada línea es "alumno;asignatura;nota". La nota va de 0 a 10 y es un entero ("7", "10") o tiene un
decimal ("7.5", "10.0"). Cada línea es una nota más (un alumno puede tener varias notas de la misma
asignatura). Las medias se calculan exactas y se redondean a 2 decimales; si quedan justo en la
mitad (por ejemplo 7.125), se redondea hacia arriba.

Escribe:
1. La línea "== alumnos ==" y, por cada alumno en orden alfabético, "alumno: media calificacion",
   con la media de todas sus notas y la calificación según esa media ya redondeada:
   menos de 5 "suspenso", de 5 a 6.99 "aprobado", de 7 a 8.99 "notable" y de 9 en adelante
   "sobresaliente".
2. La línea "== asignaturas ==" y, por cada asignatura en orden alfabético, "asignatura: media"
   con la media de todas sus notas (de todos los alumnos).
3. Una línea "mejor alumno: nombre" con el de mayor media redondeada (si empatan, el primero en
   orden alfabético), o "mejor alumno: nadie" si no hay notas.
""", ["Ana;Mates;8.5\nBea;Mates;4\nAna;Lengua;7\nBea;Lengua;5.5\nCarlos;Mates;10\n", "",
      "Z;a;7\nZ;a;7\nZ;a;7.1\n", "Ñu;x;4.9\nÁlex;x;5\nZoe;y;9\nZoe;y;8.9\n", "a;m;0\na;m;0\na;n;0.1\n",
      "p;q;7.5\np;q;7\np;q;7\np;q;7\np;q;7\np;q;7\np;q;7\np;q;7.0\n", "u;m;10\nv;m;10.0\nw;m;9\n"])
def _(lineas):
    def media(suma, n):
        return (suma * 2 + n) // (2 * n)

    def fmt(h):
        return f"{h // 100}.{h % 100:02d}"
    por_alumno, por_asig = {}, {}
    for l in lineas:
        al, asig, nota = l.split(";")
        t = int(nota.split(".")[0]) * 10 + (int(nota.split(".")[1]) if "." in nota else 0)
        por_alumno.setdefault(al, []).append(t)
        por_asig.setdefault(asig, []).append(t)
    out, medias = ["== alumnos =="], {}
    for al in sorted(por_alumno):
        h = media(sum(por_alumno[al]) * 10, len(por_alumno[al]))
        medias[al] = h
        cal = "suspenso" if h < 500 else "aprobado" if h < 700 else "notable" if h < 900 else "sobresaliente"
        out.append(f"{al}: {fmt(h)} {cal}")
    out.append("== asignaturas ==")
    for asig in sorted(por_asig):
        out.append(f"{asig}: {fmt(media(sum(por_asig[asig]) * 10, len(por_asig[asig])))}")
    if medias:
        mejor = sorted(medias, key=lambda a: (-medias[a], a))[0]
        out.append(f"mejor alumno: {mejor}")
    else:
        out.append("mejor alumno: nadie")
    return out


@tarea("38-carrito-descuentos", """
# Carrito con descuentos

Cada línea es "producto cantidad precio": producto es una palabra sin espacios, cantidad un entero
≥ 1 y precio el precio unitario en euros con como mucho 2 decimales (por ejemplo "12.5" o "3").
Cada línea es una línea de compra independiente (aunque el producto se repita). Todo se calcula en
céntimos y cada redondeo es al céntimo más cercano, hacia arriba si queda justo en la mitad.

1. Línea de compra: bruto = cantidad × precio. Descuento por cantidad: 10 % si la cantidad es 10 o
   más, 5 % si es 5 o más (y menos de 10), 0 % si no. Importe de la línea = bruto menos ese
   descuento, redondeado. Escribe "producto xCANTIDAD: importe" (2 decimales) y, si el descuento
   no es 0, añade al final " (-D%)" con D = 5 o 10.
2. "subtotal: S": suma de los importes de las líneas.
3. Si S es 100.00 o más, descuento del pedido del 5 %: escribe "descuento pedido: -X" (X = S menos
   S × 95 / 100 redondeado) y el nuevo total es S × 95 / 100 redondeado. Si no, no se escribe esa
   línea y el total es S.
4. "envio: E": 0.00 si el total ya es de 50.00 o más, y 4.95 si no.
5. "total: T" = total más envío.
Todas las cantidades de dinero se escriben con exactamente 2 decimales. Con la entrada vacía, el
subtotal es 0.00, el envío 4.95 y el total 4.95.
""", ["pan 6 1.20\nleche 2 0.95\nqueso 1 12.5\n", "", "a 10 100\n", "a 4 1\na 5 1\na 9 1\na 10 1\n",
      "x 1 33.33\ny 3 3.33\n", "b 1 49.99\n", "b 1 50\n", "z 1 99.99\nz 1 0.01\nz 1 0.01\n", "w 5 0.01\nw 10 0.05\nw 7 0.03\n"])
def _(lineas):
    out, sub = [], 0
    for l in lineas:
        p, q, pr = l.split(" ")
        q = int(q)
        d = 10 if q >= 10 else 5 if q >= 5 else 0
        imp = (q * a_centimos(pr) * (100 - d) + 50) // 100
        sub += imp
        out.append(f"{p} x{q}: {euros(imp)}" + (f" (-{d}%)" if d else ""))
    out.append(f"subtotal: {euros(sub)}")
    total = sub
    if sub >= 10000:
        total = (sub * 95 + 50) // 100
        out.append(f"descuento pedido: -{euros(sub - total)}")
    envio = 0 if total >= 5000 else 495
    out.append(f"envio: {euros(envio)}")
    out.append(f"total: {euros(total + envio)}")
    return out


@tarea("39-compresion-rle", """
# Compresión por repeticiones

Cada línea es "C texto" (comprimir) o "D texto" (descomprimir): una letra, un espacio y un texto no
vacío.
- C: el texto está formado por letras (a-z, A-Z, á é í ó ú ü ñ y mayúsculas, sin cifras ni
  espacios). Se escribe cada tanda de letras iguales seguidas como su número de repeticiones y la
  letra, todo pegado: "aaabccdd" → "3a1b2c2d". Mayúsculas y minúsculas son letras distintas.
- D: el texto es una compresión válida (una o más veces: número entero ≥ 1 sin ceros a la
  izquierda, y una letra); se escribe el texto original, por ejemplo "3a1b" → "aaab".
""", ["C aaabccdd\nD 3a1b2c2d\n", "C a\n", "D 1a\n", "C aAaA\nC zzzzzzzzzzzz\nD 12z\n", "C ñññáá\nD 2ñ3á1Ñ\n",
      "D 10a2b\nC abababab\n", "", "C Hoolaa\nD 1H2o1l1a\n"])
def _(lineas):
    out = []
    for l in lineas:
        modo, texto = l[0], l[2:]
        if modo == "C":
            r, i = "", 0
            while i < len(texto):
                j = i
                while j < len(texto) and texto[j] == texto[i]:
                    j += 1
                r += f"{j - i}{texto[i]}"
                i = j
            out.append(r)
        else:
            out.append("".join(letra * int(n) for n, letra in re.findall(r"([0-9]+)(\D)", texto)))
    return out


@tarea("40-anagramas", """
# Grupos de anagramas

Cada línea es una palabra en minúsculas (letras a-z, á é í ó ú ü ñ; puede repetirse). Dos palabras
son anagramas si tienen las mismas letras en cualquier orden (y las mismas veces cada una). Ignora
las palabras repetidas (cuenta cada palabra una vez). Escribe los grupos de 2 o más palabras
distintas que son anagramas entre sí: una línea por grupo, con sus palabras en orden alfabético
separadas por un espacio, y los grupos ordenados por su primera palabra (alfabéticamente). Si no
hay ningún grupo, escribe "ninguno".
""", ["roma\namor\nramo\nmora\ngato\ntago\nperro\n", "", "a\n", "ab\nba\nab\nba\n", "hola\nmundo\n",
      "niño\nniñó\nnoñi\nión\nnoi\nión\n", "listen\nsilent\nenlist\ninlets\ntinsel\nzzz\nzz\n",
      "sol\nlos\nslo\nñu\nuñ\nuñ\nabc\ncba\nbac\ncab\n"])
def _(lineas):
    grupos = {}
    for p in sorted(set(lineas)):
        grupos.setdefault("".join(sorted(p)), []).append(p)
    gs = sorted(" ".join(g) for g in grupos.values() if len(g) >= 2)
    gs.sort(key=lambda s: s.split(" ")[0])
    return gs or ["ninguno"]


# ---------------------------------------------------------------- generación

def texto_enunciado(nombre, enunciado, casos, funcion):
    ent = casos[0]
    sal = "".join(l + "\n" for l in funcion(ent.split("\n")[:-1]))
    ejemplo = "\n\n## Ejemplo\n\nEntrada:\n\n```\n" + ent + "```\n\nSalida:\n\n```\n" + sal + "```\n"
    if ent == "":
        ejemplo = ejemplo.replace("```\n```", "```\n(vacía)\n```")
    if sal == "":
        ejemplo = ejemplo.replace("```\n```", "```\n(sin salida)\n```")
    return enunciado.rstrip("\n") + ejemplo + COMUN


def generar(raiz):
    for nombre, enunciado, casos, funcion in TAREAS:
        carpeta = os.path.join(raiz, nombre)
        shutil.rmtree(carpeta, ignore_errors=True)
        os.makedirs(os.path.join(carpeta, "casos"))
        with open(os.path.join(carpeta, "enunciado.md"), "w") as f:
            f.write(texto_enunciado(nombre, enunciado, casos, funcion))
        for k, entrada in enumerate(casos, 1):
            with open(os.path.join(carpeta, "casos", f"{k}.entrada"), "w") as f:
                f.write(entrada)
            lineas = entrada.split("\n")[:-1]
            with open(os.path.join(carpeta, "casos", f"{k}.esperado"), "w") as f:
                f.write("".join(l + "\n" for l in funcion(lineas)))


def main():
    raiz = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tareas")
    generar(raiz)
    print(f"{len(TAREAS)} tareas en {os.path.normpath(raiz)}")


if __name__ == "__main__":
    main()
