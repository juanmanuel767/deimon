# Validar formatos

Cada línea es "tipo valor" (el valor no tiene espacios). Los tipos y sus reglas:
- correo: exactamente una "@"; antes, uno o más caracteres de a-z, 0-9, ".", "_" o "-"; después,
  el dominio: partes separadas por puntos (al menos dos partes), cada una no vacía, con solo
  a-z, 0-9 o "-" y sin empezar ni acabar en "-"; la última parte tiene al menos 2 caracteres y
  solo letras a-z.
- telefono: exactamente 9 cifras y la primera es 6, 7, 8 o 9.
- fecha: "DD/MM/AAAA" (2 cifras, barra, 2 cifras, barra, 4 cifras), fecha que existe en el
  calendario gregoriano (son bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que
  no lo son de 400) y con año de 1900 a 2100.
- cp: código postal: exactamente 5 cifras y las dos primeras forman un número de 01 a 52.
Escribe por cada línea "tipo valor: valido" o "tipo valor: invalido" (la línea de entrada, dos
puntos y la palabra).

## Ejemplo

Entrada:

```
correo ana@ejemplo.es
correo ana@@x.es
telefono 612345678
cp 28001
fecha 29/02/2024
```

Salida:

```
correo ana@ejemplo.es: valido
correo ana@@x.es: invalido
telefono 612345678: valido
cp 28001: valido
fecha 29/02/2024: valido
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
