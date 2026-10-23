use std::fs;
use std::path::Path;

use deimon::error::ErrorTraduccion;

use crate::utilidades::json_texto;

type Fallo = (String, i32);

/// `deimon comprobar archivo.dmn [--json] [--arreglar]`
pub fn comprobar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon comprobar archivo.dmn [--json] [--arreglar]";
    let mut archivo = None;
    let mut json = false;
    let mut arreglar = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
            "--arreglar" => arreglar = true,
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
    let errores = if arreglar {
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
    let _ = &fuente;

    if json {
        println!("{}", a_json(&archivo, &errores, arreglar.then_some(aplicados)));
    } else {
        if arreglar {
            println!(
                "Arreglos automáticos aplicados: {}",
                aplicados
            );
        }
        if errores.is_empty() {
            println!("✓ {} no tiene errores", archivo);
        } else {
            for error in &errores {
                println!("{}\n", error);
            }
            println!(
                "✗ {} tiene {} {}",
                archivo,
                errores.len(),
                if errores.len() == 1 { "error" } else { "errores" }
            );
        }
    }
    if errores.is_empty() {
        Ok(())
    } else {
        Err((String::new(), 1))
    }
}

fn a_json(archivo: &str, errores: &[ErrorTraduccion], aplicados: Option<usize>) -> String {
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
            format!(
                "{{\"codigo\":{},\"linea\":{},\"columna\":{},\"mensaje\":{},\"sugerencia\":{},\"arreglo\":{}}}",
                json_texto(e.codigo),
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
