#!/usr/bin/env bash
# Quita lo que instaló ./instalar.sh. No toca tus programas .dmn ni tus datos ni Rust.
set -euo pipefail

DESTINO="${DEIMON_DESTINO:-$HOME/.local/bin}"
DATOS="${XDG_DATA_HOME:-$HOME/.local/share}"

rm -f "$DESTINO/deimon"
rm -f "$DATOS/mime/packages/deimon.xml"
for tamano in 48 128 256; do
    rm -f "$DATOS/icons/hicolor/${tamano}x${tamano}/mimetypes/text-x-deimon.png"
done
[ -d "$DATOS/mime" ] && command -v update-mime-database >/dev/null 2>&1 && update-mime-database "$DATOS/mime" || true
for perfil in "$HOME/.bashrc" "$HOME/.zshrc"; do
    [ -f "$perfil" ] && sed -i '/# deimon$/d' "$perfil"
done
echo "Deimon se ha quitado. Tus programas .dmn, ~/.deimon y Rust siguen en su sitio."
