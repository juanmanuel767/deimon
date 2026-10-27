# Contar palabras con acentos

La entrada es un texto en varias líneas. Una palabra es una secuencia máxima de letras, y las
letras son solo: a-z, A-Z, á é í ó ú ü ñ y sus mayúsculas Á É Í Ó Ú Ü Ñ. Cualquier otro carácter
(espacios, cifras, signos, ¿ ¡, guiones...) separa palabras. Pasa las palabras a minúsculas.

Escribe "total: N" (palabras en total, con repeticiones), "distintas: M" y luego las 5 palabras
más repetidas, una por línea, con el formato "palabra: veces", de más a menos veces y, si
empatan, en orden alfabético. Si hay menos de 5 palabras distintas, escribe solo las que haya.

## Ejemplo

Entrada:

```
¿Qué hace el niño? El niño come; la niña no come.
```

Salida:

```
total: 11
distintas: 8
come: 2
el: 2
niño: 2
hace: 1
la: 1
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
