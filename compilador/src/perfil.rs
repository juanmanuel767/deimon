//! Perfiles de recursos: eligen cómo se compila el programa y los límites del servidor.
//!
//! Un mismo programa puede pensarse para un móvil modesto (perfil `pequeño`), para un
//! ordenador normal (`equilibrado`) o para un servidor con muchos núcleos (`servidor`).

/// Perfil de recursos elegido al compilar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perfil {
    Pequeno,
    Equilibrado,
    Servidor,
}

impl Perfil {
    /// Nombre que se muestra al usuario y se graba en el programa generado.
    pub fn nombre(self) -> &'static str {
        match self {
            Perfil::Pequeno => "pequeño",
            Perfil::Equilibrado => "equilibrado",
            Perfil::Servidor => "servidor",
        }
    }

    /// Lee el valor de `--perfil`; acepta `pequeño` y `pequeno`.
    pub fn desde_texto(texto: &str) -> Option<Self> {
        match texto.trim().to_lowercase().as_str() {
            "pequeño" | "pequeno" => Some(Perfil::Pequeno),
            "equilibrado" => Some(Perfil::Equilibrado),
            "servidor" => Some(Perfil::Servidor),
            _ => None,
        }
    }

    /// Opciones que se pasan a `rustc` para este perfil.
    pub fn argumentos_rustc(self) -> &'static [&'static str] {
        match self {
            Perfil::Pequeno => &[
                "-C",
                "opt-level=z",
                "-C",
                "codegen-units=1",
                "-C",
                "strip=symbols",
                "-C",
                "lto=fat",
            ],
            Perfil::Equilibrado => &["-O"],
            Perfil::Servidor => &[
                "-C",
                "opt-level=3",
                "-C",
                "codegen-units=1",
                "-C",
                "lto=fat",
            ],
        }
    }

    /// Hilos trabajadores cuando no hay `DEIMON_HILOS`.
    pub fn hilos_por_defecto(self, nucleos: usize) -> usize {
        match self {
            Perfil::Pequeno => 2,
            Perfil::Equilibrado => nucleos.max(1),
            Perfil::Servidor => nucleos.max(1) * 2,
        }
    }

    /// Conexiones en espera cuando no hay `DEIMON_MAX_CONEXIONES`.
    pub fn max_conexiones(self) -> usize {
        match self {
            Perfil::Pequeno => 16,
            Perfil::Equilibrado => 128,
            Perfil::Servidor => 1024,
        }
    }

    /// Tamaño máximo de la petición, en bytes, cuando no hay `DEIMON_MAX_PETICION`.
    pub fn max_peticion(self) -> usize {
        match self {
            Perfil::Pequeno => 16 * 1024,
            Perfil::Equilibrado => 256 * 1024,
            Perfil::Servidor => 1024 * 1024,
        }
    }
}

/// Núcleos disponibles en este ordenador.
pub fn nucleos() -> usize {
    std::thread::available_parallelism()
        .map(|cantidad| cantidad.get())
        .unwrap_or(1)
}

/// Memoria total en bytes, leída de `/proc/meminfo`; `None` si no se puede saber.
pub fn memoria_total_bytes() -> Option<u64> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let texto = std::fs::read_to_string("/proc/meminfo").ok()?;
    for linea in texto.lines() {
        if let Some(resto) = linea.strip_prefix("MemTotal:") {
            let kib: u64 = resto.split_whitespace().next()?.parse().ok()?;
            return Some(kib.saturating_mul(1024));
        }
    }
    None
}

/// Perfil recomendado: pequeño en equipos muy justos, servidor en equipos grandes.
pub fn recomendado(nucleos: usize, memoria: Option<u64>) -> Perfil {
    const UN_GB: u64 = 1024 * 1024 * 1024;
    let poca_memoria = memoria.is_some_and(|bytes| bytes < UN_GB);
    let mucha_memoria = memoria.is_some_and(|bytes| bytes >= 8 * UN_GB);
    if poca_memoria || nucleos <= 1 {
        Perfil::Pequeno
    } else if nucleos >= 8 && mucha_memoria {
        Perfil::Servidor
    } else {
        Perfil::Equilibrado
    }
}

/// Mensaje de error para un `--perfil` desconocido, con la lista y una sugerencia.
pub fn error_desconocido(valor: &str) -> (String, i32) {
    let mut mensaje = format!(
        "Error: el perfil '{}' no existe.\nPerfiles válidos: pequeño (o pequeno), equilibrado, servidor.",
        valor
    );
    match sugerir(valor) {
        Some(parecido) => mensaje.push_str(&format!(
            "\nSugerencia: ¿Querías decir '{}'? Usa --perfil {}.",
            parecido, parecido
        )),
        None => mensaje.push_str(
            "\nSugerencia: usa --perfil pequeño, --perfil equilibrado o --perfil servidor.",
        ),
    }
    (mensaje, 2)
}

fn sugerir(valor: &str) -> Option<&'static str> {
    let buscado = valor.to_lowercase();
    let limite = (buscado.chars().count() / 3).max(2);
    ["pequeño", "pequeno", "equilibrado", "servidor"]
        .iter()
        .map(|nombre| (distancia(&buscado, nombre), *nombre))
        .filter(|(distancia, _)| *distancia <= limite)
        .min_by_key(|(distancia, _)| *distancia)
        .map(|(_, nombre)| nombre)
}

fn distancia(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut anterior: Vec<usize> = (0..=b.len()).collect();
    let mut actual = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        actual[0] = i;
        for j in 1..=b.len() {
            let coste = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            actual[j] = (anterior[j] + 1)
                .min(actual[j - 1] + 1)
                .min(anterior[j - 1] + coste);
        }
        std::mem::swap(&mut anterior, &mut actual);
    }
    anterior[b.len()]
}
