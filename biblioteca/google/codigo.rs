// Entrar con Google (OAuth 2.0, código de autorización + OpenID Connect).

fn ext_go_texto(args: &[Valor], i: usize, funcion: &str) -> Result<String, String> {
    match args.get(i).and_then(|v| v.como_texto()) {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(format!("{}() espera un texto no vacío en el argumento {}", funcion, i + 1)),
    }
}

fn ext_go_codificar(texto: &str) -> String {
    texto
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) })
        .collect()
}

/// Direcciones de Google (en las pruebas, DEIMON_GOOGLE_URL las cambia por un servidor local).
fn ext_go_url(cual: &str) -> String {
    if let Ok(base) = std::env::var("DEIMON_GOOGLE_URL") {
        return format!("{}/{}", base.trim_end_matches('/'), cual);
    }
    match cual {
        "auth" => "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        "token" => "https://oauth2.googleapis.com/token".to_string(),
        _ => "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
    }
}

fn ext_direccion_google(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "direccion_google";
    let (cliente, vuelta) = (ext_go_texto(&args, 0, f)?, ext_go_texto(&args, 1, f)?);
    // Un código al azar que se guarda en la sesión del visitante: al volver se comprueba
    // que la vuelta empezó aquí (protección contra CSRF).
    let estado = hexadecimal(&bytes_aleatorios(16));
    guardar_en_sesion(&Valor::texto("deimon_google_estado"), &Valor::texto(estado.clone()));
    Ok(Valor::texto(format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&prompt=select_account",
        ext_go_url("auth"),
        ext_go_codificar(&cliente),
        ext_go_codificar(&vuelta),
        ext_go_codificar("openid email profile"),
        estado
    )))
}

fn ext_usuario_google(args: Vec<Valor>) -> Result<Valor, String> {
    let f = "usuario_google";
    let (cliente, secreto, vuelta) = (ext_go_texto(&args, 0, f)?, ext_go_texto(&args, 1, f)?, ext_go_texto(&args, 2, f)?);
    let guardado = leer_de_sesion(&Valor::texto("deimon_google_estado"));
    borrar_de_sesion(&Valor::texto("deimon_google_estado"));
    let recibido = campo(&Valor::texto("state"));
    let codigo = campo(&Valor::texto("code"));
    let (guardado, recibido, codigo) = match (guardado.como_texto(), recibido.como_texto(), codigo.como_texto()) {
        (Some(g), Some(r), Some(c)) if !g.is_empty() && g == r => (g.to_string(), r.to_string(), c.to_string()),
        _ => return Ok(Valor::Nada),
    };
    let _ = (guardado, recibido);
    let datos = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        ext_go_codificar(&codigo),
        ext_go_codificar(&cliente),
        ext_go_codificar(&secreto),
        ext_go_codificar(&vuelta)
    );
    let (estado, respuesta) = ext_web_peticion("POST", &ext_go_url("token"), Some(&datos), &["Content-Type: application/x-www-form-urlencoded".to_string()]).map_err(|e| format!("{}(): {}", f, e))?;
    if estado >= 400 {
        return Ok(Valor::Nada);
    }
    let token = match ext_json_leer_texto(&respuesta).ok().as_ref().and_then(|j| ext_json_seguir(j, "access_token")) {
        Some(ExtJson::Texto(t)) => t.clone(),
        _ => return Ok(Valor::Nada),
    };
    let (estado, respuesta) = ext_web_peticion("GET", &ext_go_url("usuario"), None, &[format!("Authorization: Bearer {}", token)]).map_err(|e| format!("{}(): {}", f, e))?;
    if estado >= 400 {
        return Ok(Valor::Nada);
    }
    let json = match ext_json_leer_texto(&respuesta) {
        Ok(j) => j,
        Err(_) => return Ok(Valor::Nada),
    };
    let texto = |camino: &str| match ext_json_seguir(&json, camino) {
        Some(ExtJson::Texto(t)) => Valor::Texto(Rc::new(escapar_html(t))),
        _ => Valor::Nada,
    };
    if texto("sub") == Valor::Nada {
        return Ok(Valor::Nada);
    }
    let verificado = matches!(ext_json_seguir(&json, "email_verified"), Some(ExtJson::Logico(true)));
    Ok(Valor::mapa(vec![
        ("id".to_string(), texto("sub")),
        ("email".to_string(), texto("email")),
        ("verificado".to_string(), Valor::Logico(verificado)),
        ("nombre".to_string(), texto("name")),
        ("foto".to_string(), texto("picture")),
    ]))
}
