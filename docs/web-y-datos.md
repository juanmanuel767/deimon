# Web y datos

[Volver al README](../README.md)

## Guardar datos

Los datos se guardan en archivos y siguen ahí cuando el programa termina.
Cada colección es un archivo `<nombre>.registro` dentro de la carpeta `datos`
(o la que indique la variable `DEIMON_DATOS`); la carpeta se crea sola.

### Por qué es una base de datos fuerte

- **No se pierde lo guardado:** cada cambio se *añade* al final del registro y se
  fuerza al disco antes de seguir. Si se va la luz a mitad de una escritura, esa
  línea a medias se descarta y todo lo anterior sigue intacto.
- **Detecta daños:** cada línea lleva una suma de control. Si algo cambia el
  archivo a mano o el disco lo estropea, Deimon avisa en vez de inventar datos.
- **Varios programas a la vez:** un candado de archivo hace que dos programas (o
  dos copias del mismo servidor) no se pisen. `incrementar()` suma de forma
  atómica: con 4 programas sumando a la vez no se pierde ninguna suma.
- **Rápida:** lo leído se guarda en memoria y solo se lee lo nuevo; cuando el
  registro crece mucho se compacta solo.
- Los datos del formato antiguo (`<nombre>.datos`) se pasan solos al nuevo la
  primera vez que se usan.

```deimon
guardar("amigos", "ana", 30)     # número, texto, lógico, nada o una lista
mostrar(leer("amigos", "ana"))   # escribe 30; si no existe, nada
mostrar(contar("amigos"))        # cuántas claves hay
mostrar(listar_claves("amigos")) # lista de claves, ordenada
borrar("amigos", "ana")
sea total = incrementar("contadores", "visitas", 1)   # suma atómica, devuelve el total
```

El nombre de la colección solo puede tener letras sin acentos, dígitos, `_` y
`-` (de 1 a 64 caracteres); si no, el programa termina con un error. Ejemplos:
`ejemplos/datos.dmn` y `ejemplos-web/libro.dmn` (libro de visitas en el puerto
8090).

## Páginas web

El lenguaje puede servir páginas web. Se define una `ruta` para cada dirección
y al final se llama a `servir` con el puerto:

```deimon
ruta "/" {
    devolver "<h1>Inicio</h1>"
}

ruta "/saludo/:nombre" {
    devolver "<h1>Hola, " + nombre + "</h1>"
}

servir(8080)
```

Los trozos del camino que empiezan por `:` se convierten en variables de
texto dentro de la ruta. Por seguridad, el valor que llega en la dirección se
escapa como HTML antes de usarlo, así nadie puede inyectar código en la
página. Si quieres hacer cuentas con un parámetro, conviértelo con `a_numero`.

Para probar el ejemplo incluido, desde la carpeta `deimon`:

```bash
compilador/target/release/deimon ejecutar ejemplos-web/sitio.dmn
```

Verás el mensaje `Servidor listo en http://127.0.0.1:8080`. Abre esa dirección
en el navegador (`http://127.0.0.1:8080`) y visita también
`http://127.0.0.1:8080/saludo/Ana` y `http://127.0.0.1:8080/cuadrado/5`.
Para parar el servidor, pulsa `Ctrl+C`.

Si existe la variable de entorno `DEIMON_PUERTO`, su valor sustituye al puerto
escrito en `servir(...)`. Sirve, por ejemplo, para que las pruebas usen un
puerto libre sin cambiar el ejemplo.

## Plantillas HTML

En vez de escribir el HTML dentro de textos, se escribe en un archivo `.html` con
huecos, y `plantilla("archivo.html", datos)` lo rellena:

```html
<h1>Hola, {{ usuario.nombre }}</h1>
{% si productos %}
  <ul>{% para p en productos %}<li>{{ p.nombre }}: {{ p.precio }}</li>{% fin %}</ul>
{% sino %}
  <p>No hay productos.</p>
{% fin %}
{# esto es una nota y no sale en la página #}
```

```deimon
ruta "/" {
    devolver plantilla("vistas/inicio.html", {"usuario": {"nombre": "Ana"}, "productos": lista})
}
```

- `{{ dato }}` escribe el dato protegido como HTML (nadie puede colar `<script>`);
  lo que ya venía protegido, como los campos de un formulario, no se protege dos veces.
- `{{{ dato }}}` lo escribe tal cual: úsalo solo con HTML que hayas creado tú.
- `dato.campo` entra en una lista con claves y `lista.0` en una lista.
- `{% para x en lista %}...{% fin %}` repite; `{% si dato %}...{% sino %}...{% fin %}`
  elige (`{% si no dato %}` al revés). Cuenta como falso: `nada`, `falso`, `0`, `""`
  y las listas vacías.
- La ruta del archivo es relativa al `.dmn`. El archivo se mete **dentro** del
  programa al traducir: si no existe o le falta un `{% fin %}`, Deimon avisa
  (código `E024`) antes de arrancar, y el programa no necesita llevar la carpeta.
- Si la plantilla usa un dato que no le das, la página falla con un mensaje que lo
  dice. Para una plantilla que no está en un archivo: `rellenar_plantilla(texto, datos)`.

## MySQL y MariaDB

Con `usar modulo "mysql"` se trabaja con una base MySQL o MariaDB que ya exista (la
de un alojamiento web, por ejemplo). Detalles en [modulos.md](modulos.md).

```deimon
usar modulo "mysql"
# La contraseña no se escribe aquí: va en la variable DEIMON_MYSQL, por ejemplo
#   DEIMON_MYSQL="mysql://tienda:secreta@localhost/tienda" deimon ejecutar web.dmn
para fila en consultar_mysql("", "SELECT nombre, precio FROM productos WHERE precio < ?", [10]) {
    mostrar(fila["nombre"] + ": " + fila["precio"])
}
ejecutar_mysql("", "INSERT INTO productos (nombre, precio) VALUES (?, ?)", [campo("nombre"), 8.5])
```

Funciona con los dos métodos de contraseña de MySQL 8 (`caching_sha2_password`, que
por la red se manda cifrada con la llave del servidor, y `mysql_native_password`).
Cada hilo del servidor web mantiene su conexión abierta y la reutiliza. No cifra la
conexión (TLS): si la base está en otro ordenador, úsala dentro de una red privada.

## Formularios

Una ruta atiende tanto las visitas como los formularios que se envían a ella
(`<form method='post' action='/registro'>`). Dentro de la ruta:

- `es_envio()` es verdadero si llegó un formulario.
- `campo("nombre")` da el valor de un campo (también de `?nombre=` en la
  dirección), **ya protegido como HTML**; nada si no viene.
- `redirigir("/gracias")` manda al navegador a otra página del mismo sitio (solo
  caminos que empiezan por `/`).

```deimon
ruta "/contacto" {
    si es_envio() {
        guardar("mensajes", campo("correo"), campo("texto"))
        devolver redirigir("/gracias")
    }
    devolver "<form method='post'><input name='correo'><textarea name='texto'></textarea><button>Enviar</button></form>"
}
```

El tamaño del formulario cuenta para el límite `DEIMON_MAX_PETICION` (si se pasa,
responde 413). Otros métodos que no sean visitar (GET) o enviar (POST) reciben 405.

## Subir archivos

Un formulario con `enctype='multipart/form-data'` puede mandar archivos:

```deimon
ruta "/" {
    devolver "<form method='post' action='/subir' enctype='multipart/form-data'><input type='file' name='foto'> <button>Subir</button></form>"
}

ruta "/subir" {
    sea foto = archivo_subido("foto")          # nada si no llegó archivo
    si foto == nada o no foto["es_imagen"] {
        devolver "Elige una imagen"
    }
    sea nombre = guardar_subida("foto", "fotos")
    devolver "<img src='/foto/" + nombre + "'>"
}

ruta "/foto/:nombre" {
    si no responder_archivo("fotos", nombre) {
        devolver "Esa foto no existe"
    }
}
```

- `archivo_subido(campo)` da `nombre`, `tipo`, `tamano` y `es_imagen`. Si es imagen
  se mira por su **contenido** (JPG, PNG, GIF o WEBP), no por lo que diga el navegador.
- `guardar_subida(campo, carpeta)` lo guarda en `datos/archivos/<carpeta>/` con un
  **nombre nuevo al azar** (nadie puede elegir dónde se escribe) y devuelve ese nombre.
  Las imágenes y los PDF conservan su tipo; todo lo demás se guarda como `.bin`.
- `responder_archivo(carpeta, nombre)` hace que la ruta responda con el archivo:
  imágenes y PDF se ven en el navegador; cualquier otra cosa se **descarga**, así un
  archivo subido nunca se abre como página (un `<script>` disfrazado no se ejecuta).
  Devuelve `falso` si no existe; nombres con `/` o `..` no se aceptan.
- Tamaño máximo: `DEIMON_MAX_SUBIDA` (10 MB por defecto). Una subida lenta pero real
  llega aunque tarde más de 10 segundos.

Ejemplo completo: `ejemplos-web/fotos.dmn` (galería de fotos).

## Galletas (cookies)

```deimon
guardar_galleta("idioma", "es", 365)   # dura un año (0: hasta cerrar el navegador)
sea idioma = leer_galleta("idioma")    # "es", o nada
borrar_galleta("idioma")
```

Las galletas se guardan con `HttpOnly` (JavaScript no las puede leer), `SameSite=Lax`
y, por HTTPS, `Secure`. Lo que se lee viene protegido como HTML. Los nombres que
empiezan por `deimon_` están reservados para las sesiones.

## Sesión del visitante

Para recordar cosas de cada navegador aunque nadie haya entrado como usuario
(un carrito, el idioma elegido...), como `$_SESSION` en PHP:

```deimon
guardar_en_sesion("idioma", "es")
leer_de_sesion("idioma")              # "es", o nada
agregar_a_sesion("carrito", "taza")   # añade a una lista (la crea si no existe)
borrar_de_sesion("idioma")
vaciar_sesion()
```

Va en una galleta segura (`deimon_visita`, HttpOnly, SameSite=Lax) con un código
aleatorio; en la base de datos solo se guarda su huella. Dura 24 horas desde el
último cambio y sobrevive a un reinicio del servidor.

## Usuarios y sesiones

```deimon
crear_usuario(nombre, clave)    # falso si ya existe o la clave tiene menos de 8 caracteres
comprobar_clave(nombre, clave)  # verdadero si la contraseña es la suya
iniciar_sesion(nombre)          # desde ahora este navegador "es" ese usuario
usuario_actual()                # nombre del usuario, o nada si nadie ha entrado
cerrar_sesion()
```

- Las contraseñas **nunca se guardan tal cual**: se cifran con PBKDF2-SHA256
  (100 000 vueltas y una sal aleatoria por usuario). Comprobado con los valores
  oficiales de prueba (RFC 7914).
- Los usuarios viven en una colección interna (`.usuarios`) que `leer()` y
  `guardar()` no pueden tocar.
- La sesión es una galleta (cookie) con un código aleatorio de 256 bits, marcada
  `HttpOnly` (los scripts de la página no la ven) y `SameSite=Lax` (otros sitios no
  pueden enviar formularios con ella). Dura 7 días; si el servidor se reinicia,
  hay que volver a entrar.
- Al comprobar una clave de un usuario que no existe se tarda lo mismo, para que
  nadie averigüe qué usuarios existen por el tiempo de respuesta.
- **Límite de intentos:** tras 5 contraseñas equivocadas seguidas, la cuenta se
  bloquea 15 minutos y `comprobar_clave` da falso aunque la contraseña sea buena.
  `cuenta_bloqueada(nombre)` lo dice para mostrar un mensaje.
- **Sesiones que sobreviven a un reinicio:** se guardan en la colección interna
  `.sesiones`, pero solo la huella SHA-256 del código: quien copie el archivo no
  puede hacerse pasar por nadie.

```deimon
cambiar_clave(nombre, actual, nueva)        # sabiendo la actual; cierra las demás sesiones
sea codigo = crear_codigo_recuperacion(nombre)   # un solo uso, caduca en 1 hora
restablecer_clave(nombre, codigo, nueva)    # pone la nueva con el código
```

El código de recuperación lo genera Deimon y tu programa decide cómo hacérselo
llegar al usuario (correo, mensaje, en persona...). Deimon no envía correos por sí
mismo: hacerlo bien exige una conexión cifrada (TLS) que no se puede programar
con seguridad sin librerías externas.

En las pruebas, `enviar("/entrar", "nombre=ana&clave=secreta12")` envía un
formulario sin red y guarda la galleta de sesión como un navegador, así que los
`visitar()` siguientes ya van con la sesión abierta. Si la ruta redirige, devuelve
`"redirige a /camino"`.

Ejemplo completo: `ejemplos-web/cuentas.dmn` (registro, entrada, página privada y
salida, con sus pruebas).

Aún no incluye envío de correos sin `sendmail` (ver el módulo `correo`).

## HTTPS y publicar en internet

`deimon publicar` pone HTTPS delante de un programa web usando **Caddy**, un
programa gratuito y muy probado que consigue y renueva solo los certificados.
Deimon no programa su propio cifrado: uno hecho a mano casi siempre tiene fallos,
y usar uno probado es lo seguro.

```bash
sudo apt install caddy                         # una vez
deimon publicar sitio.dmn --prueba             # HTTPS en este ordenador: https://localhost:8443
deimon publicar sitio.dmn --dominio midominio.com   # carpeta lista para un servidor de internet
```

- `--prueba` arranca el programa y Caddy con un certificado local (el navegador
  avisa la primera vez porque no es público). Ctrl+C lo para todo.
- `--dominio` compila el programa con el perfil servidor y crea una carpeta con
  el ejecutable, el `Caddyfile` y `arrancar.sh`. En el servidor: apunta el
  dominio a su IP, abre los puertos 80 y 443 y ejecuta `./arrancar.sh`.
- Cuando una página llega por HTTPS, Deimon marca las galletas de sesión como
  `Secure` (solo viajan cifradas).
- Al instalar Caddy con apt, arranca solo un servicio en el puerto 80; si no lo
  quieres, apágalo con `sudo systemctl disable --now caddy`.
