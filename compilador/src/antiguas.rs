//! Nombres antiguos de funciones integradas. Se avisan con el código E021, se sugiere
//! el nombre nuevo y se ofrece un arreglo automático para `deimon comprobar --arreglar`.

use crate::ajenas::nombres_declarados;
use crate::error::{Arreglo, ErrorTraduccion};
use crate::lexer::{Tipo, Token};

/// (nombre antiguo, nombre nuevo).
pub const ANTIGUAS: &[(&str, &str)] = &[
    ("numero", "a_numero"),
    ("largo", "medir"),
    ("claves", "listar_claves"),
];

/// Devuelve los tokens con los nombres antiguos ya cambiados y la lista de avisos.
pub fn corregir(tokens: Vec<Token>) -> (Vec<Token>, Vec<ErrorTraduccion>) {
    let declarados = nombres_declarados(&tokens);
    let mut salida = Vec::with_capacity(tokens.len());
    let mut errores = Vec::new();
    for (posicion, token) in tokens.iter().enumerate() {
        let tras_usar = posicion > 0 && matches!(tokens[posicion - 1].tipo, Tipo::Usar);
        if tras_usar && matches!(token.tipo, Tipo::Extension) {
            errores.push(
                ErrorTraduccion::nuevo(token.linea, "'usar extension' es una forma antigua")
                    .con_codigo("E021")
                    .en_columna(token.columna)
                    .con_sugerencia("Ahora se escribe 'usar modulo'")
                    .con_arreglo(Arreglo {
                        linea: token.linea,
                        columna_inicio: token.columna,
                        columna_fin: token.fin,
                        reemplazo: "modulo".to_string(),
                    }),
            );
            salida.push(Token {
                tipo: Tipo::Modulo,
                linea: token.linea,
                columna: token.columna,
                fin: token.fin,
            });
            continue;
        }
        if let Tipo::Ident(nombre) = &token.tipo {
            let es_llamada = matches!(
                tokens.get(posicion + 1).map(|siguiente| &siguiente.tipo),
                Some(Tipo::LParen)
            );
            if es_llamada && !declarados.contains(nombre) {
                if let Some((antiguo, nuevo)) =
                    ANTIGUAS.iter().find(|(antiguo, _)| *antiguo == nombre)
                {
                    errores.push(
                        ErrorTraduccion::nuevo(
                            token.linea,
                            format!("'{}' es un nombre antiguo", antiguo),
                        )
                        .con_codigo("E021")
                        .en_columna(token.columna)
                        .con_sugerencia(format!("Ahora se escribe '{}'", nuevo))
                        .con_arreglo(Arreglo {
                            linea: token.linea,
                            columna_inicio: token.columna,
                            columna_fin: token.fin,
                            reemplazo: nuevo.to_string(),
                        }),
                    );
                    salida.push(Token {
                        tipo: Tipo::Ident(nuevo.to_string()),
                        linea: token.linea,
                        columna: token.columna,
                        fin: token.fin,
                    });
                    continue;
                }
            }
        }
        salida.push(token.clone());
    }
    (salida, errores)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::lexer::tokenizar;

    fn avisos(fuente: &str) -> Vec<ErrorTraduccion> {
        corregir(tokenizar(fuente).unwrap()).1
    }

    #[test]
    fn avisa_de_los_nombres_antiguos_en_las_llamadas() {
        for (antiguo, nuevo) in ANTIGUAS {
            let errores = avisos(&format!("mostrar({}(1))\n", antiguo));
            assert_eq!(errores.len(), 1, "{}", antiguo);
            assert_eq!(errores[0].codigo, "E021");
            assert_eq!(
                errores[0].sugerencia.as_deref().unwrap(),
                format!("Ahora se escribe '{}'", nuevo)
            );
            assert_eq!(errores[0].arreglo.as_ref().unwrap().reemplazo, *nuevo);
        }
    }

    #[test]
    fn avisa_de_usar_extension_y_lo_cambia_por_usar_modulo() {
        let (tokens, errores) = corregir(tokenizar("usar extension \"texto\"\n").unwrap());
        assert_eq!(errores.len(), 1);
        assert_eq!(errores[0].codigo, "E021");
        assert_eq!(errores[0].arreglo.as_ref().unwrap().reemplazo, "modulo");
        assert!(matches!(tokens[1].tipo, Tipo::Modulo));
    }

    #[test]
    fn no_toca_una_variable_que_se_llama_como_el_nombre_antiguo() {
        assert!(avisos("sea largo = 2\nmostrar(largo)\n").is_empty());
    }
}
