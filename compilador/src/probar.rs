use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::utilidades::{ejecutar_con_limite, json_texto};

type Fallo = (String, i32);

const MARCA: &str = "\u{1}LN-PRUEBA\t";

pub struct ResultadoPrueba {
    pub nombre: String,
    pub correcta: bool,
    pub detalle: String,
}

/// `deimon probar archivo.dmn [--json]`
pub fn probar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon probar archivo.dmn [--json]";
    let mut archivo = None;
    let mut json = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => {
                if archivo.is_some() {
                    return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2));
                }
                archivo = Some(otro.to_string());
            }
        }
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;
    let fuente = fs::read_to_string(&archivo).map_err(|error| {
        (
            format!("Error: no se pudo leer el archivo '{}': {}", archivo, error),
            1,
        )
    })?;

    let resultados = match correr_archivo(&fuente, Path::new(&archivo)) {
        Ok(resultados) => resultados,
        Err(mensaje) => {
            if json {
                println!(
                    "{{\"archivo\":{},\"error\":{},\"total\":0,\"correctas\":0,\"fallidas\":0,\"listo\":false,\"pruebas\":[]}}",
                    json_texto(&archivo),
                    json_texto(&mensaje)
                );
                return Err((String::new(), 1));
            }
            return Err((mensaje, 1));
        }
    };

    let correctas = resultados.iter().filter(|r| r.correcta).count();
    let fallidas = resultados.len() - correctas;
    let listo = !resultados.is_empty() && fallidas == 0;
    if json {
        let pruebas: Vec<String> = resultados
            .iter()
            .map(|r| {
                format!(
                    "{{\"nombre\":{},\"estado\":{},\"detalle\":{}}}",
                    json_texto(&r.nombre),
                    json_texto(if r.correcta { "ok" } else { "error" }),
                    json_texto(&r.detalle)
                )
            })
            .collect();
        println!(
            "{{\"archivo\":{},\"total\":{},\"correctas\":{},\"fallidas\":{},\"listo\":{},\"pruebas\":[{}]}}",
            json_texto(&archivo),
            resultados.len(),
            correctas,
            fallidas,
            listo,
            pruebas.join(",")
        );
    } else if resultados.is_empty() {
        println!("! Este programa no tiene pruebas.");
        println!("  Escribe una así: prueba \"mi prueba\" {{ comprobar(1 + 1 == 2) }}");
    } else {
        imprimir(&resultados);
    }
    if listo {
        Ok(())
    } else {
        Err((String::new(), 1))
    }
}

/// Muestra una línea por prueba y el resumen final.
pub fn imprimir(resultados: &[ResultadoPrueba]) {
    for resultado in resultados {
        if resultado.correcta {
            println!("✓ {}", resultado.nombre);
        } else {
            println!("✗ {} — {}", resultado.nombre, resultado.detalle);
        }
    }
    let correctas = resultados.iter().filter(|r| r.correcta).count();
    println!(
        "Pruebas: {} · correctas: {} · fallidas: {}",
        resultados.len(),
        correctas,
        resultados.len() - correctas
    );
}

/// Traduce el programa, compila su variante de pruebas y la ejecuta con una carpeta de datos
/// temporal nueva (los datos reales nunca se tocan). Si no hay pruebas devuelve una lista vacía.
/// Corre las pruebas resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn correr_archivo(
    fuente: &str,
    archivo: &Path,
) -> Result<Vec<ResultadoPrueba>, String> {
    correr_con(fuente, Some(archivo))
}

fn correr_con(
    fuente: &str,
    archivo: Option<&Path>,
) -> Result<Vec<ResultadoPrueba>, String> {
    let rust = match archivo {
        Some(archivo) => deimon::traducir_pruebas_archivo_varios(fuente, archivo),
        None => deimon::traducir_pruebas_varios(fuente),
    }
    .map_err(|errores| deimon::texto_de_errores(&errores))?;
    let informacion = match archivo {
        Some(archivo) => deimon::informacion_archivo(fuente, archivo),
        None => deimon::informacion(fuente),
    }
    .map_err(|error| error.to_string())?;
    if informacion.pruebas.is_empty() {
        return Ok(Vec::new());
    }
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let trabajo = env::temp_dir().join(format!("ln_probar_{}_{}", std::process::id(), marca));
    fs::create_dir_all(&trabajo).map_err(|error| {
        format!(
            "Error: no se pudo crear la carpeta temporal '{}': {}",
            trabajo.display(),
            error
        )
    })?;
    // Con el intérprete las pruebas empiezan al instante y no hace falta Rust; si el programa
    // usa algo que el intérprete no lleva (por ejemplo `sql`), se compila con Rust como siempre.
    let resultado = match programa_interpretable(fuente, archivo) {
        Some((ejecutable, ruta)) => {
            let mut orden = Command::new(ejecutable);
            orden.arg(ORDEN_INTERNA).arg(ruta);
            correr_orden(&trabajo, &mut orden, &informacion.pruebas)
        }
        None => correr_en(&trabajo, &rust, &informacion.pruebas),
    };
    let _ = fs::remove_dir_all(&trabajo);
    resultado
}

/// Orden oculta con la que `deimon probar` se llama a sí mismo para correr las pruebas con
/// el intérprete en un proceso aparte (como el programa de pruebas del modo Rust).
pub const ORDEN_INTERNA: &str = "__probar-con-interprete";

/// El propio `deimon` y la ruta completa del programa, si el intérprete puede con él.
fn programa_interpretable(fuente: &str, archivo: Option<&Path>) -> Option<(PathBuf, PathBuf)> {
    if env::var("DEIMON_MOTOR").as_deref() == Ok("rust") {
        return None;
    }
    let archivo = archivo?;
    deimon::compilar_para_interprete(fuente, archivo).ok()?;
    Some((env::current_exe().ok()?, fs::canonicalize(archivo).ok()?))
}

/// La orden oculta: corre las pruebas de `archivo` con el intérprete.
pub fn probar_con_interprete(argumentos: &[String]) -> Result<(), Fallo> {
    let archivo = argumentos.first().ok_or_else(|| ("Error: falta el archivo.".to_string(), 2))?;
    let fuente = fs::read_to_string(archivo)
        .map_err(|error| (format!("Error: no se pudo leer '{}': {}", archivo, error), 1))?;
    let codigo = deimon::compilar_para_interprete(&fuente, Path::new(archivo))
        .map_err(|motivo| (format!("Error: el intérprete no puede con este programa: {}", motivo), 1))?;
    deimon::interprete::ejecutar_pruebas(&codigo);
    Ok(())
}

fn en_una_linea(texto: &str) -> String {
    texto.replace(['\t', '\n', '\r'], " ")
}

fn correr_en(
    trabajo: &Path,
    rust: &str,
    nombres: &[String],
) -> Result<Vec<ResultadoPrueba>, String> {
    let ruta_rust = trabajo.join("programa.rs");
    let ruta_bin = trabajo.join("programa.bin");
    fs::write(&ruta_rust, rust).map_err(|error| {
        format!(
            "Error: no se pudo escribir el archivo temporal '{}': {}",
            ruta_rust.display(),
            error
        )
    })?;

    let compilacion = ejecutar_con_limite(
        Command::new("rustc").arg(&ruta_rust).arg("-o").arg(&ruta_bin),
        Duration::from_secs(300),
    )
    .map_err(|error| format!("Error: no se pudo ejecutar 'rustc': {}", error))?;
    if !compilacion.exito {
        let mut prefijo = String::new();
        for nombre in deimon::extensiones_del_rust(rust) {
            prefijo.push_str(&format!(
                "El módulo '{}' tiene un error en su código Rust:\n",
                nombre
            ));
        }
        return Err(format!(
            "Error: no se pudo compilar el programa de pruebas:\n{}{}",
            prefijo,
            compilacion.stderr.trim()
        ));
    }

    correr_orden(trabajo, &mut Command::new(&ruta_bin), nombres)
}

/// Ejecuta el programa de pruebas (el compilado con Rust o el intérprete) en la carpeta de
/// trabajo, con datos nuevos, y recoge el resultado de cada prueba.
fn correr_orden(
    trabajo: &Path,
    orden: &mut Command,
    nombres: &[String],
) -> Result<Vec<ResultadoPrueba>, String> {
    let datos: PathBuf = trabajo.join("datos");
    let ejecucion = ejecutar_con_limite(
        orden
            .current_dir(trabajo)
            .env("DEIMON_DATOS", &datos)
            .env_remove("DEIMON_PUERTO"),
        Duration::from_secs(120),
    );
    let (salida, aviso) = match ejecucion {
        Ok(salida) => {
            let aviso = if salida.exito {
                String::new()
            } else {
                match salida.codigo {
                    Some(codigo) => format!("el programa terminó con el código {}", codigo),
                    None => "el programa se cerró de golpe".to_string(),
                }
            };
            (salida.stdout, aviso)
        }
        Err(error) => (String::new(), format!("el programa {}", error)),
    };

    let mut informados: Vec<(String, bool, String)> = Vec::new();
    for linea in salida.lines() {
        if let Some(resto) = linea.strip_prefix(MARCA) {
            let mut partes = resto.splitn(3, '\t');
            let estado = partes.next().unwrap_or("");
            let nombre = partes.next().unwrap_or("").to_string();
            let detalle = partes.next().unwrap_or("").to_string();
            informados.push((nombre, estado == "ok", detalle));
        }
    }
    let resultados = nombres
        .iter()
        .map(|nombre| {
            let buscado = en_una_linea(nombre);
            match informados.iter().find(|(n, _, _)| *n == buscado) {
                Some((_, correcta, detalle)) => ResultadoPrueba {
                    nombre: nombre.clone(),
                    correcta: *correcta,
                    detalle: detalle.clone(),
                },
                None => ResultadoPrueba {
                    nombre: nombre.clone(),
                    correcta: false,
                    detalle: if aviso.is_empty() {
                        "no se llegó a ejecutar esta prueba".to_string()
                    } else {
                        format!("no se llegó a terminar: {}", aviso)
                    },
                },
            }
        })
        .collect();
    Ok(resultados)
}
