# Contar con criterios (tipo Counter)

La primera línea es un entero N ≥ 1. Las demás forman un texto: las palabras son los trozos
separados por espacios (uno o varios seguidos; sin quitar signos ni cambiar mayúsculas: «Hola» y
«hola» son palabras distintas). Escribe, en este orden:

1. las N palabras más repetidas, una por línea, `palabra: veces`, de más a menos veces; **si dos
   palabras empatan, va primero la que apareció antes en el texto** (como `Counter.most_common`
   de Python). Si hay menos de N palabras distintas, salen todas;
2. una línea `distintas: K` con el número de palabras distintas;
3. por cada longitud de palabra (en caracteres) que exista, de menor a mayor, una línea
   `longitud L: M` con M = cuántas palabras **en total** (contando repeticiones) tienen esa longitud.

## Ejemplo

Entrada:

```
2
el gato y el perro y el pez
el gato
```

Salida:

```
el: 4
gato: 2
distintas: 5
longitud 1: 2
longitud 2: 4
longitud 3: 1
longitud 4: 2
longitud 5: 1
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
