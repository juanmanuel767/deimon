# Analizar un registro (log) con expresiones regulares

Cada línea de la entrada es una línea de un registro. Una línea es **válida** si toda ella tiene
exactamente esta forma (y no tiene nada más, ni espacios al principio o al final):

```
AAAA-MM-DD HH:MM:SS [NIVEL] IP METODO RUTA ESTADO
```

con: fecha de 4-2-2 cifras y hora de 2-2-2 cifras (solo se comprueba la forma, no que existan);
NIVEL una de las palabras `INFO`, `WARN` o `ERROR`; IP cuatro grupos de 1 a 3 cifras separados por
puntos; METODO uno de `GET`, `POST`, `PUT`, `DELETE`; RUTA un texto que empieza por «/» y no tiene
espacios; ESTADO tres cifras. Los elementos van separados por un único espacio. Las líneas que no
cumplan (también las vacías) son **inválidas**. Escribe, con solo las líneas válidas:

```
validas: N
invalidas: M
ERROR: a
INFO: b
WARN: c
estados: 200=3 404=1
hora 09: 2
ruta mas pedida: /x (3)
```

- `ERROR`, `INFO` y `WARN` (en ese orden alfabético): cuántas líneas válidas tiene cada nivel
  (0 si ninguna);
- `estados:` los estados que aparecen con cuántas veces (`estado=veces`), de menor a mayor estado
  (como números) y separados por un espacio; si no hay líneas válidas, `estados: ninguno`;
- una línea `hora HH: n` por cada hora (00 a 23, dos cifras) en que hay alguna línea válida, de
  menor a mayor hora, con cuántas líneas válidas hay;
- `ruta mas pedida: RUTA (veces)`, la ruta con más líneas válidas (empate: la primera por orden
  alfabético de código de caracteres); si no hay líneas válidas, `ruta mas pedida: ninguna`.

## Ejemplo

Entrada:

```
2026-10-27 09:15:01 [INFO] 10.0.0.1 GET /inicio 200
2026-10-27 09:15:07 [ERROR] 10.0.0.2 POST /pago 500
basura
2026-10-27 10:00:00 [WARN] 10.0.0.1 GET /inicio 404
2026-10-27 10:30:30 [INFO] 192.168.1.20 DELETE /a/b?x=1 200
```

Salida:

```
validas: 4
invalidas: 1
ERROR: 1
INFO: 2
WARN: 1
estados: 200=2 404=1 500=1
hora 09: 2
hora 10: 2
ruta mas pedida: /inicio (2)
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
