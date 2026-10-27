import json
import os
import re
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TAREAS = []
CANDADO = threading.Lock()


class Manejador(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def responder(self, codigo, dato):
        cuerpo = json.dumps(dato, ensure_ascii=False).encode("utf-8")
        self.send_response(codigo)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(cuerpo)))
        self.end_headers()
        self.wfile.write(cuerpo)

    def manejar(self):
        ruta = self.path.split("?")[0]
        largo = int(self.headers.get("Content-Length") or 0)
        cuerpo = self.rfile.read(largo) if largo else b""
        with CANDADO:
            if ruta == "/api/tareas" and self.command == "GET":
                return self.responder(200, TAREAS)
            if ruta == "/api/tareas" and self.command == "POST":
                try:
                    dato = json.loads(cuerpo.decode("utf-8"))
                except ValueError:
                    dato = None
                if not isinstance(dato, dict) or not isinstance(dato.get("titulo"), str) or not dato["titulo"].strip():
                    return self.responder(400, {"error": "peticion invalida"})
                tarea = {"id": len(TAREAS) + 1, "titulo": dato["titulo"], "hecha": False}
                TAREAS.append(tarea)
                return self.responder(201, tarea)
            m = re.fullmatch(r"/api/tareas/([^/]+)", ruta)
            if m and self.command == "GET":
                return self.responder(*self.buscar(m.group(1)))
            m = re.fullmatch(r"/api/tareas/([^/]+)/hecha", ruta)
            if m and self.command == "POST":
                codigo, dato = self.buscar(m.group(1))
                if codigo == 200:
                    dato["hecha"] = True
                return self.responder(codigo, dato)
            return self.responder(404, {"error": "no encontrado"})

    def buscar(self, texto):
        if re.fullmatch(r"[1-9][0-9]*", texto) and int(texto) <= len(TAREAS):
            return 200, TAREAS[int(texto) - 1]
        return 404, {"error": "no existe"}

    do_GET = do_POST = do_PUT = do_DELETE = do_PATCH = manejar


ThreadingHTTPServer(("127.0.0.1", int(os.environ["PUERTO"])), Manejador).serve_forever()
