/// Hace una petición con curl. Los datos y las cabeceras van por la entrada estándar (no en
/// la línea de órdenes) para que ninguna clave secreta se vea en la lista de procesos.
/// Devuelve (código HTTP, cuerpo).
fn ext_web_peticion(metodo: &str, url: &str, datos: Option<&str>, cabeceras: &[String]) -> Result<(u16, String), String> {
    use std::io::Write;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("la dirección '{}' debe empezar por https:// o http://", url));
    }
    let comillas = |texto: &str| format!("\"{}\"", texto.replace('\\', "\\\\").replace('"', "\\\""));
    let mut configuracion = format!("url = {}\nrequest = {}\n", comillas(url), metodo);
    for cabecera in cabeceras {
        if cabecera.contains(['\r', '\n']) {
            return Err("una cabecera no puede tener saltos de línea".to_string());
        }
        configuracion.push_str(&format!("header = {}\n", comillas(cabecera)));
    }
    if let Some(datos) = datos {
        configuracion.push_str(&format!("data = {}\n", comillas(datos)));
    }
    let mut proceso = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "30", "--proto", "=https,http", "-w", "\n%{http_code}", "-K", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| "no se encontró curl en este sistema; instálalo (sudo apt install curl)".to_string())?;
    if let Some(mut entrada) = proceso.stdin.take() {
        let _ = entrada.write_all(configuracion.as_bytes());
    }
    let salida = proceso.wait_with_output().map_err(|error| error.to_string())?;
    if !salida.status.success() {
        return Err(format!("no se pudo conectar: {}", String::from_utf8_lossy(&salida.stderr).trim()));
    }
    let texto = String::from_utf8_lossy(&salida.stdout).into_owned();
    let (cuerpo, codigo) = texto.rsplit_once('\n').unwrap_or(("", &texto));
    let codigo = codigo.trim().parse::<u16>().unwrap_or(0);
    Ok((codigo, cuerpo.to_string()))
}

fn ext_web_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice).and_then(|valor| valor.como_texto()) {
        Some(texto) => Ok(texto.to_string()),
        None => Err(format!("{}() espera un texto en el argumento {}", funcion, indice + 1)),
    }
}

fn ext_pedir_web(args: Vec<Valor>) -> Result<Valor, String> {
    let url = ext_web_texto(&args, 0, "pedir_web")?;
    let (codigo, cuerpo) = ext_web_peticion("GET", &url, None, &[]).map_err(|e| format!("pedir_web(): {}", e))?;
    if codigo >= 400 {
        return Err(format!("pedir_web(): '{}' respondió con el código {}", url, codigo));
    }
    Ok(Valor::texto(cuerpo))
}

fn ext_enviar_web(args: Vec<Valor>) -> Result<Valor, String> {
    let url = ext_web_texto(&args, 0, "enviar_web")?;
    let datos = ext_web_texto(&args, 1, "enviar_web")?;
    let (codigo, cuerpo) = ext_web_peticion("POST", &url, Some(&datos), &[])
        .map_err(|e| format!("enviar_web(): {}", e))?;
    if codigo >= 400 {
        return Err(format!("enviar_web(): '{}' respondió con el código {}", url, codigo));
    }
    Ok(Valor::texto(cuerpo))
}
