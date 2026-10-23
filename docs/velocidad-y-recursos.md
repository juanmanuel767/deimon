# Velocidad y recursos

[Volver al README](../README.md)

## Recursos y perfiles

El mismo programa puede prepararse para un dispositivo modesto o para un servidor
potente. El perfil se elige al compilar con `--perfil` en `deimon ejecutar`,
`deimon compilar` y `deimon app` (por defecto, `equilibrado`). También se acepta
`pequeno` sin la ñ:

```bash
deimon ejecutar ejemplos/hola.dmn --perfil pequeño
deimon compilar ejemplos/hola.dmn --perfil servidor
deimon app ejemplos-web/sitio.dmn --perfil equilibrado
```

| Perfil | Para qué | Opciones de `rustc` | Hilos | En espera | Petición máxima |
|---|---|---|---|---|---|
| `pequeño` | Móvil o dispositivo con poca memoria | `-C opt-level=z -C codegen-units=1 -C strip=symbols -C lto=fat` | 2 | 16 | 16 KB |
| `equilibrado` | Ordenador normal (por defecto) | `-O` | núcleos | 128 | 256 KB |
| `servidor` | Servidor con muchos núcleos | `-C opt-level=3 -C codegen-units=1 -C lto=fat` | núcleos × 2 | 1024 | 1 MB |

No se usa `panic=abort`: el servidor necesita recuperar los errores de una ruta
(responde 500) y las pruebas también. Las opciones `lto` y `strip` se han
probado con los destinos de Android y el NDK (arm64 y arm32) sin problemas.
En `deimon app` siempre se añade `-C strip=symbols`, aunque el perfil no lo lleve,
porque el APK se beneficia de ello.

`deimon compilar` termina mostrando el tamaño del ejecutable en KB, por ejemplo:

```text
Programa compilado para linux-x64 (x86_64-unknown-linux-gnu) con perfil pequeño: hola-linux-x64
Tamaño del ejecutable: 345.2 KB
```

El servidor ya no crea un hilo por conexión: usa un grupo fijo de hilos
trabajadores y una cola acotada. Si la cola está llena responde
`503 Servidor ocupado, inténtalo de nuevo`; si la petición pasa del tamaño
máximo responde `413 Petición demasiado grande`; y si una conexión tarda
demasiado en enviar su petición, se cierra para no bloquear un hilo. El plazo
cuenta para la petición entera: un cliente que manda datos gota a gota
(ataque «Slowloris») no retiene el hilo más de `DEIMON_TIEMPO_ESPERA` segundos.
Un tamaño de cuerpo inválido, repetido con valores distintos o enviado por trozos
(`Transfer-Encoding`) se rechaza con `400`.

Como los hilos pasan casi todo el tiempo esperando a la red, hay más hilos que
núcleos: 4 en `pequeño`, 4 por núcleo (mínimo 16) en `equilibrado` y 8 por núcleo
(mínimo 64) en `servidor`. Al arrancar se ve el perfil y los hilos, por ejemplo
`Servidor listo en http://127.0.0.1:8080 (perfil equilibrado, 16 hilos)`.

El servidor solo escucha en el propio ordenador (`127.0.0.1`). Para abrirlo a
internet usa `deimon publicar`, que pone Caddy delante con HTTPS.

El perfil queda grabado en el programa generado. Estas variables de entorno lo
sobrescriben al ejecutarlo:

| Variable | Qué hace |
|---|---|
| `DEIMON_HILOS` | número de hilos trabajadores |
| `DEIMON_MAX_CONEXIONES` | conexiones en espera antes de responder 503 |
| `DEIMON_MAX_PETICION` | tamaño máximo de la petición, en bytes (413 si se pasa) |
| `DEIMON_TIEMPO_ESPERA` | segundos máximos esperando la petición de una conexión (10 por defecto); mientras llegan datos de verdad, el plazo crece 1 segundo por cada 32 KB |
| `DEIMON_MAX_SUBIDA` | tamaño máximo de un formulario con archivos, en bytes (2 MB, 10 MB o 50 MB según el perfil; 413 si se pasa) |

`deimon entorno` muestra los núcleos y la memoria total del ordenador y recomienda
un perfil: `pequeño` con menos de 1 GB de memoria o un solo núcleo, `servidor`
con 8 núcleos y 8 GB o más, y `equilibrado` en los demás casos.

## Velocidad

Para medirla: `bash rendimiento/medir.sh` (compila y ejecuta los programas de `rendimiento/`).

| Programa | Antes | Ahora | Rust puro equivalente |
|---|---|---|---|
| fib(32) recursivo | 0,35 s | 0,024 s | 0,022 s |
| bucle de 20 millones con `%` | 1,8 s | 0,79 s | 0,76 s |
| lista de 20 000 con `agregar` | 4,1 s | 0,009 s | — |
| texto concatenado 100 000 veces | — | 0,014 s | — |

Lo que cambió: las operaciones trabajan sobre referencias en vez de copiar los valores; los textos y las listas se comparten y solo se copian si se modifican; `x = agregar(x, v)` amplía la lista en su sitio. La prueba `compilador/tests/rendimiento.rs` falla si las listas o los textos vuelven a ser lentos.
