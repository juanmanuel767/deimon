//! Lectura de los archivos de un programa (los `.dmn`, sus módulos y sus plantillas).
//!
//! Normalmente se leen del disco. En un programa empaquetado (`deimon empaquetar`) se leen
//! del propio ejecutable: llevan dentro exactamente los archivos que se usaron al
//! empaquetar, con las mismas rutas, y no se mira nada del disco del ordenador donde se abre.
//! Al empaquetar se apunta cada archivo leído para meterlo en el paquete.

use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// En un paquete, la carpeta de la biblioteca estándar: se lee de dentro de `deimon`.
pub const RAIZ_BIBLIOTECA: &str = "/__deimon__/biblioteca";
/// En un paquete, la carpeta de los módulos personales (`~/.deimon/modulos`) que se usaron.
pub const RAIZ_PERSONAL: &str = "/__deimon__/personales";

/// Archivos del programa empaquetado (ruta absoluta → contenido), si este es uno.
static PAQUETE: OnceLock<BTreeMap<PathBuf, Vec<u8>>> = OnceLock::new();
/// Archivos leídos mientras se prepara un paquete (en el orden en que se leyeron).
static APUNTADOS: Mutex<Option<BTreeMap<PathBuf, Vec<u8>>>> = Mutex::new(None);

/// A partir de ahora, los archivos se leen de `archivos` y no del disco.
pub fn usar_paquete(archivos: BTreeMap<PathBuf, Vec<u8>>) {
    let _ = PAQUETE.set(archivos);
}

/// Empieza a apuntar los archivos que se leen.
pub fn empezar_a_apuntar() {
    *APUNTADOS.lock().unwrap_or_else(|e| e.into_inner()) = Some(BTreeMap::new());
}

/// Deja de apuntar y devuelve los archivos leídos.
pub fn apuntados() -> BTreeMap<PathBuf, Vec<u8>> {
    APUNTADOS.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default()
}

fn apuntar(ruta: &Path, contenido: &[u8]) {
    if let Some(apuntados) = APUNTADOS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        apuntados.insert(ruta.to_path_buf(), contenido.to_vec());
    }
}

/// Quita los `.` y `..` de una ruta sin mirar el disco.
fn normalizar(ruta: &Path) -> PathBuf {
    let mut salida = PathBuf::new();
    for parte in ruta.components() {
        match parte {
            Component::CurDir => {}
            Component::ParentDir => {
                salida.pop();
            }
            otra => salida.push(otra.as_os_str()),
        }
    }
    salida
}

/// Contenido de un archivo del paquete (los de la biblioteca salen de dentro de `deimon`).
fn del_paquete(paquete: &BTreeMap<PathBuf, Vec<u8>>, ruta: &Path) -> Option<Vec<u8>> {
    if let Ok(relativa) = ruta.strip_prefix(RAIZ_BIBLIOTECA) {
        let relativa = relativa.to_string_lossy();
        return crate::biblioteca::ARCHIVOS
            .iter()
            .find(|(nombre, _)| *nombre == relativa)
            .map(|(_, contenido)| contenido.as_bytes().to_vec());
    }
    paquete.get(ruta).cloned()
}

/// Todas las rutas de archivo que hay en el paquete (con las de la biblioteca).
fn rutas_del_paquete(paquete: &BTreeMap<PathBuf, Vec<u8>>) -> Vec<PathBuf> {
    paquete
        .keys()
        .cloned()
        .chain(crate::biblioteca::ARCHIVOS.iter().map(|(nombre, _)| Path::new(RAIZ_BIBLIOTECA).join(nombre)))
        .collect()
}

fn no_esta(ruta: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("'{}' no está en el programa empaquetado", ruta.display()))
}

/// Como `fs::canonicalize`.
pub fn canonica(ruta: &Path) -> io::Result<PathBuf> {
    match PAQUETE.get() {
        Some(paquete) => {
            let ruta = normalizar(ruta);
            if del_paquete(paquete, &ruta).is_some() || es_carpeta(&ruta) {
                Ok(ruta)
            } else {
                Err(no_esta(&ruta))
            }
        }
        None => std::fs::canonicalize(ruta),
    }
}

/// Como `fs::read_to_string`.
pub fn leer_texto(ruta: &Path) -> io::Result<String> {
    match PAQUETE.get() {
        Some(paquete) => {
            let ruta = normalizar(ruta);
            let contenido = del_paquete(paquete, &ruta).ok_or_else(|| no_esta(&ruta))?;
            String::from_utf8(contenido).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }
        None => {
            let texto = std::fs::read_to_string(ruta)?;
            if let Ok(completa) = std::fs::canonicalize(ruta) {
                apuntar(&completa, texto.as_bytes());
            }
            Ok(texto)
        }
    }
}

/// Como `Path::is_file`.
pub fn es_archivo(ruta: &Path) -> bool {
    match PAQUETE.get() {
        Some(paquete) => del_paquete(paquete, &normalizar(ruta)).is_some(),
        None => ruta.is_file(),
    }
}

/// Como `Path::is_dir`: en un paquete, una carpeta es la de algún archivo empaquetado.
pub fn es_carpeta(ruta: &Path) -> bool {
    match PAQUETE.get() {
        Some(paquete) => {
            let ruta = normalizar(ruta);
            rutas_del_paquete(paquete).iter().any(|archivo| archivo.starts_with(&ruta) && *archivo != ruta)
        }
        None => ruta.is_dir(),
    }
}

/// Como `fs::read_dir`, pero devuelve directamente las rutas de lo que hay dentro.
pub fn listar(carpeta: &Path) -> io::Result<Vec<PathBuf>> {
    match PAQUETE.get() {
        Some(paquete) => {
            let carpeta = normalizar(carpeta);
            let mut dentro: Vec<PathBuf> = rutas_del_paquete(paquete)
                .iter()
                .filter_map(|archivo| {
                    let resto = archivo.strip_prefix(&carpeta).ok()?;
                    let primero = resto.components().next()?;
                    Some(carpeta.join(primero.as_os_str()))
                })
                .collect();
            dentro.sort();
            dentro.dedup();
            Ok(dentro)
        }
        None => Ok(std::fs::read_dir(carpeta)?.filter_map(|e| e.ok()).map(|e| e.path()).collect()),
    }
}

/// ¿Es este ejecutable un programa empaquetado?
pub fn en_paquete() -> bool {
    PAQUETE.get().is_some()
}

/// Prepara los archivos apuntados para meterlos en un paquete: la biblioteca estándar no hace
/// falta (va dentro de `deimon`) y los módulos personales pasan a `RAIZ_PERSONAL`.
pub fn para_el_paquete(
    apuntados: BTreeMap<PathBuf, Vec<u8>>,
    biblioteca: Option<&Path>,
    personal: &Path,
) -> BTreeMap<PathBuf, Vec<u8>> {
    let personal = std::fs::canonicalize(personal).unwrap_or_else(|_| personal.to_path_buf());
    apuntados
        .into_iter()
        .filter(|(ruta, _)| biblioteca.map(|b| !ruta.starts_with(b)).unwrap_or(true))
        .map(|(ruta, contenido)| match ruta.strip_prefix(&personal) {
            Ok(resto) => (Path::new(RAIZ_PERSONAL).join(resto), contenido),
            Err(_) => (ruta, contenido),
        })
        .collect()
}
