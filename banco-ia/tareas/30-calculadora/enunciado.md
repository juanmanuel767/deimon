# Calculadora con precedencia

Cada línea es una expresión con enteros sin signo (hasta 9 cifras), los operadores + - * /,
paréntesis y espacios en cualquier sitio entre símbolos. Está bien escrita. Reglas: * y / se
calculan antes que + y -; a igual prioridad, de izquierda a derecha; los paréntesis mandan; el
signo menos también puede ir delante de un número o de un paréntesis (por ejemplo "-3", "2 * -3",
"-(1+2)", "--4"), y se aplica antes que cualquier operador binario de esa expresión simple.
La división es entera redondeando hacia cero (7 / 2 = 3 y -7 / 2 = -3). Los resultados
intermedios nunca pasan de 10^15 en valor absoluto. Escribe el valor de cada expresión; si se
divide entre cero, escribe "error: division por cero".

## Ejemplo

Entrada:

```
2 + 3 * 4
(2 + 3) * 4
10 / 3
-7 / 2
```

Salida:

```
14
20
3
-3
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
