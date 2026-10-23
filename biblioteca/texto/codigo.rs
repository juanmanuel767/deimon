fn ext_texto_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice) {
        Some(valor) => match valor.como_texto() {
            Some(texto) => Ok(texto.to_string()),
            None => Err(format!(
                "{}() espera un texto en el argumento {}, no {}",
                funcion,
                indice + 1,
                valor.tipo()
            )),
        },
        None => Err(format!(
            "{}() espera un texto en el argumento {}",
            funcion,
            indice + 1
        )),
    }
}

fn ext_mayusculas(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "mayusculas")?;
    Ok(Valor::texto(texto.to_uppercase()))
}

fn ext_minusculas(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "minusculas")?;
    Ok(Valor::texto(texto.to_lowercase()))
}

fn ext_recortar(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "recortar")?;
    Ok(Valor::texto(texto.trim()))
}

fn ext_contiene(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "contiene")?;
    let buscado = ext_texto_texto(&args, 1, "contiene")?;
    Ok(Valor::Logico(texto.contains(&buscado)))
}

fn ext_reemplazar(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "reemplazar")?;
    let de = ext_texto_texto(&args, 1, "reemplazar")?;
    let a = ext_texto_texto(&args, 2, "reemplazar")?;
    Ok(Valor::texto(texto.replace(&de, &a)))
}

fn ext_partir(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "partir")?;
    let separador = ext_texto_texto(&args, 1, "partir")?;
    if separador.is_empty() {
        return Err("partir() necesita un separador que no esté vacío".to_string());
    }
    let partes = texto
        .split(separador.as_str())
        .map(Valor::texto)
        .collect();
    Ok(Valor::lista(partes))
}

fn ext_empieza_con(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "empieza_con")?;
    let inicio = ext_texto_texto(&args, 1, "empieza_con")?;
    Ok(Valor::Logico(texto.starts_with(&inicio)))
}

fn ext_termina_con(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "termina_con")?;
    let fin = ext_texto_texto(&args, 1, "termina_con")?;
    Ok(Valor::Logico(texto.ends_with(&fin)))
}

fn ext_repetir(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "repetir")?;
    let veces = match args.get(1).and_then(|valor| valor.como_numero()) {
        Some(n) if n >= 0.0 && n.fract() == 0.0 => n as usize,
        _ => return Err("repetir() espera un número entero no negativo en el argumento 2".to_string()),
    };
    Ok(Valor::texto(texto.repeat(veces)))
}

fn ext_unir(args: Vec<Valor>) -> Result<Valor, String> {
    let elementos = match args.first().and_then(|valor| valor.como_lista()) {
        Some(elementos) => elementos,
        None => return Err("unir() espera una lista en el argumento 1".to_string()),
    };
    let separador = ext_texto_texto(&args, 1, "unir")?;
    let partes: Vec<String> = elementos.iter().map(texto_de).collect();
    Ok(Valor::texto(partes.join(&separador)))
}

/// Rellena una plantilla al estilo de sprintf: %s texto, %d entero, %f y %.2f decimales,
/// %x %X %o %b bases, %c carácter, %% el signo; admite ancho, `-`, `0`, `+` y `%1$s`.
fn ext_formatear_texto(args: Vec<Valor>) -> Result<Valor, String> {
    let plantilla = ext_texto_texto(&args, 0, "formatear_texto")?;
    let valores: Vec<Valor> = match args.get(1) {
        Some(Valor::Lista(lista)) => lista.iter().cloned().collect(),
        Some(otro) => vec![otro.clone()],
        None => Vec::new(),
    };
    let letras: Vec<char> = plantilla.chars().collect();
    let mut salida = String::new();
    let mut siguiente = 0usize;
    let mut i = 0usize;
    while i < letras.len() {
        if letras[i] != '%' {
            salida.push(letras[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i < letras.len() && letras[i] == '%' {
            salida.push('%');
            i += 1;
            continue;
        }
        // Posición explícita: %2$s
        let inicio = i;
        let mut posicion: Option<usize> = None;
        while i < letras.len() && letras[i].is_ascii_digit() {
            i += 1;
        }
        if i < letras.len() && letras[i] == '$' && i > inicio {
            let numero: String = letras[inicio..i].iter().collect();
            posicion = numero.parse::<usize>().ok().filter(|n| *n > 0).map(|n| n - 1);
            i += 1;
        } else {
            i = inicio;
        }
        let (mut izquierda, mut ceros, mut signo) = (false, false, false);
        while i < letras.len() && matches!(letras[i], '-' | '0' | '+') {
            match letras[i] {
                '-' => izquierda = true,
                '0' => ceros = true,
                _ => signo = true,
            }
            i += 1;
        }
        let mut ancho = 0usize;
        while i < letras.len() && letras[i].is_ascii_digit() {
            ancho = ancho * 10 + letras[i].to_digit(10).unwrap_or(0) as usize;
            i += 1;
        }
        let mut precision: Option<usize> = None;
        if i < letras.len() && letras[i] == '.' {
            i += 1;
            let mut p = 0usize;
            while i < letras.len() && letras[i].is_ascii_digit() {
                p = p * 10 + letras[i].to_digit(10).unwrap_or(0) as usize;
                i += 1;
            }
            precision = Some(p);
        }
        let tipo = match letras.get(i) {
            Some(letra) => *letra,
            None => return Err("formatear_texto(): la plantilla termina en un % sin tipo (usa %% para escribir el signo)".to_string()),
        };
        i += 1;
        let indice = posicion.unwrap_or_else(|| {
            let actual = siguiente;
            siguiente += 1;
            actual
        });
        let valor = match valores.get(indice) {
            Some(valor) => valor.clone(),
            None => return Err(format!("formatear_texto(): faltan valores; la plantilla pide al menos {}", indice + 1)),
        };
        let numero = || -> Result<f64, String> {
            match &valor {
                Valor::Numero(n) => Ok(*n),
                Valor::Logico(b) => Ok(if *b { 1.0 } else { 0.0 }),
                Valor::Nada => Ok(0.0),
                Valor::Texto(t) => Ok(t.trim().parse::<f64>().unwrap_or(0.0)),
                otro => Err(format!("formatear_texto(): %{} espera un número, no {}", tipo, otro.tipo())),
            }
        };
        let mut parte = match tipo {
            's' => {
                let texto = texto_de(&valor);
                match precision {
                    Some(p) => texto.chars().take(p).collect(),
                    None => texto,
                }
            }
            'd' | 'u' | 'i' => {
                let n = numero()?.trunc() as i64;
                if signo && n >= 0 { format!("+{}", n) } else { n.to_string() }
            }
            'f' | 'F' => {
                let n = numero()?;
                let texto = format!("{:.*}", precision.unwrap_or(6), n);
                if signo && n >= 0.0 { format!("+{}", texto) } else { texto }
            }
            'e' | 'E' => {
                let texto = format!("{:.*e}", precision.unwrap_or(6), numero()?);
                let texto = match texto.split_once('e') {
                    Some((base, exp)) if !exp.starts_with('-') => format!("{}e+{}", base, exp),
                    _ => texto,
                };
                if tipo == 'E' { texto.to_uppercase() } else { texto }
            }
            'x' => format!("{:x}", numero()?.trunc() as i64),
            'X' => format!("{:X}", numero()?.trunc() as i64),
            'o' => format!("{:o}", numero()?.trunc() as i64),
            'b' => format!("{:b}", numero()?.trunc() as i64),
            'c' => char::from_u32(numero()?.trunc() as u32).map(String::from).unwrap_or_default(),
            otro => return Err(format!("formatear_texto(): el tipo %{} no existe; usa %s, %d, %f, %x, %o, %b, %c o %e", otro)),
        };
        let largo = parte.chars().count();
        if largo < ancho {
            let falta = ancho - largo;
            if izquierda {
                parte.push_str(&" ".repeat(falta));
            } else if ceros && tipo != 's' && tipo != 'c' {
                let (signo_texto, cifras) = match parte.chars().next() {
                    Some(c @ ('-' | '+')) => (c.to_string(), parte[1..].to_string()),
                    _ => (String::new(), parte.clone()),
                };
                parte = format!("{}{}{}", signo_texto, "0".repeat(falta), cifras);
            } else {
                parte = format!("{}{}", " ".repeat(falta), parte);
            }
        }
        salida.push_str(&parte);
    }
    Ok(Valor::texto(salida))
}
