//! `deimon empaquetar archivo.dmn [--salida RUTA] [--perfil NOMBRE]`: crea un ejecutable que lleva dentro el
//! propio `deimon` (con su intérprete) y el programa, con todos los archivos que usa. No hace
//! falta Rust ni para crearlo ni para abrirlo; se reparte como un solo archivo.
//!
//! Formato: el ejecutable de `deimon`, seguido de los archivos del programa y de una cola de
//! 24 bytes (tamaño de los archivos en 8 bytes y la marca `MARCA`). Al arrancar, `deimon`
//! mira su propio final: si lleva un programa, lo ejecuta en vez de atender órdenes.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

type Fallo = (String, i32);

const MARCA: &[u8; 16] = b"~DEIMON~PAQUETE~";
const CABECERA: &[u8] = b"DEIMON-PAQUETE-1\n";
/// Ningún programa necesita tanto; protege de un archivo roto que dijera otra cosa.
const MAXIMO: u64 = 512 * 1024 * 1024;
/// Archivo del paquete donde se guarda el perfil elegido (no es un archivo del programa).
const ARCHIVO_PERFIL: &str = "/__deimon__/perfil";

/// `deimon empaquetar archivo.dmn [--salida RUTA] [--perfil NOMBRE]`
pub fn empaquetar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon empaquetar archivo.dmn [--salida RUTA] [--perfil pequeño|equilibrado|servidor]";
    let mut archivo = None;
    let mut salida = None;
    let mut perfil = deimon::perfil::Perfil::Equilibrado;
    let mut i = 0;
    while i < argumentos.len() {
        match argumentos[i].as_str() {
            "--salida" => {
                i += 1;
                salida = Some(
                    argumentos
                        .get(i)
                        .ok_or_else(|| (format!("Error: falta el valor de '--salida'.\n{}", uso), 2))?
                        .clone(),
                );
            }
            "--perfil" => {
                i += 1;
                let valor = argumentos
                    .get(i)
                    .ok_or_else(|| (format!("Error: falta el valor de '--perfil'.\n{}", uso), 2))?;
                perfil = deimon::perfil::Perfil::desde_texto(valor).ok_or_else(|| deimon::perfil::error_desconocido(valor))?;
            }
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => {
                if archivo.is_some() {
                    return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2));
                }
                archivo = Some(PathBuf::from(otro));
            }
        }
        i += 1;
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;
    let fuente = fs::read_to_string(&archivo)
        .map_err(|error| (format!("Error: no se pudo leer '{}': {}", archivo.display(), error), 1))?;

    // Los mismos errores que `deimon ejecutar`.
    deimon::traducir_archivo_varios_con_perfil(&fuente, &archivo, perfil)
        .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;

    // Se prepara para el intérprete apuntando cada archivo que lee.
    deimon::fuentes::empezar_a_apuntar();
    let preparado = deimon::compilar_para_interprete(&fuente, &archivo);
    let apuntados = deimon::fuentes::apuntados();
    if let Err(motivo) = preparado {
        return Err((
            format!(
                "Error: este programa no se puede empaquetar sin Rust porque {}.\nSugerencia: usa 'deimon compilar {}', que lo convierte en un programa con Rust.",
                motivo,
                archivo.display()
            ),
            1,
        ));
    }

    // Como `deimon compilar`: antes se ejecutan las pruebas del programa.
    let pruebas = crate::probar::correr_archivo(&fuente, &archivo).map_err(|mensaje| (mensaje, 1))?;
    if pruebas.is_empty() {
        println!("! Aviso: este programa no tiene pruebas.");
    } else {
        crate::probar::imprimir(&pruebas);
        let falladas = pruebas.iter().filter(|p| !p.correcta).count();
        if falladas > 0 {
            return Err((
                format!(
                    "No se empaqueta el programa porque {} {} fallando. Arregla lo que falla y vuelve a intentarlo.",
                    falladas,
                    if falladas == 1 { "prueba está" } else { "pruebas están" }
                ),
                1,
            ));
        }
    }

    let principal = fs::canonicalize(&archivo)
        .map_err(|error| (format!("Error: no se pudo leer '{}': {}", archivo.display(), error), 1))?;
    let mut archivos = deimon::fuentes::para_el_paquete(
        apuntados,
        deimon::biblioteca::carpeta().as_deref(),
        &deimon::modulos::carpeta_personal(),
    );
    let cantidad = archivos.len() + 1;
    archivos.insert(principal.clone(), fuente.into_bytes());
    archivos.insert(PathBuf::from(ARCHIVO_PERFIL), perfil.nombre().as_bytes().to_vec());

    let ejecutable = std::env::current_exe()
        .map_err(|error| (format!("Error: no se encontró el propio deimon: {}", error), 1))?;
    let mut contenido = fs::read(&ejecutable)
        .map_err(|error| (format!("Error: no se pudo leer '{}': {}", ejecutable.display(), error), 1))?;
    contenido.extend(codificar(&principal, &archivos));

    let salida = match salida {
        Some(ruta) => PathBuf::from(ruta),
        None => {
            let nombre = archivo
                .file_stem()
                .map(|nombre| nombre.to_string_lossy().to_string())
                .unwrap_or_else(|| "programa".to_string());
            archivo.with_file_name(format!("{}{}", nombre, std::env::consts::EXE_SUFFIX))
        }
    };
    if fs::canonicalize(&salida).ok() == Some(principal.clone()) {
        return Err(("Error: la salida no puede ser el propio archivo .dmn.".to_string(), 1));
    }
    if let Some(padre) = salida.parent().filter(|padre| !padre.as_os_str().is_empty()) {
        fs::create_dir_all(padre)
            .map_err(|error| (format!("Error: no se pudo crear la carpeta '{}': {}", padre.display(), error), 1))?;
    }
    fs::write(&salida, &contenido)
        .map_err(|error| (format!("Error: no se pudo escribir '{}': {}", salida.display(), error), 1))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&salida, fs::Permissions::from_mode(0o755));
    }
    println!(
        "✓ Empaquetado: {} ({} archivo{} del programa, {:.1} MB). Se abre sin instalar nada: {}",
        salida.display(),
        cantidad,
        if cantidad == 1 { "" } else { "s" },
        contenido.len() as f64 / 1_048_576.0,
        if salida.is_absolute() || salida.starts_with(".") {
            salida.display().to_string()
        } else {
            format!("./{}", salida.display())
        }
    );
    Ok(())
}

fn codificar(principal: &Path, archivos: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<u8> {
    let mut datos = CABECERA.to_vec();
    // El principal va primero.
    let orden = std::iter::once(principal.to_path_buf())
        .chain(archivos.keys().filter(|ruta| ruta.as_path() != principal).cloned());
    for ruta in orden {
        let nombre = ruta.to_string_lossy().into_owned().into_bytes();
        let contenido = &archivos[&ruta];
        datos.extend((nombre.len() as u32).to_le_bytes());
        datos.extend(&nombre);
        datos.extend((contenido.len() as u64).to_le_bytes());
        datos.extend(contenido);
    }
    let tamano = datos.len() as u64;
    datos.extend(tamano.to_le_bytes());
    datos.extend(MARCA);
    datos
}

/// Si este ejecutable lleva un programa dentro, devuelve el principal y sus archivos.
fn leer_paquete() -> Option<(PathBuf, BTreeMap<PathBuf, Vec<u8>>)> {
    let mut archivo = fs::File::open(std::env::current_exe().ok()?).ok()?;
    let largo = archivo.metadata().ok()?.len();
    if largo < 24 {
        return None;
    }
    archivo.seek(SeekFrom::Start(largo - 24)).ok()?;
    let mut cola = [0u8; 24];
    archivo.read_exact(&mut cola).ok()?;
    if &cola[8..] != MARCA {
        return None;
    }
    let tamano = u64::from_le_bytes(cola[..8].try_into().ok()?);
    if tamano > MAXIMO || tamano + 24 > largo {
        return None;
    }
    archivo.seek(SeekFrom::Start(largo - 24 - tamano)).ok()?;
    let mut datos = vec![0u8; tamano as usize];
    archivo.read_exact(&mut datos).ok()?;
    let mut resto = datos.strip_prefix(CABECERA)?;
    let mut tomar = |cuantos: usize| -> Option<&[u8]> {
        if cuantos > resto.len() {
            return None;
        }
        let (trozo, siguiente) = resto.split_at(cuantos);
        resto = siguiente;
        Some(trozo)
    };
    let mut archivos = BTreeMap::new();
    let mut principal = None;
    loop {
        let Some(largo_nombre) = tomar(4) else { break };
        let largo_nombre = u32::from_le_bytes(largo_nombre.try_into().ok()?) as usize;
        let nombre = PathBuf::from(String::from_utf8(tomar(largo_nombre)?.to_vec()).ok()?);
        let largo_contenido = u64::from_le_bytes(tomar(8)?.try_into().ok()?) as usize;
        let contenido = tomar(largo_contenido)?.to_vec();
        principal.get_or_insert_with(|| nombre.clone());
        archivos.insert(nombre, contenido);
    }
    Some((principal?, archivos))
}

/// Al arrancar: si este ejecutable es un programa empaquetado, lo ejecuta y termina.
pub fn ejecutar_si_es_paquete() {
    let Some((principal, mut archivos)) = leer_paquete() else { return };
    let perfil = archivos
        .remove(Path::new(ARCHIVO_PERFIL))
        .and_then(|nombre| deimon::perfil::Perfil::desde_texto(&String::from_utf8_lossy(&nombre)));
    deimon::fuentes::usar_paquete(archivos);
    deimon::interprete::elegir_perfil(perfil.unwrap_or(deimon::perfil::Perfil::Equilibrado));
    let fuente = match deimon::fuentes::leer_texto(&principal) {
        Ok(fuente) => fuente,
        Err(error) => {
            eprintln!("Error: el programa empaquetado está dañado: {}", error);
            std::process::exit(1);
        }
    };
    match deimon::compilar_para_interprete(&fuente, &principal) {
        Ok(codigo) => {
            deimon::interprete::ejecutar(&codigo);
            std::process::exit(0);
        }
        Err(motivo) => {
            eprintln!("Error: el programa empaquetado no se pudo preparar: {}", motivo);
            std::process::exit(1);
        }
    }
}
