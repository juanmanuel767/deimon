# Banco de tareas para IAs

Mide cuántas tareas resuelve bien **a la primera** una IA que escribe en Deimon, PHP o Python
sin poder ejecutar su código. Cada fallo de Deimon señala algo que hay que mejorar en el
lenguaje o en su guía (`deimon guia`).

## Estructura

```text
banco-ia/
  tareas/NN-nombre/       enunciado.md, entrada.txt y esperado.txt de cada tarea
  soluciones/<ronda>/     deimon/*.dmn, php/*.php y python/*.py de cada ronda
  soluciones/referencia/  soluciones en Deimon escritas a mano (prueban que se puede)
  herramientas/crear_tareas.py   crea las tareas; la salida esperada sale de una solución de referencia
  medir.sh                ejecuta una ronda y cuenta los aciertos
```

## Cómo se mide

```bash
python3 herramientas/crear_tareas.py     # (re)crea tareas/
./medir.sh soluciones/ronda1             # tabla de aciertos por tarea y lenguaje
```

Para una ronda nueva, se pide a una IA sin contexto previo que lea los enunciados y escriba las
soluciones en `soluciones/<ronda>/<lenguaje>/` **sin ejecutar nada**. En Deimon solo puede leer
`deimon guia`. No puede ver `esperado.txt` ni otras soluciones.

Las 10 tareas: suma, FizzBuzz, paréntesis equilibrados, palabras más frecuentes, números romanos,
ventas con decimales, ordenar personas, líneas con errores, matriz en espiral e inventario con
órdenes.

## Resultados (24-oct-2026, IA: Claude Haiku)

| Ronda | Deimon | PHP | Python | Qué pasó |
|---|---|---|---|---|
| 1 | 7/10 | 10/10 | 10/10 | Los 3 fallos de Deimon tenían la misma causa: usar `recortar` sin escribir `usar modulo "texto"`. La lógica estaba bien. |
| 1, medida otra vez tras el arreglo | 10/10 | 10/10 | 10/10 | Mismas soluciones. Arreglo: los módulos que no necesitan nada se cargan solos. |
| 2 (IA nueva, guía corregida) | 10/10 | — | — | Sin fallos. |

Lo que las IAs dijeron que más les costó en Deimon: leer la entrada estándar (no estaba
explicado; ahora la guía dice cómo hacerlo con `pedir_texto("")`) y no saber cómo se comportan
algunas funciones sin poder probarlas.

Límites honestos: son 10 tareas cortas y una sola IA. PHP y Python los conoce de memoria; Deimon
lo aprende solo con la guía. El siguiente paso es añadir tareas más largas (webs con pruebas,
programas con varias piezas) y medir también con otras IAs.
