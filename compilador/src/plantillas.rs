//! Plantillas HTML: cambia cada `plantilla("archivo.html", datos)` por
//! `rellenar_plantilla("<contenido del archivo>", datos)`. Así el archivo viaja dentro del
//! programa y, si falta o está mal escrito, se avisa al traducir y no al visitar la página.

use std::path::Path;

use crate::ast::{Expresion, Sentencia};
use crate::error::ErrorTraduccion;

#[path = "plantilla_lectura.rs"]
mod lectura;

/// Incluye las plantillas de estas sentencias; las rutas son relativas a `carpeta`.
pub fn incluir(sentencias: &mut [Sentencia], carpeta: &Path) -> Vec<ErrorTraduccion> {
    let mut errores = Vec::new();
    for sentencia in sentencias {
        en_sentencia(sentencia, carpeta, &mut errores);
    }
    errores
}

fn en_sentencias(lista: &mut [Sentencia], carpeta: &Path, errores: &mut Vec<ErrorTraduccion>) {
    for sentencia in lista {
        en_sentencia(sentencia, carpeta, errores);
    }
}

fn en_sentencia(sentencia: &mut Sentencia, carpeta: &Path, errores: &mut Vec<ErrorTraduccion>) {
    match sentencia {
        Sentencia::Declarar { valor, .. } | Sentencia::Asignar { valor, .. } => en_expresion(valor, carpeta, errores),
        Sentencia::AsignarIndice { indices, valor, .. } => {
            for indice in indices {
                en_expresion(indice, carpeta, errores);
            }
            en_expresion(valor, carpeta, errores);
        }
        Sentencia::Si { condicion, entonces, sino, .. } => {
            en_expresion(condicion, carpeta, errores);
            en_sentencias(entonces, carpeta, errores);
            en_sentencias(sino, carpeta, errores);
        }
        Sentencia::Mientras { condicion, cuerpo, .. } => {
            en_expresion(condicion, carpeta, errores);
            en_sentencias(cuerpo, carpeta, errores);
        }
        Sentencia::Para { coleccion, cuerpo, .. } => {
            en_expresion(coleccion, carpeta, errores);
            en_sentencias(cuerpo, carpeta, errores);
        }
        Sentencia::Funcion { cuerpo, .. } | Sentencia::Ruta { cuerpo, .. } | Sentencia::Prueba { cuerpo, .. } => {
            en_sentencias(cuerpo, carpeta, errores)
        }
        Sentencia::Devolver { valor: Some(valor), .. } => en_expresion(valor, carpeta, errores),
        Sentencia::Expresion(expresion) => en_expresion(expresion, carpeta, errores),
        _ => {}
    }
}

fn en_expresion(expresion: &mut Expresion, carpeta: &Path, errores: &mut Vec<ErrorTraduccion>) {
    match expresion {
        Expresion::Llamada { nombre, argumentos, linea, columna } => {
            for argumento in argumentos.iter_mut() {
                en_expresion(argumento, carpeta, errores);
            }
            if nombre == "plantilla" && argumentos.len() == 2 {
                if let Expresion::Texto(ruta) = &argumentos[0] {
                    let completa = carpeta.join(ruta);
                    match crate::fuentes::leer_texto(&completa) {
                        Ok(texto) => match lectura::analizar_plantilla(&texto) {
                            Ok(_) => {
                                *nombre = "rellenar_plantilla".to_string();
                                argumentos[0] = Expresion::Texto(texto);
                            }
                            Err(motivo) => errores.push(
                                ErrorTraduccion::nuevo(*linea, format!("la plantilla '{}' está mal escrita: {}", ruta, motivo))
                                    .con_codigo("E024")
                                    .en_columna(*columna)
                                    .con_sugerencia("Cada {% para %} y {% si %} se cierra con {% fin %}, y los datos se escriben {{ nombre }}"),
                            ),
                        },
                        Err(_) => errores.push(
                            ErrorTraduccion::nuevo(*linea, format!("no se encontró la plantilla '{}'", ruta))
                                .con_codigo("E024")
                                .en_columna(*columna)
                                .con_sugerencia("La ruta es relativa a la carpeta del archivo .dmn que la usa"),
                        ),
                    }
                }
            }
        }
        Expresion::Lista(elementos) => {
            for elemento in elementos {
                en_expresion(elemento, carpeta, errores);
            }
        }
        Expresion::Mapa(pares) => {
            for (clave, valor) in pares {
                en_expresion(clave, carpeta, errores);
                en_expresion(valor, carpeta, errores);
            }
        }
        Expresion::Unaria { valor, .. } => en_expresion(valor, carpeta, errores),
        Expresion::Binaria { izquierda, derecha, .. } => {
            en_expresion(izquierda, carpeta, errores);
            en_expresion(derecha, carpeta, errores);
        }
        Expresion::Indice { coleccion, indice, .. } => {
            en_expresion(coleccion, carpeta, errores);
            en_expresion(indice, carpeta, errores);
        }
        _ => {}
    }
}
