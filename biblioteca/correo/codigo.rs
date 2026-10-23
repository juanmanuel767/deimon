fn ext_correo_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice).and_then(|valor| valor.como_texto()) {
        Some(texto) => Ok(texto.to_string()),
        None => Err(format!("{}() espera un texto en el argumento {}", funcion, indice + 1)),
    }
}

fn ext_correo_valido(direccion: &str) -> bool {
    let (usuario, dominio) = match direccion.split_once('@') {
        Some(partes) => partes,
        None => return false,
    };
    let permitido = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    !usuario.is_empty()
        && usuario.len() <= 64
        && usuario.chars().all(permitido)
        && dominio.contains('.')
        && !dominio.starts_with('.')
        && !dominio.ends_with('.')
        && !dominio.contains("..")
        && dominio.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn ext_es_correo_valido(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(Valor::Logico(ext_correo_valido(&ext_correo_texto(&args, 0, "es_correo_valido")?)))
}

/// Cabecera con acentos codificada según RFC 2047 (=?UTF-8?B?...?=).
fn ext_correo_cabecera(texto: &str) -> String {
    if texto.is_ascii() {
        return texto.to_string();
    }
    const TABLA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = texto.as_bytes();
    let mut base64 = String::new();
    for trozo in bytes.chunks(3) {
        let b = [trozo[0], *trozo.get(1).unwrap_or(&0), *trozo.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= trozo.len() {
                base64.push(TABLA[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                base64.push('=');
            }
        }
    }
    format!("=?UTF-8?B?{}?=", base64)
}

fn ext_enviar_correo(args: Vec<Valor>) -> Result<Valor, String> {
    use std::io::Write;
    let para = ext_correo_texto(&args, 0, "enviar_correo")?;
    let asunto = ext_correo_texto(&args, 1, "enviar_correo")?;
    let texto = ext_correo_texto(&args, 2, "enviar_correo")?;
    if !ext_correo_valido(&para) {
        return Err(format!("enviar_correo(): '{}' no es una dirección de correo válida", para));
    }
    if asunto.contains(['\r', '\n']) {
        return Err("enviar_correo(): el asunto no puede tener saltos de línea".to_string());
    }
    let programa = std::env::var("DEIMON_SENDMAIL").ok().filter(|ruta| !ruta.is_empty()).or_else(|| {
        ["/usr/sbin/sendmail", "/usr/lib/sendmail", "/usr/bin/sendmail"]
            .iter()
            .find(|ruta| std::path::Path::new(ruta).exists())
            .map(|ruta| ruta.to_string())
    });
    let programa = match programa {
        Some(programa) => programa,
        None => return Err("enviar_correo(): no hay sendmail en este sistema; instala uno (por ejemplo msmtp-mta) o indica cuál usar con DEIMON_SENDMAIL".to_string()),
    };
    let mut mensaje = String::new();
    if let Ok(de) = std::env::var("DEIMON_CORREO_DE") {
        if ext_correo_valido(&de) {
            mensaje.push_str(&format!("From: {}\r\n", de));
        }
    }
    mensaje.push_str(&format!("To: {}\r\n", para));
    mensaje.push_str(&format!("Subject: {}\r\n", ext_correo_cabecera(&asunto)));
    mensaje.push_str("MIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n");
    mensaje.push_str(&texto.replace("\r\n", "\n").replace('\n', "\r\n"));
    mensaje.push_str("\r\n");
    let mut proceso = std::process::Command::new(&programa)
        .args(["-t", "-i"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("enviar_correo(): no se pudo lanzar '{}': {}", programa, error))?;
    if let Some(mut entrada) = proceso.stdin.take() {
        let _ = entrada.write_all(mensaje.as_bytes());
    }
    let estado = proceso
        .wait()
        .map_err(|error| format!("enviar_correo(): {}", error))?;
    Ok(Valor::Logico(estado.success()))
}
