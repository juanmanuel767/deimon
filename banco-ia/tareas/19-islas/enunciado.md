# Contar islas

La primera línea es "F C" (filas y columnas, de 1 a 12). Siguen F líneas de C caracteres:
"#" tierra y "." agua. Una isla es un grupo de casillas de tierra conectadas por arriba, abajo,
izquierda o derecha (no en diagonal). Escribe dos líneas: "islas: N" (cuántas hay) y
"mayor: M" (cuántas casillas tiene la isla más grande, 0 si no hay ninguna).

## Ejemplo

Entrada:

```
4 5
##...
#..#.
...##
.#...
```

Salida:

```
islas: 3
mayor: 3
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
