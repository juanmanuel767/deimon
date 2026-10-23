# El lenguaje Deimon

[Volver al README](../README.md)

## El lenguaje en pocas palabras

```deimon
# Esto es un comentario
sea nombre = "Ana"
sea edad = 30
mostrar("Hola, " + nombre)

funcion suma(a, b) {
    devolver a + b
}

mostrar(suma(2, 3))

si edad >= 18 {
    mostrar("mayor")
} sino si edad > 12 {
    mostrar("adolescente")
} sino {
    mostrar("menor")
}

sea i = 0
mientras i < 3 {
    mostrar(i)
    i = i + 1
}
```

### Valores

- Números: `30`, `2.5`, `-7`. Internamente son números decimales (f64).
  Al mostrarlos, `5` sale como `5` y `2.5` como `2.5`.
- Textos: entre comillas dobles, por ejemplo `"hola"`. Se pueden usar los
  escapes `\n` (salto de línea), `\"` (comillas) y `\\` (barra invertida).
- Lógicos: `verdadero` y `falso`.
- Nada: `nada`.

### Operadores

`+ - * / %`, comparaciones `== != < <= > >=`, lógicos `y`, `o`, `no`,
paréntesis y el menos unario. La precedencia es la habitual: primero `* / %`,
luego `+ -`, luego las comparaciones, luego `no`, luego `y` y por último `o`.

El operador `+` concatena si alguno de los dos lados es un texto:

```deimon
mostrar("Total: " + 5)   # escribe Total: 5
```

### Variables

`sea` declara una variable. Después se puede cambiar con `=`:

```deimon
sea contador = 0
contador = contador + 1
```

Si escribes una variable que no existe, el traductor da un error en español
con el número de línea y no genera nada.

### Funciones

```deimon
funcion saludar(nombre) {
    devolver "Hola, " + nombre
}
```

Las funciones se pueden llamar antes de donde están escritas y pueden
llamarse a sí mismas (recursión). Cada función tiene su propio ámbito: ve sus
parámetros y las variables que declare dentro, pero no las variables creadas
fuera de ella.

### Condiciones y bucles

```deimon
si condicion {
    ...
} sino si otra_condicion {
    ...
} sino {
    ...
}
```

```deimon
mientras condicion {
    ...
}
```

Dentro de un `mientras` o un `para`, `romper` sale del bucle y `continuar` salta
a la siguiente vuelta.

Para elegir entre dos valores en una sola línea: `elegir(condicion, a, b)`
devuelve `a` si se cumple y `b` si no, y solo calcula el que elige:
`mostrar(elegir(edad >= 18, "adulto", "menor"))`.

### Mostrar

`mostrar(valor)` escribe el valor y un salto de línea.

### Conversiones

`a_numero(texto)` convierte un texto en un número. Si el texto no es un número
válido, el programa termina con un error en español. `a_texto(valor)` convierte
cualquier valor en su texto, igual que al mostrarlo.

```deimon
mostrar(a_numero("21") * 2)   # escribe 42
mostrar(a_texto(2.5) + "!")   # escribe 2.5!
```

## Reglas de nombres

Las funciones integradas siguen una sola regla, sin excepciones:

- Las acciones y las consultas se escriben con un verbo en infinitivo:
  `mostrar`, `medir`, `agregar`, `guardar`, `leer`, `borrar`, `contar`,
  `listar_claves`, `servir`, `visitar`, `comprobar`.
- Las conversiones se escriben `a_<tipo>`: `a_numero(texto)` y `a_texto(valor)`.
- Las palabras de un nombre se unen con `_`, por ejemplo `listar_claves`.

Los nombres antiguos `numero`, `largo` y `claves` ya no existen: usarlos da el
error `E021`, con el nombre nuevo como sugerencia y arreglo automático para
`deimon comprobar --arreglar`.

`prueba` y `extension` son palabras reservadas, igual que `funcion` o `ruta`:
no se pueden usar como nombre de variable, función o parámetro.

## Listas

Una lista guarda varios valores en orden. Se escribe entre corchetes.

```deimon
sea cosas = [1, "a", verdadero]
mostrar(cosas)          # escribe [1, a, verdadero]
mostrar(cosas[0])       # escribe 1 (se empieza a contar desde 0)
mostrar(medir(cosas))   # escribe 3; también sirve con textos
sea mas = agregar(cosas, 4)   # lista nueva, la original no cambia
mostrar(cosas + [5, 6])       # une dos listas
para cosa en cosas {
    mostrar(cosa)
}
```

También se puede cambiar un elemento de una lista por su índice:

```deimon
sea notas = [10, 20, 30]
notas[1] = 25          # [10, 25, 30]
notas[1] = notas[1] + 5
```

La variable debe existir. Si el índice no existe o no es un número entero, el
programa termina con un error en español (código 1) que dice la línea.

Más funciones para listas y textos:

| Función | Qué hace |
|---|---|
| `incluye(lista, valor)` | `verdadero` si la lista tiene el valor (o si un texto contiene otro) |
| `ordenar(lista)` | lista nueva de menor a mayor (solo números o solo textos) |
| `invertir(lista)` | lista o texto al revés |
| `cortar(lista, inicio, cantidad)` | un trozo; `inicio` negativo cuenta desde el final y `cantidad` `nada` llega hasta el final: `cortar("Deimon", 0, 3)` da `"Dei"` |
| `rango(1, 5)` | `[1, 2, 3, 4, 5]` (los dos extremos incluidos) |
| `sumar_lista(lista)` | suma los números de la lista |

### Listas con claves

Guardan valores con un nombre (como los arrays con nombre de PHP o los
diccionarios de Python). Se escriben entre llaves y recuerdan el orden:

```deimon
sea persona = {"nombre": "Ana", "edad": 30}
mostrar(persona["nombre"])        # Ana
persona["edad"] = 31              # cambia un valor
persona["ciudad"] = "Lima"        # añade uno nuevo
mostrar(persona["falta"] == nada) # verdadero: una clave que no está da nada
mostrar(tiene_clave(persona, "edad"))      # verdadero
persona = quitar_clave(persona, "ciudad")
para clave en persona {           # recorre las claves en orden
    mostrar(clave + ": " + persona[clave])
}
```

`medir(persona)` cuenta las claves, `listar_claves(persona)` las da en una
lista, y se pueden guardar con `guardar` y pasar a JSON con `a_json`.

Los textos también se pueden leer por índice, contando caracteres (no bytes):
`"año"[1]` devuelve `"ñ"`. No se pueden cambiar por partes: `texto[i] = ...`
es un error de traducción (`E023`); crea un texto nuevo.

## Ejemplos incluidos

- `ejemplos/hola.dmn`: variables, textos, función y condiciones.
- `ejemplos/fibonacci.dmn`: función recursiva.
- `ejemplos/bucles.dmn`: bucle `mientras`, funciones y operaciones.
- `ejemplos/datos.dmn`: listas, `para` y datos permanentes.
- `ejemplos/pruebas.dmn`: funciones con sus pruebas (`deimon probar ejemplos/pruebas.dmn`).
- `ejemplos/varios-archivos.dmn`: divide el programa con `usar` y reutiliza
  `ejemplos/piezas/calculos.dmn` (funciones y una prueba).
- `ejemplos/biblioteca.dmn`: usa los módulos `matematicas`, `texto` y `tiempo`
  de la biblioteca estándar.
- `ejemplos/modulo-propio.dmn`: usa el módulo propio `ejemplos/modulos/saludos/`.

Cada ejemplo de la carpeta `ejemplos/` (sin contar las subcarpetas `piezas/` y `modulos/`)
tiene un archivo `.esperado` con la salida exacta que produce. Además, `ejemplos-web/sitio.dmn` es un sitio web de prueba con tres
páginas.

La gramática completa, en notación EBNF, está en [GRAMATICA.md](GRAMATICA.md).

