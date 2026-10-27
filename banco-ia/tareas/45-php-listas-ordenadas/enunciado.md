# Listas con claves ordenadas

Cada línea es `clave=valor`: la clave es un texto sin «=» y no vacío (letras, cifras, ñ, acentos o
guiones) y el valor un entero (puede ser negativo). Se construye una lista con claves como un
array de PHP: si una clave se repite, **conserva la posición de su primera aparición** y toma el
**último** valor. Escribe cuatro bloques, cada uno con su cabecera y después una línea
`clave: valor` por elemento:

```
== asort ==    valor de menor a mayor; los empates, en el orden de primera aparición
== arsort ==   valor de mayor a menor; los empates, en el orden de primera aparición
== ksort ==    clave de menor a mayor, comparando las claves como textos por el código de sus caracteres
== krsort ==   clave de mayor a menor, igual (el orden inverso exacto de ksort)
```

(Las claves se comparan siempre como texto, también las que parecen números: «10» va antes que «9».)
Sin entrada, salen solo las cuatro cabeceras.

## Ejemplo

Entrada:

```
pera=3
manzana=5
uva=3
kiwi=-1
pera=7
```

Salida:

```
== asort ==
kiwi: -1
uva: 3
manzana: 5
pera: 7
== arsort ==
pera: 7
manzana: 5
uva: 3
kiwi: -1
== ksort ==
kiwi: -1
manzana: 5
pera: 7
uva: 3
== krsort ==
uva: 3
pera: 7
manzana: 5
kiwi: -1
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
