//! `deimon desde-cobol archivo.cob [--salida archivo.dmn] [--comprobar]`: traduce COBOL a Deimon
//! y, con --comprobar, compila el COBOL con cobc, ejecuta los dos y compara salida y código de salida.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Fallo = (String, i32);

const USO: &str = "Uso: deimon desde-cobol archivo.cob [--salida archivo.dmn] [--comprobar]";

pub fn desde_cobol(argumentos: &[String]) -> Result<(), Fallo> {
    let mut archivo = None;
    let mut salida = None;
    let mut comprobar = false;
    let mut resto = argumentos.iter();
    while let Some(argumento) = resto.next() {
        match argumento.as_str() {
            "--comprobar" => comprobar = true,
            "--salida" => match resto.next() {
                Some(ruta) => salida = Some(PathBuf::from(ruta)),
                None => return Err((format!("Error: falta la ruta después de --salida. {}", USO), 2)),
            },
            otro if otro.starts_with("--") => return Err((format!("Error: la opción '{}' no existe. {}", otro, USO), 2)),
            otro if archivo.is_none() => archivo = Some(PathBuf::from(otro)),
            otro => return Err((format!("Error: sobra el argumento '{}'. {}", otro, USO), 2)),
        }
    }
    let archivo = archivo.ok_or_else(|| (USO.to_string(), 2))?;
    let cobol = fs::read(&archivo)
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .map_err(|error| (format!("Error: no se pudo leer '{}': {}", archivo.display(), error), 1))?;
    let nombre = archivo.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let traduccion = deimon::desde_cobol::traducir(&cobol, &nombre)
        .map_err(|error| (format!("Error: el COBOL no se pudo traducir. No se ha creado ningún archivo.\n{}", error), 1))?;
    let salida = salida.unwrap_or_else(|| archivo.with_extension("dmn"));
    fs::write(&salida, &traduccion.codigo)
        .map_err(|error| (format!("Error: no se pudo escribir '{}': {}", salida.display(), error), 1))?;
    let _ = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon")))
        .arg("formatear")
        .arg(&salida)
        .output();
    println!("✓ Traducido: {} → {}", archivo.display(), salida.display());
    for (linea, aviso) in &traduccion.avisos {
        if *linea > 0 {
            println!("  aviso, línea {}: {}", linea, aviso);
        } else {
            println!("  aviso: {}", aviso);
        }
    }
    if comprobar {
        return comparar(&archivo, &salida, deimon::desde_cobol::es_formato_libre(&cobol));
    }
    Ok(())
}

/// Compila el COBOL con cobc, ejecuta los dos programas y compara lo que escriben y su código de salida.
fn comparar(cobol: &Path, dmn: &Path, libre: bool) -> Result<(), Fallo> {
    let carpeta = std::env::temp_dir().join(format!("deimon-cobol-{}", std::process::id()));
    fs::create_dir_all(&carpeta).map_err(|e| (format!("Error: no se pudo crear una carpeta temporal: {}", e), 1))?;
    let resultado = comparar_en(cobol, dmn, libre, &carpeta);
    let _ = fs::remove_dir_all(&carpeta);
    resultado
}

fn comparar_en(cobol: &Path, dmn: &Path, libre: bool, carpeta: &Path) -> Result<(), Fallo> {
    let binario = carpeta.join("programa-cobol");
    let mut compilar = Command::new("cobc");
    compilar.arg("-x");
    if libre {
        compilar.arg("-free");
    }
    let compilado = compilar
        .arg("-o")
        .arg(&binario)
        .arg(cobol)
        .output()
        .map_err(|_| ("Error: para comprobar hace falta GnuCOBOL instalado (orden 'cobc').".to_string(), 1))?;
    if !compilado.status.success() {
        return Err((format!("✗ cobc no pudo compilar el COBOL original:\n{}", String::from_utf8_lossy(&compilado.stderr).trim()), 1));
    }
    let original = Command::new(&binario)
        .output()
        .map_err(|error| (format!("Error: no se pudo ejecutar el COBOL compilado: {}", error), 1))?;
    let traducido = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon")))
        .arg("ejecutar")
        .arg(dmn)
        .output()
        .map_err(|error| (format!("Error: no se pudo ejecutar el programa traducido: {}", error), 1))?;
    let de_cobol = String::from_utf8_lossy(&original.stdout);
    let de_deimon = String::from_utf8_lossy(&traducido.stdout);
    let codigo_cobol = original.status.code().unwrap_or(-1);
    let codigo_deimon = traducido.status.code().unwrap_or(-1);
    if de_cobol == de_deimon && codigo_cobol == codigo_deimon {
        println!(
            "✓ Comprobado: COBOL y Deimon escriben exactamente lo mismo ({} líneas) y terminan con el mismo código ({}).",
            de_cobol.lines().count(),
            codigo_cobol
        );
        return Ok(());
    }
    let mut mensaje = String::new();
    if de_cobol != de_deimon {
        mensaje.push_str("✗ La salida es distinta. Primera diferencia:\n");
        let lineas_cobol: Vec<&str> = de_cobol.lines().collect();
        let lineas_deimon: Vec<&str> = de_deimon.lines().collect();
        let mut encontrada = false;
        for i in 0..lineas_cobol.len().max(lineas_deimon.len()) {
            let a = lineas_cobol.get(i).copied().unwrap_or("(nada)");
            let b = lineas_deimon.get(i).copied().unwrap_or("(nada)");
            if a != b {
                mensaje.push_str(&format!("  línea {} de la salida\n  COBOL:  {}\n  Deimon: {}\n", i + 1, a, b));
                encontrada = true;
                break;
            }
        }
        if !encontrada {
            mensaje.push_str("  (solo cambia el salto de línea final)\n");
        }
    }
    if codigo_cobol != codigo_deimon {
        mensaje.push_str(&format!("✗ El código de salida es distinto: COBOL {} y Deimon {}.", codigo_cobol, codigo_deimon));
        let errores = String::from_utf8_lossy(&traducido.stderr);
        if !errores.trim().is_empty() {
            mensaje.push_str(&format!("\n  Error del programa traducido: {}", errores.trim()));
        }
    }
    Err((mensaje.trim_end().to_string(), 1))
}
