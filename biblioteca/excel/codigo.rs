fn ext_excel_filas(args: &[Valor], funcion: &str) -> Result<(String, Vec<Vec<Valor>>), String> {
    let archivo = match args.first().and_then(|v| v.como_texto()) {
        Some(a) if !a.is_empty() => a.to_string(),
        _ => return Err(format!("{}() espera el nombre del archivo", funcion)),
    };
    let filas = match args.get(1).and_then(|v| v.como_lista()) {
        Some(filas) => filas
            .iter()
            .map(|fila| match fila.como_lista() {
                Some(celdas) => Ok(celdas.to_vec()),
                None => Ok(vec![fila.clone()]),
            })
            .collect::<Result<Vec<_>, String>>()?,
        None => return Err(format!("{}() espera las filas como una lista de listas", funcion)),
    };
    Ok((archivo, filas))
}

fn ext_excel_crc32(datos: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in datos {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Un ZIP sin comprimir (método «stored»): suficiente y válido para un .xlsx.
fn ext_excel_zip(archivos: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut zip = Vec::new();
    let mut central = Vec::new();
    for (nombre, datos) in archivos {
        let crc = ext_excel_crc32(datos);
        let posicion = zip.len() as u32;
        let cabecera = |firma: u32| {
            let mut c = Vec::new();
            c.extend_from_slice(&firma.to_le_bytes());
            c
        };
        let mut local = cabecera(0x0403_4b50);
        local.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]); // versión, flags, método 0, hora, fecha
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        local.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        local.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(nombre.as_bytes());
        zip.extend_from_slice(&local);
        zip.extend_from_slice(datos);
        let mut entrada = cabecera(0x0201_4b50);
        entrada.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]);
        entrada.extend_from_slice(&crc.to_le_bytes());
        entrada.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        entrada.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        entrada.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
        entrada.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // extra, comentario, disco, atributos
        entrada.extend_from_slice(&posicion.to_le_bytes());
        entrada.extend_from_slice(nombre.as_bytes());
        central.extend_from_slice(&entrada);
    }
    let inicio_central = zip.len() as u32;
    zip.extend_from_slice(&central);
    zip.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    zip.extend_from_slice(&[0, 0, 0, 0]);
    zip.extend_from_slice(&(archivos.len() as u16).to_le_bytes());
    zip.extend_from_slice(&(archivos.len() as u16).to_le_bytes());
    zip.extend_from_slice(&(central.len() as u32).to_le_bytes());
    zip.extend_from_slice(&inicio_central.to_le_bytes());
    zip.extend_from_slice(&[0, 0]);
    zip
}

fn ext_excel_xml(texto: &str) -> String {
    texto
        .chars()
        .filter(|c| !matches!(*c as u32, 0..=8 | 11 | 12 | 14..=31))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn ext_excel_columna(mut n: usize) -> String {
    let mut nombre = String::new();
    n += 1;
    while n > 0 {
        let resto = (n - 1) % 26;
        nombre.insert(0, (b'A' + resto as u8) as char);
        n = (n - 1) / 26;
    }
    nombre
}

fn ext_crear_excel(args: Vec<Valor>) -> Result<Valor, String> {
    let (archivo, filas) = ext_excel_filas(&args, "crear_excel")?;
    if !archivo.to_lowercase().ends_with(".xlsx") {
        return Err("crear_excel(): el archivo debe terminar en .xlsx".to_string());
    }
    let mut hoja = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>");
    for (i, fila) in filas.iter().enumerate() {
        hoja.push_str(&format!("<row r=\"{}\">", i + 1));
        for (j, celda) in fila.iter().enumerate() {
            let referencia = format!("{}{}", ext_excel_columna(j), i + 1);
            match celda.como_numero() {
                Some(n) if n.is_finite() => hoja.push_str(&format!("<c r=\"{}\"><v>{}</v></c>", referencia, n)),
                _ => hoja.push_str(&format!(
                    "<c r=\"{}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                    referencia,
                    ext_excel_xml(&texto_de(celda))
                )),
            }
        }
        hoja.push_str("</row>");
    }
    hoja.push_str("</sheetData></worksheet>");
    let archivos: Vec<(&str, Vec<u8>)> = vec![
        ("[Content_Types].xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>".to_vec()),
        ("_rels/.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".to_vec()),
        ("xl/workbook.xml", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Hoja1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>".to_vec()),
        ("xl/_rels/workbook.xml.rels", b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>".to_vec()),
        ("xl/worksheets/sheet1.xml", hoja.into_bytes()),
    ];
    std::fs::write(&archivo, ext_excel_zip(&archivos)).map_err(|e| format!("crear_excel(): no se pudo escribir '{}': {}", archivo, e))?;
    Ok(Valor::Nada)
}

fn ext_crear_csv(args: Vec<Valor>) -> Result<Valor, String> {
    let (archivo, filas) = ext_excel_filas(&args, "crear_csv")?;
    // BOM para que Excel reconozca los acentos; ';' como lo espera Excel en español.
    let mut csv = String::from("\u{FEFF}");
    for fila in filas {
        let celdas: Vec<String> = fila
            .iter()
            .map(|celda| {
                let texto = texto_de(celda);
                if texto.contains([';', '"', '\n', '\r', ',']) {
                    format!("\"{}\"", texto.replace('"', "\"\""))
                } else {
                    texto
                }
            })
            .collect();
        csv.push_str(&celdas.join(";"));
        csv.push_str("\r\n");
    }
    std::fs::write(&archivo, csv).map_err(|e| format!("crear_csv(): no se pudo escribir '{}': {}", archivo, e))?;
    Ok(Valor::Nada)
}

fn ext_leer_csv(args: Vec<Valor>) -> Result<Valor, String> {
    let archivo = match args.first().and_then(|v| v.como_texto()) {
        Some(a) => a.to_string(),
        None => return Err("leer_csv() espera el nombre del archivo".to_string()),
    };
    let texto = std::fs::read_to_string(&archivo).map_err(|e| format!("leer_csv(): no se pudo leer '{}': {}", archivo, e))?;
    let texto = texto.trim_start_matches('\u{FEFF}');
    let primera = texto.lines().next().unwrap_or("");
    let separador = if primera.matches(';').count() >= primera.matches(',').count() { ';' } else { ',' };
    let mut filas = Vec::new();
    let mut fila = Vec::new();
    let mut celda = String::new();
    let mut entre_comillas = false;
    let mut caracteres = texto.chars().peekable();
    while let Some(c) = caracteres.next() {
        match c {
            '"' if entre_comillas && caracteres.peek() == Some(&'"') => {
                celda.push('"');
                caracteres.next();
            }
            '"' => entre_comillas = !entre_comillas,
            c if c == separador && !entre_comillas => fila.push(Valor::texto(std::mem::take(&mut celda))),
            '\r' if !entre_comillas => {}
            '\n' if !entre_comillas => {
                fila.push(Valor::texto(std::mem::take(&mut celda)));
                filas.push(Valor::lista(std::mem::take(&mut fila)));
            }
            otro => celda.push(otro),
        }
    }
    if !celda.is_empty() || !fila.is_empty() {
        fila.push(Valor::texto(celda));
        filas.push(Valor::lista(fila));
    }
    Ok(Valor::lista(filas))
}
