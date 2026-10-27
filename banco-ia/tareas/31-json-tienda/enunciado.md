# Leer un JSON pequeño

La entrada es un único documento JSON (puede ocupar varias líneas, con cualquier sangría) con esta
forma: {"tienda": TEXTO, "productos": [ {"nombre": TEXTO, "precio": ENTERO, "stock": ENTERO}, ... ]}.
El precio está en céntimos. Los objetos pueden traer las claves en cualquier orden y pueden tener
además otras claves (textos, números, listas u objetos) que se ignoran. Los textos no llevan
escapes con barra invertida.

Escribe "tienda: NOMBRE"; luego, por cada producto con stock mayor que 0, en el orden en que
aparecen, "nombre: precio x stock = valor", con precio y valor en euros con 2 decimales (valor =
precio × stock). Después "agotados: N" (productos con stock 0) y "valor total: T" (suma de los
valores, 2 decimales).

## Ejemplo

Entrada:

```
{"tienda": "Casa Ana", "productos": [{"nombre": "pan", "precio": 120, "stock": 10}, {"nombre": "leche", "precio": 95, "stock": 0}]}
```

Salida:

```
tienda: Casa Ana
pan: 1.20 x 10 = 12.00
agotados: 1
valor total: 12.00
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
