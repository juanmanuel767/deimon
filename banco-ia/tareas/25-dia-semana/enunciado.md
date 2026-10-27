# Día de la semana

Cada línea es una fecha "AAAA-MM-DD" (siempre con 4 cifras, guion, 2 cifras, guion y 2 cifras; el
año va de 1600 a 2400, calendario gregoriano: son bisiestos los años múltiplos de 4, salvo los
múltiplos de 100 que no lo son de 400). Si la fecha existe, escribe "FECHA: dia", con el día de
la semana en minúsculas y sin acentos: lunes, martes, miercoles, jueves, viernes, sabado o
domingo. Si no existe (mes 00 o mayor de 12, día 00 o más del que tiene ese mes), escribe
"FECHA: invalida". FECHA es la línea tal cual.

## Ejemplo

Entrada:

```
2024-02-29
2023-02-29
2000-01-01
```

Salida:

```
2024-02-29: jueves
2023-02-29: invalida
2000-01-01: sabado
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
