import sys
from decimal import Decimal


def euros(centimos):
    return f"{centimos // 100}.{centimos % 100:02d}"


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    subtotal = 0
    for linea in data.splitlines():
        partes = linea.split()
        if len(partes) < 3:
            continue
        producto = partes[0]
        cantidad = int(partes[1])
        precio = int(Decimal(partes[2]) * 100)
        bruto = cantidad * precio
        if cantidad >= 10:
            descuento = 10
        elif cantidad >= 5:
            descuento = 5
        else:
            descuento = 0
        importe = (bruto * (100 - descuento) + 50) // 100
        subtotal += importe
        texto = f"{producto} x{cantidad}: {euros(importe)}"
        if descuento:
            texto += f" (-{descuento}%)"
        print(texto)

    print(f"subtotal: {euros(subtotal)}")
    if subtotal >= 10000:
        total = (subtotal * 95 + 50) // 100
        print(f"descuento pedido: -{euros(subtotal - total)}")
    else:
        total = subtotal
    envio = 0 if total >= 5000 else 495
    print(f"envio: {euros(envio)}")
    print(f"total: {euros(total + envio)}")


main()
