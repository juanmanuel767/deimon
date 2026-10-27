# Inventario con órdenes

Cada línea es una orden sobre un inventario que empieza vacío:
- "poner PRODUCTO N": suma N unidades (N entero positivo).
- "quitar PRODUCTO N": resta N unidades; si no hay bastantes (o el producto no existe), no cambia
  nada y escribe "error: no hay N de PRODUCTO".
- "ver": escribe una línea por producto con unidades mayores que 0, en orden alfabético, con el
  formato "PRODUCTO=N"; si no hay ninguno, escribe "vacio".

## Ejemplo

Entrada:

```
poner tornillo 10
poner clavo 5
quitar tornillo 3
ver
quitar clavo 9
quitar clavo 5
ver
quitar tuerca 1
poner arandela 2
ver
```

Salida:

```
clavo=5
tornillo=7
error: no hay 9 de clavo
tornillo=7
error: no hay 1 de tuerca
arandela=2
tornillo=7
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
