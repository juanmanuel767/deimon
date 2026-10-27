# Inventario con informe

Cada línea es una orden sobre un almacén vacío (CODIGO y NOMBRE son palabras sin espacios; los
precios están en céntimos):
- "alta CODIGO NOMBRE PRECIO STOCK": crea el producto. Si el código ya existe no cambia nada y
  escribe "error: codigo duplicado CODIGO".
- "venta CODIGO N": vende N unidades (N ≥ 1): resta N del stock y suma N × PRECIO a los ingresos
  de ese producto. Si el código no existe escribe "error: codigo desconocido CODIGO"; si no hay
  stock suficiente no cambia nada y escribe "error: stock insuficiente de CODIGO (hay H, piden N)".
- "reponer CODIGO N": suma N al stock; si el código no existe escribe "error: codigo desconocido CODIGO".
- "informe": escribe una línea por producto, en orden alfabético de código,
  "CODIGO NOMBRE stock=S ingresos=E" (E en euros con 2 decimales); luego "ingresos totales: E"
  y por último "sin stock: " seguido de los códigos con stock 0, en orden alfabético y separados
  por una coma y un espacio, o la palabra "ninguno" si no hay ninguno.

## Ejemplo

Entrada:

```
alta B2 tuerca 25 100
alta A1 tornillo 10 5
venta A1 3
venta A1 9
venta Z9 1
reponer A1 10
informe
```

Salida:

```
error: stock insuficiente de A1 (hay 2, piden 9)
error: codigo desconocido Z9
A1 tornillo stock=12 ingresos=0.30
B2 tuerca stock=100 ingresos=0.00
ingresos totales: 0.30
sin stock: ninguno
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
