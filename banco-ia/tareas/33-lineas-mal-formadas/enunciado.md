# Líneas mal formadas

Cada línea debería ser "nombre;edad;ciudad". Se comprueba en este orden y la línea se descarta
por el primer fallo:
1. Si no tiene exactamente 3 campos (separados por ";"): "numero de campos incorrecto".
2. Si el nombre está vacío: "nombre vacio".
3. Si la edad no son solo cifras 0-9 (al menos una) o su valor pasa de 120: "edad invalida".
4. Si la ciudad está vacía: "ciudad vacia".
Una línea vacía cuenta como línea (y falla el punto 1). Por cada línea descartada escribe
"linea N: motivo" (N empieza en 1), en orden. Después escribe "validas: V" y "descartadas: D" y,
por último, por cada ciudad que aparezca en las líneas válidas, en orden alfabético,
"ciudad: cuantas" (cuántas líneas válidas tiene).

## Ejemplo

Entrada:

```
Ana;30;Madrid
Luis;x;Sevilla
Bea;25;Madrid
;40;Lima
Carlos;50
```

Salida:

```
linea 2: edad invalida
linea 4: nombre vacio
linea 5: numero de campos incorrecto
validas: 2
descartadas: 3
Madrid: 2
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
