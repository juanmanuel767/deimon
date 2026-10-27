# Sesión con galleta: entrar, saber quién eres y contar

Escribe un servidor con sesiones por galleta (cookie). El servidor decide el nombre y el valor de la
galleta (el arnés la guarda y la devuelve como un navegador, por cliente). La sesión pertenece a
quien tenga esa galleta; una galleta inventada o desconocida es una sesión nueva sin nada. Todas las
respuestas son `200` con `Content-Type: text/html; charset=utf-8`. Usuarios existentes (distingue
mayúsculas): `ana` con clave `pez12345` y `luis` con clave `sol45678`.

- `POST /entrar` con los campos `nombre` y `clave`. Si son correctos → la sesión pasa a ser de ese
  usuario (sustituye al usuario anterior, si lo había) y el cuerpo es `<p>Hola, NOMBRE</p>`. Si no →
  `<p>Usuario o clave incorrectos</p>` y la sesión no cambia en nada.
- `GET /yo` → `<p>Eres NOMBRE</p>` si la sesión tiene un usuario, y `<p>No has entrado</p>` si no.
- `POST /salir` → la sesión deja de tener usuario (aunque no lo tuviera) y el cuerpo es `<p>Adios</p>`.
- `GET /contar` → cuenta una petición más a `/contar` **en esta sesión** y responde `<p>pasos: N</p>`
  (N = veces que esta sesión ha pedido `/contar`, esta incluida; la primera vez, 1). El contador no
  se reinicia al entrar ni al salir, y cada sesión tiene el suyo.

## Ejemplo (caso 1)

Peticiones y respuestas esperadas, en orden:

```
1. GET /yo
   -> 200 text/html; charset=utf-8  cuerpo "<p>No has entrado</p>"
2. POST /entrar
   cabecera Content-Type: application/x-www-form-urlencoded
   cuerpo: nombre=ana&clave=pez12345
   -> 200 text/html; charset=utf-8  cuerpo "<p>Hola, ana</p>"
3. GET /yo
   -> 200 text/html; charset=utf-8  cuerpo "<p>Eres ana</p>"
4. POST /salir
   -> 200 text/html; charset=utf-8  cuerpo "<p>Adios</p>"
5. GET /yo
   -> 200 text/html; charset=utf-8  cuerpo "<p>No has entrado</p>"
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
