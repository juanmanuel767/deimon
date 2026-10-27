# Justificar texto

La primera línea es un entero ANCHO. El resto de líneas es un texto: sus palabras (trozos
separados por uno o más espacios, de todas las líneas, en orden) se reparten en líneas de como
mucho ANCHO caracteres: en cada línea se ponen todas las palabras que quepan (separadas por un
espacio como mínimo) antes de pasar a la siguiente. Ninguna palabra es más larga que ANCHO.

Luego se justifica: en cada línea, salvo la última, que tenga 2 o más palabras, se reparten
espacios entre las palabras hasta que la línea mida exactamente ANCHO caracteres; todos los
huecos reciben los mismos espacios y, si sobran, un espacio más cada uno de los primeros huecos
(empezando por la izquierda). Una línea con una sola palabra y la última línea se dejan con las
palabras separadas por un solo espacio. Cuenta caracteres (á o ñ cuentan 1). Escribe las líneas
resultantes; si no hay palabras, no escribes nada.

## Ejemplo

Entrada:

```
20
El veloz murciélago hindú comía feliz cardillo y kiwi.
```

Salida:

```
El  veloz murciélago
hindú   comía  feliz
cardillo y kiwi.
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
