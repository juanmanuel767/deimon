#!/usr/bin/env bash
# Mide cuántas tareas resuelve bien cada lenguaje en una ronda de soluciones.
# Uso: ./medir.sh soluciones/<ronda>    (dentro: deimon/NN-nombre.dmn, php/NN-nombre.php, python/NN-nombre.py)
# Una tarea cuenta como «bien» solo si TODOS sus casos (tareas/NN-nombre/casos/K.entrada) dan
# exactamente la salida de K.esperado. El caso 1 es el visible; los demás (2, 3...) son ocultos.
# El compilador se toma de la variable DEIMON (por defecto el binario ya construido del repositorio).
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
RONDA="${1:?Uso: ./medir.sh soluciones/<ronda>}"
DEIMON="${DEIMON:-$PWD/../compilador/target/release/deimon}"

ejecutar() {
    local lenguaje="$1" archivo="$2" entrada="$3"
    case "$lenguaje" in
        deimon) timeout 20 "$DEIMON" ejecutar "$archivo" < "$entrada" ;;
        php) timeout 20 php "$archivo" < "$entrada" ;;
        python) timeout 20 python3 -I "$archivo" < "$entrada" ;;
    esac
}

declare -A EXTENSION=([deimon]=dmn [php]=php [python]=py)
printf '%-24s' "tarea"
for lenguaje in deimon php python; do printf '%-12s' "$lenguaje"; done
echo
declare -A BIEN=() OCULTOS_MAL=() OCULTOS_TOTAL=()
total=0
for carpeta in tareas/*/; do
    tarea="$(basename "$carpeta")"
    total=$((total + 1))
    printf '%-24s' "$tarea"
    for lenguaje in deimon php python; do
        archivo="$RONDA/$lenguaje/$tarea.${EXTENSION[$lenguaje]}"
        if [ ! -f "$archivo" ]; then
            printf '%-12s' "falta"
            continue
        fi
        casos=0
        fallos=0
        for entrada in "$carpeta"casos/*.entrada; do
            numero="$(basename "$entrada" .entrada)"
            casos=$((casos + 1))
            if ! ejecutar "$lenguaje" "$archivo" "$entrada" 2>/dev/null | cmp -s - "$carpeta/casos/$numero.esperado"; then
                fallos=$((fallos + 1))
                if [ "$numero" != "1" ]; then
                    OCULTOS_MAL[$lenguaje]=$(( ${OCULTOS_MAL[$lenguaje]:-0} + 1 ))
                fi
            fi
        done
        OCULTOS_TOTAL[$lenguaje]=$(( ${OCULTOS_TOTAL[$lenguaje]:-0} + casos - 1 ))
        if [ "$fallos" -eq 0 ]; then
            printf '%-12s' "bien"
            BIEN[$lenguaje]=$(( ${BIEN[$lenguaje]:-0} + 1 ))
        else
            printf '%-12s' "MAL $fallos/$casos"
        fi
    done
    echo
done
printf '%-24s' "correctas"
for lenguaje in deimon php python; do printf '%-12s' "${BIEN[$lenguaje]:-0}/$total"; done
echo
printf '%-24s' "casos ocultos fallados"
for lenguaje in deimon php python; do
    printf '%-12s' "${OCULTOS_MAL[$lenguaje]:-0}/${OCULTOS_TOTAL[$lenguaje]:-0}"
done
echo
