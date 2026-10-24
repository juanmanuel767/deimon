use crate::error::{Arreglo, ErrorTraduccion};

#[derive(Debug, Clone, PartialEq)]
pub enum Tipo {
    Numero(f64),
    Texto(String),
    /// Comentario completo, desde '#' hasta el final de la línea. Solo lo produce
    /// `tokenizar_con_comentarios` (lo usa el formateador); el parser no lo ve.
    Comentario(String),
    Ident(String),
    /// Operador o palabra de otro lenguaje (`&&`, `||`, `!`, `===`, `console.log`...).
    /// Nunca es válido en Deimon: `ajenas` lo traduce o lo avisa.
    Ajeno(String),
    Sea,
    Usar,
    Funcion,
    Ruta,
    Prueba,
    Extension,
    Modulo,
    Devolver,
    Romper,
    Continuar,
    Si,
    Sino,
    Intenta,
    Falla,
    Mientras,
    Para,
    En,
    Y,
    O,
    No,
    Verdadero,
    Falso,
    Nada,
    Mas,
    Menos,
    Por,
    Entre,
    Resto,
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    Asignacion,
    LParen,
    RParen,
    LLave,
    RLave,
    LCorchete,
    RCorchete,
    Coma,
    DosPuntos,
    NuevaLinea,
    Fin,
}

/// `columna` y `fin` se cuentan en caracteres desde 1; `fin` no se incluye.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tipo: Tipo,
    pub linea: usize,
    pub columna: usize,
    pub fin: usize,
}

/// Palabras reservadas del lenguaje, tal y como las reconoce el lexer.
pub const PALABRAS_RESERVADAS: &[&str] = &[
    "sea",
    "usar",
    "funcion",
    "ruta",
    "prueba",
    "extension",
    "modulo",
    "devolver",
    "romper",
    "continuar",
    "si",
    "sino",
    "intentar",
    "falla",
    "mientras",
    "para",
    "en",
    "y",
    "o",
    "no",
    "verdadero",
    "falso",
    "nada",
];

pub fn tokenizar(fuente: &str) -> Result<Vec<Token>, ErrorTraduccion> {
    tokenizar_interno(fuente, false)
}

/// Como `tokenizar`, pero conserva los comentarios (`Tipo::Comentario`) y los saltos de
/// línea. Lo usa el formateador; el parser sigue trabajando con `tokenizar`.
pub fn tokenizar_con_comentarios(fuente: &str) -> Result<Vec<Token>, ErrorTraduccion> {
    tokenizar_interno(fuente, true)
}

fn tokenizar_interno(
    fuente: &str,
    guardar_comentarios: bool,
) -> Result<Vec<Token>, ErrorTraduccion> {
    let caracteres: Vec<char> = fuente.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut linea = 1;
    let mut inicio_linea = 0;

    while i < caracteres.len() {
        let c = caracteres[i];
        let inicio = i;
        let columna = inicio - inicio_linea + 1;
        let siguiente = caracteres.get(i + 1).copied();
        let tercero = caracteres.get(i + 2).copied();
        // Cada rama deja `tipo` con el token reconocido (o sigue sin escribir nada).
        let tipo = match c {
            ' ' | '\t' | '\r' => {
                i += 1;
                None
            }
            '\n' => {
                tokens.push(Token {
                    tipo: Tipo::NuevaLinea,
                    linea,
                    columna,
                    fin: columna + 1,
                });
                linea += 1;
                i += 1;
                inicio_linea = i;
                None
            }
            '#' => {
                while i < caracteres.len() && caracteres[i] != '\n' {
                    i += 1;
                }
                if guardar_comentarios {
                    let texto: String = caracteres[inicio..i].iter().collect();
                    Some(Tipo::Comentario(texto))
                } else {
                    None
                }
            }
            '"' => {
                let linea_texto = linea;
                i += 1;
                let mut texto = String::new();
                let mut cerrado = false;
                let sin_cerrar = || {
                    ErrorTraduccion::nuevo(linea_texto, "el texto no está cerrado")
                        .con_codigo("E001")
                        .en_columna(columna)
                        .con_sugerencia(format!(
                            "El texto que empieza en la columna {} de la línea {} necesita unas comillas dobles (\") de cierre en la misma línea",
                            columna, linea_texto
                        ))
                };
                while i < caracteres.len() {
                    let actual = caracteres[i];
                    match actual {
                        '"' => {
                            i += 1;
                            cerrado = true;
                            break;
                        }
                        '\\' => {
                            i += 1;
                            if i >= caracteres.len() {
                                break;
                            }
                            let escape = caracteres[i];
                            match escape {
                                'n' => texto.push('\n'),
                                't' => texto.push('\t'),
                                '"' => texto.push('"'),
                                '\\' => texto.push('\\'),
                                otro => {
                                    return Err(ErrorTraduccion::nuevo(
                                        linea,
                                        format!("el escape '\\{}' no existe", otro),
                                    )
                                    .con_codigo("E002")
                                    .en_columna(i - inicio_linea)
                                    .con_sugerencia(
                                        "Los escapes válidos son \\n, \\t, \\\" y \\\\",
                                    ));
                                }
                            }
                            i += 1;
                        }
                        '\n' => {
                            return Err(sin_cerrar());
                        }
                        otro => {
                            texto.push(otro);
                            i += 1;
                        }
                    }
                }
                if !cerrado {
                    return Err(sin_cerrar());
                }
                Some(Tipo::Texto(texto))
            }
            '0'..='9' => {
                while i < caracteres.len() && caracteres[i].is_ascii_digit() {
                    i += 1;
                }
                if i + 1 < caracteres.len()
                    && caracteres[i] == '.'
                    && caracteres[i + 1].is_ascii_digit()
                {
                    i += 1;
                    while i < caracteres.len() && caracteres[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let texto: String = caracteres[inicio..i].iter().collect();
                let numero = texto.parse::<f64>().map_err(|_| {
                    ErrorTraduccion::nuevo(linea, format!("el número '{}' no es válido", texto))
                        .con_codigo("E002")
                        .en_columna(columna)
                })?;
                Some(Tipo::Numero(numero))
            }
            _ if es_inicio_identificador(c) => {
                while i < caracteres.len() && es_continuacion_identificador(caracteres[i]) {
                    i += 1;
                }
                let palabra: String = caracteres[inicio..i].iter().collect();
                let es_console_log = palabra == "console"
                    && caracteres.get(i) == Some(&'.')
                    && caracteres[i + 1..].starts_with(&['l', 'o', 'g'])
                    && !caracteres
                        .get(i + 4)
                        .map_or(false, |c| es_continuacion_identificador(*c));
                if es_console_log {
                    i += 4;
                    Some(Tipo::Ajeno("console.log".to_string()))
                } else {
                    Some(match palabra.as_str() {
                        "sea" => Tipo::Sea,
                        "usar" => Tipo::Usar,
                        "funcion" => Tipo::Funcion,
                        "ruta" => Tipo::Ruta,
                        "prueba" => Tipo::Prueba,
                        "extension" => Tipo::Extension,
                        "modulo" => Tipo::Modulo,
                        "devolver" => Tipo::Devolver,
                        "romper" => Tipo::Romper,
                        "continuar" => Tipo::Continuar,
                        "si" => Tipo::Si,
                        "sino" => Tipo::Sino,
                        "intentar" => Tipo::Intenta,
                        "falla" => Tipo::Falla,
                        "mientras" => Tipo::Mientras,
                        "para" => Tipo::Para,
                        "en" => Tipo::En,
                        "y" => Tipo::Y,
                        "o" => Tipo::O,
                        "no" => Tipo::No,
                        "verdadero" => Tipo::Verdadero,
                        "falso" => Tipo::Falso,
                        "nada" => Tipo::Nada,
                        _ => Tipo::Ident(palabra),
                    })
                }
            }
            '+' => {
                i += 1;
                Some(Tipo::Mas)
            }
            '-' => {
                i += 1;
                Some(Tipo::Menos)
            }
            '*' => {
                i += 1;
                Some(Tipo::Por)
            }
            '/' => {
                i += 1;
                Some(Tipo::Entre)
            }
            '%' => {
                i += 1;
                Some(Tipo::Resto)
            }
            '=' => {
                if siguiente == Some('=') && tercero == Some('=') {
                    i += 3;
                    Some(Tipo::Ajeno("===".to_string()))
                } else if siguiente == Some('=') {
                    i += 2;
                    Some(Tipo::Igual)
                } else {
                    i += 1;
                    Some(Tipo::Asignacion)
                }
            }
            '!' => {
                if siguiente == Some('=') && tercero == Some('=') {
                    i += 3;
                    Some(Tipo::Ajeno("!==".to_string()))
                } else if siguiente == Some('=') {
                    i += 2;
                    Some(Tipo::Distinto)
                } else {
                    i += 1;
                    Some(Tipo::Ajeno("!".to_string()))
                }
            }
            '&' | '|' if siguiente == Some(c) => {
                i += 2;
                Some(Tipo::Ajeno(format!("{}{}", c, c)))
            }
            '<' => {
                if siguiente == Some('=') {
                    i += 2;
                    Some(Tipo::MenorIgual)
                } else {
                    i += 1;
                    Some(Tipo::Menor)
                }
            }
            '>' => {
                if siguiente == Some('=') {
                    i += 2;
                    Some(Tipo::MayorIgual)
                } else {
                    i += 1;
                    Some(Tipo::Mayor)
                }
            }
            '(' => {
                i += 1;
                Some(Tipo::LParen)
            }
            ')' => {
                i += 1;
                Some(Tipo::RParen)
            }
            '{' => {
                i += 1;
                Some(Tipo::LLave)
            }
            '}' => {
                i += 1;
                Some(Tipo::RLave)
            }
            '[' => {
                i += 1;
                Some(Tipo::LCorchete)
            }
            ']' => {
                i += 1;
                Some(Tipo::RCorchete)
            }
            ',' => {
                i += 1;
                Some(Tipo::Coma)
            }
            ':' => {
                i += 1;
                Some(Tipo::DosPuntos)
            }
            otro => {
                let error = ErrorTraduccion::nuevo(
                    linea,
                    format!("el carácter '{}' no se reconoce", otro),
                )
                .con_codigo("E002")
                .en_columna(columna);
                return Err(if otro == ';' {
                    error
                        .con_sugerencia(
                            "En Deimon no se usa ';': cada sentencia va en su propia línea. Borra el ';'",
                        )
                        .con_arreglo(Arreglo {
                            linea,
                            columna_inicio: columna,
                            columna_fin: columna + 1,
                            reemplazo: String::new(),
                        })
                } else {
                    error.con_sugerencia(
                        "Ese carácter no existe en Deimon: bórralo o ponlo dentro de un texto entre comillas",
                    )
                });
            }
        };
        if let Some(tipo) = tipo {
            tokens.push(Token {
                tipo,
                linea,
                columna,
                fin: i - inicio_linea + 1,
            });
        }
    }

    tokens.push(Token {
        tipo: Tipo::Fin,
        linea,
        columna: caracteres.len() - inicio_linea + 1,
        fin: caracteres.len() - inicio_linea + 1,
    });
    Ok(tokens)
}

fn es_inicio_identificador(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn es_continuacion_identificador(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn reconoce_una_declaracion() {
        let tokens = tokenizar("sea x = 2.5\n").unwrap();
        let tipos: Vec<Tipo> = tokens.into_iter().map(|t| t.tipo).collect();
        assert_eq!(
            tipos,
            vec![
                Tipo::Sea,
                Tipo::Ident("x".to_string()),
                Tipo::Asignacion,
                Tipo::Numero(2.5),
                Tipo::NuevaLinea,
                Tipo::Fin,
            ]
        );
    }

    #[test]
    fn reconoce_textos_con_escapes() {
        let tokens = tokenizar(r#"mostrar("a\n\"b\"")"#).unwrap();
        assert_eq!(tokens[2].tipo, Tipo::Texto("a\n\"b\"".to_string()));
    }

    #[test]
    fn cuenta_las_lineas() {
        let tokens = tokenizar("mostrar(1)\n\nmostrar(2)\n").unwrap();
        let segunda = tokens
            .iter()
            .find(|t| t.tipo == Tipo::Numero(2.0))
            .expect("no se encontró el segundo número");
        assert_eq!(segunda.linea, 3);
    }

    #[test]
    fn informa_un_texto_sin_cerrar() {
        let error = tokenizar("mostrar(\"hola)\n").unwrap_err();
        assert_eq!(error.linea, 1);
        assert_eq!(error.mensaje, "el texto no está cerrado");
    }
}
