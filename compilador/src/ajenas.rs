//! Palabras y operadores de otros lenguajes que una IA (o una persona) suele escribir por costumbre.
//! Se detectan sobre los tokens, se avisan con su equivalente exacto en Deimon y se sustituyen
//! «virtualmente» para que el análisis siga y pueda encontrar más errores.

use std::collections::HashSet;

use crate::error::{Arreglo, ErrorTraduccion};
use crate::lexer::{Tipo, Token};

/// (lo que se escribe en otro lenguaje, lo que se escribe en Deimon).
pub const EQUIVALENCIAS: &[(&str, &str)] = &[
    ("if", "si"),
    ("else if", "sino si"),
    ("elseif", "sino si"),
    ("elif", "sino si"),
    ("else", "sino"),
    ("while", "mientras"),
    ("for", "para"),
    ("in", "en"),
    ("function", "funcion"),
    ("def", "funcion"),
    ("fn", "funcion"),
    ("return", "devolver"),
    ("let", "sea"),
    ("var", "sea"),
    ("const", "sea"),
    ("print", "mostrar"),
    ("console.log", "mostrar"),
    ("true", "verdadero"),
    ("True", "verdadero"),
    ("false", "falso"),
    ("False", "falso"),
    ("null", "nada"),
    ("None", "nada"),
    ("and", "y"),
    ("&&", "y"),
    ("or", "o"),
    ("||", "o"),
    ("not", "no"),
    ("!", "no"),
    ("===", "=="),
    ("!==", "!="),
];

fn equivalente(palabra: &str) -> Option<&'static str> {
    EQUIVALENCIAS
        .iter()
        .find(|(otra, _)| *otra == palabra)
        .map(|(_, nuestro)| *nuestro)
}

fn tokens_nativos(palabra_ln: &str, base: &Token) -> Vec<Tipo> {
    match palabra_ln {
        "si" => vec![Tipo::Si],
        "sino" => vec![Tipo::Sino],
        "sino si" => vec![Tipo::Sino, Tipo::Si],
        "mientras" => vec![Tipo::Mientras],
        "para" => vec![Tipo::Para],
        "en" => vec![Tipo::En],
        "funcion" => vec![Tipo::Funcion],
        "devolver" => vec![Tipo::Devolver],
        "sea" => vec![Tipo::Sea],
        "mostrar" => vec![Tipo::Ident("mostrar".to_string())],
        "verdadero" => vec![Tipo::Verdadero],
        "falso" => vec![Tipo::Falso],
        "nada" => vec![Tipo::Nada],
        "y" => vec![Tipo::Y],
        "o" => vec![Tipo::O],
        "no" => vec![Tipo::No],
        "==" => vec![Tipo::Igual],
        "!=" => vec![Tipo::Distinto],
        _ => vec![base.tipo.clone()],
    }
}

/// Nombres que el propio programa declara (con `sea`, `funcion`, `para` o como parámetros):
/// si el programa ya usa una de estas palabras como nombre, no se toca.
pub(crate) fn nombres_declarados(tokens: &[Token]) -> HashSet<String> {
    let mut declarados = HashSet::new();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].tipo {
            Tipo::Sea | Tipo::Funcion | Tipo::Para => {
                if let Some(Token {
                    tipo: Tipo::Ident(nombre),
                    ..
                }) = tokens.get(i + 1)
                {
                    declarados.insert(nombre.clone());
                }
                if matches!(tokens[i].tipo, Tipo::Funcion)
                    && matches!(tokens.get(i + 2).map(|t| &t.tipo), Some(Tipo::LParen))
                {
                    let mut j = i + 3;
                    while j < tokens.len() && !matches!(tokens[j].tipo, Tipo::RParen | Tipo::Fin) {
                        if let Tipo::Ident(nombre) = &tokens[j].tipo {
                            declarados.insert(nombre.clone());
                        }
                        j += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    declarados
}

fn error_ajeno(palabra: &str, nuestro: &str, token: &Token, fin: usize, reemplazo: String) -> ErrorTraduccion {
    let sugerencia = match palabra {
        "for" => "En Deimon se escribe 'para': para elemento en lista { ... }".to_string(),
        "def" | "function" | "fn" => {
            format!("En Deimon se escribe '{}': funcion nombre(a, b) {{ ... }}", nuestro)
        }
        _ => format!("En Deimon se escribe '{}'", nuestro),
    };
    ErrorTraduccion::nuevo(token.linea, format!("'{}' no existe en Deimon", palabra))
        .con_codigo("E008")
        .en_columna(token.columna)
        .con_sugerencia(sugerencia)
        .con_arreglo(Arreglo {
            linea: token.linea,
            columna_inicio: token.columna,
            columna_fin: fin,
            reemplazo,
        })
}

/// Devuelve los tokens con las palabras ajenas ya traducidas a Deimon y la lista de avisos.
pub fn corregir(tokens: Vec<Token>) -> (Vec<Token>, Vec<ErrorTraduccion>) {
    let declarados = nombres_declarados(&tokens);
    let mut salida: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut errores = Vec::new();
    let mut for_en_linea: Option<usize> = None;
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        let palabra: Option<&str> = match &token.tipo {
            Tipo::Ident(nombre) if !declarados.contains(nombre) => Some(nombre.as_str()),
            Tipo::Ajeno(texto) => Some(texto.as_str()),
            _ => None,
        };
        let palabra = match palabra {
            Some(p) if equivalente(p).is_some() => p,
            _ => {
                salida.push(token.clone());
                i += 1;
                continue;
            }
        };
        // `in` solo es ajeno dentro de un `for ... in ...`.
        if palabra == "in" && for_en_linea != Some(token.linea) {
            salida.push(token.clone());
            i += 1;
            continue;
        }
        // `else if` son dos palabras que se corrigen juntas.
        if palabra == "else" {
            if let Some(siguiente) = tokens.get(i + 1) {
                if matches!(&siguiente.tipo, Tipo::Ident(n) if n == "if" && !declarados.contains(n))
                    && siguiente.linea == token.linea
                {
                    errores.push(error_ajeno(
                        "else if",
                        "sino si",
                        token,
                        siguiente.fin,
                        "sino si".to_string(),
                    ));
                    for (tipo, original) in [(Tipo::Sino, token), (Tipo::Si, siguiente)] {
                        salida.push(Token {
                            tipo,
                            linea: original.linea,
                            columna: original.columna,
                            fin: original.fin,
                        });
                    }
                    i += 2;
                    continue;
                }
            }
        }
        if palabra == "for" {
            for_en_linea = Some(token.linea);
        }
        let nuestro = equivalente(palabra).unwrap_or(palabra);
        // Los operadores de símbolos pegados a otra cosa necesitan espacios al volverse palabras.
        let es_simbolo = matches!(palabra, "&&" | "||" | "!");
        let pegado_antes = i > 0
            && tokens[i - 1].linea == token.linea
            && tokens[i - 1].fin == token.columna
            && !matches!(tokens[i - 1].tipo, Tipo::NuevaLinea);
        let pegado_despues = tokens
            .get(i + 1)
            .map_or(false, |t| t.linea == token.linea && t.columna == token.fin);
        let mut reemplazo = nuestro.to_string();
        if es_simbolo && pegado_antes {
            reemplazo.insert(0, ' ');
        }
        if es_simbolo && pegado_despues {
            reemplazo.push(' ');
        }
        errores.push(error_ajeno(palabra, nuestro, token, token.fin, reemplazo));
        for tipo in tokens_nativos(nuestro, token) {
            salida.push(Token {
                tipo,
                linea: token.linea,
                columna: token.columna,
                fin: token.fin,
            });
        }
        i += 1;
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
    fn no_toca_los_nombres_que_el_programa_declara() {
        assert!(avisos("sea print = 1\nmostrar(print)\n").is_empty());
    }

    #[test]
    fn else_if_se_avisa_como_una_sola_cosa() {
        let errores = avisos("si verdadero {\n} else if falso {\n}\n");
        assert_eq!(errores.len(), 1);
        assert_eq!(errores[0].arreglo.as_ref().unwrap().reemplazo, "sino si");
    }

    #[test]
    fn el_operador_pegado_recibe_espacios() {
        let errores = avisos("sea x = a&&b\n");
        assert_eq!(errores[0].arreglo.as_ref().unwrap().reemplazo, " y ");
    }
}
