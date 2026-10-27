#!/usr/bin/env python3
"""Arnés de las tareas web (51-55): arranca el servidor de una solución, espera a que el puerto responda
(máximo 10 s), lanza las peticiones de cada caso, compara y SIEMPRE mata el servidor.

Uso: python3 herramientas/medir_web.py <deimon|php|python> <archivo> <carpeta_de_la_tarea> [-v]
Imprime una línea: «CASOS FALLADOS OCULTOS_FALLADOS» (el caso 1 es el visible; los demás, ocultos).
Con -v cuenta en stderr qué petición falló y por qué. Código de salida 0 siempre que el arnés funcione.
La variable DEIMON elige el compilador (por defecto ../compilador/target/release/deimon).

Contrato (ver README.md): el servidor escucha en 127.0.0.1:$PUERTO (el arnés también pone DEIMON_PUERTO),
la carpeta de trabajo y la variable DATOS (también DEIMON_DATOS) son una carpeta temporal vacía por caso."""
import glob
import http.client
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

RAIZ = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
DEIMON = os.environ.get("DEIMON") or os.path.join(RAIZ, "..", "compilador", "target", "release", "deimon")
ESPERA_ARRANQUE = 10
MAX_CASO = 60
MAX_PETICION = 10


def puerto_libre():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def orden(lenguaje, archivo, puerto):
    archivo = os.path.abspath(archivo)
    if lenguaje == "deimon":
        return [os.path.abspath(DEIMON), "ejecutar", archivo]
    if lenguaje == "php":
        return ["php", "-S", f"127.0.0.1:{puerto}", archivo]
    return ["python3", "-I", archivo]


class Servidor:
    """El servidor de un caso; se puede reiniciar con la misma carpeta de datos."""

    def __init__(self, lenguaje, archivo, datos):
        self.lenguaje, self.archivo, self.datos = lenguaje, archivo, datos
        self.puerto = puerto_libre()
        self.proceso = None

    def arrancar(self):
        entorno = dict(os.environ, PUERTO=str(self.puerto), DEIMON_PUERTO=str(self.puerto),
                       DATOS=self.datos, DEIMON_DATOS=self.datos)
        self.proceso = subprocess.Popen(orden(self.lenguaje, self.archivo, self.puerto), cwd=self.datos, env=entorno,
                                        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                        start_new_session=True)
        limite = time.monotonic() + ESPERA_ARRANQUE
        while time.monotonic() < limite:
            if self.proceso.poll() is not None:
                raise RuntimeError(f"el servidor terminó al arrancar (código {self.proceso.returncode})")
            try:
                with socket.create_connection(("127.0.0.1", self.puerto), timeout=0.5):
                    return
            except OSError:
                time.sleep(0.05)
        raise RuntimeError(f"el puerto {self.puerto} no respondió en {ESPERA_ARRANQUE} s")

    def parar(self):
        p, self.proceso = self.proceso, None
        if p is None:
            return
        for senal in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.killpg(p.pid, senal)
            except (ProcessLookupError, PermissionError):
                break
            try:
                p.wait(timeout=2)
                break
            except subprocess.TimeoutExpired:
                continue
        try:
            os.killpg(p.pid, signal.SIGKILL)  # por si quedó algún hijo
        except (ProcessLookupError, PermissionError):
            pass
        try:
            p.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass


def normalizar_tipo(t):
    return "".join((t or "").lower().split())


def igual_json(a, b):
    """Como ==, pero distingue True de 1 y 1.0 de 1 solo cuando es lógico; los números se comparan por valor."""
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(igual_json(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(igual_json(x, y) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    return type(a) is type(b) and a == b


def actualizar_galletas(jar, cabeceras):
    for valor in cabeceras:
        par = valor.split(";")[0]
        if "=" not in par:
            continue
        nombre, _, v = par.partition("=")
        nombre, v = nombre.strip(), v.strip()
        atributos = [a.strip().lower() for a in valor.split(";")[1:]]
        if v == "" or "max-age=0" in atributos or any(a.startswith("max-age=-") for a in atributos):
            jar.pop(nombre, None)
        else:
            jar[nombre] = v


def hacer_peticion(servidor, jar, p):
    cabeceras = dict(p.get("cabeceras") or {})
    if not any(k.lower() == "cookie" for k in cabeceras) and jar:
        cabeceras["Cookie"] = "; ".join(f"{k}={v}" for k, v in jar.items())
    cabeceras.setdefault("Connection", "close")
    cuerpo = p["cuerpo"].encode("utf-8") if "cuerpo" in p else None
    conexion = http.client.HTTPConnection("127.0.0.1", servidor.puerto, timeout=MAX_PETICION)
    try:
        conexion.request(p["metodo"], p["ruta"], body=cuerpo, headers=cabeceras)
        r = conexion.getresponse()
        datos = r.read().decode("utf-8", errors="replace")
        return r.status, r.getheader("Content-Type"), datos, r.msg.get_all("Set-Cookie") or []
    finally:
        conexion.close()


def comparar(esperado, estado, tipo, cuerpo):
    """Devuelve None si coincide, o el motivo del fallo."""
    if estado != esperado["codigo"]:
        return f"código {estado}, se esperaba {esperado['codigo']} (cuerpo: {cuerpo[:120]!r})"
    if normalizar_tipo(tipo) != normalizar_tipo(esperado["tipo"]):
        return f"Content-Type {tipo!r}, se esperaba {esperado['tipo']!r}"
    if "json" in esperado:
        try:
            dato = json.loads(cuerpo)
        except ValueError:
            return f"el cuerpo no es JSON: {cuerpo[:120]!r}"
        if not igual_json(dato, esperado["json"]):
            return f"JSON {dato!r}, se esperaba {esperado['json']!r}"
    elif cuerpo != esperado["cuerpo"]:
        return f"cuerpo {cuerpo[:200]!r}, se esperaba {esperado['cuerpo'][:200]!r}"
    return None


def ejecutar_caso(lenguaje, archivo, peticiones, esperado):
    """Devuelve None si el caso pasa, o un texto con el primer fallo."""
    datos = tempfile.mkdtemp(prefix="banco-web-")
    servidor = Servidor(lenguaje, archivo, datos)
    jares = {}
    fin = time.monotonic() + MAX_CASO
    try:
        servidor.arrancar()
        for i, (p, e) in enumerate(zip(peticiones, esperado), 1):
            if time.monotonic() > fin:
                return f"el caso tardó más de {MAX_CASO} s"
            if p.get("accion") == "reiniciar":
                servidor.parar()
                servidor.arrancar()
                continue
            jar = jares.setdefault(p.get("cliente", "A"), {})
            try:
                estado, tipo, cuerpo, set_cookies = hacer_peticion(servidor, jar, p)
            except (OSError, http.client.HTTPException) as error:
                return f"petición {i} ({p['metodo']} {p['ruta']}): {type(error).__name__}: {error}"
            actualizar_galletas(jar, set_cookies)
            motivo = comparar(e, estado, tipo, cuerpo)
            if motivo:
                return f"petición {i} ({p['metodo']} {p['ruta']}): {motivo}"
        return None
    except RuntimeError as error:
        return str(error)
    finally:
        servidor.parar()
        shutil.rmtree(datos, ignore_errors=True)


def casos_de(carpeta):
    """Lista ordenada de (k, peticiones, esperado) de una tarea web."""
    ks = sorted(int(os.path.basename(f).split(".")[0])
                for f in glob.glob(os.path.join(carpeta, "casos", "*.peticiones.json")))
    out = []
    for k in ks:
        with open(os.path.join(carpeta, "casos", f"{k}.peticiones.json"), encoding="utf-8") as f:
            peticiones = json.load(f)
        with open(os.path.join(carpeta, "casos", f"{k}.esperado.json"), encoding="utf-8") as f:
            esperado = json.load(f)
        out.append((k, peticiones, esperado))
    return out


def medir_tarea(lenguaje, archivo, carpeta, detalle=None):
    """Devuelve (casos, fallados, ocultos_fallados)."""
    casos = fallos = ocultos = 0
    for k, peticiones, esperado in casos_de(carpeta):
        casos += 1
        motivo = ejecutar_caso(lenguaje, archivo, peticiones, esperado)
        if motivo:
            fallos += 1
            if k != 1:
                ocultos += 1
            if detalle:
                detalle(f"  caso {k}: {motivo}")
    return casos, fallos, ocultos


def main():
    args = [a for a in sys.argv[1:] if a != "-v"]
    if len(args) != 3 or args[0] not in ("deimon", "php", "python"):
        sys.exit(__doc__)
    detalle = (lambda t: print(t, file=sys.stderr)) if "-v" in sys.argv else None
    c, f, o = medir_tarea(args[0], args[1], args[2], detalle)
    print(c, f, o)


if __name__ == "__main__":
    main()
