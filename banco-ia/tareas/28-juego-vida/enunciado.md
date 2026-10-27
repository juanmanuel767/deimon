# Juego de la vida

La primera línea es "F C G": filas, columnas (de 1 a 10) y número de generaciones (de 0 a 20).
Siguen F líneas de C caracteres: "#" célula viva y "." muerta. En cada generación, todas las
células cambian a la vez: una célula viva con 2 o 3 vecinas vivas sigue viva, una muerta con
exactamente 3 vecinas vivas nace, y las demás quedan o pasan a muertas. Las vecinas son las 8
casillas que rodean a la célula; fuera de la cuadrícula todo está muerto (no hay vuelta
alrededor de los bordes). Escribe la cuadrícula tras G generaciones, en F líneas.

## Ejemplo

Entrada:

```
5 5 2
.....
..#..
..#..
..#..
.....
```

Salida:

```
.....
..#..
..#..
..#..
.....
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
