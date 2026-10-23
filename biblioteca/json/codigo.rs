#[derive(Clone)]
enum ExtJson {
    Nulo,
    Logico(bool),
    Numero(f64),
    Texto(String),
    Lista(Vec<ExtJson>),
    Objeto(Vec<(String, ExtJson)>),
}

fn ext_json_escapar(texto: &str) -> String {
    let mut salida = String::from("\"");
    for c in texto.chars() {
        match c {
            '"' => salida.push_str("\\\""),
            '\\' => salida.push_str("\\\\"),
            '\n' => salida.push_str("\\n"),
            '\r' => salida.push_str("\\r"),
            '\t' => salida.push_str("\\t"),
            c if (c as u32) < 0x20 => salida.push_str(&format!("\\u{:04x}", c as u32)),
            otro => salida.push(otro),
        }
    }
    salida.push('"');
    salida
}

fn ext_json_escribir(j: &ExtJson) -> String {
    match j {
        ExtJson::Nulo => "null".to_string(),
        ExtJson::Logico(b) => b.to_string(),
        ExtJson::Numero(n) => texto_de(&Valor::Numero(*n)),
        ExtJson::Texto(t) => ext_json_escapar(t),
        ExtJson::Lista(l) => format!("[{}]", l.iter().map(ext_json_escribir).collect::<Vec<_>>().join(",")),
        ExtJson::Objeto(o) => format!(
            "{{{}}}",
            o.iter().map(|(k, v)| format!("{}:{}", ext_json_escapar(k), ext_json_escribir(v))).collect::<Vec<_>>().join(",")
        ),
    }
}

fn ext_json_desde_valor(valor: &Valor) -> ExtJson {
    if let Some(n) = valor.como_numero() {
        return ExtJson::Numero(n);
    }
    if let Some(t) = valor.como_texto() {
        return ExtJson::Texto(t.to_string());
    }
    if let Some(l) = valor.como_lista() {
        return ExtJson::Lista(l.iter().map(ext_json_desde_valor).collect());
    }
    if let Some(pares) = valor.como_mapa() {
        return ExtJson::Objeto(pares.iter().map(|(c, v)| (c.clone(), ext_json_desde_valor(v))).collect());
    }
    match texto_de(valor).as_str() {
        "verdadero" => ExtJson::Logico(true),
        "falso" => ExtJson::Logico(false),
        _ => ExtJson::Nulo,
    }
}

fn ext_json_a_valor(j: &ExtJson) -> Valor {
    match j {
        ExtJson::Nulo => Valor::Nada,
        ExtJson::Logico(b) => Valor::Logico(*b),
        ExtJson::Numero(n) => Valor::Numero(*n),
        ExtJson::Texto(t) => Valor::texto(t.clone()),
        ExtJson::Lista(l) => Valor::lista(l.iter().map(ext_json_a_valor).collect()),
        ExtJson::Objeto(_) => Valor::texto(ext_json_escribir(j)),
    }
}

struct ExtJsonLector<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ExtJsonLector<'a> {
    fn espacios(&mut self) {
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn error(&self, que: &str) -> String {
        format!("el JSON no es válido cerca del carácter {}: {}", self.pos + 1, que)
    }
    fn valor(&mut self, profundidad: usize) -> Result<ExtJson, String> {
        if profundidad > 200 {
            return Err(self.error("demasiados niveles anidados"));
        }
        self.espacios();
        match self.bytes.get(self.pos) {
            Some(b'{') => {
                self.pos += 1;
                let mut pares = Vec::new();
                self.espacios();
                if self.bytes.get(self.pos) == Some(&b'}') {
                    self.pos += 1;
                    return Ok(ExtJson::Objeto(pares));
                }
                loop {
                    self.espacios();
                    let clave = match self.valor(profundidad + 1)? {
                        ExtJson::Texto(t) => t,
                        _ => return Err(self.error("las claves van entre comillas")),
                    };
                    self.espacios();
                    if self.bytes.get(self.pos) != Some(&b':') {
                        return Err(self.error("falta ':'"));
                    }
                    self.pos += 1;
                    let valor = self.valor(profundidad + 1)?;
                    pares.push((clave, valor));
                    self.espacios();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(ExtJson::Objeto(pares));
                        }
                        _ => return Err(self.error("falta ',' o '}'")),
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                let mut elementos = Vec::new();
                self.espacios();
                if self.bytes.get(self.pos) == Some(&b']') {
                    self.pos += 1;
                    return Ok(ExtJson::Lista(elementos));
                }
                loop {
                    elementos.push(self.valor(profundidad + 1)?);
                    self.espacios();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(ExtJson::Lista(elementos));
                        }
                        _ => return Err(self.error("falta ',' o ']'")),
                    }
                }
            }
            Some(b'"') => {
                self.pos += 1;
                let mut bytes = Vec::new();
                loop {
                    match self.bytes.get(self.pos) {
                        None => return Err(self.error("texto sin cerrar")),
                        Some(b'"') => {
                            self.pos += 1;
                            return Ok(ExtJson::Texto(String::from_utf8_lossy(&bytes).into_owned()));
                        }
                        Some(b'\\') => {
                            let escape = *self.bytes.get(self.pos + 1).ok_or_else(|| self.error("escape sin terminar"))?;
                            self.pos += 2;
                            let c = match escape {
                                b'n' => '\n',
                                b't' => '\t',
                                b'r' => '\r',
                                b'b' => '\u{8}',
                                b'f' => '\u{c}',
                                b'/' => '/',
                                b'\\' => '\\',
                                b'"' => '"',
                                b'u' => {
                                    let hex = std::str::from_utf8(self.bytes.get(self.pos..self.pos + 4).ok_or_else(|| self.error("\\u incompleto"))?)
                                        .map_err(|_| self.error("\\u no válido"))?;
                                    let mut codigo = u32::from_str_radix(hex, 16).map_err(|_| self.error("\\u no válido"))?;
                                    self.pos += 4;
                                    if (0xD800..0xDC00).contains(&codigo) && self.bytes.get(self.pos..self.pos + 2) == Some(b"\\u") {
                                        let bajo = std::str::from_utf8(&self.bytes[self.pos + 2..(self.pos + 6).min(self.bytes.len())]).ok().and_then(|h| u32::from_str_radix(h, 16).ok()).unwrap_or(0);
                                        if (0xDC00..0xE000).contains(&bajo) {
                                            codigo = 0x10000 + ((codigo - 0xD800) << 10) + (bajo - 0xDC00);
                                            self.pos += 6;
                                        }
                                    }
                                    char::from_u32(codigo).unwrap_or('\u{FFFD}')
                                }
                                _ => return Err(self.error("escape desconocido")),
                            };
                            let mut buffer = [0u8; 4];
                            bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
                        }
                        Some(&b) => {
                            bytes.push(b);
                            self.pos += 1;
                        }
                    }
                }
            }
            Some(b't') if self.bytes[self.pos..].starts_with(b"true") => {
                self.pos += 4;
                Ok(ExtJson::Logico(true))
            }
            Some(b'f') if self.bytes[self.pos..].starts_with(b"false") => {
                self.pos += 5;
                Ok(ExtJson::Logico(false))
            }
            Some(b'n') if self.bytes[self.pos..].starts_with(b"null") => {
                self.pos += 4;
                Ok(ExtJson::Nulo)
            }
            Some(b) if *b == b'-' || b.is_ascii_digit() => {
                let inicio = self.pos;
                while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                    self.pos += 1;
                }
                std::str::from_utf8(&self.bytes[inicio..self.pos])
                    .ok()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(ExtJson::Numero)
                    .ok_or_else(|| self.error("número no válido"))
            }
            _ => Err(self.error("se esperaba un valor")),
        }
    }
}

fn ext_json_leer_texto(texto: &str) -> Result<ExtJson, String> {
    let mut lector = ExtJsonLector { bytes: texto.as_bytes(), pos: 0 };
    let valor = lector.valor(0)?;
    lector.espacios();
    if lector.pos != lector.bytes.len() {
        return Err(lector.error("sobra texto al final"));
    }
    Ok(valor)
}

/// Sigue un camino como "usuario.direcciones[0].ciudad".
fn ext_json_seguir<'a>(raiz: &'a ExtJson, camino: &str) -> Option<&'a ExtJson> {
    let mut actual = raiz;
    let mut resto = camino.trim();
    while !resto.is_empty() {
        if let Some(despues) = resto.strip_prefix('.') {
            resto = despues;
            continue;
        }
        if let Some(despues) = resto.strip_prefix('[') {
            let (indice, otra) = despues.split_once(']')?;
            let indice: usize = indice.trim().parse().ok()?;
            actual = match actual {
                ExtJson::Lista(l) => l.get(indice)?,
                _ => return None,
            };
            resto = otra;
            continue;
        }
        let fin = resto.find(['.', '[']).unwrap_or(resto.len());
        let clave = &resto[..fin];
        actual = match actual {
            ExtJson::Objeto(pares) => &pares.iter().find(|(k, _)| k == clave)?.1,
            _ => return None,
        };
        resto = &resto[fin..];
    }
    Some(actual)
}

fn ext_json_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice).and_then(|valor| valor.como_texto()) {
        Some(texto) => Ok(texto.to_string()),
        None => Err(format!("{}() espera un texto en el argumento {}", funcion, indice + 1)),
    }
}

fn ext_a_json(args: Vec<Valor>) -> Result<Valor, String> {
    let valor = args.first().cloned().unwrap_or(Valor::Nada);
    Ok(Valor::texto(ext_json_escribir(&ext_json_desde_valor(&valor))))
}

fn ext_leer_json(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_json_texto(&args, 0, "leer_json")?;
    let camino = ext_json_texto(&args, 1, "leer_json")?;
    let raiz = ext_json_leer_texto(&texto).map_err(|e| format!("leer_json(): {}", e))?;
    Ok(ext_json_seguir(&raiz, &camino).map(ext_json_a_valor).unwrap_or(Valor::Nada))
}

fn ext_listar_claves_json(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_json_texto(&args, 0, "listar_claves_json")?;
    let camino = ext_json_texto(&args, 1, "listar_claves_json")?;
    let raiz = ext_json_leer_texto(&texto).map_err(|e| format!("listar_claves_json(): {}", e))?;
    match ext_json_seguir(&raiz, &camino) {
        Some(ExtJson::Objeto(pares)) => Ok(Valor::lista(pares.iter().map(|(k, _)| Valor::texto(k.clone())).collect())),
        Some(_) => Err(format!("listar_claves_json(): en '{}' no hay un objeto", camino)),
        None => Ok(Valor::lista(Vec::new())),
    }
}

/// Convierte un texto JSON completo en valores de Deimon: los objetos pasan a ser
/// listas con claves y los arrays, listas.
fn ext_json_a_valor_completo(j: &ExtJson) -> Valor {
    match j {
        ExtJson::Lista(l) => Valor::lista(l.iter().map(ext_json_a_valor_completo).collect()),
        ExtJson::Objeto(pares) => Valor::mapa(pares.iter().map(|(c, v)| (c.clone(), ext_json_a_valor_completo(v))).collect()),
        otro => ext_json_a_valor(otro),
    }
}

fn ext_desde_json(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_json_texto(&args, 0, "desde_json")?;
    let raiz = ext_json_leer_texto(&texto).map_err(|e| format!("desde_json(): {}", e))?;
    Ok(ext_json_a_valor_completo(&raiz))
}
