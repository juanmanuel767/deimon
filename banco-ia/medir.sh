#!/usr/bin/env bash
# Mide cuántas tareas resuelve bien cada lenguaje en una ronda de soluciones.
# Uso: ./medir.sh soluciones/<ronda>    (dentro: deimon/NN-nombre.dmn, php/NN-nombre.php, python/NN-nombre.py)
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
RONDA="${1:?Uso: ./medir.sh soluciones/<ronda>}"

ejecutar() {
    local lenguaje="$1" archivo="$2" entrada="$3"
    case "$lenguaje" in
        deimon) timeout 20 deimon ejecutar "$archivo" < "$entrada" ;;
        php) timeout 20 php "$archivo" < "$entrada" ;;
        python) timeout 20 python3 -I "$archivo" < "$entrada" ;;
    esac
}

declare -A EXTENSION=([deimon]=dmn [php]=php [python]=py)
printf '%-16s' "tarea"
for lenguaje in deimon php python; do printf '%-9s' "$lenguaje"; done
echo
declare -A BIEN=()
total=0
for carpeta in tareas/*/; do
    tarea="$(basename "$carpeta")"
    total=$((total + 1))
    printf '%-16s' "$tarea"
    for lenguaje in deimon php python; do
        archivo="$RONDA/$lenguaje/$tarea.${EXTENSION[$lenguaje]}"
        if [ ! -f "$archivo" ]; then
            printf '%-9s' "falta"
            continue
        fi
        if ejecutar "$lenguaje" "$archivo" "$carpeta/entrada.txt" 2>/dev/null | cmp -s - "$carpeta/esperado.txt"; then
            printf "%-9s" "bien"
            BIEN[$lenguaje]=$(( ${BIEN[$lenguaje]:-0} + 1 ))
        else
            printf "%-9s" "MAL"
        fi
    done
    echo
done
printf '%-16s' "correctas"
for lenguaje in deimon php python; do printf '%-9s' "${BIEN[$lenguaje]:-0}/$total"; done
echo
