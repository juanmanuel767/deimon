# Órdenes de Deimon

[Volver al README](../README.md)

## Resumen de órdenes

| Orden | Qué hace |
|---|---|
| `deimon traducir archivo.dmn` | escribe el Rust generado, sin ejecutarlo |
| `deimon ejecutar archivo.dmn [--perfil NOMBRE]` | ejecuta al instante con el intérprete propio; si el programa usa algo que aún no sabe hacer, lo traduce, compila y ejecuta como siempre |
| `deimon comprobar archivo.dmn [--json] [--arreglar]` | busca errores sin ejecutar |
| `deimon formatear archivo.dmn [--comprobar]` | aplica el estilo único |
| `deimon probar archivo.dmn [--json]` | ejecuta solo las pruebas del programa |
| `deimon app archivo.dmn [--nombre N] [--paquete P] [--salida RUTA] [--perfil NOMBRE]` | crea un APK de Android |
| `deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil NOMBRE]` | crea un ejecutable para otro sistema |
| `deimon destinos [--json]` | lista los destinos disponibles |
| `deimon publicar archivo.dmn --prueba \| --dominio midominio.com` | pone HTTPS con Caddy: en este ordenador (prueba) o prepara la carpeta para un servidor de internet |
| `deimon desde-php archivo.php [--salida archivo.dmn] [--comprobar]` | traduce PHP a Deimon; con --comprobar ejecuta los dos y compara que escriben lo mismo |
| `deimon desde-cobol archivo.cob [--salida archivo.dmn] [--comprobar]` | traduce un COBOL básico a Deimon; con --comprobar lo compila con cobc, ejecuta los dos y compara salida y código de salida |
| `deimon modulos [archivo.dmn] [--json]` | lista los módulos que se pueden usar (biblioteca estándar, `~/.deimon/modulos` y los de junto al programa) con sus funciones |
| `deimon modulo instalar <https://... \| carpeta> [--aqui]` | instala un módulo de la comunidad con git en `~/.deimon/modulos` (o en `modulos/` con `--aqui`) |
| `deimon modulo actualizar <nombre> [--aqui]` | trae la última versión de un módulo instalado |
| `deimon modulo quitar <nombre> [--aqui]` | borra un módulo instalado |
| `deimon entorno [archivo.dmn] [--json]` | revisa el ordenador y un programa |
| `deimon guia [archivo.dmn] [--json]` | muestra la referencia completa para una IA |
| `deimon jugar [--puerto N] [--sin-abrir]` | abre una página en este ordenador para escribir código Deimon y ejecutarlo (puerto 8160 por defecto) |
| `deimon mcp` | servidor para que una IA use Deimon directamente (ver «Usar Deimon desde una IA») |
| `deimon empaquetar archivo.dmn [--salida RUTA] [--perfil NOMBRE]` | crea un solo ejecutable con el programa dentro, sin necesitar Rust (ver abajo) |

## Usar Deimon desde una IA (MCP)

`deimon mcp` es un servidor MCP (el estándar con el que los asistentes de IA usan
herramientas): habla JSON-RPC por la entrada y la salida estándar. Con él una IA comprueba,
ejecuta y prueba código Deimon por su cuenta, sin que nadie copie y pegue. Para conectarlo a
Claude Code:

```bash
claude mcp add deimon -- deimon mcp
```

Herramientas:

- `deimon_buscar_funcion`: busca funciones que existen de verdad (integradas y de los
  módulos) y dice su firma, para qué sirven, un ejemplo y el `usar modulo` que necesitan. Si no
  hay ninguna, lo dice claro, para que la IA no se la invente.
- `deimon_guia`: la referencia completa del lenguaje.
- `deimon_comprobar`: los errores en JSON (código, línea, columna, sugerencia y arreglo).
- `deimon_ejecutar`: ejecuta con el intérprete; con límites de 60 s, 2 000 millones de pasos y
  64 KB de salida, así que un programa que no termina nunca cuelga el servidor.
- `deimon_probar`: corre los bloques `prueba` y devuelve el resultado de cada uno.
- `deimon_formatear`: el código con el formato oficial.

Cada una recibe el código en `codigo` o la ruta de un `.dmn` en `archivo`.

## Repartir un programa sin Rust

```bash
deimon empaquetar tienda.dmn                # crea ./tienda (tienda.exe en Windows)
deimon empaquetar tienda.dmn --salida dist/tienda
deimon empaquetar tienda.dmn --perfil servidor   # el paquete recuerda su perfil
./tienda                                    # se abre sin instalar nada
```

`deimon empaquetar` corre antes las pruebas del programa (si alguna falla, no crea nada) y
después junta en un solo archivo el propio `deimon` (con su intérprete) y el programa con
todo lo que usa: los archivos de `usar`, los módulos escritos en Deimon (también los de
`~/.deimon/modulos`) y las plantillas. Ocupa unos 4 MB y no necesita Rust ni para crearlo
ni para abrirlo. El programa empaquetado no lee sus archivos del disco: aunque cambies o
borres los originales, sigue haciendo lo mismo. Los datos (`guardar`, sesiones...) se
guardan en `./datos` junto a donde se abre, igual que un programa compilado.

Límites: sirve para el mismo sistema en el que se crea (un paquete hecho en Linux funciona
en Linux). Si el programa necesita algo que el intérprete no lleva (`pantallas`, un módulo
con `codigo.rs` propio, o `sql` en un `deimon` preparado sin SQLite dentro), no se empaqueta y te dice que uses
`deimon compilar`, que lo convierte con Rust.

## Probar código en el navegador

```bash
deimon jugar                 # abre http://127.0.0.1:8160
deimon jugar --puerto 8170   # otro puerto
deimon jugar --sin-abrir     # no abre el navegador (solo escribe la dirección)
```

Abre una página con un área para escribir código Deimon, un botón «Ejecutar» (o Ctrl+Enter) y una lista de ejemplos. Los errores salen con su línea, mensaje y sugerencia.

- Solo escucha en `127.0.0.1` (este ordenador). Cada ejecución usa una carpeta temporal que se borra después.
- Un programa puede tardar como máximo 15 segundos (contando su preparación); después se detiene. La salida se corta a 64 KB y el código a 100 KB.
- Solo se ejecuta un programa a la vez; si llega otro, recibe «ocupado».
- Seguridad: la página lleva una clave aleatoria que cambia en cada arranque; sin ella, o desde otra web (`Host` u `Origin` distintos), se rechaza la petición.

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

## Dos motores

`deimon ejecutar` tiene dos motores y elige solo el mejor:

- **Intérprete propio** (el de siempre que se pueda): no necesita Rust y empieza al instante (unos 15 ms para un «hola»; la tienda de `proyectos/tienda` está lista en 0,1 s frente a 15 s compilando). Entiende todo el lenguaje (incluidos `intentar` y varios archivos con `usar`), todas las funciones integradas, las páginas web (`ruta`, `servir`, `visitar`, `enviar`, plantillas, formularios, sesiones, usuarios, galletas, subir y servir archivos) y todos los módulos de la biblioteca estándar salvo `pantallas` (también `sql`: `deimon` lleva SQLite dentro si al prepararlo había `libsqlite3.a`, como en Ubuntu con `libsqlite3-dev`; así no hace falta SQLite donde se abra el programa). Usa el mismo servidor web que el modo Rust.
- **Modo Rust**: se usa solo si el programa necesita algo que el intérprete no lleva dentro: `pantallas` (Android), módulos propios o de la comunidad con su `codigo.rs`. Los perfiles (`--perfil pequeño|equilibrado|servidor`) funcionan con los dos motores. No hay que hacer nada: pasa solo y el resultado es el mismo.

Límites opcionales del intérprete (idea tomada del intérprete de pruebas `dvm`), útiles cuando una IA prueba código: `DEIMON_MAX_PASOS=1000000` detiene el programa al pasar de ese número de instrucciones (por ejemplo, un bucle que no termina) y `DEIMON_MAX_SALIDA=65536` al escribir más de esos bytes con `mostrar`/`escribir`. Sin esas variables no hay límite. `intentar` no puede atrapar estos cortes. `deimon jugar` los usa siempre.

Los dos motores tienen el mismo límite de seguridad: si una función se llama a sí misma sin parar, se detiene al pasar de 10 000 llamadas una dentro de otra con un error claro (que `intentar` puede atrapar), en vez de romper el programa o gastar toda la memoria.

Los dos motores usan el mismo análisis y el mismo núcleo (`compilador/src/nucleo/`), así que dan la misma salida, los mismos errores (`E0xx`, y los de ejecución con su línea) y el mismo código de salida.

Para forzar uno: `DEIMON_MOTOR=interprete deimon ejecutar app.dmn` o `DEIMON_MOTOR=rust deimon ejecutar app.dmn`. Con `DEIMON_MOTOR_MOTIVO=1` avisa de por qué pasó al modo Rust. `deimon compilar` y `deimon app` no cambian (siguen generando el programa rápido con Rust).

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

