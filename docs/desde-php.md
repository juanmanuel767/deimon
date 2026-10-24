# Pasar de PHP a Deimon

[Volver al README](../README.md)

`deimon desde-php` traduce un programa de PHP a Deimon y, con `--comprobar`,
**demuestra que hace lo mismo**: ejecuta el PHP original y el Deimon traducido y
compara, carácter a carácter, lo que escriben.

```bash
deimon desde-php tienda.php                 # crea tienda.dmn
deimon desde-php tienda.php --comprobar     # además, compara las dos salidas
deimon desde-php tienda.php --salida otra.dmn
```

Resultado de `--comprobar`:

```text
✓ Traducido: tienda.php → tienda.dmn
  Todo el PHP se pudo traducir.
✓ Comprobado: PHP y Deimon escriben exactamente lo mismo (7 líneas).
```

## Sitios web

Si le das una **carpeta** (o una página que usa `$_GET`, `$_POST`, `header()` o
HTML completo), traduce el sitio entero: cada `archivo.php` pasa a ser una página
`ruta "/archivo.php"` (e `index.php` responde también en `/`) y al final pone
`servir(8000)`.

```bash
deimon desde-php mi-sitio/ --comprobar \
    --visitar "/index.php?buscar=ta" \
    --enviar "/contacto.php" "correo=a%40b.com&mensaje=hola"
```

Las visitas se hacen en orden y **guardando las galletas como un navegador**, así
que se pueden comprobar recorridos con sesión (añadir al carrito, verlo, vaciarlo).

Con `--comprobar` arranca **los dos servidores** (el de PHP con `php -S` y el de
Deimon), visita las mismas páginas en los dos (todas sin datos, más las que
indiques con `--visitar` y `--enviar`) y compara el código de respuesta, la
redirección y el HTML:

```text
✓ Traducido: sitio → sitio.dmn (sitio web: 4 páginas)
  ✓ visitar /index.php?buscar=ta
  ✓ enviar /contacto.php (correo=a%40b.com&mensaje=hola)
✓ Comprobado: las 9 páginas responden exactamente igual en PHP y en Deimon.
```

| PHP | Deimon |
|---|---|
| `echo` y el HTML de la página | `escribir(...)` (dentro de una página, va a la página) |
| `$_GET['x']`, `$_POST['x']`, `$_REQUEST['x']` | `campo("x")` (ya protegido como HTML) |
| `$_SERVER['REQUEST_METHOD'] === 'POST'` | `es_envio()` |
| `isset($x)` / `empty($x)` | `x != nada` / `(x == nada o x == "")` |
| `header("Location: gracias.php"); exit;` | `devolver redirigir("/gracias.php")` |
| `exit;` / `die("mensaje");` | `devolver ""` / `escribir("mensaje")` y `devolver ""` |
| `htmlspecialchars($x)` | `proteger_html(x)` (o nada, si `x` ya viene de `campo()`) |
| `session_start()` | se quita (Deimon abre la sesión sola cuando hace falta) |
| `$_SESSION['x'] = v` / `$_SESSION['x']` | `guardar_en_sesion("x", v)` / `leer_de_sesion("x")` |
| `$_SESSION['carrito'][] = v` | `agregar_a_sesion("carrito", v)` |
| `unset($_SESSION['x'])` / `session_destroy()` | `borrar_de_sesion("x")` / `vaciar_sesion()` |

**Más seguro que el original:** si el PHP escribe lo que llega de un formulario
sin `htmlspecialchars` (un fallo de seguridad típico), en Deimon sale protegido
igualmente; `--comprobar` lo mostrará como una diferencia, y es a propósito.

Ejemplos: `ejemplos-php/carrito/` (carrito con `$_SESSION`, comprobado paso a paso),
`ejemplos-php/menu.php` (`switch`) y `ejemplos-php/sitio/` (portada con buscador, contacto con formulario y
redirección, saludo y página de gracias) y su traducción `ejemplos-php/sitio.dmn`.

## Qué traduce

| PHP | Deimon |
|---|---|
| `$x = 1;` (primera vez) | `sea x = 1` |
| `$x += 2;` `$x++;` `$t .= "a";` | `x = x + 2` `x = x + 1` `t = t + "a"` |
| `echo "Hola\n";` | `mostrar("Hola")` |
| `echo $a, $b;` | `escribir(a)` `escribir(b)` |
| `"Hola $nombre"` / `"{$nombre}"` | `"Hola " + nombre` |
| `$a . $b` | `a + b` (o `a_texto(a) + b` si los dos pueden ser números) |
| `if / elseif / else` | `si / sino si / sino` |
| `while`, `for (...;...;...)` | `mientras` |
| `foreach ($lista as $x)` | `para x en lista` |
| `function f($a) { return ...; }` | `funcion f(a) { devolver ... }` |
| `[1, 2]`, `array(1, 2)`, `$l[] = 3`, `$l[0] = 5` | `[1, 2]`, `agregar`, `l[0] = 5` |
| `&&` `\|\|` `!` `===` `!==` | `y` `o` `no` `==` `!=` |
| `['a' => 1]`, `foreach ($m as $k => $v)`, `unset($m['k'])`, `array_key_exists`, `array_keys` | `{"a": 1}`, `para k en listar_claves(m)`, `quitar_clave`, `tiene_clave`, `listar_claves` |
| `"{$m['k']}"`, `"$l[0]"` | `"" + m["k"]`, `l[0]` |
| `break`, `continue` (en un `for`, el paso se hace antes de `continuar`) | `romper`, `continuar` |
| `$c ? $a : $b`, `$a ?: $b`, `$a ?? $b` | `elegir(c, a, b)` (solo calcula la opción elegida) |
| `sprintf`, `printf` (`%s %d %.2f %05d %-8s %x %b %e %%`, `%1$s`) | `formatear_texto(plantilla, [valores])` del módulo `texto` |
| `in_array` `sort` `rsort` `array_reverse` `strrev` `array_sum` `range` `array_merge` `substr` `array_slice` | `incluye` `ordenar` `invertir` `sumar_lista` `rango` `+` `cortar` |
| `rand` `mt_rand` `random_int` `json_encode` `json_decode($t, true)` | `entero_al_azar` (módulo `matematicas`), `a_json` y `desde_json` (módulo `json`) |
| `$_COOKIE['x']`, `setcookie('x', $v, time() + 86400 * 30)`, `setcookie('x', '', time() - 1)` | `leer_galleta("x")`, `guardar_galleta("x", v, 30)`, `borrar_galleta("x")` |
| `switch` con `case`, `default` y `break` | `si / sino si / sino` (varios `case` seguidos se unen con `o`) |
| `true` `false` `null` `PHP_EOL` | `verdadero` `falso` `nada` `"\n"` |
| HTML fuera de `<?php ?>` y `<?= ... ?>` | `escribir("...")` |
| `strlen` `count` `strval` `floatval` | `medir` `medir` `a_texto` `a_numero` |
| `strtoupper` `strtolower` `trim` `str_contains` `str_starts_with` `str_ends_with` `str_replace` `explode` `implode` `str_repeat` `ucfirst` | módulo `texto` (se añade `usar modulo "texto"` solo) |
| `preg_match` (con o sin `$grupos`), `preg_match_all`, `preg_replace`, `preg_split` con patrón fijo `/.../`, `#...#` o `~...~` (modificadores `i` y `u`) | módulo `patrones`: `buscar_patron` / `coincide_patron`, `buscar_columnas_patron`, `reemplazar_patron`, `partir_patron` (la `i` pasa a `(?i)`) |
| `number_format`, `str_pad` (con `STR_PAD_LEFT`, `STR_PAD_RIGHT`, `STR_PAD_BOTH`) | `formatear_numero` y `rellenar_texto` del módulo `texto` (se rellenan los argumentos que PHP deja por defecto) |
| `print_r`, `var_dump` | `texto_completo` del módulo `texto` (formato legible de `print_r`; `var_dump` se muestra con ese mismo formato) |
| `sqrt` `pow` `abs` `round` `floor` `ceil` `min` `max` `intval` | módulo `matematicas` |
| `date(formato[, ts])`, `time()` | `formatear_fecha(ts o ahora(), formato, zona)` y `suelo(ahora())` del módulo `tiempo`; la zona es la de `date_default_timezone_set('...')` si aparece (si no, `"UTC"`) |
| `mktime(h, i, s, m, d, Y)`, `checkdate(m, d, Y)`, `cal_days_in_month(CAL_GREGORIAN, m, Y)` | `crear_fecha`, `dias_del_mes` y una comprobación con `dias_del_mes` del módulo `tiempo` |
| `strtotime("AAAA-MM-DD...")`, `strtotime("+N day(s)/week(s)/month(s)/year(s)"[, base])` | `leer_fecha` para fechas absolutas y `sumar_tiempo(base o ahora(), N, unidad)` para las relativas (el texto tiene que ser fijo) |

Las fechas de PHP se traducen al módulo `tiempo`. `date()` usa la zona de
`date_default_timezone_set('...')` (o `"UTC"` si no aparece); `mktime` y las
fechas absolutas de `strtotime` se interpretan en UTC, igual que `crear_fecha` y
`leer_fecha`. `strtotime` con textos dinámicos solo traduce fechas absolutas, y
lo avisa. Los formatos de fecha aceptados son `AAAA-MM-DD`,
`AAAA-MM-DD HH:MM[:SS]`, `AAAA-MM-DDTHH:MM:SS[Z o ±HH:MM]` y `DD/MM/AAAA`.

Los nombres de PHP que chocan con palabras de Deimon (`$si`, `$prueba`, una
función `medir`...) se renombran con `_php` al final.

## Errores: try, catch y throw

`try { } catch (Exception $e) { }` pasa a `intentar { } si falla e { }`,
`$e->getMessage()` a `e["mensaje"]` y `throw new Exception(m)` a
`provocar_error(m)`. También vale `catch (Throwable $e)`. Con `finally`, con
varios `catch`, con otros tipos de error o con varios tipos en un `catch`, deja
un aviso `# PENDIENTE`.

## Qué NO traduce (y lo avisa)

Nada se pierde en silencio: cada cosa que no sabe traducir aparece al principio
del archivo como `# PENDIENTE (línea N del PHP): motivo` y en la pantalla.

- Clases y objetos (`class`, `new`, `->`), un `case` de
  `switch` que sigue en el siguiente sin `break`, `break 2`, y `break` dentro
  de un `if` de un `case`.
- Arrays que mezclan valores con clave y sin clave (`[1, 'a' => 2]`).
- `$_FILES` y `move_uploaded_file` (en Deimon: `archivo_subido` y `guardar_subida`) y `header()` que no sea una redirección.
- Expresiones regulares con mirar adelante o atrás (`(?=`, `(?<`), referencias
  (`\1`), escapes como `\A` o `\p{L}`, modificadores distintos de `i` y `u`
  (`m`, `s`, `x`...), un patrón que no es un texto fijo, `preg_replace` con
  límite, `preg_split` con opciones o `preg_match_all` con opciones. En un
  `preg_match` con `$grupos`, el traductor guarda los grupos justo antes de la
  instrucción (no funciona dentro de la condición de un `elseif` ni de un `while`).
- `include`/`require` (se traduce cada archivo y se usa `usar "archivo.dmn"`).

Diferencias sutiles que el traductor avisa: `intval()` con números negativos.
Otras que `--comprobar` detecta: `echo preg_match(...)` escribe `1` en PHP y `verdadero` en Deimon, `echo true` escribe `1` en PHP y `verdadero` en
Deimon, y algunos decimales se muestran con más cifras en Deimon.

Ejemplos: `ejemplos-php/tienda.php`, `fizzbuzz.php`, `dificil.php` (todos dan la
misma salida) y `no-soportado.php` (muestra los avisos).
