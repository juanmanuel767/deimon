# Apps y destinos

[Volver al README](../README.md)

## Crear una app de Android

Un programa con rutas web se puede convertir en una app para el móvil:

```bash
compilador/target/release/deimon app ejemplos-web/app.dmn --nombre "Mi App" --paquete com.ejemplo.miapp
```

`--nombre` y `--paquete` son opcionales (por defecto se usan el nombre del
archivo y `com.deimon.<nombre>`). Con `--salida ruta.apk` eliges dónde queda el
archivo; si no, el APK se crea junto al `.dmn` (`ejemplos-web/app.apk`).

La app abre una pantalla con un navegador que muestra tu web, que funciona
dentro del propio móvil. Para instalarla con el móvil conectado por USB:

```bash
adb install ejemplos-web/app.apk
```

Necesitas el SDK de Android (en `~/Android/Sdk` o en `ANDROID_HOME`) con
`build-tools`, una plataforma 35 o superior y el NDK, además de Java (JDK) y el
destino de Rust: `rustup target add aarch64-linux-android`.

El APK se firma con una clave de pruebas que se crea sola la primera vez en
`~/.deimon/depuracion.keystore` (contraseña: `android`). Sirve para instalar en tu
móvil, no para publicar en una tienda.

De momento solo funciona en móviles de tipo arm64 (los modernos).

## Compilar para otros sistemas

El mismo programa `.dmn` se puede compilar para otro sistema o procesador. Primero
mira qué hay instalado en tu ordenador:

```bash
deimon destinos
```

Cada destino aparece con `✓` si está disponible o `✗` con lo que falta y la orden
exacta para instalarlo. Después:

```bash
deimon compilar ejemplos/hola.dmn --destino linux-x64-estatico
deimon compilar ejemplos/hola.dmn --destino android-arm64 --salida /tmp/hola-android
```

Si no se escribe `--destino`, se usa el del ordenador actual. La salida se llama
`<nombre>-<destino>` y se crea junto al archivo `.dmn` (con `.exe` en Windows), salvo
que se indique `--salida RUTA`. Antes de compilar se ejecutan las pruebas del
programa en el ordenador, igual que con `deimon app`; si alguna falla, no se compila.

| Destino | Triple de Rust | Para qué sirve | Red / archivos / hilos | Enlazador |
|---|---|---|---|---|
| `linux-x64` | `x86_64-unknown-linux-gnu` | Linux de 64 bits para este ordenador | sí / sí / sí | el de por defecto |
| `linux-x64-estatico` | `x86_64-unknown-linux-musl` | Cualquier Linux de 64 bits, sin dependencias | sí / sí / sí | el de por defecto |
| `android-arm64` | `aarch64-linux-android` | Android en móviles modernos de 64 bits | sí / sí / sí | `aarch64-linux-android24-clang` del NDK |
| `android-arm32` | `armv7-linux-androideabi` | Android en móviles antiguos de 32 bits | sí / sí / sí | `armv7a-linux-androideabi24-clang` del NDK |
| `linux-arm64` | `aarch64-unknown-linux-gnu` | Linux de 64 bits para ARM (Raspberry Pi y similares) | sí / sí / sí | `aarch64-linux-gnu-gcc` |
| `windows-x64` | `x86_64-pc-windows-gnu` | Windows de 64 bits | sí / sí / sí | `x86_64-w64-mingw32-gcc` |

`linux-arm64` y `windows-x64` pueden no estar instalados. En ese caso `deimon destinos`
dice cómo conseguirlos, por ejemplo:

```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```

Los destinos de Android reutilizan el SDK y el NDK que ya usa `deimon app`. `deimon destinos --json`
escribe la misma información en JSON.

## Windows, Raspberry Pi e iPhone

- **Windows** (`--destino windows-x64`) y **Raspberry Pi / Linux ARM**
  (`--destino linux-arm64`) funcionan. Se probaron ejecutando los programas con
  `wine` (Windows) y `qemu-aarch64` (ARM), incluidos los datos permanentes. Para
  instalarlos: `rustup target add x86_64-pc-windows-gnu aarch64-unknown-linux-gnu`
  y `sudo apt install gcc-mingw-w64-x86-64 gcc-aarch64-linux-gnu`
  (`deimon destinos` muestra lo que falte).
- **iPhone no es posible desde Linux**: Apple solo permite crear apps de iPhone
  en un Mac con Xcode y una cuenta de desarrollador. Mientras tanto, cualquier
  programa web de Deimon se puede abrir en el navegador del iPhone.

## Apps con pantallas nativas

Además de mostrar páginas web, una app puede usar **pantallas nativas de
Android**: títulos, botones y cuadros de texto de verdad, como cualquier app
del móvil. Se describen con el módulo `pantallas` y cada ruta devuelve una
`pantalla([...])`:

```deimon
usar modulo "pantallas"

ruta "/" {
    devolver pantalla([titulo("Mis notas"), cuadro_texto("nota", "Escribe una nota"), boton_enviar("Guardar", "/guardar"), boton("Ver notas", "/notas")])
}
```

| Función | Qué dibuja |
|---|---|
| `titulo(t)` | texto grande en morado |
| `parrafo(t)` | texto normal |
| `elemento_lista(t)` | una fila con viñeta |
| `separador()` | una línea fina |
| `boton(texto, camino)` | botón que abre otra pantalla |
| `cuadro_texto(nombre, ayuda)` / `cuadro_clave(nombre, ayuda)` | cuadro para escribir (el segundo oculta lo escrito); se lee con `campo(nombre)` |
| `boton_enviar(texto, camino)` | envía los cuadros de la pantalla, como un formulario |

Funcionan los formularios, `redirigir()`, las sesiones y el botón «atrás» del
móvil. Si una ruta devuelve HTML normal, la app lo muestra como página web, así
que se pueden mezclar. Las pruebas se escriben igual, con `visitar()` y
`enviar()`.

Ejemplo probado en un móvil real: `ejemplos-web/notas-nativa.dmn`
(`deimon app ejemplos-web/notas-nativa.dmn --nombre "Notas Deimon"`).
