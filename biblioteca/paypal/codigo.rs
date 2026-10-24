// PayPal Orders v2: token OAuth2, crear el pedido y cobrarlo (capture).

fn ext_pp_base() -> String {
    std::env::var("DEIMON_PAYPAL_URL").unwrap_or_else(|_| "https://api-m.paypal.com".to_string()).trim_end_matches('/').to_string()
}

fn ext_pp_texto(args: &[Valor], i: usize, funcion: &str) -> Result<String, String> {
    match args.get(i).and_then(|v| v.como_texto()) {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(format!("{}() espera un texto no vacío en el argumento {}", funcion, i + 1)),
    }
}

fn ext_pp_base64(datos: &[u8]) -> String {
    const LETRAS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut salida = String::new();
    for trozo in datos.chunks(3) {
        let n = (trozo[0] as u32) << 16 | (*trozo.get(1).unwrap_or(&0) as u32) << 8 | *trozo.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= trozo.len() {
                salida.push(LETRAS[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                salida.push('=');
            }
        }
    }
    salida
}

fn ext_pp_token(cliente: &str, secreto: &str, funcion: &str) -> Result<String, String> {
    let cabeceras = vec![
        format!("Authorization: Basic {}", ext_pp_base64(format!("{}:{}", cliente, secreto).as_bytes())),
        "Content-Type: application/x-www-form-urlencoded".to_string(),
    ];
    let url = format!("{}/v1/oauth2/token", ext_pp_base());
    let (codigo, respuesta) = ext_web_peticion("POST", &url, Some("grant_type=client_credentials"), &cabeceras).map_err(|e| format!("{}(): {}", funcion, e))?;
    if codigo >= 400 {
        return Err(format!("{}(): PayPal no aceptó el client id y el secret (código {})", funcion, codigo));
    }
    match ext_json_leer_texto(&respuesta).ok().as_ref().and_then(|j| ext_json_seguir(j, "access_token")) {
        Some(ExtJson::Texto(t)) => Ok(t.clone()),
        _ => Err(format!("{}(): PayPal no devolvió el token de acceso", funcion)),
    }
}

fn ext_crear_pago_paypal(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "crear_pago_paypal";
    let (cliente, secreto) = (ext_pp_texto(&args, 0, f)?, ext_pp_texto(&args, 1, f)?);
    let importe = match args.get(2) {
        Some(Valor::Numero(n)) if n.is_finite() && *n > 0.0 => *n,
        _ => return Err(format!("{}(): el importe debe ser un número mayor que 0", f)),
    };
    let moneda = ext_pp_texto(&args, 3, f)?.to_ascii_uppercase();
    let (vuelta, cancelar) = (ext_pp_texto(&args, 4, f)?, ext_pp_texto(&args, 5, f)?);
    // Estas monedas no llevan decimales en PayPal.
    let valor = if ["HUF", "JPY", "TWD"].contains(&moneda.as_str()) { format!("{:.0}", importe) } else { format!("{:.2}", importe) };
    let texto = |t: &str| ExtJson::Texto(t.to_string());
    let cuerpo = ExtJson::Objeto(vec![
        ("intent".to_string(), texto("CAPTURE")),
        ("purchase_units".to_string(), ExtJson::Lista(vec![ExtJson::Objeto(vec![(
            "amount".to_string(),
            ExtJson::Objeto(vec![("currency_code".to_string(), texto(&moneda)), ("value".to_string(), texto(&valor))]),
        )])])),
        ("payment_source".to_string(), ExtJson::Objeto(vec![(
            "paypal".to_string(),
            ExtJson::Objeto(vec![(
                "experience_context".to_string(),
                ExtJson::Objeto(vec![
                    ("return_url".to_string(), texto(&vuelta)),
                    ("cancel_url".to_string(), texto(&cancelar)),
                    ("user_action".to_string(), texto("PAY_NOW")),
                ]),
            )]),
        )])),
    ]);
    let token = ext_pp_token(&cliente, &secreto, f)?;
    let cabeceras = vec![
        format!("Authorization: Bearer {}", token),
        "Content-Type: application/json".to_string(),
        format!("PayPal-Request-Id: {}", hexadecimal(&bytes_aleatorios(16))),
    ];
    let url = format!("{}/v2/checkout/orders", ext_pp_base());
    let (codigo, respuesta) = ext_web_peticion("POST", &url, Some(&ext_json_escribir(&cuerpo)), &cabeceras).map_err(|e| format!("{}(): {}", f, e))?;
    if codigo >= 400 {
        return Err(format!("{}(): PayPal respondió con el código {}", f, codigo));
    }
    let json = ext_json_leer_texto(&respuesta).map_err(|e| format!("{}(): respuesta que no se entiende: {}", f, e))?;
    if let Some(ExtJson::Lista(enlaces)) = ext_json_seguir(&json, "links") {
        for enlace in enlaces {
            let rel = match ext_json_seguir(enlace, "rel") { Some(ExtJson::Texto(t)) => t.as_str(), _ => "" };
            if rel == "payer-action" || rel == "approve" {
                if let Some(ExtJson::Texto(url)) = ext_json_seguir(enlace, "href") {
                    return Ok(Valor::texto(url.clone()));
                }
            }
        }
    }
    Err(format!("{}(): PayPal no devolvió la dirección para aprobar el pago", f))
}

fn ext_cobrar_pago_paypal(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "cobrar_pago_paypal";
    let (cliente, secreto, pedido) = (ext_pp_texto(&args, 0, f)?, ext_pp_texto(&args, 1, f)?, ext_pp_texto(&args, 2, f)?);
    if pedido.len() > 64 || !pedido.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("{}(): '{}' no es un id de pedido de PayPal", f, escapar_html(&pedido)));
    }
    let token = ext_pp_token(&cliente, &secreto, f)?;
    let cabeceras = vec![
        format!("Authorization: Bearer {}", token),
        "Content-Type: application/json".to_string(),
        format!("PayPal-Request-Id: cobro-{}", pedido),
    ];
    let estado = |respuesta: &str| match ext_json_leer_texto(respuesta).ok().as_ref().and_then(|j| ext_json_seguir(j, "status")) {
        Some(ExtJson::Texto(t)) => t.clone(),
        _ => String::new(),
    };
    let url = format!("{}/v2/checkout/orders/{}/capture", ext_pp_base(), pedido);
    let (codigo, respuesta) = ext_web_peticion("POST", &url, Some("{}"), &cabeceras).map_err(|e| format!("{}(): {}", f, e))?;
    if codigo < 400 {
        return Ok(Valor::Logico(estado(&respuesta) == "COMPLETED"));
    }
    // Si ya se había cobrado (o no se pudo), se mira cómo está el pedido.
    let url = format!("{}/v2/checkout/orders/{}", ext_pp_base(), pedido);
    let (codigo, respuesta) = ext_web_peticion("GET", &url, None, &cabeceras[..1]).map_err(|e| format!("{}(): {}", f, e))?;
    Ok(Valor::Logico(codigo < 400 && estado(&respuesta) == "COMPLETED"))
}
