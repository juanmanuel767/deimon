// Mercado Pago Checkout Pro (API de preferencias): crear el pago y consultar su estado.

fn ext_mp_base() -> String {
    std::env::var("DEIMON_MERCADOPAGO_URL").unwrap_or_else(|_| "https://api.mercadopago.com".to_string()).trim_end_matches('/').to_string()
}

fn ext_mp_texto(args: &[Valor], i: usize, funcion: &str) -> Result<String, String> {
    match args.get(i).and_then(|v| v.como_texto()) {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(format!("{}() espera un texto no vacío en el argumento {}", funcion, i + 1)),
    }
}

fn ext_mp_error(codigo: u16, respuesta: &str, funcion: &str) -> String {
    let mensaje = ext_json_leer_texto(respuesta).ok().and_then(|j| match ext_json_seguir(&j, "message") {
        Some(ExtJson::Texto(t)) => Some(t.clone()),
        _ => None,
    });
    format!("{}(): Mercado Pago respondió con el código {}{}", funcion, codigo, mensaje.map(|m| format!(": {}", m)).unwrap_or_default())
}

fn ext_crear_pago_mercadopago(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "crear_pago_mercadopago";
    let token = ext_mp_texto(&args, 0, f)?;
    let descripcion = ext_mp_texto(&args, 1, f)?;
    let importe = match args.get(2) {
        Some(Valor::Numero(n)) if n.is_finite() && *n > 0.0 => (*n * 100.0).round() / 100.0,
        _ => return Err(format!("{}(): el importe debe ser un número mayor que 0", f)),
    };
    let moneda = ext_mp_texto(&args, 3, f)?.to_ascii_uppercase();
    let vuelta = ext_mp_texto(&args, 4, f)?;
    let referencia = ext_mp_texto(&args, 5, f)?;
    if !(vuelta.starts_with("https://") || vuelta.starts_with("http://")) {
        return Err(format!("{}(): la dirección de vuelta debe empezar por https://", f));
    }
    let texto = |t: &str| ExtJson::Texto(t.to_string());
    let mut cuerpo = vec![
        ("items".to_string(), ExtJson::Lista(vec![ExtJson::Objeto(vec![
            ("title".to_string(), texto(&descripcion)),
            ("quantity".to_string(), ExtJson::Numero(1.0)),
            ("unit_price".to_string(), ExtJson::Numero(importe)),
            ("currency_id".to_string(), texto(&moneda)),
        ])])),
        ("back_urls".to_string(), ExtJson::Objeto(vec![
            ("success".to_string(), texto(&vuelta)),
            ("failure".to_string(), texto(&vuelta)),
            ("pending".to_string(), texto(&vuelta)),
        ])),
        ("external_reference".to_string(), texto(&referencia)),
    ];
    // Mercado Pago solo vuelve solo a la tienda tras pagar si la dirección es https.
    if vuelta.starts_with("https://") {
        cuerpo.push(("auto_return".to_string(), texto("approved")));
    }
    let cabeceras = vec![
        format!("Authorization: Bearer {}", token),
        "Content-Type: application/json".to_string(),
        format!("X-Idempotency-Key: {}", hexadecimal(&bytes_aleatorios(16))),
    ];
    let url = format!("{}/checkout/preferences", ext_mp_base());
    let (codigo, respuesta) = ext_web_peticion("POST", &url, Some(&ext_json_escribir(&ExtJson::Objeto(cuerpo))), &cabeceras).map_err(|e| format!("{}(): {}", f, e))?;
    if codigo >= 400 {
        return Err(ext_mp_error(codigo, &respuesta, f));
    }
    let json = ext_json_leer_texto(&respuesta).map_err(|e| format!("{}(): respuesta que no se entiende: {}", f, e))?;
    // Con credenciales de prueba se paga en el entorno de pruebas.
    let campo_url = if token.starts_with("TEST-") { "sandbox_init_point" } else { "init_point" };
    match ext_json_seguir(&json, campo_url).or_else(|| ext_json_seguir(&json, "init_point")) {
        Some(ExtJson::Texto(url)) if url.starts_with("https://") || url.starts_with("http://") => Ok(Valor::texto(url.clone())),
        _ => Err(format!("{}(): Mercado Pago no devolvió la dirección de pago", f)),
    }
}

fn ext_consultar_pago_mercadopago(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "consultar_pago_mercadopago";
    let token = ext_mp_texto(&args, 0, f)?;
    let id = match args.get(1) {
        Some(Valor::Numero(n)) => texto_de(&Valor::Numero(*n)),
        Some(Valor::Texto(t)) => (**t).clone(),
        _ => String::new(),
    };
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{}(): el id del pago solo tiene cifras", f));
    }
    let url = format!("{}/v1/payments/{}", ext_mp_base(), id);
    let (codigo, respuesta) = ext_web_peticion("GET", &url, None, &[format!("Authorization: Bearer {}", token)]).map_err(|e| format!("{}(): {}", f, e))?;
    if codigo >= 400 {
        return Err(ext_mp_error(codigo, &respuesta, f));
    }
    let json = ext_json_leer_texto(&respuesta).map_err(|e| format!("{}(): respuesta que no se entiende: {}", f, e))?;
    let leer = |camino: &str| match ext_json_seguir(&json, camino) {
        Some(ExtJson::Texto(t)) => Valor::Texto(Rc::new(escapar_html(t))),
        Some(ExtJson::Numero(n)) => Valor::Numero(*n),
        _ => Valor::Nada,
    };
    Ok(Valor::mapa(vec![
        ("estado".to_string(), leer("status")),
        ("detalle".to_string(), leer("status_detail")),
        ("referencia".to_string(), leer("external_reference")),
        ("importe".to_string(), leer("transaction_amount")),
        ("moneda".to_string(), leer("currency_id")),
    ]))
}
