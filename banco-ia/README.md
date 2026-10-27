# Banco de tareas para IAs

Mide cuántas tareas resuelve bien **a la primera** una IA que escribe en Deimon, PHP o Python
sin poder ejecutar su código. Cada fallo de Deimon señala algo que hay que mejorar en el
lenguaje o en su guía (`deimon guia`).

## Estructura

```text
banco-ia/
  tareas/NN-nombre/       enunciado.md y casos/K.entrada + casos/K.esperado (mínimo 4 casos)
  soluciones/<ronda>/     deimon/*.dmn, php/*.php y python/*.py de cada ronda
  soluciones/referencia/  soluciones en Deimon escritas a mano (prueban que se puede)
  herramientas/crear_tareas.py     crea las 40 tareas; la salida esperada la calcula la referencia en Python
  herramientas/comprobar_tareas.py comprueba el banco (casos, ejemplo en el enunciado, Deimon = Python)
  medir.sh                ejecuta una ronda y cuenta los aciertos
```

Cada tarea tiene varios casos. El **caso 1** es el ejemplo visible (está copiado dentro de
`enunciado.md`, con su entrada y su salida); los demás (2, 3...) son **ocultos**: la IA no los ve y
cubren casos límite (entrada vacía, un solo elemento, empates, negativos, números grandes, acentos
y ñ...). Una tarea cuenta como «bien» solo si **todos** sus casos dan la salida exacta.

## Cómo se mide

```bash
python3 herramientas/crear_tareas.py     # (re)crea tareas/
python3 herramientas/comprobar_tareas.py # el banco está sano (referencia de Deimon = referencia de Python)
./medir.sh soluciones/ronda3             # aciertos por tarea y lenguaje + casos ocultos fallados
```

`medir.sh` usa el compilador ya construido en `../compilador/target/release/deimon`; con la
variable `DEIMON=/ruta/a/deimon ./medir.sh ...` se puede usar otro. Cada celda dice `bien` o
`MAL fallados/casos`, y al final salen las tareas correctas por lenguaje y cuántos casos ocultos
fallaron en total.

Para una ronda nueva, se pide a una IA sin contexto previo que lea los enunciados y escriba las
soluciones en `soluciones/<ronda>/<lenguaje>/` **sin ejecutar nada**, con el nombre
`NN-nombre.dmn`, `.php` o `.py`. En Deimon solo puede leer `deimon guia`. No puede ver `casos/`
(salvo el ejemplo del enunciado) ni otras soluciones.

Las 40 tareas: 01-10 las originales (suma, FizzBuzz, paréntesis, palabras frecuentes, romanos,
ventas, personas, líneas con errores, matriz en espiral, inventario). 11-40, más difíciles:
CSV con comillas, plantilla, justificar texto, palabras con acentos, mapas anidados, agrupar y
ordenar, pila y cola, camino en cuadrícula, islas, factorial/Fibonacci/producto de enteros
grandes, IVA con redondeo exacto, fechas (días entre, día de la semana, sumar días), cola de
banco, juego de la vida, robot, calculadora con precedencia, JSON pequeño, validar formatos,
líneas mal formadas, permutaciones, torres de Hanoi, inventario con informe, notas de alumnos,
carrito con descuentos, compresión por repeticiones y anagramas.

Esta es la fase 1 del punto 2.4 de `docs/PLAN.md` («100 tareas, varias IAs, webs completas»); aún
faltan tareas hasta 100, más IAs y webs completas.

## Resultados (24-oct-2026, IA: Claude Haiku)

| Ronda | Deimon | PHP | Python | Qué pasó |
|---|---|---|---|---|
| 1 | 7/10 | 10/10 | 10/10 | Los 3 fallos de Deimon tenían la misma causa: usar `recortar` sin escribir `usar modulo "texto"`. La lógica estaba bien. |
| 1, medida otra vez tras el arreglo | 10/10 | 10/10 | 10/10 | Mismas soluciones. Arreglo: los módulos que no necesitan nada se cargan solos. |
| 2 (IA nueva, guía corregida) | 10/10 | — | — | Sin fallos. |

Ronda 3: pendiente (40 tareas con casos ocultos).

Lo que las IAs dijeron que más les costó en Deimon: leer la entrada estándar (no estaba
explicado; ahora la guía dice cómo hacerlo con `pedir_texto("")`) y no saber cómo se comportan
algunas funciones sin poder probarlas.

Límites honestos: son 10 tareas cortas y una sola IA. PHP y Python los conoce de memoria; Deimon
lo aprende solo con la guía. El siguiente paso es añadir tareas más largas (webs con pruebas,
programas con varias piezas) y medir también con otras IAs.
