// Lectura de plantillas HTML. Este archivo lo usan el traductor (para revisar las
// plantillas antes de ejecutar) y el programa generado (para rellenarlas): no debe usar
// nada más que la biblioteca estándar.

#[allow(dead_code)]
pub enum TrozoPlantilla {
    Texto(String),
    Dato { camino: Vec<String>, proteger: bool },
    Para { variable: String, camino: Vec<String>, cuerpo: Vec<TrozoPlantilla> },
    Si { negar: bool, camino: Vec<String>, entonces: Vec<TrozoPlantilla>, sino: Vec<TrozoPlantilla> },
}

fn camino_de_plantilla(texto: &str) -> Result<Vec<String>, String> {
    let partes: Vec<String> = texto.trim().split('.').map(|p| p.trim().to_string()).collect();
    if partes.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_alphanumeric() || c == '_')) {
        return Err(format!("'{}' no es un nombre válido (usa nombre o nombre.campo)", texto.trim()));
    }
    Ok(partes)
}

/// Lee los trozos hasta una etiqueta de cierre ({% fin %} o {% sino %}); devuelve cuál la cerró.
fn leer_trozos(texto: &str, pos: &mut usize, linea: &mut usize) -> Result<(Vec<TrozoPlantilla>, Option<String>), String> {
    let mut trozos = Vec::new();
    loop {
        let resto = &texto[*pos..];
        let siguiente = ["{{", "{%", "{#"].iter().filter_map(|marca| resto.find(marca)).min();
        let inicio = match siguiente {
            None => {
                if !resto.is_empty() {
                    trozos.push(TrozoPlantilla::Texto(resto.to_string()));
                }
                *pos = texto.len();
                return Ok((trozos, None));
            }
            Some(inicio) => inicio,
        };
        if inicio > 0 {
            trozos.push(TrozoPlantilla::Texto(resto[..inicio].to_string()));
        }
        *linea += resto[..inicio].matches('\n').count();
        let marca = &resto[inicio..];
        let (cierre, crudo) = if marca.starts_with("{{{") {
            ("}}}", true)
        } else if marca.starts_with("{{") {
            ("}}", false)
        } else if marca.starts_with("{%") {
            ("%}", false)
        } else {
            ("#}", false)
        };
        let apertura = if crudo { 3 } else { 2 };
        let fin = match marca[apertura..].find(cierre) {
            Some(fin) => fin,
            None => return Err(format!("línea {}: falta cerrar con '{}'", linea, cierre)),
        };
        let dentro = &marca[apertura..apertura + fin];
        *linea += dentro.matches('\n').count();
        *pos += inicio + apertura + fin + cierre.len();
        if marca.starts_with("{#") {
            continue;
        }
        if marca.starts_with("{{") {
            let camino = camino_de_plantilla(dentro).map_err(|e| format!("línea {}: {}", linea, e))?;
            trozos.push(TrozoPlantilla::Dato { camino, proteger: !crudo });
            continue;
        }
        let palabras: Vec<&str> = dentro.split_whitespace().collect();
        match palabras.as_slice() {
            ["fin"] => return Ok((trozos, Some("fin".to_string()))),
            ["sino"] => return Ok((trozos, Some("sino".to_string()))),
            ["para", variable, "en", lista] => {
                let linea_abre = *linea;
                let variable = camino_de_plantilla(variable).map_err(|e| format!("línea {}: {}", linea, e))?;
                let camino = camino_de_plantilla(lista).map_err(|e| format!("línea {}: {}", linea, e))?;
                let (cuerpo, cierre) = leer_trozos(texto, pos, linea)?;
                if cierre.as_deref() != Some("fin") {
                    return Err(format!("línea {}: falta {{% fin %}} para el 'para'", linea_abre));
                }
                trozos.push(TrozoPlantilla::Para { variable: variable.join("."), camino, cuerpo });
            }
            ["si", resto @ ..] if !resto.is_empty() => {
                let linea_abre = *linea;
                let (negar, dato) = match resto {
                    ["no", dato] => (true, *dato),
                    [dato] => (false, *dato),
                    _ => return Err(format!("línea {}: escribe {{% si dato %}} o {{% si no dato %}}", linea)),
                };
                let camino = camino_de_plantilla(dato).map_err(|e| format!("línea {}: {}", linea, e))?;
                let (entonces, cierre) = leer_trozos(texto, pos, linea)?;
                let sino = match cierre.as_deref() {
                    Some("fin") => Vec::new(),
                    Some("sino") => {
                        let (sino, cierre) = leer_trozos(texto, pos, linea)?;
                        if cierre.as_deref() != Some("fin") {
                            return Err(format!("línea {}: falta {{% fin %}} para el 'si'", linea_abre));
                        }
                        sino
                    }
                    _ => return Err(format!("línea {}: falta {{% fin %}} para el 'si'", linea_abre)),
                };
                trozos.push(TrozoPlantilla::Si { negar, camino, entonces, sino });
            }
            _ => return Err(format!("línea {}: no se entiende {{% {} %}}", linea, dentro.trim())),
        }
    }
}

pub fn analizar_plantilla(texto: &str) -> Result<Vec<TrozoPlantilla>, String> {
    let (mut pos, mut linea) = (0usize, 1usize);
    let (trozos, cierre) = leer_trozos(texto, &mut pos, &mut linea)?;
    match cierre {
        None => Ok(trozos),
        Some(etiqueta) => Err(format!("línea {}: sobra {{% {} %}}", linea, etiqueta)),
    }
}

