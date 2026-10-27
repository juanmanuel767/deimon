import os
import re
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs

FORMULARIO = ('<form method="post" action="/contacto"><input name="nombre"><input name="edad">'
              '<textarea name="mensaje"></textarea><button>Enviar</button></form>')


def esc(t):
    return (t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            .replace('"', "&quot;").replace("'", "&#039;"))


def pagina_contacto(campos):
    nombre = campos.get("nombre", "").strip()
    edad = campos.get("edad", "").strip()
    mensaje = campos.get("mensaje", "").strip()
    errores = []
    if not 2 <= len(nombre) <= 20:
        errores.append(f"nombre &quot;{esc(nombre)}&quot;: debe tener entre 2 y 20 caracteres")
    if not (re.fullmatch(r"[0-9]+", edad) and 18 <= int(edad) <= 120):
        errores.append(f"edad &quot;{esc(edad)}&quot;: debe ser un entero entre 18 y 120")
    if not 1 <= len(mensaje) <= 50:
        errores.append(f"mensaje &quot;{esc(mensaje)}&quot;: debe tener entre 1 y 50 caracteres")
    if errores:
        return "<h1>Errores</h1><ul>" + "".join(f"<li>{e}</li>" for e in errores) + "</ul>"
    return f"<h1>Gracias, {esc(nombre)}</h1><p>{int(edad)} anos: {esc(mensaje)}</p>"


class Manejador(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def responder(self, codigo, texto):
        cuerpo = texto.encode("utf-8")
        self.send_response(codigo)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(cuerpo)))
        self.end_headers()
        self.wfile.write(cuerpo)

    def do_GET(self):
        if self.path.split("?")[0] == "/contacto":
            return self.responder(200, FORMULARIO)
        self.responder(404, "no encontrado")

    def do_POST(self):
        largo = int(self.headers.get("Content-Length") or 0)
        cuerpo = self.rfile.read(largo).decode("utf-8") if largo else ""
        if self.path.split("?")[0] == "/contacto":
            campos = {k: v[0] for k, v in parse_qs(cuerpo, keep_blank_values=True).items()}
            return self.responder(200, pagina_contacto(campos))
        self.responder(404, "no encontrado")


ThreadingHTTPServer(("127.0.0.1", int(os.environ["PUERTO"])), Manejador).serve_forever()
