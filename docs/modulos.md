# Archivos y módulos

[Volver al README](../README.md)

## Dividir en varios archivos

La instrucción `usar` carga otro archivo `.dmn`. Solo se escribe en el nivel
principal (no dentro de una función, un `si` o un bucle):

```deimon
usar "piezas/calculos.dmn"
mostrar(doble(4))
```

- La ruta es relativa a la carpeta del archivo que contiene el `usar`. Así,
  dentro de `piezas/otra.dmn`, `usar "base.dmn"` busca `piezas/base.dmn`.
- Del archivo usado se cargan sus `funcion`, `ruta` y `prueba` (y, si ese
  archivo usa otros, también los suyos). Sus instrucciones sueltas de nivel
  principal y su `servir` se ignoran.
- Un mismo archivo usado por caminos distintos se carga una sola vez.
- Es un error que la ruta no exista (`E014`), que dos archivos se usen en
  círculo (`E015`, con la cadena `a.dmn → b.dmn → a.dmn`) o que dos archivos
  definan la misma función o la misma ruta (`E016`, diciendo los dos archivos).
- Los errores de un archivo usado dicen su nombre:
  `Error en piezas/calculos.dmn, línea 2: ...`.
- `deimon probar` también ejecuta las pruebas de los archivos usados.

Ejemplo completo: `ejemplos/varios-archivos.dmn` usa `ejemplos/piezas/calculos.dmn`.

## Módulos: capacidades especiales sin complicar el lenguaje

El núcleo de `deimon` es pequeño a propósito. Lo especial (matemáticas, textos,
fechas...) va en **módulos**: se cargan solo cuando un programa los pide.

```deimon
usar modulo "matematicas"
mostrar(raiz(16))   # escribe 4
```

Un módulo es una carpeta con:

```text
saludos/
  modulo.txt      obligatorio: nombre, descripción y lo que necesita
  funciones.txt   opcional: funciones escritas en Rust (lista)
  codigo.rs       opcional: funciones escritas en Rust (código)
  saludos.dmn      opcional: funciones escritas en Deimon (uno o varios .dmn)
```

### modulo.txt

```text
nombre: saludos
descripcion: saludos con una función en Rust y otra en Deimon
requiere:
```

- `nombre` debe coincidir con el nombre de la carpeta.
- `requiere` lista lo que el módulo necesita del sistema: `red`, `archivos` y/o
  `hilos` (vacío si nada). Si el destino de `deimon compilar --destino` o de
  `deimon app` no lo tiene, se avisa antes de compilar.

### Dónde se busca un módulo

`usar modulo "nombre"` busca, en este orden:

1. `modulos/<nombre>/` junto al archivo `.dmn` que lo usa;
2. `~/.deimon/modulos/<nombre>/` (tus módulos para todos los proyectos);
3. la **biblioteca estándar**, que va dentro del propio `deimon`.

Un módulo se incluye una sola vez aunque se use desde varios archivos.
`usar extension "x"` es la forma antigua: da el aviso `E021` y
`deimon comprobar --arreglar` la cambia sola por `usar modulo "x"`.

`deimon modulos` lista los módulos disponibles con sus funciones.

### Compartir módulos

```bash
deimon modulo instalar https://github.com/alguien/modulo-saludos.git
deimon modulo actualizar saludos
deimon modulo quitar saludos
```

- El origen es una dirección `https://` de git o una carpeta de tu ordenador.
  Hace falta tener `git` instalado. Se rechaza `http://` sin `s`.
- Se instala en `~/.deimon/modulos/<nombre>/`; con `--aqui`, en `modulos/<nombre>/`
  junto a tu programa. Antes se comprueba que tiene `modulo.txt` (y `funciones.txt`
  si trae Rust), que el nombre usa solo `a-z`, `0-9` y `_`, y que no es un módulo
  de la biblioteca estándar.
- Si ya está instalado da error: usa `actualizar` (hace `git pull --ff-only` y
  vuelve a validar). `quitar` borra solo la carpeta de ese módulo.
- Un módulo trae código Rust que se ejecuta en tu equipo: instala solo módulos
  de quien confíes.
- La variable `DEIMON_CASA` cambia la carpeta que hace de casa (por defecto `HOME`),
  útil en pruebas.

## Biblioteca estándar

| Módulo | Funciones |
|---|---|
| `matematicas` | `raiz(x)`, `potencia(x, y)`, `absoluto(x)`, `redondear(x)`, `suelo(x)`, `techo(x)`, `minimo(a, b)`, `maximo(a, b)`, `aleatorio()` (entre 0 y 1), `entero_al_azar(minimo, maximo)` (como `rand`), y en Deimon `factorial(n)`, `es_par(n)` |
| `texto` | `mayusculas(t)`, `minusculas(t)`, `recortar(t)`, `contiene(t, buscado)`, `reemplazar(t, de, a)`, `partir(t, separador)` (lista), `empieza_con(t, inicio)`, `termina_con(t, fin)`, `repetir(t, veces)`, `unir(lista, separador)`, `formatear_texto(plantilla, [valores])` (como `sprintf`: `%s`, `%d`, `%.2f`, `%05d`, `%-8s`), `formatear_numero(n, decimales, separador_decimal, separador_miles)` (como `number_format`), `rellenar_texto(t, largo, relleno, lado)` (como `str_pad`, con `"derecha"`, `"izquierda"` o `"ambos"` y contando caracteres), `primera_mayuscula(t)` (como `ucfirst`, también con acentos), `texto_completo(v)` (devuelve el texto, como `print_r`, con listas y listas con claves anidadas) |
| `patrones` | expresiones regulares: `coincide_patron(t, patron)` (verdadero/falso), `buscar_patron(t, patron)` (lista `[todo, grupo1, ...]` o nada), `buscar_todos_patron(t, patron)` (lista de esas listas), `buscar_columnas_patron(t, patron)` (como `PREG_PATTERN_ORDER` de PHP), `reemplazar_patron(t, patron, nuevo)` (con `$1`, `$2`, `${1}` en `nuevo`), `partir_patron(t, patron)`. Ver [Patrones](#patrones-expresiones-regulares) |
| `tiempo` | `ahora()` (segundos desde 1970, pueden llevar decimales), `esperar(milisegundos)`, `fecha(segundos)` (texto `AAAA-MM-DD HH:MM:SS` en UTC), `crear_fecha(año, mes, día, hora, minuto, segundo)` (segundos UTC; error claro si la fecha no existe, como el 31 de febrero), `leer_fecha(texto)` (`AAAA-MM-DD`, `AAAA-MM-DD HH:MM[:SS]`, `AAAA-MM-DDTHH:MM:SS[Z o ±HH:MM]` o `DD/MM/AAAA`; devuelve nada si no se entiende), `formatear_fecha(segundos, formato, zona)` (códigos de `date()` de PHP: `Y m n d j H G i s D l N w z t L y A a g h U F M`, escapando con `\`; `l`, `F` y `M` en español), `sumar_tiempo(segundos, cantidad, unidad)` (`segundos`, `minutos`, `horas`, `dias`, `semanas`, `meses`, `años`), `diferencia_fechas(a, b, unidad)` (entero en esa unidad, `a - b`), `dia_de_la_semana(segundos, zona)` (1 lunes ... 7 domingo), `es_bisiesto(año)`, `dias_del_mes(año, mes)` |
| `json` | `leer_json(texto, "usuario.nombre")` (también `lista[0]`), `listar_claves_json(texto, camino)`, `a_json(valor)`, `desde_json(texto)` (todo el JSON: los objetos pasan a listas con claves) |
| `archivos` | `escribir_archivo(ruta, texto)`, `leer_archivo(ruta)` (nada si no existe), `existe_archivo`, `borrar_archivo`, `listar_archivos(carpeta)` |
| `excel` | `crear_excel("x.xlsx", filas)` (Excel, LibreOffice, Google Sheets), `crear_csv("x.csv", filas)` (con `;`, como Excel en español), `leer_csv("x.csv")` |
| `graficos` | `grafico_barras`, `grafico_lineas`, `grafico_tarta` (título, etiquetas, valores): devuelven una imagen SVG para la página o para `escribir_archivo` |
| `mysql` | `ejecutar_mysql(conexion, orden, parámetros)`, `consultar_mysql(conexion, orden, parámetros)` (lista de filas, cada una una lista con claves) y `ultimo_id_mysql(conexion)`. Habla directamente con MySQL 5.7/8 y MariaDB, sin instalar nada más. La conexión es `"mysql://usuario:clave@servidor:puerto/base"` (con `localhost` usa el enchufe local); con `""` se lee de la variable `DEIMON_MYSQL` para no escribir la contraseña en el programa. Los valores van en la lista (`?`) y se protegen, así no se puede colar SQL |
| `sql` | `ejecutar_sql(base, orden, parámetros)` y `consultar_sql(base, orden, parámetros)` con SQLite; los valores van en la lista de parámetros (`?`), así no se puede colar SQL. Necesita `libsqlite3` (`sudo apt install libsqlite3-dev`) y de momento funciona en Linux |
| `pantallas` | `pantalla(lista)`, `titulo`, `parrafo`, `elemento_lista`, `separador`, `boton(texto, camino)`, `cuadro_texto(nombre, ayuda)`, `cuadro_clave`, `boton_enviar(texto, camino)`: pantallas nativas para apps de Android (ver [apps-y-destinos.md](apps-y-destinos.md)) |
| `pdf` | `crear_pdf(archivo, título, líneas)`: PDF A4 con acentos, ñ y €; parte las líneas largas y añade páginas solo |
| `correo` | `enviar_correo(para, asunto, texto)` (usa el `sendmail` del sistema, como `mail()` de PHP), `es_correo_valido(texto)` |
| `web` | `pedir_web(dirección)` y `enviar_web(dirección, datos)`: hablan con otros sitios por HTTP o HTTPS (usa `curl`) |
| `pagos` | `crear_pago(clave, céntimos, moneda, descripción, dirección_éxito, dirección_cancelar)` devuelve dónde pagar con Stripe; `pago_completado(clave, sesión)` |
| `google` | `direccion_google(id_cliente, dirección_vuelta)` devuelve a dónde mandar a la persona para entrar con Google; `usuario_google(id_cliente, secreto, dirección_vuelta)` en la vuelta devuelve `{id, email, nombre, verificado, foto}` o nada si algo no cuadra |
| `mercadopago` | `crear_pago_mercadopago(token, descripción, importe, moneda, dirección_vuelta, referencia)` devuelve dónde pagar con Mercado Pago; `consultar_pago_mercadopago(token, id)` devuelve `{estado, detalle, referencia, importe, moneda}` |
| `paypal` | `crear_pago_paypal(id_cliente, secret, importe, moneda, dirección_vuelta, dirección_cancelar)` devuelve dónde aprobar el pago con PayPal; `cobrar_pago_paypal(id_cliente, secret, id_pedido)` cobra el pedido aprobado |
| `whatsapp` | `enviar_whatsapp(token, id_número, teléfono, texto)`, `enviar_plantilla_whatsapp(token, id_número, teléfono, plantilla, idioma)`, `verificar_whatsapp(clave)` para la comprobación de Meta y `mensajes_whatsapp(secreto_app)` para leer los avisos firmados |

### Notas sobre `tiempo` y las zonas horarias

- Una fecha es un número de segundos desde el 1 de enero de 1970 (UTC), igual que
  devuelve `ahora()`. `crear_fecha` y `leer_fecha` también trabajan en UTC.
- `formatear_fecha` y `dia_de_la_semana` aceptan como `zona` `"UTC"`, un desfase
  fijo (`"-05:00"`, `"+01:00"`, `"-0500"`, `"+01"`) o un nombre IANA
  (`"America/Lima"`, `"Europe/Madrid"`). Los nombres se leen del archivo TZif de
  `/usr/share/zoneinfo`, con los cambios de horario de verano; si falta el
  archivo se explica con un error claro.
- `sumar_tiempo` con `"meses"` y `"años"` funciona como PHP: no recorta al
  último día del mes. Por ejemplo, el 31 de enero de 2026 más 1 mes es el 3 de
  marzo de 2026 (febrero tiene 28 días); en 2024, al tener 29, es el 2 de marzo.
- `diferencia_fechas` devuelve `a - b` truncado hacia cero en las unidades fijas;
  en `"meses"` y `"años"` cuenta meses completos (31 de enero a 1 de marzo son
  0 meses y 29 días).
- Nombres en español: `l` da `lunes`...`domingo`, `F` da `enero`...`diciembre` y
  `M` da `ene`...`dic`. `D` sigue el formato corto inglés de PHP (`Mon`...`Sun`)
  para que los formatos importados de PHP no cambien de significado.

Ejemplo completo: `ejemplos/biblioteca.dmn`. El código está en `biblioteca/`.

### Patrones (expresiones regulares)

El módulo `patrones` trae su propio motor, sin programas externos, y trabaja con
caracteres (acentos y ñ incluidos).

Soporta: letras normales, `.`, clases `[a-z]` y `[^...]`, `\d \w \s \D \W \S`,
`\b \B`, `^ $`, grupos `( )` y `(?: )`, alternativa `|`, repeticiones
`* + ? {n} {n,} {n,m}` y sus versiones perezosas `*? +? ?? {n,m}?`, escapes
(`\.` `\\` `\/` `\n` `\t`) y `(?i)` al principio para no distinguir mayúsculas.

No soporta: mirar adelante o atrás (`(?=`, `(?<`), referencias a grupos (`\1`),
grupos con nombre ni modo multilínea. `\w` incluye letras con acento y `\d` solo
0-9; `$` solo casa al final del texto; un grupo que no participa da `""`.

```
usar modulo "patrones"
sea f = buscar_patron("pedido 2024-05-17", "(\\d+)-(\\d+)-(\\d+)")
mostrar(f[1])                                              # 2024
mostrar(reemplazar_patron("2024-05", "(\\d+)-(\\d+)", "$2/$1"))   # 05/2024
```

Dentro de un texto de Deimon la barra se escribe doble (`"\\d+"`).

Protección: cada búsqueda tiene un límite de pasos; un patrón que tardaría
muchísimo (como `(a+)+b` sobre `aaaa...c`) da un error claro en vez de colgarse.
Un patrón mal escrito da `patrón mal escrito, en la posición N: motivo`.

### Notas sobre pdf, correo, web y pagos

- **pdf** está hecho entero dentro de Deimon y no necesita nada más.
- **correo** necesita un programa `sendmail` en el ordenador (por ejemplo
  `sudo apt install msmtp-mta`, configurado con tu cuenta de correo), igual que
  `mail()` de PHP. Otro programa se puede indicar con `DEIMON_SENDMAIL`, y el
  remitente con `DEIMON_CORREO_DE`. El asunto con saltos de línea se rechaza para
  que nadie pueda colar destinatarios ocultos.
- **web** y **pagos** usan el programa `curl` para el cifrado HTTPS (viene en casi
  todos los Linux). Las claves secretas se le pasan por dentro, no en la línea de
  órdenes, para que no se vean en la lista de procesos.
- **pagos** usa Stripe Checkout: la persona paga en la página segura de Stripe y
  vuelve a tu `dirección_éxito?sesion=ID`; ahí compruebas el pago con
  `pago_completado(clave, campo("sesion"))`. Usa siempre la clave **secreta**
  (`sk_...`, nunca la pública `pk_...`) y guárdala fuera del código, por ejemplo
  con `leer("ajustes", "stripe")`. Para hacer pruebas, Stripe da claves `sk_test_`.

## Crear tus propias funciones en Rust

### funciones.txt

Tiene una línea por función, con cuatro campos separados por `|`:

```text
nombre | parámetros | descripción | ejemplo
```

Las líneas vacías y las que empiezan por `#` se ignoran. Por ejemplo:

```text
# Módulo de saludo
saludar | nombre | devuelve un saludo para una persona | saludar("Ana")
```

El `nombre` es el que se escribe en el programa `.dmn`. Los `parámetros` son sus
nombres separados por comas (`texto, veces`), o `0` si no tiene; también vale poner
solo cuántos son (`2`), aunque así `deimon modulos` y la guía no pueden decir qué es
cada uno. La llamada debe llevar ese número de argumentos (si no, sale el error `E007`).
La descripción y el ejemplo se muestran en `deimon guia archivo.dmn`.
El ejemplo debe ser una línea que se pueda ejecutar y anunciar su resultado con `# ...`
(`mayusculas("hola")   # HOLA`), salvo que necesite red o cambie cada vez; la prueba
`tests/ejemplos_documentados.rs` lo ejecuta y compara.

### codigo.rs

Para cada función de `funciones.txt` se escribe una función de Rust llamada
`ext_` seguida del mismo nombre. Su forma es siempre esta:

```rust
fn ext_saludar(args: Vec<Valor>) -> Result<Valor, String> {
    match args.as_slice() {
        [valor] => match valor.como_texto() {
            Some(nombre) => Ok(Valor::texto(format!("Hola, {}", nombre))),
            None => Err(format!("saludar() espera un texto, no {}", valor.tipo())),
        },
        _ => Err("saludar() espera un texto".to_string()),
    }
}
```

Recibe los argumentos en `args` y devuelve `Ok(valor)` o `Err(mensaje)`. Los
valores posibles son `Valor::Numero(n)` (un número decimal), `Valor::Texto(s)`,
`Valor::Logico(b)`, `Valor::Nada` y `Valor::Lista(vec)`. El método `.tipo()`
dice el tipo en español y sirve para los mensajes de error.

Para no depender de cómo se guardan por dentro los textos y las listas, hay
funciones de ayuda:

- `Valor::texto("hola")`, `Valor::texto(String)` o `Valor::texto(texto.to_uppercase())`
  crean un texto.
- `Valor::lista(vec![...])` crea una lista.
- `valor.como_texto()` devuelve `Some(&str)` si es un texto y `None` si no.
- `valor.como_lista()` devuelve `Some(&[Valor])` si es una lista.
- `valor.como_numero()` devuelve `Some(f64)` si es un número.

Así, en lugar de construir `Valor::Texto(...)` o `Valor::Lista(...)` a mano,
basta con `Valor::texto(...)` y `Valor::lista(...)`. El ejemplo completo
`biblioteca/texto/codigo.rs` usa estas ayudas.

Si una función devuelve `Err`, el programa se detiene con un error en ejecución
que incluye la línea, por ejemplo:
`Error en la línea 2 (en ejecución): saludar() espera un texto`.

Si `codigo.rs` no compila, `deimon ejecutar` lo avisa antes del error de Rust:
`El módulo 'saludo' tiene un error en su código Rust:`.

Los nombres de las funciones no pueden repetirse ni tapar una función
integrada. Los errores propios de los módulos son `E017` (no se encuentra; si
el nombre se parece a uno de la biblioteca, lo sugiere), `E018` (`modulo.txt`,
`funciones.txt` o `codigo.rs` mal formado o ausente), `E019` (nombre de módulo
no válido) y `E020` (función repetida o que choca con otra).

Ejemplo completo: `ejemplos/modulo-propio.dmn` con el módulo
`ejemplos/modulos/saludos/` (una función en Rust y otra en Deimon).

