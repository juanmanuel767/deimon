# Plan: Deimon para programar de todo

[Volver al README](../README.md)

Objetivo: que una IA pueda programar cualquier cosa en Deimon sin equivocarse. Se trabaja en
este orden y cada punto se da por hecho solo con pruebas que lo demuestren. Cuando un punto se
termina, se marca con HECHO y la fecha.

## 1. Lenguaje
1. HECHO (24-oct-2026, `ordenar_por(lista, ["-edad", "nombre"])`, tests/plan_lenguaje.rs) Ordenar con reglas propias (por edad y luego por nombre).
2. HECHO (24-oct-2026, `sea f = doble`, `f(x)`, `filtrar`, `transformar`, `ordenar_por(lista, f)`; los tres motores) Funciones como valores (pasar una función a otra) para filtrar, transformar y ordenar.
3. HECHO (26-oct-2026, `tipo Persona { nombre: texto }`, `Persona({...})`, tests/plan_lenguaje.rs) Tipos propios con nombre (como «Persona» con nombre y edad) que avisen si falta un campo.
4. HECHO (26-oct-2026, `9007199254740993`, factorial de 30 exacto en los dos motores; con NaN las comparaciones dan falso; tests/plan_lenguaje.rs) Números enteros exactos y grandes.
5. HECHO (26-oct-2026, `f"Hola {nombre}"`, `"""varias líneas"""`, tests/plan_lenguaje.rs) Textos de varias líneas y textos con variables dentro: `"Hola {nombre}"`.
6. HECHO (26-oct-2026, tipo devuelto por las funciones propias, tipos de los argumentos que entran en ellas, tabla de argumentos de integradas e índices; tests/plan_lenguaje.rs) Comprobar tipos antes de ejecutar en todo el programa.

## 2. Para que una IA no falle
1. HECHO (26-oct-2026, cada ejemplo de biblioteca/*/funciones.txt que se ejecuta anuncia su resultado con `# ...` y la prueba lo exige; las excepciones están en SIN_RESULTADO, tests/ejemplos_documentados.rs) Ejemplos ejecutables en la guía para cada función de los módulos.
2. `deimon comprobar --estricto` que avise de todo lo dudoso.
3. Más arreglos automáticos con `--arreglar`.
4. Banco de pruebas más grande: 100 tareas, varias IAs, webs completas.

## 3. Biblioteca
1. Leer la entrada y archivos de forma sencilla (`leer_lineas()`).
2. Fechas y horas locales completas.
3. Cifrado: firmas, huellas (hash) y azar seguro.
4. Comprimir, XML y CSV con comillas.
5. APIs con cabeceras y WebSockets.
6. PostgreSQL.

## 4. Varias cosas a la vez
1. Tareas en paralelo y esperas sin bloquear.
2. Temporizadores y tareas programadas.

## 5. Gráficos y juegos
1. Ventanas, dibujos, imágenes, ratón y sonido.
2. Juegos 2D.
3. Pantallas nativas completas en el móvil.

## 6. Sistema operativo propio (pc-sin-sistema)
1. Reloj e interrupciones del teclado.
2. Disco y archivos.
3. Varias tareas y memoria protegida.
4. Arrancar en PCs modernos (UEFI) y red.

## 7. Velocidad y destinos
1. Más velocidad en cálculos pesados.
2. iPhone (necesita un Mac), WebAssembly y Mac.

## 8. Herramientas
1. Depurador paso a paso.
2. Gestor de paquetes con versiones.
3. Medir qué parte del programa tarda más.
