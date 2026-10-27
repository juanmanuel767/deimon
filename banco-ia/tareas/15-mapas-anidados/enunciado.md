# Mapas anidados

Cada línea es "ruta=valor". La ruta son una o más claves separadas por puntos (por ejemplo
"servidor.puerto"); el valor es todo lo que sigue al primer "=" (puede contener puntos, "=" y
espacios; nunca está vacío ni empieza o acaba en espacio). Las claves no están vacías ni
contienen "=" ni ".". Ninguna ruta es prefijo propio de otra (si "a.b" existe, no existe "a" ni
"a.b.c"). Si una ruta se repite, vale el último valor.

Construye el árbol de claves y escríbelo: cada clave en su propia línea, con 2 espacios de
sangría por nivel de profundidad (nivel 0 sin sangría), y las claves de cada nivel en orden
alfabético. Una clave con hijos se escribe "clave:"; una clave con valor, "clave: valor".

## Ejemplo

Entrada:

```
servidor.puerto=8080
servidor.host=local host
nombre=app
servidor.tls.activo=si
```

Salida:

```
nombre: app
servidor:
  host: local host
  puerto: 8080
  tls:
    activo: si
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
