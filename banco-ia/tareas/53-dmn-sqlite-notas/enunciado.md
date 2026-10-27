# Notas y contador de visitas con SQLite

Escribe un servidor que guarda sus datos en una base de datos **SQLite** en el archivo `datos.db` de la
carpeta `DATOS` (la carpeta de trabajo). Los datos deben **sobrevivir** a una parada y un nuevo
arranque del servidor (el arnés lo hace en algunos casos). Todas las respuestas son `200` con
`Content-Type: text/html; charset=utf-8`:

- `POST /notas` con el campo de formulario `texto`. Se le quitan los espacios de los extremos. Si el
  resultado queda vacío (o el campo no llega) → cuerpo `<p>texto vacio</p>` y no se guarda nada. Si no,
  se guarda la nota con el siguiente número (1, 2, 3...; el contador de notas no se gasta con las
  vacías) y el cuerpo es `<p>nota N guardada</p>`.
- `GET /notas` → si no hay notas, `<p>sin notas</p>`; si hay, `<ol>` + `<li>TEXTO</li>` por nota (por
  orden de número) + `</ol>`, sin espacios ni saltos de línea. TEXTO va escapado como HTML:
  `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `'` → `&#039;`.
- `GET /visitas` → cuenta una visita más y responde `<p>visitas: N</p>` con N el total incluida esta
  (la primera vez, 1). El total también se guarda en la base de datos.

## Ejemplo (caso 1)

Peticiones y respuestas esperadas, en orden:

```
1. GET /notas
   -> 200 text/html; charset=utf-8  cuerpo "<p>sin notas</p>"
2. POST /notas
   cabecera Content-Type: application/x-www-form-urlencoded
   cuerpo: texto=Primera
   -> 200 text/html; charset=utf-8  cuerpo "<p>nota 1 guardada</p>"
3. POST /notas
   cabecera Content-Type: application/x-www-form-urlencoded
   cuerpo: texto=Segunda+%26+%3Cb%3E
   -> 200 text/html; charset=utf-8  cuerpo "<p>nota 2 guardada</p>"
4. GET /notas
   -> 200 text/html; charset=utf-8  cuerpo "<ol><li>Primera</li><li>Segunda &amp; &lt;b&gt;</li></ol>"
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
