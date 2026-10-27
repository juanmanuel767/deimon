# CSV con comillas simples

Cada línea (nunca vacía) es una fila CSV con los campos separados por comas. Un campo puede ir
entre comillas simples (') y entonces las comas de dentro no separan campos; las comillas no
forman parte del valor. Las comillas siempre abarcan el campo entero (no hay comillas dentro de
un campo sin comillas) y siempre están cerradas. Los espacios se conservan. Un campo puede estar
vacío (por ejemplo "a,,b" tiene 3 campos, y "a,b," también).

Para cada fila escribe "N campos: [campo1][campo2]..." con N el número de campos y cada valor
entre corchetes, todo pegado. (Siempre se escribe la palabra "campos".)

## Ejemplo

Entrada:

```
ana,'Madrid, España',30
x,y
```

Salida:

```
3 campos: [ana][Madrid, España][30]
2 campos: [x][y]
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
