# Producto cartesiano y combinaciones

La primera línea es un entero R ≥ 1. Las demás (al menos una) son listas de elementos separados
por comas, sin espacios, todos no vacíos (por ejemplo `rojo,verde`); un elemento puede repetirse.
Escribe:

- la línea `producto:` y después una línea por cada elemento del producto cartesiano de todas las
  listas (uno de cada lista), con sus elementos separados por un espacio, en el orden de
  `itertools.product` de Python (la primera lista cambia más despacio, la última más deprisa);
- la línea `combinaciones:` y después una línea por cada combinación de R elementos de la
  **primera** lista (`itertools.combinations`: se eligen por posición, no por valor, y salen en
  el orden de la lista), con sus elementos separados por un espacio. Si R es mayor que la
  cantidad de elementos de la primera lista, no sale ninguna línea.

## Ejemplo

Entrada:

```
2
a,b,c
1,2
```

Salida:

```
producto:
a 1
a 2
b 1
b 2
c 1
c 2
combinaciones:
a b
a c
b c
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
