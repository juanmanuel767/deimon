const EXT_GRAFICOS_COLORES: [&str; 8] = ["#7c3aed", "#0ea5e9", "#f59e0b", "#10b981", "#ef4444", "#ec4899", "#64748b", "#84cc16"];

fn ext_graficos_xml(texto: &str) -> String {
    texto.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn ext_graficos_datos(args: &[Valor], funcion: &str) -> Result<(String, Vec<String>, Vec<f64>), String> {
    let titulo = args.first().map(texto_de).unwrap_or_default();
    let etiquetas: Vec<String> = match args.get(1).and_then(|v| v.como_lista()) {
        Some(lista) => lista.iter().map(texto_de).collect(),
        None => return Err(format!("{}(): las etiquetas van en una lista", funcion)),
    };
    let valores: Vec<f64> = match args.get(2).and_then(|v| v.como_lista()) {
        Some(lista) => lista
            .iter()
            .map(|v| v.como_numero().ok_or_else(|| format!("{}(): los valores deben ser números", funcion)))
            .collect::<Result<_, _>>()?,
        None => return Err(format!("{}(): los valores van en una lista", funcion)),
    };
    if etiquetas.len() != valores.len() || valores.is_empty() {
        return Err(format!("{}(): hace falta el mismo número de etiquetas que de valores (y al menos uno)", funcion));
    }
    Ok((titulo, etiquetas, valores))
}

fn ext_graficos_numero(n: f64) -> String {
    texto_de(&Valor::Numero((n * 100.0).round() / 100.0))
}

fn ext_graficos_inicio(titulo: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 640 400\" width=\"640\" height=\"400\" font-family=\"sans-serif\"><rect width=\"640\" height=\"400\" fill=\"#ffffff\"/><text x=\"320\" y=\"32\" text-anchor=\"middle\" font-size=\"20\" font-weight=\"bold\" fill=\"#1f2937\">{}</text>",
        ext_graficos_xml(titulo)
    )
}

/// Ejes y escala comunes de barras y líneas. Devuelve (svg, mínimo, máximo).
fn ext_graficos_ejes(valores: &[f64]) -> (String, f64, f64) {
    let maximo = valores.iter().cloned().fold(0.0_f64, f64::max).max(1e-9);
    let minimo = valores.iter().cloned().fold(0.0_f64, f64::min);
    let mut svg = String::new();
    for i in 0..=4 {
        let valor = minimo + (maximo - minimo) * i as f64 / 4.0;
        let y = 340.0 - 280.0 * i as f64 / 4.0;
        svg.push_str(&format!(
            "<line x1=\"70\" y1=\"{y}\" x2=\"610\" y2=\"{y}\" stroke=\"#e5e7eb\"/><text x=\"62\" y=\"{}\" text-anchor=\"end\" font-size=\"12\" fill=\"#6b7280\">{}</text>",
            y + 4.0,
            ext_graficos_numero(valor)
        ));
    }
    (svg, minimo, maximo)
}

fn ext_grafico_barras(args: Vec<Valor>) -> Result<Valor, String> {
    let (titulo, etiquetas, valores) = ext_graficos_datos(&args, "grafico_barras")?;
    let (ejes, minimo, maximo) = ext_graficos_ejes(&valores);
    let mut svg = ext_graficos_inicio(&titulo) + &ejes;
    let ancho = 540.0 / valores.len() as f64;
    let y_de = |v: f64| 340.0 - 280.0 * (v - minimo) / (maximo - minimo);
    for (i, (etiqueta, valor)) in etiquetas.iter().zip(&valores).enumerate() {
        let x = 70.0 + ancho * i as f64 + ancho * 0.15;
        let (arriba, abajo) = if *valor >= 0.0 { (y_de(*valor), y_de(0.0)) } else { (y_de(0.0), y_de(*valor)) };
        svg.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"{}\"><title>{}: {}</title></rect><text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#374151\">{}</text><text x=\"{:.1}\" y=\"362\" text-anchor=\"middle\" font-size=\"12\" fill=\"#374151\">{}</text>",
            x, arriba, ancho * 0.7, (abajo - arriba).max(1.0), EXT_GRAFICOS_COLORES[0],
            ext_graficos_xml(etiqueta), ext_graficos_numero(*valor),
            x + ancho * 0.35, arriba - 6.0, ext_graficos_numero(*valor),
            x + ancho * 0.35, ext_graficos_xml(etiqueta)
        ));
    }
    svg.push_str("</svg>");
    Ok(Valor::texto(svg))
}

fn ext_grafico_lineas(args: Vec<Valor>) -> Result<Valor, String> {
    let (titulo, etiquetas, valores) = ext_graficos_datos(&args, "grafico_lineas")?;
    let (ejes, minimo, maximo) = ext_graficos_ejes(&valores);
    let mut svg = ext_graficos_inicio(&titulo) + &ejes;
    let paso = if valores.len() > 1 { 520.0 / (valores.len() - 1) as f64 } else { 0.0 };
    let puntos: Vec<(f64, f64)> = valores
        .iter()
        .enumerate()
        .map(|(i, v)| (80.0 + paso * i as f64, 340.0 - 280.0 * (v - minimo) / (maximo - minimo)))
        .collect();
    let trazo: Vec<String> = puntos.iter().map(|(x, y)| format!("{:.1},{:.1}", x, y)).collect();
    svg.push_str(&format!("<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"3\"/>", trazo.join(" "), EXT_GRAFICOS_COLORES[0]));
    for ((x, y), (etiqueta, valor)) in puntos.iter().zip(etiquetas.iter().zip(&valores)) {
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"5\" fill=\"{}\"><title>{}: {}</title></circle><text x=\"{:.1}\" y=\"362\" text-anchor=\"middle\" font-size=\"12\" fill=\"#374151\">{}</text>",
            x, y, EXT_GRAFICOS_COLORES[0], ext_graficos_xml(etiqueta), ext_graficos_numero(*valor), x, ext_graficos_xml(etiqueta)
        ));
    }
    svg.push_str("</svg>");
    Ok(Valor::texto(svg))
}

fn ext_grafico_tarta(args: Vec<Valor>) -> Result<Valor, String> {
    let (titulo, etiquetas, valores) = ext_graficos_datos(&args, "grafico_tarta")?;
    if valores.iter().any(|v| *v < 0.0) {
        return Err("grafico_tarta(): los valores no pueden ser negativos".to_string());
    }
    let total: f64 = valores.iter().sum();
    if total <= 0.0 {
        return Err("grafico_tarta(): la suma de los valores debe ser mayor que 0".to_string());
    }
    let mut svg = ext_graficos_inicio(&titulo);
    let (cx, cy, r) = (230.0_f64, 215.0_f64, 150.0_f64);
    let mut angulo = -std::f64::consts::FRAC_PI_2;
    for (i, (etiqueta, valor)) in etiquetas.iter().zip(&valores).enumerate() {
        let parte = valor / total;
        let color = EXT_GRAFICOS_COLORES[i % EXT_GRAFICOS_COLORES.len()];
        if parte >= 0.9999 {
            svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>", cx, cy, r, color));
        } else if parte > 0.0 {
            let fin = angulo + parte * std::f64::consts::TAU;
            let (x1, y1) = (cx + r * angulo.cos(), cy + r * angulo.sin());
            let (x2, y2) = (cx + r * fin.cos(), cy + r * fin.sin());
            svg.push_str(&format!(
                "<path d=\"M{cx},{cy} L{:.2},{:.2} A{r},{r} 0 {} 1 {:.2},{:.2} Z\" fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"2\"><title>{}: {}</title></path>",
                x1, y1, if parte > 0.5 { 1 } else { 0 }, x2, y2, color, ext_graficos_xml(etiqueta), ext_graficos_numero(*valor)
            ));
            angulo = fin;
        }
        let y = 90.0 + 28.0 * i as f64;
        svg.push_str(&format!(
            "<rect x=\"420\" y=\"{}\" width=\"16\" height=\"16\" rx=\"3\" fill=\"{}\"/><text x=\"444\" y=\"{}\" font-size=\"14\" fill=\"#374151\">{} ({}%)</text>",
            y - 13.0, color, y, ext_graficos_xml(etiqueta), ext_graficos_numero(parte * 100.0)
        ));
    }
    svg.push_str("</svg>");
    Ok(Valor::texto(svg))
}
