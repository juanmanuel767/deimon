# Formato de números

Cada línea tiene un número decimal N y un número de decimales D (de 0 a 4) separados por «;»,
por ejemplo `1234567.891;2`. N se escribe con cifras, un «-» opcional delante y, si tiene parte
decimal, un punto (nunca notación científica); su valor absoluto es menor que 1.000.000.000 y
nunca es un número negativo que al redondear dé cero. Escribe una línea con tres formatos
separados por « | » (espacio, barra, espacio):

1. como `number_format(N, D, ',', '.')` de PHP: D decimales con coma y miles con punto;
2. como `number_format(N, D, '.', ',')` de PHP: D decimales con punto y miles con coma;
3. como `sprintf("%010.Df", round(N, D))` de PHP: D decimales con punto, sin separador de miles, rellenado
   con ceros a la izquierda hasta 10 caracteres (el signo «-», si lo hay, va delante de los ceros;
   si ya tiene 10 o más caracteres no se rellena).

El redondeo es el decimal habitual: se redondea al valor más cercano con D decimales y, si queda
justo a la mitad, se aleja del cero (2.5 → 3, -2.5 → -3, 1.005 con 2 decimales → 1.01), calculado
sobre el número tal como está escrito (no sobre su aproximación en coma flotante).

## Ejemplo

Entrada:

```
1234567.891;2
0.5;0
-1234.5;1
7;3
```

Salida:

```
1.234.567,89 | 1,234,567.89 | 1234567.89
1 | 1 | 0000000001
-1.234,5 | -1,234.5 | -0001234.5
7,000 | 7.000 | 000007.000
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
