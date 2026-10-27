# Grupos de anagramas

Cada línea es una palabra en minúsculas (letras a-z, á é í ó ú ü ñ; puede repetirse). Dos palabras
son anagramas si tienen las mismas letras en cualquier orden (y las mismas veces cada una). Ignora
las palabras repetidas (cuenta cada palabra una vez). Escribe los grupos de 2 o más palabras
distintas que son anagramas entre sí: una línea por grupo, con sus palabras en orden alfabético
separadas por un espacio, y los grupos ordenados por su primera palabra (alfabéticamente). Si no
hay ningún grupo, escribe "ninguno".

## Ejemplo

Entrada:

```
roma
amor
ramo
mora
gato
tago
perro
```

Salida:

```
amor mora ramo roma
gato tago
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
