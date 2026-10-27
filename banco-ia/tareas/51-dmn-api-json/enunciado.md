# API JSON de tareas (GET/POST, códigos 201, 400 y 404)

Escribe una API JSON en memoria (los datos solo viven mientras el servidor está en marcha). Todas
las respuestas llevan `Content-Type: application/json` y un cuerpo JSON. Una «tarea» es un objeto
`{"id": entero, "titulo": texto, "hecha": lógico}`. Las rutas:

- `GET /api/tareas` → 200 y la lista de todas las tareas por orden de id (`[]` si no hay).
- `POST /api/tareas` con un cuerpo JSON `{"titulo": "..."}` → crea una tarea con el siguiente id
  (1, 2, 3...), `hecha` en `false` y el título tal cual llegó (sin recortar); responde **201** y la tarea
  creada. Se ignoran los demás campos del cuerpo. Si el cuerpo no es JSON válido, o no es un objeto, o
  no tiene `titulo`, o `titulo` no es un texto, o es un texto que queda vacío al quitarle los espacios
  de los extremos → **400** y `{"error": "peticion invalida"}` (y no se gasta ningún id).
- `GET /api/tareas/{id}` → 200 y la tarea. `{id}` es cualquier trozo no vacío sin «/»; si no es un
  entero positivo escrito sin ceros a la izquierda (`1`, `25`; no `0`, `01`, `-1`, `abc`) o no existe
  esa tarea → **404** y `{"error": "no existe"}`.
- `POST /api/tareas/{id}/hecha` (sin cuerpo útil) → 200 y la tarea con `hecha` en `true` (si ya lo
  estaba, igual); mismo 404 `{"error": "no existe"}` para un `{id}` no válido o inexistente.
- Cualquier otra combinación de método y ruta (otra ruta, `/api/tareas/` con barra final, `DELETE`,
  `PUT`, tres trozos de ruta, `GET` sobre `.../hecha`...) → **404** y `{"error": "no encontrado"}`.

## Ejemplo (caso 1)

Peticiones y respuestas esperadas, en orden:

```
1. GET /api/tareas
   -> 200 application/json  JSON []
2. POST /api/tareas
   cabecera Content-Type: application/json
   cuerpo: {"titulo": "Comprar pan"}
   -> 201 application/json  JSON {"id": 1, "titulo": "Comprar pan", "hecha": false}
3. GET /api/tareas/1
   -> 200 application/json  JSON {"id": 1, "titulo": "Comprar pan", "hecha": false}
4. GET /api/tareas/2
   -> 404 application/json  JSON {"error": "no existe"}
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
