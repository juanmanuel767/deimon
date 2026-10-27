import sys
from decimal import Decimal, ROUND_HALF_UP

CENTIMO = Decimal("0.01")


def media(notas):
    return (sum(notas, Decimal("0")) / len(notas)).quantize(CENTIMO, rounding=ROUND_HALF_UP)


def calificacion(m):
    if m < 5:
        return "suspenso"
    if m < 7:
        return "aprobado"
    if m < 9:
        return "notable"
    return "sobresaliente"


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    data = sys.stdin.read()
    por_alumno = {}
    por_asignatura = {}
    for linea in data.splitlines():
        partes = linea.strip().split(";")
        if len(partes) != 3:
            continue
        alumno, asignatura, nota = partes
        valor = Decimal(nota.strip())
        por_alumno.setdefault(alumno, []).append(valor)
        por_asignatura.setdefault(asignatura, []).append(valor)

    print("== alumnos ==")
    medias = {}
    for alumno in sorted(por_alumno):
        m = media(por_alumno[alumno])
        medias[alumno] = m
        print(f"{alumno}: {m:.2f} {calificacion(m)}")

    print("== asignaturas ==")
    for asignatura in sorted(por_asignatura):
        print(f"{asignatura}: {media(por_asignatura[asignatura]):.2f}")

    if not medias:
        print("mejor alumno: nadie")
        return
    mejor = None
    for alumno in sorted(medias):
        if mejor is None or medias[alumno] > medias[mejor]:
            mejor = alumno
    print(f"mejor alumno: {mejor}")


main()
