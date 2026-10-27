"""Tareas 51 a 55 (especialidad de Deimon: webs completas). Las usa crear_tareas.py.

Cada tarea web es un servidor HTTP. Cada caso es una secuencia de peticiones y la respuesta esperada:

    tareas/NN-nombre/casos/K.peticiones.json   lista de {"cliente", "metodo", "ruta", "cabeceras", "cuerpo"}
                                               o {"accion": "reiniciar"} (parar el servidor y volver a arrancarlo
                                               con la misma carpeta DATOS)
    tareas/NN-nombre/casos/K.esperado.json     lista del mismo largo; por petición {"codigo", "tipo", "cuerpo"|"json"}
                                               (null para «reiniciar»)

A diferencia de las tareas 01-50, las respuestas esperadas NO las calcula una referencia: están escritas a mano
aquí y se verifican con DOS referencias independientes (Python y Deimon) en comprobar_tareas.py."""
import json
from urllib.parse import urlencode

WEBS = []  # (nombre, enunciado, casos); casos = [(peticiones, esperado), ...]

HTML = "text/html; charset=utf-8"
JSON = "application/json"
TEXTO = "text/plain; charset=utf-8"
CSS = "text/css; charset=utf-8"
FORM = {"Content-Type": "application/x-www-form-urlencoded"}

COMUN_WEB = """
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
"""


def get(ruta, cliente=None, cabeceras=None):
    return peticion("GET", ruta, cliente, cabeceras)


def peticion(metodo, ruta, cliente=None, cabeceras=None, cuerpo=None):
    p = {"metodo": metodo, "ruta": ruta}
    if cliente:
        p["cliente"] = cliente
    if cabeceras:
        p["cabeceras"] = cabeceras
    if cuerpo is not None:
        p["cuerpo"] = cuerpo
    return p


def post_form(ruta, cliente=None, **campos):
    return peticion("POST", ruta, cliente, FORM, urlencode(campos))


def post_crudo(ruta, cuerpo, cliente=None, cabeceras=FORM):
    return peticion("POST", ruta, cliente, cabeceras, cuerpo)


def post_json(ruta, texto):
    """texto: el JSON ya escrito (puede estar mal formado a propósito)."""
    return peticion("POST", ruta, None, {"Content-Type": "application/json"}, texto)


REINICIAR = {"accion": "reiniciar"}


def resp(codigo, tipo, cuerpo=None, dato=None):
    r = {"codigo": codigo, "tipo": tipo}
    if dato is not None or cuerpo is None:
        r["json"] = dato
    else:
        r["cuerpo"] = cuerpo
    return r


def web(nombre, enunciado):
    def registrar(funcion):
        WEBS.append((nombre, enunciado.strip() + "\n", funcion()))
        return funcion
    return registrar


def texto_ejemplo(peticiones, esperado):
    """Cómo se ve un caso dentro del enunciado."""
    lineas = []
    n = 0
    for p, e in zip(peticiones, esperado):
        if p.get("accion") == "reiniciar":
            lineas.append("--- se para el servidor y se vuelve a arrancar (misma carpeta DATOS) ---")
            continue
        n += 1
        cl = f" [cliente {p['cliente']}]" if p.get("cliente") else ""
        lineas.append(f"{n}. {p['metodo']} {p['ruta']}{cl}")
        for k, v in (p.get("cabeceras") or {}).items():
            lineas.append(f"   cabecera {k}: {v}")
        if "cuerpo" in p:
            lineas.append(f"   cuerpo: {p['cuerpo']}")
        if "json" in e:
            lineas.append(f"   -> {e['codigo']} {e['tipo']}  JSON {json.dumps(e['json'], ensure_ascii=False)}")
        else:
            lineas.append(f"   -> {e['codigo']} {e['tipo']}  cuerpo {json.dumps(e['cuerpo'], ensure_ascii=False)}")
    return "\n".join(lineas)


# ------------------------------------------------------------------------------------------------- 51

def tarea(i, titulo, hecha=False):
    return {"id": i, "titulo": titulo, "hecha": hecha}


NO_ENCONTRADO = resp(404, JSON, dato={"error": "no encontrado"})
NO_EXISTE = resp(404, JSON, dato={"error": "no existe"})
INVALIDA = resp(400, JSON, dato={"error": "peticion invalida"})


@web("51-dmn-api-json", """
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
""")
def _():
    t1 = tarea(1, "Comprar pan")
    c1 = ([get("/api/tareas"), post_json("/api/tareas", '{"titulo": "Comprar pan"}'), get("/api/tareas/1"),
           get("/api/tareas/2")],
          [resp(200, JSON, dato=[]), resp(201, JSON, dato=t1), resp(200, JSON, dato=t1), NO_EXISTE])
    c2 = ([post_json("/api/tareas", '{"titulo": "Estudiar ñandú"}'),
           post_json("/api/tareas", '{"titulo": "Decir \\"hola\\" <b>&"}'),
           post_json("/api/tareas", '{"titulo": "x", "hecha": true, "id": 99}'),
           peticion("POST", "/api/tareas/2/hecha"), peticion("POST", "/api/tareas/2/hecha"),
           get("/api/tareas"), get("/api/tareas/3")],
          [resp(201, JSON, dato=tarea(1, "Estudiar ñandú")), resp(201, JSON, dato=tarea(2, 'Decir "hola" <b>&')),
           resp(201, JSON, dato=tarea(3, "x")), resp(200, JSON, dato=tarea(2, 'Decir "hola" <b>&', True)),
           resp(200, JSON, dato=tarea(2, 'Decir "hola" <b>&', True)),
           resp(200, JSON, dato=[tarea(1, "Estudiar ñandú"), tarea(2, 'Decir "hola" <b>&', True), tarea(3, "x")]),
           resp(200, JSON, dato=tarea(3, "x"))])
    malos = ['{titulo', '[]', '{"titulo": ""}', '{"titulo": "   "}', '{"titulo": 5}', '{}', '', 'null',
             '{"titulo": null}', '"texto"']
    c3 = ([post_json("/api/tareas", m) for m in malos] + [post_json("/api/tareas", '{"titulo": "  hola  "}'),
                                                           get("/api/tareas")],
          [INVALIDA] * len(malos) + [resp(201, JSON, dato=tarea(1, "  hola  ")),
                                     resp(200, JSON, dato=[tarea(1, "  hola  ")])])
    c4 = ([post_json("/api/tareas", '{"titulo": "uno"}'), get("/api/tareas/abc"), get("/api/tareas/0"),
           get("/api/tareas/01"), get("/api/tareas/-1"), peticion("POST", "/api/tareas/9/hecha"),
           peticion("POST", "/api/tareas/abc/hecha"), get("/otra"), get("/api"), get("/api/tareas/"),
           peticion("DELETE", "/api/tareas"), peticion("DELETE", "/api/tareas/1"), get("/api/tareas/1/hecha"),
           peticion("POST", "/api/tareas/1/2/3"), peticion("PUT", "/api/tareas/1"), peticion("POST", "/api/tareas/"),
           get("/")],
          [resp(201, JSON, dato=tarea(1, "uno"))] + [NO_EXISTE] * 6 + [NO_ENCONTRADO] * 10)
    c5 = ([post_json("/api/tareas", '{"titulo": "a"}'), post_json("/api/tareas", '{"titulo": "b"}'),
           post_json("/api/tareas", '{"titulo": "c"}'), peticion("POST", "/api/tareas/1/hecha"),
           peticion("POST", "/api/tareas/3/hecha"), get("/api/tareas"), get("/api/tareas/2"),
           peticion("POST", "/api/tareas/4/hecha")],
          [resp(201, JSON, dato=tarea(1, "a")), resp(201, JSON, dato=tarea(2, "b")), resp(201, JSON, dato=tarea(3, "c")),
           resp(200, JSON, dato=tarea(1, "a", True)), resp(200, JSON, dato=tarea(3, "c", True)),
           resp(200, JSON, dato=[tarea(1, "a", True), tarea(2, "b"), tarea(3, "c", True)]),
           resp(200, JSON, dato=tarea(2, "b")), NO_EXISTE])
    return [c1, c2, c3, c4, c5]


# ------------------------------------------------------------------------------------------------- 52

FORMULARIO = ('<form method="post" action="/contacto"><input name="nombre"><input name="edad">'
              '<textarea name="mensaje"></textarea><button>Enviar</button></form>')


def gracias(n, e, m):
    return f"<h1>Gracias, {n}</h1><p>{e} anos: {m}</p>"


def errores(*li):
    return "<h1>Errores</h1><ul>" + "".join(f"<li>{x}</li>" for x in li) + "</ul>"


E_NOM = "nombre &quot;{}&quot;: debe tener entre 2 y 20 caracteres"
E_EDAD = "edad &quot;{}&quot;: debe ser un entero entre 18 y 120"
E_MSJ = "mensaje &quot;{}&quot;: debe tener entre 1 y 50 caracteres"


@web("52-dmn-formulario", """
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
""")
def _():
    g = lambda n, e, m: resp(200, HTML, gracias(n, e, m))
    f = lambda *li: resp(200, HTML, errores(*li))
    c1 = ([get("/contacto"), post_form("/contacto", nombre="Ana", edad="30", mensaje="Hola"),
           post_form("/contacto", nombre="A", edad="abc", mensaje="")],
          [resp(200, HTML, FORMULARIO), g("Ana", "30", "Hola"),
           f(E_NOM.format("A"), E_EDAD.format("abc"), E_MSJ.format(""))])
    c2 = ([post_form("/contacto", nombre="<b>Jo&e</b>", edad="18", mensaje="\"Hola\" 'mundo' & <x>")],
          [g("&lt;b&gt;Jo&amp;e&lt;/b&gt;", "18", "&quot;Hola&quot; &#039;mundo&#039; &amp; &lt;x&gt;")])
    n20, n21 = "abcdefghijklmnopqrst", "abcdefghijklmnopqrstu"
    c3 = ([post_form("/contacto", nombre=n20, edad="120", mensaje="m" * 50),
           post_form("/contacto", nombre=n21, edad="17", mensaje="m" * 51),
           post_form("/contacto", nombre="Ñu", edad="18", mensaje="ñ"),
           post_form("/contacto", nombre="ñ" * 20, edad="19", mensaje="ñ" * 50),
           post_form("/contacto", nombre="ñ" * 21, edad="121", mensaje="ok")],
          [g(n20, "120", "m" * 50), f(E_NOM.format(n21), E_EDAD.format("17"), E_MSJ.format("m" * 51)),
           g("Ñu", "18", "ñ"), g("ñ" * 20, "19", "ñ" * 50), f(E_NOM.format("ñ" * 21), E_EDAD.format("121"))])
    c4 = ([post_form("/contacto", nombre="  Ana  ", edad=" 45 ", mensaje="  hola mundo  "),
           post_crudo("/contacto", ""), post_form("/contacto", nombre="Bob"),
           post_form("/contacto", nombre="Eva", edad="-5", mensaje="x"),
           post_form("/contacto", nombre="Eva", edad="30.5", mensaje="x"),
           post_form("/contacto", nombre="Eva", edad="3 0", mensaje="x"),
           post_form("/contacto", nombre="Eva", edad="+30", mensaje="x"),
           post_form("/contacto", nombre="Eva", edad="1e2", mensaje="x"),
           post_form("/contacto", nombre="Eva", edad="99999999999999999999", mensaje="x"),
           post_form("/contacto", nombre="   ", edad="18", mensaje="   ")],
          [g("Ana", "45", "hola mundo"),
           f(E_NOM.format(""), E_EDAD.format(""), E_MSJ.format("")),
           f(E_EDAD.format(""), E_MSJ.format("")),
           f(E_EDAD.format("-5")), f(E_EDAD.format("30.5")), f(E_EDAD.format("3 0")), f(E_EDAD.format("+30")),
           f(E_EDAD.format("1e2")), f(E_EDAD.format("99999999999999999999")),
           f(E_NOM.format(""), E_MSJ.format(""))])
    c5 = ([post_form("/contacto", nombre="<", edad="\"x\"", mensaje="&" * 51),
           post_form("/contacto", nombre="'", edad="'", mensaje="'" * 50),
           post_form("/contacto", nombre="a&b", edad="18", mensaje="<script>alert('x')</script>"),
           get("/contacto")],
          [f(E_NOM.format("&lt;"), E_EDAD.format("&quot;x&quot;"), E_MSJ.format("&amp;" * 51)),
           f(E_NOM.format("&#039;"), E_EDAD.format("&#039;")),
           g("a&amp;b", "18", "&lt;script&gt;alert(&#039;x&#039;)&lt;/script&gt;"),
           resp(200, HTML, FORMULARIO)])
    return [c1, c2, c3, c4, c5]


# ------------------------------------------------------------------------------------------------- 53

@web("53-dmn-sqlite-notas", """
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
""")
def _():
    ok = lambda n: resp(200, HTML, f"<p>nota {n} guardada</p>")
    vacio = resp(200, HTML, "<p>texto vacio</p>")
    sin = resp(200, HTML, "<p>sin notas</p>")
    lista = lambda *t: resp(200, HTML, "<ol>" + "".join(f"<li>{x}</li>" for x in t) + "</ol>")
    vis = lambda n: resp(200, HTML, f"<p>visitas: {n}</p>")
    c1 = ([get("/notas"), post_form("/notas", texto="Primera"), post_form("/notas", texto="Segunda & <b>"),
           get("/notas")],
          [sin, ok(1), ok(2), lista("Primera", "Segunda &amp; &lt;b&gt;")])
    c2 = ([get("/visitas"), get("/visitas"), get("/visitas"), REINICIAR, get("/visitas"), get("/notas"),
           post_form("/notas", texto="sigue aqui"), REINICIAR, get("/notas"), get("/visitas")],
          [vis(1), vis(2), vis(3), None, vis(4), sin, ok(1), None, lista("sigue aqui"), vis(5)])
    c3 = ([post_form("/notas", texto=""), post_crudo("/notas", ""), post_form("/notas", texto="   "),
           post_form("/notas", texto=" Hola "), post_form("/notas", otro="x"), get("/notas")],
          [vacio, vacio, vacio, ok(1), vacio, lista("Hola")])
    c4 = ([post_form("/notas", texto="'; DROP TABLE notas; --"), post_form("/notas", texto="ñandú <i>é</i> \"q\""),
           post_form("/notas", texto="a%b+c&d=e"), get("/notas"), get("/visitas"), get("/notas")],
          [ok(1), ok(2), ok(3),
           lista("&#039;; DROP TABLE notas; --", "ñandú &lt;i&gt;é&lt;/i&gt; &quot;q&quot;", "a%b+c&amp;d=e"),
           vis(1), lista("&#039;; DROP TABLE notas; --", "ñandú &lt;i&gt;é&lt;/i&gt; &quot;q&quot;", "a%b+c&amp;d=e")])
    c5 = ([post_form("/notas", texto="uno"), post_form("/notas", texto="dos"), post_form("/notas", texto="tres"),
           REINICIAR, post_form("/notas", texto="cuatro"), get("/notas"), get("/visitas"), REINICIAR,
           get("/visitas"), post_form("/notas", texto=" "), post_form("/notas", texto="cinco"),
           get("/notas")],
          [ok(1), ok(2), ok(3), None, ok(4), lista("uno", "dos", "tres", "cuatro"), vis(1), None, vis(2), vacio,
           ok(5), lista("uno", "dos", "tres", "cuatro", "cinco")])
    return [c1, c2, c3, c4, c5]


# ------------------------------------------------------------------------------------------------- 54

@web("54-dmn-sesion-galleta", """
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
""")
def _():
    hola = lambda n: resp(200, HTML, f"<p>Hola, {n}</p>")
    eres = lambda n: resp(200, HTML, f"<p>Eres {n}</p>")
    nadie = resp(200, HTML, "<p>No has entrado</p>")
    mal = resp(200, HTML, "<p>Usuario o clave incorrectos</p>")
    adios = resp(200, HTML, "<p>Adios</p>")
    pasos = lambda n: resp(200, HTML, f"<p>pasos: {n}</p>")
    c1 = ([get("/yo"), post_form("/entrar", nombre="ana", clave="pez12345"), get("/yo"),
           peticion("POST", "/salir"), get("/yo")],
          [nadie, hola("ana"), eres("ana"), adios, nadie])
    c2 = ([post_form("/entrar", nombre="ana", clave="mala"), get("/yo"), post_form("/entrar", nombre="pepe", clave="x"),
           post_form("/entrar", nombre="luis", clave="sol45678"), post_form("/entrar", nombre="ana", clave="otra"),
           get("/yo")],
          [mal, nadie, mal, hola("luis"), mal, eres("luis")])
    c3 = ([get("/contar"), get("/contar"), post_form("/entrar", nombre="ana", clave="pez12345"), get("/contar"),
           peticion("POST", "/salir"), get("/contar"), get("/contar", cliente="B"), get("/contar")],
          [pasos(1), pasos(2), hola("ana"), pasos(3), adios, pasos(4), pasos(1), pasos(5)])
    c4 = ([post_form("/entrar", nombre="ana", clave="pez12345", cliente="A"),
           post_form("/entrar", nombre="luis", clave="sol45678", cliente="B"),
           get("/yo", cliente="A"), get("/yo", cliente="B"), get("/yo", cliente="C"),
           get("/yo", cliente="D", cabeceras={"Cookie": "usuario=ana; sesion=ana; PHPSESSID=ana; id=1"}),
           peticion("POST", "/salir", cliente="A"), get("/yo", cliente="A"), get("/yo", cliente="B")],
          [hola("ana"), hola("luis"), eres("ana"), eres("luis"), nadie, nadie, adios, nadie, eres("luis")])
    c5 = ([post_form("/entrar", nombre="", clave=""), post_form("/entrar", nombre="Ana", clave="pez12345"),
           post_form("/entrar", nombre="ana", clave="PEZ12345"), peticion("POST", "/salir"), get("/yo"),
           post_form("/entrar", nombre="luis", clave="sol45678"), get("/yo"),
           post_form("/entrar", nombre="ana", clave="pez12345"), get("/yo"), get("/contar")],
          [mal, mal, mal, adios, nadie, hola("luis"), eres("luis"), hola("ana"), eres("ana"), pasos(1)])
    return [c1, c2, c3, c4, c5]


# ------------------------------------------------------------------------------------------------- 55

NF = resp(404, TEXTO, "no encontrado")
NUM = resp(400, JSON, dato={"error": "numeros enteros"})


def suma(a, b, s):
    return resp(200, JSON, dato={"a": a, "b": b, "suma": s})


@web("55-dmn-rutas-cabeceras", """
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
""")
def _():
    c1 = ([get("/hola.txt"), get("/estilo.css"), get("/pagina"), get("/saludo/Ana"), get("/suma/2/3"),
           get("/otra")],
          [resp(200, TEXTO, "hola\n"), resp(200, CSS, "body{margin:0}"), resp(200, HTML, "<h1>Pagina</h1>"),
           resp(200, TEXTO, "Hola, Ana"), suma(2, 3, 5), NF])
    c2 = ([get("/suma/-5/12"), get("/suma/-10/-20"), get("/suma/999999999999999/1"), get("/suma/0/0"),
           get("/suma/abc/1"), get("/suma/1.5/2"), get("/suma/+3/1"), get("/suma/1/"), get("/suma/1"),
           get("/suma/1/2/3"), get("/suma/1234567890123456/1"), get("/suma/-/1")],
          [suma(-5, 12, 7), suma(-10, -20, -30), suma(999999999999999, 1, 1000000000000000), suma(0, 0, 0),
           NUM, NUM, NUM, NF, NF, NF, NUM, NUM])
    c3 = ([get("/cabeceras", cabeceras={"X-Demo": "hola"}), get("/cabeceras"),
           get("/cabeceras", cabeceras={"x-demo": "uno dos"}), get("/cabeceras", cabeceras={"X-Otro": "1"}),
           peticion("POST", "/cabeceras", None, {"X-Demo": "p"}, "")],
          [resp(200, JSON, dato={"x_demo": "hola"}), resp(200, JSON, dato={"x_demo": None}),
           resp(200, JSON, dato={"x_demo": "uno dos"}), resp(200, JSON, dato={"x_demo": None}), NF])
    c4 = ([get("/saludo/Mar%C3%ADa"), get("/saludo/a%20b"), get("/saludo/"), get("/saludo"),
           peticion("POST", "/saludo/x", None, None, ""), get("/saludo/%C3%91and%C3%BA"), get("/saludo/a+b"),
           get("/saludo/x/y")],
          [resp(200, TEXTO, "Hola, María"), resp(200, TEXTO, "Hola, a b"), NF, NF, NF,
           resp(200, TEXTO, "Hola, Ñandú"), resp(200, TEXTO, "Hola, a+b"), NF])
    c5 = ([get("/nada"), get("/hola.txt/extra"), peticion("POST", "/hola.txt", None, None, ""),
           peticion("DELETE", "/pagina"), get("/"), get("/estilo.css"), get("/Pagina"), get("/hola.txt")],
          [NF, NF, NF, NF, NF, resp(200, CSS, "body{margin:0}"), NF, resp(200, TEXTO, "hola\n")])
    return [c1, c2, c3, c4, c5]
