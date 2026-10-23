fn ext_pagos_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice).and_then(|valor| valor.como_texto()) {
        Some(texto) => Ok(texto.to_string()),
        None => Err(format!("{}() espera un texto en el argumento {}", funcion, indice + 1)),
    }
}

/// Codifica un valor para un formulario (application/x-www-form-urlencoded).
fn ext_pagos_codificar(texto: &str) -> String {
    let mut salida = String::new();
    for byte in texto.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => salida.push(byte as char),
            _ => salida.push_str(&format!("%{:02X}", byte)),
        }
    }
    salida
}

/// Saca el valor de texto de una clave JSON ("clave": "valor"), sin librerías.
fn ext_pagos_json(json: &str, clave: &str) -> Option<String> {
    let marca = format!("\"{}\"", clave);
    let mut resto = json;
    while let Some(posicion) = resto.find(&marca) {
        resto = &resto[posicion + marca.len()..];
        let tras = resto.trim_start();
        if let Some(valor) = tras.strip_prefix(':') {
            let valor = valor.trim_start();
            if let Some(cadena) = valor.strip_prefix('"') {
                let mut salida = String::new();
                let mut caracteres = cadena.chars();
                while let Some(c) = caracteres.next() {
                    match c {
                        '"' => return Some(salida),
                        '\\' => match caracteres.next() {
                            Some('n') => salida.push('\n'),
                            Some('t') => salida.push('\t'),
                            Some(otro) => salida.push(otro),
                            None => return None,
                        },
                        otro => salida.push(otro),
                    }
                }
                return None;
            }
            return Some(valor.split([',', '}']).next().unwrap_or("").trim().to_string());
        }
    }
    None
}

fn ext_pagos_base() -> String {
    std::env::var("DEIMON_STRIPE_URL").unwrap_or_else(|_| "https://api.stripe.com".to_string())
}

fn ext_pagos_clave(clave: &str, funcion: &str) -> Result<String, String> {
    if !(clave.starts_with("sk_") || clave.starts_with("rk_")) || clave.contains(['\r', '\n', '"']) {
        return Err(format!("{}(): la clave secreta de Stripe empieza por sk_ (o rk_); nunca uses la clave pública pk_", funcion));
    }
    Ok(format!("Authorization: Bearer {}", clave))
}

fn ext_pagos_error(cuerpo: &str, codigo: u16, funcion: &str) -> String {
    match ext_pagos_json(cuerpo, "message") {
        Some(mensaje) => format!("{}(): Stripe dice: {}", funcion, mensaje),
        None => format!("{}(): Stripe respondió con el código {}", funcion, codigo),
    }
}

fn ext_crear_pago(args: Vec<Valor>) -> Result<Valor, String> {
    let autorizacion = ext_pagos_clave(&ext_pagos_texto(&args, 0, "crear_pago")?, "crear_pago")?;
    let centimos = match args.get(1).and_then(|valor| valor.como_numero()) {
        Some(n) if n >= 1.0 && n.fract() == 0.0 => n as u64,
        _ => return Err("crear_pago(): la cantidad va en céntimos, como número entero mayor que 0 (15 € = 1500)".to_string()),
    };
    let moneda = ext_pagos_texto(&args, 2, "crear_pago")?.to_lowercase();
    if moneda.len() != 3 || !moneda.chars().all(|c| c.is_ascii_lowercase()) {
        return Err("crear_pago(): la moneda son tres letras, por ejemplo \"eur\", \"usd\" o \"mxn\"".to_string());
    }
    let descripcion = ext_pagos_texto(&args, 3, "crear_pago")?;
    let exito = ext_pagos_texto(&args, 4, "crear_pago")?;
    let cancelar = ext_pagos_texto(&args, 5, "crear_pago")?;
    for direccion in [&exito, &cancelar] {
        if !(direccion.starts_with("https://") || direccion.starts_with("http://")) {
            return Err(format!("crear_pago(): '{}' debe ser una dirección completa (https://...)", direccion));
        }
    }
    let separador = if exito.contains('?') { '&' } else { '?' };
    let exito = format!("{}{}sesion={{CHECKOUT_SESSION_ID}}", exito, separador);
    let datos = [
        ("mode", "payment".to_string()),
        ("success_url", exito),
        ("cancel_url", cancelar),
        ("line_items[0][quantity]", "1".to_string()),
        ("line_items[0][price_data][currency]", moneda),
        ("line_items[0][price_data][unit_amount]", centimos.to_string()),
        ("line_items[0][price_data][product_data][name]", descripcion),
    ]
    .iter()
    .map(|(clave, valor)| format!("{}={}", ext_pagos_codificar(clave), ext_pagos_codificar(valor)))
    .collect::<Vec<_>>()
    .join("&");
    let url = format!("{}/v1/checkout/sessions", ext_pagos_base());
    let (codigo, cuerpo) = ext_web_peticion("POST", &url, Some(&datos), &[autorizacion])
        .map_err(|e| format!("crear_pago(): {}", e))?;
    if codigo >= 400 {
        return Err(ext_pagos_error(&cuerpo, codigo, "crear_pago"));
    }
    match ext_pagos_json(&cuerpo, "url") {
        Some(direccion) if direccion.starts_with("http") => Ok(Valor::texto(direccion)),
        _ => Err("crear_pago(): Stripe no devolvió la dirección de pago".to_string()),
    }
}

fn ext_pago_completado(args: Vec<Valor>) -> Result<Valor, String> {
    let autorizacion = ext_pagos_clave(&ext_pagos_texto(&args, 0, "pago_completado")?, "pago_completado")?;
    let sesion = ext_pagos_texto(&args, 1, "pago_completado")?;
    if sesion.is_empty() || !sesion.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Ok(Valor::Logico(false));
    }
    let url = format!("{}/v1/checkout/sessions/{}", ext_pagos_base(), sesion);
    let (codigo, cuerpo) = ext_web_peticion("GET", &url, None, &[autorizacion])
        .map_err(|e| format!("pago_completado(): {}", e))?;
    if codigo == 404 {
        return Ok(Valor::Logico(false));
    }
    if codigo >= 400 {
        return Err(ext_pagos_error(&cuerpo, codigo, "pago_completado"));
    }
    Ok(Valor::Logico(ext_pagos_json(&cuerpo, "payment_status").as_deref() == Some("paid")))
}
