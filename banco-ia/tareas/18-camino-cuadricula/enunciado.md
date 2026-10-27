# Camino más corto en una cuadrícula

La primera línea es "F C" (filas y columnas, de 1 a 12). Siguen F líneas de C caracteres:
"." casilla libre, "#" muro, "S" salida y "E" entrada (hay exactamente una de cada). Moviéndote
una casilla cada vez hacia arriba, abajo, izquierda o derecha (no en diagonal), sin pisar muros
ni salir de la cuadrícula, escribe el número mínimo de movimientos para ir de S a E. Si no se
puede llegar, escribe "sin camino".

## Ejemplo

Entrada:

```
3 4
S..#
.#..
...E
```

Salida:

```
5
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
