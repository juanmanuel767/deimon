# Procesar un JSON anidado

La entrada completa (varias líneas) es un único documento JSON con esta forma:

```
{"tienda": "Nombre",
 "pedidos": [
   {"id": 1,
    "cliente": {"nombre": "Ana", "ciudad": "Madrid"},
    "lineas": [{"sku": "A1", "cantidad": 2, "centimos": 150}],
    "cancelado": true}
 ]}
```

`cantidad` y `centimos` (precio de una unidad) son enteros ≥ 0. `ciudad` puede faltar (entonces la
ciudad es «sin ciudad»), `lineas` puede ser una lista vacía y `cancelado` solo aparece (con
`true`) en algunos pedidos. Los textos pueden llevar letras con acento, ñ y secuencias `\uXXXX`.
`pedidos` puede ser una lista vacía. Los importes se escriben en euros con dos decimales
(1250 céntimos → `12.50`). Escribe:

- `tienda: NOMBRE`;
- una línea por pedido, en el orden del documento: `pedido ID: CLIENTE (CIUDAD) - TOTAL`, donde
  TOTAL es la suma de cantidad × centimos de sus líneas; si el pedido está cancelado, en vez de
  ` - TOTAL` se escribe ` - cancelado`;
- una línea por ciudad con pedidos no cancelados: `ciudad NOMBRE: TOTAL` (suma de los totales de
  esos pedidos), de más a menos total y, si empatan, por orden alfabético de código de caracteres;
- `total tienda: TOTAL` (suma de los pedidos no cancelados).

## Ejemplo

Entrada:

```
{"tienda": "Demo", "pedidos": [
 {"id": 1, "cliente": {"nombre": "Ana", "ciudad": "Madrid"},
  "lineas": [{"sku": "A1", "cantidad": 2, "centimos": 150}, {"sku": "B2", "cantidad": 1, "centimos": 1000}]},
 {"id": 2, "cliente": {"nombre": "Luis"}, "lineas": [{"sku": "A1", "cantidad": 3, "centimos": 150}]},
 {"id": 3, "cliente": {"nombre": "Eva", "ciudad": "Madrid"}, "lineas": [], "cancelado": true}
]}
```

Salida:

```
tienda: Demo
pedido 1: Ana (Madrid) - 13.00
pedido 2: Luis (sin ciudad) - 4.50
pedido 3: Eva (Madrid) - cancelado
ciudad Madrid: 13.00
ciudad sin ciudad: 4.50
total tienda: 17.50
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
