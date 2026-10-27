# Carrito con descuentos

Cada línea es "producto cantidad precio": producto es una palabra sin espacios, cantidad un entero
≥ 1 y precio el precio unitario en euros con como mucho 2 decimales (por ejemplo "12.5" o "3").
Cada línea es una línea de compra independiente (aunque el producto se repita). Todo se calcula en
céntimos y cada redondeo es al céntimo más cercano, hacia arriba si queda justo en la mitad.

1. Línea de compra: bruto = cantidad × precio. Descuento por cantidad: 10 % si la cantidad es 10 o
   más, 5 % si es 5 o más (y menos de 10), 0 % si no. Importe de la línea = bruto menos ese
   descuento, redondeado. Escribe "producto xCANTIDAD: importe" (2 decimales) y, si el descuento
   no es 0, añade al final " (-D%)" con D = 5 o 10.
2. "subtotal: S": suma de los importes de las líneas.
3. Si S es 100.00 o más, descuento del pedido del 5 %: escribe "descuento pedido: -X" (X = S menos
   S × 95 / 100 redondeado) y el nuevo total es S × 95 / 100 redondeado. Si no, no se escribe esa
   línea y el total es S.
4. "envio: E": 0.00 si el total ya es de 50.00 o más, y 4.95 si no.
5. "total: T" = total más envío.
Todas las cantidades de dinero se escriben con exactamente 2 decimales. Con la entrada vacía, el
subtotal es 0.00, el envío 4.95 y el total 4.95.

## Ejemplo

Entrada:

```
pan 6 1.20
leche 2 0.95
queso 1 12.5
```

Salida:

```
pan x6: 6.84 (-5%)
leche x2: 1.90
queso x1: 12.50
subtotal: 21.24
envio: 4.95
total: 26.19
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
