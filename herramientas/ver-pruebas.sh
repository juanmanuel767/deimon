#!/usr/bin/env bash
# Muestra las pruebas en vivo: cada vez que cambia un archivo .dmn o el compilador,
# vuelve a compilar y a ejecutar todos los ejemplos.
AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 1
RAIZ="$(cd "$AQUI/.." && pwd)" || exit 1
BIN="$RAIZ/compilador/target/release/deimon"

verde=$'\e[32m'; rojo=$'\e[31m'; gris=$'\e[90m'; neg=$'\e[1m'; fin=$'\e[0m'

probar() {
    clear
    echo "${neg}Pruebas en vivo — $(date +%H:%M:%S)${fin}"
    echo "${gris}(se repiten solas al guardar cualquier cambio; Ctrl+C para salir)${fin}"
    echo
    if ! cargo build --release -q --manifest-path "$RAIZ/compilador/Cargo.toml" 2>&1; then
        echo "${rojo}✗ El compilador tiene errores${fin}"
        return
    fi
    local bien=0 mal=0
    for f in "$RAIZ"/ejemplos/*.dmn; do
        local nombre esperado salida
        nombre="$(basename "$f" .dmn)"
        esperado="$RAIZ/ejemplos/$nombre.esperado"
        echo "${neg}▶ $nombre${fin}"
        salida="$("$BIN" ejecutar "$f" 2>&1)"
        echo "$salida" | sed 's/^/   /'
        if [ -f "$esperado" ] && [ "$salida" == "$(cat "$esperado")" ]; then
            echo "   ${verde}✓ correcto${fin}"; bien=$((bien + 1))
        elif [ -f "$esperado" ]; then
            echo "   ${rojo}✗ no coincide con lo esperado${fin}"; mal=$((mal + 1))
        fi
        echo
    done
    echo "${neg}Resultado: ${verde}$bien correctas${fin}${neg}, ${rojo}$mal fallidas${fin}"

    local hay_pruebas=0
    for f in "$RAIZ"/ejemplos/*.dmn "$RAIZ"/ejemplos-web/*.dmn; do
        grep -qF 'prueba "' "$f" || continue
        if [ "$hay_pruebas" -eq 0 ]; then
            echo
            echo "${neg}Pruebas dentro de los programas${fin}"
            hay_pruebas=1
        fi
        echo "${neg}▶ $f${fin}"
        "$BIN" probar "$f" 2>&1 | while IFS= read -r linea; do
            case "$linea" in
                "✓"*) echo "   ${verde}$linea${fin}" ;;
                "✗"*) echo "   ${rojo}$linea${fin}" ;;
                *) echo "   $linea" ;;
            esac
        done
    done
}

probar
while inotifywait -qq -r -e close_write,create,moved_to "$RAIZ/ejemplos" "$RAIZ/ejemplos-web" "$RAIZ/biblioteca" "$RAIZ/compilador/src"; do
    sleep 0.3
    probar
done
