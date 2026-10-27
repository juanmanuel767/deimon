# Gramática del lenguaje Deimon

Esta es la gramática completa de `deimon`, escrita en notación EBNF y explicada en
español sencillo. Un programa `.dmn` se guarda como texto UTF-8 y se traduce a
Rust con `deimon traducir`, `deimon ejecutar` o `deimon compilar`.

La notación EBNF que se usa aquí es la habitual:

| Símbolo | Significado |
|---|---|
| `= ` | la regla de la izquierda se escribe como lo de la derecha |
| `,` | una cosa detrás de otra |
| `;` | termina la regla |
| `{ ... }` | lo de dentro se puede repetir cero o más veces |
| `[ ... ]` | lo de dentro es opcional (cero o una vez) |
| `"..."` | un texto literal que se escribe tal cual |
| `\|` | una cosa u otra |
| `(* ... *)` | un comentario de la gramática, no del programa |

## 1. Programa

```ebnf
programa        = { linea_vacia | sentencia }, fin_archivo ;
sentencia       = declaracion
                | tipo_propio
                | uso
                | funcion
                | ruta
                | prueba
                | condicional
                | bucle_mientras
                | bucle_para
                | intento
                | devolucion
                | "romper"
                | "continuar"
                | asignacion
                | expresion ;
```

- Un programa es una lista de sentencias.
- Cada sentencia va en su propia línea. No se usa `;`.
- Las líneas vacías se ignoran.
- Las sentencias de nivel principal se ejecutan de arriba abajo.
- `funcion`, `ruta`, `prueba` y `usar` solo se escriben en el nivel principal
  (no dentro de una función, un `si` o un bucle).

## 2. Declaraciones, uso de archivos y bloques

```ebnf
declaracion     = "sea", identificador, "=", expresion ;

tipo_propio     = "tipo", identificador, "{", salto_de_linea,
                  { campo, salto_de_linea }, "}" ;
campo           = identificador, [ ":", tipo_de_campo ] ;
tipo_de_campo   = "numero" | "texto" | "logico" | "lista"
                | "lista_con_claves" | "funcion" ;

uso             = "usar", texto
                | "usar", "modulo", texto ;

funcion         = "funcion", identificador, "(", [ parametros ], ")", bloque ;
parametros      = identificador, { ",", identificador } ;

ruta            = "ruta", texto, bloque ;

prueba          = "prueba", texto, bloque ;

bloque          = "{", { linea_vacia | sentencia }, "}" ;
linea_vacia     = salto_de_linea, { salto_de_linea } ;
```

- `sea nombre = valor` crea una variable. Para cambiarla después se escribe
  `nombre = nuevo_valor` (sin `sea`).
- `tipo Persona { nombre: texto  edad: numero }` (un campo por línea) crea una
  función `Persona(datos)` que comprueba los campos y los devuelve en el orden
  del tipo. El tipo de cada campo es opcional. Solo se escribe en el nivel
  principal, y `tipo` sigue valiendo como nombre normal en otros sitios.
- `usar "piezas/calculos.dmn"` carga las funciones, rutas y pruebas de otro
  archivo. La ruta es relativa a la carpeta del archivo que la contiene.
- `usar modulo "texto"` carga un módulo: una carpeta con `modulo.txt` y
  funciones en Rust y/o en Deimon. Se busca en `modulos/<nombre>/` junto al
  archivo, en `~/.deimon/modulos/<nombre>/` y en la biblioteca estándar
  (`matematicas`, `texto`, `tiempo`). Ver [modulos.md](modulos.md).
- El bloque se abre con `{` al final de la misma línea que la condición, el
  nombre o la ruta, y se cierra con `}`.
- Un bloque puede estar vacío: `{ }`.

## 3. Condiciones y bucles

```ebnf
condicional     = "si", expresion, bloque,
                  { "sino", "si", expresion, bloque },
                  [ "sino", bloque ] ;

bucle_mientras  = "mientras", expresion, bloque ;

bucle_para      = "para", identificador, "en", expresion, bloque ;
```

- `si condicion { ... }` ejecuta el bloque si la condición es `verdadero`.
- Después puede venir `sino si otra_condicion { ... }` y, al final, un
  `sino { ... }` sin condición.
- `mientras condicion { ... }` repite el bloque mientras la condición sea
  verdadera.
- `para elemento en lista { ... }` recorre una lista y da a `elemento` cada
  valor en orden. Con una lista con claves, `elemento` recibe cada clave.
- `romper` sale del bucle en el que está; `continuar` salta a la siguiente
  vuelta. Fuera de un `mientras` o un `para` son un error (E011).
- `intentar { ... } si falla e { ... }` ejecuta el primer bloque; si algo falla
  (una división entre cero, un índice fuera de rango, `provocar_error(...)`),
  salta al segundo bloque. `e` es un mapa con `"mensaje"` y `"linea"`. Se pueden
  anidar, y dentro valen `romper`, `continuar` y `devolver`. Un error fuera de
  `intentar` sigue terminando el programa.

```ebnf
intento         = "intentar", bloque, "si", "falla", identificador, bloque ;
```

## 4. Devolver, asignar y expresiones sueltas

```ebnf
devolucion      = "devolver", [ expresion ] ;

asignacion      = destino, "=", expresion ;
destino         = identificador
                | primaria, "[", expresion, "]", { "[", expresion, "]" } ;
```

- `devolver valor` termina una función, una ruta o una prueba y entrega el
  valor. `devolver` sin valor entrega `nada`.
- `nombre = valor` cambia una variable que ya existe.
- `lista[i] = valor` cambia un elemento de una lista; `lista[i][j] = valor`
  cambia un elemento de una lista dentro de otra.
- Una expresión suelta (por ejemplo `mostrar(1)`) también es una sentencia.

## 5. Expresiones y precedencia

La precedencia va de menor a mayor: primero `o`, luego `y`, luego `no`, luego
las comparaciones, luego `+ -`, después `* / %`, después el menos unario y, por
último, las llamadas, los índices y los paréntesis.

```ebnf
expresion            = expresion_o ;
expresion_o          = expresion_y, { "o", expresion_y } ;
expresion_y          = expresion_no, { "y", expresion_no } ;
expresion_no         = "no", expresion_no
                     | expresion_comparacion ;
expresion_comparacion = expresion_suma, { comparacion, expresion_suma } ;
comparacion          = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expresion_suma       = expresion_producto, { ( "+" | "-" ), expresion_producto } ;
expresion_producto   = expresion_unaria, { ( "*" | "/" | "%" ), expresion_unaria } ;
expresion_unaria     = "-", expresion_unaria
                     | primaria ;
```

- `+` suma números, concatena textos y une listas.
- `-`, `*`, `/` y `%` trabajan con números. Dividir entre cero es un error al
  ejecutar.
- Las comparaciones devuelven `verdadero` o `falso`. `<`, `<=`, `>` y `>=`
  solo se usan con números.
- `y`, `o` y `no` solo se usan con valores lógicos.

## 6. Valores, llamadas e índices

```ebnf
primaria        = base, { "[", expresion, "]" } ;

base            = numero
                | texto
                | "verdadero"
                | "falso"
                | "nada"
                | lista
                | lista_con_claves
                | llamada
                | identificador
                | "(", expresion, ")" ;

lista           = "[", [ expresion, { ",", expresion } ], "]" ;
lista_con_claves = "{", [ par, { ",", par } ], "}" ;
par             = expresion, ":", expresion ;
llamada         = identificador, "(", [ argumentos ], ")" ;
argumentos      = expresion, { ",", expresion } ;
```

- Un número es decimal: `30`, `2.5`, `-7`. El menos forma parte de la
  expresión, no del número.
- Un texto va entre comillas dobles y en una sola línea. Los escapes válidos
  son `\n` (salto de línea), `\t` (tabulador), `\"` (comillas) y `\\` (barra
  invertida). Con `f` pegada a la comilla (`f"Hola {nombre}"`) las llaves
  llevan dentro cualquier expresión (`{{` y `}}` escriben una llave). Un texto
  entre `"""` y `"""` puede ocupar varias líneas (también `f"""..."""`): se
  quita el salto de línea tras la apertura, la última línea si solo tiene
  espacios y la sangría común.
- Una lista se escribe entre corchetes y puede mezclar valores:
  `[1, "a", verdadero]`.
- `lista[i]` lee un elemento; se empieza a contar desde `0`. También se puede
  leer un texto por índice: `"año"[1]`.
- Una lista con claves va entre llaves: `{"nombre": "Ana", "edad": 30}`.
  `p["nombre"]` lee un valor (o `nada` si la clave no está) y `p["edad"] = 31`
  lo cambia o lo añade. Las claves son textos (un número entero se guarda como
  texto) y se recuerdan en el orden en que se añadieron. Una lista vacía `[]`
  que recibe una clave de texto pasa a ser una lista con claves, como en PHP.
- Una llamada usa paréntesis y argumentos separados por comas:
  `suma(2, 3)`.
- Los paréntesis cambian la precedencia: `(2 + 3) * 4`.

## 7. Piezas del texto

```ebnf
identificador   = ( letra | "_" ), { letra | digito | "_" } ;
numero          = digito, { digito }, [ ".", digito, { digito } ] ;
texto           = '"', { caracter_texto | escape }, '"' ;
escape          = "\\", ( "n" | "t" | '"' | "\\" ) ;
comentario      = "#", { cualquier_caracter_sin_salto } ;
salto_de_linea  = "\n" ;
fin_archivo     = (* el final del texto *) ;
```

- Las letras pueden llevar acentos y la letra `ñ`; también se aceptan letras
  de otros idiomas.
- Un comentario empieza por `#` y llega hasta el final de su línea. El lexer
  los ignora al traducir, pero `deimon formatear` los conserva.
- Los espacios y los tabuladores separan las piezas; fuera de un texto no
  cambian el significado.

## 8. Reglas de nombres

- Los nombres de variables, funciones, parámetros y pruebas pueden tener
  letras, dígitos y `_`, pero no pueden empezar por un dígito.
- Las acciones y las consultas se escriben con un verbo en infinitivo:
  `mostrar`, `medir`, `agregar`, `guardar`, `leer`, `borrar`, `contar`,
  `listar_claves`, `servir`, `visitar`, `comprobar`.
- Las conversiones se escriben `a_<tipo>`: `a_numero(texto)` y
  `a_texto(valor)`.
- Las palabras de un nombre se unen con `_`, por ejemplo `listar_claves` o
  `mayor_de`. No se usan mayúsculas ni guiones en los nombres.
- Los nombres antiguos `numero`, `largo` y `claves` ya no existen: dan el
  error E021 con el nombre nuevo como sugerencia.
- Una palabra reservada no se puede usar como nombre de variable, función,
  parámetro, prueba ni parámetro de ruta. Si se intenta, sale el error E022.
  Excepción: `y` y `o` son palabras contextuales (ver sección 9).

## 9. Palabras reservadas

Estas son todas las palabras que el lexer reconoce como parte del lenguaje:

| Palabra | Para qué sirve |
|---|---|
| `sea` | declara una variable |
| `usar` | carga otro archivo o un módulo |
| `funcion` | define una función |
| `ruta` | define una página web |
| `prueba` | define una prueba |
| `modulo` | acompaña a `usar` para cargar un módulo |
| `extension` | forma antigua de `modulo` (E021 la corrige sola) |
| `devolver` | termina y entrega un valor |
| `romper` | sale del bucle |
| `continuar` | salta a la siguiente vuelta del bucle |
| `si` | empieza una condición |
| `sino` | rama alternativa de un `si` |
| `mientras` | bucle con condición |
| `para` | recorre una lista |
| `intentar` | ejecuta un bloque y atrapa sus errores |
| `falla` | en `si falla e { ... }`, el bloque que se ejecuta si hubo un error |
| `en` | separa la variable y la lista en `para` |
| `y` | y lógico (contextual: solo justo después de un operando completo; en otro sitio es un nombre normal) |
| `o` | o lógico (contextual, igual que `y`) |
| `no` | negación lógica |
| `verdadero` | valor lógico cierto |
| `falso` | valor lógico falso |
| `nada` | ausencia de valor |

## 10. Estilo de formato

`deimon formatear archivo.dmn` reescribe el archivo con un único estilo. Con
`deimon formatear archivo.dmn --comprobar` no escribe nada: dice qué líneas
cambiarían y termina con código 1 si el archivo no está formateado.

- Sangría de 4 espacios por cada nivel de bloque.
- La `{` va al final de la línea que abre el bloque.
- La `}` va sola en su línea o seguida de ` sino {` o ` sino si condicion {`.
- Un espacio a cada lado de los operadores binarios y de `=`.
- Ningún espacio después de `(` ni de `[`, y ninguno antes de `)`, `]` ni `,`.
- Un espacio después de cada `,`.
- El menos unario va pegado a su valor: `-x`, no `- x`.
- No se dejan espacios al final de las líneas.
- Como máximo hay una línea vacía seguida.
- Hay una línea vacía entre las definiciones de nivel superior: `funcion`,
  `ruta` y `prueba`.
- El archivo termina con un único salto de línea.
- Los comentarios se conservan en su sitio. Un comentario al final de una
  línea de código se separa con dos espacios.
- El texto entre comillas no se toca.

El formateador es idempotente: formatear dos veces el mismo archivo da
exactamente el mismo resultado. Si el archivo tiene errores de sintaxis, no se
toca y se muestran los errores igual que con `deimon comprobar`.

## 11. Ejemplo completo

```deimon
# Saludo y suma
funcion suma(a, b) {
    devolver a + b
}

sea nombre = "Ana"
si suma(2, 3) == 5 {
    mostrar("Hola, " + nombre)
} sino {
    mostrar("Algo no cuadra")
}
```
