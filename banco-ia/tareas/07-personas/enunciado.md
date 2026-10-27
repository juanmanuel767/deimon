# Ordenar personas

Cada línea es "nombre,edad" (edad entera, no negativa). Escribe las personas ordenadas por edad de
mayor a menor; si tienen la misma edad, por nombre en orden alfabético. Formato de cada línea:
"nombre (edad)". Después, una línea "media: x" con la media de las edades con 1 decimal (si queda
justo en la mitad, se redondea hacia arriba: 28.25 → 28.3). La entrada tiene al menos una persona.

## Ejemplo

Entrada:

```
Lucia,34
Pedro,28
Ana,34
Zoe,19
Bruno,28
```

Salida:

```
Ana (34)
Lucia (34)
Bruno (28)
Pedro (28)
Zoe (19)
media: 28.6
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
