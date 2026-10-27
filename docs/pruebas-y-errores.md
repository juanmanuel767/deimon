# Pruebas y errores

[Volver al README](../README.md)

## Pruebas

Un programa puede llevar sus propias pruebas. Se escriben al nivel principal con
`prueba "nombre" { ... }`:

```text
funcion suma(a, b) {
    devolver a + b
}

ruta "/saludo/:nombre" {
    devolver "<h1>Hola, " + nombre + "</h1>"
}

prueba "suma básica" {
    comprobar(suma(2, 3) == 5)
    comprobar_igual(suma(2, 2), 4)
    comprobar_igual(visitar("/saludo/Ana"), "<h1>Hola, Ana</h1>")
}
```

- El cuerpo de una prueba funciona como una función sin parámetros: puede llamar
  a las funciones del programa, usar listas y usar `guardar` / `leer`.
- `comprobar(condicion)` falla si la condición no es verdadera.
- `comprobar_igual(obtenido, esperado)` falla si los dos valores son distintos y
  dice: `línea 7: se esperaba 4 y se obtuvo 5`.
- `visitar("/camino")` visita una ruta del programa sin usar la red y devuelve
  el texto de la página (igual que el servidor: decodifica y protege los
  parámetros). Si no hay ruta, devuelve la página de «no encontrada».
- Cuando una comprobación falla, esa prueba termina y se pasa a la siguiente.
  Un error en la ejecución dentro de una prueba también cuenta como fallo de esa
  prueba, sin detener las demás.
- Dos pruebas no pueden tener el mismo nombre.

Para correrlas:

```bash
deimon probar ejemplos/pruebas.dmn          # una línea por prueba
deimon probar ejemplos-web/app.dmn --json   # resultado escrito para otro programa
```

```text
✓ suma básica
✗ saludo — línea 12: se esperaba "adiós" y se obtuvo "hola"
Pruebas: 2 · correctas: 1 · fallidas: 1
```

`deimon probar` termina con código 0 solo si hay al menos una prueba y todas pasan.

Las pruebas se corren con el intérprete propio, sin Rust y casi al instante (las 6 pruebas de `proyectos/tienda` tardan unos 3 s, frente a unos 35 s compilando). Si el programa usa algo que el intérprete no lleva (por ejemplo el módulo `sql`), se compilan con Rust como antes, con el mismo resultado. `DEIMON_MOTOR=rust deimon probar ...` obliga a usar Rust.
Solo corre las pruebas: no ejecuta el resto del programa ni arranca el
servidor. Usa una carpeta de datos temporal que se borra al terminar, así que
tus datos reales nunca se tocan. `deimon ejecutar` ignora las pruebas.

`deimon app` corre las pruebas antes de crear el APK. Si alguna falla, no crea el
APK y te lo explica. Si el programa no tiene pruebas, avisa («este programa no
tiene pruebas») y sigue.

## Errores y sugerencias

Hay dos momentos en los que pueden aparecer errores:

- Al traducir (también con `deimon comprobar`): variables o funciones que no existen,
  argumentos de más o de menos, sintaxis, llaves sin cerrar... El programa termina
  con código 1 y no genera nada.
- Al ejecutar: dividir entre cero, tipos incompatibles, índice fuera de rango,
  número inválido... Se escribe por la salida de errores y el programa termina con
  código 1.

Todos los errores de traducción tienen el mismo formato. La primera línea no cambia;
debajo van la línea de código con un `^` bajo la columna, la sugerencia y el código
estable del error:

```text
Error en la línea 2: la variable 'totl' no existe
  2 | mostrar(totl)
    |         ^
Sugerencia: ¿Quisiste decir 'total'?
Código: E005
```

- Si un nombre está mal escrito (distancia de edición de 2 como máximo), se
  sugiere el nombre visible más parecido y se ofrece un arreglo automático.
- Si una llamada tiene argumentos de más o de menos, se muestra la firma:
  `La firma es: suma(a, b)`.
- Las palabras de otros lenguajes (`if`, `else`, `elif`, `while`, `for`, `function`,
  `def`, `fn`, `return`, `let`, `var`, `const`, `print`, `console.log`, `true`, `false`,
  `null`, `None`, `and`, `or`, `not`, `&&`, `||`, `!`, `===`...) dan el error `E008` con su
  equivalente exacto («En Deimon se escribe 'si'») y un arreglo automático. Si tu programa
  declara una variable con uno de esos nombres, no se toca.
- Las llaves, paréntesis, corchetes y comillas sin cerrar dicen en qué línea se abrieron.
- Asignar a una variable que no existe sugiere `sea x = ...`.

Un error puede llevar un **arreglo automático**: «en la línea L, de la columna A (incluida)
a la columna B (no incluida), pon este texto». Las columnas se cuentan en caracteres
desde 1.

Los errores en ejecución dicen la línea del programa `.dmn` que falló:

```text
Error en la línea 3 (en ejecución): no se puede dividir entre cero
```

Dentro de `deimon probar`, el detalle de la prueba termina con `(línea N)`.

### Tabla de códigos de error

| Código | Significado |
|--------|-------------|
| E001 | un texto entre comillas no está cerrado |
| E002 | carácter, escape o número que Deimon no reconoce |
| E003 | error de sintaxis: se esperaba otra cosa |
| E004 | llave, paréntesis o corchete sin cerrar (o sobrante) |
| E005 | la variable no existe |
| E006 | la función no existe |
| E007 | número de argumentos incorrecto |
| E008 | palabra u operador de otro lenguaje |
| E009 | se asigna a una variable que no se ha declarado |
| E010 | nombre repetido (variable, función, parámetro, ruta o prueba) |
| E011 | sentencia en un lugar donde no se permite |
| E012 | uso incorrecto de una función integrada |
| E013 | ruta web no válida |
| E014 | archivo usado con 'usar' no encontrado |
| E015 | uso circular entre archivos |
| E016 | función o ruta repetida entre archivos |
| E017 | extensión no encontrada |
| E018 | funciones.txt de una extensión mal formado |
| E019 | nombre de extensión no válido |
| E020 | función de extensión repetida o que choca con otra función |
| E021 | nombre antiguo de una función integrada |
| E022 | palabra reservada usada como nombre |
| E023 | se intenta cambiar un texto por partes |
| E024 | plantilla HTML que no existe o está mal escrita |
| E025 | operación con tipos que no encajan (fallaría seguro al ejecutarse) |
| E026 | función, ruta o módulo que no existe en el destino pc-sin-sistema (sin disco, red ni navegador) |
| E027 | tipo propio (`tipo Persona { ... }`): falta un campo, sobra uno o el valor no es del tipo pedido |
| E028 | llave sin cerrar, vacía o suelta dentro de un texto con valores `f"...{x}..."` |
| E999 | otro error |

## Comprobar un programa

```bash
deimon comprobar archivo.dmn              # lista todos los errores
deimon comprobar archivo.dmn --json       # lo mismo, escrito para que lo lea otro programa
deimon comprobar archivo.dmn --arreglar   # aplica los arreglos automáticos seguros
```

Antes de ejecutar nada, Deimon revisa también los tipos (`E025`): avisa de las operaciones
que fallarían seguro al llegar a ellas, como restar un texto (`"Ana" - 1`), comparar con `<`
algo que no es un número, usar un número como condición (`si edad { ... }`), recorrer un
número con `para` o poner corchetes a un número. Solo avisa cuando está seguro del tipo (un
valor escrito tal cual, una variable que solo ha recibido ese tipo, el resultado de
`a_texto`, `medir`...); si duda, calla, así que nunca da falsas alarmas. Los mensajes son los
mismos que daría el programa al fallar, con «(se ve sin ejecutar el programa)».

La revisión alcanza a todo el programa, no solo a cada función por separado:

- **Lo que devuelve una función tuya** se conoce si todos sus `devolver` dan el mismo tipo y la
  función no puede acabar sin `devolver`; así `funcion nombre() { devolver "Ana" }` y
  `nombre() - 1` avisa aunque la función esté definida más abajo. Si a veces devuelve texto y a
  veces número, o a veces no devuelve nada, no se avisa.
- **Lo que entra en una función tuya**: al llamarla con argumentos de tipo seguro se revisa su
  cuerpo con esos tipos. Con `funcion doble(x) { devolver x * 2 }` y `doble("a")` el aviso señala
  la línea de dentro de la función y la llamada: «(con doble("a") de la línea 4; se ve sin
  ejecutar el programa)». Cada fallo se avisa una sola vez.
- **Argumentos de las integradas**: `medir(5)` avisa «medir() espera una lista o un texto, no
  número», igual que al ejecutar. Cubre `a_numero`, `medir`, `incluye`, `ordenar`, `ordenar_por`,
  `invertir`, `cortar`, `sumar_lista`, `agregar`, `tiene_clave`, `quitar_clave` y
  `rellenar_plantilla`.
- **Índices**: `[1, 2]["a"]` avisa «el índice debe ser un número entero, no texto» (una lista
  vacía no, porque es también una lista con claves).

Las funciones guardadas en variables (`sea f = doble`) y los argumentos de tipo dudoso no se
revisan: si duda, calla.

`deimon comprobar` traduce el programa sin compilarlo ni ejecutarlo. Los errores de nombres,
argumentos y estructura se recogen todos a la vez; los de caracteres y de sintaxis
detienen el análisis en el primero. Termina con código 1 si hay errores y con 0 si no
(«✓ archivo.dmn no tiene errores»).

Con `--json` escribe, por ejemplo:

```json
{"archivo":"a.dmn","correcto":false,"errores":[{"codigo":"E005","linea":2,"columna":9,"mensaje":"la variable 'totl' no existe","sugerencia":"¿Quisiste decir 'total'?","arreglo":{"linea":2,"columna_inicio":9,"columna_fin":13,"reemplazo":"total"}}]}
```

`sugerencia` y `arreglo` son `null` cuando no hay. Con `--arreglar` se añade
`"arreglos_aplicados":N`.

`--arreglar` modifica el archivo: aplica los arreglos que no se solapan, vuelve a
comprobar y repite mientras queden arreglos que aplicar; al final informa de cuántos
aplicó y de los errores que sigan ahí. Haz una copia antes si no usas control de versiones.

## Formatear el código

`deimon formatear` deja un archivo `.dmn` con el estilo único del lenguaje:

```bash
deimon formatear ejemplos/hola.dmn              # lo reescribe
deimon formatear ejemplos/hola.dmn --comprobar  # solo comprueba, no escribe
```

Con `--comprobar` no cambia nada: dice qué líneas cambiarían y termina con
código 1 si el archivo no está formateado (0 si ya lo está). Formatear dos
veces el mismo archivo da exactamente el mismo resultado.

El estilo es: sangría de 4 espacios por nivel, `{` al final de la línea que
abre el bloque, `}` sola o seguida de ` sino {` / ` sino si condicion {`, un
espacio alrededor de los operadores binarios y de `=`, sin espacios tras `(`
ni `[` ni antes de `)`, `]` o `,`, un espacio tras cada `,`, menos unario pegado
(`-x`), sin espacios al final de línea, como máximo una línea vacía seguida y
una línea vacía entre `funcion`, `ruta` y `prueba`. Los comentarios `#` se
conservan (al final de una línea, separados por dos espacios) y los textos
entre comillas no se tocan. Si el archivo tiene errores de sintaxis, no se
modifica y se muestran como con `deimon comprobar`.

La gramática completa, las reglas de nombres y el estilo están en
[GRAMATICA.md](GRAMATICA.md).

## Guía para la IA

```bash
deimon guia           # referencia completa en texto, para pegarla en el contexto de una IA
deimon guia --json    # la misma referencia en JSON
```

La guía incluye las reglas del lenguaje, las palabras clave, los operadores y su
precedencia, los tipos de valor, cada función integrada con su firma, qué hace y un
ejemplo, las rutas, los datos y las pruebas, las órdenes de la línea de comandos, las
variables de entorno (`DEIMON_PUERTO`, `DEIMON_DATOS`, `DEIMON_HILOS`, `DEIMON_MAX_CONEXIONES`,
`DEIMON_MAX_PETICION`, `DEIMON_TIEMPO_ESPERA`), los perfiles de recursos, la lista «si vienes de otro lenguaje»
y la tabla de códigos de error.

Las funciones integradas salen de una tabla única (`compilador/src/integradas.rs`) que
también usa el generador para validar las llamadas, y una prueba comprueba que todas
aparecen en la guía.

## Pruebas del compilador

Para quien trabaja en el propio `deimon`:

```bash
cd compilador
mkdir -p target/tmp
TMPDIR=$PWD/target/tmp cargo test
```

Las pruebas comprueban el lexer, el parser, los ejemplos completos y los
errores de traducción y de ejecución.

