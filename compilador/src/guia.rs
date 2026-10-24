//! Referencia completa y compacta del lenguaje (`deimon guia`), pensada para pegarla en el contexto de
//! una IA. Las funciones integradas, las equivalencias de otros lenguajes y los códigos de error
//! salen de las mismas tablas que usa el compilador, así que nunca se desincronizan.

use crate::ajenas::EQUIVALENCIAS;
use crate::ast::Extension;
use crate::error::CODIGOS;
use crate::integradas::INTEGRADAS;
use crate::json;

/// Palabras reservadas y para qué sirven.
pub const PALABRAS_CLAVE: &[(&str, &str)] = &[
    ("sea", "declara una variable: sea x = 1"),
    ("usar", "carga funciones, rutas y pruebas de otro archivo (solo en el nivel principal): usar \"piezas/calculos.dmn\"; con 'usar modulo \"matematicas\"' carga un módulo"),
    ("funcion", "define una función (solo en el nivel principal): funcion suma(a, b) { devolver a + b }"),
    ("devolver", "termina la función, ruta o prueba y entrega un valor: devolver x"),
    ("si", "condicional: si cond { ... } sino si otra { ... } sino { ... }"),
    ("sino", "rama alternativa de un 'si' (va en la misma línea que la '}')"),
    ("mientras", "bucle mientras la condición sea verdadera: mientras i < 3 { ... }"),
    ("romper", "sale del bucle 'mientras' o 'para' en el que está: si x == 0 { romper }"),
    ("continuar", "salta a la siguiente vuelta del bucle: si x == 0 { continuar }"),
    ("intentar", "atrapa los errores de un bloque: intentar { ... } si falla e { mostrar(e[\"mensaje\"]) }; e es una lista con claves {\"mensaje\", \"linea\"}; provocar_error(m) lanza uno"),
    ("falla", "parte de 'intentar { ... } si falla e { ... }' (va en la misma línea que la '}')"),
    ("para", "recorre una lista: para x en lista { ... }"),
    ("en", "separa la variable y la lista en 'para'"),
    ("ruta", "página web (solo en el nivel principal): ruta \"/saludo/:nombre\" { devolver \"hola \" + nombre }"),
    ("prueba", "palabra reservada que define una prueba (solo en el nivel principal): prueba \"suma\" { comprobar(1 + 1 == 2) }"),
    ("modulo", "carga un módulo (carpeta con funciones en Rust y/o en Deimon): usar modulo \"texto\""),
    ("extension", "palabra reservada antigua: 'usar extension' ahora se escribe 'usar modulo' (E021)"),
    ("y", "y lógico (solo con valores lógicos)"),
    ("o", "o lógico (solo con valores lógicos)"),
    ("no", "negación lógica"),
    ("verdadero", "valor lógico cierto"),
    ("falso", "valor lógico falso"),
    ("nada", "ausencia de valor"),
];

/// Precedencia de operadores, de la más fuerte a la más débil.
pub const OPERADORES: &[(&str, &str)] = &[
    ("lista[i]", "índice (se empieza a contar desde 0); fuera de rango es un error"),
    ("-x", "menos unario"),
    ("*  /  %", "multiplicar, dividir, resto (dividir entre cero es un error)"),
    ("+  -", "sumar y restar; '+' concatena si algún lado es texto y une dos listas"),
    ("==  !=  <  <=  >  >=", "comparaciones (< <= > >= solo con números)"),
    ("no", "negación lógica"),
    ("y", "y lógico"),
    ("o", "o lógico"),
];

pub const TIPOS: &[(&str, &str)] = &[
    ("número", "decimal (f64): 30, 2.5, -7; 5 se muestra como 5"),
    ("texto", r#"entre comillas dobles, con escapes \n \t \" \\; en una sola línea"#),
    ("lógico", "verdadero o falso; las condiciones deben ser lógicas (no hay verdad por defecto)"),
    ("nada", "el valor nada; leer() de una clave inexistente lo devuelve"),
    ("lista", "[1, \"a\", verdadero]; puede mezclar tipos; l[0] es el primero y l[i] = v cambia uno"),
    ("lista con claves", "{\"nombre\": \"Ana\", \"edad\": 30}; recuerda el orden de las claves; m[\"edad\"] lee y m[\"edad\"] = 31 cambia o añade; listar_claves(m), tiene_clave(m, c) y quitar_clave(m, c)"),
];

pub const ORDENES: &[(&str, &str)] = &[
    ("deimon traducir archivo.dmn", "escribe el Rust generado"),
    ("deimon ejecutar archivo.dmn [--perfil NOMBRE]", "traduce, compila y ejecuta; los errores salen por stderr con código 1"),
    ("deimon comprobar archivo.dmn [--json] [--arreglar]", "solo revisa: lista todos los errores (código 1) o dice que no hay (código 0); --arreglar aplica los arreglos automáticos seguros"),
    ("deimon formatear archivo.dmn [--comprobar]", "reescribe el archivo con el estilo único del lenguaje; --comprobar no escribe nada, dice qué líneas cambiarían y sale con 1 si falta formatear"),
    ("deimon probar archivo.dmn [--json]", "corre las pruebas del programa sin ejecutar el resto"),
    ("deimon entorno [archivo.dmn] [--json]", "revisa que el ordenador tenga todo lo necesario y recomienda un perfil"),
    ("deimon app archivo.dmn [--nombre N] [--paquete P] [--salida ruta.apk] [--perfil NOMBRE]", "crea una app de Android a partir de un programa con rutas"),
    ("deimon mcp", "servidor MCP para una IA: herramientas deimon_buscar_funcion, deimon_guia, deimon_comprobar, deimon_ejecutar, deimon_probar y deimon_formatear (JSON-RPC por la entrada y la salida)"),
    ("deimon empaquetar archivo.dmn [--salida RUTA] [--perfil NOMBRE]", "corre las pruebas y crea un solo ejecutable con deimon y el programa dentro (con sus archivos usar, módulos .dmn y plantillas); no necesita Rust ni para crearlo ni para abrirlo; para este mismo sistema"),
    ("deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil NOMBRE]", "traduce, corre las pruebas y compila para otro sistema o procesador; sin --destino usa el de este ordenador; al final muestra el tamaño en KB"),
    ("deimon arrancar archivo.dmn [--sin-ventana] [--memoria MB]", "compila el programa para pc-sin-sistema (el programa es el sistema operativo) y lo enciende en QEMU; --sin-ventana usa la terminal como pantalla y teclado"),
    ("deimon publicar archivo.dmn --prueba | --dominio midominio.com", "pone HTTPS delante del programa con Caddy (certificados automáticos); --prueba en este ordenador, --dominio prepara la carpeta para un servidor"),
    ("deimon desde-php archivo.php [--salida archivo.dmn] [--comprobar]", "traduce un programa de PHP a Deimon, avisa de lo que no puede traducir y, con --comprobar, demuestra que los dos escriben lo mismo"),
    ("deimon modulos [archivo.dmn] [--json]", "lista los módulos disponibles (biblioteca estándar, ~/.deimon/modulos y modulos/ junto al programa) con lo que necesitan y sus funciones"),
    ("deimon modulo instalar <https://... | carpeta> [--aqui]", "instala un módulo de la comunidad (git) en ~/.deimon/modulos, o en modulos/ con --aqui; trae código Rust: instala solo módulos de quien confíes"),
    ("deimon modulo actualizar <nombre> [--aqui]", "trae la última versión de un módulo instalado y la vuelve a validar"),
    ("deimon modulo quitar <nombre> [--aqui]", "borra un módulo instalado"),
    ("deimon destinos [--json]", "lista los destinos de compilación (linux-x64, linux-x64-estatico, android-arm64, android-arm32, linux-arm64, windows-x64, pc-sin-sistema) con lo que falta y cómo instalarlo"),
    ("deimon guia [--json]", "imprime esta referencia"),
];

pub const PERFILES: &[(&str, &str)] = &[
    (
        "pequeño",
        "dispositivo limitado: rustc -C opt-level=z -C codegen-units=1 -C strip=symbols -C lto=fat; 2 hilos, cola 16, petición máxima 16 KB",
    ),
    (
        "equilibrado",
        "ordenador normal (por defecto): rustc -O; hilos = núcleos, cola 128, petición máxima 256 KB",
    ),
    (
        "servidor",
        "servidor potente: rustc -C opt-level=3 -C codegen-units=1 -C lto=fat; hilos = núcleos × 2, cola 1024, petición máxima 1 MB",
    ),
];

pub const VARIABLES_DE_ENTORNO: &[(&str, &str)] = &[
    ("DEIMON_CASA", "carpeta que hace de casa para ~/.deimon/modulos (por defecto HOME)"),
    ("DEIMON_PUERTO", "sustituye el puerto escrito en servir(...)"),
    ("DEIMON_DATOS", "carpeta de datos (por defecto ./datos); cada colección es un archivo <nombre>.datos"),
    ("DEIMON_HILOS", "número de hilos trabajadores del servidor (por defecto, según el perfil)"),
    ("DEIMON_MAX_CONEXIONES", "conexiones en espera antes de responder 503 (por defecto, según el perfil)"),
    ("DEIMON_MAX_PETICION", "tamaño máximo de la petición en bytes; si se pasa, responde 413 (por defecto, según el perfil)"),
    ("DEIMON_TIEMPO_ESPERA", "segundos máximos esperando la petición de una conexión (por defecto 10)"),
];

pub const REGLAS: &[&str] = &[
    "Una sentencia por línea; no se usa ';'. Los comentarios empiezan por '#'.",
    "Los bloques se abren con '{' en la misma línea de la condición/nombre y se cierran con '}'.",
    "Solo comillas dobles para los textos. No hay +=, ++, ni punto (a.b): usa x = x + 1.",
    "Cada función tiene su propio ámbito: no ve las variables creadas fuera de ella.",
    "Las funciones se pueden llamar antes de definirlas y pueden ser recursivas.",
    "'sea' no se puede repetir con el mismo nombre en el mismo ámbito; para cambiar el valor se escribe x = ...",
    "Los parámetros de una ruta (:nombre) son textos ya escapados como HTML; usa a_numero() para hacer cuentas.",
    "Colecciones: nombre de 1 a 64 caracteres con letras sin acentos, dígitos, '_' o '-'; las claves son textos.",
    "Una prueba es como una función sin parámetros; si falla una comprobación, termina y sigue la siguiente.",
    "Errores al ejecutar: «Error en la línea N (en ejecución): ...» y código de salida 1.",
    "usar \"ruta.dmn\" divide el programa: la ruta es relativa a la carpeta del archivo que la contiene y solo se cargan sus funciones, rutas y pruebas (sus demás instrucciones y su servir se ignoran).",
    "usar modulo \"nombre\" (solo en el nivel principal) carga un módulo: una carpeta con modulo.txt y funciones en Rust (funciones.txt + codigo.rs) y/o en Deimon (*.dmn). Se busca en modulos/<nombre>/ junto al archivo .dmn, después en ~/.deimon/modulos/<nombre>/ y por último en la biblioteca estándar (matematicas, texto, tiempo). deimon modulos los lista.",
    "El estilo único del código (sangría de 4 espacios, espacios de los operadores, comentarios y líneas vacías) lo aplica deimon formatear. La gramática completa en EBNF, las reglas de nombres y el estilo están en docs/GRAMATICA.md.",
];

/// La guía completa como texto.
pub fn texto() -> String {
    texto_con_extensiones(&[])
}

/// Como `texto`, y al final añade las funciones de las extensiones usadas por un archivo.
pub fn texto_con_extensiones(extensiones: &[Extension]) -> String {
    let mut salida = String::new();
    salida.push_str("# Guía del lenguaje Deimon (programas en archivos .dmn, se traducen a Rust)\n\n");

    salida.push_str("## Ejemplo\n");
    salida.push_str("sea nombre = \"Ana\"\nfuncion suma(a, b) {\n    devolver a + b\n}\nsi suma(2, 3) >= 5 {\n    mostrar(\"Hola, \" + nombre)\n} sino {\n    mostrar(\"no\")\n}\n\n");

    salida.push_str("## Reglas\n");
    for regla in REGLAS {
        salida.push_str(&format!("- {}\n", regla));
    }

    salida.push_str("\n## Palabras clave\n");
    for (palabra, descripcion) in PALABRAS_CLAVE {
        salida.push_str(&format!("- {}: {}\n", palabra, descripcion));
    }

    salida.push_str("\n## Operadores (de mayor a menor precedencia)\n");
    for (operador, descripcion) in OPERADORES {
        salida.push_str(&format!("- {}: {}\n", operador, descripcion));
    }

    salida.push_str("\n## Tipos de valor\n");
    for (tipo, descripcion) in TIPOS {
        salida.push_str(&format!("- {}: {}\n", tipo, descripcion));
    }

    salida.push_str("\n## Reglas de nombres\n");
    salida.push_str("- Las acciones y las consultas se escriben con un verbo en infinitivo: mostrar, medir, agregar, guardar, leer, borrar, contar, listar_claves, servir, visitar, comprobar.\n");
    salida.push_str("- Las conversiones se escriben a_<tipo>: a_numero(texto) y a_texto(valor).\n");
    salida.push_str("- Las palabras de un nombre se unen con _ (por ejemplo listar_claves). No hay excepciones.\n");
    salida.push_str("- Los nombres antiguos numero, largo y claves ya no existen: dan el error E021 con su nombre nuevo y arreglo automático.\n");

    salida.push_str("\n## Funciones integradas (firma, qué hace, ejemplo)\n");
    for integrada in INTEGRADAS {
        salida.push_str(&format!(
            "- {} [{}]: {}. Ej: {}\n",
            integrada.firma(),
            integrada.categoria,
            integrada.descripcion,
            integrada.ejemplo
        ));
    }

    salida.push_str("\n## Rutas web, datos y pruebas\n");
    salida.push_str("- ruta \"/camino\" { devolver \"<h1>texto</h1>\" } y ruta \"/saludo/:nombre\" { ... }: cada ruta devuelve el texto de la página; después, servir(8080) al final.\n");
    salida.push_str("- guardar/leer/borrar/contar/listar_claves guardan datos permanentes en archivos (ver arriba).\n");
    salida.push_str("- prueba \"nombre\" { comprobar(...) comprobar_igual(obtenido, esperado) visitar(\"/camino\") } se corre con deimon probar.\n");
    salida.push_str("- usar \"piezas/calculos.dmn\" trae las funciones, rutas y pruebas de ese archivo (y las de los archivos que él use). Un mismo archivo se carga una vez; un uso circular o un nombre de función o ruta repetido entre archivos es un error.\n");

    salida.push_str("\n## Órdenes de la línea de comandos\n");
    for (orden, descripcion) in ORDENES {
        salida.push_str(&format!("- {}: {}\n", orden, descripcion));
    }

    salida.push_str("\n## Compilar para otros sistemas\n");
    salida.push_str("- deimon destinos muestra cada destino con ✓ o ✗, lo que falta y la orden exacta para instalarlo.\n");
    salida.push_str("- deimon compilar archivo.dmn --destino NOMBRE traduce el programa, corre sus pruebas y llama a rustc con el triple del destino; sin --destino usa el del ordenador actual.\n");
    salida.push_str("- La salida por defecto es <nombre>-<destino> junto al .dmn (con .exe en windows y .iso en pc-sin-sistema); --salida RUTA elige otro lugar.\n");

    salida.push_str("\n## Sistema operativo propio (destino pc-sin-sistema)\n");
    salida.push_str("- deimon compilar archivo.dmn --destino pc-sin-sistema crea una imagen .iso que arranca sola en un PC (GRUB, sin Linux): el programa ES el sistema. deimon arrancar archivo.dmn la enciende en QEMU.\n");
    salida.push_str("- Funciona todo el lenguaje (también intentar), las integradas básicas, de conversiones, de listas y textos y de máquina, y el módulo texto o módulos escritos solo en Deimon. No hay disco, red ni navegador: guardar, rutas, sesiones y módulos con Rust dan el error E026 al compilar.\n");
    salida.push_str("- Pantalla de 80 × 25: limpiar_pantalla, escribir_en(fila, columna, texto), poner_color(letra, fondo), mostrar y escribir. Teclado español: leer_tecla y pedir_texto. apagar_maquina y reiniciar_maquina.\n");
    salida.push_str("- Las mismas funciones van en la terminal: prueba el programa con deimon ejecutar y deimon probar antes de arrancarlo. Un error sin intentar se escribe en rojo y detiene la máquina.\n");

    salida.push_str("\n## Perfiles de recursos (--perfil en ejecutar, compilar y app)\n");
    for (perfil, descripcion) in PERFILES {
        salida.push_str(&format!("- {}: {}\n", perfil, descripcion));
    }
    salida.push_str("- Sin --perfil se usa equilibrado. El perfil se graba en el programa y las variables de entorno lo sobrescriben.\n");
    salida.push_str("- Con --perfil pequeño también se acepta 'pequeno'. deimon compilar muestra el tamaño del ejecutable en KB.\n");

    salida.push_str("\n## Variables de entorno\n");
    for (variable, descripcion) in VARIABLES_DE_ENTORNO {
        salida.push_str(&format!("- {}: {}\n", variable, descripcion));
    }

    salida.push_str("\n## Si vienes de otro lenguaje (se avisa con el código E008)\n");
    for (otro, nuestro) in EQUIVALENCIAS {
        salida.push_str(&format!("- {} -> {}\n", otro, nuestro));
    }

    salida.push_str("\n## Códigos de error (primera línea: «Error en la línea N: ...»)\n");
    for (codigo, descripcion) in CODIGOS {
        salida.push_str(&format!("- {}: {}\n", codigo, descripcion));
    }
    salida.push_str("Cada error trae la línea de código con ^, «Sugerencia: ...» y, si es seguro, un arreglo automático (deimon comprobar --arreglar).\n");

    if !extensiones.is_empty() {
        salida.push_str("\n## Módulos usados (funciones en Rust)\n");
        for extension in extensiones {
            for funcion in &extension.funciones {
                salida.push_str(&format!(
                    "- {} [{}]: {}. Ej: {}\n",
                    funcion.firma(),
                    extension.nombre,
                    funcion.descripcion,
                    funcion.ejemplo
                ));
            }
        }
    }
    salida
}

fn pares_json(pares: &[(&str, &str)], clave_a: &str, clave_b: &str) -> String {
    let elementos: Vec<String> = pares
        .iter()
        .map(|(a, b)| {
            format!(
                "{{{}:{},{}:{}}}",
                json::texto(clave_a),
                json::texto(a),
                json::texto(clave_b),
                json::texto(b)
            )
        })
        .collect();
    format!("[{}]", elementos.join(","))
}

/// La guía completa como JSON.
pub fn json() -> String {
    json_con_extensiones(&[])
}

/// Como `json`, y añade las funciones de las extensiones usadas por un archivo.
pub fn json_con_extensiones(extensiones: &[Extension]) -> String {
    let integradas: Vec<String> = INTEGRADAS
        .iter()
        .map(|i| {
            let parametros: Vec<String> = i.parametros.iter().map(|p| json::texto(p)).collect();
            format!(
                "{{\"nombre\":{},\"firma\":{},\"parametros\":[{}],\"categoria\":{},\"descripcion\":{},\"ejemplo\":{}}}",
                json::texto(i.nombre),
                json::texto(&i.firma()),
                parametros.join(","),
                json::texto(i.categoria),
                json::texto(i.descripcion),
                json::texto(i.ejemplo)
            )
        })
        .collect();
    let reglas: Vec<String> = REGLAS.iter().map(|r| json::texto(r)).collect();
    let usadas: Vec<String> = extensiones
        .iter()
        .flat_map(|extension| {
            extension.funciones.iter().map(move |funcion| {
                format!(
                    "{{\"nombre\":{},\"firma\":{},\"modulo\":{},\"descripcion\":{},\"ejemplo\":{}}}",
                    json::texto(&funcion.nombre),
                    json::texto(&funcion.firma()),
                    json::texto(&extension.nombre),
                    json::texto(&funcion.descripcion),
                    json::texto(&funcion.ejemplo)
                )
            })
        })
        .collect();
    format!(
        "{{\"lenguaje\":\"deimon\",\"reglas\":[{}],\"palabras_clave\":{},\"operadores\":{},\"tipos\":{},\"perfiles\":{},\"integradas\":[{}],\"modulos_usados\":[{}],\"ordenes\":{},\"variables_de_entorno\":{},\"desde_otro_lenguaje\":{},\"codigos_de_error\":{}}}",
        reglas.join(","),
        pares_json(PALABRAS_CLAVE, "palabra", "descripcion"),
        pares_json(OPERADORES, "operador", "descripcion"),
        pares_json(TIPOS, "tipo", "descripcion"),
        pares_json(PERFILES, "perfil", "descripcion"),
        integradas.join(","),
        usadas.join(","),
        pares_json(ORDENES, "orden", "descripcion"),
        pares_json(VARIABLES_DE_ENTORNO, "variable", "descripcion"),
        pares_json(EQUIVALENCIAS, "otro", "deimon"),
        pares_json(CODIGOS, "codigo", "descripcion"),
    )
}
