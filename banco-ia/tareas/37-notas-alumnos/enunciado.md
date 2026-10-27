# Notas de alumnos con informe

Cada línea es "alumno;asignatura;nota". La nota va de 0 a 10 y es un entero ("7", "10") o tiene un
decimal ("7.5", "10.0"). Cada línea es una nota más (un alumno puede tener varias notas de la misma
asignatura). Las medias se calculan exactas y se redondean a 2 decimales; si quedan justo en la
mitad (por ejemplo 7.125), se redondea hacia arriba.

Escribe:
1. La línea "== alumnos ==" y, por cada alumno en orden alfabético, "alumno: media calificacion",
   con la media de todas sus notas y la calificación según esa media ya redondeada:
   menos de 5 "suspenso", de 5 a 6.99 "aprobado", de 7 a 8.99 "notable" y de 9 en adelante
   "sobresaliente".
2. La línea "== asignaturas ==" y, por cada asignatura en orden alfabético, "asignatura: media"
   con la media de todas sus notas (de todos los alumnos).
3. Una línea "mejor alumno: nombre" con el de mayor media redondeada (si empatan, el primero en
   orden alfabético), o "mejor alumno: nadie" si no hay notas.

## Ejemplo

Entrada:

```
Ana;Mates;8.5
Bea;Mates;4
Ana;Lengua;7
Bea;Lengua;5.5
Carlos;Mates;10
```

Salida:

```
== alumnos ==
Ana: 7.75 notable
Bea: 4.75 suspenso
Carlos: 10.00 sobresaliente
== asignaturas ==
Lengua: 6.25
Mates: 7.50
mejor alumno: Carlos
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
