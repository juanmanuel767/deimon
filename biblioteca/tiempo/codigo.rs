fn ext_ahora(_args: Vec<Valor>) -> Result<Valor, String> {
    let segundos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "ahora(): el reloj del sistema está antes de 1970".to_string())?
        .as_secs_f64();
    Ok(Valor::Numero(segundos))
}

fn ext_esperar(args: Vec<Valor>) -> Result<Valor, String> {
    match args.first().and_then(|valor| valor.como_numero()) {
        Some(ms) if ms >= 0.0 => {
            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            Ok(Valor::Nada)
        }
        _ => Err("esperar() espera un número de milisegundos que no sea negativo".to_string()),
    }
}

fn ext_fecha(args: Vec<Valor>) -> Result<Valor, String> {
    let total = match args.first().and_then(|valor| valor.como_numero()) {
        Some(segundos) => segundos.floor() as i64,
        None => return Err("fecha() espera un número de segundos desde 1970".to_string()),
    };
    let dias = total.div_euclid(86_400);
    let resto = total.rem_euclid(86_400);
    // Conversión de días desde 1970 a fecha civil (algoritmo de Howard Hinnant).
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let anio = yoe + era * 400 + if mes <= 2 { 1 } else { 0 };
    Ok(Valor::texto(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        anio,
        mes,
        dia,
        resto / 3_600,
        (resto % 3_600) / 60,
        resto % 60
    )))
}
