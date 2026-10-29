fn ext_matematicas_numero(args: &[Valor], indice: usize, funcion: &str) -> Result<f64, String> {
    match args.get(indice) {
        Some(valor) => match valor.como_numero() {
            Some(numero) => Ok(numero),
            None => Err(format!(
                "{}() espera un número en el argumento {}, no {}",
                funcion,
                indice + 1,
                valor.tipo()
            )),
        },
        None => Err(format!("{}() espera un número en el argumento {}", funcion, indice + 1)),
    }
}

fn ext_raiz(args: Vec<Valor>) -> Result<Valor, String> {
    let x = ext_matematicas_numero(&args, 0, "raiz")?;
    if x < 0.0 {
        return Err(format!("raiz() no puede calcular la raíz de un número negativo ({})", texto_de(&args[0])));
    }
    Ok(Valor::Numero(x.sqrt()))
}

fn ext_potencia(args: Vec<Valor>) -> Result<Valor, String> {
    let base = ext_matematicas_numero(&args, 0, "potencia")?;
    let exponente = ext_matematicas_numero(&args, 1, "potencia")?;
    let resultado = base.powf(exponente);
    if resultado.is_nan() {
        return Err("potencia() no tiene resultado real para esos números".to_string());
    }
    Ok(Valor::Numero(resultado))
}

/// Un entero grande ya es entero: redondear, suelo y techo lo devuelven tal cual (sin pasar por f64).
fn ext_matematicas_grande(args: &[Valor]) -> Option<Rc<EnteroGrande>> {
    match args.first() {
        Some(Valor::Grande(entero)) => Some(entero.clone()),
        _ => None,
    }
}

fn ext_absoluto(args: Vec<Valor>) -> Result<Valor, String> {
    if let Some(entero) = ext_matematicas_grande(&args) {
        return Ok(if entero.negativo { Valor::Grande(Rc::new(entero.negado())) } else { Valor::Grande(entero) });
    }
    Ok(Valor::Numero(ext_matematicas_numero(&args, 0, "absoluto")?.abs()))
}

fn ext_redondear(args: Vec<Valor>) -> Result<Valor, String> {
    if let Some(entero) = ext_matematicas_grande(&args) {
        return Ok(Valor::Grande(entero));
    }
    Ok(Valor::Numero(ext_matematicas_numero(&args, 0, "redondear")?.round()))
}

fn ext_suelo(args: Vec<Valor>) -> Result<Valor, String> {
    if let Some(entero) = ext_matematicas_grande(&args) {
        return Ok(Valor::Grande(entero));
    }
    Ok(Valor::Numero(ext_matematicas_numero(&args, 0, "suelo")?.floor()))
}

fn ext_techo(args: Vec<Valor>) -> Result<Valor, String> {
    if let Some(entero) = ext_matematicas_grande(&args) {
        return Ok(Valor::Grande(entero));
    }
    Ok(Valor::Numero(ext_matematicas_numero(&args, 0, "techo")?.ceil()))
}

/// El menor (o el mayor) de dos números comparando exacto, para no perder cifras de un entero grande.
fn ext_matematicas_elegir(args: &[Valor], funcion: &str, quiero: std::cmp::Ordering) -> Result<Valor, String> {
    let a = ext_matematicas_numero(args, 0, funcion)?;
    let b = ext_matematicas_numero(args, 1, funcion)?;
    if a.is_nan() || b.is_nan() {
        return Ok(Valor::Numero(if funcion == "minimo" { a.min(b) } else { a.max(b) }));
    }
    match orden_numerico(&args[0], &args[1]) {
        Some(orden) if orden == quiero => Ok(args[0].clone()),
        _ => Ok(args[1].clone()),
    }
}

fn ext_minimo(args: Vec<Valor>) -> Result<Valor, String> {
    ext_matematicas_elegir(&args, "minimo", std::cmp::Ordering::Less)
}

fn ext_maximo(args: Vec<Valor>) -> Result<Valor, String> {
    ext_matematicas_elegir(&args, "maximo", std::cmp::Ordering::Greater)
}

thread_local! {
    static EXT_MATEMATICAS_SEMILLA: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn ext_aleatorio(_args: Vec<Valor>) -> Result<Valor, String> {
    // Generador xorshift64*: sin dependencias; la semilla sale del reloj la primera vez.
    let siguiente = EXT_MATEMATICAS_SEMILLA.with(|semilla| {
        let mut x = semilla.get();
        if x == 0 {
            x = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E37_79B9_7F4A_7C15)
                | 1;
        }
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        semilla.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    });
    Ok(Valor::Numero((siguiente >> 11) as f64 / (1u64 << 53) as f64))
}

fn ext_entero_al_azar(args: Vec<Valor>) -> Result<Valor, String> {
    let minimo = ext_matematicas_numero(&args, 0, "entero_al_azar")?.ceil();
    let maximo = ext_matematicas_numero(&args, 1, "entero_al_azar")?.floor();
    if minimo > maximo {
        return Err(format!("entero_al_azar(): el mínimo ({}) es mayor que el máximo ({})", minimo, maximo));
    }
    let azar = match ext_aleatorio(Vec::new())? {
        Valor::Numero(n) => n,
        _ => 0.0,
    };
    Ok(Valor::Numero((minimo + (azar * (maximo - minimo + 1.0)).floor()).min(maximo)))
}
