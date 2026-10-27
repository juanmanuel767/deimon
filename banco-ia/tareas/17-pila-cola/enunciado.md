# Pila y cola

Hay una pila y una cola, las dos vacías al principio. Cada línea es una orden (los elementos X
son palabras sin espacios):
- "apilar X": pone X encima de la pila.
- "desapilar": quita el elemento de arriba de la pila y escribe "pila->X" (con X ese elemento);
  si la pila está vacía escribe "error: pila vacia".
- "encolar X": pone X al final de la cola.
- "desencolar": quita el primer elemento de la cola y escribe "cola->X"; si la cola está vacía
  escribe "error: cola vacia".
- "mostrar": escribe dos líneas: "pila: " seguido de los elementos desde el fondo hasta arriba
  separados por un espacio, y "cola: " seguido de los elementos desde el primero hasta el último
  separados por un espacio. Si está vacía, escribe "vacia" en lugar de los elementos
  (por ejemplo "pila: vacia").

## Ejemplo

Entrada:

```
apilar a
apilar b
encolar 1
encolar 2
mostrar
desapilar
desencolar
mostrar
```

Salida:

```
pila: a b
cola: 1 2
pila->b
cola->1
pila: a
cola: 2
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
