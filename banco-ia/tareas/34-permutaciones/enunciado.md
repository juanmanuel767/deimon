# Permutaciones en orden

La entrada es una línea con una palabra de 1 a 6 letras, todas distintas entre sí (pueden ser
minúsculas, mayúsculas, con acento o con ñ). Escribe todas las permutaciones de sus letras (cada
orden posible de las letras), una por línea, ordenadas alfabéticamente.

## Ejemplo

Entrada:

```
abc
```

Salida:

```
abc
acb
bac
bca
cab
cba
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
