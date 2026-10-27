#!/usr/bin/env bash
# Mide cuántas tareas resuelve bien cada lenguaje en una ronda de soluciones.
# Uso: ./medir.sh soluciones/<ronda>    (dentro: deimon/NN-nombre.dmn, php/NN-nombre.php, python/NN-nombre.py)
# Tareas 01-50 (entrada estándar -> salida estándar): cuentan como «bien» solo si TODOS sus casos
# (tareas/NN-nombre/casos/K.entrada) dan exactamente la salida de K.esperado. El caso 1 es el visible; los demás, ocultos.
# Tareas web (51-55, casos/K.peticiones.json): las mide herramientas/medir_web.py, que arranca el servidor de la
# solución, hace las peticiones, compara código, Content-Type y cuerpo, y lo mata siempre al acabar.
# Al final, la tabla general y una línea por bloque: general (01-40), especialidad de PHP (41-45), de Python (46-50)
# y de Deimon (51-55, webs).
# El compilador se toma de la variable DEIMON (por defecto el binario ya construido del repositorio).
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
RONDA="${1:?Uso: ./medir.sh soluciones/<ronda>}"
export DEIMON="${DEIMON:-$PWD/../compilador/target/release/deimon}"

ejecutar() {
    local lenguaje="$1" archivo="$2" entrada="$3"
    case "$lenguaje" in
        deimon) timeout 20 "$DEIMON" ejecutar "$archivo" < "$entrada" ;;
        php) timeout 20 php "$archivo" < "$entrada" ;;
        python) timeout 20 python3 -I "$archivo" < "$entrada" ;;
    esac
}

# Bloque de una tarea según su número: general (01-40), php (41-45), python (46-50), deimon (51-55).
bloque_de() {
    local n=$((10#${1%%-*}))
    if [ "$n" -le 40 ]; then echo general
    elif [ "$n" -le 45 ]; then echo php
    elif [ "$n" -le 50 ]; then echo python
    else echo deimon
    fi
}
declare -A ROTULO=([general]="general (01-40)" [php]="especialidad PHP (41-45)" [python]="especialidad Python (46-50)" [deimon]="especialidad Deimon (51-55)")
BLOQUES=(general php python deimon)

declare -A EXTENSION=([deimon]=dmn [php]=php [python]=py)
printf '%-28s' "tarea"
for lenguaje in deimon php python; do printf '%-12s' "$lenguaje"; done
echo
declare -A BIEN=() OCULTOS_MAL=() OCULTOS_TOTAL=() BIEN_B=() TOTAL_B=()
total=0
for carpeta in tareas/*/; do
    tarea="$(basename "$carpeta")"
    bloque="$(bloque_de "$tarea")"
    total=$((total + 1))
    TOTAL_B[$bloque]=$(( ${TOTAL_B[$bloque]:-0} + 1 ))
    printf '%-28s' "$tarea"
    for lenguaje in deimon php python; do
        archivo="$RONDA/$lenguaje/$tarea.${EXTENSION[$lenguaje]}"
        if [ ! -f "$archivo" ]; then
            printf '%-12s' "falta"
            continue
        fi
        casos=0
        fallos=0
        if [ -f "$carpeta/casos/1.peticiones.json" ]; then
            read -r casos fallos ocultos_mal < <(python3 -I herramientas/medir_web.py "$lenguaje" "$archivo" "$carpeta" 2>/dev/null || echo "1 1 0")
            OCULTOS_MAL[$lenguaje]=$(( ${OCULTOS_MAL[$lenguaje]:-0} + ocultos_mal ))
        else
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
        fi
        OCULTOS_TOTAL[$lenguaje]=$(( ${OCULTOS_TOTAL[$lenguaje]:-0} + casos - 1 ))
        if [ "$fallos" -eq 0 ]; then
            printf '%-12s' "bien"
            BIEN[$lenguaje]=$(( ${BIEN[$lenguaje]:-0} + 1 ))
            BIEN_B[$lenguaje:$bloque]=$(( ${BIEN_B[$lenguaje:$bloque]:-0} + 1 ))
        else
            printf '%-12s' "MAL $fallos/$casos"
        fi
    done
    echo
done
printf '%-28s' "correctas"
for lenguaje in deimon php python; do printf '%-12s' "${BIEN[$lenguaje]:-0}/$total"; done
echo
printf '%-28s' "casos ocultos fallados"
for lenguaje in deimon php python; do
    printf '%-12s' "${OCULTOS_MAL[$lenguaje]:-0}/${OCULTOS_TOTAL[$lenguaje]:-0}"
done
echo
echo
printf '%-28s' "por bloque"
for lenguaje in deimon php python; do printf '%-12s' "$lenguaje"; done
echo
for bloque in "${BLOQUES[@]}"; do
    printf '%-28s' "${ROTULO[$bloque]}"
    for lenguaje in deimon php python; do
        printf '%-12s' "${BIEN_B[$lenguaje:$bloque]:-0}/${TOTAL_B[$bloque]:-0}"
    done
    echo
done
