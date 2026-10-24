import sys

pares = {")": "(", "]": "[", "}": "{"}
aperturas = set(pares.values())

for linea in sys.stdin.read().splitlines():
    pila = []
    ok = True
    for c in linea:
        if c in aperturas:
            pila.append(c)
        elif c in pares:
            if not pila or pila[-1] != pares[c]:
                ok = False
                break
            pila.pop()
    if ok and not pila:
        print("si")
    else:
        print("no")
