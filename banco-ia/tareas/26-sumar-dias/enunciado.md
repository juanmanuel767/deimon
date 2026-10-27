# Sumar días a una fecha

Cada línea es "AAAA-MM-DD N": una fecha válida (años de 1600 a 2400, calendario gregoriano: son
bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que no lo son de 400) y un entero
N entre -100000 y 100000 (puede ser negativo). Escribe la fecha que resulta de sumarle N días,
también como "AAAA-MM-DD" (año con 4 cifras, mes y día con 2). El resultado siempre cae entre
los años 1000 y 9999.

## Ejemplo

Entrada:

```
2024-01-31 1
2024-02-28 2
2023-12-31 1
```

Salida:

```
2024-02-01
2024-03-01
2024-01-01
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
