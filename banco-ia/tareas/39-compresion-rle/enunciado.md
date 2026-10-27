# Compresión por repeticiones

Cada línea es "C texto" (comprimir) o "D texto" (descomprimir): una letra, un espacio y un texto no
vacío.
- C: el texto está formado por letras (a-z, A-Z, á é í ó ú ü ñ y mayúsculas, sin cifras ni
  espacios). Se escribe cada tanda de letras iguales seguidas como su número de repeticiones y la
  letra, todo pegado: "aaabccdd" → "3a1b2c2d". Mayúsculas y minúsculas son letras distintas.
- D: el texto es una compresión válida (una o más veces: número entero ≥ 1 sin ceros a la
  izquierda, y una letra); se escribe el texto original, por ejemplo "3a1b" → "aaab".

## Ejemplo

Entrada:

```
C aaabccdd
D 3a1b2c2d
```

Salida:

```
3a1b2c2d
aaabccdd
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
