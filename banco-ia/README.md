# Banco de tareas para IAs

Mide cuántas tareas resuelve bien **a la primera** una IA que escribe en Deimon, PHP o Python
sin poder ejecutar su código. Cada fallo de Deimon señala algo que hay que mejorar en el
lenguaje o en su guía (`deimon guia`).

## Estructura

```text
banco-ia/
  tareas/NN-nombre/       enunciado.md y casos/K.entrada + casos/K.esperado (mínimo 4 casos)
                          (las webs 51-55: casos/K.peticiones.json + casos/K.esperado.json)
  soluciones/<ronda>/     deimon/*.dmn, php/*.php y python/*.py de cada ronda
  soluciones/referencia/  soluciones en Deimon escritas a mano (prueban que se puede)
  soluciones/referencia/python/    solo las webs 51-55 (segunda referencia, independiente de la de Deimon)
  herramientas/crear_tareas.py     crea las 55 tareas; la salida esperada de 01-50 la calcula la referencia en Python
  herramientas/tareas_web.py       las 5 webs (51-55): enunciados, contrato y respuestas esperadas escritas a mano
  herramientas/medir_web.py        arnés de las webs: arranca el servidor, hace las peticiones, compara y lo mata siempre
  herramientas/comprobar_tareas.py comprueba el banco (casos, ejemplo en el enunciado, Deimon = Python, webs con 2 referencias)
  medir.sh                ejecuta una ronda y cuenta los aciertos (tabla general + una línea por bloque)
```

Cada tarea tiene varios casos. El **caso 1** es el ejemplo visible (está copiado dentro de
`enunciado.md`, con su entrada y su salida); los demás (2, 3...) son **ocultos**: la IA no los ve y
cubren casos límite (entrada vacía, un solo elemento, empates, negativos, números grandes, acentos
y ñ...). Una tarea cuenta como «bien» solo si **todos** sus casos dan la salida exacta.

## Cómo se mide

```bash
python3 herramientas/crear_tareas.py     # (re)crea tareas/
python3 herramientas/comprobar_tareas.py # el banco está sano (referencia de Deimon = referencia de Python; webs con 2 referencias)
./medir.sh soluciones/ronda3             # aciertos por tarea y lenguaje + casos ocultos fallados + una línea por bloque
```

`medir.sh` usa el compilador ya construido en `../compilador/target/release/deimon`; con la
variable `DEIMON=/ruta/a/deimon ./medir.sh ...` se puede usar otro. Cada celda dice `bien` o
`MAL fallados/casos`, y al final salen las tareas correctas por lenguaje y cuántos casos ocultos
fallaron en total.

Para una ronda nueva, se pide a una IA sin contexto previo que lea los enunciados y escriba las
soluciones en `soluciones/<ronda>/<lenguaje>/` **sin ejecutar nada**, con el nombre
`NN-nombre.dmn`, `.php` o `.py`. En Deimon solo puede leer `deimon guia`. No puede ver `casos/`
(salvo el ejemplo del enunciado) ni otras soluciones.

Las tareas 01-40: 01-10 las originales (suma, FizzBuzz, paréntesis, palabras frecuentes, romanos,
ventas, personas, líneas con errores, matriz en espiral, inventario). 11-40, más difíciles:
CSV con comillas, plantilla, justificar texto, palabras con acentos, mapas anidados, agrupar y
ordenar, pila y cola, camino en cuadrícula, islas, factorial/Fibonacci/producto de enteros
grandes, IVA con redondeo exacto, fechas (días entre, día de la semana, sumar días), cola de
banco, juego de la vida, robot, calculadora con precedencia, JSON pequeño, validar formatos,
líneas mal formadas, permutaciones, torres de Hanoi, inventario con informe, notas de alumnos,
carrito con descuentos, compresión por repeticiones y anagramas.

## Bloque de especialidades (tareas 41-55)

Las 40 tareas generales miden lo mismo en los tres lenguajes. Este bloque de 15 deja que cada uno compita
en lo que mejor sabe hacer; `medir.sh` da su propia línea por bloque (general 01-40, PHP 41-45,
Python 46-50, Deimon 51-55) y se identifica por el número y por el nombre (`41-php-...`, `46-py-...`, `51-dmn-...`).
Los enunciados dicen exactamente qué comportamiento de PHP o de Python se copia (por ejemplo «como
`number_format` de PHP»), y las referencias de las tareas 41-50 se comprobaron también con PHP real.

| Bloque | Tarea | Qué mide |
|---|---|---|
| Fuerte de PHP | 41-php-tabla-html | tabla HTML con datos escapados (`& < > " '`) |
| | 42-php-numeros | `number_format` y `sprintf` con relleno y separadores |
| | 43-php-fechas | formato de fechas (sin zona horaria), nombres en español |
| | 44-php-texto | `ucfirst`, `ucwords`, `wordwrap`, `str_pad` |
| | 45-php-listas-ordenadas | `asort`, `arsort`, `ksort`, `krsort` con empates definidos |
| Fuerte de Python | 46-py-contador | tipo `Counter.most_common` con desempate por primera aparición |
| | 47-py-combinaciones | `product` y `combinations` en orden |
| | 48-py-conjuntos | unión, intersección, diferencias, subconjunto |
| | 49-py-registro-regex | expresiones regulares sobre un registro (log) |
| | 50-py-json-anidado | JSON de entrada anidado |
| Fuerte de Deimon (webs) | 51-dmn-api-json | API JSON con GET/POST y códigos 201, 400, 404 |
| | 52-dmn-formulario | formulario POST con validación y errores escapados |
| | 53-dmn-sqlite-notas | notas y contador de visitas con SQLite (sobreviven a un reinicio) |
| | 54-dmn-sesion-galleta | sesión por galleta: entrar, quién soy, contar |
| | 55-dmn-rutas-cabeceras | rutas con parámetros, cabeceras y Content-Type correcto |

### Contrato de las webs (51-55)

Es el mismo para PHP, Python y Deimon y está copiado en cada `enunciado.md`:

- Un único archivo que es un servidor HTTP en `127.0.0.1`, en el puerto de la variable de entorno `PUERTO`.
  Se arranca con `php -S 127.0.0.1:$PUERTO archivo.php` (el archivo es el enrutador), `python3 -I archivo.py`
  (solo biblioteca estándar) o `deimon ejecutar archivo.dmn`.
- **Cómo recibe el puerto cada lenguaje.** PHP lo recibe en la línea de órdenes y Python lee `PUERTO`. Deimon
  no tiene ninguna función para leer variables de entorno, pero `servir(...)` obedece `DEIMON_PUERTO`: el arnés pone las dos variables
  con el mismo valor y el programa en Deimon termina en `servir(8080)`. Lo mismo con los datos: el arnés pone
  `DATOS` y `DEIMON_DATOS` (carpeta de datos de las colecciones de Deimon) y arranca el servidor con esa
  carpeta como carpeta de trabajo (así `datos.db` cae dentro en los tres lenguajes).
- Cada caso empieza con una carpeta `DATOS` temporal vacía; el arnés espera hasta 10 s a que el puerto
  responda, lanza las peticiones una a una, compara código, Content-Type y cuerpo (exacto, o JSON comparado como
  dato) y mata el servidor siempre; máximo 60 s por caso. Cada «cliente» guarda y devuelve las galletas como un navegador.
- Formato de un caso: `casos/K.peticiones.json` (lista de `{cliente, metodo, ruta, cabeceras, cuerpo}` o
  `{"accion": "reiniciar"}`, que para el servidor y lo arranca otra vez con la misma carpeta `DATOS`) y
  `casos/K.esperado.json` (por petición `{codigo, tipo, cuerpo | json}`). El caso 1 está en el enunciado; los demás
  (ocultos) cubren límites. Las respuestas esperadas están escritas a mano en `tareas_web.py` y las
  validan **dos referencias independientes** (Python en `soluciones/referencia/python/` y Deimon en
  `soluciones/referencia/deimon/`).
- Medir una sola web: `python3 herramientas/medir_web.py <deimon|php|python> <archivo> tareas/<tarea> -v`.

### Lo que Deimon no puede hacer hoy (por eso no tiene referencia en 51 y 55)

La referencia de Deimon pasa 3 de las 5 webs (52, 53, 54). `comprobar_tareas.py` lo lista en
`IMPOSIBLE_EN_DEIMON` y avisará si algún día se escribe la referencia. Las dos que faltan:

- **51 (API JSON) y 55 (Content-Type):** una `ruta` solo responde `200 text/html; charset=utf-8` (o `303` si
  redirige, o el `404` propio de una ruta que no existe); no hay función para elegir el código (`201`, `400`,
  `404` con cuerpo propio) ni el `Content-Type` (`application/json`, `text/plain`, `text/css`), y solo se
  aceptan los métodos GET y POST (PUT y DELETE reciben `405`). Ejemplo mínimo:
  `ruta "/api/tareas" { devolver "[]" }` responde `200 text/html`, nunca `application/json`.

Otras carencias menores encontradas al escribir las referencias (no impiden resolver las tareas):

- `campo()` y los parámetros de ruta ya vienen escapados (la comilla simple sale `&#39;`, no el `&#039;` de
  PHP), y para contar caracteres reales o guardar el texto original en la base hay que deshacer el escape a mano.
- `desde_json` convierte los enteros de más de 64 bits en decimales (`{"n": 99999999999999999999}` da
  `100000000000000000000`), aunque los enteros de Deimon sí son exactos.
- No hay una función como `wordwrap` (la referencia de 44 parte las líneas a mano). `primera_mayuscula` pasa a
  mayúscula también las letras con acento (PHP `ucfirst` no lo hace), así que no vale tal cual para la tarea 44.

Esta es la fase 1 del punto 2.4 de `docs/PLAN.md` («100 tareas, varias IAs, webs completas»); aún
faltan tareas hasta 100, más IAs y webs completas.

## Resultados (24-oct-2026, IA: Claude Haiku)

| Ronda | Deimon | PHP | Python | Qué pasó |
|---|---|---|---|---|
| 1 | 7/10 | 10/10 | 10/10 | Los 3 fallos de Deimon tenían la misma causa: usar `recortar` sin escribir `usar modulo "texto"`. La lógica estaba bien. |
| 1, medida otra vez tras el arreglo | 10/10 | 10/10 | 10/10 | Mismas soluciones. Arreglo: los módulos que no necesitan nada se cargan solos. |
| 2 (IA nueva, guía corregida) | 10/10 | — | — | Sin fallos. |

## Ronda 3 (27-oct-2026, IA: Claude Haiku, 40 tareas con casos ocultos, sin ejecutar nada)

Tres IAs sin contexto previo, una por lenguaje, en una carpeta que solo tenía los enunciados (y la
guía de Deimon). La tarea 37 queda anulada: su ejemplo estaba mal (medias ×10) y se corrigió
después.

| | Deimon | PHP | Python |
|---|---|---|---|
| Tareas bien (de 39) | 28 | 38 | 39 |
| Casos ocultos fallados | 47/242 | 6/242 | 0/242 |

Por qué falló Deimon (11 tareas):
- 5 tareas (24, 25, 26, 29, 32): la IA usó `y` como nombre de variable (coordenada, año) y en
  Deimon es la palabra reservada «and». Es una trampa del lenguaje, no de la IA.
- 5 tareas (06, 10, 16, 33, 36): recorrió `listar_claves(m)` creyendo que sale en orden
  alfabético; sale en el orden en que se añadieron. Falta decirlo claro en la guía.
- 1 tarea (28): escribió `in` en vez de `en` (costumbre de Python); `deimon comprobar` lo habría
  visto.

Soluciones en `soluciones/ronda3-haiku/`.

Lo que las IAs dijeron que más les costó en Deimon: leer la entrada estándar (no estaba
explicado; ahora la guía dice cómo hacerlo con `pedir_texto("")`) y no saber cómo se comportan
algunas funciones sin poder probarlas.

Límites honestos: son 10 tareas cortas y una sola IA. PHP y Python los conoce de memoria; Deimon
lo aprende solo con la guía. El siguiente paso es añadir tareas más largas (webs con pruebas,
programas con varias piezas) y medir también con otras IAs.
