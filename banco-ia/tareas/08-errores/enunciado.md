# Números con errores

Cada línea debería ser un número entero: un signo menos opcional seguido de una o más cifras 0-9,
sin espacios ni otros símbolos (por ejemplo "-5" y "007" valen; "+5", " 5", "3.5", "1e3", "-" y
la línea vacía no). Suma los que lo sean. Para cada línea que no sea un entero válido escribe
"linea N: no es un numero: TEXTO" (N empieza en 1 y TEXTO es la línea tal cual). Al final
escribe "suma: S".

## Ejemplo

Entrada:

```
10
veinte
-5
3.5
1e3
7
```

Salida:

```
linea 2: no es un numero: veinte
linea 4: no es un numero: 3.5
linea 5: no es un numero: 1e3
suma: 12
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
