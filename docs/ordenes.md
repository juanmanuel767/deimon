# Órdenes de Deimon

[Volver al README](../README.md)

## Resumen de órdenes

| Orden | Qué hace |
|---|---|
| `deimon traducir archivo.dmn` | escribe el Rust generado, sin ejecutarlo |
| `deimon ejecutar archivo.dmn [--perfil NOMBRE]` | traduce, compila y ejecuta |
| `deimon comprobar archivo.dmn [--json] [--arreglar]` | busca errores sin ejecutar |
| `deimon formatear archivo.dmn [--comprobar]` | aplica el estilo único |
| `deimon probar archivo.dmn [--json]` | ejecuta solo las pruebas del programa |
| `deimon app archivo.dmn [--nombre N] [--paquete P] [--salida RUTA] [--perfil NOMBRE]` | crea un APK de Android |
| `deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil NOMBRE]` | crea un ejecutable para otro sistema |
| `deimon destinos [--json]` | lista los destinos disponibles |
| `deimon publicar archivo.dmn --prueba \| --dominio midominio.com` | pone HTTPS con Caddy: en este ordenador (prueba) o prepara la carpeta para un servidor de internet |
| `deimon desde-php archivo.php [--salida archivo.dmn] [--comprobar]` | traduce PHP a Deimon; con --comprobar ejecuta los dos y compara que escriben lo mismo |
| `deimon modulos [archivo.dmn] [--json]` | lista los módulos que se pueden usar (biblioteca estándar, `~/.deimon/modulos` y los de junto al programa) con sus funciones |
| `deimon entorno [archivo.dmn] [--json]` | revisa el ordenador y un programa |
| `deimon guia [archivo.dmn] [--json]` | muestra la referencia completa para una IA |

## Cómo compilar el traductor

```bash
cd compilador
cargo build --release
```

El programa queda en `compilador/target/release/deimon`.

## Cómo ejecutar un ejemplo

Desde la carpeta `deimon`:

```bash
compilador/target/release/deimon ejecutar ejemplos/hola.dmn
```

También puedes ver el Rust que se genera, sin ejecutarlo:

```bash
compilador/target/release/deimon traducir ejemplos/hola.dmn
```

## Comprobar el entorno

Antes de trabajar (o cuando algo no arranca) puedes preguntarle a `deimon` si tu
ordenador tiene todo lo necesario:

```bash
deimon entorno                      # revisa el ordenador
deimon entorno ejemplos-web/app.dmn  # además, revisa ese programa
deimon entorno --json               # lo mismo, escrito para que lo lea otro programa
```

Cada línea empieza con una marca:

- `✓` está bien.
- `✗` falta algo imprescindible (por ejemplo `rustc`, un error al traducir el
  programa o una puerta ocupada). En ese caso `deimon entorno` termina con código 1.
- `!` es un aviso: no impide trabajar, pero algo no está disponible (por
  ejemplo, el SDK de Android solo hace falta para crear apps).

Se revisa: `rustc` y su versión, un resumen de los destinos de compilación
disponibles (los mismos de `deimon destinos`), el destino de Rust para Android, el SDK de
Android (con `build-tools`, plataforma y NDK), `adb` y los móviles conectados
(espera como mucho 5 segundos), la carpeta de datos (`DEIMON_DATOS` o `./datos`) y la
carpeta temporal. Si pasas un archivo, también se comprueba que se traduce, cuántas
pruebas tiene y, si usa `servir(N)`, que la puerta esté libre. Si está
ocupada, te dice otra libre y la orden exacta, por ejemplo:

```text
✗ puerto: la puerta 8080 está ocupada por otro programa. Prueba con: DEIMON_PUERTO=8081 deimon ejecutar app.dmn
```

Si lanzas el programa con la puerta ocupada, el mensaje de error también te
sugiere una puerta libre.

