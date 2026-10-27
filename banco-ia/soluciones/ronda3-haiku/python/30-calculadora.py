import sys


def tokenizar(texto):
    tokens = []
    i = 0
    while i < len(texto):
        c = texto[i]
        if c.isspace():
            i += 1
        elif "0" <= c <= "9":
            j = i
            while j < len(texto) and "0" <= texto[j] <= "9":
                j += 1
            tokens.append(int(texto[i:j]))
            i = j
        else:
            tokens.append(c)
            i += 1
    return tokens


def dividir(a, b):
    # Division entera truncando hacia cero. Si b es 0, lanza ZeroDivisionError.
    cociente = abs(a) // abs(b)
    return cociente if (a < 0) == (b < 0) else -cociente


class Analizador:
    def __init__(self, tokens):
        self.tokens = tokens
        self.pos = 0

    def mirar(self):
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def avanzar(self):
        tok = self.mirar()
        self.pos += 1
        return tok

    def expresion(self):
        valor = self.termino()
        while self.mirar() in ("+", "-"):
            op = self.avanzar()
            derecha = self.termino()
            valor = valor + derecha if op == "+" else valor - derecha
        return valor

    def termino(self):
        valor = self.unario()
        while self.mirar() in ("*", "/"):
            op = self.avanzar()
            derecha = self.unario()
            valor = valor * derecha if op == "*" else dividir(valor, derecha)
        return valor

    def unario(self):
        if self.mirar() == "-":
            self.avanzar()
            return -self.unario()
        if self.mirar() == "+":
            self.avanzar()
            return self.unario()
        return self.primario()

    def primario(self):
        tok = self.avanzar()
        if isinstance(tok, int):
            return tok
        if tok == "(":
            valor = self.expresion()
            self.avanzar()  # cierra el parentesis
            return valor
        raise ValueError("expresion mal formada")


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    for linea in data.splitlines():
        if not linea.strip():
            continue
        try:
            valor = Analizador(tokenizar(linea)).expresion()
            print(valor)
        except ZeroDivisionError:
            print("error: division por cero")


main()
