//! Aplica los arreglos automáticos seguros de los errores a un texto de programa.

use std::path::Path;

use crate::error::{Arreglo, ErrorTraduccion};

/// Devuelve el programa arreglado y cuántos arreglos se aplicaron.
/// Los arreglos que se solapan con otro anterior se saltan (se aplicarán en la siguiente vuelta).
pub fn aplicar(fuente: &str, errores: &[ErrorTraduccion]) -> (String, usize) {
    let mut arreglos: Vec<&Arreglo> = errores
        .iter()
        .filter(|e| e.archivo.is_none())
        .filter_map(|e| e.arreglo.as_ref())
        .collect();
    arreglos.sort_by_key(|a| (a.linea, a.columna_inicio, a.columna_fin));

    let mut elegidos: Vec<&Arreglo> = Vec::new();
    for arreglo in arreglos {
        let solapa = elegidos.last().map_or(false, |anterior| {
            anterior.linea == arreglo.linea && arreglo.columna_inicio < anterior.columna_fin
        });
        if !solapa && arreglo.columna_inicio >= 1 && arreglo.columna_fin >= arreglo.columna_inicio {
            elegidos.push(arreglo);
        }
    }

    let mut lineas: Vec<Vec<char>> = fuente.split('\n').map(|l| l.chars().collect()).collect();
    let mut aplicados = 0;
    // De derecha a izquierda para que las columnas anteriores no se desplacen.
    for arreglo in elegidos.iter().rev() {
        let Some(linea) = lineas.get_mut(arreglo.linea.wrapping_sub(1)) else {
            continue;
        };
        let inicio = arreglo.columna_inicio - 1;
        let fin = arreglo.columna_fin - 1;
        if fin > linea.len() {
            continue;
        }
        linea.splice(inicio..fin, arreglo.reemplazo.chars());
        aplicados += 1;
    }
    let texto = lineas
        .into_iter()
        .map(|l| l.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    (texto, aplicados)
}

/// Aplica arreglos y vuelve a comprobar hasta que no haya más que aplicar (con un máximo de
/// vueltas). Devuelve el programa final, el total de arreglos aplicados y los errores restantes.
pub fn arreglar(fuente: &str) -> (String, usize, Vec<ErrorTraduccion>) {
    arreglar_en(fuente, None)
}

/// Como `arreglar`, resolviendo los `usar` relativos a la carpeta de `archivo`.
/// Los arreglos de los archivos usados no se aplican: solo se toca el archivo principal.
pub fn arreglar_en(
    fuente: &str,
    archivo: Option<&Path>,
) -> (String, usize, Vec<ErrorTraduccion>) {
    let comprobar = |texto: &str| match archivo {
        Some(archivo) => crate::comprobar_archivo(texto, archivo),
        None => crate::comprobar(texto),
    };
    let mut texto = fuente.to_string();
    let mut total = 0;
    for _ in 0..30 {
        let errores = comprobar(&texto);
        let (nuevo, aplicados) = aplicar(&texto, &errores);
        if aplicados == 0 {
            return (texto, total, errores);
        }
        texto = nuevo;
        total += aplicados;
    }
    let errores = comprobar(&texto);
    (texto, total, errores)
}
