fn ext_gritar(args: Vec<Valor>) -> Result<Valor, String> {
    match args.first().and_then(|valor| valor.como_texto()) {
        Some(texto) => Ok(Valor::texto(format!("¡{}!", texto.to_uppercase()))),
        None => Err("gritar() espera un texto".to_string()),
    }
}
