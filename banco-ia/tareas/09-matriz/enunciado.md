# Matriz en espiral

La primera línea tiene dos enteros "F C" (filas y columnas, de 1 a 10). Siguen F líneas con C
enteros separados por un espacio. Escribe en una sola línea, separados por un espacio, los
números recorridos en espiral: primera fila de izquierda a derecha, después hacia abajo por la
última columna, hacia la izquierda por la última fila, hacia arriba, y así hacia dentro.

## Ejemplo

Entrada:

```
3 4
1 2 3 4
5 6 7 8
9 10 11 12
```

Salida:

```
1 2 3 4 8 12 11 10 9 5 6 7
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
