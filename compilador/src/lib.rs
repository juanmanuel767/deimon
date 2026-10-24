pub mod ajenas;
pub mod antiguas;
pub mod arreglos;
pub mod ast;
pub mod biblioteca;
pub mod desde_cobol;
pub mod desde_php;
pub mod error;
pub mod formato;
pub mod fuentes;
pub mod generador;
pub mod guia;
pub mod informacion;
pub mod interprete;
pub mod integradas;
pub mod json;
pub mod lexer;
pub mod modulos;
pub mod numerico;
pub mod parser;
pub mod plantillas;
pub mod tipos;
pub mod perfil;

use std::collections::HashMap;
use std::path::Path;

use crate::ast::Extension;
use crate::error::ErrorTraduccion;

pub use crate::error::{texto_de_errores, Arreglo};
pub use crate::informacion::{informacion, informacion_archivo, Informacion, UsoServir};

pub fn traducir(fuente: &str) -> Result<String, ErrorTraduccion> {
    traducir_con_modo(fuente, generador::Modo::Normal, None).map_err(|mut errores| errores.remove(0))
}

/// Como `traducir`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn traducir_archivo(fuente: &str, archivo: &Path) -> Result<String, ErrorTraduccion> {
    traducir_con_modo(fuente, generador::Modo::Normal, Some(archivo))
        .map_err(|mut errores| errores.remove(0))
}

/// Variante del programa cuyo `main` ejecuta solo las pruebas (la usa `deimon probar`).
pub fn traducir_pruebas(fuente: &str) -> Result<String, ErrorTraduccion> {
    traducir_con_modo(fuente, generador::Modo::Pruebas, None)
        .map_err(|mut errores| errores.remove(0))
}

/// Como `traducir_pruebas`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn traducir_pruebas_archivo(
    fuente: &str,
    archivo: &Path,
) -> Result<String, ErrorTraduccion> {
    traducir_con_modo(fuente, generador::Modo::Pruebas, Some(archivo))
        .map_err(|mut errores| errores.remove(0))
}

/// Como `traducir`, pero devuelve todos los errores encontrados (ordenados por posición).
pub fn traducir_varios(fuente: &str) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo(fuente, generador::Modo::Normal, None)
}

/// Como `traducir_varios`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn traducir_archivo_varios(
    fuente: &str,
    archivo: &Path,
) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo(fuente, generador::Modo::Normal, Some(archivo))
}

/// Como `traducir_archivo_varios`, grabando el perfil de recursos en el programa generado.
pub fn traducir_archivo_varios_con_perfil(
    fuente: &str,
    archivo: &Path,
    perfil: perfil::Perfil,
) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo_perfil(
        fuente,
        generador::Modo::Normal,
        Some(archivo),
        perfil.nombre(),
    )
}

/// Traduce para el destino `pc-sin-sistema`: el programa arranca solo en un PC, sin Linux.
pub fn traducir_archivo_sin_sistema(
    fuente: &str,
    archivo: &Path,
    perfil: perfil::Perfil,
) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo_perfil(
        fuente,
        generador::Modo::SinSistema,
        Some(archivo),
        perfil.nombre(),
    )
}

/// Como `traducir_pruebas`, pero devuelve todos los errores encontrados.
pub fn traducir_pruebas_varios(fuente: &str) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo(fuente, generador::Modo::Pruebas, None)
}

/// Como `traducir_pruebas_varios`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn traducir_pruebas_archivo_varios(
    fuente: &str,
    archivo: &Path,
) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo(fuente, generador::Modo::Pruebas, Some(archivo))
}

/// Devuelve las extensiones que usa un archivo (también las de los archivos que carga con
/// `usar`), sin generar Rust. La usa `deimon guia archivo.dmn`.
pub fn extensiones_usadas_archivo(
    fuente: &str,
    archivo: &Path,
) -> Result<Vec<Extension>, Vec<ErrorTraduccion>> {
    let tokens = lexer::tokenizar(fuente).map_err(|error| vec![error])?;
    let (tokens, mut errores) = ajenas::corregir(tokens);
    let (tokens, mut de_antiguas) = antiguas::corregir(tokens);
    errores.append(&mut de_antiguas);
    let programa = match parser::analizar(tokens) {
        Ok(programa) => programa,
        Err(error) => {
            errores.push(error);
            return Err(errores);
        }
    };
    match modulos::resolver(programa, Some(archivo)) {
        Ok(programa) => Ok(programa.extensiones),
        Err(mut de_modulos) => {
            errores.append(&mut de_modulos);
            Err(errores)
        }
    }
}

/// Prepara un programa para el intérprete: mismo análisis que `traducir` (lexer, parser y
/// módulos) y compilación a código intermedio. `Err` lleva el motivo por el que no se puede
/// interpretar (por ejemplo, rutas web) y entonces hay que usar el modo Rust. Los errores del
/// programa los detecta antes `traducir_archivo_varios`.
pub fn compilar_para_interprete(
    fuente: &str,
    archivo: &Path,
) -> Result<interprete::Codigo, String> {
    let tokens = lexer::tokenizar(fuente).map_err(|error| error.mensaje)?;
    let (tokens, errores) = ajenas::corregir(tokens);
    let (tokens, de_antiguas) = antiguas::corregir(tokens);
    if !errores.is_empty() || !de_antiguas.is_empty() {
        return Err("el programa tiene avisos".to_string());
    }
    let programa = parser::analizar(tokens).map_err(|error| error.mensaje)?;
    let programa = modulos::resolver(programa, Some(archivo))
        .map_err(|_| "no se pudieron cargar los módulos".to_string())?;
    interprete::compilar(&programa)
}

/// Nombres de las extensiones incluidas en el Rust generado (las marca el generador).
pub fn extensiones_del_rust(rust: &str) -> Vec<String> {
    rust.lines()
        .filter_map(|linea| linea.strip_prefix("// DEIMON_EXTENSION: "))
        .map(str::to_string)
        .collect()
}

/// Traduce sin compilar ni ejecutar y devuelve todos los errores (vacío si todo está bien).
/// Los errores de caracteres (lexer) y de sintaxis detienen el análisis en el primero;
/// los de nombres y argumentos se recogen todos.
pub fn comprobar(fuente: &str) -> Vec<ErrorTraduccion> {
    traducir_varios(fuente).err().unwrap_or_default()
}

/// Como `comprobar`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn comprobar_archivo(fuente: &str, archivo: &Path) -> Vec<ErrorTraduccion> {
    traducir_archivo_varios(fuente, archivo)
        .err()
        .unwrap_or_default()
}

fn traducir_con_modo(
    fuente: &str,
    modo: generador::Modo,
    archivo: Option<&Path>,
) -> Result<String, Vec<ErrorTraduccion>> {
    traducir_con_modo_perfil(
        fuente,
        modo,
        archivo,
        perfil::Perfil::Equilibrado.nombre(),
    )
}

fn traducir_con_modo_perfil(
    fuente: &str,
    modo: generador::Modo,
    archivo: Option<&Path>,
    perfil: &str,
) -> Result<String, Vec<ErrorTraduccion>> {
    let tokens = lexer::tokenizar(fuente).map_err(|error| ordenar(vec![error], fuente, &HashMap::new()))?;
    let (tokens, mut errores) = ajenas::corregir(tokens);
    let (tokens, mut de_antiguas) = antiguas::corregir(tokens);
    errores.append(&mut de_antiguas);
    let programa = match parser::analizar(tokens) {
        Ok(programa) => programa,
        Err(error) => {
            errores.push(error);
            return Err(ordenar(errores, fuente, &HashMap::new()));
        }
    };
    let (programa, fuentes) = match modulos::resolver_con_fuentes(programa, archivo) {
        Ok(programa) => programa,
        Err(mut de_modulos) => {
            errores.append(&mut de_modulos);
            return Err(ordenar(errores, fuente, &HashMap::new()));
        }
    };
    // Operaciones que fallarían seguro (E025), antes de ejecutar nada.
    errores.append(&mut tipos::revisar(&programa));
    match generador::generar_con_modo_perfil(&programa, modo, perfil) {
        Ok(rust) if errores.is_empty() => Ok(rust),
        Ok(_) => Err(ordenar(errores, fuente, &fuentes)),
        Err(mut del_generador) => {
            errores.append(&mut del_generador);
            Err(ordenar(errores, fuente, &fuentes))
        }
    }
}

/// Ordena por posición, quita repetidos y añade a cada error el texto de su línea.
fn ordenar(
    mut errores: Vec<ErrorTraduccion>,
    fuente: &str,
    fuentes: &HashMap<String, String>,
) -> Vec<ErrorTraduccion> {
    errores.sort_by_key(|e| (e.linea, e.columna));
    errores.dedup_by(|a, b| {
        a.linea == b.linea
            && a.columna == b.columna
            && a.mensaje == b.mensaje
            && a.archivo == b.archivo
    });
    errores
        .into_iter()
        .map(|error| {
            if error.linea_codigo.is_some() {
                return error;
            }
            let texto = match &error.archivo {
                Some(archivo) => fuentes.get(archivo).map(String::as_str).unwrap_or(fuente),
                None => fuente,
            };
            error.con_fuente(texto)
        })
        .collect()
}
