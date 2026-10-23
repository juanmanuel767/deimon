/// Convierte texto a la codificación WinAnsi de las fuentes estándar del PDF (acentos, ñ, ¿, €).
fn ext_pdf_codificar(texto: &str) -> Vec<u8> {
    let mut salida = Vec::new();
    for caracter in texto.chars() {
        let codigo = caracter as u32;
        let byte = match caracter {
            '€' => 0x80,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            _ if codigo < 0x80 => codigo as u8,
            _ if (0xA0..=0xFF).contains(&codigo) => codigo as u8,
            _ => b'?',
        };
        if matches!(byte, b'(' | b')' | b'\\') {
            salida.push(b'\\');
        }
        salida.push(byte);
    }
    salida
}

/// Parte una línea en trozos de como mucho `ancho` caracteres, cortando por espacios.
fn ext_pdf_partir(linea: &str, ancho: usize) -> Vec<String> {
    let mut trozos = Vec::new();
    let mut actual = String::new();
    for palabra in linea.split(' ') {
        let mut palabra = palabra.to_string();
        while palabra.chars().count() > ancho {
            let cabeza: String = palabra.chars().take(ancho).collect();
            palabra = palabra.chars().skip(ancho).collect();
            if !actual.is_empty() {
                trozos.push(std::mem::take(&mut actual));
            }
            trozos.push(cabeza);
        }
        let largo = actual.chars().count() + palabra.chars().count() + usize::from(!actual.is_empty());
        if largo > ancho && !actual.is_empty() {
            trozos.push(std::mem::take(&mut actual));
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(&palabra);
    }
    trozos.push(actual);
    trozos
}

fn ext_crear_pdf(args: Vec<Valor>) -> Result<Valor, String> {
    let archivo = match args.first().and_then(|valor| valor.como_texto()) {
        Some(archivo) if archivo.to_lowercase().ends_with(".pdf") => archivo.to_string(),
        _ => return Err("crear_pdf() espera como primer argumento el nombre de un archivo que termine en .pdf".to_string()),
    };
    let titulo = match args.get(1) {
        Some(valor) => texto_de(valor),
        None => String::new(),
    };
    let lineas: Vec<String> = match args.get(2) {
        Some(valor) => match valor.como_lista() {
            Some(elementos) => elementos.iter().map(texto_de).collect(),
            None => texto_de(valor).lines().map(str::to_string).collect(),
        },
        None => Vec::new(),
    };

    // A4: 595 x 842 puntos; márgenes de 50; Helvetica de 11 puntos (unos 90 caracteres por línea).
    let por_pagina = 46;
    let mut renglones = Vec::new();
    for linea in &lineas {
        renglones.extend(ext_pdf_partir(linea, 90));
    }
    let mut paginas: Vec<Vec<String>> = renglones.chunks(por_pagina).map(|trozo| trozo.to_vec()).collect();
    if paginas.is_empty() {
        paginas.push(Vec::new());
    }

    let mut objetos: Vec<Vec<u8>> = Vec::new();
    // 1 catálogo, 2 páginas, 3 fuente normal, 4 fuente negrita; después página + contenido.
    objetos.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let hijos: Vec<String> = (0..paginas.len()).map(|i| format!("{} 0 R", 5 + 2 * i)).collect();
    objetos.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", hijos.join(" "), paginas.len()).into_bytes());
    objetos.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    objetos.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    for (numero, pagina) in paginas.iter().enumerate() {
        let mut contenido = Vec::new();
        let mut y = 792;
        if numero == 0 && !titulo.is_empty() {
            contenido.extend_from_slice(format!("BT /F2 16 Tf 50 {} Td (", y).as_bytes());
            contenido.extend(ext_pdf_codificar(&titulo));
            contenido.extend_from_slice(b") Tj ET\n");
            y -= 28;
        }
        for renglon in pagina {
            contenido.extend_from_slice(format!("BT /F1 11 Tf 50 {} Td (", y).as_bytes());
            contenido.extend(ext_pdf_codificar(renglon));
            contenido.extend_from_slice(b") Tj ET\n");
            y -= 15;
        }
        contenido.extend_from_slice(
            format!("BT /F1 8 Tf 520 30 Td (Pagina {} de {}) Tj ET\n", numero + 1, paginas.len()).as_bytes(),
        );
        objetos.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                6 + 2 * numero
            )
            .into_bytes(),
        );
        let mut flujo = format!("<< /Length {} >>\nstream\n", contenido.len()).into_bytes();
        flujo.extend(contenido);
        flujo.extend_from_slice(b"\nendstream");
        objetos.push(flujo);
    }

    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut posiciones = Vec::new();
    for (indice, objeto) in objetos.iter().enumerate() {
        posiciones.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", indice + 1).as_bytes());
        pdf.extend_from_slice(objeto);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let inicio_tabla = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objetos.len() + 1).as_bytes());
    for posicion in posiciones {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", posicion).as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", objetos.len() + 1, inicio_tabla).as_bytes(),
    );
    std::fs::write(&archivo, pdf).map_err(|error| format!("crear_pdf(): no se pudo escribir '{}': {}", archivo, error))?;
    Ok(Valor::Nada)
}
