import os
import secrets
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs

USUARIOS = {"ana": "pez12345", "luis": "sol45678"}
SESIONES = {}
CANDADO = threading.Lock()


class Manejador(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def sesion(self):
        """Devuelve (sesión, cabecera Set-Cookie o None); una galleta desconocida es una sesión nueva."""
        galletas = {}
        for trozo in (self.headers.get("Cookie") or "").split(";"):
            if "=" in trozo:
                k, _, v = trozo.strip().partition("=")
                galletas[k] = v
        sid = galletas.get("sid")
        if sid in SESIONES:
            return SESIONES[sid], None
        sid = secrets.token_hex(16)
        SESIONES[sid] = {"usuario": None, "pasos": 0}
        return SESIONES[sid], f"sid={sid}; Path=/; HttpOnly"

    def responder(self, texto, nueva):
        cuerpo = texto.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(cuerpo)))
        if nueva:
            self.send_header("Set-Cookie", nueva)
        self.end_headers()
        self.wfile.write(cuerpo)

    def manejar(self):
        largo = int(self.headers.get("Content-Length") or 0)
        cuerpo = self.rfile.read(largo).decode("utf-8") if largo else ""
        ruta = self.path.split("?")[0]
        with CANDADO:
            ses, nueva = self.sesion()
            if self.command == "POST" and ruta == "/entrar":
                campos = parse_qs(cuerpo, keep_blank_values=True)
                nombre = campos.get("nombre", [""])[0]
                if USUARIOS.get(nombre) is not None and USUARIOS[nombre] == campos.get("clave", [""])[0]:
                    ses["usuario"] = nombre
                    return self.responder(f"<p>Hola, {nombre}</p>", nueva)
                return self.responder("<p>Usuario o clave incorrectos</p>", nueva)
            if self.command == "POST" and ruta == "/salir":
                ses["usuario"] = None
                return self.responder("<p>Adios</p>", nueva)
            if self.command == "GET" and ruta == "/yo":
                return self.responder(f"<p>Eres {ses['usuario']}</p>" if ses["usuario"] else "<p>No has entrado</p>", nueva)
            if self.command == "GET" and ruta == "/contar":
                ses["pasos"] += 1
                return self.responder(f"<p>pasos: {ses['pasos']}</p>", nueva)
        self.send_error(404)

    do_GET = do_POST = manejar


ThreadingHTTPServer(("127.0.0.1", int(os.environ["PUERTO"])), Manejador).serve_forever()
