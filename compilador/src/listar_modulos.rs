//! `deimon modulos [archivo.dmn] [--json]`: lista los módulos que se pueden usar con
//! `usar modulo "nombre"` y las funciones de cada uno.

use std::path::Path;

use deimon::modulos::{fichas_en, Ficha};

type Fallo = (String, i32);

pub fn listar(argumentos: &[String]) -> Result<(), Fallo> {
    let mut json = false;
    let mut archivo = None;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
            otro if otro.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe. Uso: deimon modulos [archivo.dmn] [--json]", otro), 2))
            }
            otro if archivo.is_none() => archivo = Some(otro.to_string()),
            otro => return Err((format!("Error: sobra el argumento '{}'. Uso: deimon modulos [archivo.dmn] [--json]", otro), 2)),
        }
    }

    let mut fichas = Vec::new();
    if let Some(archivo) = &archivo {
        let carpeta = Path::new(archivo)
            .parent()
            .map(|padre| if padre.as_os_str().is_empty() { Path::new(".") } else { padre })
            .unwrap_or(Path::new("."))
            .join("modulos");
        fichas.extend(fichas_en(&carpeta, "junto al programa"));
    }
    fichas.extend(fichas_en(&deimon::modulos::carpeta_personal(), "~/.deimon/modulos"));
    if let Some(carpeta) = deimon::biblioteca::carpeta() {
        fichas.extend(fichas_en(&carpeta, "biblioteca estándar"));
    }

    if json {
        println!("{}", a_json(&fichas));
        return Ok(());
    }
    println!("Módulos disponibles (se usan con: usar modulo \"nombre\")");
    for ficha in &fichas {
        let requiere = if ficha.requiere.is_empty() {
            "nada".to_string()
        } else {
            ficha.requiere.join(", ")
        };
        println!();
        println!("■ {} ({}): {}", ficha.nombre, ficha.origen, ficha.descripcion);
        println!("  necesita: {}", requiere);
        for (firma, descripcion) in &ficha.funciones {
            println!("  - {}: {}", firma, descripcion);
        }
    }
    Ok(())
}

fn a_json(fichas: &[Ficha]) -> String {
    let texto = deimon::json::texto;
    let modulos: Vec<String> = fichas
        .iter()
        .map(|ficha| {
            let requiere: Vec<String> = ficha.requiere.iter().map(|r| texto(r)).collect();
            let funciones: Vec<String> = ficha
                .funciones
                .iter()
                .map(|(firma, descripcion)| {
                    format!("{{\"firma\":{},\"descripcion\":{}}}", texto(firma), texto(descripcion))
                })
                .collect();
            format!(
                "{{\"nombre\":{},\"descripcion\":{},\"origen\":{},\"requiere\":[{}],\"funciones\":[{}]}}",
                texto(&ficha.nombre),
                texto(&ficha.descripcion),
                texto(&ficha.origen),
                requiere.join(","),
                funciones.join(",")
            )
        })
        .collect();
    format!("{{\"modulos\":[{}]}}", modulos.join(","))
}
