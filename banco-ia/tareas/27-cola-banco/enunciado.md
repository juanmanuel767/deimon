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

## Ejemplo

Entrada:

```
2
0 5 ana
1 3 bea
2 4 carlos
3 1 dora
```

Salida:

```
ana: caja 1 empieza 0 espera 0
bea: caja 2 empieza 1 espera 0
carlos: caja 2 empieza 4 espera 2
dora: caja 1 empieza 5 espera 2
espera total: 4
termina: 8
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
