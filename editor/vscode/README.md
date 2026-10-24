# Deimon para VS Code

Esta extensión ayuda a escribir programas en Deimon (archivos `.dmn`):

- Pone colores a las palabras clave, textos, números, comentarios y funciones.
- Al abrir y al guardar un archivo `.dmn`, busca errores con `deimon comprobar` y los subraya, con su mensaje y su sugerencia.

## Cómo instalarla

Opción 1 (la más fácil): copia esta carpeta a `~/.vscode/extensions/deimon` y reinicia VS Code.

    cp -r editor/vscode ~/.vscode/extensions/deimon

Opción 2: crea un paquete con `vsce` (hace falta Node.js) e instálalo.

    npx @vscode/vsce package
    code --install-extension deimon-0.1.0.vsix

## Ajuste

- `deimon.ruta`: ruta del programa `deimon`. Por defecto es `deimon`.
  Si VS Code no lo encuentra, te avisará una vez; escribe aquí la ruta completa (por ejemplo `/home/tu_usuario/.local/bin/deimon`).
