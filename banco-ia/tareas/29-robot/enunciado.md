# Robot en una cuadrícula

Cada línea (nunca vacía) es una secuencia de órdenes con las letras A, G e I, para un robot que
empieza en la posición (0, 0) mirando al norte. "A" avanza una casilla en la dirección en que mira,
"G" gira 90° a la derecha sin moverse e "I" gira 90° a la izquierda. El norte es y creciente,
el este x creciente, el sur y decreciente y el oeste x decreciente (mirando al norte, girar a la
derecha lleva al este). Cada línea es un robot nuevo, que parte otra vez de (0, 0) mirando al norte.

Para cada línea escribe "x,y direccion max=D": la posición final, la dirección final (norte, este,
sur u oeste) y D, el mayor valor de x×x + y×y entre todas las posiciones por las que pasó (la
inicial incluida).

## Ejemplo

Entrada:

```
AAGAAIA
GGA
```

Salida:

```
2,3 norte max=13
0,-1 sur max=1
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
