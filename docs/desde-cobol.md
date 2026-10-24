# Pasar de COBOL a Deimon

[Volver al README](../README.md)

`deimon desde-cobol` traduce un programa COBOL **sencillo** a Deimon. Con
`--comprobar` **demuestra que hace lo mismo**: compila el COBOL con `cobc`
(GnuCOBOL), ejecuta los dos programas y compara lo que escriben y su código de salida.

```bash
deimon desde-cobol nomina.cob                 # crea nomina.dmn
deimon desde-cobol nomina.cob --comprobar     # además, compara con GnuCOBOL
deimon desde-cobol nomina.cob --salida otra.dmn
```

Resultado de `--comprobar`:

```text
✓ Traducido: nomina.cob → nomina.dmn
✓ Comprobado: COBOL y Deimon escriben exactamente lo mismo (11 líneas) y terminan con el mismo código (0).
```

Si hay diferencias, se muestra la primera línea distinta (`COBOL:` frente a `Deimon:`).
Para `--comprobar` hace falta tener GnuCOBOL instalado (la orden `cobc`).

Los ejemplos están en `ejemplos-cobol/`: `hola`, `nomina` (decimales), `tabla`
(PERFORM VARYING), `intereses` (COMPUTE) y `clasificar` (EVALUATE).

## Qué se entiende

- Formato fijo (columnas 8 a 72, `*` o `/` en la columna 7 es comentario) y formato libre.
- `WORKING-STORAGE SECTION` con niveles 01 y 77 y `PIC 9(n)`, `S9(n)`, `9(n)V9(m)`, `X(n)`,
  con `VALUE` (número, texto, `SPACES`, `ZEROS`). Los grupos (un 01 con campos 05 dentro) se
  aceptan, pero se usan campo a campo.
- `DISPLAY` (varios elementos, `WITH NO ADVANCING`), `MOVE`, `ADD`, `SUBTRACT`, `MULTIPLY`,
  `DIVIDE` (con `GIVING`, `REMAINDER` y `ROUNDED`), `COMPUTE` con `+ - * /` y paréntesis.
- `IF / ELSE / END-IF` con `=`, `>`, `<`, `>=`, `<=`, `NOT`, `AND`, `OR`, `EQUAL TO`,
  `GREATER THAN`, `LESS THAN`, `IS POSITIVE/NEGATIVE/ZERO`.
- `EVALUATE TRUE` y `EVALUATE dato` con `WHEN`, `THRU` y `OTHER`.
- Párrafos y `PERFORM`: de un párrafo, `THRU`, `n TIMES`, `UNTIL`, `VARYING ... FROM ... BY ... UNTIL`
  (con párrafo o en línea con `END-PERFORM`). `STOP RUN`, `GOBACK`, `EXIT`, `CONTINUE`.

## Cómo queda igual que en COBOL

El Deimon generado empieza con unas funciones ayudantes (`cb_...`) y guarda los números como
enteros escalados, así que las sumas, restas y multiplicaciones son exactas.

- `PIC 9(5)` con 42 se muestra `00042`.
- Con signo (`S9`) se muestra `+00042` o `-00042`.
- Con decimales (`9(3)V99`) se muestra con el punto: `012.50`.
- `X(n)` se rellena con espacios a la derecha.
- Al guardar se trunca: 1234 en un `PIC 9(3)` deja `234`; los decimales sobrantes se cortan
  (o se redondean con `ROUNDED`); un dato sin signo guarda el valor sin el signo.
- Un texto movido a un `X(n)` más corto se corta por la derecha.

Cada `PERFORM` de un párrafo copia ese párrafo en el sitio (las funciones de Deimon no ven las
variables de fuera), y todo el programa va dentro de una función `principal()`.

## Qué NO se entiende (da un error claro)

Si algo no está soportado, la orden se detiene, **no crea ningún archivo** y dice la línea y la
instrucción, por ejemplo:

```text
Error: el COBOL no se pudo traducir. No se ha creado ningún archivo.
línea 9: la instrucción GO no está soportada todavía.
    GO TO FIN.
```

No están soportados: `GO TO`, `ACCEPT`, archivos (`OPEN`, `READ`, `WRITE`), `CALL`, `STRING`,
`INSPECT`, `INITIALIZE`, `SET`, `SEARCH`, `ON SIZE ERROR`, `FUNCTION`, la potencia `**`,
`OCCURS`, `REDEFINES`, `COMP`/`BINARY`, máscaras con edición (`ZZ9.99`), nivel 88,
`SECTION` en el procedimiento, `PERFORM ... AFTER`, `WITH TEST AFTER`, `EXIT PERFORM`,
comparar textos con `<` o `>`, condiciones abreviadas (`A > B OR C`) y más de 15 cifras en un dato.

## Diferencias que conviene saber

- Las divisiones se calculan con 8 decimales y se truncan al guardar. Dividir entre cero detiene
  el programa traducido (COBOL seguiría sin avisar). La orden lo avisa al traducir.
- Los textos con acentos cuentan caracteres, no bytes, al rellenar o cortar un `X(n)`.
