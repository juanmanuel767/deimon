// WhatsApp Cloud API (Meta): enviar mensajes y leer los avisos (webhooks) firmados.

fn ext_wa_base() -> String {
    let base = std::env::var("DEIMON_WHATSAPP_URL").unwrap_or_else(|_| "https://graph.facebook.com".to_string());
    let version = std::env::var("DEIMON_WHATSAPP_VERSION").unwrap_or_else(|_| "v26.0".to_string());
    format!("{}/{}", base.trim_end_matches('/'), version)
}

fn ext_wa_texto(args: &[Valor], i: usize, funcion: &str) -> Result<String, String> {
    match args.get(i).and_then(|v| v.como_texto()) {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(format!("{}() espera un texto no vacío en el argumento {}", funcion, i + 1)),
    }
}

fn ext_wa_telefono(texto: &str, funcion: &str) -> Result<String, String> {
    let cifras: String = texto.chars().filter(|c| c.is_ascii_digit()).collect();
    if cifras.len() < 8 || cifras.len() > 15 || texto.chars().any(|c| !(c.is_ascii_digit() || " +-()".contains(c))) {
        return Err(format!("{}(): '{}' no es un teléfono válido (escríbelo con el prefijo del país, como 51987654321)", funcion, texto));
    }
    Ok(cifras)
}

fn ext_wa_enviar(token: &str, id_numero: &str, cuerpo: ExtJson, funcion: &str) -> Result<Valor, String> {
    if !id_numero.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{}(): el id del número solo tiene cifras", funcion));
    }
    let url = format!("{}/{}/messages", ext_wa_base(), id_numero);
    let cabeceras = vec![format!("Authorization: Bearer {}", token), "Content-Type: application/json".to_string()];
    let (codigo, respuesta) = ext_web_peticion("POST", &url, Some(&ext_json_escribir(&cuerpo)), &cabeceras).map_err(|e| format!("{}(): {}", funcion, e))?;
    let json = ext_json_leer_texto(&respuesta).ok();
    if codigo >= 400 {
        let mensaje = json.as_ref().and_then(|j| ext_json_seguir(j, "error.message")).and_then(|m| match m { ExtJson::Texto(t) => Some(t.clone()), _ => None }).unwrap_or_else(|| format!("código {}", codigo));
        return Err(format!("{}(): WhatsApp respondió: {}", funcion, mensaje));
    }
    match json.as_ref().and_then(|j| ext_json_seguir(j, "messages[0].id")) {
        Some(ExtJson::Texto(id)) => Ok(Valor::texto(id.clone())),
        _ => Err(format!("{}(): WhatsApp no devolvió el id del mensaje", funcion)),
    }
}

fn ext_enviar_whatsapp(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "enviar_whatsapp";
    let (token, id_numero) = (ext_wa_texto(&args, 0, f)?, ext_wa_texto(&args, 1, f)?);
    let telefono = ext_wa_telefono(&ext_wa_texto(&args, 2, f)?, f)?;
    let texto = ext_wa_texto(&args, 3, f)?;
    if texto.chars().count() > 4096 {
        return Err("enviar_whatsapp(): el texto pasa de 4096 letras".to_string());
    }
    let cuerpo = ExtJson::Objeto(vec![
        ("messaging_product".to_string(), ExtJson::Texto("whatsapp".to_string())),
        ("to".to_string(), ExtJson::Texto(telefono)),
        ("type".to_string(), ExtJson::Texto("text".to_string())),
        ("text".to_string(), ExtJson::Objeto(vec![("body".to_string(), ExtJson::Texto(texto))])),
    ]);
    ext_wa_enviar(&token, &id_numero, cuerpo, f)
}

fn ext_enviar_plantilla_whatsapp(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "enviar_plantilla_whatsapp";
    let (token, id_numero) = (ext_wa_texto(&args, 0, f)?, ext_wa_texto(&args, 1, f)?);
    let telefono = ext_wa_telefono(&ext_wa_texto(&args, 2, f)?, f)?;
    let (plantilla, idioma) = (ext_wa_texto(&args, 3, f)?, ext_wa_texto(&args, 4, f)?);
    let cuerpo = ExtJson::Objeto(vec![
        ("messaging_product".to_string(), ExtJson::Texto("whatsapp".to_string())),
        ("to".to_string(), ExtJson::Texto(telefono)),
        ("type".to_string(), ExtJson::Texto("template".to_string())),
        ("template".to_string(), ExtJson::Objeto(vec![
            ("name".to_string(), ExtJson::Texto(plantilla)),
            ("language".to_string(), ExtJson::Objeto(vec![("code".to_string(), ExtJson::Texto(idioma))])),
        ])),
    ]);
    ext_wa_enviar(&token, &id_numero, cuerpo, f)
}

/// Meta comprueba el aviso con GET ?hub.mode=subscribe&hub.verify_token=...&hub.challenge=...
fn ext_verificar_whatsapp(args: Vec<Valor>) -> Result<Valor, String> {
    let clave = ext_wa_texto(&args, 0, "verificar_whatsapp")?;
    let leer = |nombre: &str| campo(&Valor::texto(nombre)).como_texto().map(|t| t.to_string());
    match (leer("hub.mode"), leer("hub.verify_token"), leer("hub.challenge")) {
        (Some(modo), Some(token), Some(reto)) if modo == "subscribe" && token == clave && reto.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') => Ok(Valor::texto(reto)),
        _ => Ok(Valor::Nada),
    }
}

/// Lee los mensajes del aviso de Meta. Comprueba la firma X-Hub-Signature-256 (HMAC-SHA256
/// del cuerpo con el secreto de la app): un aviso falso da error.
fn ext_mensajes_whatsapp(args: Vec<Valor>) -> Result<Valor, String> {
    let secreto = ext_wa_texto(&args, 0, "mensajes_whatsapp")?;
    let cuerpo = match cuerpo_recibido() {
        Valor::Texto(t) => (*t).clone(),
        _ => String::new(),
    };
    let firma = match cabecera_recibida(&Valor::texto("X-Hub-Signature-256")) {
        Valor::Texto(t) => (*t).clone(),
        _ => String::new(),
    };
    let esperada = format!("sha256={}", hexadecimal(&hmac_sha256(secreto.as_bytes(), cuerpo.as_bytes())));
    let iguales = firma.len() == esperada.len() && firma.bytes().zip(esperada.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0;
    if !iguales {
        return Err("mensajes_whatsapp(): la firma del aviso no es válida (no viene de Meta o el secreto de la app no es ese)".to_string());
    }
    let json = ext_json_leer_texto(&cuerpo).map_err(|e| format!("mensajes_whatsapp(): el aviso no es JSON válido: {}", e))?;
    let mut lista = Vec::new();
    let texto_de_json = |j: Option<&ExtJson>| match j {
        Some(ExtJson::Texto(t)) => Valor::Texto(Rc::new(escapar_html(t))),
        _ => Valor::Nada,
    };
    if let Some(ExtJson::Lista(entradas)) = ext_json_seguir(&json, "entry") {
        for entrada in entradas {
            if let Some(ExtJson::Lista(cambios)) = ext_json_seguir(entrada, "changes") {
                for cambio in cambios {
                    let nombre = texto_de_json(ext_json_seguir(cambio, "value.contacts[0].profile.name"));
                    if let Some(ExtJson::Lista(mensajes)) = ext_json_seguir(cambio, "value.messages") {
                        for m in mensajes {
                            lista.push(Valor::mapa(vec![
                                ("de".to_string(), texto_de_json(ext_json_seguir(m, "from"))),
                                ("nombre".to_string(), nombre.clone()),
                                ("texto".to_string(), texto_de_json(ext_json_seguir(m, "text.body"))),
                                ("tipo".to_string(), texto_de_json(ext_json_seguir(m, "type"))),
                                ("id".to_string(), texto_de_json(ext_json_seguir(m, "id"))),
                            ]));
                        }
                    }
                }
            }
        }
    }
    Ok(Valor::lista(lista))
}
