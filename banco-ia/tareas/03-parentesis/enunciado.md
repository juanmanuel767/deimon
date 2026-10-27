# Paréntesis equilibrados

Cada línea es un texto con (), [] y {} mezclados con otras letras. Para cada línea escribe
"si" si todos los símbolos de apertura se cierran en el orden correcto, y "no" si no.
Las demás letras se ignoran. Una línea sin símbolos (también una línea vacía) está equilibrada.

## Ejemplo

Entrada:

```
(a[b]{c})
([)]
((
hola
{[()()]}
)(
```

Salida:

```
si
no
no
si
si
no
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
