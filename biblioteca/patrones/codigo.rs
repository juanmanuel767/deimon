// Motor de expresiones regulares propio (sin crates externos), que trabaja con caracteres.
// Soporta: literales, `.`, clases [a-z] [^...], \d \w \s \D \W \S \b \B, ^ $, grupos ( ) y (?: ),
// alternativa |, cuantificadores * + ? {n} {n,} {n,m} (y perezosos con ?), escapes y el
// prefijo (?i) para ignorar mayúsculas. No soporta: mirar adelante/atrás ni referencias \1.

const PATRON_LIMITE_PASOS: u64 = 5_000_000;
const PATRON_LIMITE_PROFUNDIDAD: usize = 200_000;

enum PatronItem {
    Uno(char),
    Rango(char, char),
    Digito(bool),
    Palabra(bool),
    Espacio(bool),
}

enum PatronNodo {
    Car(char),
    Cualquiera,
    Clase(Vec<PatronItem>, bool),
    Inicio,
    Fin,
    Limite(bool),
    Grupo(Box<PatronNodo>, Option<usize>),
    Alt(Vec<PatronNodo>),
    Sec(Vec<PatronNodo>),
    Rep(Box<PatronNodo>, usize, Option<usize>, bool),
}

struct PatronCompilado {
    raiz: PatronNodo,
    grupos: usize,
    ignorar: bool,
}

struct PatronAnalizador {
    c: Vec<char>,
    i: usize,
    grupos: usize,
}

fn patron_error<T>(pos: usize, mensaje: &str) -> Result<T, String> {
    Err(format!("patrón mal escrito, en la posición {}: {}", pos + 1, mensaje))
}

impl PatronAnalizador {
    fn alternativa(&mut self) -> Result<PatronNodo, String> {
        let mut opciones = vec![self.secuencia()?];
        while self.c.get(self.i) == Some(&'|') {
            self.i += 1;
            opciones.push(self.secuencia()?);
        }
        Ok(if opciones.len() == 1 { opciones.pop().unwrap() } else { PatronNodo::Alt(opciones) })
    }

    fn secuencia(&mut self) -> Result<PatronNodo, String> {
        let mut items = Vec::new();
        while let Some(&ch) = self.c.get(self.i) {
            if ch == '|' || ch == ')' {
                break;
            }
            let inicio = self.i;
            let atomo = self.atomo()?;
            let atomo = self.cuantificar(atomo, inicio)?;
            items.push(atomo);
        }
        Ok(PatronNodo::Sec(items))
    }

    fn cuantificar(&mut self, atomo: PatronNodo, inicio: usize) -> Result<PatronNodo, String> {
        let (min, max) = match self.c.get(self.i) {
            Some('*') => { self.i += 1; (0, None) }
            Some('+') => { self.i += 1; (1, None) }
            Some('?') => { self.i += 1; (0, Some(1)) }
            Some('{') => match self.llaves() {
                Some(r) => r?,
                None => return Ok(atomo),
            },
            _ => return Ok(atomo),
        };
        if matches!(atomo, PatronNodo::Inicio | PatronNodo::Fin | PatronNodo::Limite(_)) {
            return patron_error(inicio, "no se puede repetir ^, $ ni \\b");
        }
        let perezoso = if self.c.get(self.i) == Some(&'?') { self.i += 1; true } else { false };
        if matches!(self.c.get(self.i), Some('*') | Some('+') | Some('?')) {
            return patron_error(self.i, "no se puede repetir dos veces seguidas");
        }
        Ok(PatronNodo::Rep(Box::new(atomo), min, max, perezoso))
    }

    // Lee {n}, {n,} o {n,m}. Si no tiene esa forma, la llave es un carácter normal (None).
    fn llaves(&mut self) -> Option<Result<(usize, Option<usize>), String>> {
        let mut j = self.i + 1;
        let mut numero = |j: &mut usize| -> Option<usize> {
            let desde = *j;
            while self.c.get(*j).map_or(false, |c| c.is_ascii_digit()) {
                *j += 1;
            }
            if *j == desde { None } else { self.c[desde..*j].iter().collect::<String>().parse().ok() }
        };
        let min = numero(&mut j)?;
        let max = if self.c.get(j) == Some(&',') {
            j += 1;
            numero(&mut j)
        } else {
            Some(min)
        };
        if self.c.get(j) != Some(&'}') {
            return None;
        }
        let pos = self.i;
        self.i = j + 1;
        if let Some(m) = max {
            if m < min {
                return Some(patron_error(pos, "en {n,m} el primer número no puede ser mayor que el segundo"));
            }
        }
        if min > 1000 || max.map_or(false, |m| m > 1000) {
            return Some(patron_error(pos, "las repeticiones {n,m} no pueden pasar de 1000"));
        }
        Some(Ok((min, max)))
    }

    fn atomo(&mut self) -> Result<PatronNodo, String> {
        let pos = self.i;
        let ch = self.c[self.i];
        self.i += 1;
        match ch {
            '.' => Ok(PatronNodo::Cualquiera),
            '^' => Ok(PatronNodo::Inicio),
            '$' => Ok(PatronNodo::Fin),
            '*' | '+' | '?' => patron_error(pos, "no hay nada que repetir antes de este símbolo"),
            '(' => self.grupo(pos),
            '[' => self.clase(pos),
            '\\' => self.escape(pos),
            otro => Ok(PatronNodo::Car(otro)),
        }
    }

    fn grupo(&mut self, pos: usize) -> Result<PatronNodo, String> {
        let mut numero = None;
        if self.c.get(self.i) == Some(&'?') {
            match self.c.get(self.i + 1) {
                Some(':') => self.i += 2,
                Some('=') | Some('!') | Some('<') => {
                    return patron_error(pos, "mirar adelante o atrás no está soportado");
                }
                Some('i') => return patron_error(pos, "(?i) solo puede ir al principio del patrón"),
                _ => return patron_error(pos, "este tipo de grupo no está soportado"),
            }
        } else {
            self.grupos += 1;
            numero = Some(self.grupos);
        }
        let dentro = self.alternativa()?;
        if self.c.get(self.i) != Some(&')') {
            return patron_error(pos, "falta cerrar este paréntesis con )");
        }
        self.i += 1;
        Ok(PatronNodo::Grupo(Box::new(dentro), numero))
    }

    fn escape_comun(&self, pos: usize, ch: char) -> Result<PatronItem, String> {
        Ok(match ch {
            'd' => PatronItem::Digito(true),
            'D' => PatronItem::Digito(false),
            'w' => PatronItem::Palabra(true),
            'W' => PatronItem::Palabra(false),
            's' => PatronItem::Espacio(true),
            'S' => PatronItem::Espacio(false),
            'n' => PatronItem::Uno('\n'),
            't' => PatronItem::Uno('\t'),
            'r' => PatronItem::Uno('\r'),
            '0'..='9' => return patron_error(pos, "las referencias a grupos como \\1 no están soportadas"),
            c if c.is_alphabetic() => return patron_error(pos, &format!("el escape \\{} no existe", c)),
            c => PatronItem::Uno(c),
        })
    }

    fn escape(&mut self, pos: usize) -> Result<PatronNodo, String> {
        let Some(&ch) = self.c.get(self.i) else {
            return patron_error(pos, "el patrón no puede acabar en \\");
        };
        self.i += 1;
        match ch {
            'b' => Ok(PatronNodo::Limite(true)),
            'B' => Ok(PatronNodo::Limite(false)),
            _ => match self.escape_comun(pos, ch)? {
                PatronItem::Uno(c) => Ok(PatronNodo::Car(c)),
                item => Ok(PatronNodo::Clase(vec![item], false)),
            },
        }
    }

    fn clase(&mut self, pos: usize) -> Result<PatronNodo, String> {
        let negada = if self.c.get(self.i) == Some(&'^') { self.i += 1; true } else { false };
        let mut items = Vec::new();
        let mut primero = true;
        loop {
            let Some(&ch) = self.c.get(self.i) else {
                return patron_error(pos, "falta cerrar esta clase con ]");
            };
            if ch == ']' && !primero {
                self.i += 1;
                break;
            }
            primero = false;
            let inicio = self.i;
            self.i += 1;
            let item = if ch == '\\' {
                let Some(&e) = self.c.get(self.i) else {
                    return patron_error(inicio, "el patrón no puede acabar en \\");
                };
                self.i += 1;
                self.escape_comun(inicio, e)?
            } else {
                PatronItem::Uno(ch)
            };
            // ¿Es un rango a-z?
            if let PatronItem::Uno(desde) = item {
                if self.c.get(self.i) == Some(&'-') && self.c.get(self.i + 1).map_or(false, |&c| c != ']') {
                    let mut hasta = self.c[self.i + 1];
                    let mut avance = 2;
                    if hasta == '\\' {
                        let Some(&e) = self.c.get(self.i + 2) else {
                            return patron_error(self.i + 1, "el patrón no puede acabar en \\");
                        };
                        match self.escape_comun(self.i + 1, e)? {
                            PatronItem::Uno(c) => hasta = c,
                            _ => return patron_error(self.i + 1, "un rango no puede acabar en \\d, \\w ni \\s"),
                        }
                        avance = 3;
                    }
                    if hasta < desde {
                        return patron_error(inicio, "el rango está al revés (el primero es mayor que el último)");
                    }
                    self.i += avance;
                    items.push(PatronItem::Rango(desde, hasta));
                    continue;
                }
            }
            items.push(item);
        }
        Ok(PatronNodo::Clase(items, negada))
    }
}

fn patron_compilar(patron: &str) -> Result<PatronCompilado, String> {
    let (ignorar, texto) = match patron.strip_prefix("(?i)") {
        Some(resto) => (true, resto),
        None => (false, patron),
    };
    let desplazo = patron.chars().count() - texto.chars().count();
    let mut a = PatronAnalizador { c: texto.chars().collect(), i: 0, grupos: 0 };
    let raiz = match a.alternativa() {
        Ok(r) => r,
        Err(e) => return Err(patron_ajustar(e, desplazo)),
    };
    if a.i < a.c.len() {
        return patron_error(a.i + desplazo, "este paréntesis ) no tiene su pareja (");
    }
    Ok(PatronCompilado { raiz, grupos: a.grupos, ignorar })
}

// Suma el desplazamiento del prefijo (?i) al número de posición de un error ya formado.
fn patron_ajustar(error: String, desplazo: usize) -> String {
    if desplazo == 0 {
        return error;
    }
    let marca = "en la posición ";
    if let Some(p) = error.find(marca) {
        let resto = &error[p + marca.len()..];
        let digitos: String = resto.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digitos.parse::<usize>() {
            return format!("{}{}{}{}", &error[..p + marca.len()], n + desplazo, "", &resto[digitos.len()..]);
        }
    }
    error
}

fn patron_es_palabra(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn patron_minuscula(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn patron_mayuscula(c: char) -> char {
    c.to_uppercase().next().unwrap_or(c)
}

fn patron_item_coincide(item: &PatronItem, c: char) -> bool {
    match item {
        PatronItem::Uno(x) => *x == c,
        PatronItem::Rango(a, b) => *a <= c && c <= *b,
        PatronItem::Digito(si) => c.is_ascii_digit() == *si,
        PatronItem::Palabra(si) => patron_es_palabra(c) == *si,
        PatronItem::Espacio(si) => c.is_whitespace() == *si,
    }
}

struct PatronEjecutor<'a> {
    c: &'a [char],
    ignorar: bool,
    pasos: u64,
    profundidad: usize,
    grupos: Vec<Option<(usize, usize)>>,
}

type PatronSiguiente<'k> = &'k mut dyn FnMut(&mut PatronEjecutor, usize) -> Result<bool, String>;

impl<'a> PatronEjecutor<'a> {
    fn paso(&mut self) -> Result<(), String> {
        self.pasos += 1;
        if self.pasos > PATRON_LIMITE_PASOS {
            return Err("el patrón es demasiado complicado para este texto y tardaría muchísimo; simplifícalo (por ejemplo, evita repeticiones dentro de repeticiones como (a+)+)".to_string());
        }
        Ok(())
    }

    // ¿El carácter en la posición i cumple este nodo de un solo carácter?
    fn uno(&self, nodo: &PatronNodo, i: usize) -> Option<bool> {
        let c = match self.c.get(i) {
            Some(&c) => c,
            None => return Some(false),
        };
        match nodo {
            PatronNodo::Car(x) => Some(*x == c || (self.ignorar && patron_minuscula(*x) == patron_minuscula(c))),
            PatronNodo::Cualquiera => Some(c != '\n'),
            PatronNodo::Clase(items, negada) => {
                let mut dentro = items.iter().any(|it| patron_item_coincide(it, c));
                if !dentro && self.ignorar {
                    let (m, may) = (patron_minuscula(c), patron_mayuscula(c));
                    dentro = items.iter().any(|it| patron_item_coincide(it, m) || patron_item_coincide(it, may));
                }
                Some(dentro != *negada)
            }
            _ => None,
        }
    }

    fn emp(&mut self, nodo: &PatronNodo, i: usize, k: PatronSiguiente) -> Result<bool, String> {
        self.paso()?;
        if self.profundidad > PATRON_LIMITE_PROFUNDIDAD {
            return Err("el patrón es demasiado complicado para este texto; simplifícalo".to_string());
        }
        self.profundidad += 1;
        let r = self.emp_dentro(nodo, i, k);
        self.profundidad -= 1;
        r
    }

    fn emp_dentro(&mut self, nodo: &PatronNodo, i: usize, k: PatronSiguiente) -> Result<bool, String> {
        match nodo {
            PatronNodo::Car(_) | PatronNodo::Cualquiera | PatronNodo::Clase(..) => {
                if self.uno(nodo, i) == Some(true) { k(self, i + 1) } else { Ok(false) }
            }
            PatronNodo::Inicio => if i == 0 { k(self, i) } else { Ok(false) },
            PatronNodo::Fin => if i == self.c.len() { k(self, i) } else { Ok(false) },
            PatronNodo::Limite(si) => {
                let antes = i > 0 && patron_es_palabra(self.c[i - 1]);
                let despues = i < self.c.len() && patron_es_palabra(self.c[i]);
                if (antes != despues) == *si { k(self, i) } else { Ok(false) }
            }
            PatronNodo::Grupo(dentro, None) => self.emp(dentro, i, k),
            PatronNodo::Grupo(dentro, Some(n)) => {
                let n = *n;
                let anterior = self.grupos[n];
                let r = self.emp(dentro, i, &mut |s, j| {
                    let previo = s.grupos[n];
                    s.grupos[n] = Some((i, j));
                    if k(s, j)? { return Ok(true); }
                    s.grupos[n] = previo;
                    Ok(false)
                })?;
                if !r { self.grupos[n] = anterior; }
                Ok(r)
            }
            PatronNodo::Alt(opciones) => {
                for o in opciones {
                    if self.emp(o, i, k)? { return Ok(true); }
                }
                Ok(false)
            }
            PatronNodo::Sec(items) => self.sec(items, i, k),
            PatronNodo::Rep(dentro, min, max, perezoso) => {
                if self.uno(dentro, i).is_some() {
                    return self.rep_simple(dentro, *min, *max, *perezoso, i, k);
                }
                self.rep(dentro, *min, *max, *perezoso, 0, i, k)
            }
        }
    }

    fn sec(&mut self, items: &[PatronNodo], i: usize, k: PatronSiguiente) -> Result<bool, String> {
        match items.split_first() {
            None => k(self, i),
            Some((primero, resto)) => self.emp(primero, i, &mut |s, j| s.sec(resto, j, k)),
        }
    }

    // Repetición de un solo carácter (a*, [0-9]+, .*): sin recursión por cada repetición.
    fn rep_simple(&mut self, nodo: &PatronNodo, min: usize, max: Option<usize>, perezoso: bool, i: usize, k: PatronSiguiente) -> Result<bool, String> {
        let tope = max.unwrap_or(usize::MAX);
        if perezoso {
            let mut n = 0;
            loop {
                if n >= min {
                    self.paso()?;
                    if k(self, i + n)? { return Ok(true); }
                }
                if n >= tope || self.uno(nodo, i + n) != Some(true) { return Ok(false); }
                self.paso()?;
                n += 1;
            }
        }
        let mut n = 0;
        while n < tope && self.uno(nodo, i + n) == Some(true) {
            self.paso()?;
            n += 1;
        }
        loop {
            if n < min { return Ok(false); }
            self.paso()?;
            if k(self, i + n)? { return Ok(true); }
            if n == 0 { return Ok(false); }
            n -= 1;
        }
    }

    fn rep(&mut self, nodo: &PatronNodo, min: usize, max: Option<usize>, perezoso: bool, cuenta: usize, i: usize, k: PatronSiguiente) -> Result<bool, String> {
        if cuenta < min {
            return self.emp(nodo, i, &mut |s, j| s.rep(nodo, min, max, perezoso, cuenta + 1, j, k));
        }
        let puede_mas = max.map_or(true, |m| cuenta < m);
        if perezoso {
            if k(self, i)? { return Ok(true); }
            if !puede_mas { return Ok(false); }
            return self.emp(nodo, i, &mut |s, j| {
                if j == i { Ok(false) } else { s.rep(nodo, min, max, perezoso, cuenta + 1, j, k) }
            });
        }
        if puede_mas {
            let r = self.emp(nodo, i, &mut |s, j| {
                if j == i { Ok(false) } else { s.rep(nodo, min, max, perezoso, cuenta + 1, j, k) }
            })?;
            if r { return Ok(true); }
        }
        k(self, i)
    }
}

// Busca la primera coincidencia desde `desde`. Devuelve los límites de todo y de cada grupo.
fn patron_buscar_desde(
    p: &PatronCompilado,
    c: &[char],
    desde: usize,
    pasos: &mut u64,
) -> Result<Option<Vec<Option<(usize, usize)>>>, String> {
    let mut e = PatronEjecutor { c, ignorar: p.ignorar, pasos: *pasos, profundidad: 0, grupos: vec![None; p.grupos + 1] };
    for inicio in desde..=c.len() {
        for g in e.grupos.iter_mut() { *g = None; }
        let mut fin = None;
        let r = e.emp(&p.raiz, inicio, &mut |_, j| { fin = Some(j); Ok(true) });
        if let Err(m) = r {
            *pasos = e.pasos;
            return Err(m);
        }
        if let Some(j) = fin {
            e.grupos[0] = Some((inicio, j));
            *pasos = e.pasos;
            return Ok(Some(e.grupos));
        }
    }
    *pasos = e.pasos;
    Ok(None)
}

// Todas las coincidencias, sin solaparse (tras una vacía se avanza un carácter).
fn patron_buscar_todas(p: &PatronCompilado, c: &[char]) -> Result<Vec<Vec<Option<(usize, usize)>>>, String> {
    let mut pasos = 0;
    let mut todas = Vec::new();
    let mut pos = 0;
    while pos <= c.len() {
        match patron_buscar_desde(p, c, pos, &mut pasos)? {
            None => break,
            Some(g) => {
                let (s, f) = g[0].unwrap();
                pos = if f == s { f + 1 } else { f };
                todas.push(g);
            }
        }
    }
    Ok(todas)
}

fn patron_a_textos(c: &[char], g: &[Option<(usize, usize)>]) -> Vec<String> {
    g.iter()
        .map(|r| match r {
            Some((a, b)) => c[*a..*b].iter().collect::<String>(),
            None => String::new(),
        })
        .collect()
}

fn patron_lista(textos: Vec<String>) -> Valor {
    Valor::lista(textos.into_iter().map(Valor::texto).collect())
}

fn patron_texto(args: &[Valor], indice: usize, funcion: &str) -> Result<String, String> {
    match args.get(indice).and_then(|v| v.como_texto()) {
        Some(t) => Ok(t.to_string()),
        None => Err(format!("{}() espera un texto en el argumento {}", funcion, indice + 1)),
    }
}

// Las funciones corren en un hilo con mucha pila, porque el motor es recursivo.
fn patron_en_hilo<T: Send>(f: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
    std::thread::scope(|ambito| {
        let hilo = std::thread::Builder::new()
            .stack_size(400 * 1024 * 1024)
            .spawn_scoped(ambito, f)
            .map_err(|_| "no se pudo preparar la búsqueda con patrones".to_string())?;
        hilo.join().map_err(|_| "la búsqueda con patrones falló".to_string())?
    })
}

fn patron_args(args: &[Valor], nombre: &str, cuantos: usize) -> Result<Vec<String>, String> {
    (0..cuantos).map(|i| patron_texto(args, i, nombre)).collect()
}

fn ext_coincide_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "coincide_patron", 2)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        Ok(patron_buscar_desde(&p, &c, 0, &mut 0)?.is_some())
    })
    .map(Valor::Logico)
}

fn ext_buscar_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "buscar_patron", 2)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        Ok(patron_buscar_desde(&p, &c, 0, &mut 0)?.map(|g| patron_a_textos(&c, &g)))
    })
    .map(|r| match r {
        Some(textos) => patron_lista(textos),
        None => Valor::Nada,
    })
}

fn ext_buscar_todos_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "buscar_todos_patron", 2)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        let todas = patron_buscar_todas(&p, &c)?;
        Ok(todas.iter().map(|g| patron_a_textos(&c, g)).collect::<Vec<_>>())
    })
    .map(|filas| Valor::lista(filas.into_iter().map(patron_lista).collect()))
}

fn ext_reemplazar_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "reemplazar_patron", 3)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        let nuevo: Vec<char> = a[2].chars().collect();
        let mut salida = String::new();
        let mut ultimo = 0;
        for g in patron_buscar_todas(&p, &c)? {
            let (s, f) = g[0].unwrap();
            salida.extend(&c[ultimo..s]);
            let mut j = 0;
            while j < nuevo.len() {
                let ch = nuevo[j];
                let mut n_inicio = None;
                if ch == '$' && j + 1 < nuevo.len() {
                    if nuevo[j + 1].is_ascii_digit() {
                        n_inicio = Some((j + 1, false));
                    } else if nuevo[j + 1] == '{' {
                        n_inicio = Some((j + 2, true));
                    }
                }
                let Some((d, llaves)) = n_inicio else {
                    salida.push(ch);
                    j += 1;
                    continue;
                };
                let mut fin = d;
                while fin < nuevo.len() && nuevo[fin].is_ascii_digit() && (llaves || fin - d < 2) {
                    fin += 1;
                }
                if fin == d || (llaves && nuevo.get(fin) != Some(&'}')) {
                    salida.push(ch);
                    j += 1;
                    continue;
                }
                let n: usize = nuevo[d..fin].iter().collect::<String>().parse().unwrap_or(usize::MAX);
                if let Some(Some((a, b))) = g.get(n) {
                    salida.extend(&c[*a..*b]);
                }
                j = if llaves { fin + 1 } else { fin };
            }
            ultimo = f;
        }
        salida.extend(&c[ultimo..]);
        Ok(salida)
    })
    .map(Valor::texto)
}

fn ext_partir_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "partir_patron", 2)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        let mut trozos = Vec::new();
        let mut ultimo = 0;
        for g in patron_buscar_todas(&p, &c)? {
            let (s, f) = g[0].unwrap();
            if f == s && (s == 0 || s == c.len()) {
                continue;
            }
            trozos.push(c[ultimo..s].iter().collect::<String>());
            ultimo = f;
        }
        trozos.push(c[ultimo..].iter().collect::<String>());
        Ok(trozos)
    })
    .map(patron_lista)
}

fn ext_buscar_columnas_patron(args: Vec<Valor>) -> Result<Valor, String> {
    let a = patron_args(&args, "buscar_columnas_patron", 2)?;
    patron_en_hilo(|| {
        let p = patron_compilar(&a[1])?;
        let c: Vec<char> = a[0].chars().collect();
        let mut columnas: Vec<Vec<String>> = vec![Vec::new(); p.grupos + 1];
        for g in patron_buscar_todas(&p, &c)? {
            for (n, texto) in patron_a_textos(&c, &g).into_iter().enumerate() {
                columnas[n].push(texto);
            }
        }
        Ok(columnas)
    })
    .map(|columnas| Valor::lista(columnas.into_iter().map(patron_lista).collect()))
}
