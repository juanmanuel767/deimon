#!/usr/bin/env bash
# Arranca el proyecto deimon. Se puede lanzar desde cualquier carpeta.
set -euo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILADOR="$RAIZ/compilador"
BIN="$COMPILADOR/target/release/deimon"
TEMPORALES="$COMPILADOR/target/tmp"

comprobar_cargo() {
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Error: no se encuentra 'cargo'. Instala Rust desde https://rustup.rs y vuelve a intentarlo." >&2
        exit 1
    fi
}

preparar() {
    comprobar_cargo
    echo "Compilando el traductor (modo release)..."
    cargo build --release --manifest-path "$COMPILADOR/Cargo.toml"
    echo "Listo: $BIN"
}

asegurar_traductor() {
    if [ ! -x "$BIN" ]; then
        echo "El traductor no está compilado todavía."
        preparar
    fi
}

probar() {
    comprobar_cargo
    mkdir -p "$TEMPORALES"
    echo "Ejecutando todas las pruebas del compilador..."
    TMPDIR="$TEMPORALES" cargo test --manifest-path "$COMPILADOR/Cargo.toml"
}

vivo() {
    asegurar_traductor
    echo "Abriendo las pruebas en vivo (Ctrl+C para salir)..."
    bash "$RAIZ/herramientas/ver-pruebas.sh"
}

ejemplo() {
    local nombre="${1:-hola}"
    asegurar_traductor
    local archivo="$RAIZ/ejemplos/$nombre.dmn"
    if [ ! -f "$archivo" ]; then
        echo "Error: no existe el ejemplo '$nombre' ($archivo)." >&2
        echo "Ejemplos disponibles:" >&2
        for f in "$RAIZ"/ejemplos/*.dmn; do
            echo "  - $(basename "$f" .dmn)" >&2
        done
        exit 1
    fi
    echo "Ejecutando ejemplos/$nombre.dmn..."
    "$BIN" ejecutar "$archivo"
}

web() {
    asegurar_traductor
    local puerto="${DEIMON_PUERTO:-8150}"
    echo "Arrancando la web en http://127.0.0.1:$puerto (Ctrl+C para parar)..."
    DEIMON_PUERTO="$puerto" "$BIN" ejecutar "$RAIZ/ejemplos-web/sitio.dmn"
}

entorno() {
    asegurar_traductor
    "$BIN" entorno
}

iconos() {
    # Hace que el explorador de archivos muestre los .dmn con el icono del demonio.
    local datos="${XDG_DATA_HOME:-$HOME/.local/share}"
    mkdir -p "$datos/mime/packages"
    cp "$RAIZ/herramientas/deimon-mime.xml" "$datos/mime/packages/deimon.xml"
    local tamano
    for tamano in 48 128 256; do
        mkdir -p "$datos/icons/hicolor/${tamano}x${tamano}/mimetypes"
        cp "$RAIZ/herramientas/icono-$tamano.png" \
            "$datos/icons/hicolor/${tamano}x${tamano}/mimetypes/text-x-deimon.png"
    done
    if command -v update-mime-database >/dev/null 2>&1; then
        update-mime-database "$datos/mime"
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$datos/icons/hicolor" >/dev/null 2>&1 || true
    fi
    echo "Listo: los archivos .dmn se verán con el icono de Deimon (puede hacer falta cerrar y abrir el explorador)."
}

ayuda() {
    cat <<'TEXTO'
Uso: ./arrancar.sh <orden>

Órdenes:
  preparar         compila el traductor en modo release
  probar           ejecuta todas las pruebas del compilador
  vivo             abre las pruebas en vivo (herramientas/ver-pruebas.sh)
  ejemplo [nombre] ejecuta ejemplos/<nombre>.dmn (por defecto: hola)
  web              arranca ejemplos-web/sitio.dmn (DEIMON_PUERTO, por defecto 8150)
  entorno          revisa el ordenador con 'deimon entorno'
  iconos           muestra los archivos .dmn con el icono de Deimon en el explorador
  ayuda            muestra esta ayuda
TEXTO
}

case "${1:-ayuda}" in
    preparar) preparar ;;
    probar) probar ;;
    vivo) vivo ;;
    ejemplo) ejemplo "${2:-hola}" ;;
    web) web ;;
    entorno) entorno ;;
    iconos) iconos ;;
    ayuda) ayuda ;;
    *)
        echo "Error: la orden '${1}' no existe." >&2
        ayuda >&2
        exit 2
        ;;
esac
