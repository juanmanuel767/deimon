# Formulario con validación y errores escapados

Escribe un servidor con dos rutas bajo `/contacto`. Todas las respuestas son `200` con
`Content-Type: text/html; charset=utf-8`.

- `GET /contacto` → el cuerpo exacto (una sola línea, sin salto final):
  `<form method="post" action="/contacto"><input name="nombre"><input name="edad"><textarea name="mensaje"></textarea><button>Enviar</button></form>`
- `POST /contacto` con los campos de formulario `nombre`, `edad` y `mensaje` (si alguno no llega, vale
  el texto vacío). Primero se quitan los espacios de los extremos de cada valor (**VALOR** es el valor
  ya recortado; la longitud se cuenta en caracteres, no en bytes: «ñ» cuenta 1). Se validan:
  - `nombre`: entre 2 y 20 caracteres.
  - `edad`: solo cifras 0-9 (nada de signos, puntos ni espacios en medio) y un número entre 18 y 120.
  - `mensaje`: entre 1 y 50 caracteres.

  Si todo es válido, el cuerpo es `<h1>Gracias, NOMBRE</h1><p>EDAD anos: MENSAJE</p>` (EDAD como número
  entero). Si hay errores, el cuerpo es `<h1>Errores</h1><ul>` + un `<li>...</li>` por cada campo
  inválido, **en el orden nombre, edad, mensaje**, + `</ul>`, donde cada `<li>` es, según el campo:
  - `nombre &quot;VALOR&quot;: debe tener entre 2 y 20 caracteres`
  - `edad &quot;VALOR&quot;: debe ser un entero entre 18 y 120`
  - `mensaje &quot;VALOR&quot;: debe tener entre 1 y 50 caracteres`

  Todo VALOR, NOMBRE y MENSAJE que se escriba va **escapado como HTML**: `&` → `&amp;`, `<` → `&lt;`,
  `>` → `&gt;`, `"` → `&quot;` y `'` → `&#039;` (cada carácter se sustituye una sola vez).

## Ejemplo (caso 1)

Peticiones y respuestas esperadas, en orden:

```
1. GET /contacto
   -> 200 text/html; charset=utf-8  cuerpo "<form method=\"post\" action=\"/contacto\"><input name=\"nombre\"><input name=\"edad\"><textarea name=\"mensaje\"></textarea><button>Enviar</button></form>"
2. POST /contacto
   cabecera Content-Type: application/x-www-form-urlencoded
   cuerpo: nombre=Ana&edad=30&mensaje=Hola
   -> 200 text/html; charset=utf-8  cuerpo "<h1>Gracias, Ana</h1><p>30 anos: Hola</p>"
3. POST /contacto
   cabecera Content-Type: application/x-www-form-urlencoded
   cuerpo: nombre=A&edad=abc&mensaje=
   -> 200 text/html; charset=utf-8  cuerpo "<h1>Errores</h1><ul><li>nombre &quot;A&quot;: debe tener entre 2 y 20 caracteres</li><li>edad &quot;abc&quot;: debe ser un entero entre 18 y 120</li><li>mensaje &quot;&quot;: debe tener entre 1 y 50 caracteres</li></ul>"
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
