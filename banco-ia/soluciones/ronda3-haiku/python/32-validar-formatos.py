import datetime
import re
import string
import sys

LETRAS = string.ascii_lowercase
LOCAL = string.ascii_lowercase + string.digits + "._-"
DOMINIO = string.ascii_lowercase + string.digits + "-"


def correo_valido(valor):
    if valor.count("@") != 1:
        return False
    local, dominio = valor.split("@")
    if not local or any(c not in LOCAL for c in local):
        return False
    partes = dominio.split(".")
    if len(partes) < 2:
        return False
    for parte in partes:
        if not parte or parte[0] == "-" or parte[-1] == "-":
            return False
        if any(c not in DOMINIO for c in parte):
            return False
    ultima = partes[-1]
    return len(ultima) >= 2 and all(c in LETRAS for c in ultima)


def telefono_valido(valor):
    return re.fullmatch(r"[6-9][0-9]{8}", valor) is not None


def fecha_valida(valor):
    m = re.fullmatch(r"([0-9]{2})/([0-9]{2})/([0-9]{4})", valor)
    if not m:
        return False
    dia, mes, anio = map(int, m.groups())
    if not 1900 <= anio <= 2100:
        return False
    try:
        datetime.date(anio, mes, dia)
    except ValueError:
        return False
    return True


def cp_valido(valor):
    if not re.fullmatch(r"[0-9]{5}", valor):
        return False
    return 1 <= int(valor[:2]) <= 52


VALIDADORES = {
    "correo": correo_valido,
    "telefono": telefono_valido,
    "fecha": fecha_valida,
    "cp": cp_valido,
}


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        texto = linea.strip()
        if not texto:
            continue
        tipo, _, valor = texto.partition(" ")
        validador = VALIDADORES.get(tipo)
        ok = validador is not None and validador(valor)
        print(f"{texto}: {'valido' if ok else 'invalido'}")


main()
