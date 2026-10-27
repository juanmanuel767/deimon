# Plantilla con marcadores

La primera línea es una plantilla de texto. Las demás líneas (puede no haber ninguna) son
definiciones "clave=valor": la clave es lo que hay antes del primer "=", y el valor es todo lo
que sigue (puede contener "=" y espacios; nunca está vacío). Si una clave se define varias veces,
vale la última.

En la plantilla, busca de izquierda a derecha cada "{". Si lo que sigue hasta la primera "}"
(sin incluirla) es un nombre válido, es decir, una o más letras ASCII sin acento, cifras o "_",
el marcador "{nombre}" se sustituye por su valor, o se deja tal cual (con las llaves) si esa
clave no está definida; y se sigue justo después de la "}". Si no es un nombre válido (o no hay
"}" después), la "{" se escribe tal cual y se sigue con el carácter siguiente. El texto
sustituido no se vuelve a examinar. Escribe una sola línea con el resultado.

## Ejemplo

Entrada:

```
Hola {nombre}, tienes {n} mensajes y {otro}.
nombre=Ana
n=3
```

Salida:

```
Hola Ana, tienes 3 mensajes y {otro}.
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
