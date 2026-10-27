# Formato de fechas

Cada línea es una fecha y hora válida, sin zona horaria (todo en el mismo horario), con la forma
`AAAA-MM-DD HH:MM:SS` y año entre 1970 y 2100. Escribe una línea con tres formatos separados por
« | » (espacio, barra, espacio):

1. `DIA J de MES de AAAA, G:MM am|pm`: nombre del día de la semana y del mes **en español y en
   minúsculas** (lunes, martes, miércoles, jueves, viernes, sábado, domingo; enero, febrero...
   diciembre), J el día del mes sin cero delante, el año con 4 cifras, G la hora en formato de 12
   horas sin cero delante (la medianoche es 12 am y el mediodía 12 pm), MM los minutos con dos
   cifras, y «am» o «pm» en minúsculas;
2. como `date('d/m/y')` de PHP: día y mes con dos cifras y año de dos cifras;
3. `dia N`, donde N es el día del año (1 el 1 de enero; 366 el 31 de diciembre de un año bisiesto).

## Ejemplo

Entrada:

```
1970-01-01 00:00:00
2024-02-29 13:05:09
2000-12-31 23:59:59
```

Salida:

```
jueves 1 de enero de 1970, 12:00 am | 01/01/70 | dia 1
jueves 29 de febrero de 2024, 1:05 pm | 29/02/24 | dia 60
domingo 31 de diciembre de 2000, 11:59 pm | 31/12/00 | dia 366
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
