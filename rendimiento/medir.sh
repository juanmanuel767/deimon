#!/usr/bin/env bash
# Mide el tiempo de compilación y de ejecución de los programas de esta carpeta.
# Se puede lanzar desde cualquier carpeta: usa su propia ubicación para encontrar
# el traductor y guarda los temporales en compilador/target/tmp/rendimiento.

set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILADOR="$AQUI/../compilador"
LN="$COMPILADOR/target/release/deimon"
TRABAJO="$COMPILADOR/target/tmp/rendimiento"

mkdir -p "$COMPILADOR/target/tmp" "$TRABAJO"
export TMPDIR="$COMPILADOR/target/tmp"

if [ ! -x "$LN" ]; then
    echo "El traductor no está compilado; compilando con cargo build --release..."
    (cd "$COMPILADOR" && cargo build --release)
fi

ms_actual() {
    date +%s%N
}

segundos() {
    awk -v ms="$1" 'BEGIN { printf "%.3f", ms / 1000000000 }'
}

medir() {
    local nombre="$1"
    local fuente="$AQUI/$nombre.dmn"
    local generado="$TRABAJO/${nombre}_gen.rs"
    local binario="$TRABAJO/prog_$nombre"
    local salida="$TRABAJO/${nombre}.salida"

    local inicio fin compilacion ejecucion resultado

    inicio="$(ms_actual)"
    "$LN" traducir "$fuente" > "$generado"
    rustc -O --crate-name prog "$generado" -o "$binario"
    fin="$(ms_actual)"
    compilacion="$(segundos "$((fin - inicio))")"

    inicio="$(ms_actual)"
    (cd "$TRABAJO" && "$binario") > "$salida"
    fin="$(ms_actual)"
    ejecucion="$(segundos "$((fin - inicio))")"

    resultado="$(cat "$salida")"
    printf '%-10s %14s %12s   %s\n' "$nombre" "$compilacion s" "$ejecucion s" "$resultado"
}

echo "Rendimiento de Deimon"
echo "Traductor: $LN"
echo
printf '%-10s %14s %12s   %s\n' "Programa" "Compilación" "Ejecución" "Resultado"
printf '%s\n' "------------------------------------------------------------"
medir fib
medir bucle
medir lista
medir texto
