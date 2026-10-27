# Funciones de texto al estilo PHP

Cada línea tiene la forma `operacion;argumento;texto`: separa solo por los dos primeros «;», así
que el texto es todo lo que viene después (puede llevar más «;», o estar vacío). Aplica la
operación y escribe el resultado entre corchetes `[...]`; si el resultado ocupa varias líneas,
escribe cada línea entre sus propios corchetes. Operaciones:

- `ucfirst` (argumento vacío): como `ucfirst` de PHP: la primera letra pasa a mayúscula **solo si
  es una letra a-z sin acento**; si es cualquier otro carácter (incluidas ñ, á, etc.) el texto no cambia.
- `ucwords` (argumento vacío): como `ucfirst` pero para cada palabra: una palabra empieza al
  principio del texto o justo después de un espacio; solo se pasan a mayúscula las letras a-z.
  El resto del texto (otras letras, mayúsculas ya escritas) no cambia.
- `wordwrap` (argumento: ancho W, entero ≥ 1): como `wordwrap($texto, W, "\n", false)` de PHP,
  para textos solo con letras, cifras y signos ASCII y palabras separadas por un único espacio
  (sin espacios al principio ni al final). Se van poniendo las palabras en la línea actual
  mientras la línea resultante no pase de W caracteres; si la siguiente palabra no cabe, empieza
  una línea nueva. Una palabra más larga que W no se corta: queda sola en su línea. Un texto
  vacío da una sola línea vacía.
- `str_pad` (argumento: `LARGO,LADO,RELLENO`; LARGO entero, LADO `izq`, `der` o `centro`,
  RELLENO uno o más caracteres ASCII sin comas ni «;»): como `str_pad` de PHP para textos ASCII.
  Si el texto ya tiene LARGO caracteres o más, no cambia. Si no, faltan F = LARGO − largo del
  texto caracteres: `izq` pone F caracteres de relleno delante, `der` detrás, y `centro` pone
  F ÷ 2 (redondeado hacia abajo) delante y el resto detrás. El relleno se repite desde su primer
  carácter tantas veces como haga falta y se corta al llegar a los caracteres necesarios (cada
  lado empieza otra vez por el primer carácter del relleno).

## Ejemplo

Entrada:

```
ucfirst;;hola mundo
ucwords;;hola mundo feliz
wordwrap;10;el veloz zorro salta la valla
str_pad;8,izq,*;abc
```

Salida:

```
[Hola mundo]
[Hola Mundo Feliz]
[el veloz]
[zorro]
[salta la]
[valla]
[*****abc]
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
