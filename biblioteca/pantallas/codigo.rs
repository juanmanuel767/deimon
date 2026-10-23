/// Una línea de la pantalla: tipo y textos separados por tabuladores (escapados).
fn ext_pantallas_linea(tipo: &str, args: &[Valor]) -> Valor {
    let mut linea = tipo.to_string();
    for valor in args {
        linea.push('\t');
        for c in texto_de(valor).chars() {
            match c {
                '\\' => linea.push_str("\\\\"),
                '\t' => linea.push_str("\\t"),
                '\n' => linea.push_str("\\n"),
                '\r' => {}
                otro => linea.push(otro),
            }
        }
    }
    Valor::texto(linea)
}

fn ext_pantallas_camino(args: &[Valor], indice: usize, funcion: &str) -> Result<(), String> {
    match args.get(indice).and_then(|valor| valor.como_texto()) {
        Some(camino) if camino.starts_with('/') => Ok(()),
        _ => Err(format!("{}(): el camino debe empezar por '/', por ejemplo \"/notas\"", funcion)),
    }
}

fn ext_pantalla(args: Vec<Valor>) -> Result<Valor, String> {
    let elementos = match args.first().and_then(|valor| valor.como_lista()) {
        Some(elementos) => elementos,
        None => return Err("pantalla() espera una lista de elementos: pantalla([titulo(\"Hola\"), ...])".to_string()),
    };
    let mut texto = String::from("#deimon-pantalla\n");
    for elemento in elementos {
        texto.push_str(&texto_de(elemento));
        texto.push('\n');
    }
    Ok(Valor::texto(texto))
}

fn ext_titulo(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("titulo", &args))
}

fn ext_parrafo(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("parrafo", &args))
}

fn ext_elemento_lista(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("elemento", &args))
}

fn ext_separador(_args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("separador", &[]))
}

fn ext_boton(args: Vec<Valor>) -> Result<Valor, String> {
    ext_pantallas_camino(&args, 1, "boton")?;
    Ok(ext_pantallas_linea("boton", &args))
}

fn ext_cuadro_texto(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("cuadro", &args))
}

fn ext_cuadro_clave(args: Vec<Valor>) -> Result<Valor, String> {
    Ok(ext_pantallas_linea("clave", &args))
}

fn ext_boton_enviar(args: Vec<Valor>) -> Result<Valor, String> {
    ext_pantallas_camino(&args, 1, "boton_enviar")?;
    Ok(ext_pantallas_linea("enviar", &args))
}
