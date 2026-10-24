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

// ---------------------------------------------------------------------------
// Fechas completas (segundos desde 1970, zonas horarias y formato estilo PHP).
// Todo con la biblioteca estándar de Rust.
// ---------------------------------------------------------------------------

const TIEMPO_ANIO_MIN: i64 = -9999;
const TIEMPO_ANIO_MAX: i64 = 9999;
const TIEMPO_MAX_ARCHIVO_ZONA: u64 = 4 * 1024 * 1024;
const TIEMPO_MAX_TRANSICIONES: u32 = 200_000;
const TIEMPO_MAX_TIPOS: u32 = 1_000;
const TIEMPO_MAX_CARACTERES: u32 = 200_000;
const TIEMPO_MAX_SALTOS: u32 = 100_000;

fn tiempo_numero(args: &[Valor], indice: usize, funcion: &str) -> Result<f64, String> {
    match args.get(indice).and_then(|valor| valor.como_numero()) {
        Some(numero) if numero.is_finite() => Ok(numero),
        Some(_) => Err(format!(
            "{}() espera un número finito en el argumento {}",
            funcion,
            indice + 1
        )),
        None => Err(format!(
            "{}() espera un número en el argumento {}",
            funcion,
            indice + 1
        )),
    }
}

fn tiempo_entero(args: &[Valor], indice: usize, funcion: &str) -> Result<i64, String> {
    let numero = tiempo_numero(args, indice, funcion)?;
    if numero.fract() != 0.0 {
        return Err(format!(
            "{}() espera un número entero en el argumento {}",
            funcion,
            indice + 1
        ));
    }
    Ok(numero as i64)
}

fn tiempo_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
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

fn tiempo_bisiesto(anio: i64) -> bool {
    (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0
}

fn tiempo_dias_del_mes(anio: i64, mes: i64) -> i64 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if tiempo_bisiesto(anio) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn tiempo_mes_nombre(mes: i64) -> &'static str {
    match mes {
        1 => "enero",
        2 => "febrero",
        3 => "marzo",
        4 => "abril",
        5 => "mayo",
        6 => "junio",
        7 => "julio",
        8 => "agosto",
        9 => "septiembre",
        10 => "octubre",
        11 => "noviembre",
        12 => "diciembre",
        _ => "mes",
    }
}

/// Días desde 1970-01-01 de una fecha civil (algoritmo de Howard Hinnant).
fn tiempo_dias_desde_civil(anio: i64, mes: i64, dia: i64) -> i64 {
    let anio = anio - if mes <= 2 { 1 } else { 0 };
    let era = anio.div_euclid(400);
    let anio_era = anio - era * 400;
    let dia_anio = (153 * (mes + if mes > 2 { -3 } else { 9 }) + 2) / 5 + dia - 1;
    let dia_era = anio_era * 365 + anio_era / 4 - anio_era / 100 + dia_anio;
    era * 146_097 + dia_era - 719_468
}

/// Fecha civil (año, mes, día) a partir de los días desde 1970-01-01.
fn tiempo_civil_desde_dias(dias: i64) -> (i64, i64, i64) {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let dia_era = z - era * 146_097;
    let anio_era = (dia_era - dia_era / 1_460 + dia_era / 36_524 - dia_era / 146_096) / 365;
    let dia_anio = dia_era - (365 * anio_era + anio_era / 4 - anio_era / 100);
    let mes_aproximado = (5 * dia_anio + 2) / 153;
    let dia = dia_anio - (153 * mes_aproximado + 2) / 5 + 1;
    let mes = if mes_aproximado < 10 {
        mes_aproximado + 3
    } else {
        mes_aproximado - 9
    };
    let anio = anio_era + era * 400 + if mes <= 2 { 1 } else { 0 };
    (anio, mes, dia)
}

fn tiempo_campos_validos(anio: i64, mes: i64, dia: i64, hora: i64, minuto: i64, segundo: i64) -> bool {
    (TIEMPO_ANIO_MIN..=TIEMPO_ANIO_MAX).contains(&anio)
        && (1..=12).contains(&mes)
        && dia >= 1
        && dia <= tiempo_dias_del_mes(anio, mes)
        && (0..=23).contains(&hora)
        && (0..=59).contains(&minuto)
        && (0..=59).contains(&segundo)
}

fn tiempo_segundos_civil(anio: i64, mes: i64, dia: i64, hora: i64, minuto: i64, segundo: i64) -> i64 {
    tiempo_dias_desde_civil(anio, mes, dia) * 86_400 + hora * 3_600 + minuto * 60 + segundo
}

fn tiempo_validar_campos(
    anio: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
    funcion: &str,
) -> Result<(), String> {
    if !(TIEMPO_ANIO_MIN..=TIEMPO_ANIO_MAX).contains(&anio) {
        return Err(format!(
            "{}() espera un año entre {} y {}",
            funcion, TIEMPO_ANIO_MIN, TIEMPO_ANIO_MAX
        ));
    }
    if !(1..=12).contains(&mes) {
        return Err(format!(
            "{}(): el mes {} no existe (tiene que ser de 1 a 12)",
            funcion, mes
        ));
    }
    let dias = tiempo_dias_del_mes(anio, mes);
    if !(1..=dias).contains(&dia) {
        return Err(format!(
            "{}(): el {} de {} de {} no existe ({} tiene {} días)",
            funcion,
            dia,
            tiempo_mes_nombre(mes),
            anio,
            tiempo_mes_nombre(mes),
            dias
        ));
    }
    if !(0..=23).contains(&hora) {
        return Err(format!(
            "{}(): la hora {} no existe (tiene que ser de 0 a 23)",
            funcion, hora
        ));
    }
    if !(0..=59).contains(&minuto) {
        return Err(format!(
            "{}(): el minuto {} no existe (tiene que ser de 0 a 59)",
            funcion, minuto
        ));
    }
    if !(0..=59).contains(&segundo) {
        return Err(format!(
            "{}(): el segundo {} no existe (tiene que ser de 0 a 59)",
            funcion, segundo
        ));
    }
    Ok(())
}

fn ext_crear_fecha(args: Vec<Valor>) -> Result<Valor, String> {
    let anio = tiempo_entero(&args, 0, "crear_fecha")?;
    let mes = tiempo_entero(&args, 1, "crear_fecha")?;
    let dia = tiempo_entero(&args, 2, "crear_fecha")?;
    let hora = tiempo_entero(&args, 3, "crear_fecha")?;
    let minuto = tiempo_entero(&args, 4, "crear_fecha")?;
    let segundo = tiempo_entero(&args, 5, "crear_fecha")?;
    tiempo_validar_campos(anio, mes, dia, hora, minuto, segundo, "crear_fecha")?;
    Ok(Valor::Numero(tiempo_segundos_civil(
        anio, mes, dia, hora, minuto, segundo,
    ) as f64))
}

/// Desfase fijo escrito como "-05:00", "+01:00", "-0500", "+01" o "Z".
fn tiempo_desfase_fijo(texto: &str) -> Option<i64> {
    let texto = texto.trim();
    if texto.eq_ignore_ascii_case("Z") {
        return Some(0);
    }
    let primero = texto.chars().next()?;
    let signo = match primero {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let resto = &texto[primero.len_utf8()..];
    let (horas, minutos) = match resto.split_once(':') {
        Some((horas, minutos)) => (horas, minutos),
        None => match resto.len() {
            4 => (&resto[..2], &resto[2..]),
            1 | 2 => (resto, "0"),
            _ => return None,
        },
    };
    if horas.is_empty() || horas.len() > 2 || minutos.len() > 2 {
        return None;
    }
    if !horas.bytes().all(|b| b.is_ascii_digit()) || !minutos.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let horas: i64 = horas.parse().ok()?;
    let minutos: i64 = minutos.parse().ok()?;
    if horas > 23 || minutos > 59 {
        return None;
    }
    Some(signo * (horas * 3_600 + minutos * 60))
}

struct TiempoBloqueTzif {
    transiciones: Vec<i64>,
    indices: Vec<u8>,
    desfases: Vec<i32>,
    es_verano: Vec<bool>,
    fin: usize,
}

fn tiempo_cuenta_tzif(datos: &[u8], inicio: usize, desde: usize) -> u32 {
    u32::from_be_bytes([
        datos[inicio + desde],
        datos[inicio + desde + 1],
        datos[inicio + desde + 2],
        datos[inicio + desde + 3],
    ])
}

/// Lee un bloque de datos TZif (cabecera incluida) que empieza en `inicio`.
/// Con `largo` usa tiempos de 64 bits (versión 2/3/4).
fn tiempo_tzif_bloque(datos: &[u8], inicio: usize, largo: bool) -> Result<TiempoBloqueTzif, String> {
    let fin_cabecera = inicio
        .checked_add(44)
        .ok_or_else(|| "el archivo TZif es demasiado grande".to_string())?;
    if fin_cabecera > datos.len() {
        return Err("el archivo TZif está cortado en la cabecera".to_string());
    }
    if &datos[inicio..inicio + 4] != b"TZif" {
        return Err("el archivo de zona horaria no empieza por 'TZif'".to_string());
    }
    let ttisgmtcnt = tiempo_cuenta_tzif(datos, inicio, 20);
    let ttisstdcnt = tiempo_cuenta_tzif(datos, inicio, 24);
    let saltos = tiempo_cuenta_tzif(datos, inicio, 28);
    let transiciones = tiempo_cuenta_tzif(datos, inicio, 32);
    let tipos = tiempo_cuenta_tzif(datos, inicio, 36);
    let caracteres = tiempo_cuenta_tzif(datos, inicio, 40);
    if transiciones > TIEMPO_MAX_TRANSICIONES
        || tipos > TIEMPO_MAX_TIPOS
        || caracteres > TIEMPO_MAX_CARACTERES
        || saltos > TIEMPO_MAX_SALTOS
        || ttisstdcnt > tipos
        || ttisgmtcnt > tipos
    {
        return Err("el archivo de zona horaria pide más datos de los permitidos".to_string());
    }
    if tipos == 0 {
        return Err("el archivo de zona horaria no tiene ningún tipo de hora".to_string());
    }
    let tam = if largo { 8u64 } else { 4u64 };
    let total = transiciones as u64 * tam
        + transiciones as u64
        + tipos as u64 * 6
        + caracteres as u64
        + saltos as u64 * (tam + 4)
        + ttisstdcnt as u64
        + ttisgmtcnt as u64;
    let fin = (inicio as u64 + 44 + total) as usize;
    if fin > datos.len() {
        return Err("el archivo de zona horaria está cortado".to_string());
    }
    let mut pos = fin_cabecera;
    let mut lista_transiciones = Vec::with_capacity(transiciones as usize);
    for _ in 0..transiciones {
        let valor = if largo {
            let bytes = [
                datos[pos], datos[pos + 1], datos[pos + 2], datos[pos + 3],
                datos[pos + 4], datos[pos + 5], datos[pos + 6], datos[pos + 7],
            ];
            i64::from_be_bytes(bytes)
        } else {
            let bytes = [datos[pos], datos[pos + 1], datos[pos + 2], datos[pos + 3]];
            i32::from_be_bytes(bytes) as i64
        };
        lista_transiciones.push(valor);
        pos += tam as usize;
    }
    let lista_indices = datos[pos..pos + transiciones as usize].to_vec();
    if lista_indices.iter().any(|indice| *indice as u32 >= tipos) {
        return Err("el archivo de zona horaria tiene un tipo de hora que no existe".to_string());
    }
    pos += transiciones as usize;
    let mut desfases = Vec::with_capacity(tipos as usize);
    let mut es_verano = Vec::with_capacity(tipos as usize);
    for _ in 0..tipos {
        let bytes = [datos[pos], datos[pos + 1], datos[pos + 2], datos[pos + 3]];
        let desfase = i32::from_be_bytes(bytes);
        let verano = datos[pos + 4];
        let diseno = datos[pos + 5];
        if verano > 1 {
            return Err("el archivo de zona horaria tiene un tipo de hora inválido".to_string());
        }
        if caracteres > 0 && diseno as u32 >= caracteres {
            return Err("el archivo de zona horaria tiene un nombre de hora inválido".to_string());
        }
        desfases.push(desfase);
        es_verano.push(verano == 1);
        pos += 6;
    }
    Ok(TiempoBloqueTzif {
        transiciones: lista_transiciones,
        indices: lista_indices,
        desfases,
        es_verano,
        fin,
    })
}

/// Desfase (en segundos) de la zona IANA de un instante UTC.
fn tiempo_tzif_desfase(datos: &[u8], utc: i64) -> Result<i64, String> {
    if datos.len() < 44 || &datos[0..4] != b"TZif" {
        return Err("el archivo de zona horaria no es TZif".to_string());
    }
    let version = datos[4];
    let bloque = match version {
        0 | b'1' => tiempo_tzif_bloque(datos, 0, false)?,
        b'2' | b'3' | b'4' => {
            let primero = tiempo_tzif_bloque(datos, 0, false)?;
            tiempo_tzif_bloque(datos, primero.fin, true)?
        }
        otra => {
            return Err(format!(
                "la versión {} de los archivos de zona horaria no está soportada (usa 2 o 3)",
                otra
            ))
        }
    };
    let posicion = bloque.transiciones.partition_point(|transicion| *transicion <= utc);
    let indice = if posicion == 0 {
        bloque
            .es_verano
            .iter()
            .position(|verano| !*verano)
            .unwrap_or(0)
    } else {
        bloque.indices[posicion - 1] as usize
    };
    bloque
        .desfases
        .get(indice)
        .map(|desfase| *desfase as i64)
        .ok_or_else(|| "el archivo de zona horaria no tiene el desfase pedido".to_string())
}

fn tiempo_desfase_zona(zona: &str, utc: i64) -> Result<i64, String> {
    let zona = zona.trim();
    if zona.is_empty() || zona.eq_ignore_ascii_case("UTC") {
        return Ok(0);
    }
    if let Some(desfase) = tiempo_desfase_fijo(zona) {
        return Ok(desfase);
    }
    let valida = !zona.starts_with('/')
        && !zona.contains("..")
        && !zona.contains('\\')
        && zona
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+' | '.'));
    if !valida {
        return Err(format!(
            "la zona horaria '{}' no es válida (usa \"UTC\", un desfase como \"-05:00\" o un nombre como \"America/Lima\")",
            zona
        ));
    }
    let ruta = format!("/usr/share/zoneinfo/{}", zona);
    let informacion = std::fs::metadata(&ruta).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "no se encontró la zona horaria '{}': falta el archivo {}",
                zona, ruta
            )
        } else {
            format!("no se pudo leer la zona horaria '{}' ({}): {}", zona, ruta, error)
        }
    })?;
    if !informacion.is_file() {
        return Err(format!("la zona horaria '{}' no es un archivo", zona));
    }
    if informacion.len() > TIEMPO_MAX_ARCHIVO_ZONA {
        return Err(format!(
            "el archivo de la zona horaria '{}' es demasiado grande",
            zona
        ));
    }
    let datos = std::fs::read(&ruta)
        .map_err(|error| format!("no se pudo leer la zona horaria '{}': {}", zona, error))?;
    tiempo_tzif_desfase(&datos, utc)
        .map_err(|error| format!("la zona horaria '{}' no se pudo leer: {}", zona, error))
}

fn tiempo_partir_anio_mes_dia(fecha: &str) -> Option<(i64, i64, i64)> {
    let partes: Vec<&str> = fecha.split('-').collect();
    if partes.len() != 3 {
        return None;
    }
    Some((
        partes[0].parse().ok()?,
        partes[1].parse().ok()?,
        partes[2].parse().ok()?,
    ))
}

fn tiempo_partir_hora(hora: &str) -> Option<(i64, i64, i64)> {
    let partes: Vec<&str> = hora.split(':').collect();
    if partes.len() < 2 || partes.len() > 3 {
        return None;
    }
    Some((
        partes[0].parse().ok()?,
        partes[1].parse().ok()?,
        if partes.len() == 3 {
            partes[2].parse().ok()?
        } else {
            0
        },
    ))
}

fn tiempo_separar_desfase(hora: &str) -> Option<(&str, i64)> {
    let hora = hora.trim();
    if let Some(sin_z) = hora.strip_suffix('Z').or_else(|| hora.strip_suffix('z')) {
        return Some((sin_z, 0));
    }
    if let Some(posicion) = hora.find(['+', '-']) {
        let (hora, desfase) = hora.split_at(posicion);
        return Some((hora, tiempo_desfase_fijo(desfase)?));
    }
    Some((hora, 0))
}

fn tiempo_leer_fecha(texto: &str) -> Option<i64> {
    let texto = texto.trim();
    if texto.is_empty() {
        return None;
    }
    let (anio, mes, dia, hora, minuto, segundo, desfase) = if let Some((fecha, hora)) = texto.split_once('T') {
        let (hora, desfase) = tiempo_separar_desfase(hora)?;
        let (anio, mes, dia) = tiempo_partir_anio_mes_dia(fecha)?;
        let (hora, minuto, segundo) = tiempo_partir_hora(hora)?;
        (anio, mes, dia, hora, minuto, segundo, desfase)
    } else if let Some((fecha, hora)) = texto.split_once(|c: char| c.is_whitespace()) {
        let (anio, mes, dia) = tiempo_partir_anio_mes_dia(fecha)?;
        let (hora, minuto, segundo) = tiempo_partir_hora(hora.trim())?;
        (anio, mes, dia, hora, minuto, segundo, 0)
    } else if texto.contains('/') {
        let partes: Vec<&str> = texto.split('/').collect();
        if partes.len() != 3 {
            return None;
        }
        (
            partes[2].parse().ok()?,
            partes[1].parse().ok()?,
            partes[0].parse().ok()?,
            0,
            0,
            0,
            0,
        )
    } else {
        let (anio, mes, dia) = tiempo_partir_anio_mes_dia(texto)?;
        (anio, mes, dia, 0, 0, 0, 0)
    };
    if !tiempo_campos_validos(anio, mes, dia, hora, minuto, segundo) {
        return None;
    }
    Some(
        tiempo_segundos_civil(anio, mes, dia, hora, minuto, segundo)
            .checked_sub(desfase)?,
    )
}

fn ext_leer_fecha(args: Vec<Valor>) -> Result<Valor, String> {
    let texto = tiempo_texto(&args, 0, "leer_fecha")?;
    match tiempo_leer_fecha(&texto) {
        Some(segundos) => Ok(Valor::Numero(segundos as f64)),
        None => Ok(Valor::Nada),
    }
}

const TIEMPO_MESES_ES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto",
    "septiembre", "octubre", "noviembre", "diciembre",
];
const TIEMPO_MESES_CORTOS_ES: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];
const TIEMPO_DIAS_ES: [&str; 7] = [
    "lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo",
];
const TIEMPO_DIAS_CORTOS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn tiempo_anio_texto(anio: i64) -> String {
    if (0..=9999).contains(&anio) {
        format!("{:04}", anio)
    } else {
        anio.to_string()
    }
}

fn tiempo_formatear(segundos: i64, formato: &str, desfase: i64) -> Result<String, String> {
    let local = segundos
        .checked_add(desfase)
        .ok_or_else(|| "formatear_fecha(): la fecha se sale del rango".to_string())?;
    let dias = local.div_euclid(86_400);
    let resto = local.rem_euclid(86_400);
    let (anio, mes, dia) = tiempo_civil_desde_dias(dias);
    let hora = resto / 3_600;
    let minuto = (resto % 3_600) / 60;
    let segundo = resto % 60;
    let semana = (dias + 3).rem_euclid(7) as usize; // 0 = lunes
    let domingo = (dias + 4).rem_euclid(7) as usize; // 0 = domingo
    let dia_anio = dias - tiempo_dias_desde_civil(anio, 1, 1);
    let mut salida = String::new();
    let mut letras = formato.chars();
    while let Some(letra) = letras.next() {
        if letra == '\\' {
            if let Some(escapada) = letras.next() {
                salida.push(escapada);
            } else {
                salida.push('\\');
            }
            continue;
        }
        match letra {
            'Y' => salida.push_str(&tiempo_anio_texto(anio)),
            'y' => salida.push_str(&format!("{:02}", anio.rem_euclid(100))),
            'm' => salida.push_str(&format!("{:02}", mes)),
            'n' => salida.push_str(&mes.to_string()),
            'd' => salida.push_str(&format!("{:02}", dia)),
            'j' => salida.push_str(&dia.to_string()),
            'H' => salida.push_str(&format!("{:02}", hora)),
            'G' => salida.push_str(&hora.to_string()),
            'i' => salida.push_str(&format!("{:02}", minuto)),
            's' => salida.push_str(&format!("{:02}", segundo)),
            'D' => salida.push_str(TIEMPO_DIAS_CORTOS_EN[semana]),
            'l' => salida.push_str(TIEMPO_DIAS_ES[semana]),
            'N' => salida.push_str(&(semana + 1).to_string()),
            'w' => salida.push_str(&domingo.to_string()),
            'z' => salida.push_str(&dia_anio.to_string()),
            't' => salida.push_str(&tiempo_dias_del_mes(anio, mes).to_string()),
            'L' => salida.push(if tiempo_bisiesto(anio) { '1' } else { '0' }),
            'A' => salida.push_str(if hora < 12 { "AM" } else { "PM" }),
            'a' => salida.push_str(if hora < 12 { "am" } else { "pm" }),
            'g' => {
                let hora12 = if hora % 12 == 0 { 12 } else { hora % 12 };
                salida.push_str(&hora12.to_string());
            }
            'h' => {
                let hora12 = if hora % 12 == 0 { 12 } else { hora % 12 };
                salida.push_str(&format!("{:02}", hora12));
            }
            'U' => salida.push_str(&segundos.to_string()),
            'F' => salida.push_str(TIEMPO_MESES_ES[(mes - 1) as usize]),
            'M' => salida.push_str(TIEMPO_MESES_CORTOS_ES[(mes - 1) as usize]),
            otra => salida.push(otra),
        }
    }
    Ok(salida)
}

fn ext_formatear_fecha(args: Vec<Valor>) -> Result<Valor, String> {
    let segundos = tiempo_entero(&args, 0, "formatear_fecha")?;
    let formato = tiempo_texto(&args, 1, "formatear_fecha")?;
    let zona = tiempo_texto(&args, 2, "formatear_fecha")?;
    let desfase = tiempo_desfase_zona(&zona, segundos)?;
    Ok(Valor::texto(tiempo_formatear(segundos, &formato, desfase)?))
}

fn tiempo_sumar_meses(segundos: i64, meses: i64) -> Result<i64, String> {
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    let (anio, mes, dia) = tiempo_civil_desde_dias(dias);
    let total = (anio * 12 + (mes - 1))
        .checked_add(meses)
        .ok_or_else(|| "sumar_tiempo(): la fecha se sale del rango".to_string())?;
    let mut nuevo_anio = total.div_euclid(12);
    let mut nuevo_mes = total.rem_euclid(12) + 1;
    let mut nuevo_dia = dia;
    let dias_mes = tiempo_dias_del_mes(nuevo_anio, nuevo_mes);
    if nuevo_dia > dias_mes {
        // Como PHP: 31 de enero + 1 mes = 3 de marzo (no se recorta al último día).
        nuevo_dia -= dias_mes;
        if nuevo_mes == 12 {
            nuevo_mes = 1;
            nuevo_anio += 1;
        } else {
            nuevo_mes += 1;
        }
    }
    if !(TIEMPO_ANIO_MIN..=TIEMPO_ANIO_MAX).contains(&nuevo_anio) {
        return Err("sumar_tiempo(): la fecha resultante se sale del rango".to_string());
    }
    Ok(tiempo_dias_desde_civil(nuevo_anio, nuevo_mes, nuevo_dia) * 86_400 + resto)
}

fn ext_sumar_tiempo(args: Vec<Valor>) -> Result<Valor, String> {
    let segundos = tiempo_entero(&args, 0, "sumar_tiempo")?;
    let cantidad = tiempo_entero(&args, 1, "sumar_tiempo")?;
    let unidad = tiempo_texto(&args, 2, "sumar_tiempo")?.to_lowercase();
    let resultado = match unidad.as_str() {
        "segundos" | "segundo" => cantidad
            .checked_mul(1)
            .and_then(|total| segundos.checked_add(total)),
        "minutos" | "minuto" => cantidad
            .checked_mul(60)
            .and_then(|total| segundos.checked_add(total)),
        "horas" | "hora" => cantidad
            .checked_mul(3_600)
            .and_then(|total| segundos.checked_add(total)),
        "dias" | "días" | "dia" | "día" => cantidad
            .checked_mul(86_400)
            .and_then(|total| segundos.checked_add(total)),
        "semanas" | "semana" => cantidad
            .checked_mul(604_800)
            .and_then(|total| segundos.checked_add(total)),
        "meses" | "mes" => return tiempo_sumar_meses(segundos, cantidad).map(|total| Valor::Numero(total as f64)),
        "años" | "anos" | "año" | "ano" => {
            let meses = cantidad
                .checked_mul(12)
                .ok_or_else(|| "sumar_tiempo(): la cantidad de años es demasiado grande".to_string())?;
            return tiempo_sumar_meses(segundos, meses).map(|total| Valor::Numero(total as f64));
        }
        otra => {
            return Err(format!(
                "sumar_tiempo() no conoce la unidad '{}'; usa \"segundos\", \"minutos\", \"horas\", \"dias\", \"semanas\", \"meses\" o \"años\"",
                otra
            ))
        }
    };
    match resultado {
        Some(total) => Ok(Valor::Numero(total as f64)),
        None => Err("sumar_tiempo(): la fecha se sale del rango".to_string()),
    }
}

/// Meses completos entre dos instantes (positivo si `a` es posterior a `b`).
/// Se cuenta como PHP: si sumar esos meses a `b` se pasa de `a`, se quita uno.
fn tiempo_meses_entre(a: i64, b: i64) -> i64 {
    let dias_a = a.div_euclid(86_400);
    let dias_b = b.div_euclid(86_400);
    let (anio_a, mes_a, _) = tiempo_civil_desde_dias(dias_a);
    let (anio_b, mes_b, _) = tiempo_civil_desde_dias(dias_b);
    let mut meses = (anio_a * 12 + (mes_a - 1)) - (anio_b * 12 + (mes_b - 1));
    if meses > 0 {
        while meses > 0 {
            match tiempo_sumar_meses(b, meses) {
                Ok(candidato) if candidato > a => meses -= 1,
                _ => break,
            }
        }
    } else if meses < 0 {
        while meses < 0 {
            match tiempo_sumar_meses(b, meses) {
                Ok(candidato) if candidato < a => meses += 1,
                _ => break,
            }
        }
    }
    meses
}

fn ext_diferencia_fechas(args: Vec<Valor>) -> Result<Valor, String> {
    let a = tiempo_entero(&args, 0, "diferencia_fechas")?;
    let b = tiempo_entero(&args, 1, "diferencia_fechas")?;
    let unidad = tiempo_texto(&args, 2, "diferencia_fechas")?.to_lowercase();
    let diferencia = a
        .checked_sub(b)
        .ok_or_else(|| "diferencia_fechas(): las fechas se salen del rango".to_string())?;
    let resultado = match unidad.as_str() {
        "segundos" | "segundo" => diferencia,
        "minutos" | "minuto" => diferencia / 60,
        "horas" | "hora" => diferencia / 3_600,
        "dias" | "días" | "dia" | "día" => diferencia / 86_400,
        "semanas" | "semana" => diferencia / 604_800,
        "meses" | "mes" => tiempo_meses_entre(a, b),
        "años" | "anos" | "año" | "ano" => tiempo_meses_entre(a, b) / 12,
        otra => {
            return Err(format!(
                "diferencia_fechas() no conoce la unidad '{}'; usa \"segundos\", \"minutos\", \"horas\", \"dias\", \"semanas\", \"meses\" o \"años\"",
                otra
            ))
        }
    };
    Ok(Valor::Numero(resultado as f64))
}

fn ext_dia_de_la_semana(args: Vec<Valor>) -> Result<Valor, String> {
    let segundos = tiempo_entero(&args, 0, "dia_de_la_semana")?;
    let zona = tiempo_texto(&args, 1, "dia_de_la_semana")?;
    let desfase = tiempo_desfase_zona(&zona, segundos)?;
    let local = segundos
        .checked_add(desfase)
        .ok_or_else(|| "dia_de_la_semana(): la fecha se sale del rango".to_string())?;
    let dias = local.div_euclid(86_400);
    Ok(Valor::Numero(((dias + 3).rem_euclid(7) + 1) as f64))
}

fn ext_es_bisiesto(args: Vec<Valor>) -> Result<Valor, String> {
    let anio = tiempo_entero(&args, 0, "es_bisiesto")?;
    if !(TIEMPO_ANIO_MIN..=TIEMPO_ANIO_MAX).contains(&anio) {
        return Err(format!(
            "es_bisiesto() espera un año entre {} y {}",
            TIEMPO_ANIO_MIN, TIEMPO_ANIO_MAX
        ));
    }
    Ok(Valor::Logico(tiempo_bisiesto(anio)))
}

fn ext_dias_del_mes(args: Vec<Valor>) -> Result<Valor, String> {
    let anio = tiempo_entero(&args, 0, "dias_del_mes")?;
    let mes = tiempo_entero(&args, 1, "dias_del_mes")?;
    if !(TIEMPO_ANIO_MIN..=TIEMPO_ANIO_MAX).contains(&anio) {
        return Err(format!(
            "dias_del_mes() espera un año entre {} y {}",
            TIEMPO_ANIO_MIN, TIEMPO_ANIO_MAX
        ));
    }
    if !(1..=12).contains(&mes) {
        return Err(format!(
            "dias_del_mes(): el mes {} no existe (tiene que ser de 1 a 12)",
            mes
        ));
    }
    Ok(Valor::Numero(tiempo_dias_del_mes(anio, mes) as f64))
}
