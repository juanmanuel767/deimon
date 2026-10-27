# Números romanos

Cada línea tiene un entero entre 1 y 3999. Escribe su número romano (I, V, X, L, C, D, M, con
las restas IV, IX, XL, XC, CD y CM).

## Ejemplo

Entrada:

```
1
4
9
14
40
90
400
1994
2024
3999
```

Salida:

```
I
IV
IX
XIV
XL
XC
CD
MCMXCIV
MMXXIV
MMMCMXCIX
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
