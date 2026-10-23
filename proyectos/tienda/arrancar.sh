#!/usr/bin/env bash
# Tienda Deimon:  ./arrancar.sh          arranca la tienda (http://127.0.0.1:8130)
#                 ./arrancar.sh probar   ejecuta las pruebas
set -euo pipefail
cd "$(dirname "$0")"
command -v deimon >/dev/null || { echo "Falta Deimon: instálalo con ../../instalar.sh" >&2; exit 1; }
case "${1:-}" in
    probar) deimon probar tienda.dmn ;;
    "") echo "Tienda en http://127.0.0.1:${DEIMON_PUERTO:-8130} (Ctrl+C para parar)"; deimon ejecutar tienda.dmn ;;
    *) echo "Uso: ./arrancar.sh [probar]" >&2; exit 1 ;;
esac
