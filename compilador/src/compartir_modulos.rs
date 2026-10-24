//! `deimon modulo instalar|actualizar|quitar`: comparte módulos de la comunidad.
//! Usa la orden `git` del sistema; no añade dependencias.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Fallo = (String, i32);

const USO: &str = "Uso: deimon modulo instalar <https://... | carpeta> [--aqui] | deimon modulo actualizar <nombre> [--aqui] | deimon modulo quitar <nombre> [--aqui]";

pub fn modulo(argumentos: &[String]) -> Result<(), Fallo> {
    let orden = argumentos
        .first()
        .map(String::as_str)
        .ok_or_else(|| (USO.to_string(), 2))?;
    let mut aqui = false;
    let mut valor = None;
    for argumento in &argumentos[1..] {
        match argumento.as_str() {
            "--aqui" => aqui = true,
            otro if otro.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otro, USO), 2))
            }
            otro if valor.is_none() => valor = Some(otro.to_string()),
            otro => return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, USO), 2)),
        }
    }
    let base = if aqui {
        PathBuf::from("modulos")
    } else {
        deimon::modulos::carpeta_personal()
    };
    match orden {
        "instalar" | "actualizar" | "quitar" => {
            let valor = valor.ok_or_else(|| (format!("Error: falta un argumento.\n{}", USO), 2))?;
            match orden {
                "instalar" => instalar(&valor, &base),
                "actualizar" => actualizar(&valor, &base),
                _ => quitar(&valor, &base),
            }
        }
        otra => Err((format!("Error: la orden 'modulo {}' no existe.\n{}", otra, USO), 2)),
    }
}

fn fallo(mensaje: impl Into<String>) -> Fallo {
    (mensaje.into(), 1)
}

fn nombre_valido(nombre: &str) -> bool {
    !nombre.is_empty() && nombre.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

fn comprobar_nombre(nombre: &str) -> Result<(), Fallo> {
    if nombre_valido(nombre) {
        Ok(())
    } else {
        Err(fallo(format!(
            "Error: el nombre '{}' no vale: usa solo letras minúsculas sin tilde, números y _.",
            nombre
        )))
    }
}

fn git(argumentos: &[&std::ffi::OsStr], protocolo: &str) -> Result<(), Fallo> {
    let salida = Command::new("git")
        .args(argumentos)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", protocolo)
        .output()
        .map_err(|error| {
            fallo(format!(
                "Error: no se pudo ejecutar 'git' (¿está instalado?): {}",
                error
            ))
        })?;
    if salida.status.success() {
        Ok(())
    } else {
        Err(fallo(format!(
            "Error: git ha fallado: {}",
            String::from_utf8_lossy(&salida.stderr).trim()
        )))
    }
}

fn instalar(origen: &str, base: &Path) -> Result<(), Fallo> {
    if origen.starts_with('-') {
        return Err(fallo("Error: el origen no puede empezar por '-'."));
    }
    if origen.to_lowercase().starts_with("http://") {
        return Err(fallo(
            "Error: el origen usa http:// y no es seguro. Usa una dirección https://.",
        ));
    }
    let es_web = origen.to_lowercase().starts_with("https://");
    let local = if es_web {
        None
    } else {
        let ruta = Path::new(origen);
        if !ruta.is_dir() {
            return Err(fallo(format!(
                "Error: el origen '{}' no es una dirección https:// ni una carpeta que exista.",
                origen
            )));
        }
        Some(fs::canonicalize(ruta).map_err(|e| fallo(format!("Error: no se pudo leer '{}': {}", origen, e)))?)
    };

    fs::create_dir_all(base).map_err(|e| {
        fallo(format!("Error: no se pudo crear la carpeta '{}': {}", base.display(), e))
    })?;
    let temporal = base.join(format!(".instalando-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temporal);
    fs::create_dir_all(&temporal).map_err(|e| {
        fallo(format!("Error: no se pudo crear la carpeta temporal '{}': {}", temporal.display(), e))
    })?;
    let resultado = instalar_en(origen, es_web, local.as_deref(), base, &temporal);
    let _ = fs::remove_dir_all(&temporal);
    resultado
}

fn instalar_en(
    origen: &str,
    es_web: bool,
    local: Option<&Path>,
    base: &Path,
    temporal: &Path,
) -> Result<(), Fallo> {
    let clon = temporal.join("clon");
    match local {
        Some(carpeta) if !carpeta.join(".git").exists() => copiar(carpeta, &clon)?,
        _ => {
            let origen_git = local.map(Path::as_os_str).unwrap_or(origen.as_ref());
            let protocolo = if es_web { "https" } else { "file" };
            git(
                &[
                    "clone".as_ref(),
                    "--depth".as_ref(),
                    "1".as_ref(),
                    "--".as_ref(),
                    origen_git,
                    clon.as_os_str(),
                ],
                protocolo,
            )?;
        }
    }
    let nombre = deimon::modulos::validar_carpeta(&clon)
        .map_err(|mensaje| fallo(format!("Error: el módulo no es válido: {}", mensaje)))?;
    if deimon::biblioteca::nombres().contains(&nombre.as_str()) {
        return Err(fallo(format!(
            "Error: '{}' es un módulo de la biblioteca estándar; el módulo que instalas tiene que llamarse de otra forma.",
            nombre
        )));
    }
    let destino = base.join(&nombre);
    if destino.exists() {
        return Err(fallo(format!(
            "Error: el módulo '{}' ya está instalado en '{}'. Para ponerlo al día usa: deimon modulo actualizar {}",
            nombre,
            destino.display(),
            nombre
        )));
    }
    fs::rename(&clon, &destino).map_err(|e| {
        fallo(format!("Error: no se pudo mover el módulo a '{}': {}", destino.display(), e))
    })?;
    println!("Módulo '{}' instalado en {}", nombre, destino.display());
    println!("Aviso: el módulo trae código Rust que se ejecuta en tu equipo; instala solo módulos de quien confíes.");
    Ok(())
}

/// Copia una carpeta (sin enlaces simbólicos) para orígenes locales que no son repositorios.
fn copiar(origen: &Path, destino: &Path) -> Result<(), Fallo> {
    fs::create_dir_all(destino)
        .map_err(|e| fallo(format!("Error: no se pudo crear '{}': {}", destino.display(), e)))?;
    let entradas = fs::read_dir(origen)
        .map_err(|e| fallo(format!("Error: no se pudo leer '{}': {}", origen.display(), e)))?;
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        let tipo = entrada
            .file_type()
            .map_err(|e| fallo(format!("Error: no se pudo leer '{}': {}", ruta.display(), e)))?;
        let hasta = destino.join(entrada.file_name());
        if tipo.is_dir() {
            copiar(&ruta, &hasta)?;
        } else if tipo.is_file() {
            fs::copy(&ruta, &hasta)
                .map_err(|e| fallo(format!("Error: no se pudo copiar '{}': {}", ruta.display(), e)))?;
        } else {
            return Err(fallo(format!(
                "Error: '{}' es un enlace simbólico y no se copia.",
                ruta.display()
            )));
        }
    }
    Ok(())
}

fn carpeta_de(nombre: &str, base: &Path) -> Result<PathBuf, Fallo> {
    comprobar_nombre(nombre)?;
    let carpeta = base.join(nombre);
    let es_carpeta = fs::symlink_metadata(&carpeta)
        .map(|datos| datos.file_type().is_dir())
        .unwrap_or(false);
    if es_carpeta {
        Ok(carpeta)
    } else {
        Err(fallo(format!(
            "Error: el módulo '{}' no está instalado en '{}'.",
            nombre,
            base.display()
        )))
    }
}

fn actualizar(nombre: &str, base: &Path) -> Result<(), Fallo> {
    let carpeta = carpeta_de(nombre, base)?;
    if !carpeta.join(".git").exists() {
        return Err(fallo(format!(
            "Error: el módulo '{}' no se instaló con git y no se puede actualizar. Quítalo e instálalo otra vez.",
            nombre
        )));
    }
    git(
        &["-C".as_ref(), carpeta.as_os_str(), "pull".as_ref(), "--ff-only".as_ref()],
        "https:file",
    )?;
    match deimon::modulos::validar_carpeta(&carpeta) {
        Ok(declarado) if declarado == nombre => {}
        Ok(declarado) => {
            return Err(fallo(format!(
                "Error: tras actualizar, modulo.txt dice 'nombre: {}' pero la carpeta se llama '{}'.",
                declarado, nombre
            )))
        }
        Err(mensaje) => {
            return Err(fallo(format!(
                "Error: tras actualizar, el módulo no es válido: {}",
                mensaje
            )))
        }
    }
    println!("Módulo '{}' actualizado.", nombre);
    println!("Aviso: el módulo trae código Rust que se ejecuta en tu equipo; instala solo módulos de quien confíes.");
    Ok(())
}

fn quitar(nombre: &str, base: &Path) -> Result<(), Fallo> {
    let carpeta = carpeta_de(nombre, base)?;
    fs::remove_dir_all(&carpeta)
        .map_err(|e| fallo(format!("Error: no se pudo borrar '{}': {}", carpeta.display(), e)))?;
    println!("Módulo '{}' quitado.", nombre);
    Ok(())
}
