use std::path::Path;

use crate::ast::*;
use crate::error::ErrorTraduccion;
use crate::{lexer, modulos, parser};

/// Cómo usa el programa la instrucción `servir`.
#[derive(Debug, Clone, PartialEq)]
pub enum UsoServir {
    /// El programa no llama a `servir`.
    Ninguno,
    /// `servir(N)` con un número escrito directamente.
    Puerto(u16),
    /// `servir(...)` con un valor que solo se conoce al ejecutar.
    Calculado,
}

/// Datos útiles de un programa sin generar ni compilar nada.
#[derive(Debug, Clone, PartialEq)]
pub struct Informacion {
    pub pruebas: Vec<String>,
    pub servir: UsoServir,
}

pub fn informacion(fuente: &str) -> Result<Informacion, ErrorTraduccion> {
    informacion_en(fuente, None)
}

/// Como `informacion`, resolviendo los `usar` relativos a la carpeta de `archivo`.
pub fn informacion_archivo(
    fuente: &str,
    archivo: &Path,
) -> Result<Informacion, ErrorTraduccion> {
    informacion_en(fuente, Some(archivo))
}

fn informacion_en(
    fuente: &str,
    archivo: Option<&Path>,
) -> Result<Informacion, ErrorTraduccion> {
    let programa = parser::analizar(lexer::tokenizar(fuente)?)?;
    let programa = modulos::resolver(programa, archivo)
        .map_err(|mut errores| errores.remove(0))?;
    let pruebas = programa
        .sentencias
        .iter()
        .filter_map(|sentencia| match sentencia {
            Sentencia::Prueba { nombre, .. } => Some(nombre.clone()),
            _ => None,
        })
        .collect();
    let servir = match buscar_en_sentencias(&programa.sentencias) {
        None => UsoServir::Ninguno,
        Some(Some(puerto)) => UsoServir::Puerto(puerto),
        Some(None) => UsoServir::Calculado,
    };
    Ok(Informacion { pruebas, servir })
}

fn buscar_en_sentencias(sentencias: &[Sentencia]) -> Option<Option<u16>> {
    sentencias.iter().find_map(buscar_en_sentencia)
}

fn buscar_en_sentencia(sentencia: &Sentencia) -> Option<Option<u16>> {
    match sentencia {
        Sentencia::Declarar { valor, .. } | Sentencia::Asignar { valor, .. } => {
            buscar_en_expresion(valor)
        }
        Sentencia::AsignarIndice { indices, valor, .. } => indices
            .iter()
            .find_map(buscar_en_expresion)
            .or_else(|| buscar_en_expresion(valor)),
        Sentencia::Si {
            condicion,
            entonces,
            sino,
            ..
        } => buscar_en_expresion(condicion)
            .or_else(|| buscar_en_sentencias(entonces))
            .or_else(|| buscar_en_sentencias(sino)),
        Sentencia::Mientras {
            condicion, cuerpo, ..
        } => buscar_en_expresion(condicion).or_else(|| buscar_en_sentencias(cuerpo)),
        Sentencia::Intentar { cuerpo, captura, .. } => {
            buscar_en_sentencias(cuerpo).or_else(|| buscar_en_sentencias(captura))
        }
        Sentencia::Para {
            coleccion, cuerpo, ..
        } => buscar_en_expresion(coleccion).or_else(|| buscar_en_sentencias(cuerpo)),
        Sentencia::Funcion { cuerpo, .. } | Sentencia::Ruta { cuerpo, .. } => {
            buscar_en_sentencias(cuerpo)
        }
        Sentencia::Prueba { .. }
        | Sentencia::Usar { .. }
        | Sentencia::UsarExtension { .. }
        | Sentencia::Romper { .. }
        | Sentencia::Continuar { .. } => None,
        Sentencia::Devolver { valor, .. } => valor.as_ref().and_then(buscar_en_expresion),
        Sentencia::Expresion(expresion) => buscar_en_expresion(expresion),
    }
}

fn buscar_en_expresion(expresion: &Expresion) -> Option<Option<u16>> {
    match expresion {
        Expresion::Llamada {
            nombre, argumentos, ..
        } => {
            if nombre == "servir" {
                return Some(match argumentos.first() {
                    Some(Expresion::Numero(n))
                        if n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u16::MAX) =>
                    {
                        Some(*n as u16)
                    }
                    _ => None,
                });
            }
            argumentos.iter().find_map(buscar_en_expresion)
        }
        Expresion::Lista(elementos) => elementos.iter().find_map(buscar_en_expresion),
        Expresion::TextoConValores { partes, .. } => partes.iter().find_map(|parte| match parte {
            ParteTexto::Valor(valor) => buscar_en_expresion(valor),
            ParteTexto::Literal(_) => None,
        }),
        Expresion::Unaria { valor, .. } => buscar_en_expresion(valor),
        Expresion::Binaria {
            izquierda, derecha, ..
        } => buscar_en_expresion(izquierda).or_else(|| buscar_en_expresion(derecha)),
        Expresion::Indice {
            coleccion, indice, ..
        } => buscar_en_expresion(coleccion).or_else(|| buscar_en_expresion(indice)),
        _ => None,
    }
}
