# Torres de Hanoi

La entrada es una línea con N, el número de discos (1 a 8). Los discos se numeran de 1 (el más
pequeño) a N (el más grande) y empiezan apilados en la varilla A; hay que llevarlos a la varilla C
usando la B, moviendo un disco cada vez y sin poner nunca uno grande sobre uno pequeño. Escribe la
solución con el menor número de movimientos (que es única), una línea por movimiento:
"mover disco D de X a Y". Al final escribe "total: T" con el número de movimientos.

## Ejemplo

Entrada:

```
2
```

Salida:

```
mover disco 1 de A a B
mover disco 2 de A a C
mover disco 1 de B a C
total: 3
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
