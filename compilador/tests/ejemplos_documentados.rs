//! La documentación no puede mentir: cada ejemplo que se enseña (biblioteca/*/funciones.txt,
//! la tabla de integradas, y los bloques de código de README.md y docs/*.md) se convierte en un
//! programa mínimo y se pasa por `deimon comprobar --json`. Los que no necesitan red, servicios,
//! rutas web ni sesiones se ejecutan además con el intérprete, y si el ejemplo anuncia su
//! resultado con un comentario (`# 3`, `# escribe 42`) se compara con lo que escribe de verdad.
//!
//! Para que nada se salte en silencio hay cuatro listas explícitas, cada entrada con su motivo:
//! EXCEPCIONES (no se comprueban), NO_EJECUTAR (solo se comprueban), NO_ES_RESULTADO
//! (comentarios que no anuncian un resultado) y SIN_RESULTADO (funciones de biblioteca cuyo
//! ejemplo se ejecuta pero no puede anunciar su resultado). Un ejemplo nuevo se comprueba solo;
//! si falla, la prueba falla hasta arreglarlo o apuntarlo en la lista que toque.
//!
//! Regla fija: todo ejemplo de biblioteca/*/funciones.txt que se ejecuta DEBE anunciar su
//! resultado con `# ...`, para que quien lea la guía vea qué devuelve la función.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Ejemplos que no se pueden comprobar de ninguna manera: (origen, motivo).
const EXCEPCIONES: &[(&str, &str)] = &[
    (
        "docs/modulos.md#1",
        "usa un archivo propio del usuario (piezas/calculos.dmn) que no existe en la documentación",
    ),
];

/// Funciones o palabras que impiden ejecutar el ejemplo (solo se comprueba): (palabra, motivo).
const NO_EJECUTAR: &[(&str, &str)] = &[
    ("ruta", "las rutas web necesitan el servidor"),
    ("servir", "levanta un servidor que no termina"),
    ("pedir_web", "necesita red"),
    ("enviar_web", "necesita red"),
    ("enviar_correo", "necesita sendmail"),
    ("enviar_whatsapp", "necesita WhatsApp y red"),
    ("enviar_plantilla_whatsapp", "necesita WhatsApp y red"),
    ("verificar_whatsapp", "solo vale dentro de una ruta web"),
    ("mensajes_whatsapp", "solo vale dentro de una ruta web"),
    ("crear_pago_mercadopago", "necesita Mercado Pago y red"),
    ("consultar_pago_mercadopago", "necesita Mercado Pago y red"),
    ("crear_pago_paypal", "necesita PayPal y red"),
    ("cobrar_pago_paypal", "necesita PayPal y red"),
    ("crear_pago", "necesita Stripe y red"),
    ("pago_completado", "necesita Stripe y red"),
    ("direccion_google", "necesita Google"),
    ("usuario_google", "necesita Google y red"),
    ("ejecutar_mysql", "necesita un servidor MySQL"),
    ("consultar_mysql", "necesita un servidor MySQL"),
    ("ultimo_id_mysql", "necesita un servidor MySQL"),
    ("campo", "lee el formulario de una petición web"),
    ("es_envio", "solo vale dentro de una petición web"),
    ("redirigir", "solo vale dentro de una petición web"),
    ("redirigir_fuera", "solo vale dentro de una petición web"),
    ("archivo_subido", "necesita una petición web con archivo"),
    ("guardar_subida", "necesita una petición web con archivo"),
    ("responder_archivo", "solo vale dentro de una petición web"),
    ("responder", "solo vale dentro de una petición web"),
    ("responder_json", "solo vale dentro de una petición web"),
    ("metodo_recibido", "necesita una petición web"),
    ("camino_recibido", "necesita una petición web"),
    ("cabecera_recibida", "necesita una petición web"),
    ("cuerpo_recibido", "necesita una petición web"),
    ("leer_galleta", "necesita un navegador"),
    ("guardar_galleta", "necesita un navegador"),
    ("borrar_galleta", "necesita un navegador"),
    ("plantilla", "devuelve una página web desde una ruta"),
    ("crear_usuario", "usuarios y sesiones: necesita un navegador"),
    ("comprobar_clave", "usuarios y sesiones: necesita un navegador"),
    ("iniciar_sesion", "usuarios y sesiones: necesita un navegador"),
    ("usuario_actual", "usuarios y sesiones: necesita un navegador"),
    ("cerrar_sesion", "usuarios y sesiones: necesita un navegador"),
    ("cuenta_bloqueada", "usuarios y sesiones: necesita un navegador"),
    ("cambiar_clave", "usuarios y sesiones: necesita un navegador"),
    ("crear_codigo_recuperacion", "usuarios y sesiones: necesita un navegador"),
    ("restablecer_clave", "usuarios y sesiones: necesita un navegador"),
    ("guardar_en_sesion", "sesiones: necesita un navegador"),
    ("leer_de_sesion", "sesiones: necesita un navegador"),
    ("agregar_a_sesion", "sesiones: necesita un navegador"),
    ("borrar_de_sesion", "sesiones: necesita un navegador"),
    ("vaciar_sesion", "sesiones: necesita un navegador"),
    ("visitar", "pruebas de rutas: se ejecutan con `deimon probar`"),
    ("leer_tecla", "espera a que se pulse una tecla"),
    ("pedir_texto", "espera a que se escriba una línea"),
    ("apagar_maquina", "termina el programa"),
    ("reiniciar_maquina", "termina el programa"),
    ("enviar", "pruebas de rutas: se ejecutan con `deimon probar`"),
    ("mientras condicion", "bucle de esquema que no termina"),
    ("esperar", "espera de verdad; no aporta nada ejecutarla"),
    ("leer_csv", "lee un archivo que el ejemplo no crea"),
    ("consultar_sql", "necesita una base de datos que el ejemplo no crea"),
    ("ejecutar_sql", "necesita una base de datos que el ejemplo no crea"),
];

/// Comentarios que NO anuncian un resultado (se ignoran al comparar): (comentario, motivo).
const NO_ES_RESULTADO: &[(&str, &str)] = &[
    ("al final del programa, después de las rutas", "explica dónde va"),
    ("suma atómica, devuelve el total", "explica qué devuelve"),
    ("cuántas claves hay", "explica qué devuelve"),
    ("lista de claves, ordenada", "explica qué devuelve"),
    ("escribe 30 si ya guardaste ese dato", "depende de un guardar anterior"),
    ("dentro de ruta \"/whatsapp\": si r != nada, devolver r", "explica cómo se usa"),
    (
        "el texto tal cual; para leerlo como datos: usar modulo \"json\" y desde_json(cuerpo)",
        "explica cómo se usa",
    ),
    ("un dado", "explica para qué sirve, el número es al azar"),
    ("y lo siguiente sigue en la misma línea", "explica el comportamiento"),
    ("datos[\"usuario\"][\"nombre\"]", "enseña cómo leer el resultado, no lo que escribe"),
    ("lista nueva, la original no cambia", "explica el comportamiento"),
    ("une dos listas", "explica el comportamiento"),
    ("cambia un valor", "explica el comportamiento"),
    ("añade uno nuevo", "explica el comportamiento"),
    ("recorre las claves en orden", "explica el comportamiento"),
    ("nada si no llegó archivo", "explica el comportamiento"),
];

/// Funciones de biblioteca cuyo ejemplo se ejecuta pero no anuncia su resultado: (función, motivo).
const SIN_RESULTADO: &[(&str, &str)] = &[
    ("listar_archivos", "el resultado depende de lo que haya en la carpeta"),
    ("grafico_barras", "devuelve un SVG de una sola línea muy larga"),
    ("grafico_lineas", "devuelve un SVG de una sola línea muy larga"),
    ("grafico_tarta", "devuelve un SVG de una sola línea muy larga"),
    ("aleatorio", "el número es distinto cada vez"),
    ("entero_al_azar", "el número es distinto cada vez"),
    ("ahora", "la hora es distinta cada vez"),
    ("titulo", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("parrafo", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("elemento_lista", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("separador", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("boton", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("cuadro_texto", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("cuadro_clave", "devuelve el formato interno de la pantalla, con tabuladores"),
    ("boton_enviar", "devuelve el formato interno de la pantalla, con tabuladores"),
];

/// Valores de muestra para las variables que un ejemplo usa sin definir.
const MUESTRAS: &[(&str, &str)] = &[
    ("cliente", "\"123-abc.apps.googleusercontent.com\""),
    ("secreto", "\"secreto-de-muestra\""),
    ("secreto_app", "\"secreto-de-muestra\""),
    ("token", "\"token-de-muestra\""),
    ("clave", "\"clave-de-muestra\""),
    ("id", "\"id-de-muestra\""),
    ("id_numero", "\"123456789\""),
    ("respuesta", "\"{\\\"usuario\\\": {\\\"nombre\\\": \\\"Ana\\\"}}\""),
    ("edad", "30"),
    ("color", "\"rojo\""),
    ("lista", "[{\"nombre\": \"taza\", \"precio\": 8.5}]"),
    ("persona", "{\"nombre\": \"Ana\", \"edad\": 30}"),
    ("direccion_de_pago", "\"https://pago.ejemplo.com\""),
    ("nombre", "\"ana\""),
    ("actual", "\"clave-actual\""),
    ("nueva", "\"clave-nueva-123\""),
    ("codigo", "\"codigo-de-muestra\""),
    ("condicion", "verdadero"),
    ("otra_condicion", "falso"),
];

/// Código que hace falta antes de un ejemplo que usa algo que define el propio programa.
const PRELUDIOS: &[(&str, &str)] = &[
    ("suma(", "funcion suma(a, b) {\n    devolver a + b\n}\n"),
    ("es_par)", "funcion es_par(n) {\n    devolver n % 2 == 0\n}\n"),
    ("doble)", "funcion doble(n) {\n    devolver n * 2\n}\n"),
    (
        "visitar(\"/saludo/Ana\")",
        "ruta \"/saludo/:nombre\" {\n    devolver \"<h1>Hola, \" + nombre + \"</h1>\"\n}\n",
    ),
    ("servir(", "ruta \"/\" {\n    devolver \"inicio\"\n}\n"),
    (
        "enviar(\"/entrar\"",
        "ruta \"/entrar\" {\n    devolver redirigir(\"/\")\n}\nruta \"/\" {\n    devolver \"inicio\"\n}\n",
    ),
];

/// Líneas `sea x = muestra` para las variables de muestra que el texto usa sin definir.
fn declaraciones_de(texto: &str) -> Vec<String> {
    let ya = definidos(texto);
    let mut hechas = BTreeSet::new();
    let mut lineas = Vec::new();
    for (palabra, anterior, con_parentesis) in palabras(texto) {
        if con_parentesis || anterior == "sea" || anterior == "para" || ya.contains(&palabra) {
            continue;
        }
        if let Some((_, valor)) = MUESTRAS.iter().find(|(n, _)| *n == palabra) {
            if hechas.insert(palabra.clone()) {
                lineas.push(format!("sea {} = {}", palabra, valor));
            }
        }
    }
    lineas
}

static COMPARADOS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Ejemplo {
    origen: String,
    modulo: Option<String>,
    codigo: String,
    /// Parámetros anunciados en funciones.txt (solo en los ejemplos de biblioteca).
    parametros: Vec<String>,
}

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn carpeta(nombre: &str) -> PathBuf {
    let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("ejemplos_doc_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&c);
    fs::create_dir_all(&c).expect("no se pudo crear la carpeta de prueba");
    c
}

// ---------------------------------------------------------------- inventario

fn ejemplos_de_biblioteca(salida: &mut Vec<Ejemplo>) {
    let mut modulos: Vec<_> = fs::read_dir(raiz().join("biblioteca"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("funciones.txt").is_file())
        .collect();
    modulos.sort();
    for ruta in modulos {
        let modulo = ruta.file_name().unwrap().to_string_lossy().to_string();
        let texto = fs::read_to_string(ruta.join("funciones.txt")).unwrap();
        for linea in texto.lines() {
            if linea.trim().is_empty() || linea.starts_with('#') {
                continue;
            }
            let campos: Vec<&str> = linea.splitn(4, " | ").collect();
            assert_eq!(campos.len(), 4, "línea mal hecha en {}: {}", modulo, linea);
            salida.push(Ejemplo {
                origen: format!("biblioteca/{}/funciones.txt:{}", modulo, campos[0].trim()),
                modulo: Some(modulo.clone()),
                codigo: campos[3].trim().to_string(),
                parametros: campos[1].split(',').map(|p| p.trim().to_string()).filter(|p| p != "0" && !p.is_empty()).collect(),
            });
        }
    }
}

fn ejemplos_de_integradas(salida: &mut Vec<Ejemplo>) {
    for i in deimon::integradas::INTEGRADAS {
        salida.push(Ejemplo {
            origen: format!("integradas:{}", i.nombre),
            modulo: None,
            codigo: i.ejemplo.to_string(),
            parametros: Vec::new(),
        });
    }
}

fn es_codigo_deimon(lenguaje: &str, lineas: &[&str]) -> bool {
    match lenguaje {
        "deimon" | "" => true,
        "text" => lineas.iter().any(|l| l.starts_with("funcion ") || l.starts_with("prueba ")),
        _ => false,
    }
}

fn ejemplos_de_documentos(salida: &mut Vec<Ejemplo>) {
    let mut archivos = vec![PathBuf::from("README.md")];
    let mut docs: Vec<_> = fs::read_dir(raiz().join("docs"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        // TRASPASO.md son notas internas (fuera de git), no documentación del lenguaje.
        .filter(|p| p.file_name().map(|n| n != "TRASPASO.md").unwrap_or(true))
        .map(|p| PathBuf::from("docs").join(p.file_name().unwrap()))
        .collect();
    docs.sort();
    archivos.extend(docs);
    for relativo in archivos {
        let texto = fs::read_to_string(raiz().join(&relativo)).unwrap();
        let mut dentro: Option<(String, Vec<&str>)> = None;
        let mut numero = 0;
        for linea in texto.lines() {
            if let Some(marca) = linea.strip_prefix("```") {
                match dentro.take() {
                    Some((lenguaje, lineas)) => {
                        if es_codigo_deimon(&lenguaje, &lineas) {
                            salida.push(Ejemplo {
                                origen: format!("{}#{}", relativo.display(), numero),
                                modulo: None,
                                codigo: lineas.join("\n"),
                                parametros: Vec::new(),
                            });
                        }
                    }
                    None => {
                        numero += 1;
                        dentro = Some((marca.trim().to_string(), Vec::new()));
                    }
                }
            } else if let Some((_, lineas)) = dentro.as_mut() {
                lineas.push(linea);
            }
        }
    }
}

// ---------------------------------------------------------------- análisis del texto

/// Separa la parte de código y el comentario de una línea (el `#` dentro de un texto no cuenta).
fn partir_comentario(linea: &str) -> (&str, Option<&str>) {
    let mut en_texto = false;
    let mut escapado = false;
    for (i, c) in linea.char_indices() {
        if en_texto {
            if escapado {
                escapado = false;
            } else if c == '\\' {
                escapado = true;
            } else if c == '"' {
                en_texto = false;
            }
        } else if c == '"' {
            en_texto = true;
        } else if c == '#' {
            return (linea[..i].trim_end(), Some(linea[i + 1..].trim()));
        }
    }
    (linea, None)
}

/// Código sin textos ni comentarios, para buscar nombres sin confundirlos con el contenido.
fn sin_textos(codigo: &str) -> String {
    let mut salida = String::new();
    for linea in codigo.lines() {
        let (parte, _) = partir_comentario(linea);
        let mut en_texto = false;
        let mut escapado = false;
        for c in parte.chars() {
            if en_texto {
                if escapado {
                    escapado = false;
                } else if c == '\\' {
                    escapado = true;
                } else if c == '"' {
                    en_texto = false;
                }
                salida.push(' ');
            } else if c == '"' {
                en_texto = true;
                salida.push(' ');
            } else {
                salida.push(c);
            }
        }
        salida.push('\n');
    }
    salida
}

/// Palabras (identificadores) con la palabra anterior y si la sigue un paréntesis.
fn palabras(codigo: &str) -> Vec<(String, String, bool)> {
    let texto = sin_textos(codigo);
    let chars: Vec<char> = texto.chars().collect();
    let mut resultado = Vec::new();
    let mut anterior = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let inicio = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let palabra: String = chars[inicio..i].iter().collect();
            let precedido_por_punto = inicio > 0 && chars[inicio - 1] == '.';
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let con_parentesis = j < chars.len() && chars[j] == '(';
            if !precedido_por_punto {
                resultado.push((palabra.clone(), anterior.clone(), con_parentesis));
            }
            anterior = palabra;
        } else if chars[i].is_alphanumeric() {
            i += 1;
        } else {
            if !chars[i].is_whitespace() && chars[i] != ',' {
                anterior.clear();
            }
            i += 1;
        }
    }
    resultado
}

/// Nombres que el propio ejemplo define: `sea x`, `para x en`, parámetros y `:parametros` de ruta.
fn definidos(codigo: &str) -> BTreeSet<String> {
    let mut conjunto = BTreeSet::new();
    for (palabra, anterior, _) in palabras(codigo) {
        if anterior == "sea" || anterior == "para" {
            conjunto.insert(palabra);
        }
    }
    for linea in codigo.lines() {
        let l = linea.trim_start();
        if let Some(resto) = l.strip_prefix("funcion ") {
            if let (Some(a), Some(b)) = (resto.find('('), resto.find(')')) {
                for p in resto[a + 1..b].split(',') {
                    conjunto.insert(p.trim().to_string());
                }
            }
        }
    }
    let mut resto = codigo;
    while let Some(p) = resto.find("/:") {
        let nombre: String = resto[p + 2..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        conjunto.insert(nombre);
        resto = &resto[p + 2..];
    }
    conjunto
}

fn sustituir_puntos(codigo: &str) -> String {
    let mut lineas: Vec<String> = Vec::new();
    for linea in codigo.lines() {
        if linea.trim() == "..." {
            let sangria: String = linea.chars().take_while(|c| *c == ' ').collect();
            lineas.push(format!("{}mostrar(\"hecho\")", sangria));
        } else {
            lineas.push(linea.replace("{ ... }", "{ mostrar(\"hecho\") }"));
        }
    }
    lineas.join("\n")
}

// ---------------------------------------------------------------- construcción del programa

struct Armado {
    programa: String,
    /// (número de marca, resultado anunciado) para comparar al ejecutar.
    esperados: Vec<(usize, String, String)>,
    es_prueba: bool,
}

fn resultado_anunciado(comentario: &str) -> Option<Vec<String>> {
    if NO_ES_RESULTADO.iter().any(|(c, _)| *c == comentario) {
        return None;
    }
    let base = comentario.strip_prefix("escribe ").unwrap_or(comentario).trim();
    let mut candidatos = vec![base.to_string()];
    for corte in [" (", ";", ": "] {
        if let Some(p) = base.find(corte) {
            candidatos.push(base[..p].trim().to_string());
        }
    }
    Some(candidatos)
}

fn empieza_con_palabra(codigo: &str, palabras: &[&str]) -> bool {
    palabras.iter().any(|p| {
        codigo.strip_prefix(p).map(|r| r.starts_with(|c: char| !(c.is_alphanumeric() || c == '_'))).unwrap_or(false)
    })
}

fn armar(ej: &Ejemplo) -> Armado {
    let codigo = sustituir_puntos(&ej.codigo);
    let multilinea = codigo.contains('\n');
    let mut lineas: Vec<String> = Vec::new();
    let mut esperados = Vec::new();
    let mut profundidad: i32 = 0;
    for linea in codigo.lines() {
        let (parte, comentario) = partir_comentario(linea);
        let anunciado = comentario.and_then(resultado_anunciado);
        let abre = parte.matches('{').count() as i32 - parte.matches('}').count() as i32;
        let en_nivel_0 = profundidad == 0 && !linea.starts_with(' ') && !parte.is_empty();
        let mut hecho = false;
        if let (Some(candidatos), true) = (&anunciado, en_nivel_0 && abre == 0) {
            let n = esperados.len();
            // `mostrar(...)` y un `intentar { ... }` de una línea escriben ellos mismos el resultado.
            if parte.starts_with("mostrar(") || empieza_con_palabra(parte, &["intentar"]) {
                lineas.push(format!("mostrar(\"@@{}\")", n));
                lineas.push(parte.to_string());
                esperados.push((n, candidatos.join("\u{1}"), linea.to_string()));
                hecho = true;
            } else if let Some(resto) = parte.strip_prefix("sea ") {
                if let Some((nombre, _)) = resto.split_once(" = ") {
                    lineas.push(parte.to_string());
                    lineas.push(format!("mostrar(\"@@{}\")", n));
                    lineas.push(format!("mostrar({})", nombre.trim()));
                    esperados.push((n, candidatos.join("\u{1}"), linea.to_string()));
                    hecho = true;
                }
            } else if !multilinea
                && !empieza_con_palabra(
                    parte,
                    &["escribir", "si", "para", "mientras", "ruta", "devolver", "guardar", "borrar"],
                )
                && !parte.contains(" = ")
            {
                lineas.push(format!("mostrar(\"@@{}\")", n));
                lineas.push(format!("mostrar({})", parte));
                esperados.push((n, candidatos.join("\u{1}"), linea.to_string()));
                hecho = true;
            }
        }
        if !hecho {
            lineas.push(linea.to_string());
        }
        profundidad += abre;
    }
    let mut cuerpo = lineas.join("\n");

    // `devolver` suelto solo vale dentro de una ruta; `comprobar` dentro de una prueba.
    let mut es_prueba = false;
    let tiene = |c: &str, p: &str| palabras(c).iter().any(|(w, _, _)| w == p);
    if tiene(&cuerpo, "comprobar") || tiene(&cuerpo, "comprobar_igual") {
        if !tiene(&cuerpo, "prueba") {
            cuerpo = format!("prueba \"ejemplo\" {{\n{}\n}}", cuerpo);
        }
        es_prueba = true;
    } else if tiene(&cuerpo, "devolver") && !tiene(&cuerpo, "ruta") && !tiene(&cuerpo, "funcion") {
        cuerpo = format!("ruta \"/ejemplo\" {{\n{}\n}}", cuerpo);
    }

    // Variables usadas sin definir: se declaran con valores de muestra. Las rutas, funciones y
    // pruebas no ven las variables de fuera, así que ahí se declaran dentro.
    let mut cabecera = String::new();
    if let Some(m) = &ej.modulo {
        cabecera.push_str(&format!("usar modulo \"{}\"\n", m));
    }
    let mut trozos: Vec<Vec<String>> = Vec::new();
    let mut profundidad = 0;
    for linea in cuerpo.lines() {
        if profundidad == 0 {
            trozos.push(Vec::new());
        }
        trozos.last_mut().unwrap().push(linea.to_string());
        let (parte, _) = partir_comentario(linea);
        profundidad += parte.matches('{').count() as i32 - parte.matches('}').count() as i32;
    }
    let mut fuera = Vec::new();
    let mut texto_final: Vec<String> = Vec::new();
    for trozo in trozos {
        let primera = trozo[0].clone();
        let aislado = ["ruta ", "funcion ", "prueba "].iter().any(|k| primera.starts_with(k));
        if aislado {
            let texto = trozo.join("\n");
            let declaraciones = declaraciones_de(&texto);
            texto_final.push(primera);
            texto_final.extend(declaraciones);
            texto_final.extend(trozo[1..].iter().cloned());
        } else {
            fuera.extend(trozo.iter().cloned());
            texto_final.extend(trozo);
        }
    }
    for d in declaraciones_de(&fuera.join("\n")) {
        cabecera.push_str(&d);
        cabecera.push('\n');
    }
    cuerpo = texto_final.join("\n");
    for (disparador, preludio) in PRELUDIOS {
        let definicion = preludio.lines().next().unwrap_or("");
        if cuerpo.contains(disparador) && !cuerpo.contains(definicion) {
            cabecera.push_str(preludio);
        }
    }
    Armado { programa: format!("{}{}\n", cabecera, cuerpo), esperados, es_prueba }
}

// ---------------------------------------------------------------- ejecución

fn lanzar(c: &Path, args: &[&str], motor_interprete: bool) -> (bool, String) {
    let mut orden = Command::new(env!("CARGO_BIN_EXE_deimon"));
    orden
        .args(args)
        .current_dir(c)
        .env("TMPDIR", c)
        .env("DEIMON_DATOS", c.join("datos"))
        .env_remove("DEIMON_PUERTO");
    if motor_interprete {
        orden.env("DEIMON_MOTOR", "interprete");
    }
    let salida = orden.output().expect("no se pudo lanzar deimon");
    let texto = format!(
        "{}{}",
        String::from_utf8_lossy(&salida.stdout),
        String::from_utf8_lossy(&salida.stderr)
    );
    (salida.status.success(), texto)
}

fn se_ejecuta(ej: &Ejemplo, programa: &str) -> bool {
    let limpio = sin_textos(&format!("{}\n{}", ej.codigo, programa));
    let mut p = palabras(&limpio);
    p.sort();
    let hay = |clave: &str| {
        if clave.contains(' ') {
            limpio.contains(clave)
        } else {
            p.iter().any(|(w, _, _)| w == clave)
        }
    };
    !NO_EJECUTAR.iter().any(|(clave, _)| hay(clave))
}

/// Nombre de la función si el ejemplo viene de biblioteca/*/funciones.txt.
fn funcion_de_biblioteca(ej: &Ejemplo) -> Option<&str> {
    if ej.origen.starts_with("biblioteca/") {
        ej.origen.rsplit(':').next()
    } else {
        None
    }
}

/// Argumentos de primer nivel de la primera llamada `nombre(...)` del texto.
fn argumentos_de_llamada(codigo: &str, nombre: &str) -> Option<Vec<String>> {
    let inicio = codigo.find(&format!("{}(", nombre))? + nombre.len() + 1;
    let (mut profundidad, mut en_texto, mut escapado) = (0, false, false);
    let (mut args, mut actual) = (Vec::new(), String::new());
    for c in codigo[inicio..].chars() {
        if en_texto {
            if escapado {
                escapado = false;
            } else if c == '\\' {
                escapado = true;
            } else if c == '"' {
                en_texto = false;
            }
        } else if c == '"' {
            en_texto = true;
        } else if "([{".contains(c) {
            profundidad += 1;
        } else if ")]}".contains(c) {
            if profundidad == 0 {
                if !actual.trim().is_empty() {
                    args.push(actual.trim().to_string());
                }
                return Some(args);
            }
            profundidad -= 1;
        } else if c == ',' && profundidad == 0 {
            args.push(actual.trim().to_string());
            actual.clear();
            continue;
        }
        actual.push(c);
    }
    None
}

/// Los parámetros de fecha (`anio`, `mes`, `dia`...) deben ir en orden cronológico y, si el
/// ejemplo los pasa como números sueltos, caer en un rango razonable. Así una cabecera con los
/// parámetros en otro orden que la función real (`mes, anio` frente a `anio, mes`) no pasa.
fn problemas_de_parametros_de_fecha(ej: &Ejemplo) -> Vec<String> {
    const ORDEN: &[(&str, i64, i64)] = &[
        ("anio", 100, 9999),
        ("mes", 1, 12),
        ("dia", 1, 31),
        ("hora", 0, 23),
        ("minuto", 0, 59),
    ];
    let mut problemas = Vec::new();
    let posiciones: Vec<Option<usize>> =
        ORDEN.iter().map(|(n, _, _)| ej.parametros.iter().position(|p| p == n)).collect();
    let presentes: Vec<usize> = posiciones.iter().flatten().copied().collect();
    if presentes.windows(2).any(|w| w[0] > w[1]) {
        problemas.push(format!(
            "los parámetros ({}) no están en orden año, mes, día, hora, minuto",
            ej.parametros.join(", ")
        ));
    }
    let Some(funcion) = funcion_de_biblioteca(ej) else { return problemas };
    let Some(args) = argumentos_de_llamada(&ej.codigo, funcion) else { return problemas };
    for ((nombre, minimo, maximo), posicion) in ORDEN.iter().zip(&posiciones) {
        let Some(valor) = posicion.and_then(|p| args.get(p)).and_then(|a| a.parse::<i64>().ok()) else { continue };
        if valor < *minimo || valor > *maximo {
            problemas.push(format!(
                "el ejemplo pasa {} como `{}` (tiene que estar entre {} y {}): ¿están los parámetros en el orden real?",
                valor, nombre, minimo, maximo
            ));
        }
    }
    problemas
}

/// Devuelve (comprobado, ejecutado, anuncia su resultado).
fn comprobar_ejemplo(indice: usize, ej: &Ejemplo) -> Result<(bool, bool, bool), String> {
    let armado = armar(ej);
    let c = carpeta(&indice.to_string());
    fs::write(c.join("programa.dmn"), &armado.programa).unwrap();
    // Las plantillas que nombra el ejemplo tienen que existir para que `comprobar` las encuentre.
    for trozo in armado.programa.split("plantilla(\"").skip(1) {
        if let Some(fin) = trozo.find('"') {
            let ruta = c.join(&trozo[..fin]);
            fs::create_dir_all(ruta.parent().unwrap()).unwrap();
            fs::write(ruta, "<p>{{ nombre }}</p>\n").unwrap();
        }
    }
    let (_, json) = lanzar(&c, &["comprobar", "--json", "programa.dmn"], false);
    if !json.contains("\"correcto\":true") {
        return Err(format!("no pasa `comprobar`:\n{}\nprograma:\n{}", json.trim(), armado.programa));
    }
    let ejecutable = se_ejecuta(ej, &armado.programa);
    if !ejecutable {
        let _ = fs::remove_dir_all(&c);
        return Ok((true, false, false));
    }
    let orden = if armado.es_prueba { "probar" } else { "ejecutar" };
    let (bien, salida) = lanzar(&c, &[orden, "programa.dmn"], true);
    if !bien {
        return Err(format!("falla al `{}`:\n{}\nprograma:\n{}", orden, salida.trim(), armado.programa));
    }
    // Resultados anunciados: tras cada marca @@N viene la línea que escribió el ejemplo.
    let lineas: Vec<&str> = salida.lines().collect();
    COMPARADOS.fetch_add(armado.esperados.len(), std::sync::atomic::Ordering::SeqCst);
    for (n, candidatos, original) in &armado.esperados {
        let marca = format!("@@{}", n);
        let real = lineas
            .iter()
            .position(|l| *l == marca)
            .and_then(|p| lineas.get(p + 1))
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| "(no escribió nada)".to_string());
        if !candidatos.split('\u{1}').any(|e| e == real) {
            return Err(format!(
                "el resultado anunciado no coincide en `{}`: anuncia {:?} y escribe {:?}",
                original.trim(),
                candidatos.split('\u{1}').collect::<Vec<_>>(),
                real
            ));
        }
    }
    let _ = fs::remove_dir_all(&c);
    Ok((true, true, !armado.esperados.is_empty()))
}

#[test]
fn todos_los_ejemplos_de_la_documentacion_son_ciertos() {
    let mut ejemplos = Vec::new();
    ejemplos_de_biblioteca(&mut ejemplos);
    ejemplos_de_integradas(&mut ejemplos);
    ejemplos_de_documentos(&mut ejemplos);

    let mut fallos = Vec::new();
    let origenes: BTreeSet<&str> = ejemplos.iter().map(|e| e.origen.as_str()).collect();
    for (origen, _) in EXCEPCIONES {
        if !origenes.contains(origen) {
            fallos.push(format!("la excepción '{}' ya no corresponde a ningún ejemplo", origen));
        }
    }
    let funciones: BTreeSet<&str> = ejemplos.iter().filter_map(funcion_de_biblioteca).collect();
    for (funcion, _) in SIN_RESULTADO {
        if !funciones.contains(funcion) {
            fallos.push(format!(
                "SIN_RESULTADO: '{}' ya no es una función de biblioteca/*/funciones.txt; quítala de la lista",
                funcion
            ));
        }
    }
    for ej in &ejemplos {
        for problema in problemas_de_parametros_de_fecha(ej) {
            fallos.push(format!("[{}] {}", ej.origen, problema));
        }
    }
    let (mut comprobados, mut ejecutados, mut excepciones) = (0, 0, 0);
    // Los ejemplos son independientes: se reparten entre unos pocos hilos (los que caen al
    // modo Rust tardan unos segundos cada uno).
    let siguiente = std::sync::atomic::AtomicUsize::new(0);
    let resultados = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|ambito| {
        for _ in 0..4 {
            ambito.spawn(|| loop {
                let i = siguiente.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some(ej) = ejemplos.get(i) else { break };
                if EXCEPCIONES.iter().any(|(o, _)| *o == ej.origen) {
                    resultados.lock().unwrap().push((i, Ok(None)));
                } else {
                    let r = comprobar_ejemplo(i, ej).map(Some);
                    resultados.lock().unwrap().push((i, r));
                }
            });
        }
    });
    let mut resultados = resultados.into_inner().unwrap();
    resultados.sort_by_key(|(i, _)| *i);
    for (i, r) in resultados {
        match r {
            Ok(None) => excepciones += 1,
            Ok(Some((_, ejecutado, anuncia))) => {
                comprobados += 1;
                if ejecutado {
                    ejecutados += 1;
                }
                if let Some(funcion) = funcion_de_biblioteca(&ejemplos[i]) {
                    let en_lista = SIN_RESULTADO.iter().any(|(f, _)| *f == funcion);
                    if ejecutado && !anuncia && !en_lista {
                        fallos.push(format!(
                            "[{}] el ejemplo de `{}` no anuncia su resultado: añade `   # resultado` al final del ejemplo en su funciones.txt (ejecútalo con `deimon ejecutar` para ver el resultado exacto), o, si no puede anunciarse, apunta la función en SIN_RESULTADO con su motivo",
                            ejemplos[i].origen, funcion
                        ));
                    } else if en_lista && (anuncia || !ejecutado) {
                        fallos.push(format!(
                            "[{}] SIN_RESULTADO ya no hace falta para `{}` (el ejemplo {}); quítala de la lista",
                            ejemplos[i].origen,
                            funcion,
                            if anuncia { "ya anuncia su resultado" } else { "no se ejecuta" }
                        ));
                    }
                }
            }
            Err(e) => fallos.push(format!("[{}] {}", ejemplos[i].origen, e)),
        }
    }
    println!(
        "ejemplos: {} · comprobados: {} · ejecutados: {} · excepciones: {} · resultados comparados: {}",
        ejemplos.len(),
        comprobados,
        ejecutados,
        excepciones,
        COMPARADOS.load(std::sync::atomic::Ordering::SeqCst)
    );
    // Evita que una lista de motivos quede sin usar sin que nadie lo note.
    assert!(ejemplos.len() > 100, "se esperaban más ejemplos, ¿se movieron las carpetas?");
    assert!(fallos.is_empty(), "{} ejemplo(s) mienten:\n\n{}", fallos.len(), fallos.join("\n\n"));
}
