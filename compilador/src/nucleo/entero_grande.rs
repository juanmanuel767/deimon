// Entero con signo de tamaño arbitrario, sin dependencias. Se guarda en base 2^32 con los
// dígitos en orden inverso (el primero es el menos significativo) y sin ceros delante. Lo
// usan los dos motores y también el destino pc-sin-sistema, así que solo usa Vec y String.

const BASE_ENTERO: u64 = 1 << 32;
const GRUPO_DECIMAL: u32 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnteroGrande {
    negativo: bool,
    digitos: Vec<u32>,
}

impl EnteroGrande {
    fn cero() -> Self {
        EnteroGrande { negativo: false, digitos: Vec::new() }
    }

    fn desde_u64(mut numero: u64) -> Self {
        let mut digitos = Vec::new();
        while numero > 0 {
            digitos.push(numero as u32);
            numero >>= 32;
        }
        EnteroGrande { negativo: false, digitos }
    }

    fn desde_texto(texto: &str) -> Option<Self> {
        let texto = texto.trim();
        let (negativo, cifras) = if let Some(resto) = texto.strip_prefix('-') {
            (true, resto)
        } else if let Some(resto) = texto.strip_prefix('+') {
            (false, resto)
        } else {
            (false, texto)
        };
        if cifras.is_empty() || !cifras.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut digitos: Vec<u32> = Vec::new();
        for byte in cifras.bytes() {
            let mut acarreo = u64::from(byte - b'0');
            for digito in digitos.iter_mut() {
                let total = u64::from(*digito) * 10 + acarreo;
                *digito = total as u32;
                acarreo = total >> 32;
            }
            if acarreo > 0 {
                digitos.push(acarreo as u32);
            }
        }
        let mut entero = EnteroGrande { negativo, digitos };
        if entero.es_cero() {
            entero.negativo = false;
        }
        Some(entero)
    }

    fn desde_f64(numero: f64) -> Option<Self> {
        if !numero.is_finite() || numero.fract() != 0.0 {
            return None;
        }
        Self::desde_texto(&format!("{:.0}", numero))
    }

    fn a_f64(&self) -> f64 {
        self.a_texto().parse::<f64>().unwrap_or(f64::INFINITY)
    }

    fn es_cero(&self) -> bool {
        self.digitos.is_empty()
    }

    fn cabe_en_numero(&self) -> bool {
        match self.digitos.len() {
            0 => true,
            1 => true,
            2 => u64::from(self.digitos[0]) | (u64::from(self.digitos[1]) << 32) <= 1 << 53,
            _ => false,
        }
    }

    fn negado(&self) -> Self {
        EnteroGrande { negativo: !self.negativo && !self.es_cero(), digitos: self.digitos.clone() }
    }

    fn a_texto(&self) -> String {
        if self.es_cero() {
            return "0".to_string();
        }
        let mut copia = self.digitos.clone();
        let mut grupos: Vec<u32> = Vec::new();
        while !copia.is_empty() {
            let mut resto = 0u64;
            for digito in copia.iter_mut().rev() {
                let actual = (resto << 32) | u64::from(*digito);
                *digito = (actual / u64::from(GRUPO_DECIMAL)) as u32;
                resto = actual % u64::from(GRUPO_DECIMAL);
            }
            while copia.last() == Some(&0) {
                copia.pop();
            }
            grupos.push(resto as u32);
        }
        let mut salida = String::new();
        if self.negativo {
            salida.push('-');
        }
        if let Some(primero) = grupos.last() {
            salida.push_str(&primero.to_string());
        }
        for grupo in grupos.iter().rev().skip(1) {
            salida.push_str(&format!("{:09}", grupo));
        }
        salida
    }

    fn comparar(&self, otra: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        if self.es_cero() && otra.es_cero() {
            return Ordering::Equal;
        }
        if self.negativo != otra.negativo {
            return if self.negativo { Ordering::Less } else { Ordering::Greater };
        }
        let orden = comparar_abs(&self.digitos, &otra.digitos);
        if self.negativo { orden.reverse() } else { orden }
    }

    fn sumar(&self, otra: &Self) -> Self {
        use std::cmp::Ordering;
        if self.negativo == otra.negativo {
            let digitos = sumar_abs(&self.digitos, &otra.digitos);
            return EnteroGrande { negativo: self.negativo && !digitos.is_empty(), digitos };
        }
        match comparar_abs(&self.digitos, &otra.digitos) {
            Ordering::Equal => Self::cero(),
            Ordering::Greater => EnteroGrande {
                negativo: self.negativo,
                digitos: restar_abs(&self.digitos, &otra.digitos),
            },
            Ordering::Less => EnteroGrande {
                negativo: otra.negativo,
                digitos: restar_abs(&otra.digitos, &self.digitos),
            },
        }
    }

    fn restar(&self, otra: &Self) -> Self {
        self.sumar(&otra.negado())
    }

    fn multiplicar(&self, otra: &Self) -> Self {
        let digitos = multiplicar_abs(&self.digitos, &otra.digitos);
        EnteroGrande {
            negativo: self.negativo != otra.negativo && !digitos.is_empty(),
            digitos,
        }
    }

    /// Cociente y resto. El resto tiene el signo del dividendo, como `%` en Deimon.
    fn dividir_con_resto(&self, otra: &Self) -> (Self, Self) {
        let (cociente, resto) = dividir_abs(&self.digitos, &otra.digitos);
        let cociente = EnteroGrande {
            negativo: self.negativo != otra.negativo && !cociente.is_empty(),
            digitos: cociente,
        };
        let resto = EnteroGrande {
            negativo: self.negativo && !resto.is_empty(),
            digitos: resto,
        };
        (cociente, resto)
    }
}

fn comparar_abs(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match a.len().cmp(&b.len()) {
        Ordering::Equal => {}
        otro => return otro,
    }
    for posicion in (0..a.len()).rev() {
        match a[posicion].cmp(&b[posicion]) {
            Ordering::Equal => {}
            otro => return otro,
        }
    }
    Ordering::Equal
}

fn sumar_abs(a: &[u32], b: &[u32]) -> Vec<u32> {
    let largo = a.len().max(b.len());
    let mut resultado = Vec::with_capacity(largo + 1);
    let mut acarreo = 0u64;
    for posicion in 0..largo {
        let total = a.get(posicion).copied().unwrap_or(0) as u64
            + b.get(posicion).copied().unwrap_or(0) as u64
            + acarreo;
        resultado.push(total as u32);
        acarreo = total >> 32;
    }
    if acarreo > 0 {
        resultado.push(acarreo as u32);
    }
    resultado
}

fn restar_abs(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut resultado = Vec::with_capacity(a.len());
    let mut prestamo = 0i64;
    for posicion in 0..a.len() {
        let resta = i64::from(a[posicion])
            - prestamo
            - i64::from(b.get(posicion).copied().unwrap_or(0));
        if resta < 0 {
            resultado.push((resta + BASE_ENTERO as i64) as u32);
            prestamo = 1;
        } else {
            resultado.push(resta as u32);
            prestamo = 0;
        }
    }
    while resultado.last() == Some(&0) {
        resultado.pop();
    }
    resultado
}

fn multiplicar_por_u32(digitos: &[u32], factor: u32) -> Vec<u32> {
    if factor == 0 || digitos.is_empty() {
        return Vec::new();
    }
    let mut resultado = Vec::with_capacity(digitos.len() + 1);
    let mut acarreo = 0u64;
    for digito in digitos {
        let total = u64::from(*digito) * u64::from(factor) + acarreo;
        resultado.push(total as u32);
        acarreo = total >> 32;
    }
    if acarreo > 0 {
        resultado.push(acarreo as u32);
    }
    resultado
}

fn multiplicar_abs(a: &[u32], b: &[u32]) -> Vec<u32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut resultado = vec![0u32; a.len() + b.len() + 1];
    for (i, izquierda) in a.iter().enumerate() {
        let mut acarreo = 0u64;
        for (j, derecha) in b.iter().enumerate() {
            let posicion = i + j;
            let total = u64::from(resultado[posicion])
                + u64::from(*izquierda) * u64::from(*derecha)
                + acarreo;
            resultado[posicion] = total as u32;
            acarreo = total >> 32;
        }
        let mut posicion = i + b.len();
        while acarreo > 0 {
            let total = u64::from(resultado[posicion]) + acarreo;
            resultado[posicion] = total as u32;
            acarreo = total >> 32;
            posicion += 1;
        }
    }
    while resultado.last() == Some(&0) {
        resultado.pop();
    }
    resultado
}

fn dividir_abs(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    use std::cmp::Ordering;
    if b.is_empty() {
        return (Vec::new(), Vec::new());
    }
    if comparar_abs(a, b) == Ordering::Less {
        return (Vec::new(), a.to_vec());
    }
    let mut cociente = vec![0u32; a.len()];
    let mut resto: Vec<u32> = Vec::new();
    for posicion in (0..a.len()).rev() {
        resto.insert(0, a[posicion]);
        while resto.last() == Some(&0) {
            resto.pop();
        }
        let mut bajo = 0u64;
        let mut alto = u64::from(u32::MAX);
        while bajo < alto {
            let medio = (bajo + alto + 1) / 2;
            let producto = multiplicar_por_u32(b, medio as u32);
            if comparar_abs(&producto, &resto) != Ordering::Greater {
                bajo = medio;
            } else {
                alto = medio - 1;
            }
        }
        if bajo > 0 {
            let producto = multiplicar_por_u32(b, bajo as u32);
            resto = restar_abs(&resto, &producto);
        }
        cociente[posicion] = bajo as u32;
    }
    while cociente.last() == Some(&0) {
        cociente.pop();
    }
    while resto.last() == Some(&0) {
        resto.pop();
    }
    (cociente, resto)
}
