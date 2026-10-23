fn ext_archivos_ruta(args: &[Valor], funcion: &str) -> Result<String, String> {
    match args.first().and_then(|valor| valor.como_texto()) {
        Some(ruta) if !ruta.is_empty() => Ok(ruta.to_string()),
        _ => Err(format!("{}() espera la ruta del archivo como texto", funcion)),
    }
}

fn ext_escribir_archivo(args: Vec<Valor>) -> Result<Valor, String> {
    let ruta = ext_archivos_ruta(&args, "escribir_archivo")?;
    let texto = args.get(1).map(texto_de).unwrap_or_default();
    std::fs::write(&ruta, texto).map_err(|e| format!("escribir_archivo(): no se pudo escribir '{}': {}", ruta, e))?;
    Ok(Valor::Nada)
}

fn ext_leer_archivo(args: Vec<Valor>) -> Result<Valor, String> {
    let ruta = ext_archivos_ruta(&args, "leer_archivo")?;
    match std::fs::read_to_string(&ruta) {
        Ok(texto) => Ok(Valor::texto(texto)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Valor::Nada),
        Err(e) => Err(format!("leer_archivo(): no se pudo leer '{}': {}", ruta, e)),
    }
}

fn ext_existe_archivo(args: Vec<Valor>) -> Result<Valor, String> {
    let ruta = ext_archivos_ruta(&args, "existe_archivo")?;
    Ok(Valor::Logico(std::path::Path::new(&ruta).exists()))
}

fn ext_borrar_archivo(args: Vec<Valor>) -> Result<Valor, String> {
    let ruta = ext_archivos_ruta(&args, "borrar_archivo")?;
    match std::fs::remove_file(&ruta) {
        Ok(_) => Ok(Valor::Nada),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Valor::Nada),
        Err(e) => Err(format!("borrar_archivo(): no se pudo borrar '{}': {}", ruta, e)),
    }
}

fn ext_listar_archivos(args: Vec<Valor>) -> Result<Valor, String> {
    let ruta = ext_archivos_ruta(&args, "listar_archivos")?;
    let mut nombres: Vec<String> = std::fs::read_dir(&ruta)
        .map_err(|e| format!("listar_archivos(): no se pudo leer la carpeta '{}': {}", ruta, e))?
        .filter_map(|entrada| entrada.ok().map(|e| e.file_name().to_string_lossy().to_string()))
        .collect();
    nombres.sort();
    Ok(Valor::lista(nombres.into_iter().map(Valor::texto).collect()))
}
