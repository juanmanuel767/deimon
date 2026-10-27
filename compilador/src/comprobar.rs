use std::fs;
use std::path::Path;

use deimon::error::ErrorTraduccion;

use crate::utilidades::json_texto;

type Fallo = (String, i32);

/// `deimon comprobar archivo.dmn [--json] [--arreglar] [--estricto]`
pub fn comprobar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon comprobar archivo.dmn [--json] [--arreglar] [--estricto]";
    let mut archivo = None;
    let mut json = false;
    let mut arreglar = false;
    let mut estricto = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
            "--arreglar" => arreglar = true,
            "--estricto" => estricto = true,
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
    let mut fuente = fs::read_to_string(&archivo).map_err(|error| {
        (
            format!("Error: no se pudo leer el archivo '{}': {}", archivo, error),
            1,
        )
    })?;

    let mut aplicados = 0;
    let mut errores = if arreglar {
        let (nuevo, total, errores) =
            deimon::arreglos::arreglar_en(&fuente, Some(Path::new(&archivo)));
        if total > 0 {
            fs::write(&archivo, &nuevo).map_err(|error| {
                (
                    format!("Error: no se pudo escribir el archivo '{}': {}", archivo, error),
                    1,
                )
            })?;
            fuente = nuevo;
        }
        aplicados = total;
        errores
    } else {
        deimon::comprobar_archivo(&fuente, Path::new(&archivo))
    };
    // Con --estricto, tras los errores van los avisos (A001...).
    let avisos = if estricto {
        deimon::avisos_archivo(&fuente, Path::new(&archivo))
    } else {
        Vec::new()
    };
    let total_avisos = avisos.len();
    errores.extend(avisos);

    if json {
        println!(
            "{}",
            a_json_con_nivel(&archivo, &errores, arreglar.then_some(aplicados), estricto)
        );
    } else {
        if arreglar {
            println!(
                "Arreglos automáticos aplicados: {}",
                aplicados
            );
        }
        if errores.is_empty() {
            if estricto {
                println!("✓ {} no tiene errores ni avisos", archivo);
            } else {
                println!("✓ {} no tiene errores", archivo);
            }
        } else {
            for error in &errores {
                println!("{}\n", error);
            }
            let total_errores = errores.len() - total_avisos;
            let mut partes = Vec::new();
            if total_errores > 0 {
                partes.push(format!(
                    "{} {}",
                    total_errores,
                    if total_errores == 1 { "error" } else { "errores" }
                ));
            }
            if total_avisos > 0 {
                partes.push(format!(
                    "{} {}",
                    total_avisos,
                    if total_avisos == 1 { "aviso" } else { "avisos" }
                ));
            }
            println!("✗ {} tiene {}", archivo, partes.join(" y "));
        }
    }
    if errores.is_empty() {
        Ok(())
    } else {
        Err((String::new(), 1))
    }
}

pub fn a_json(archivo: &str, errores: &[ErrorTraduccion], aplicados: Option<usize>) -> String {
    a_json_con_nivel(archivo, errores, aplicados, false)
}

/// Como `a_json`; con `con_nivel` (modo `--estricto`) cada entrada lleva `"nivel"`:
/// `"error"` o `"aviso"` (los avisos son los de código A...).
fn a_json_con_nivel(
    archivo: &str,
    errores: &[ErrorTraduccion],
    aplicados: Option<usize>,
    con_nivel: bool,
) -> String {
    let lista: Vec<String> = errores
        .iter()
        .map(|e| {
            let sugerencia = match &e.sugerencia {
                Some(texto) => json_texto(texto),
                None => "null".to_string(),
            };
            let arreglo = match &e.arreglo {
                Some(a) => format!(
                    "{{\"linea\":{},\"columna_inicio\":{},\"columna_fin\":{},\"reemplazo\":{}}}",
                    a.linea,
                    a.columna_inicio,
                    a.columna_fin,
                    json_texto(&a.reemplazo)
                ),
                None => "null".to_string(),
            };
            let nivel = if !con_nivel {
                String::new()
            } else if e.es_aviso() {
                "\"nivel\":\"aviso\",".to_string()
            } else {
                "\"nivel\":\"error\",".to_string()
            };
            format!(
                "{{\"codigo\":{},{}\"linea\":{},\"columna\":{},\"mensaje\":{},\"sugerencia\":{},\"arreglo\":{}}}",
                json_texto(e.codigo),
                nivel,
                e.linea,
                e.columna,
                json_texto(&e.mensaje),
                sugerencia,
                arreglo
            )
        })
        .collect();
    let extra = match aplicados {
        Some(n) => format!(",\"arreglos_aplicados\":{}", n),
        None => String::new(),
    };
    format!(
        "{{\"archivo\":{},\"correcto\":{},\"errores\":[{}]{}}}",
        json_texto(archivo),
        errores.is_empty(),
        lista.join(","),
        extra
    )
}

/// `deimon guia [archivo.dmn] [--json]`
pub fn guia(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon guia [archivo.dmn] [--json]";
    let mut json = false;
    let mut archivo = None;
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

    let extensiones = match &archivo {
        Some(archivo) => {
            let fuente = fs::read_to_string(archivo).map_err(|error| {
                (
                    format!("Error: no se pudo leer el archivo '{}': {}", archivo, error),
                    1,
                )
            })?;
            deimon::extensiones_usadas_archivo(&fuente, Path::new(archivo))
                .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?
        }
        None => Vec::new(),
    };

    if json {
        println!("{}", deimon::guia::json_con_extensiones(&extensiones));
    } else {
        print!("{}", deimon::guia::texto_con_extensiones(&extensiones));
    }
    Ok(())
}
