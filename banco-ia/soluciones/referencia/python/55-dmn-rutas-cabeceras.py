import json
import os
import re
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote

TEXTO = "text/plain; charset=utf-8"


class Manejador(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def responder(self, codigo, tipo, cuerpo):
        datos = cuerpo.encode("utf-8")
        self.send_response(codigo)
        self.send_header("Content-Type", tipo)
        self.send_header("Content-Length", str(len(datos)))
        self.end_headers()
        self.wfile.write(datos)

    def responder_json(self, codigo, dato):
        self.responder(codigo, "application/json", json.dumps(dato))

    def manejar(self):
        largo = int(self.headers.get("Content-Length") or 0)
        if largo:
            self.rfile.read(largo)
        ruta = self.path.split("?")[0]
        if self.command == "GET":
            if ruta == "/hola.txt":
                return self.responder(200, TEXTO, "hola\n")
            if ruta == "/estilo.css":
                return self.responder(200, "text/css; charset=utf-8", "body{margin:0}")
            if ruta == "/pagina":
                return self.responder(200, "text/html; charset=utf-8", "<h1>Pagina</h1>")
            if ruta == "/cabeceras":
                return self.responder_json(200, {"x_demo": self.headers.get("X-Demo")})
            m = re.fullmatch(r"/saludo/([^/]+)", ruta)
            if m:
                return self.responder(200, TEXTO, "Hola, " + unquote(m.group(1)))
            m = re.fullmatch(r"/suma/([^/]+)/([^/]+)", ruta)
            if m:
                a, b = unquote(m.group(1)), unquote(m.group(2))
                if re.fullmatch(r"-?[0-9]{1,15}", a) and re.fullmatch(r"-?[0-9]{1,15}", b):
                    return self.responder_json(200, {"a": int(a), "b": int(b), "suma": int(a) + int(b)})
                return self.responder_json(400, {"error": "numeros enteros"})
        self.responder(404, TEXTO, "no encontrado")

    do_GET = do_POST = do_PUT = do_DELETE = do_PATCH = manejar


ThreadingHTTPServer(("127.0.0.1", int(os.environ["PUERTO"])), Manejador).serve_forever()
