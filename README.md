<p align="center"><img src="docs/imagenes/logo.png" alt="Logo de Deimon" width="360"></p>

# Deimon

**Código simple. Ideas grandes.**

`deimon` es un lenguaje de programación escrito en español. Los programas se guardan
en archivos `.dmn` y el traductor los convierte en Rust y los ejecuta por ti. No
hace falta saber Rust: solo tener Rust instalado (`cargo` y `rustc`).

## Estructura

```text
deimon/
  compilador/     el traductor de Deimon a Rust
  ejemplos/       programas de ejemplo con su salida esperada
  ejemplos-web/   ejemplos de páginas web y de app
  ejemplos-php/   programas de PHP para probar el traductor (desde-php)
  proyectos/      proyectos reales hechos en Deimon (tienda/: tienda en línea completa)
  biblioteca/     13 módulos estándar (texto, json, sql, excel, pdf, graficos, pagos, correo...)
  rendimiento/    programas para medir velocidad y medir.sh
  herramientas/   ver-pruebas.sh para ver las pruebas en vivo
  docs/           documentación detallada
  instalar.sh     instala la orden deimon (desinstalar.sh la quita)
  arrancar.sh     atajos: preparar, probar, ejemplo, web, iconos...
```

## Instalar

```bash
./instalar.sh       # un solo paso: deja la orden `deimon` lista en cualquier carpeta
./desinstalar.sh    # la quita (no toca tus programas ni tus datos)
```

Si falta Rust, el instalador lo instala desde su web oficial (rustup.rs).

## Cómo empezar

Desde esta carpeta:

```bash
./arrancar.sh preparar        # compila el traductor
./arrancar.sh ejemplo hola    # ejecuta ejemplos/hola.dmn
./arrancar.sh ayuda           # lista todas las órdenes
./arrancar.sh iconos          # los archivos .dmn se ven con el icono de Deimon
```

También puedes usar el traductor directamente:

```bash
compilador/target/release/deimon ejecutar ejemplos/hola.dmn
```

## Documentación

- [El lenguaje](docs/lenguaje.md): sintaxis, valores, operadores, listas y funciones.
- [Pasar de PHP a Deimon](docs/desde-php.md): traductor con comprobación de que hace lo mismo.
- [Archivos y módulos](docs/modulos.md): `usar`, módulos propios y biblioteca estándar.
- [Web y datos](docs/web-y-datos.md): páginas web y guardar datos.
- [Apps y destinos](docs/apps-y-destinos.md): app de Android y otros sistemas.
- [Pruebas y errores](docs/pruebas-y-errores.md): pruebas, comprobar, errores y guía para la IA.
- [Velocidad y recursos](docs/velocidad-y-recursos.md): perfiles y mediciones.
- [Órdenes de Deimon](docs/ordenes.md): todas las órdenes y sus opciones.
- [Gramática](docs/GRAMATICA.md): gramática completa en EBNF.
- [Ideas](docs/IDEAS.md): objetivos y ventajas del proyecto.

## Licencia

MIT: puedes usar, cambiar y compartir Deimon libremente. Ver [LICENSE](LICENSE).
