# Ventas por producto

Cada línea es una venta: "producto;cantidad;precio_unitario" (cantidad entera, precio con punto
decimal y como mucho 2 decimales). Escribe una línea por producto, en orden alfabético, con el
formato "producto: total" donde total es la suma de cantidad × precio con exactamente 2 decimales.
Al final, una línea "TOTAL: x" con la suma de todo, también con 2 decimales. Si no hay ventas,
solo se escribe "TOTAL: 0.00".

## Ejemplo

Entrada:

```
pan;3;1.20
leche;2;0.95
pan;1;1.20
huevos;12;0.25
leche;1;0.99
```

Salida:

```
huevos: 3.00
leche: 2.89
pan: 4.80
TOTAL: 10.69
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
