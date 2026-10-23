#!/usr/bin/env bash
# Instala Deimon con un solo paso: ./instalar.sh
# Deja la orden `deimon` disponible en cualquier carpeta y los iconos de los .dmn.
# Para quitarlo: ./desinstalar.sh
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")" && pwd)"
DESTINO="${DEIMON_DESTINO:-$HOME/.local/bin}"
morado=$'\e[38;5;135m'; verde=$'\e[32m'; rojo=$'\e[31m'; fin=$'\e[0m'
[ -t 1 ] || { morado=""; verde=""; rojo=""; fin=""; }

paso() { echo "${morado}▶${fin} $1"; }
bien() { echo "  ${verde}✓${fin} $1"; }
mal() { echo "  ${rojo}✗${fin} $1" >&2; exit 1; }

echo "${morado}Instalando Deimon · Código simple. Ideas grandes.${fin}"

# 1. Rust: Deimon lo usa para convertir cada programa en un ejecutable rápido.
paso "Comprobando Rust"
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    elif [ "${DEIMON_SIN_RUST:-}" = "1" ]; then
        mal "falta Rust y DEIMON_SIN_RUST=1 impide instalarlo"
    else
        command -v curl >/dev/null 2>&1 || mal "falta curl para descargar Rust (sudo apt install curl)"
        echo "  Rust no está instalado: se instala desde https://rustup.rs (oficial)."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi
bien "$(rustc --version)"

# 2. Compilar Deimon.
paso "Compilando Deimon (la primera vez tarda un poco)"
cargo build --release --quiet --manifest-path "$RAIZ/compilador/Cargo.toml"
bien "compilado"

# 3. Dejar la orden `deimon` en el camino de búsqueda.
paso "Instalando la orden deimon en $DESTINO"
mkdir -p "$DESTINO"
install -m 755 "$RAIZ/compilador/target/release/deimon" "$DESTINO/deimon"
bien "$DESTINO/deimon"
case ":$PATH:" in
    *":$DESTINO:"*) ;;
    *)
        for perfil in "$HOME/.bashrc" "$HOME/.zshrc"; do
            if [ -f "$perfil" ] && ! grep -q "# deimon" "$perfil"; then
                printf '\nexport PATH="%s:$PATH"  # deimon\n' "$DESTINO" >> "$perfil"
            fi
        done
        echo "  Abre una terminal nueva para usar 'deimon' (se añadió $DESTINO al PATH)."
        ;;
esac

# 4. Iconos de los archivos .dmn (solo en escritorios Linux).
if command -v update-mime-database >/dev/null 2>&1 && [ -f "$RAIZ/arrancar.sh" ]; then
    paso "Iconos de los archivos .dmn"
    bash "$RAIZ/arrancar.sh" iconos >/dev/null && bien "los .dmn se verán con el demonio"
fi

# 5. Comprobación final.
paso "Comprobando"
"$DESTINO/deimon" --version | tail -n +3 | head -2
echo
echo "${verde}Deimon está instalado.${fin} Prueba:"
echo "  deimon ejecutar \"$RAIZ/ejemplos/hola.dmn\""
echo "  deimon guia"
