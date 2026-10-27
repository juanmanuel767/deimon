# Palabras más frecuentes

La entrada es un texto en varias líneas. Las palabras son trozos separados por espacios (puede
haber varios espacios seguidos); antes de contarlas pásalas a minúsculas (también las letras con
acento y la ñ) y quítales los signos . , ; : ! ? del principio y del final (si se queda vacía, no
cuenta; los demás signos, como ¿ o ¡, no se quitan). Escribe las 3 palabras más repetidas, una por
línea, con el formato "palabra: veces", de más a menos veces; si empatan, en orden alfabético.
Si hay menos de 3 palabras distintas, escribe solo las que haya.

## Ejemplo

Entrada:

```
El perro y el gato.
El gato duerme; el PERRO no.
Y el gato, ¿qué?
```

Salida:

```
el: 5
gato: 3
perro: 2
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
