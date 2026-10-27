# Operaciones de conjuntos

La entrada tiene exactamente dos líneas: el conjunto A y el conjunto B. Cada una son palabras
separadas por espacios (pueden repetirse; la línea puede estar vacía = conjunto vacío). Escribe
estas siete líneas, en este orden, donde cada conjunto se escribe con sus palabras sin repetir,
ordenadas por código de caracteres y separadas por un espacio:

```
union: ...           todas las palabras de A o de B
interseccion: ...    las que están en A y en B
solo A: ...          las de A que no están en B
solo B: ...          las de B que no están en A
simetrica: ...       las que están en uno solo de los dos
A en B: si|no        «si» si todas las palabras de A están en B (un A vacío está en cualquier B)
disjuntos: si|no     «si» si no tienen ninguna palabra en común
```

Si un conjunto sale vacío, la línea es solo la etiqueta con los dos puntos, sin espacio detrás
(por ejemplo `interseccion:`).

## Ejemplo

Entrada:

```
a b c c
b c d
```

Salida:

```
union: a b c d
interseccion: b c
solo A: a
solo B: d
simetrica: a d
A en B: no
disjuntos: no
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
