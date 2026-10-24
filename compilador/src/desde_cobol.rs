//! Traductor básico de COBOL a Deimon (`deimon desde-cobol archivo.cob`).
//!
//! Entiende un subconjunto pequeño de COBOL (datos simples en WORKING-STORAGE y las
//! instrucciones más comunes de la PROCEDURE DIVISION). Lo que no entiende NUNCA se traduce
//! a medias: se para con un error en español que dice la línea y la instrucción.
//!
//! Cómo se guardan los datos en el Deimon generado:
//! - Un número (PIC 9, S9, V) es un entero "escalado": 12.50 con PIC 9(3)V99 vale 1250.
//!   Así las sumas, restas y multiplicaciones son exactas.
//! - Un texto (PIC X(n)) siempre ocupa exactamente n caracteres (con espacios a la derecha).
//! - Los ayudantes `cb_...` del principio dan formato y truncan igual que GnuCOBOL.
//! - Cada párrafo se copia en el sitio donde se le llama (PERFORM), porque las funciones de
//!   Deimon no ven las variables de fuera. Todo el programa vive dentro de `principal()`.

use std::collections::{HashMap, HashSet};

use crate::integradas;

/// Resultado de traducir un archivo COBOL.
pub struct Traduccion {
    pub codigo: String,
    /// (línea del COBOL, explicación) de diferencias sutiles que conviene revisar.
    pub avisos: Vec<(usize, String)>,
}

type R<T> = Result<T, String>;

/// Decimales que se calculan en una división antes de truncar al guardar.
const DECIMALES_DIVISION: u32 = 8;

const AYUDANTES: &str = r#"# Ayudantes para que los números y textos se comporten como en COBOL.
funcion cb_trunc(x) {
    devolver x - x % 1
}

funcion cb_reducir(v, p) {
    devolver cb_trunc(v / p)
}

funcion cb_redondear(v, p) {
    si v < 0 {
        devolver 0 - cb_trunc((0 - v + p / 2) / p)
    }
    devolver cb_trunc((v + p / 2) / p)
}

funcion cb_ajustar_sin_signo(v, p) {
    sea a = v
    si v < 0 {
        a = 0 - v
    }
    devolver a % p + 0
}

funcion cb_ajustar_con_signo(v, p) {
    devolver v % p + 0
}

funcion cb_texto(t, n) {
    sea r = cortar(t, 0, n)
    mientras medir(r) < n {
        r = r + " "
    }
    devolver r
}

funcion cb_formato(v, dig, esc, signo) {
    sea a = v
    sea neg = v < 0
    si neg {
        a = 0 - v
    }
    sea t = a_texto(a)
    mientras medir(t) < dig {
        t = "0" + t
    }
    si esc > 0 {
        sea n = medir(t) - esc
        t = cortar(t, 0, n) + "." + cortar(t, n, esc)
    }
    si signo {
        si neg {
            t = "-" + t
        } sino {
            t = "+" + t
        }
    }
    devolver t
}
"#;

// ---------------------------------------------------------------------------
// Lectura del texto: líneas útiles y piezas.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum T {
    Pal(String),
    Num(String),
    Txt(String),
    Pic(String),
    Op(String),
    Punto,
}

#[derive(Debug, Clone)]
struct Tok {
    t: T,
    linea: usize,
}

/// Dice si el programa está escrito en formato libre (sin columnas fijas).
pub fn es_formato_libre(fuente: &str) -> bool {
    for l in fuente.lines() {
        if l.trim().is_empty() {
            continue;
        }
        if l.trim_start().starts_with(">>") {
            if l.to_uppercase().contains("FREE") {
                return true;
            }
            continue;
        }
        let ch: Vec<char> = l.chars().collect();
        let cabecera = ch.iter().take(6).all(|c| c.is_ascii_digit() || *c == ' ');
        let c7 = ch.get(6).copied().unwrap_or(' ');
        if !cabecera || !matches!(c7, ' ' | '*' | '/' | '-' | 'D' | 'd') {
            return true;
        }
    }
    false
}

/// Quita comentarios `*>` que no estén dentro de un texto.
fn quitar_comentario(l: &str) -> String {
    let ch: Vec<char> = l.chars().collect();
    let mut comilla: Option<char> = None;
    let mut i = 0;
    while i < ch.len() {
        match comilla {
            Some(c) if ch[i] == c => comilla = None,
            Some(_) => {}
            None if ch[i] == '"' || ch[i] == '\'' => comilla = Some(ch[i]),
            None if ch[i] == '*' && ch.get(i + 1) == Some(&'>') => return ch[..i].iter().collect(),
            None => {}
        }
        i += 1;
    }
    l.to_string()
}

/// (número de línea, contenido) de las líneas con código.
fn lineas_utiles(fuente: &str, libre: bool) -> R<Vec<(usize, String)>> {
    let mut salida = Vec::new();
    for (i, l) in fuente.lines().enumerate() {
        let n = i + 1;
        let l = l.trim_end_matches('\r');
        let contenido: String = if libre {
            let t = l.trim_start();
            if t.starts_with('*') || t.starts_with(">>") {
                continue;
            }
            l.to_string()
        } else {
            let ch: Vec<char> = l.chars().collect();
            if ch.len() <= 6 {
                continue;
            }
            match ch[6] {
                '*' | '/' | 'D' | 'd' => continue,
                '-' => {
                    return Err(format!(
                        "línea {}: las líneas de continuación (un texto largo partido en dos líneas) no están soportadas todavía.\n    {}",
                        n,
                        l.trim()
                    ))
                }
                _ => {}
            }
            ch[7.min(ch.len())..72.min(ch.len())].iter().collect()
        };
        let contenido = quitar_comentario(&contenido);
        if !contenido.trim().is_empty() {
            salida.push((n, contenido));
        }
    }
    Ok(salida)
}

fn es_letra_nombre(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn leer_piezas(lineas: &[(usize, String)]) -> R<Vec<Tok>> {
    let mut toks: Vec<Tok> = Vec::new();
    let mut esperando_pic = false;
    for (n, l) in lineas {
        let n = *n;
        let ch: Vec<char> = l.chars().collect();
        let mut i = 0;
        let poner = |t: T, toks: &mut Vec<Tok>| toks.push(Tok { t, linea: n });
        while i < ch.len() {
            let c = ch[i];
            if c.is_whitespace() {
                i += 1;
                continue;
            }
            if esperando_pic {
                // Después de PIC se lee la máscara entera tal cual.
                let ini = i;
                while i < ch.len() && !ch[i].is_whitespace() {
                    i += 1;
                }
                let palabra: String = ch[ini..i].iter().collect();
                if palabra.eq_ignore_ascii_case("IS") {
                    poner(T::Pal("IS".into()), &mut toks);
                    continue;
                }
                esperando_pic = false;
                if let Some(sin) = palabra.strip_suffix('.') {
                    poner(T::Pic(sin.to_uppercase()), &mut toks);
                    poner(T::Punto, &mut toks);
                } else {
                    poner(T::Pic(palabra.to_uppercase()), &mut toks);
                }
                continue;
            }
            if c == '"' || c == '\'' {
                let mut j = i + 1;
                let mut texto = String::new();
                loop {
                    if j >= ch.len() {
                        return Err(format!("línea {}: un texto entre comillas no se cierra en la misma línea.\n    {}", n, l.trim()));
                    }
                    if ch[j] == c {
                        if ch.get(j + 1) == Some(&c) {
                            texto.push(c);
                            j += 2;
                            continue;
                        }
                        break;
                    }
                    texto.push(ch[j]);
                    j += 1;
                }
                poner(T::Txt(texto), &mut toks);
                i = j + 1;
                continue;
            }
            if c.is_ascii_digit() {
                let ini = i;
                while i < ch.len() && ch[i].is_ascii_digit() {
                    i += 1;
                }
                if i < ch.len() && ch[i] == '.' && ch.get(i + 1).map(|d| d.is_ascii_digit()).unwrap_or(false) {
                    i += 1;
                    while i < ch.len() && ch[i].is_ascii_digit() {
                        i += 1;
                    }
                } else if i < ch.len() && (ch[i].is_ascii_alphabetic() || ch[i] == '-' || ch[i] == '_') {
                    // Un nombre que empieza por número (1A): se lee entero.
                    while i < ch.len() && es_letra_nombre(ch[i]) {
                        i += 1;
                    }
                    let p: String = ch[ini..i].iter().collect();
                    poner(T::Pal(p.trim_end_matches('-').to_uppercase()), &mut toks);
                    continue;
                }
                poner(T::Num(ch[ini..i].iter().collect()), &mut toks);
                continue;
            }
            if c.is_ascii_alphabetic() {
                let ini = i;
                while i < ch.len() && es_letra_nombre(ch[i]) {
                    i += 1;
                }
                let p: String = ch[ini..i].iter().collect();
                let p = p.trim_end_matches('-').to_uppercase();
                if p == "PIC" || p == "PICTURE" {
                    esperando_pic = true;
                }
                poner(T::Pal(p), &mut toks);
                continue;
            }
            match c {
                '.' => {
                    poner(T::Punto, &mut toks);
                    i += 1;
                }
                ',' | ';' => i += 1,
                '+' | '-' | '/' | '(' | ')' | '=' => {
                    poner(T::Op(c.to_string()), &mut toks);
                    i += 1;
                }
                '*' => {
                    if ch.get(i + 1) == Some(&'*') {
                        poner(T::Op("**".into()), &mut toks);
                        i += 2;
                    } else {
                        poner(T::Op("*".into()), &mut toks);
                        i += 1;
                    }
                }
                '<' | '>' => {
                    let doble = match ch.get(i + 1) {
                        Some('=') => Some("="),
                        Some('>') if c == '<' => Some(">"),
                        _ => None,
                    };
                    match doble {
                        Some(s) => {
                            poner(T::Op(format!("{}{}", c, s)), &mut toks);
                            i += 2;
                        }
                        None => {
                            poner(T::Op(c.to_string()), &mut toks);
                            i += 1;
                        }
                    }
                }
                otro => {
                    return Err(format!("línea {}: el carácter '{}' no está soportado todavía.\n    {}", n, otro, l.trim()));
                }
            }
        }
    }
    Ok(toks)
}

// ---------------------------------------------------------------------------
// Datos.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Tipo {
    /// dig = cifras en total (enteras y decimales), esc = cifras decimales.
    Num { dig: u32, esc: u32, signo: bool },
    Txt { lon: usize },
}

#[derive(Debug, Clone)]
struct Var {
    id: String,
    tipo: Tipo,
}

enum Valor {
    Txt(String),
    Num(String, bool),
    Ceros,
    Espacios,
}

fn pot_i(n: u32) -> i128 {
    10i128.pow(n)
}

fn pot(n: u32) -> String {
    format!("1{}", "0".repeat(n as usize))
}

fn texto(s: &str) -> String {
    let mut r = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => r.push_str("\\\""),
            '\\' => r.push_str("\\\\"),
            '\t' => r.push_str("\\t"),
            c => r.push(c),
        }
    }
    r.push('"');
    r
}

fn ajustar_txt(s: &str, lon: usize) -> String {
    let mut r: String = s.chars().take(lon).collect();
    while r.chars().count() < lon {
        r.push(' ');
    }
    r
}

fn leer_pic(pic: &str) -> Result<Tipo, String> {
    let ch: Vec<char> = pic.chars().collect();
    let mut i = 0;
    let mut signo = false;
    let mut enteras = 0u32;
    let mut decimales = 0u32;
    let mut txt = 0usize;
    let mut tras_v = false;
    let mut visto_v = false;
    if ch.first() == Some(&'S') {
        signo = true;
        i = 1;
    }
    let mut anterior: Option<char> = None;
    while i < ch.len() {
        let c = ch[i];
        match c {
            '9' | 'X' | 'A' => {
                if matches!(anterior, Some(a) if (a == '9') != (c == '9') && a != 'V') {
                    return Err("mezcla números y letras en un mismo PIC".into());
                }
                if c == '9' {
                    if tras_v {
                        decimales += 1
                    } else {
                        enteras += 1
                    }
                } else {
                    txt += 1
                }
                anterior = Some(c);
            }
            'V' => {
                if visto_v {
                    return Err("tiene dos V".into());
                }
                visto_v = true;
                tras_v = true;
            }
            '(' => {
                let cierre = ch[i..].iter().position(|&x| x == ')').ok_or("le falta el paréntesis")?;
                let n: usize = ch[i + 1..i + cierre].iter().collect::<String>().parse().map_err(|_| "tiene una repetición que no es un número")?;
                let previo = anterior.ok_or("empieza por un paréntesis")?;
                if n == 0 {
                    return Err("repite cero veces".into());
                }
                match previo {
                    '9' => {
                        if tras_v {
                            decimales += n as u32 - 1
                        } else {
                            enteras += n as u32 - 1
                        }
                    }
                    _ => txt += n - 1,
                }
                i += cierre;
            }
            _ => return Err(format!("usa el símbolo '{}' (solo se aceptan 9, S, V y X)", c)),
        }
        i += 1;
    }
    if txt > 0 {
        if signo || enteras + decimales > 0 || visto_v {
            return Err("mezcla números y letras en un mismo PIC".into());
        }
        return Ok(Tipo::Txt { lon: txt });
    }
    let dig = enteras + decimales;
    if dig == 0 {
        return Err("no tiene ninguna cifra".into());
    }
    if dig > 15 {
        return Err("tiene más de 15 cifras (Deimon calcula con 15 cifras exactas)".into());
    }
    Ok(Tipo::Num { dig, esc: decimales, signo })
}

/// Pasa un valor entero escalado (k con `esc_o` decimales) a las cifras de una variable.
fn convertir_k(k: i128, esc_o: u32, dig: u32, esc: u32, signo: bool, redondear: bool) -> i128 {
    let k = if esc >= esc_o {
        k * pot_i(esc - esc_o)
    } else {
        let d = pot_i(esc_o - esc);
        if redondear {
            let m = (k.abs() + d / 2) / d;
            if k < 0 { -m } else { m }
        } else {
            k / d
        }
    };
    let r = k % pot_i(dig);
    if signo { r } else { r.abs() }
}

fn literal_a_k(s: &str) -> Option<(i128, u32)> {
    let (e, d) = match s.split_once('.') {
        Some((e, d)) => (e, d),
        None => (s, ""),
    };
    let k: i128 = format!("{}{}", e, d).parse().ok()?;
    Some((k, d.len() as u32))
}

// ---------------------------------------------------------------------------
// Expresiones numéricas (exactas, con escala).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct E {
    cod: String,
    esc: u32,
    k: Option<i128>,
}

impl E {
    fn constante(k: i128, esc: u32) -> E {
        E { cod: if k < 0 { format!("({})", k) } else { k.to_string() }, esc, k: Some(k) }
    }
    fn alinear(&self, esc: u32) -> String {
        if esc == self.esc {
            self.cod.clone()
        } else {
            format!("({} * {})", self.cod, pot(esc - self.esc))
        }
    }
}

fn e_suma(a: &E, b: &E, resta: bool) -> E {
    let s = a.esc.max(b.esc);
    if let (Some(x), Some(y)) = (a.k, b.k) {
        let (x, y) = (x * pot_i(s - a.esc), y * pot_i(s - b.esc));
        return E::constante(if resta { x - y } else { x + y }, s);
    }
    E { cod: format!("({} {} {})", a.alinear(s), if resta { "-" } else { "+" }, b.alinear(s)), esc: s, k: None }
}

fn e_mult(a: &E, b: &E) -> E {
    if let (Some(x), Some(y)) = (a.k, b.k) {
        return E::constante(x * y, a.esc + b.esc);
    }
    E { cod: format!("({} * {})", a.cod, b.cod), esc: a.esc + b.esc, k: None }
}

fn e_div(a: &E, b: &E) -> E {
    // Resultado con DECIMALES_DIVISION decimales, truncado.
    let r = DECIMALES_DIVISION;
    let k = r as i64 + b.esc as i64 - a.esc as i64;
    let cod = if k >= 0 {
        format!("cb_trunc({} * {} / {})", a.cod, pot(k as u32), b.cod)
    } else {
        format!("cb_trunc({} / ({} * {}))", a.cod, b.cod, pot((-k) as u32))
    };
    E { cod, esc: r, k: None }
}

fn e_neg(a: &E) -> E {
    match a.k {
        Some(k) => E::constante(-k, a.esc),
        None => E { cod: format!("(0 - {})", a.cod), esc: a.esc, k: None },
    }
}

fn e_total(items: &[E]) -> E {
    let mut total = items[0].clone();
    for e in &items[1..] {
        total = e_suma(&total, e, false);
    }
    total
}

enum Op {
    N { e: E, lit: Option<String>, var: Option<Var> },
    T { cod: String, lon: usize, lit: Option<String> },
    Fig(char),
}

struct Item {
    e: E,
    var: Option<Var>,
    red: bool,
}

// ---------------------------------------------------------------------------
// El traductor.
// ---------------------------------------------------------------------------

struct G {
    toks: Vec<Tok>,
    fuente: Vec<String>,
    pos: usize,
    lim: usize,
    vars: HashMap<String, Var>,
    grupos: HashSet<String>,
    parrafos: Vec<(String, usize, usize)>,
    pila: Vec<usize>,
    contador: usize,
    usa_division: bool,
    avisos: Vec<(usize, String)>,
}

const PALABRAS_DEIMON: &[&str] = &[
    "sea", "usar", "funcion", "ruta", "prueba", "modulo", "extension", "devolver", "romper", "continuar", "si", "sino", "mientras", "para", "en", "y", "o",
    "no", "verdadero", "falso", "nada", "principal",
];

fn nombre_deimon(cobol: &str, usados: &HashSet<String>) -> String {
    let mut n = cobol.to_lowercase().replace('-', "_");
    if n.starts_with(|c: char| c.is_ascii_digit()) {
        n = format!("v_{}", n);
    }
    while PALABRAS_DEIMON.contains(&n.as_str()) || integradas::buscar(&n).is_some() || n.starts_with("cb_") || n.starts_with('_') || usados.contains(&n) {
        n = format!("{}_c", n.trim_start_matches('_'));
    }
    n
}

impl G {
    fn linea_actual(&self) -> usize {
        if self.toks.is_empty() {
            return 0;
        }
        self.toks[self.pos.min(self.toks.len() - 1)].linea
    }

    fn err(&self, msg: &str) -> String {
        let n = self.linea_actual();
        let origen = self.fuente.get(n.wrapping_sub(1)).map(|s| s.trim().to_string()).unwrap_or_default();
        format!("línea {}: {}\n    {}", n, msg, origen)
    }

    fn mirar(&self) -> Option<&T> {
        self.mirar_n(0)
    }
    fn mirar_n(&self, n: usize) -> Option<&T> {
        if self.pos + n < self.lim {
            Some(&self.toks[self.pos + n].t)
        } else {
            None
        }
    }
    fn es_pal(&self, p: &str) -> bool {
        matches!(self.mirar(), Some(T::Pal(x)) if x == p)
    }
    fn es_pal_n(&self, n: usize, p: &str) -> bool {
        matches!(self.mirar_n(n), Some(T::Pal(x)) if x == p)
    }
    fn acepta(&mut self, p: &str) -> bool {
        if self.es_pal(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn espera(&mut self, p: &str) -> R<()> {
        if self.acepta(p) {
            Ok(())
        } else {
            Err(self.err(&format!("se esperaba '{}'.", p)))
        }
    }
    fn es_op(&self, o: &str) -> bool {
        matches!(self.mirar(), Some(T::Op(x)) if x == o)
    }
    fn acepta_op(&mut self, o: &str) -> bool {
        if self.es_op(o) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn nuevo_temporal(&mut self, base: &str) -> String {
        self.contador += 1;
        format!("cb_{}{}", base, self.contador)
    }

    fn variable(&self, nombre: &str) -> R<Var> {
        match self.vars.get(nombre) {
            Some(v) => Ok(v.clone()),
            None if self.grupos.contains(nombre) => Err(self.err(&format!(
                "'{}' es un grupo de datos y usarlo entero no está soportado todavía (usa sus campos uno a uno).",
                nombre
            ))),
            None => Err(self.err(&format!("'{}' no está declarado en WORKING-STORAGE.", nombre))),
        }
    }

    // ----- Datos -----

    fn datos(&mut self) -> R<Vec<String>> {
        let mut decl = Vec::new();
        let mut usados: HashSet<String> = HashSet::new();
        while self.pos < self.lim {
            if let Some(T::Pal(p)) = self.mirar() {
                let p = p.clone();
                match p.as_str() {
                    "WORKING-STORAGE" | "LOCAL-STORAGE" => {
                        self.pos += 1;
                        self.espera("SECTION")?;
                        if self.mirar() == Some(&T::Punto) {
                            self.pos += 1;
                        }
                        continue;
                    }
                    "FILE" | "LINKAGE" | "SCREEN" | "REPORT" | "COMMUNICATION" => {
                        return Err(self.err(&format!("la sección '{}' no está soportada todavía (solo WORKING-STORAGE).", p)))
                    }
                    _ => {}
                }
            }
            let nivel: u32 = match self.mirar() {
                Some(T::Num(n)) => n.parse().unwrap_or(0),
                _ => return Err(self.err("se esperaba el número de nivel de un dato (01, 05, 77...).")),
            };
            self.pos += 1;
            if nivel == 88 {
                return Err(self.err("los nombres de condición (nivel 88) no están soportados todavía."));
            }
            if nivel == 66 || nivel == 0 || (nivel > 49 && nivel != 77) {
                return Err(self.err(&format!("el nivel {} no está soportado.", nivel)));
            }
            let nombre = match self.mirar() {
                Some(T::Pal(p)) if p != "PIC" && p != "PICTURE" && p != "VALUE" => {
                    let p = p.clone();
                    self.pos += 1;
                    if p == "FILLER" { None } else { Some(p) }
                }
                _ => None,
            };
            let mut tipo: Option<Tipo> = None;
            let mut valor: Option<Valor> = None;
            while self.mirar() != Some(&T::Punto) {
                if self.pos >= self.lim {
                    return Err(self.err("falta el punto al final de la declaración del dato."));
                }
                match self.mirar().cloned() {
                    Some(T::Pal(p)) if p == "PIC" || p == "PICTURE" => {
                        self.pos += 1;
                        self.acepta("IS");
                        match self.mirar().cloned() {
                            Some(T::Pic(s)) => {
                                self.pos += 1;
                                tipo = Some(leer_pic(&s).map_err(|m| self.err(&format!("el PIC {} {}; no está soportado todavía.", s, m)))?);
                            }
                            _ => return Err(self.err("después de PIC falta la máscara.")),
                        }
                    }
                    Some(T::Pal(p)) if p == "VALUE" || p == "VALUES" => {
                        self.pos += 1;
                        self.acepta("IS");
                        self.acepta("ARE");
                        valor = Some(match self.mirar().cloned() {
                            Some(T::Txt(s)) => {
                                self.pos += 1;
                                Valor::Txt(s)
                            }
                            Some(T::Num(s)) => {
                                self.pos += 1;
                                Valor::Num(s, false)
                            }
                            Some(T::Op(o)) if (o == "-" || o == "+") && matches!(self.mirar_n(1), Some(T::Num(_))) => {
                                self.pos += 1;
                                let s = match self.mirar() {
                                    Some(T::Num(s)) => s.clone(),
                                    _ => unreachable!(),
                                };
                                self.pos += 1;
                                Valor::Num(s, o == "-")
                            }
                            Some(T::Pal(p)) if p == "SPACE" || p == "SPACES" => {
                                self.pos += 1;
                                Valor::Espacios
                            }
                            Some(T::Pal(p)) if p == "ZERO" || p == "ZEROS" || p == "ZEROES" => {
                                self.pos += 1;
                                Valor::Ceros
                            }
                            _ => return Err(self.err("este VALUE no está soportado todavía (usa un número, un texto, SPACES o ZEROS).")),
                        });
                    }
                    Some(T::Pal(p)) if p == "USAGE" => {
                        self.pos += 1;
                        self.acepta("IS");
                        if !self.acepta("DISPLAY") {
                            return Err(self.err("solo se acepta USAGE DISPLAY (nada de COMP ni BINARY)."));
                        }
                    }
                    Some(T::Pal(p)) if p == "DISPLAY" => self.pos += 1,
                    Some(T::Pal(p)) => return Err(self.err(&format!("la cláusula {} de un dato no está soportada todavía.", p))),
                    _ => return Err(self.err("no entiendo esta declaración de datos.")),
                }
            }
            self.pos += 1; // el punto
            let nombre_c = match (&nombre, &tipo) {
                (Some(n), _) => n.clone(),
                (None, _) => {
                    // FILLER: no se usa nunca, se ignora.
                    continue;
                }
            };
            match tipo {
                None => {
                    if valor.is_some() {
                        return Err(self.err("un grupo de datos no puede llevar VALUE todavía."));
                    }
                    self.grupos.insert(nombre_c);
                }
                Some(tipo) => {
                    if self.vars.contains_key(&nombre_c) {
                        return Err(self.err(&format!("el nombre '{}' está repetido (los nombres calificados con OF no están soportados).", nombre_c)));
                    }
                    let id = nombre_deimon(&nombre_c, &usados);
                    usados.insert(id.clone());
                    let inicial = self.valor_inicial(&tipo, valor)?;
                    decl.push(format!("sea {} = {}", id, inicial));
                    self.vars.insert(nombre_c, Var { id, tipo });
                }
            }
        }
        Ok(decl)
    }

    fn valor_inicial(&self, tipo: &Tipo, valor: Option<Valor>) -> R<String> {
        match (tipo, valor) {
            (Tipo::Num { .. }, None) | (Tipo::Num { .. }, Some(Valor::Ceros)) => Ok("0".into()),
            (Tipo::Num { dig, esc, signo }, Some(Valor::Num(s, neg))) => {
                let (k, e) = literal_a_k(&s).ok_or_else(|| self.err("el número del VALUE es demasiado grande."))?;
                let k = if neg { -k } else { k };
                Ok(convertir_k(k, e, *dig, *esc, *signo, false).to_string())
            }
            (Tipo::Num { .. }, Some(_)) => Err(self.err("un dato numérico solo puede llevar un número en el VALUE.")),
            (Tipo::Txt { lon }, None) | (Tipo::Txt { lon }, Some(Valor::Espacios)) => Ok(texto(&" ".repeat(*lon))),
            (Tipo::Txt { lon }, Some(Valor::Txt(s))) => Ok(texto(&ajustar_txt(&s, *lon))),
            (Tipo::Txt { lon }, Some(Valor::Ceros)) => Ok(texto(&"0".repeat(*lon))),
            (Tipo::Txt { .. }, Some(Valor::Num(..))) => Err(self.err("un dato de texto (PIC X) debe llevar un texto entre comillas en el VALUE.")),
        }
    }

    // ----- Expresiones -----

    fn expr(&mut self) -> R<E> {
        let mut a = self.termino()?;
        loop {
            if self.acepta_op("+") {
                let b = self.termino()?;
                a = e_suma(&a, &b, false);
            } else if self.acepta_op("-") {
                let b = self.termino()?;
                a = e_suma(&a, &b, true);
            } else {
                return Ok(a);
            }
        }
    }

    fn termino(&mut self) -> R<E> {
        let mut a = self.factor()?;
        loop {
            if self.acepta_op("*") {
                let b = self.factor()?;
                a = e_mult(&a, &b);
            } else if self.acepta_op("/") {
                let b = self.factor()?;
                self.usa_division = true;
                a = e_div(&a, &b);
            } else {
                return Ok(a);
            }
        }
    }

    fn factor(&mut self) -> R<E> {
        if self.acepta_op("-") {
            let f = self.factor()?;
            return Ok(e_neg(&f));
        }
        if self.acepta_op("+") {
            return self.factor();
        }
        let p = self.primaria()?;
        if self.es_op("**") {
            return Err(self.err("la potencia (**) no está soportada todavía."));
        }
        Ok(p)
    }

    fn primaria(&mut self) -> R<E> {
        match self.mirar().cloned() {
            Some(T::Num(s)) => {
                self.pos += 1;
                let (k, esc) = literal_a_k(&s).ok_or_else(|| self.err("este número es demasiado grande."))?;
                Ok(E::constante(k, esc))
            }
            Some(T::Pal(p)) if p == "ZERO" || p == "ZEROS" || p == "ZEROES" => {
                self.pos += 1;
                Ok(E::constante(0, 0))
            }
            Some(T::Pal(p)) if p == "FUNCTION" => Err(self.err("las funciones (FUNCTION ...) no están soportadas todavía.")),
            Some(T::Pal(p)) => {
                let v = self.variable(&p)?;
                match v.tipo {
                    Tipo::Num { esc, .. } => {
                        self.pos += 1;
                        Ok(E { cod: v.id, esc, k: None })
                    }
                    Tipo::Txt { .. } => Err(self.err(&format!("'{}' es un texto (PIC X) y aquí hace falta un número.", p))),
                }
            }
            Some(T::Op(o)) if o == "(" => {
                self.pos += 1;
                let e = self.expr()?;
                if !self.acepta_op(")") {
                    return Err(self.err("falta cerrar el paréntesis."));
                }
                Ok(e)
            }
            _ => Err(self.err("se esperaba un número o una variable numérica.")),
        }
    }

    fn operando(&mut self) -> R<Op> {
        match self.mirar().cloned() {
            Some(T::Txt(s)) => {
                self.pos += 1;
                Ok(Op::T { cod: texto(&s), lon: s.chars().count(), lit: Some(s) })
            }
            Some(T::Pal(p)) if p == "SPACE" || p == "SPACES" => {
                self.pos += 1;
                Ok(Op::Fig(' '))
            }
            Some(T::Pal(p)) if p == "ZERO" || p == "ZEROS" || p == "ZEROES" => {
                self.pos += 1;
                Ok(Op::Fig('0'))
            }
            Some(T::Pal(p)) if p.starts_with("HIGH-") || p.starts_with("LOW-") || p == "QUOTE" || p == "QUOTES" => {
                Err(self.err(&format!("la constante figurativa {} no está soportada todavía.", p)))
            }
            Some(T::Pal(p)) if matches!(self.vars.get(&p), Some(Var { tipo: Tipo::Txt { .. }, .. })) => {
                let v = self.variable(&p)?;
                self.pos += 1;
                match v.tipo {
                    Tipo::Txt { lon } => Ok(Op::T { cod: v.id, lon, lit: None }),
                    _ => unreachable!(),
                }
            }
            _ => {
                let ini = self.pos;
                let e = self.expr()?;
                let mut lit = None;
                let mut var = None;
                if self.pos - ini == 1 {
                    match &self.toks[ini].t {
                        T::Num(s) => lit = Some(s.clone()),
                        T::Pal(p) => var = self.vars.get(p).cloned(),
                        _ => {}
                    }
                } else if self.pos - ini == 2 {
                    if let (T::Op(o), T::Num(s)) = (&self.toks[ini].t, &self.toks[ini + 1].t) {
                        if o == "-" {
                            lit = Some(format!("-{}", s));
                        } else {
                            lit = Some(s.clone());
                        }
                    }
                }
                Ok(Op::N { e, lit, var })
            }
        }
    }

    // ----- Condiciones -----

    fn cond(&mut self) -> R<String> {
        let mut a = self.cond_y()?;
        while self.acepta("OR") {
            let b = self.cond_y()?;
            a = format!("({} o {})", a, b);
        }
        Ok(a)
    }

    fn cond_y(&mut self) -> R<String> {
        let mut a = self.cond_no()?;
        while self.acepta("AND") {
            let b = self.cond_no()?;
            a = format!("({} y {})", a, b);
        }
        Ok(a)
    }

    fn cond_no(&mut self) -> R<String> {
        if self.acepta("NOT") {
            let c = self.cond_no()?;
            return Ok(format!("no {}", c));
        }
        if self.es_op("(") {
            let guardado = self.pos;
            self.pos += 1;
            if let Ok(c) = self.cond() {
                if self.acepta_op(")") {
                    let sigue_relacion = match self.mirar() {
                        Some(T::Op(o)) => o != ")",
                        Some(T::Pal(p)) => matches!(p.as_str(), "IS" | "EQUAL" | "GREATER" | "LESS" | "NOT"),
                        _ => false,
                    };
                    if !sigue_relacion {
                        return Ok(format!("({})", c));
                    }
                }
            }
            self.pos = guardado;
        }
        self.relacion()
    }

    fn relacion(&mut self) -> R<String> {
        let a = self.operando()?;
        // Condiciones de signo: IS [NOT] POSITIVE / NEGATIVE / ZERO.
        let mut k = 0;
        if self.es_pal_n(0, "IS") {
            k = 1;
        }
        let negado = self.es_pal_n(k, "NOT");
        let k2 = k + negado as usize;
        for (palabra, op) in [("POSITIVE", ">"), ("NEGATIVE", "<"), ("ZERO", "=="), ("ZEROS", "=="), ("ZEROES", "==")] {
            if self.es_pal_n(k2, palabra) {
                self.pos += k2 + 1;
                let cero = Op::N { e: E::constante(0, 0), lit: None, var: None };
                let c = self.comparar(&a, op, &cero)?;
                return Ok(if negado { format!("no {}", c) } else { c });
            }
        }
        if self.es_pal_n(k2, "NUMERIC") || self.es_pal_n(k2, "ALPHABETIC") {
            return Err(self.err("las pruebas NUMERIC / ALPHABETIC no están soportadas todavía."));
        }
        let op = self.op_relacional()?;
        let b = self.operando()?;
        self.comparar(&a, &op, &b)
    }

    fn op_relacional(&mut self) -> R<String> {
        self.acepta("IS");
        let negado = self.acepta("NOT");
        let op = if self.acepta_op("=") || self.acepta("EQUAL") {
            self.acepta("TO");
            "=="
        } else if self.acepta_op("<>") {
            "!="
        } else if self.acepta_op(">=") {
            ">="
        } else if self.acepta_op("<=") {
            "<="
        } else if self.acepta_op(">") || self.acepta("GREATER") {
            self.acepta("THAN");
            if self.es_pal("OR") && self.es_pal_n(1, "EQUAL") {
                self.pos += 2;
                self.acepta("TO");
                ">="
            } else {
                ">"
            }
        } else if self.acepta_op("<") || self.acepta("LESS") {
            self.acepta("THAN");
            if self.es_pal("OR") && self.es_pal_n(1, "EQUAL") {
                self.pos += 2;
                self.acepta("TO");
                "<="
            } else {
                "<"
            }
        } else {
            return Err(self.err("se esperaba una comparación (=, >, <, NOT, EQUAL TO, GREATER THAN...). Las condiciones abreviadas (A > B OR C) no están soportadas todavía."));
        };
        Ok(if negado {
            match op {
                "==" => "!=",
                "!=" => "==",
                "<" => ">=",
                ">" => "<=",
                "<=" => ">",
                _ => "<",
            }
        } else {
            op
        }
        .to_string())
    }

    fn comparar(&self, a: &Op, op: &str, b: &Op) -> R<String> {
        let numero = |o: &Op| -> Option<E> {
            match o {
                Op::N { e, .. } => Some(e.clone()),
                Op::Fig('0') => Some(E::constante(0, 0)),
                _ => None,
            }
        };
        if let (Some(x), Some(y)) = (numero(a), numero(b)) {
            let s = x.esc.max(y.esc);
            return Ok(format!("({} {} {})", x.alinear(s), op, y.alinear(s)));
        }
        let longitud = |o: &Op| match o {
            Op::T { lon, .. } => Some(*lon),
            _ => None,
        };
        let es_texto = |o: &Op| matches!(o, Op::T { .. } | Op::Fig(_));
        if es_texto(a) && es_texto(b) {
            if op != "==" && op != "!=" {
                return Err(self.err("comparar textos con < o > no está soportado todavía (solo = y NOT =)."));
            }
            let l = match (longitud(a), longitud(b)) {
                (Some(x), Some(y)) => x.max(y),
                (Some(x), None) | (None, Some(x)) => x,
                (None, None) => return Err(self.err("no se pueden comparar dos constantes.")),
            };
            let pieza = |o: &Op| -> String {
                match o {
                    Op::T { lit: Some(s), .. } => texto(&ajustar_txt(s, l)),
                    Op::T { cod, lon, .. } => {
                        if *lon == l {
                            cod.clone()
                        } else {
                            format!("cb_texto({}, {})", cod, l)
                        }
                    }
                    Op::Fig(c) => texto(&c.to_string().repeat(l)),
                    _ => unreachable!(),
                }
            };
            return Ok(format!("({} {} {})", pieza(a), op, pieza(b)));
        }
        Err(self.err("no se puede comparar un número con un texto."))
    }

    // ----- Guardar valores -----

    fn guardar(&self, v: &Var, e: &E, redondear: bool) -> R<String> {
        match v.tipo {
            Tipo::Num { dig, esc, signo } => {
                if let Some(k) = e.k {
                    let r = convertir_k(k, e.esc, dig, esc, signo, redondear);
                    return Ok(format!("{} = {}", v.id, if r < 0 { format!("-{}", -r) } else { r.to_string() }));
                }
                let ajustado = if esc >= e.esc {
                    e.alinear(esc)
                } else if redondear {
                    format!("cb_redondear({}, {})", e.cod, pot(e.esc - esc))
                } else {
                    format!("cb_reducir({}, {})", e.cod, pot(e.esc - esc))
                };
                Ok(format!("{} = cb_ajustar_{}({}, {})", v.id, if signo { "con_signo" } else { "sin_signo" }, ajustado, pot(dig)))
            }
            Tipo::Txt { .. } => Err(self.err(&format!("'{}' es un texto y aquí hace falta un número.", v.id))),
        }
    }

    fn actual(v: &Var) -> E {
        match v.tipo {
            Tipo::Num { esc, .. } => E { cod: v.id.clone(), esc, k: None },
            _ => unreachable!(),
        }
    }

    fn lista(&mut self) -> R<Vec<Item>> {
        let mut v = Vec::new();
        loop {
            let ok = match self.mirar() {
                Some(T::Num(_)) => true,
                Some(T::Pal(p)) => self.vars.contains_key(p) || self.grupos.contains(p) || matches!(p.as_str(), "ZERO" | "ZEROS" | "ZEROES"),
                Some(T::Op(o)) => o == "(" || (v.is_empty() && (o == "-" || o == "+")),
                _ => false,
            };
            if !ok {
                break;
            }
            let ini = self.pos;
            let e = self.expr()?;
            let var = if self.pos - ini == 1 {
                match &self.toks[ini].t {
                    T::Pal(p) => self.vars.get(p).cloned(),
                    _ => None,
                }
            } else {
                None
            };
            let red = self.acepta("ROUNDED");
            v.push(Item { e, var, red });
        }
        if v.is_empty() {
            return Err(self.err("se esperaba un número o una variable."));
        }
        Ok(v)
    }

    fn destinos(&self, items: &[Item]) -> R<Vec<(Var, bool)>> {
        let mut r = Vec::new();
        for it in items {
            match &it.var {
                Some(v @ Var { tipo: Tipo::Num { .. }, .. }) => r.push((v.clone(), it.red)),
                _ => return Err(self.err("aquí hace falta una variable numérica donde guardar el resultado.")),
            }
        }
        Ok(r)
    }

    fn fin_verbo(&mut self, fin: &str) -> R<()> {
        if self.es_pal("ON") || self.es_pal("SIZE") || (self.es_pal("NOT") && (self.es_pal_n(1, "ON") || self.es_pal_n(1, "SIZE"))) {
            return Err(self.err("ON SIZE ERROR no está soportado todavía."));
        }
        self.acepta(fin);
        Ok(())
    }

    // ----- Instrucciones -----

    fn bloque(&mut self) -> R<Vec<String>> {
        let mut out = Vec::new();
        loop {
            match self.mirar() {
                None | Some(T::Punto) => break,
                Some(T::Pal(p)) if matches!(p.as_str(), "ELSE" | "END-IF" | "END-EVALUATE" | "END-PERFORM" | "WHEN") => break,
                _ => out.extend(self.sentencia()?),
            }
        }
        Ok(out)
    }

    fn cuerpo(&mut self) -> R<Vec<String>> {
        let mut out = Vec::new();
        loop {
            out.extend(self.bloque()?);
            if self.pos >= self.lim {
                return Ok(out);
            }
            if self.mirar() == Some(&T::Punto) {
                self.pos += 1;
            } else {
                return Err(self.err("no esperaba esta palabra aquí (¿sobra un ELSE, END-IF, END-PERFORM o WHEN?)."));
            }
        }
    }

    fn sentencia(&mut self) -> R<Vec<String>> {
        let verbo = match self.mirar().cloned() {
            Some(T::Pal(p)) => p,
            _ => return Err(self.err("se esperaba una instrucción de COBOL.")),
        };
        self.pos += 1;
        match verbo.as_str() {
            "DISPLAY" => self.v_display(),
            "MOVE" => self.v_move(),
            "ADD" => self.v_add(),
            "SUBTRACT" => self.v_subtract(),
            "MULTIPLY" => self.v_multiply(),
            "DIVIDE" => self.v_divide(),
            "COMPUTE" => self.v_compute(),
            "IF" => self.v_if(),
            "EVALUATE" => self.v_evaluate(),
            "PERFORM" => self.v_perform(),
            "STOP" => {
                self.espera("RUN")?;
                Ok(vec!["devolver nada".into()])
            }
            "GOBACK" => Ok(vec!["devolver nada".into()]),
            "EXIT" => {
                if self.acepta("PROGRAM") {
                    Ok(vec!["devolver nada".into()])
                } else if self.es_pal("PERFORM") || self.es_pal("PARAGRAPH") || self.es_pal("SECTION") {
                    self.pos -= 1;
                    Err(self.err("EXIT PERFORM / EXIT PARAGRAPH / EXIT SECTION no están soportados todavía."))
                } else {
                    Ok(vec![])
                }
            }
            "CONTINUE" => Ok(vec![]),
            otro => {
                self.pos -= 1;
                if self.vars.contains_key(otro) || self.grupos.contains(otro) {
                    Err(self.err(&format!("'{}' no es una instrucción (¿falta un punto o una instrucción antes?).", otro)))
                } else {
                    Err(self.err(&format!("la instrucción {} no está soportada todavía.", otro)))
                }
            }
        }
    }

    fn v_display(&mut self) -> R<Vec<String>> {
        let mut piezas: Vec<String> = Vec::new();
        loop {
            let sigue = match self.mirar() {
                Some(T::Txt(_)) | Some(T::Num(_)) => true,
                Some(T::Op(o)) => o == "-",
                Some(T::Pal(p)) => self.vars.contains_key(p) || self.grupos.contains(p) || matches!(p.as_str(), "SPACE" | "SPACES" | "ZERO" | "ZEROS" | "ZEROES"),
                _ => false,
            };
            if !sigue {
                break;
            }
            match self.operando()? {
                Op::T { cod, .. } => piezas.push(cod),
                Op::Fig(c) => piezas.push(texto(&c.to_string())),
                Op::N { lit: Some(l), .. } => piezas.push(texto(&l)),
                Op::N { var: Some(v), .. } => match v.tipo {
                    Tipo::Num { dig, esc, signo } => piezas.push(format!("cb_formato({}, {}, {}, {})", v.id, dig, esc, if signo { "verdadero" } else { "falso" })),
                    _ => unreachable!(),
                },
                Op::N { .. } => return Err(self.err("DISPLAY solo muestra datos o constantes, no cuentas.")),
            }
        }
        let sin_salto = if self.es_pal("WITH") {
            self.pos += 1;
            self.espera("NO")?;
            self.espera("ADVANCING")?;
            true
        } else {
            false
        };
        if self.es_pal("UPON") {
            return Err(self.err("DISPLAY ... UPON no está soportado todavía."));
        }
        self.acepta("END-DISPLAY");
        let todo = if piezas.is_empty() { "\"\"".to_string() } else { piezas.join(" + ") };
        Ok(vec![format!("{}({})", if sin_salto { "escribir" } else { "mostrar" }, todo)])
    }

    fn v_move(&mut self) -> R<Vec<String>> {
        if self.es_pal("CORRESPONDING") || self.es_pal("CORR") {
            return Err(self.err("MOVE CORRESPONDING no está soportado todavía."));
        }
        let origen = self.operando()?;
        self.espera("TO")?;
        let mut out = Vec::new();
        let mut hay = false;
        while let Some(T::Pal(p)) = self.mirar().cloned() {
            if !self.vars.contains_key(&p) && !self.grupos.contains(&p) {
                break;
            }
            let v = self.variable(&p)?;
            self.pos += 1;
            hay = true;
            match &v.tipo {
                Tipo::Num { .. } => match &origen {
                    Op::N { e, .. } => out.push(self.guardar(&v, e, false)?),
                    Op::Fig('0') => out.push(self.guardar(&v, &E::constante(0, 0), false)?),
                    _ => return Err(self.err(&format!("no se puede mover un texto a '{}', que es numérico.", p))),
                },
                Tipo::Txt { lon } => {
                    let lon = *lon;
                    match &origen {
                        Op::T { lit: Some(s), .. } => out.push(format!("{} = {}", v.id, texto(&ajustar_txt(s, lon)))),
                        Op::T { cod, lon: l2, .. } => {
                            out.push(if *l2 == lon { format!("{} = {}", v.id, cod) } else { format!("{} = cb_texto({}, {})", v.id, cod, lon) })
                        }
                        Op::Fig(c) => out.push(format!("{} = {}", v.id, texto(&c.to_string().repeat(lon)))),
                        Op::N { var: Some(Var { id, tipo: Tipo::Num { dig, esc, signo: false } }), .. } if *esc == 0 => {
                            out.push(format!("{} = cb_texto(cb_formato({}, {}, 0, falso), {})", v.id, id, dig, lon))
                        }
                        Op::N { lit: Some(l), .. } if l.chars().all(|c| c.is_ascii_digit()) => out.push(format!("{} = {}", v.id, texto(&ajustar_txt(l, lon)))),
                        _ => {
                            return Err(self.err("mover un número con signo o con decimales a un texto no está soportado todavía."));
                        }
                    }
                }
            }
        }
        if !hay {
            return Err(self.err("después de TO falta la variable donde guardar."));
        }
        Ok(out)
    }

    fn v_add(&mut self) -> R<Vec<String>> {
        let items = self.lista()?;
        let suma: Vec<E> = items.iter().map(|i| i.e.clone()).collect();
        let mut out = Vec::new();
        if self.acepta("TO") {
            let resto = self.lista()?;
            if self.acepta("GIVING") {
                let dest = self.lista()?;
                let mut todos = suma.clone();
                todos.extend(resto.iter().map(|i| i.e.clone()));
                let total = e_total(&todos);
                for (v, red) in self.destinos(&dest)? {
                    out.push(self.guardar(&v, &total, red)?);
                }
            } else {
                let total = e_total(&suma);
                for (v, red) in self.destinos(&resto)? {
                    out.push(self.guardar(&v, &e_suma(&Self::actual(&v), &total, false), red)?);
                }
            }
        } else if self.acepta("GIVING") {
            let dest = self.lista()?;
            let total = e_total(&suma);
            for (v, red) in self.destinos(&dest)? {
                out.push(self.guardar(&v, &total, red)?);
            }
        } else {
            return Err(self.err("ADD necesita TO o GIVING."));
        }
        self.fin_verbo("END-ADD")?;
        Ok(out)
    }

    fn v_subtract(&mut self) -> R<Vec<String>> {
        let items = self.lista()?;
        let total = e_total(&items.iter().map(|i| i.e.clone()).collect::<Vec<_>>());
        self.espera("FROM")?;
        let resto = self.lista()?;
        let mut out = Vec::new();
        if self.acepta("GIVING") {
            if resto.len() != 1 {
                return Err(self.err("SUBTRACT ... FROM ... GIVING admite un solo dato después de FROM."));
            }
            let dest = self.lista()?;
            let resultado = e_suma(&resto[0].e, &total, true);
            for (v, red) in self.destinos(&dest)? {
                out.push(self.guardar(&v, &resultado, red)?);
            }
        } else {
            for (v, red) in self.destinos(&resto)? {
                out.push(self.guardar(&v, &e_suma(&Self::actual(&v), &total, true), red)?);
            }
        }
        self.fin_verbo("END-SUBTRACT")?;
        Ok(out)
    }

    fn v_multiply(&mut self) -> R<Vec<String>> {
        let a = self.lista()?;
        if a.len() != 1 {
            return Err(self.err("MULTIPLY admite un solo dato antes de BY."));
        }
        self.espera("BY")?;
        let b = self.lista()?;
        let mut out = Vec::new();
        if self.acepta("GIVING") {
            if b.len() != 1 {
                return Err(self.err("MULTIPLY ... BY ... GIVING admite un solo dato después de BY."));
            }
            let dest = self.lista()?;
            let r = e_mult(&a[0].e, &b[0].e);
            for (v, red) in self.destinos(&dest)? {
                out.push(self.guardar(&v, &r, red)?);
            }
        } else {
            for (v, red) in self.destinos(&b)? {
                out.push(self.guardar(&v, &e_mult(&Self::actual(&v), &a[0].e), red)?);
            }
        }
        self.fin_verbo("END-MULTIPLY")?;
        Ok(out)
    }

    fn v_divide(&mut self) -> R<Vec<String>> {
        self.usa_division = true;
        let a = self.lista()?;
        if a.len() != 1 {
            return Err(self.err("DIVIDE admite un solo dato antes de INTO o BY."));
        }
        let por = if self.acepta("INTO") {
            false
        } else if self.acepta("BY") {
            true
        } else {
            return Err(self.err("DIVIDE necesita INTO o BY."));
        };
        let b = self.lista()?;
        let mut out = Vec::new();
        if self.acepta("GIVING") {
            if b.len() != 1 {
                return Err(self.err("DIVIDE ... GIVING admite un solo dato antes de GIVING."));
            }
            let dest = self.lista()?;
            let (dividendo, divisor) = if por { (a[0].e.clone(), b[0].e.clone()) } else { (b[0].e.clone(), a[0].e.clone()) };
            let cociente = e_div(&dividendo, &divisor);
            let destinos = self.destinos(&dest)?;
            for (v, red) in &destinos {
                out.push(self.guardar(v, &cociente, *red)?);
            }
            if self.acepta("REMAINDER") {
                let resto = self.lista()?;
                let q = Self::actual(&destinos[0].0);
                let r = e_suma(&dividendo, &e_mult(&q, &divisor), true);
                for (v, red) in self.destinos(&resto)? {
                    out.push(self.guardar(&v, &r, red)?);
                }
            }
        } else {
            if por {
                return Err(self.err("DIVIDE ... BY necesita GIVING."));
            }
            for (v, red) in self.destinos(&b)? {
                out.push(self.guardar(&v, &e_div(&Self::actual(&v), &a[0].e), red)?);
            }
            if self.es_pal("REMAINDER") {
                return Err(self.err("REMAINDER necesita GIVING."));
            }
        }
        self.fin_verbo("END-DIVIDE")?;
        Ok(out)
    }

    fn v_compute(&mut self) -> R<Vec<String>> {
        let objetivos = self.lista()?;
        if !(self.acepta_op("=") || self.acepta("EQUAL")) {
            return Err(self.err("COMPUTE necesita un '=' después de la variable."));
        }
        let e = self.expr()?;
        let mut out = Vec::new();
        for (v, red) in self.destinos(&objetivos)? {
            out.push(self.guardar(&v, &e, red)?);
        }
        self.fin_verbo("END-COMPUTE")?;
        Ok(out)
    }

    fn v_if(&mut self) -> R<Vec<String>> {
        let c = self.cond()?;
        self.acepta("THEN");
        if self.es_pal("NEXT") {
            return Err(self.err("NEXT SENTENCE no está soportado todavía."));
        }
        let si = self.bloque()?;
        let mut out = vec![format!("si {} {{", c)];
        out.extend(sangrar(si));
        if self.acepta("ELSE") {
            let sino = self.bloque()?;
            out.push("} sino {".into());
            out.extend(sangrar(sino));
        }
        out.push("}".into());
        self.acepta("END-IF");
        Ok(out)
    }

    fn v_evaluate(&mut self) -> R<Vec<String>> {
        let sujeto: Option<Op> = if self.acepta("TRUE") { None } else { Some(self.operando()?) };
        if self.es_pal("ALSO") {
            return Err(self.err("EVALUATE ... ALSO no está soportado todavía."));
        }
        let mut ramas: Vec<(String, Vec<String>)> = Vec::new();
        let mut otra: Option<Vec<String>> = None;
        while self.es_pal("WHEN") {
            let mut conds: Vec<String> = Vec::new();
            let mut es_otra = false;
            while self.acepta("WHEN") {
                if self.acepta("OTHER") {
                    es_otra = true;
                    continue;
                }
                match &sujeto {
                    None => conds.push(self.cond()?),
                    Some(s) => {
                        let v = self.operando()?;
                        if self.acepta("THRU") || self.acepta("THROUGH") {
                            let hasta = self.operando()?;
                            let a = self.comparar(s, ">=", &v)?;
                            let b = self.comparar(s, "<=", &hasta)?;
                            conds.push(format!("({} y {})", a, b));
                        } else {
                            conds.push(self.comparar(s, "==", &v)?);
                        }
                    }
                }
            }
            let cuerpo = self.bloque()?;
            if es_otra {
                otra = Some(cuerpo);
            } else {
                ramas.push((conds.join(" o "), cuerpo));
            }
        }
        self.acepta("END-EVALUATE");
        let mut out = Vec::new();
        for (i, (c, cuerpo)) in ramas.into_iter().enumerate() {
            out.push(if i == 0 { format!("si {} {{", c) } else { format!("}} sino si {} {{", c) });
            out.extend(sangrar(cuerpo));
        }
        match (out.is_empty(), otra) {
            (true, Some(o)) => out.extend(o),
            (false, Some(o)) => {
                out.push("} sino {".into());
                out.extend(sangrar(o));
                out.push("}".into());
            }
            (false, None) => out.push("}".into()),
            (true, None) => {}
        }
        Ok(out)
    }

    fn buscar_parrafo(&self, nombre: &str) -> Option<usize> {
        self.parrafos.iter().position(|(n, _, _)| n == nombre)
    }

    fn expandir(&mut self, a: usize, b: usize) -> R<Vec<String>> {
        let mut out = Vec::new();
        for i in a..=b {
            if self.pila.contains(&i) {
                return Err(self.err(&format!("el párrafo {} se llama a sí mismo (la recursión no está soportada).", self.parrafos[i].0)));
            }
            let (_, ini, fin) = self.parrafos[i].clone();
            self.pila.push(i);
            let (gp, gl) = (self.pos, self.lim);
            self.pos = ini;
            self.lim = fin;
            let r = self.cuerpo();
            self.pos = gp;
            self.lim = gl;
            self.pila.pop();
            out.extend(r?);
        }
        Ok(out)
    }

    fn v_perform(&mut self) -> R<Vec<String>> {
        // Forma con párrafo o en línea.
        let mut cuerpo_parrafo: Option<(usize, usize)> = None;
        if let Some(T::Pal(p)) = self.mirar().cloned() {
            if let Some(a) = self.buscar_parrafo(&p) {
                self.pos += 1;
                let mut b = a;
                if self.acepta("THRU") || self.acepta("THROUGH") {
                    match self.mirar().cloned() {
                        Some(T::Pal(q)) => {
                            b = self.buscar_parrafo(&q).ok_or_else(|| self.err(&format!("el párrafo {} no existe.", q)))?;
                            self.pos += 1;
                            if b < a {
                                return Err(self.err("en PERFORM ... THRU el segundo párrafo va antes que el primero."));
                            }
                        }
                        _ => return Err(self.err("después de THRU falta el nombre del párrafo.")),
                    }
                }
                cuerpo_parrafo = Some((a, b));
            } else if matches!(p.as_str(), "FOREVER") {
                return Err(self.err("PERFORM FOREVER no está soportado todavía."));
            }
        }
        enum Bucle {
            Una,
            Veces(E),
            Hasta(String),
            Variando(Var, E, E, String),
        }
        let mut bucle = Bucle::Una;
        if self.acepta("WITH") {
            self.espera("TEST")?;
            if self.acepta("AFTER") {
                return Err(self.err("WITH TEST AFTER no está soportado todavía."));
            }
            self.espera("BEFORE")?;
        }
        if self.acepta("UNTIL") {
            bucle = Bucle::Hasta(self.cond()?);
        } else if self.acepta("VARYING") {
            let v = match self.mirar().cloned() {
                Some(T::Pal(p)) => self.variable(&p)?,
                _ => return Err(self.err("después de VARYING falta la variable.")),
            };
            if !matches!(v.tipo, Tipo::Num { .. }) {
                return Err(self.err("la variable de VARYING debe ser numérica."));
            }
            self.pos += 1;
            self.espera("FROM")?;
            let desde = self.expr()?;
            let paso = if self.acepta("BY") { self.expr()? } else { E::constante(1, 0) };
            self.espera("UNTIL")?;
            let c = self.cond()?;
            if self.es_pal("AFTER") {
                return Err(self.err("PERFORM VARYING ... AFTER (bucles anidados) no está soportado todavía."));
            }
            bucle = Bucle::Variando(v, desde, paso, c);
        } else if matches!(self.mirar(), Some(T::Num(_)) | Some(T::Pal(_)) | Some(T::Op(_))) && self.hay_times() {
            let e = self.expr()?;
            self.espera("TIMES")?;
            bucle = Bucle::Veces(e);
        }
        let cuerpo = match cuerpo_parrafo {
            Some((a, b)) => self.expandir(a, b)?,
            None => {
                let c = self.bloque()?;
                if !self.acepta("END-PERFORM") {
                    return Err(self.err("falta END-PERFORM al final del PERFORM en línea."));
                }
                c
            }
        };
        let mut out = Vec::new();
        match bucle {
            Bucle::Una => out.extend(cuerpo),
            Bucle::Hasta(c) => {
                out.push(format!("mientras no {} {{", c));
                out.extend(sangrar(cuerpo));
                out.push("}".into());
            }
            Bucle::Veces(e) => {
                let t = self.nuevo_temporal("vuelta");
                let n = self.nuevo_temporal("veces");
                let veces = match e.k {
                    Some(k) => convertir_k(k, e.esc, 15, 0, true, false).to_string(),
                    None if e.esc == 0 => e.cod.clone(),
                    None => format!("cb_reducir({}, {})", e.cod, pot(e.esc)),
                };
                out.push(format!("sea {} = 0", t));
                out.push(format!("sea {} = {}", n, veces));
                out.push(format!("mientras {} < {} {{", t, n));
                out.extend(sangrar(cuerpo));
                out.push(format!("    {} = {} + 1", t, t));
                out.push("}".into());
            }
            Bucle::Variando(v, desde, paso, c) => {
                out.push(self.guardar(&v, &desde, false)?);
                out.push(format!("mientras no {} {{", c));
                let mut dentro = cuerpo;
                dentro.push(self.guardar(&v, &e_suma(&Self::actual(&v), &paso, false), false)?);
                out.extend(sangrar(dentro));
                out.push("}".into());
            }
        }
        Ok(out)
    }

    /// ¿Hay un `TIMES` más adelante antes de acabar la frase? (para `PERFORM n TIMES`)
    fn hay_times(&self) -> bool {
        let mut i = self.pos;
        while i < self.lim {
            match &self.toks[i].t {
                T::Pal(p) if p == "TIMES" => return true,
                T::Num(_) | T::Op(_) => i += 1,
                T::Pal(p) if self.vars.contains_key(p) => i += 1,
                _ => return false,
            }
        }
        false
    }
}

fn sangrar(lineas: Vec<String>) -> Vec<String> {
    lineas.into_iter().map(|l| format!("    {}", l)).collect()
}

// ---------------------------------------------------------------------------
// Programa completo.
// ---------------------------------------------------------------------------

/// Traduce un programa COBOL. `nombre` solo se usa para el comentario del principio.
pub fn traducir(cobol: &str, nombre: &str) -> Result<Traduccion, String> {
    let libre = es_formato_libre(cobol);
    let utiles = lineas_utiles(cobol, libre)?;
    let toks = leer_piezas(&utiles)?;
    let fuente: Vec<String> = cobol.lines().map(|l| l.to_string()).collect();
    // Divisiones: posición de cada "X DIVISION".
    let mut divisiones: Vec<(String, usize)> = Vec::new();
    for i in 0..toks.len() {
        if let (T::Pal(a), Some(Tok { t: T::Pal(b), .. })) = (&toks[i].t, toks.get(i + 1)) {
            if b == "DIVISION" {
                divisiones.push((a.clone(), i));
            }
        }
    }
    let n = toks.len();
    let mut g = G {
        toks,
        fuente,
        pos: 0,
        lim: n,
        vars: HashMap::new(),
        grupos: HashSet::new(),
        parrafos: Vec::new(),
        pila: Vec::new(),
        contador: 0,
        usa_division: false,
        avisos: Vec::new(),
    };
    if divisiones.is_empty() {
        return Err("Error: no encuentro ninguna división (IDENTIFICATION DIVISION, DATA DIVISION, PROCEDURE DIVISION...): ¿es un programa COBOL?".into());
    }
    let proc_pos = divisiones
        .iter()
        .find(|(n, _)| n == "PROCEDURE")
        .map(|(_, p)| *p)
        .ok_or("Error: el programa no tiene PROCEDURE DIVISION.")?;
    let fin_de = |i: usize| divisiones.iter().map(|(_, p)| *p).filter(|p| *p > i).min().unwrap_or(n);
    // Datos.
    let mut declaraciones = Vec::new();
    for (nom, p) in &divisiones {
        if nom == "DATA" {
            g.pos = *p + 2;
            if g.mirar_pos_punto() {
                g.pos += 1;
            }
            g.lim = fin_de(*p);
            declaraciones = g.datos()?;
        }
    }
    // Procedimiento.
    let mut ini = proc_pos + 2;
    g.pos = ini;
    g.lim = n;
    if g.es_pal("USING") || g.es_pal("RETURNING") {
        return Err(g.err("PROCEDURE DIVISION USING no está soportado todavía."));
    }
    if g.mirar() == Some(&T::Punto) {
        ini += 1;
    }
    // Párrafos: un nombre solo al principio de frase, seguido de un punto.
    let mut cabeceras: Vec<(String, usize, usize)> = Vec::new(); // (nombre, posición de la cabecera, inicio del cuerpo)
    let mut inicio_frase = true;
    let mut i = ini;
    while i < n {
        match &g.toks[i].t {
            T::Punto => inicio_frase = true,
            T::Pal(p) if inicio_frase => {
                if matches!(g.toks.get(i + 1), Some(Tok { t: T::Pal(s), .. }) if s == "SECTION") {
                    g.pos = i;
                    return Err(g.err("las SECTION del procedimiento no están soportadas todavía (usa solo párrafos)."));
                }
                if matches!(g.toks.get(i + 1), Some(Tok { t: T::Punto, .. })) && !matches!(p.as_str(), "EXIT" | "CONTINUE" | "GOBACK") {
                    cabeceras.push((p.clone(), i, i + 2));
                    i += 2;
                    inicio_frase = true;
                    continue;
                }
                inicio_frase = false;
            }
            _ => inicio_frase = false,
        }
        i += 1;
    }
    let primero = cabeceras.first().map(|c| c.1).unwrap_or(n);
    if primero > ini {
        g.parrafos.push((String::new(), ini, primero));
    }
    for (k, (nom, _, cuerpo)) in cabeceras.iter().enumerate() {
        let fin = cabeceras.get(k + 1).map(|c| c.1).unwrap_or(n);
        if g.parrafos.iter().any(|(x, _, _)| x == nom) {
            g.pos = *cuerpo - 2;
            return Err(g.err(&format!("el párrafo {} está repetido.", nom)));
        }
        g.parrafos.push((nom.clone(), *cuerpo, fin));
    }
    if g.parrafos.is_empty() {
        return Err("Error: la PROCEDURE DIVISION está vacía.".into());
    }
    let ultimo = g.parrafos.len() - 1;
    let cuerpo = g.expandir(0, ultimo)?;
    if g.usa_division {
        g.avisos.push((
            0,
            "las divisiones se calculan con 8 decimales y se truncan al guardar; dividir entre cero detiene el programa (COBOL seguiría).".to_string(),
        ));
    }
    let mut codigo = format!("# Traducido de COBOL ({}) con deimon desde-cobol.\n", nombre);
    codigo.push_str(AYUDANTES);
    codigo.push_str("\nfuncion principal() {\n");
    for d in &declaraciones {
        codigo.push_str(&format!("    {}\n", d));
    }
    for l in &cuerpo {
        codigo.push_str(&format!("    {}\n", l));
    }
    codigo.push_str("}\n\nprincipal()\n");
    Ok(Traduccion { codigo, avisos: g.avisos })
}

impl G {
    fn mirar_pos_punto(&self) -> bool {
        self.mirar() == Some(&T::Punto)
    }
}
