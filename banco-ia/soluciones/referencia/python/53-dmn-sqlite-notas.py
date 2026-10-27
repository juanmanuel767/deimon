import os
import sqlite3
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs

BASE = os.path.join(os.environ["DATOS"], "datos.db")


def esc(t):
    return (t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            .replace('"', "&quot;").replace("'", "&#039;"))


def abrir():
    con = sqlite3.connect(BASE, timeout=10)
    con.execute("CREATE TABLE IF NOT EXISTS notas (id INTEGER PRIMARY KEY AUTOINCREMENT, texto TEXT NOT NULL)")
    con.execute("CREATE TABLE IF NOT EXISTS visitas (id INTEGER PRIMARY KEY, total INTEGER NOT NULL)")
    return con


class Manejador(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def responder(self, texto):
        cuerpo = texto.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(cuerpo)))
        self.end_headers()
        self.wfile.write(cuerpo)

    def do_GET(self):
        ruta = self.path.split("?")[0]
        con = abrir()
        try:
            if ruta == "/notas":
                filas = con.execute("SELECT texto FROM notas ORDER BY id").fetchall()
                if not filas:
                    return self.responder("<p>sin notas</p>")
                return self.responder("<ol>" + "".join(f"<li>{esc(t)}</li>" for (t,) in filas) + "</ol>")
            if ruta == "/visitas":
                con.execute("INSERT OR IGNORE INTO visitas (id, total) VALUES (1, 0)")
                con.execute("UPDATE visitas SET total = total + 1 WHERE id = 1")
                con.commit()
                (total,) = con.execute("SELECT total FROM visitas WHERE id = 1").fetchone()
                return self.responder(f"<p>visitas: {total}</p>")
        finally:
            con.close()
        self.send_error(404)

    def do_POST(self):
        largo = int(self.headers.get("Content-Length") or 0)
        cuerpo = self.rfile.read(largo).decode("utf-8") if largo else ""
        if self.path.split("?")[0] != "/notas":
            return self.send_error(404)
        texto = parse_qs(cuerpo, keep_blank_values=True).get("texto", [""])[0].strip()
        if not texto:
            return self.responder("<p>texto vacio</p>")
        con = abrir()
        try:
            cursor = con.execute("INSERT INTO notas (texto) VALUES (?)", (texto,))
            con.commit()
            self.responder(f"<p>nota {cursor.lastrowid} guardada</p>")
        finally:
            con.close()


ThreadingHTTPServer(("127.0.0.1", int(os.environ["PUERTO"])), Manejador).serve_forever()
