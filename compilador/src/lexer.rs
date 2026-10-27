use crate::error::{Arreglo, ErrorTraduccion};

#[derive(Debug, Clone, PartialEq)]
pub enum Tipo {
    Numero(f64),
    /// Entero escrito con cifras que pasa de 2^53: se guarda como texto para no perder cifras.
    EnteroGrande(String),
    Texto(String),
    /// Texto `f"..{x}.."` o de varias líneas `"""..."""`. `crudo` es el texto tal cual se
    /// escribió (lo usa el formateador); `partes` es su contenido ya separado.
    TextoEspecial {
        crudo: String,
        partes: Vec<ParteTexto>,
    },
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

/// Trozo de un texto especial: letras fijas o una expresión entre `{ }` ya convertida en tokens
/// (terminan en `Fin`, con líneas y columnas del programa original).
#[derive(Debug, Clone, PartialEq)]
pub enum ParteTexto {
    Literal(String),
    Valor(Vec<Token>),
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
            '"' | 'f' if (c == 'f' && siguiente == Some('"'))
                || (c == '"' && siguiente == Some('"') && tercero == Some('"')) =>
            {
                let con_valores = c == 'f';
                let desde = if con_valores { i + 1 } else { i };
                let leido = leer_texto_especial(&caracteres, desde, con_valores, columna, linea, inicio_linea)?;
                let crudo: String = caracteres[inicio..leido.fin].iter().collect();
                tokens.push(Token {
                    tipo: Tipo::TextoEspecial {
                        crudo,
                        partes: leido.partes,
                    },
                    linea,
                    columna,
                    fin: leido.fin - leido.inicio_linea + 1,
                });
                i = leido.fin;
                linea = leido.linea;
                inicio_linea = leido.inicio_linea;
                None
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
                let cifras = texto.trim_start_matches('0');
                let es_grande = texto.chars().all(|c| c.is_ascii_digit())
                    && (cifras.len() > 16 || (cifras.len() == 16 && cifras > "9007199254740992"));
                if es_grande {
                    Some(Tipo::EnteroGrande(cifras.to_string()))
                } else {
                    let numero = texto.parse::<f64>().map_err(|_| {
                        ErrorTraduccion::nuevo(linea, format!("el número '{}' no es válido", texto))
                            .con_codigo("E002")
                            .en_columna(columna)
                    })?;
                    Some(Tipo::Numero(numero))
                }
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
    resolver_y_o(&mut tokens);
    Ok(tokens)
}

/// `y` y `o` son palabras contextuales: solo son el operador lógico justo después de un
/// operando completo (en la misma línea); en cualquier otro sitio son un nombre normal.
fn resolver_y_o(tokens: &mut [Token]) {
    let mut anterior: Option<usize> = None;
    for i in 0..tokens.len() {
        if matches!(tokens[i].tipo, Tipo::Comentario(_)) {
            continue;
        }
        if matches!(tokens[i].tipo, Tipo::Y | Tipo::O) {
            let tras_operando = anterior.is_some_and(|a| {
                matches!(
                    tokens[a].tipo,
                    Tipo::Numero(_)
                        | Tipo::EnteroGrande(_)
                        | Tipo::Texto(_)
                        | Tipo::TextoEspecial { .. }
                        | Tipo::Ident(_)
                        | Tipo::RParen
                        | Tipo::RCorchete
                        | Tipo::RLave
                        | Tipo::Verdadero
                        | Tipo::Falso
                        | Tipo::Nada
                )
            });
            if !tras_operando {
                let nombre = if tokens[i].tipo == Tipo::Y { "y" } else { "o" };
                tokens[i].tipo = Tipo::Ident(nombre.to_string());
            }
        }
        anterior = Some(i);
    }
}

struct TextoLeido {
    partes: Vec<ParteTexto>,
    /// Posición justo después de la comilla de cierre.
    fin: usize,
    linea: usize,
    inicio_linea: usize,
}

enum Cierre {
    Cierre(usize),
    SinCerrar,
    LlaveSinCerrar(usize),
}

/// Busca el `}` que cierra la expresión que empieza en `desde` (justo después de la `{`),
/// equilibrando ( [ { y saltando los textos internos con sus escapes.
fn fin_de_expresion(caracteres: &[char], desde: usize, permite_saltos: bool) -> Option<usize> {
    let mut i = desde;
    let mut profundidad = 0usize;
    while i < caracteres.len() {
        match caracteres[i] {
            '\n' if !permite_saltos => return None,
            '(' | '[' | '{' => profundidad += 1,
            ')' | ']' => profundidad = profundidad.saturating_sub(1),
            '}' => {
                if profundidad == 0 {
                    return Some(i);
                }
                profundidad -= 1;
            }
            '"' => {
                if caracteres.get(i + 1) == Some(&'"') && caracteres.get(i + 2) == Some(&'"') {
                    i += 3;
                    while i < caracteres.len() {
                        if caracteres[i] == '\\' {
                            i += 1;
                        } else if caracteres[i] == '"'
                            && caracteres.get(i + 1) == Some(&'"')
                            && caracteres.get(i + 2) == Some(&'"')
                        {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                    loop {
                        match caracteres.get(i) {
                            None | Some('\n') => return None,
                            Some('\\') => i += 1,
                            Some('"') => break,
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Busca el final de un texto especial que empieza en la comilla `desde`.
/// Devuelve la posición de la comilla de cierre (la primera de las tres si es de varias líneas).
fn buscar_cierre(caracteres: &[char], desde: usize, con_valores: bool, triple: bool) -> Cierre {
    let mut i = desde + if triple { 3 } else { 1 };
    while i < caracteres.len() {
        match caracteres[i] {
            '"' if !triple => return Cierre::Cierre(i),
            '"' if caracteres.get(i + 1) == Some(&'"') && caracteres.get(i + 2) == Some(&'"') => {
                return Cierre::Cierre(i)
            }
            '\n' if !triple => return Cierre::SinCerrar,
            '\\' => i += 1,
            '{' if con_valores => {
                if caracteres.get(i + 1) == Some(&'{') {
                    i += 1;
                } else {
                    match fin_de_expresion(caracteres, i + 1, triple) {
                        Some(fin) => i = fin,
                        None => return Cierre::LlaveSinCerrar(i),
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    Cierre::SinCerrar
}

/// Lee un texto `f"..."`, `"""..."""` o `f"""..."""`. `desde` es la posición de la primera
/// comilla; `columna`, `linea` e `inicio_linea` son los del principio del texto (con la `f`).
fn leer_texto_especial(
    caracteres: &[char],
    desde: usize,
    con_valores: bool,
    columna: usize,
    linea: usize,
    inicio_linea: usize,
) -> Result<TextoLeido, ErrorTraduccion> {
    let triple = caracteres.get(desde + 1) == Some(&'"') && caracteres.get(desde + 2) == Some(&'"');
    // Línea y columna (desde 1) de una posición cualquiera del texto.
    let situar = |posicion: usize| {
        let mut l = linea;
        let mut ini = inicio_linea;
        for (k, c) in caracteres[..posicion].iter().enumerate().skip(desde) {
            if *c == '\n' {
                l += 1;
                ini = k + 1;
            }
        }
        (l, ini)
    };
    let error_llave = |posicion: usize, mensaje: &str, sugerencia: &str| {
        let (l, ini) = situar(posicion);
        let c = posicion - ini + 1;
        ErrorTraduccion::nuevo(l, mensaje)
            .con_codigo("E028")
            .en_columna(c)
            .con_sugerencia(sugerencia)
    };
    let cierre = match buscar_cierre(caracteres, desde, con_valores, triple) {
        Cierre::Cierre(cierre) => cierre,
        Cierre::SinCerrar => {
            return Err(if triple {
                ErrorTraduccion::nuevo(linea, "el texto de varias líneas no está cerrado")
                    .con_codigo("E001")
                    .en_columna(columna)
                    .con_sugerencia(format!(
                        "El texto que empieza en la columna {} de la línea {} necesita tres comillas dobles (\"\"\") de cierre",
                        columna, linea
                    ))
            } else {
                ErrorTraduccion::nuevo(linea, "el texto no está cerrado")
                    .con_codigo("E001")
                    .en_columna(columna)
                    .con_sugerencia(format!(
                        "El texto que empieza en la columna {} de la línea {} necesita unas comillas dobles (\") de cierre en la misma línea",
                        columna, linea
                    ))
            });
        }
        Cierre::LlaveSinCerrar(posicion) => {
            return Err(error_llave(
                posicion,
                "la llave '{' del texto no está cerrada",
                "Cierra la llave con '}'. Si querías escribir una llave normal, ponla doble: {{",
            ));
        }
    };
    let mut inicio = desde + if triple { 3 } else { 1 };
    let mut fin_cuerpo = cierre;
    let mut indentacion = 0;
    if triple {
        // Un salto de línea justo después de las comillas de apertura no cuenta.
        let salto = if caracteres.get(inicio) == Some(&'\n') {
            Some(1)
        } else if caracteres.get(inicio) == Some(&'\r') && caracteres.get(inicio + 1) == Some(&'\n') {
            Some(2)
        } else {
            None
        };
        let con_salto_inicial = salto.is_some();
        if let Some(largo) = salto {
            inicio += largo;
        }
        // Una última línea solo con espacios (y su salto de línea) tampoco cuenta.
        let cuerpo = &caracteres[inicio..fin_cuerpo];
        let ultimo_salto = cuerpo.iter().rposition(|c| *c == '\n');
        let resto = match ultimo_salto {
            Some(posicion) => Some(posicion + 1),
            None if con_salto_inicial => Some(0),
            None => None,
        };
        if let Some(posicion) = resto {
            if cuerpo[posicion..].iter().all(|c| *c == ' ' || *c == '\t') {
                fin_cuerpo = match ultimo_salto {
                    Some(salto) if salto > 0 && cuerpo[salto - 1] == '\r' => inicio + salto - 1,
                    Some(salto) => inicio + salto,
                    None => inicio,
                };
            }
        }
        // Sangría común mínima de las líneas que no están vacías.
        let cuerpo: String = caracteres[inicio..fin_cuerpo].iter().collect();
        indentacion = cuerpo
            .split('\n')
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count())
            .min()
            .unwrap_or(0);
    }

    let (mut l, mut ini) = situar(inicio);
    let mut j = inicio;
    let mut partes = Vec::new();
    let mut literal = String::new();
    while j < fin_cuerpo {
        if triple && (j == inicio || caracteres[j - 1] == '\n') {
            let mut quitados = 0;
            while quitados < indentacion
                && j < fin_cuerpo
                && (caracteres[j] == ' ' || caracteres[j] == '\t')
            {
                j += 1;
                quitados += 1;
            }
            if j >= fin_cuerpo {
                break;
            }
        }
        match caracteres[j] {
            '\\' => {
                j += 1;
                match caracteres.get(j) {
                    Some('n') => literal.push('\n'),
                    Some('t') => literal.push('\t'),
                    Some('"') => literal.push('"'),
                    Some('\\') => literal.push('\\'),
                    otro => {
                        let posicion = j.min(caracteres.len());
                        let (el, eini) = situar(posicion);
                        let ec = posicion - eini + 1;
                        return Err(ErrorTraduccion::nuevo(
                            el,
                            format!(
                                "el escape '\\{}' no existe",
                                otro.map(|c| c.to_string()).unwrap_or_default()
                            ),
                        )
                        .con_codigo("E002")
                        .en_columna(ec.saturating_sub(1))
                        .con_sugerencia("Los escapes válidos son \\n, \\t, \\\" y \\\\"));
                    }
                }
                j += 1;
            }
            '{' if con_valores => {
                if caracteres.get(j + 1) == Some(&'{') {
                    literal.push('{');
                    j += 2;
                    continue;
                }
                let fin = fin_de_expresion(caracteres, j + 1, triple).unwrap_or(fin_cuerpo);
                let fuente: String = caracteres[j + 1..fin].iter().collect();
                if fuente.trim().is_empty() {
                    return Err(error_llave(
                        j,
                        "hay unas llaves vacías ({}) dentro del texto",
                        "Escribe dentro de las llaves lo que quieres mostrar, por ejemplo {nombre}. Si querías llaves normales, ponlas dobles: {{ }}",
                    ));
                }
                if !literal.is_empty() {
                    partes.push(ParteTexto::Literal(std::mem::take(&mut literal)));
                }
                let (linea_valor, ini_valor) = situar(j + 1);
                let columna_valor = j + 1 - ini_valor + 1;
                partes.push(ParteTexto::Valor(tokens_de_valor(&fuente, linea_valor, columna_valor)?));
                for (k, c) in caracteres[j + 1..fin].iter().enumerate() {
                    if *c == '\n' {
                        l += 1;
                        ini = j + 1 + k + 1;
                    }
                }
                j = fin + 1;
            }
            '}' if con_valores => {
                if caracteres.get(j + 1) == Some(&'}') {
                    literal.push('}');
                    j += 2;
                } else {
                    return Err(error_llave(
                        j,
                        "hay una '}' suelta dentro del texto",
                        "Si querías escribir una llave normal, ponla doble: }}. Si no, bórrala",
                    ));
                }
            }
            otro => {
                if otro == '\n' {
                    l += 1;
                    ini = j + 1;
                }
                literal.push(otro);
                j += 1;
            }
        }
    }
    // Las líneas del texto que quedaron sin recorrer (última línea quitada y cierre).
    let (l_fin, ini_fin) = {
        let (mut le, mut ie) = (l, ini);
        for (k, c) in caracteres[j.min(cierre)..cierre].iter().enumerate() {
            if *c == '\n' {
                le += 1;
                ie = j.min(cierre) + k + 1;
            }
        }
        (le, ie)
    };
    if !literal.is_empty() || partes.is_empty() {
        partes.push(ParteTexto::Literal(literal));
    }
    Ok(TextoLeido {
        partes,
        fin: cierre + if triple { 3 } else { 1 },
        linea: l_fin,
        inicio_linea: ini_fin,
    })
}

/// Convierte en tokens el trozo de programa que va entre `{ }`, dejando sus líneas y
/// columnas como en el programa original. Los saltos de línea se descartan.
fn tokens_de_valor(
    fuente: &str,
    linea: usize,
    columna: usize,
) -> Result<Vec<Token>, ErrorTraduccion> {
    let ajustar_columna = |l: usize, c: usize| if l == 1 && c > 0 { c + columna - 1 } else { c };
    let mut tokens = tokenizar_interno(fuente, false).map_err(|mut error| {
        if let Some(arreglo) = error.arreglo.as_mut() {
            arreglo.columna_inicio = ajustar_columna(arreglo.linea, arreglo.columna_inicio);
            arreglo.columna_fin = ajustar_columna(arreglo.linea, arreglo.columna_fin);
            arreglo.linea += linea - 1;
        }
        error.columna = ajustar_columna(error.linea, error.columna);
        error.linea += linea - 1;
        error
    })?;
    tokens.retain(|t| t.tipo != Tipo::NuevaLinea);
    for token in tokens.iter_mut() {
        token.columna = ajustar_columna(token.linea, token.columna);
        token.fin = ajustar_columna(token.linea, token.fin);
        token.linea += linea - 1;
    }
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
