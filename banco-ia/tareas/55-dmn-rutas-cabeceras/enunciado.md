# Rutas con parámetros y cabeceras (Content-Type correcto)

Escribe un servidor con estas rutas. El `Content-Type` de cada respuesta es parte de lo que se comprueba.

- `GET /hola.txt` → 200, `text/plain; charset=utf-8`, cuerpo exacto `hola` + un salto de línea.
- `GET /estilo.css` → 200, `text/css; charset=utf-8`, cuerpo exacto `body{margin:0}` (sin salto final).
- `GET /pagina` → 200, `text/html; charset=utf-8`, cuerpo exacto `<h1>Pagina</h1>` (sin salto final).
- `GET /saludo/{nombre}` → 200, `text/plain; charset=utf-8`, cuerpo exacto `Hola, NOMBRE` (sin salto
  final). `{nombre}` es un trozo no vacío sin «/», **decodificado** (`%C3%AD` → `í`, `%20` → espacio; un «+» en la ruta se queda como «+»).
- `GET /suma/{a}/{b}` (`a` y `b` son trozos no vacíos sin «/») → si `a` y `b` son enteros (un «-» opcional y de 1 a 15 cifras, nada más) →
  200, `application/json` y el JSON `{"a": A, "b": B, "suma": S}` con números. Si alguno no lo es →
  **400**, `application/json` y `{"error": "numeros enteros"}`.
- `GET /cabeceras` → 200, `application/json` y `{"x_demo": VALOR}` donde VALOR es el texto de la cabecera
  de petición `X-Demo` (sin distinguir mayúsculas en el nombre de la cabecera) o `null` si no llega.
- Cualquier otra cosa (otra ruta, otro método, `/suma` con otro número de trozos, `/saludo/` sin
  nombre...) → **404**, `text/plain; charset=utf-8` y cuerpo exacto `no encontrado` (sin salto final).

## Ejemplo (caso 1)

Peticiones y respuestas esperadas, en orden:

```
1. GET /hola.txt
   -> 200 text/plain; charset=utf-8  cuerpo "hola\n"
2. GET /estilo.css
   -> 200 text/css; charset=utf-8  cuerpo "body{margin:0}"
3. GET /pagina
   -> 200 text/html; charset=utf-8  cuerpo "<h1>Pagina</h1>"
4. GET /saludo/Ana
   -> 200 text/plain; charset=utf-8  cuerpo "Hola, Ana"
5. GET /suma/2/3
   -> 200 application/json  JSON {"a": 2, "b": 3, "suma": 5}
6. GET /otra
   -> 404 text/plain; charset=utf-8  cuerpo "no encontrado"
```

## Contrato común de las webs (igual para PHP, Python y Deimon)

- El programa es **un servidor HTTP en un único archivo**. Escucha en `127.0.0.1`, en el puerto que
  llega en la variable de entorno `PUERTO` (el arnés elige uno libre y lo pone también en
  `DEIMON_PUERTO`, que es la variable con la que Deimon cambia el puerto de `servir(...)`: en Deimon
  basta con escribir `servir(8080)` al final del programa). No imprimas nada por la salida estándar.
- Se arranca así, con la carpeta de trabajo ya cambiada a `DATOS`:
  PHP `php -S 127.0.0.1:$PUERTO archivo.php` (el archivo es el enrutador),
  Python `python3 -I archivo.py` (solo biblioteca estándar),
  Deimon `deimon ejecutar archivo.dmn`.
- `DATOS` (variable de entorno, y también la carpeta de trabajo) es una carpeta temporal **vacía** y
  propia de cada caso, donde el programa puede guardar archivos o bases de datos. Se borra al terminar el caso.
- El arnés espera hasta 10 segundos a que el puerto responda; después hace las peticiones de un caso
  una tras otra (cada una con una conexión nueva) y compara cada respuesta con lo esperado: el **código**
  HTTP, el **Content-Type** (sin distinguir mayúsculas ni espacios: `text/html; charset=utf-8` y
  `text/html;charset=UTF-8` son lo mismo) y el **cuerpo**, que es exactamente el texto indicado
  (byte a byte, sin salto de línea final salvo que se diga) o, si el enunciado dice «JSON», se
  compara como dato (el orden de las claves y los espacios no importan; sí importan los tipos).
- Cada caso tiene un máximo de 60 segundos; el servidor se detiene siempre al acabar, falle o no.
- El arnés se comporta como un navegador: cada «cliente» (A por defecto) guarda las galletas
  (`Set-Cookie`) que recibe y las devuelve en su `Cookie`; una galleta con `Max-Age=0` o valor vacío se borra.
- Los cuerpos de formulario llegan como `application/x-www-form-urlencoded` (con `+` por espacio y `%XX`
  en UTF-8). Los textos son UTF-8. Se comprobará con varias secuencias de peticiones distintas, también
  con casos límite que no se ven aquí; en cada caso el servidor arranca desde cero.
