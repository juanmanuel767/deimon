# Inventario con órdenes

Cada línea es una orden sobre un inventario que empieza vacío:
- "poner PRODUCTO N": suma N unidades (N entero positivo).
- "quitar PRODUCTO N": resta N unidades; si no hay bastantes, no cambia nada y escribe
  "error: no hay N de PRODUCTO".
- "ver": escribe una línea por producto con unidades mayores que 0, en orden alfabético, con el
  formato "PRODUCTO=N"; si no hay ninguno, escribe "vacio".

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas; la última termina en salto
de línea) y escribe exactamente la salida pedida, cada línea terminada en salto de línea, sin
espacios de más.
