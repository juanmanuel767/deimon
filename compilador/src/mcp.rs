//! `deimon mcp`: servidor MCP (Model Context Protocol) por la entrada y salida estándar, para
//! que una IA use Deimon directamente: comprobar, ejecutar, probar, dar formato, leer la guía y
//! buscar funciones que de verdad existen (así no se las inventa).
//!
//! Cada herramienta llama al propio `deimon` en un proceso aparte con límites (tiempo, pasos y
//! salida), de modo que un programa que falla o no termina nunca tumba el servidor.

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use deimon::interprete::{leer_json, Json};

use crate::utilidades::{ejecutar_con_limite, json_texto};

type Fallo = (String, i32);

const VERSION_PROTOCOLO: &str = "2025-06-18";
/// Límites con los que se ejecutan los programas que pide la IA.
const MAX_PASOS: &str = "2000000000";
const MAX_SALIDA: usize = 64 * 1024;
const TIEMPO_MAXIMO: Duration = Duration::from_secs(60);

/// `deimon mcp`
pub fn mcp(argumentos: &[String]) -> Result<(), Fallo> {
    if !argumentos.is_empty() {
        return Err(("Uso: deimon mcp   (lo arranca una IA; habla JSON-RPC por la entrada y la salida)".to_string(), 2));
    }
    let entrada = io::stdin();
    let mut salida = io::stdout();
    for linea in entrada.lock().lines() {
        let Ok(linea) = linea else { break };
        if linea.trim().is_empty() {
            continue;
        }
        if let Some(respuesta) = atender(&linea) {
            let _ = writeln!(salida, "{}", respuesta);
            let _ = salida.flush();
        }
    }
    Ok(())
}

/// Atiende un mensaje JSON-RPC. Las notificaciones (sin `id`) no llevan respuesta.
fn atender(linea: &str) -> Option<String> {
    let mensaje = match leer_json(linea) {
        Ok(mensaje) => mensaje,
        Err(error) => return Some(error_rpc("null", -32700, &format!("JSON mal formado: {}", error))),
    };
    let id = mensaje.campo("id").map(Json::escribir)?;
    let metodo = mensaje.campo("method").and_then(Json::como_texto).unwrap_or("");
    let parametros = mensaje.campo("params").cloned().unwrap_or(Json::Objeto(Vec::new()));
    let resultado = match metodo {
        "initialize" => {
            let version = parametros.campo("protocolVersion").and_then(Json::como_texto).unwrap_or(VERSION_PROTOCOLO);
            format!(
                "{{\"protocolVersion\":{},\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"deimon\",\"version\":{}}},\"instructions\":{}}}",
                json_texto(version),
                json_texto(env!("CARGO_PKG_VERSION")),
                json_texto("Deimon es un lenguaje en español. Antes de escribir código busca las funciones con deimon_buscar_funcion y lee deimon_guia; después de escribirlo usa deimon_comprobar y deimon_probar, y corrige con los errores (traen código, línea y sugerencia).")
            )
        }
        "ping" => "{}".to_string(),
        "tools/list" => format!("{{\"tools\":[{}]}}", HERRAMIENTAS.iter().map(|h| h.descripcion_json()).collect::<Vec<_>>().join(",")),
        "tools/call" => {
            let nombre = parametros.campo("name").and_then(Json::como_texto).unwrap_or("");
            let argumentos = parametros.campo("arguments").cloned().unwrap_or(Json::Objeto(Vec::new()));
            match HERRAMIENTAS.iter().find(|h| h.nombre == nombre) {
                Some(herramienta) => {
                    let (texto, es_error) = match (herramienta.funcion)(&argumentos) {
                        Ok(texto) => (texto, false),
                        Err(texto) => (texto, true),
                    };
                    format!(
                        "{{\"content\":[{{\"type\":\"text\",\"text\":{}}}],\"isError\":{}}}",
                        json_texto(&texto),
                        es_error
                    )
                }
                None => return Some(error_rpc(&id, -32602, &format!("la herramienta '{}' no existe", nombre))),
            }
        }
        otro => return Some(error_rpc(&id, -32601, &format!("método desconocido: {}", otro))),
    };
    Some(format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}", id, resultado))
}

fn error_rpc(id: &str, codigo: i32, mensaje: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":{}}}}}",
        id,
        codigo,
        json_texto(mensaje)
    )
}

struct Herramienta {
    nombre: &'static str,
    descripcion: &'static str,
    /// Propiedades del `inputSchema`, ya en JSON.
    propiedades: &'static str,
    obligatorias: &'static [&'static str],
    funcion: fn(&Json) -> Result<String, String>,
}

impl Herramienta {
    fn descripcion_json(&self) -> String {
        format!(
            "{{\"name\":{},\"description\":{},\"inputSchema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}}}",
            json_texto(self.nombre),
            json_texto(self.descripcion),
            self.propiedades,
            self.obligatorias.iter().map(|o| json_texto(o)).collect::<Vec<_>>().join(",")
        )
    }
}

const PROGRAMA: &str = "\"codigo\":{\"type\":\"string\",\"description\":\"código Deimon (si no se da archivo)\"},\"archivo\":{\"type\":\"string\",\"description\":\"ruta de un archivo .dmn (si no se da codigo)\"}";

const HERRAMIENTAS: &[Herramienta] = &[
    Herramienta {
        nombre: "deimon_comprobar",
        descripcion: "Revisa un programa Deimon sin ejecutarlo y devuelve sus errores en JSON (código, línea, columna, mensaje, sugerencia y arreglo).",
        propiedades: PROGRAMA,
        obligatorias: &[],
        funcion: comprobar,
    },
    Herramienta {
        nombre: "deimon_ejecutar",
        descripcion: "Ejecuta un programa Deimon con el intérprete (al instante) y devuelve lo que escribe, sus errores y el código de salida. Con límites: 60 s, 2 000 millones de pasos y 64 KB de salida.",
        propiedades: PROGRAMA,
        obligatorias: &[],
        funcion: ejecutar,
    },
    Herramienta {
        nombre: "deimon_probar",
        descripcion: "Corre los bloques `prueba` del programa y devuelve el resultado de cada una en JSON.",
        propiedades: PROGRAMA,
        obligatorias: &[],
        funcion: probar,
    },
    Herramienta {
        nombre: "deimon_formatear",
        descripcion: "Devuelve el código con el formato oficial de Deimon (no cambia lo que hace).",
        propiedades: "\"codigo\":{\"type\":\"string\",\"description\":\"código Deimon\"}",
        obligatorias: &["codigo"],
        funcion: formatear,
    },
    Herramienta {
        nombre: "deimon_guia",
        descripcion: "Referencia completa del lenguaje para una IA: sintaxis, palabras, funciones integradas, módulos y órdenes.",
        propiedades: "",
        obligatorias: &[],
        funcion: guia,
    },
    Herramienta {
        nombre: "deimon_buscar_funcion",
        descripcion: "Busca funciones que existen de verdad (integradas y de los módulos de la biblioteca) por nombre o descripción. Devuelve su firma, para qué sirve, un ejemplo y el `usar modulo` que necesita.",
        propiedades: "\"texto\":{\"type\":\"string\",\"description\":\"palabra a buscar, por ejemplo 'fecha', 'json' o 'mayusculas'\"}",
        obligatorias: &["texto"],
        funcion: buscar_funcion,
    },
];

/// Carpeta temporal propia para una llamada.
fn carpeta_temporal() -> Result<PathBuf, String> {
    let marca = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let carpeta = std::env::temp_dir().join(format!("deimon_mcp_{}_{}", std::process::id(), marca));
    fs::create_dir_all(&carpeta).map_err(|error| format!("no se pudo crear la carpeta temporal: {}", error))?;
    Ok(carpeta)
}

/// El programa que pide la herramienta: un archivo existente o el código dado (en una carpeta
/// temporal, que se borra después). Devuelve (archivo, carpeta donde ejecutar, carpeta a borrar).
fn programa(argumentos: &Json) -> Result<(PathBuf, PathBuf, Option<PathBuf>), String> {
    if let Some(archivo) = argumentos.campo("archivo").and_then(Json::como_texto) {
        let ruta = fs::canonicalize(archivo).map_err(|error| format!("no se pudo abrir '{}': {}", archivo, error))?;
        let carpeta = ruta.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        return Ok((ruta, carpeta, None));
    }
    let codigo = argumentos
        .campo("codigo")
        .and_then(Json::como_texto)
        .ok_or_else(|| "falta 'codigo' o 'archivo'".to_string())?;
    let carpeta = carpeta_temporal()?;
    let ruta = carpeta.join("programa.dmn");
    fs::write(&ruta, codigo).map_err(|error| format!("no se pudo escribir el programa: {}", error))?;
    Ok((ruta, carpeta.clone(), Some(carpeta)))
}

/// Ejecuta el propio `deimon` con los límites y devuelve (salida, errores, código).
fn deimon(argumentos: &[&str], carpeta: &Path) -> Result<(String, String, Option<i32>), String> {
    let ejecutable = std::env::current_exe().map_err(|error| format!("no se encontró deimon: {}", error))?;
    let mut orden = Command::new(ejecutable);
    orden
        .args(argumentos)
        .current_dir(carpeta)
        .env("DEIMON_MAX_PASOS", MAX_PASOS)
        .env("DEIMON_MAX_SALIDA", MAX_SALIDA.to_string())
        .env_remove("DEIMON_MOTOR_MOTIVO");
    let salida = ejecutar_con_limite(&mut orden, TIEMPO_MAXIMO)?;
    let recortar = |mut texto: String| {
        if texto.len() > MAX_SALIDA {
            let mut corte = MAX_SALIDA;
            while !texto.is_char_boundary(corte) {
                corte -= 1;
            }
            texto.truncate(corte);
            texto.push_str("\n… (salida recortada)");
        }
        texto
    };
    Ok((recortar(salida.stdout), recortar(salida.stderr), salida.codigo))
}

fn con_programa(
    argumentos: &Json,
    orden: impl FnOnce(&Path, &Path) -> Result<String, String>,
) -> Result<String, String> {
    let (archivo, carpeta, borrar) = programa(argumentos)?;
    let resultado = orden(&archivo, &carpeta);
    if let Some(borrar) = borrar {
        let _ = fs::remove_dir_all(borrar);
    }
    resultado
}

fn comprobar(argumentos: &Json) -> Result<String, String> {
    con_programa(argumentos, |archivo, carpeta| {
        let (salida, errores, _) = deimon(&["comprobar", "--json", &archivo.to_string_lossy()], carpeta)?;
        if salida.trim().is_empty() { Err(errores) } else { Ok(salida) }
    })
}

fn ejecutar(argumentos: &Json) -> Result<String, String> {
    con_programa(argumentos, |archivo, carpeta| {
        let (salida, errores, codigo) = deimon(&["ejecutar", &archivo.to_string_lossy()], carpeta)?;
        let texto = format!(
            "{{\"salida\":{},\"errores\":{},\"codigo_de_salida\":{}}}",
            json_texto(&salida),
            json_texto(&errores),
            codigo.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string())
        );
        if codigo == Some(0) { Ok(texto) } else { Err(texto) }
    })
}

fn probar(argumentos: &Json) -> Result<String, String> {
    con_programa(argumentos, |archivo, carpeta| {
        let (salida, errores, codigo) = deimon(&["probar", "--json", &archivo.to_string_lossy()], carpeta)?;
        let texto = if salida.trim().is_empty() { errores } else { salida };
        if codigo == Some(0) { Ok(texto) } else { Err(texto) }
    })
}

fn formatear(argumentos: &Json) -> Result<String, String> {
    let codigo = argumentos.campo("codigo").and_then(Json::como_texto).ok_or("falta 'codigo'")?;
    deimon::formato::formatear(codigo).map_err(|error| deimon::texto_de_errores(&[error]))
}

fn guia(_: &Json) -> Result<String, String> {
    let carpeta = std::env::temp_dir();
    let (salida, errores, codigo) = deimon(&["guia"], &carpeta)?;
    if codigo == Some(0) { Ok(salida) } else { Err(errores) }
}

fn buscar_funcion(argumentos: &Json) -> Result<String, String> {
    let texto = argumentos.campo("texto").and_then(Json::como_texto).ok_or("falta 'texto'")?.to_lowercase();
    let mut encontradas = Vec::new();
    for integrada in deimon::integradas::INTEGRADAS {
        if integrada.nombre.contains(&texto) || integrada.descripcion.to_lowercase().contains(&texto) {
            encontradas.push(format!(
                "{{\"firma\":{},\"descripcion\":{},\"ejemplo\":{},\"necesita\":null}}",
                json_texto(&integrada.firma()),
                json_texto(integrada.descripcion),
                json_texto(integrada.ejemplo)
            ));
        }
    }
    for (ruta, contenido) in deimon::biblioteca::ARCHIVOS {
        let Some(modulo) = ruta.strip_suffix("/funciones.txt") else { continue };
        for linea in contenido.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let partes: Vec<&str> = linea.splitn(4, '|').map(str::trim).collect();
            let [nombre, cuantos, descripcion, ejemplo] = partes[..] else { continue };
            if nombre.contains(&texto) || descripcion.to_lowercase().contains(&texto) || modulo == texto {
                encontradas.push(format!(
                    "{{\"firma\":{},\"descripcion\":{},\"ejemplo\":{},\"necesita\":{}}}",
                    json_texto(&format!("{}({} {})", nombre, cuantos, if cuantos == "1" { "argumento" } else { "argumentos" })),
                    json_texto(descripcion),
                    json_texto(ejemplo),
                    json_texto(&format!("usar modulo \"{}\"", modulo))
                ));
            }
        }
    }
    if encontradas.is_empty() {
        return Err(format!("no hay ninguna función con '{}': no la inventes; prueba con otra palabra", texto));
    }
    Ok(format!("[{}]", encontradas.join(",")))
}
