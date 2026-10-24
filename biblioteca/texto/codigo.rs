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

/// Mueve la coma de un texto en notación científica ("1.5e-7") a notación normal.
fn ext_mover_coma(base: &str, exponente: i32) -> String {
    let (entero, fraccion) = match base.split_once('.') {
        Some((entero, fraccion)) => (entero, fraccion),
        None => (base, ""),
    };
    let digitos = format!("{}{}", entero, fraccion);
    let punto = entero.len() as i32 + exponente;
    if punto <= 0 {
        format!("0.{}{}", "0".repeat((-punto) as usize), digitos)
    } else if punto as usize >= digitos.len() {
        format!("{}{}", digitos, "0".repeat(punto as usize - digitos.len()))
    } else {
        format!("{}.{}", &digitos[..punto as usize], &digitos[punto as usize..])
    }
}

/// Redondea a `decimales` cifras partiendo de la representación decimal más corta del número,
/// como hace PHP (así 1.005 con 2 cifras da 1.01). Devuelve si es negativo y el texto redondeado
/// con exactamente `decimales` cifras decimales (sin separadores).
fn ext_redondear_decimal(numero: f64, decimales: usize) -> (bool, String) {
    let negativo = numero.is_sign_negative();
    if !numero.is_finite() {
        return (negativo, format!("{}", numero.abs()));
    }
    let mut texto = format!("{}", numero.abs());
    if let Some((base, exponente)) = texto.split_once(['e', 'E']) {
        let exponente = exponente.parse::<i32>().unwrap_or(0);
        texto = ext_mover_coma(base, exponente);
    }
    let (entero, fraccion) = match texto.split_once('.') {
        Some((entero, fraccion)) => (entero.to_string(), fraccion.to_string()),
        None => (texto, String::new()),
    };
    let largo_entero = entero.len();
    let mut digitos: Vec<u8> = entero
        .bytes()
        .chain(fraccion.bytes())
        .map(|byte| byte - b'0')
        .collect();
    let conservar = largo_entero + decimales;
    while digitos.len() < conservar {
        digitos.push(0);
    }
    if digitos.len() > conservar {
        let sube = digitos[conservar] >= 5;
        digitos.truncate(conservar);
        if sube {
            let mut i = conservar;
            loop {
                if i == 0 {
                    digitos.insert(0, 1);
                    break;
                }
                i -= 1;
                if digitos[i] < 9 {
                    digitos[i] += 1;
                    break;
                }
                digitos[i] = 0;
            }
        }
    }
    let punto = digitos.len() - decimales;
    let entero: String = digitos[..punto].iter().map(|d| (d + b'0') as char).collect();
    let entero = entero.trim_start_matches('0');
    let entero = if entero.is_empty() { "0" } else { entero };
    let fraccion: String = digitos[punto..].iter().map(|d| (d + b'0') as char).collect();
    let cero = entero == "0" && fraccion.bytes().all(|byte| byte == b'0');
    let salida = if decimales == 0 {
        entero.to_string()
    } else {
        format!("{}.{}", entero, fraccion)
    };
    (negativo && !cero, salida)
}

/// Formatea un número como `number_format` de PHP: decimales, separador decimal y de miles,
/// redondeando como PHP (1.005 con 2 cifras da 1.01) y con el signo solo si el resultado no es cero.
fn ext_formatear_numero(args: Vec<Valor>) -> Result<Valor, String> {
    let numero = match args.first() {
        Some(Valor::Numero(numero)) => *numero,
        Some(Valor::Logico(logico)) => {
            if *logico {
                1.0
            } else {
                0.0
            }
        }
        Some(Valor::Nada) => 0.0,
        Some(Valor::Texto(texto)) => texto.trim().parse::<f64>().map_err(|_| {
            "formatear_numero() espera un número en el argumento 1".to_string()
        })?,
        Some(otro) => {
            return Err(format!(
                "formatear_numero() espera un número en el argumento 1, no {}",
                otro.tipo()
            ))
        }
        None => return Err("formatear_numero() espera un número en el argumento 1".to_string()),
    };
    let decimales = match args.get(1).and_then(|valor| valor.como_numero()) {
        Some(numero) if numero >= 0.0 && numero.fract() == 0.0 => numero as usize,
        None => 0,
        _ => {
            return Err(
                "formatear_numero() espera un número entero no negativo de decimales en el argumento 2"
                    .to_string(),
            )
        }
    };
    let separador_decimal = match args.get(2) {
        Some(_) => ext_texto_texto(&args, 2, "formatear_numero")?,
        None => ".".to_string(),
    };
    let separador_miles = match args.get(3) {
        Some(_) => ext_texto_texto(&args, 3, "formatear_numero")?,
        None => ",".to_string(),
    };
    let (negativo, redondeado) = ext_redondear_decimal(numero, decimales);
    let (entero, fraccion) = match redondeado.split_once('.') {
        Some((entero, fraccion)) => (entero, Some(fraccion)),
        None => (redondeado.as_str(), None),
    };
    let mut con_miles = String::new();
    let largo = entero.len();
    for (indice, cifra) in entero.chars().enumerate() {
        if indice > 0 && (largo - indice) % 3 == 0 && !separador_miles.is_empty() {
            con_miles.push_str(&separador_miles);
        }
        con_miles.push(cifra);
    }
    let mut salida = String::new();
    if negativo {
        salida.push('-');
    }
    salida.push_str(&con_miles);
    if let Some(fraccion) = fraccion {
        salida.push_str(&separador_decimal);
        salida.push_str(fraccion);
    }
    Ok(Valor::texto(salida))
}

/// Rellena un texto por la derecha, por la izquierda o por los dos lados (como `str_pad` de
/// PHP, pero contando caracteres y no bytes, para que los acentos y la ñ no fallen).
fn ext_rellenar_texto(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "rellenar_texto")?;
    let largo = match args.get(1).and_then(|valor| valor.como_numero()) {
        Some(numero) if numero >= 0.0 && numero.fract() == 0.0 => numero as usize,
        _ => {
            return Err(
                "rellenar_texto() espera un número entero no negativo en el argumento 2"
                    .to_string(),
            )
        }
    };
    let relleno = ext_texto_texto(&args, 2, "rellenar_texto")?;
    let lado = match args.get(3) {
        Some(valor) => match valor.como_texto() {
            Some(lado) => lado.to_string(),
            None => {
                return Err(
                    "rellenar_texto() espera \"derecha\", \"izquierda\" o \"ambos\" en el argumento 4"
                        .to_string(),
                )
            }
        },
        None => "derecha".to_string(),
    };
    let actuales = texto.chars().count();
    if actuales >= largo {
        return Ok(Valor::texto(texto));
    }
    if relleno.is_empty() {
        return Err("rellenar_texto() necesita un relleno que no esté vacío".to_string());
    }
    let falta = largo - actuales;
    let (izquierda, derecha) = match lado.as_str() {
        "derecha" => (0, falta),
        "izquierda" => (falta, 0),
        "ambos" => (falta / 2, falta - falta / 2),
        otro => {
            return Err(format!(
                "rellenar_texto() espera \"derecha\", \"izquierda\" o \"ambos\" en el argumento 4, no \"{}\"",
                otro
            ))
        }
    };
    let rellenar = |cantidad: usize| -> String {
        relleno.chars().cycle().take(cantidad).collect()
    };
    let mut salida = String::new();
    salida.push_str(&rellenar(izquierda));
    salida.push_str(&texto);
    salida.push_str(&rellenar(derecha));
    Ok(Valor::texto(salida))
}

/// Pone en mayúscula la primera letra (como `ucfirst` de PHP, pero con acentos: "ñandú" → "Ñandú").
fn ext_primera_mayuscula(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = ext_texto_texto(&args, 0, "primera_mayuscula")?;
    let mut letras = texto.chars();
    let salida = match letras.next() {
        Some(primera) => {
            let resto: String = letras.collect();
            format!("{}{}", primera.to_uppercase(), resto)
        }
        None => String::new(),
    };
    Ok(Valor::texto(salida))
}

/// Muestra cualquier valor como `print_r` de PHP: listas y listas con claves anidadas, con
/// índices y sangría. Los textos se muestran sin comillas y los lógicos como 1 o nada.
fn ext_texto_completo(args: Vec<Valor>) -> Result<Valor, String> {
    let valor = match args.first() {
        Some(valor) => valor,
        None => return Err("texto_completo() espera un valor".to_string()),
    };
    let texto = match valor {
        Valor::Lista(_) | Valor::Mapa(_) => ext_texto_texto_completo(valor, 0),
        otro => ext_texto_texto_completo_escalar(otro),
    };
    Ok(Valor::texto(texto))
}

fn ext_texto_texto_completo(valor: &Valor, margen: usize) -> String {
    let mut salida = String::from("Array\n");
    salida.push_str(&" ".repeat(margen));
    salida.push_str("(\n");
    match valor {
        Valor::Lista(elementos) => {
            for (indice, elemento) in elementos.iter().enumerate() {
                salida.push_str(&" ".repeat(margen + 4));
                salida.push_str(&format!("[{}] => ", indice));
                salida.push_str(&ext_texto_texto_completo_elemento(elemento, margen + 8));
            }
        }
        Valor::Mapa(mapa) => {
            for (clave, elemento) in mapa.pares() {
                salida.push_str(&" ".repeat(margen + 4));
                salida.push('[');
                salida.push_str(clave);
                salida.push_str("] => ");
                salida.push_str(&ext_texto_texto_completo_elemento(elemento, margen + 8));
            }
        }
        _ => {}
    }
    salida.push_str(&" ".repeat(margen));
    salida.push_str(")\n");
    salida
}

fn ext_texto_texto_completo_elemento(valor: &Valor, margen: usize) -> String {
    match valor {
        Valor::Lista(_) | Valor::Mapa(_) => {
            let mut salida = ext_texto_texto_completo(valor, margen);
            salida.push('\n');
            salida
        }
        otro => format!("{}\n", ext_texto_texto_completo_escalar(otro)),
    }
}

fn ext_texto_texto_completo_escalar(valor: &Valor) -> String {
    match valor {
        Valor::Logico(true) => "1".to_string(),
        Valor::Logico(false) | Valor::Nada => String::new(),
        otro => texto_de(otro),
    }
}
