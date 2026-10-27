import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    pareja = {")": "(", "]": "[", "}": "{"}
    for linea in data.splitlines():
        pila = []
        ok = True
        for c in linea:
            if c in "([{":
                pila.append(c)
            elif c in ")]}":
                if not pila or pila.pop() != pareja[c]:
                    ok = False
                    break
        if ok and pila:
            ok = False
        print("si" if ok else "no")


main()
