# Sistema operativo propio (destino pc-sin-sistema)

[Volver al README](../README.md)

Un programa de Deimon puede ser **el sistema operativo** de un PC: arranca solo, sin
Linux ni Windows debajo, usa la pantalla y el teclado de verdad y se apaga cuando él
quiere. El ejemplo completo es [`ejemplos-nucleo/consola.dmn`](../ejemplos-nucleo/consola.dmn)
(DeimonOS: calculadora, notas, reparto de procesos y colores).

## Probarlo

```bash
deimon ejecutar ejemplos-nucleo/consola.dmn    # en la terminal, como un programa normal
deimon probar ejemplos-nucleo/consola.dmn      # sus pruebas
deimon arrancar ejemplos-nucleo/consola.dmn    # lo enciende como PC en QEMU (ventana)
deimon arrancar ejemplos-nucleo/consola.dmn --sin-ventana   # la terminal hace de pantalla y teclado
```

Para grabarlo en un USB o usarlo en una máquina virtual:

```bash
deimon compilar ejemplos-nucleo/consola.dmn --destino pc-sin-sistema
# crea ejemplos-nucleo/consola-pc-sin-sistema.iso
```

La imagen `.iso` arranca con GRUB en cualquier PC de 64 bits con BIOS (o con el modo
de compatibilidad «CSM/Legacy» activado). Grabarla en un USB borra lo que tenga ese USB.

Hace falta, una sola vez:

```bash
rustup target add x86_64-unknown-none
sudo apt install grub-pc-bin xorriso mtools qemu-system-x86
```

`deimon destinos` dice si falta algo.

## Qué funciona

- **Todo el lenguaje**: variables, funciones, recursión (hasta 10 000 llamadas), listas,
  listas con claves, `para`, `mientras`, `romper`, `continuar`, `intentar ... si falla`
  y `provocar_error`. Los valores se comportan igual que en Linux: el traductor copia
  las mismas partes de `compilador/src/nucleo/base.rs`.
- **Integradas** básicas, de conversiones, de listas y textos, y de máquina, más
  `proteger_html`, `listar_claves` (de listas con claves), `comprobar` y `comprobar_igual`.
- **Módulos**: `texto` y cualquier módulo escrito solo en Deimon.
- **`usar "piezas/archivo.dmn"`** para repartir el sistema en piezas.

Lo que necesita disco, red o navegador (`guardar`, `leer`, rutas web, `servir`, usuarios,
sesiones, módulos con código Rust como `json` o `matematicas`) da el error **E026** al
compilar, con la línea exacta.

## Pantalla y teclado

| Función | Qué hace |
| --- | --- |
| `limpiar_pantalla()` | borra la pantalla (80 columnas × 25 filas) |
| `escribir_en(fila, columna, texto)` | escribe en esa posición (fila 0–24, columna 0–79) |
| `poner_color(letra, fondo)` | colores de lo siguiente: negro, azul, verde, cian, rojo, morado, marron, gris, gris_oscuro, azul_claro, verde_claro, cian_claro, rojo_claro, rosa, amarillo, blanco |
| `leer_tecla()` | espera una tecla: la letra (`"ñ"`, `"A"`) o su nombre (`"enter"`, `"borrar"`, `"tab"`, `"escape"`, `"arriba"`, `"abajo"`, `"izquierda"`, `"derecha"`) |
| `pedir_texto(aviso)` | escribe el aviso y devuelve la línea tecleada (con borrar) |
| `apagar_maquina()` | apaga el PC (en QEMU, lo cierra) |
| `reiniciar_maquina()` | reinicia el PC |

`mostrar` y `escribir` escriben en la pantalla y además por el puerto serie (COM1), y
el teclado también se lee de ese puerto: así `deimon arrancar --sin-ventana` y las
pruebas automáticas manejan el PC desde la terminal. El teclado es el español (ñ, ç,
¡, ¿ y acentos en la pantalla).

En la terminal (`deimon ejecutar`) estas funciones hacen lo mismo con colores de la
terminal y la entrada estándar; `leer_tecla()` y `pedir_texto()` devuelven `nada` cuando
la entrada se acaba. Así un sistema se prueba primero como programa normal:

```deimon
limpiar_pantalla()
poner_color("blanco", "azul")
escribir_en(0, 0, " Mi sistema ")
poner_color("gris", "negro")
escribir_en(2, 0, "")
mostrar("Listo")
```

Un error sin `intentar` alrededor se escribe en rojo con su línea («Error en la línea N
(en ejecución): ...») y detiene la máquina; con `deimon arrancar` sale con código 1.

## Cómo está hecho

- `compilador/src/nucleo/sin_sistema.rs`: el arranque (cabecera multiboot2 para GRUB,
  paso a 64 bits con páginas de 2 MB para los primeros 4 GB), la memoria (192 MB con
  listas libres por tamaño, que se reutiliza), la pantalla de texto VGA, el teclado PS/2,
  el puerto serie y los saltos de `intentar` (lo que en Linux hace `catch_unwind`).
- Las partes de `nucleo/base.rs` entre `// @PURO_INICIO@` y `// @PURO_FIN@` no usan el
  sistema; el traductor las copia tal cual. Si se añade algo ahí, tiene que compilar sin
  `std` (lo comprueba `cargo test --test sin_sistema`).
- `deimon compilar --destino pc-sin-sistema` compila con `rustc --target
  x86_64-unknown-none` y mete el núcleo en una imagen con `grub-mkrescue`.
- Pruebas: `compilador/tests/sin_sistema.rs` arranca programas en QEMU y exige la misma
  salida que en Linux; `compilador/tests/maquina.rs` prueba las funciones de máquina en
  la terminal.

Límites de esta primera versión: un solo procesador, sin interrupciones (el teclado se
consulta sin parar mientras se espera una tecla), sin reloj ni disco, y solo PCs con
BIOS (no UEFI puro).
