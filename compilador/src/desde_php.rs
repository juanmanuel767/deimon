//! Traductor de PHP a Deimon (`deimon desde-php archivo.php`).
//!
//! Traduce scripts de PHP: variables, echo/print, if/elseif/else, while, for, foreach,
//! funciones, return, listas y las funciones de texto y números más usadas. Lo que no sabe
//! traducir NUNCA se pierde en silencio: queda como comentario `# PHP sin traducir` y se
//! informa con su línea, para que una persona (o una IA) lo termine.

use std::collections::{BTreeSet, HashSet};

use crate::{integradas, lexer};

/// Resultado de traducir un archivo PHP.
pub struct Traduccion {
    pub codigo: String,
    /// (línea del PHP, explicación) de lo que no se pudo traducir.
    pub pendientes: Vec<(usize, String)>,
    /// (línea del PHP, explicación) de diferencias sutiles que conviene revisar.
    pub avisos: Vec<(usize, String)>,
}

// ---------------------------------------------------------------------------
// Lectura del PHP en piezas.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Pieza {
    Variable(String),
    Nombre(String),
    Numero(String),
    /// Texto con sus trozos: literales y variables a sustituir ("Hola $nombre").
    Texto(Vec<Trozo>),
    Signo(String),
    Html(String),
    Fin,
}

#[derive(Debug, Clone, PartialEq)]
enum Trozo {
    Literal(String),
    Variable(String),
    /// "$a[0]", "$a[clave]" o "{$a['clave']}": variable, clave y si la clave es un número.
    Indice(String, String, bool),
}

#[derive(Debug, Clone)]
struct PiezaConLinea {
    pieza: Pieza,
    linea: usize,
}

fn leer_piezas(php: &str) -> Result<Vec<PiezaConLinea>, String> {
    let caracteres: Vec<char> = php.chars().collect();
    let mut piezas = Vec::new();
    let mut i = 0;
    let mut linea = 1;
    let mut dentro_de_php = false;
    let empieza = |i: usize, texto: &str| -> bool {
        texto.chars().enumerate().all(|(k, c)| caracteres.get(i + k) == Some(&c))
    };
    while i < caracteres.len() {
        if !dentro_de_php {
            // HTML fuera de <?php ... ?>: se escribe tal cual.
            let mut html = String::new();
            while i < caracteres.len() && !empieza(i, "<?php") && !empieza(i, "<?=") {
                if caracteres[i] == '\n' {
                    linea += 1;
                }
                html.push(caracteres[i]);
                i += 1;
            }
            if !html.is_empty() {
                piezas.push(PiezaConLinea { pieza: Pieza::Html(html), linea });
            }
            if empieza(i, "<?php") {
                i += 5;
            } else if empieza(i, "<?=") {
                i += 3;
                piezas.push(PiezaConLinea { pieza: Pieza::Nombre("echo".to_string()), linea });
            }
            dentro_de_php = true;
            continue;
        }
        let c = caracteres[i];
        if c == '\n' {
            linea += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if empieza(i, "?>") {
            i += 2;
            if caracteres.get(i) == Some(&'\n') {
                i += 1;
                linea += 1;
            }
            piezas.push(PiezaConLinea { pieza: Pieza::Signo(";".to_string()), linea });
            dentro_de_php = false;
            continue;
        }
        if c == '#' || empieza(i, "//") {
            while i < caracteres.len() && caracteres[i] != '\n' && !empieza(i, "?>") {
                i += 1;
            }
            continue;
        }
        if empieza(i, "/*") {
            i += 2;
            while i < caracteres.len() && !empieza(i, "*/") {
                if caracteres[i] == '\n' {
                    linea += 1;
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '$' && caracteres.get(i + 1).is_some_and(|d| d.is_alphabetic() || *d == '_') {
            i += 1;
            let mut nombre = String::new();
            while i < caracteres.len() && (caracteres[i].is_alphanumeric() || caracteres[i] == '_') {
                nombre.push(caracteres[i]);
                i += 1;
            }
            piezas.push(PiezaConLinea { pieza: Pieza::Variable(nombre), linea });
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '\\' {
            let mut nombre = String::new();
            while i < caracteres.len() && (caracteres[i].is_alphanumeric() || caracteres[i] == '_' || caracteres[i] == '\\') {
                nombre.push(caracteres[i]);
                i += 1;
            }
            let nombre = nombre.trim_start_matches('\\').to_string();
            piezas.push(PiezaConLinea { pieza: Pieza::Nombre(nombre), linea });
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && caracteres.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let mut numero = String::new();
            while i < caracteres.len() && (caracteres[i].is_ascii_digit() || caracteres[i] == '.' || caracteres[i] == '_') {
                if caracteres[i] != '_' {
                    numero.push(caracteres[i]);
                }
                i += 1;
            }
            piezas.push(PiezaConLinea { pieza: Pieza::Numero(numero), linea });
            continue;
        }
        if c == '\'' {
            i += 1;
            let mut texto = String::new();
            while i < caracteres.len() && caracteres[i] != '\'' {
                if caracteres[i] == '\\' && matches!(caracteres.get(i + 1), Some('\'') | Some('\\')) {
                    texto.push(caracteres[i + 1]);
                    i += 2;
                    continue;
                }
                if caracteres[i] == '\n' {
                    linea += 1;
                }
                texto.push(caracteres[i]);
                i += 1;
            }
            if i >= caracteres.len() {
                return Err(format!("línea {}: falta cerrar un texto con '", linea));
            }
            i += 1;
            piezas.push(PiezaConLinea { pieza: Pieza::Texto(vec![Trozo::Literal(texto)]), linea });
            continue;
        }
        if c == '"' {
            let inicio = linea;
            i += 1;
            let mut trozos = Vec::new();
            let mut actual = String::new();
            while i < caracteres.len() && caracteres[i] != '"' {
                let d = caracteres[i];
                if d == '\\' {
                    let siguiente = caracteres.get(i + 1).copied().unwrap_or('\\');
                    let escapado = match siguiente {
                        'n' => Some('\n'),
                        't' => Some('\t'),
                        'r' => Some('\r'),
                        '"' => Some('"'),
                        '\\' => Some('\\'),
                        '$' => Some('$'),
                        _ => None,
                    };
                    match escapado {
                        Some(e) => {
                            actual.push(e);
                            i += 2;
                        }
                        None => {
                            actual.push('\\');
                            i += 1;
                        }
                    }
                    continue;
                }
                let llave = d == '{' && caracteres.get(i + 1) == Some(&'$');
                if (d == '$' && caracteres.get(i + 1).is_some_and(|e| e.is_alphabetic() || *e == '_')) || llave {
                    i += if llave { 2 } else { 1 };
                    let mut nombre = String::new();
                    while i < caracteres.len() && (caracteres[i].is_alphanumeric() || caracteres[i] == '_') {
                        nombre.push(caracteres[i]);
                        i += 1;
                    }
                    let mut clave: Option<(String, bool)> = None;
                    if caracteres.get(i) == Some(&'[') {
                        // $a[0], $a[clave] o, entre llaves, $a['clave'] / $a["clave"].
                        let mut j = i + 1;
                        let comilla = caracteres.get(j).copied().filter(|c| llave && (*c == '\'' || *c == '"'));
                        if comilla.is_some() {
                            j += 1;
                        }
                        let mut texto = String::new();
                        while j < caracteres.len() && (caracteres[j].is_alphanumeric() || caracteres[j] == '_' || caracteres[j] == '-' || (comilla.is_some() && caracteres[j] == ' ')) {
                            texto.push(caracteres[j]);
                            j += 1;
                        }
                        if let Some(c) = comilla {
                            if caracteres.get(j) != Some(&c) {
                                return Err(format!("línea {}: solo se traducen textos con {{$variable}} sencillas", linea));
                            }
                            j += 1;
                        }
                        if caracteres.get(j) == Some(&']') && !texto.is_empty() {
                            let numero = comilla.is_none() && texto.chars().all(|c| c.is_ascii_digit());
                            if comilla.is_none() && !numero && llave {
                                return Err(format!("línea {}: solo se traducen textos con {{$variable}} sencillas", linea));
                            }
                            clave = Some((texto, numero));
                            i = j + 1;
                        } else if llave {
                            return Err(format!("línea {}: solo se traducen textos con {{$variable}} sencillas", linea));
                        }
                    }
                    if llave {
                        if caracteres.get(i) != Some(&'}') {
                            return Err(format!("línea {}: solo se traducen textos con {{$variable}} sencillas", linea));
                        }
                        i += 1;
                    }
                    if !actual.is_empty() {
                        trozos.push(Trozo::Literal(std::mem::take(&mut actual)));
                    }
                    trozos.push(match clave {
                        Some((clave, numero)) => Trozo::Indice(nombre, clave, numero),
                        None => Trozo::Variable(nombre),
                    });
                    continue;
                }
                if d == '\n' {
                    linea += 1;
                }
                actual.push(d);
                i += 1;
            }
            if i >= caracteres.len() {
                return Err(format!("línea {}: falta cerrar un texto con \"", inicio));
            }
            i += 1;
            if !actual.is_empty() || trozos.is_empty() {
                trozos.push(Trozo::Literal(actual));
            }
            piezas.push(PiezaConLinea { pieza: Pieza::Texto(trozos), linea: inicio });
            continue;
        }
        let tres: String = caracteres[i..(i + 3).min(caracteres.len())].iter().collect();
        let dos: String = caracteres[i..(i + 2).min(caracteres.len())].iter().collect();
        let signo = if ["===", "!==", "**="].contains(&tres.as_str()) {
            tres
        } else if ["==", "!=", "<>", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=", "/=", ".=", "%=", "=>", "->", "::", "**", "??"].contains(&dos.as_str()) {
            dos
        } else {
            c.to_string()
        };
        i += signo.chars().count();
        piezas.push(PiezaConLinea { pieza: Pieza::Signo(signo), linea });
    }
    piezas.push(PiezaConLinea { pieza: Pieza::Fin, linea });
    Ok(piezas)
}

// ---------------------------------------------------------------------------
// Árbol del PHP.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Expr {
    Numero(String),
    Texto(Vec<Trozo>),
    Logico(bool),
    Nulo,
    Variable(String),
    Lista(Vec<Expr>),
    /// Array con claves: ['a' => 1].
    Mapa(Vec<(Expr, Expr)>),
    Llamada(String, Vec<Expr>, usize),
    Indice(Box<Expr>, Box<Expr>),
    Binaria(String, Box<Expr>, Box<Expr>),
    No(Box<Expr>),
    Menos(Box<Expr>),
    SinTraducir(String),
}

#[derive(Debug, Clone)]
enum Instr {
    Escribir(Vec<Expr>),
    Html(String),
    Asignar(String, Expr),
    AsignarIndice(String, Expr, Expr),
    Agregar(String, Expr),
    Si(Vec<(Expr, Vec<Instr>)>, Option<Vec<Instr>>),
    Mientras(Expr, Vec<Instr>),
    Para(Vec<Instr>, Option<Expr>, Vec<Instr>, Vec<Instr>),
    ParaCada(Expr, String, Vec<Instr>),
    Funcion(String, Vec<String>, Vec<Instr>),
    Devolver(Option<Expr>),
    Expresion(Expr),
    /// Varias instrucciones que vienen de una sola de PHP (un switch, un unset...).
    Varias(Vec<Instr>),
    /// exit/die en una página: termina la página (con un mensaje opcional).
    Salir(Option<Expr>),
    /// header("Location: ...") en una página.
    Redirigir(Expr),
    SinTraducir(usize, String),
    Romper,
    Continuar,
    /// try { } catch (Exception $e) { }: cuerpo, nombre de la variable del error y cuerpo del fallo.
    Intentar(Vec<Instr>, String, Vec<Instr>),
}

struct Analizador {
    piezas: Vec<PiezaConLinea>,
    pos: usize,
    pendientes: Vec<(usize, String)>,
    /// Página web: se aceptan $_GET, $_POST, $_SERVER, header() y exit.
    web: bool,
    /// Bloques abiertos donde vale `break`: verdadero en un bucle, falso en un switch.
    saltos: Vec<bool>,
}

impl Analizador {
    fn actual(&self) -> &Pieza {
        &self.piezas[self.pos].pieza
    }
    fn linea(&self) -> usize {
        self.piezas[self.pos].linea
    }
    fn avanzar(&mut self) -> Pieza {
        let pieza = self.piezas[self.pos].pieza.clone();
        if self.pos + 1 < self.piezas.len() {
            self.pos += 1;
        }
        pieza
    }
    fn es_signo(&self, signo: &str) -> bool {
        matches!(self.actual(), Pieza::Signo(s) if s == signo)
    }
    fn es_nombre(&self, nombre: &str) -> bool {
        matches!(self.actual(), Pieza::Nombre(n) if n.eq_ignore_ascii_case(nombre))
    }
    fn esperar(&mut self, signo: &str) -> Result<(), String> {
        if self.es_signo(signo) {
            self.avanzar();
            Ok(())
        } else {
            Err(format!("se esperaba '{}'", signo))
        }
    }

    /// Salta hasta el final de la instrucción (';' o el bloque { } completo).
    fn saltar_instruccion(&mut self) -> String {
        let mut texto = Vec::new();
        let mut profundidad = 0;
        loop {
            match self.actual().clone() {
                Pieza::Fin => break,
                Pieza::Signo(s) if s == "{" => profundidad += 1,
                Pieza::Signo(s) if s == "}" => {
                    if profundidad == 0 {
                        break;
                    }
                    profundidad -= 1;
                    if profundidad == 0 {
                        texto.push("}".to_string());
                        self.avanzar();
                        if !self.es_nombre("else") && !self.es_nombre("elseif") && !self.es_nombre("catch") && !self.es_nombre("finally") {
                            break;
                        }
                        continue;
                    }
                }
                Pieza::Signo(s) if s == ";" && profundidad == 0 => {
                    self.avanzar();
                    break;
                }
                _ => {}
            }
            texto.push(describir(&self.avanzar()));
        }
        let mut resumen = texto.join(" ");
        if resumen.chars().count() > 70 {
            resumen = resumen.chars().take(70).collect::<String>() + "…";
        }
        resumen
    }

    /// Cuerpo de un bucle o de un switch: dentro, `break` sale de él.
    fn cuerpo_con_salto(&mut self, es_bucle: bool) -> Vec<Instr> {
        self.saltos.push(es_bucle);
        let cuerpo = self.bloque();
        self.saltos.pop();
        cuerpo
    }

    fn bloque(&mut self) -> Vec<Instr> {
        if self.es_signo("{") {
            self.avanzar();
            let mut instrucciones = Vec::new();
            while !self.es_signo("}") && *self.actual() != Pieza::Fin {
                instrucciones.push(self.instruccion());
            }
            self.avanzar();
            instrucciones
        } else {
            vec![self.instruccion()]
        }
    }

    fn instruccion(&mut self) -> Instr {
        let linea = self.linea();
        let inicio = self.pos;
        match self.intentar_instruccion() {
            Ok(instruccion) => instruccion,
            Err(motivo) => {
                self.pos = inicio;
                let texto = self.saltar_instruccion();
                self.pendientes.push((linea, format!("{} ({})", motivo, texto)));
                Instr::SinTraducir(linea, texto)
            }
        }
    }

    fn intentar_instruccion(&mut self) -> Result<Instr, String> {
        if let Pieza::Html(html) = self.actual().clone() {
            self.avanzar();
            return Ok(Instr::Html(html));
        }
        if self.es_signo(";") {
            self.avanzar();
            return Ok(Instr::Expresion(Expr::Nulo));
        }
        if self.es_signo("{") {
            return Err("bloque suelto sin instrucción".to_string());
        }
        if self.es_nombre("echo") || self.es_nombre("print") {
            self.avanzar();
            let mut partes = vec![self.expresion()?];
            while self.es_signo(",") {
                self.avanzar();
                partes.push(self.expresion()?);
            }
            self.fin_instruccion()?;
            return Ok(Instr::Escribir(partes));
        }
        if self.es_nombre("if") {
            self.avanzar();
            let mut ramas = Vec::new();
            self.esperar("(")?;
            let condicion = self.expresion()?;
            self.esperar(")")?;
            ramas.push((condicion, self.bloque()));
            let mut otro = None;
            loop {
                if self.es_nombre("elseif") || (self.es_nombre("else") && matches!(self.piezas.get(self.pos + 1).map(|p| &p.pieza), Some(Pieza::Nombre(n)) if n.eq_ignore_ascii_case("if"))) {
                    if self.es_nombre("else") {
                        self.avanzar();
                    }
                    self.avanzar();
                    self.esperar("(")?;
                    let condicion = self.expresion()?;
                    self.esperar(")")?;
                    ramas.push((condicion, self.bloque()));
                } else if self.es_nombre("else") {
                    self.avanzar();
                    otro = Some(self.bloque());
                    break;
                } else {
                    break;
                }
            }
            return Ok(Instr::Si(ramas, otro));
        }
        if self.es_nombre("while") {
            self.avanzar();
            self.esperar("(")?;
            let condicion = self.expresion()?;
            self.esperar(")")?;
            return Ok(Instr::Mientras(condicion, self.cuerpo_con_salto(true)));
        }
        if self.es_nombre("for") {
            self.avanzar();
            self.esperar("(")?;
            let mut inicio = Vec::new();
            while !self.es_signo(";") {
                inicio.push(self.asignacion_simple()?);
                if self.es_signo(",") {
                    self.avanzar();
                }
            }
            self.avanzar();
            let condicion = if self.es_signo(";") { None } else { Some(self.expresion()?) };
            self.esperar(";")?;
            let mut paso = Vec::new();
            while !self.es_signo(")") {
                paso.push(self.asignacion_simple()?);
                if self.es_signo(",") {
                    self.avanzar();
                }
            }
            self.avanzar();
            return Ok(Instr::Para(inicio, condicion, paso, self.cuerpo_con_salto(true)));
        }
        if self.es_nombre("foreach") {
            self.avanzar();
            self.esperar("(")?;
            let lista = self.expresion()?;
            if !self.es_nombre("as") {
                return Err("foreach sin 'as'".to_string());
            }
            self.avanzar();
            let variable = match self.avanzar() {
                Pieza::Variable(v) => v,
                _ => return Err("foreach con una forma que no se traduce".to_string()),
            };
            if self.es_signo("=>") {
                // foreach ($m as $k => $v): se recorren las claves y se lee cada valor.
                self.avanzar();
                let valor = match self.avanzar() {
                    Pieza::Variable(v) => v,
                    _ => return Err("foreach con una forma que no se traduce".to_string()),
                };
                self.esperar(")")?;
                let mut previas = Vec::new();
                let lista = match lista {
                    Expr::Variable(_) => lista,
                    otra => {
                        let temporal = format!("recorrido_{}", self.pos);
                        previas.push(Instr::Asignar(temporal.clone(), otra));
                        Expr::Variable(temporal)
                    }
                };
                let mut cuerpo = vec![Instr::Asignar(valor, Expr::Indice(Box::new(lista.clone()), Box::new(Expr::Variable(variable.clone()))))];
                cuerpo.extend(self.cuerpo_con_salto(true));
                let linea = self.linea();
                previas.push(Instr::ParaCada(Expr::Llamada("__claves".to_string(), vec![lista], linea), variable, cuerpo));
                return Ok(if previas.len() == 1 { previas.pop().expect("una") } else { Instr::Varias(previas) });
            }
            self.esperar(")")?;
            return Ok(Instr::ParaCada(lista, variable, self.cuerpo_con_salto(true)));
        }
        if self.es_nombre("function") {
            self.avanzar();
            let nombre = match self.avanzar() {
                Pieza::Nombre(n) => n,
                _ => return Err("función sin nombre (anónima)".to_string()),
            };
            self.esperar("(")?;
            let mut parametros = Vec::new();
            while !self.es_signo(")") {
                match self.avanzar() {
                    Pieza::Variable(v) => parametros.push(v),
                    Pieza::Nombre(_) => continue, // tipos: int, string...
                    Pieza::Signo(s) if s == "," || s == "?" => continue,
                    Pieza::Signo(s) if s == "=" => return Err("parámetros con valor por defecto".to_string()),
                    Pieza::Signo(s) if s == "&" => return Err("parámetros por referencia (&)".to_string()),
                    _ => return Err("parámetros de una forma que no se traduce".to_string()),
                }
            }
            self.avanzar();
            if self.es_signo(":") {
                self.avanzar();
                while !self.es_signo("{") && *self.actual() != Pieza::Fin {
                    self.avanzar();
                }
            }
            let saltos = std::mem::take(&mut self.saltos);
            let cuerpo = self.bloque();
            self.saltos = saltos;
            return Ok(Instr::Funcion(nombre, parametros, cuerpo));
        }
        if self.es_nombre("exit") || self.es_nombre("die") {
            if !self.web {
                return Err("exit/die (Deimon termina el programa al acabar)".to_string());
            }
            self.avanzar();
            let mut mensaje = None;
            if self.es_signo("(") {
                self.avanzar();
                if !self.es_signo(")") {
                    mensaje = Some(self.expresion()?);
                }
                self.esperar(")")?;
            }
            self.fin_instruccion()?;
            return Ok(Instr::Salir(mensaje));
        }
        if self.es_nombre("switch") {
            return self.switch();
        }
        if self.es_nombre("unset") {
            self.avanzar();
            self.esperar("(")?;
            let argumentos = self.elementos(")")?;
            self.fin_instruccion()?;
            let mut borrar = Vec::new();
            for argumento in argumentos {
                match argumento {
                    Expr::Llamada(n, a, l) if n == "__sesion" => borrar.push(Instr::Expresion(Expr::Llamada("__sesion_borrar".to_string(), a, l))),
                    Expr::Indice(base, clave) => match *base {
                        Expr::Variable(v) => {
                            let linea = self.linea();
                            let quitar = Expr::Llamada("__quitar_clave".to_string(), vec![Expr::Variable(v.clone()), *clave], linea);
                            borrar.push(Instr::Asignar(v, quitar));
                        }
                        _ => return Err("unset() de algo que no es $variable[clave]".to_string()),
                    },
                    _ => return Err("unset() de algo que no es $variable[clave] ni $_SESSION[...]".to_string()),
                }
            }
            return Ok(Instr::Varias(borrar));
        }
        if self.es_nombre("session_start") {
            self.saltar_instruccion();
            return Ok(Instr::Expresion(Expr::Nulo));
        }
        if self.es_nombre("return") {
            self.avanzar();
            if self.es_signo(";") {
                self.avanzar();
                return Ok(Instr::Devolver(None));
            }
            let valor = self.expresion()?;
            self.fin_instruccion()?;
            return Ok(Instr::Devolver(Some(valor)));
        }
        if self.es_nombre("break") || self.es_nombre("continue") {
            let romper = self.es_nombre("break");
            self.avanzar();
            if !self.es_signo(";") {
                return Err("break o continue con número (sal de un solo bucle cada vez)".to_string());
            }
            self.avanzar();
            return match self.saltos.last() {
                Some(true) => Ok(if romper { Instr::Romper } else { Instr::Continuar }),
                Some(false) => Err("break o continue dentro de un if de un switch (pon el break al final del case)".to_string()),
                None => Err("break o continue fuera de un bucle".to_string()),
            };
        }
        if self.es_nombre("try") {
            let pendientes = self.pendientes.len();
            let resultado = self.intentar_try();
            if resultado.is_err() {
                self.pendientes.truncate(pendientes);
            }
            return resultado;
        }
        if self.es_nombre("throw") {
            self.avanzar();
            let nuevo = matches!(self.actual(), Pieza::Nombre(n) if n.eq_ignore_ascii_case("new"));
            if !nuevo {
                return Err("throw de algo que no es 'new Exception(mensaje)'".to_string());
            }
            self.avanzar();
            let tipo_valido = matches!(self.actual(), Pieza::Nombre(n) if n.eq_ignore_ascii_case("Exception"));
            if !tipo_valido {
                return Err("throw de un tipo de excepción que no es Exception".to_string());
            }
            self.avanzar();
            self.esperar("(")?;
            let mensaje = if self.es_signo(")") { Expr::Texto(Vec::new()) } else { self.expresion()? };
            if self.es_signo(",") {
                return Err("throw new Exception con más de un argumento".to_string());
            }
            self.esperar(")")?;
            self.fin_instruccion()?;
            let linea = self.linea();
            return Ok(Instr::Expresion(Expr::Llamada("__provocar_error".to_string(), vec![mensaje], linea)));
        }
        for (palabra, motivo) in [
            ("class", "clases (Deimon no tiene clases)"),
            ("include", "include (usa 'usar' con el archivo traducido)"),
            ("require", "require (usa 'usar' con el archivo traducido)"),
            ("include_once", "include_once (usa 'usar' con el archivo traducido)"),
            ("require_once", "require_once (usa 'usar' con el archivo traducido)"),
            ("global", "variables globales dentro de funciones"),
            ("static", "variables static"),
            ("do", "do ... while"),
            ("namespace", "namespace"),
            ("use", "use"),
            ("declare", "declare"),
        ] {
            if self.es_nombre(palabra) {
                return Err(motivo.to_string());
            }
        }
        if let Pieza::Variable(_) = self.actual() {
            if matches!(self.piezas.get(self.pos + 1).map(|p| &p.pieza), Some(Pieza::Signo(s)) if ["=", "+=", "-=", "*=", "/=", ".=", "%=", "++", "--", "["].contains(&s.as_str())) {
                let instruccion = self.asignacion_simple()?;
                self.fin_instruccion()?;
                return Ok(instruccion);
            }
        }
        let expresion = self.expresion()?;
        self.fin_instruccion()?;
        if let Expr::Llamada(nombre, argumentos, _) = &expresion {
            if self.web && nombre.eq_ignore_ascii_case("header") && argumentos.len() == 1 {
                if let Expr::Texto(trozos) = &argumentos[0] {
                    if let Some(Trozo::Literal(primero)) = trozos.first() {
                        if let Some(resto) = primero.strip_prefix("Location:") {
                            let mut destino = trozos.clone();
                            let mut camino = resto.trim_start().to_string();
                            if !camino.starts_with('/') && !camino.starts_with("http") {
                                // "gracias.php" es relativo a la página: en el sitio es "/gracias.php".
                                camino.insert(0, '/');
                            }
                            destino[0] = Trozo::Literal(camino);
                            return Ok(Instr::Redirigir(Expr::Texto(destino)));
                        }
                    }
                }
                return Err("header() que no es una redirección (Location:)".to_string());
            }
            if nombre.eq_ignore_ascii_case("array_push") && argumentos.len() == 2 {
                if let Expr::Variable(variable) = &argumentos[0] {
                    return Ok(Instr::Agregar(variable.clone(), argumentos[1].clone()));
                }
            }
        }
        Ok(Instr::Expresion(expresion))
    }

    /// switch: se traduce a si / sino si / sino. Los case seguidos sin código se juntan
    /// con 'o'; cada caso con código debe terminar en break, return o exit.
    fn switch(&mut self) -> Result<Instr, String> {
        self.avanzar();
        self.esperar("(")?;
        let sujeto = self.expresion()?;
        self.esperar(")")?;
        self.esperar("{")?;
        // (etiquetas: None = default, cuerpo, termina)
        let mut grupos: Vec<(Vec<Option<Expr>>, Vec<Instr>, bool)> = Vec::new();
        let mut etiquetas: Vec<Option<Expr>> = Vec::new();
        loop {
            if self.es_signo("}") {
                self.avanzar();
                break;
            }
            if *self.actual() == Pieza::Fin {
                return Err("switch sin cerrar".to_string());
            }
            if self.es_nombre("case") {
                self.avanzar();
                etiquetas.push(Some(self.expresion()?));
            } else if self.es_nombre("default") {
                self.avanzar();
                etiquetas.push(None);
            } else {
                return Err("switch con algo fuera de un case".to_string());
            }
            if self.es_signo(":") || self.es_signo(";") {
                self.avanzar();
            } else {
                return Err("se esperaba ':' después del case".to_string());
            }
            if self.es_nombre("case") || self.es_nombre("default") {
                continue; // varios case seguidos para el mismo código
            }
            let mut cuerpo = Vec::new();
            let mut termina = false;
            while !self.es_nombre("case") && !self.es_nombre("default") && !self.es_signo("}") && *self.actual() != Pieza::Fin {
                if termina {
                    self.saltar_instruccion(); // código tras el break: nunca se ejecuta
                    continue;
                }
                if self.es_nombre("break") {
                    self.avanzar();
                    if self.es_signo(";") {
                        self.avanzar();
                    }
                    termina = true;
                    continue;
                }
                self.saltos.push(false);
                let instruccion = self.instruccion();
                self.saltos.pop();
                termina = matches!(instruccion, Instr::Devolver(_) | Instr::Salir(_) | Instr::Redirigir(_));
                cuerpo.push(instruccion);
            }
            grupos.push((std::mem::take(&mut etiquetas), cuerpo, termina));
        }
        if !etiquetas.is_empty() {
            grupos.push((etiquetas, Vec::new(), true));
        }
        let ultimo = grupos.len().saturating_sub(1);
        if grupos.iter().enumerate().any(|(i, (_, _, termina))| !termina && i != ultimo) {
            return Err("switch con un case que sigue en el siguiente (sin break)".to_string());
        }
        let mut previas = Vec::new();
        let sujeto = match sujeto {
            Expr::Variable(_) => sujeto,
            otro => {
                let temporal = format!("eleccion_{}", self.pos);
                previas.push(Instr::Asignar(temporal.clone(), otro));
                Expr::Variable(temporal)
            }
        };
        let mut ramas = Vec::new();
        let mut otro = None;
        for (etiquetas, cuerpo, _) in grupos {
            if etiquetas.iter().any(Option::is_none) {
                otro = Some(cuerpo);
                continue;
            }
            let condicion = etiquetas
                .into_iter()
                .flatten()
                .map(|valor| Expr::Binaria("==".to_string(), Box::new(sujeto.clone()), Box::new(valor)))
                .reduce(|a, b| Expr::Binaria("||".to_string(), Box::new(a), Box::new(b)))
                .expect("al menos una etiqueta");
            ramas.push((condicion, cuerpo));
        }
        let instruccion = if ramas.is_empty() {
            Instr::Varias(otro.unwrap_or_default())
        } else {
            Instr::Si(ramas, otro)
        };
        previas.push(instruccion);
        Ok(if previas.len() == 1 { previas.pop().expect("una") } else { Instr::Varias(previas) })
    }

    /// try { } catch (Exception $e) { } -> intentar { } si falla e { }.
    fn intentar_try(&mut self) -> Result<Instr, String> {
        self.avanzar();
        if !self.es_signo("{") {
            return Err("try sin llaves".to_string());
        }
        let cuerpo = self.bloque();
        if !self.es_nombre("catch") {
            return Err("try sin catch (finally no se traduce)".to_string());
        }
        self.avanzar();
        self.esperar("(")?;
        let tipo_valido = matches!(self.actual(), Pieza::Nombre(n) if n.eq_ignore_ascii_case("Exception") || n.eq_ignore_ascii_case("Throwable"));
        if !tipo_valido {
            return Err("catch de un tipo que no es Exception ni Throwable".to_string());
        }
        self.avanzar();
        let variable = match self.avanzar() {
            Pieza::Variable(v) => v,
            _ => return Err("catch con varios tipos o sin variable".to_string()),
        };
        self.esperar(")")?;
        if !self.es_signo("{") {
            return Err("catch sin llaves".to_string());
        }
        let fallo = self.bloque();
        if self.es_nombre("catch") || self.es_nombre("finally") {
            return Err("varios catch o finally".to_string());
        }
        Ok(Instr::Intentar(cuerpo, variable, fallo))
    }

    fn fin_instruccion(&mut self) -> Result<(), String> {
        if self.es_signo(";") {
            self.avanzar();
            Ok(())
        } else if matches!(self.actual(), Pieza::Html(_) | Pieza::Fin) {
            Ok(())
        } else {
            Err("falta ';' o hay algo que no se traduce".to_string())
        }
    }

    /// $x = e, $x op= e, $x++, $x--, $x[] = e, $x[i] = e (sin el ';').
    fn asignacion_simple(&mut self) -> Result<Instr, String> {
        let variable = match self.avanzar() {
            Pieza::Variable(v) => v,
            _ => return Err("se esperaba una variable".to_string()),
        };
        let signo = match self.avanzar() {
            Pieza::Signo(s) => s,
            _ => return Err("asignación que no se traduce".to_string()),
        };
        let leer = Expr::Variable(variable.clone());
        let operacion = |op: &str, valor: Expr| Expr::Binaria(op.to_string(), Box::new(leer.clone()), Box::new(valor));
        Ok(match signo.as_str() {
            "=" => {
                if self.es_signo("&") {
                    return Err("asignación por referencia (&)".to_string());
                }
                Instr::Asignar(variable, self.expresion()?)
            }
            "++" => Instr::Asignar(variable, operacion("+", Expr::Numero("1".into()))),
            "--" => Instr::Asignar(variable, operacion("-", Expr::Numero("1".into()))),
            "+=" | "-=" | "*=" | "/=" | "%=" | ".=" => {
                let op = &signo[..1];
                let valor = self.expresion()?;
                Instr::Asignar(variable, operacion(op, valor))
            }
            "[" => {
                if self.es_signo("]") {
                    self.avanzar();
                    self.esperar("=")?;
                    Instr::Agregar(variable, self.expresion()?)
                } else {
                    let indice = self.expresion()?;
                    self.esperar("]")?;
                    if variable == "_SESSION" && self.es_signo("[") {
                        self.avanzar();
                        self.esperar("]")?;
                        self.esperar("=")?;
                        let valor = self.expresion()?;
                        let linea = self.linea();
                        return Ok(Instr::Expresion(Expr::Llamada("__sesion_agregar".to_string(), vec![indice, valor], linea)));
                    }
                    if !self.es_signo("=") {
                        return Err("solo se traduce $lista[i] = valor".to_string());
                    }
                    self.avanzar();
                    Instr::AsignarIndice(variable, indice, self.expresion()?)
                }
            }
            _ => return Err(format!("operador '{}' en una asignación", signo)),
        })
    }

    fn expresion(&mut self) -> Result<Expr, String> {
        let linea = self.linea();
        let izquierda = self.o_logico()?;
        if self.es_signo("??") {
            // $a ?? $b: $a si existe y no es null; si no, $b.
            self.avanzar();
            let otro = self.expresion()?;
            let existe = Expr::Binaria("!=".to_string(), Box::new(izquierda.clone()), Box::new(Expr::Nulo));
            return Ok(Expr::Llamada("__elegir".to_string(), vec![existe, izquierda, otro], linea));
        }
        if self.es_signo("?") {
            self.avanzar();
            let si_verdad = if self.es_signo(":") { izquierda.clone() } else { self.expresion()? };
            self.esperar(":")?;
            let si_falso = self.expresion()?;
            return Ok(Expr::Llamada("__elegir".to_string(), vec![izquierda, si_verdad, si_falso], linea));
        }
        Ok(izquierda)
    }

    fn binaria(&mut self, siguiente: fn(&mut Self) -> Result<Expr, String>, signos: &[&str], nombres: &[&str]) -> Result<Expr, String> {
        let mut izquierda = siguiente(self)?;
        loop {
            let operador = match self.actual() {
                Pieza::Signo(s) if signos.contains(&s.as_str()) => s.clone(),
                Pieza::Nombre(n) if nombres.iter().any(|m| m.eq_ignore_ascii_case(n)) => n.to_lowercase(),
                _ => break,
            };
            self.avanzar();
            let derecha = siguiente(self)?;
            izquierda = Expr::Binaria(operador, Box::new(izquierda), Box::new(derecha));
        }
        Ok(izquierda)
    }

    fn o_logico(&mut self) -> Result<Expr, String> {
        self.binaria(Self::y_logico, &["||"], &["or"])
    }
    fn y_logico(&mut self) -> Result<Expr, String> {
        self.binaria(Self::comparacion, &["&&"], &["and"])
    }
    fn comparacion(&mut self) -> Result<Expr, String> {
        self.binaria(Self::concatenacion, &["==", "!=", "===", "!==", "<>", "<", "<=", ">", ">="], &[])
    }
    fn concatenacion(&mut self) -> Result<Expr, String> {
        self.binaria(Self::suma, &["."], &[])
    }
    fn suma(&mut self) -> Result<Expr, String> {
        self.binaria(Self::producto, &["+", "-"], &[])
    }
    fn producto(&mut self) -> Result<Expr, String> {
        self.binaria(Self::unaria, &["*", "/", "%"], &[])
    }
    fn unaria(&mut self) -> Result<Expr, String> {
        if self.es_signo("!") {
            self.avanzar();
            return Ok(Expr::No(Box::new(self.unaria()?)));
        }
        if self.es_signo("-") {
            self.avanzar();
            return Ok(Expr::Menos(Box::new(self.unaria()?)));
        }
        if self.es_signo("+") {
            self.avanzar();
            return self.unaria();
        }
        if self.es_signo("(") && matches!(self.piezas.get(self.pos + 1).map(|p| &p.pieza), Some(Pieza::Nombre(n)) if ["int", "float", "string", "bool"].contains(&n.to_lowercase().as_str())) && matches!(self.piezas.get(self.pos + 2).map(|p| &p.pieza), Some(Pieza::Signo(s)) if s == ")") {
            self.avanzar();
            let tipo = match self.avanzar() {
                Pieza::Nombre(n) => n.to_lowercase(),
                _ => unreachable!(),
            };
            self.avanzar();
            let valor = self.unaria()?;
            let linea = self.linea();
            let funcion = match tipo.as_str() {
                "int" => "intval",
                "float" => "floatval",
                "string" => "strval",
                _ => return Err("conversión (bool)".to_string()),
            };
            return Ok(Expr::Llamada(funcion.to_string(), vec![valor], linea));
        }
        self.posfija()
    }
    fn posfija(&mut self) -> Result<Expr, String> {
        let mut base = self.primaria()?;
        loop {
            if self.es_signo("[") {
                self.avanzar();
                let indice = self.expresion()?;
                self.esperar("]")?;
                base = Expr::Indice(Box::new(base), Box::new(indice));
            } else if self.es_signo("->")
                && matches!(self.piezas.get(self.pos + 1).map(|p| &p.pieza), Some(Pieza::Nombre(n)) if n.eq_ignore_ascii_case("getMessage"))
                && matches!(self.piezas.get(self.pos + 2).map(|p| &p.pieza), Some(Pieza::Signo(s)) if s == "(")
                && matches!(self.piezas.get(self.pos + 3).map(|p| &p.pieza), Some(Pieza::Signo(s)) if s == ")")
            {
                self.avanzar();
                self.avanzar();
                self.avanzar();
                self.avanzar();
                base = Expr::Indice(Box::new(base), Box::new(Expr::Texto(vec![Trozo::Literal("mensaje".to_string())])));
            } else if self.es_signo("->") || self.es_signo("::") {
                return Err("objetos y clases (-> o ::)".to_string());
            } else if self.es_signo("++") || self.es_signo("--") {
                return Err("++ o -- dentro de una expresión".to_string());
            } else {
                break;
            }
        }
        Ok(base)
    }
    fn primaria(&mut self) -> Result<Expr, String> {
        let linea = self.linea();
        match self.avanzar() {
            Pieza::Numero(n) => Ok(Expr::Numero(n)),
            Pieza::Texto(trozos) => Ok(Expr::Texto(trozos)),
            Pieza::Variable(v) => {
                if v.starts_with('_') && v.chars().skip(1).all(|c| c.is_ascii_uppercase()) {
                    if !self.web {
                        return Err(format!("${} (es una página web: tradúcela como página)", v));
                    }
                    let campo = matches!(v.as_str(), "_GET" | "_POST" | "_REQUEST");
                    if v == "_SESSION" {
                        self.esperar("[")?;
                        let indice = self.expresion()?;
                        self.esperar("]")?;
                        return Ok(Expr::Llamada("__sesion".to_string(), vec![indice], linea));
                    }
                    if v == "_COOKIE" {
                        self.esperar("[")?;
                        let indice = self.expresion()?;
                        self.esperar("]")?;
                        return Ok(Expr::Llamada("__galleta".to_string(), vec![indice], linea));
                    }
                    if !(campo || v == "_SERVER") {
                        return Err(format!("${} (los archivos subidos aún no se traducen: usa archivo_subido y guardar_subida)", v));
                    }
                    self.esperar("[")?;
                    let indice = self.expresion()?;
                    self.esperar("]")?;
                    if campo {
                        return Ok(Expr::Llamada("__campo".to_string(), vec![indice], linea));
                    }
                    return match indice {
                        Expr::Texto(t) if t == vec![Trozo::Literal("REQUEST_METHOD".to_string())] => {
                            Ok(Expr::Llamada("__metodo".to_string(), Vec::new(), linea))
                        }
                        _ => Err("$_SERVER (solo se traduce $_SERVER['REQUEST_METHOD'])".to_string()),
                    };
                }
                Ok(Expr::Variable(v))
            }
            Pieza::Signo(s) if s == "(" => {
                let interior = self.expresion()?;
                self.esperar(")")?;
                Ok(interior)
            }
            Pieza::Signo(s) if s == "[" => self.arreglo("]"),
            Pieza::Nombre(n) => {
                let minuscula = n.to_lowercase();
                match minuscula.as_str() {
                    "true" => return Ok(Expr::Logico(true)),
                    "false" => return Ok(Expr::Logico(false)),
                    "null" => return Ok(Expr::Nulo),
                    "array" if self.es_signo("(") => {
                        self.avanzar();
                        return self.arreglo(")");
                    }
                    "isset" | "empty" if self.es_signo("(") => {
                        self.avanzar();
                        let argumentos = self.elementos(")")?;
                        if argumentos.is_empty() {
                            return Err(format!("'{}' sin argumentos", n));
                        }
                        let mut resultado: Option<Expr> = None;
                        for argumento in argumentos {
                            let prueba = if minuscula == "isset" {
                                Expr::Binaria("!=".to_string(), Box::new(argumento), Box::new(Expr::Nulo))
                            } else {
                                Expr::Llamada("__vacio".to_string(), vec![argumento], linea)
                            };
                            resultado = Some(match resultado {
                                None => prueba,
                                Some(previo) => Expr::Binaria("&&".to_string(), Box::new(previo), Box::new(prueba)),
                            });
                        }
                        return Ok(resultado.expect("al menos un argumento"));
                    }
                    "new" | "fn" | "function" | "match" | "isset" | "empty" | "unset" | "list" => {
                        return Err(format!("'{}'", n));
                    }
                    _ => {}
                }
                if self.es_signo("(") {
                    self.avanzar();
                    let argumentos = self.elementos(")")?;
                    return Ok(Expr::Llamada(n, argumentos, linea));
                }
                if n.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    return Ok(match n.as_str() {
                        "PHP_EOL" => Expr::Texto(vec![Trozo::Literal("\n".to_string())]),
                        "M_PI" => Expr::Numero("3.141592653589793".to_string()),
                        _ => Expr::SinTraducir(n),
                    });
                }
                Err(format!("'{}'", n))
            }
            otra => Err(format!("{} en una expresión", describir(&otra))),
        }
    }
    /// Un array de PHP: lista si no lleva claves, lista con claves si todas las llevan.
    fn arreglo(&mut self, cierre: &str) -> Result<Expr, String> {
        let mut elementos = Vec::new();
        let mut pares = Vec::new();
        while !self.es_signo(cierre) {
            let primero = self.expresion()?;
            if self.es_signo("=>") {
                self.avanzar();
                pares.push((primero, self.expresion()?));
            } else {
                elementos.push(primero);
            }
            if !elementos.is_empty() && !pares.is_empty() {
                return Err("array que mezcla valores con clave y sin clave".to_string());
            }
            if self.es_signo(",") {
                self.avanzar();
            } else if !self.es_signo(cierre) {
                return Err(format!("se esperaba ',' o '{}'", cierre));
            }
        }
        self.avanzar();
        Ok(if pares.is_empty() { Expr::Lista(elementos) } else { Expr::Mapa(pares) })
    }

    fn elementos(&mut self, cierre: &str) -> Result<Vec<Expr>, String> {
        let mut elementos = Vec::new();
        while !self.es_signo(cierre) {
            elementos.push(self.expresion()?);
            if self.es_signo("=>") {
                return Err("'=>' fuera de un array".to_string());
            }
            if self.es_signo(",") {
                self.avanzar();
            } else if !self.es_signo(cierre) {
                return Err(format!("se esperaba ',' o '{}'", cierre));
            }
        }
        self.avanzar();
        Ok(elementos)
    }
}

fn describir(pieza: &Pieza) -> String {
    match pieza {
        Pieza::Variable(v) => format!("${}", v),
        Pieza::Nombre(n) => n.clone(),
        Pieza::Numero(n) => n.clone(),
        Pieza::Texto(_) => "\"…\"".to_string(),
        Pieza::Signo(s) => s.clone(),
        Pieza::Html(_) => "HTML".to_string(),
        Pieza::Fin => "fin del archivo".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Escritura en Deimon.
// ---------------------------------------------------------------------------

/// (función de PHP, función de Deimon, módulo, orden de los argumentos en PHP).
const EQUIVALENCIAS: &[(&str, &str, Option<&str>, &[usize])] = &[
    ("strlen", "medir", None, &[0]),
    ("mb_strlen", "medir", None, &[0]),
    ("count", "medir", None, &[0]),
    ("strval", "a_texto", None, &[0]),
    ("floatval", "a_numero", None, &[0]),
    ("strtoupper", "mayusculas", Some("texto"), &[0]),
    ("mb_strtoupper", "mayusculas", Some("texto"), &[0]),
    ("strtolower", "minusculas", Some("texto"), &[0]),
    ("mb_strtolower", "minusculas", Some("texto"), &[0]),
    ("trim", "recortar", Some("texto"), &[0]),
    ("str_contains", "contiene", Some("texto"), &[0, 1]),
    ("str_starts_with", "empieza_con", Some("texto"), &[0, 1]),
    ("str_ends_with", "termina_con", Some("texto"), &[0, 1]),
    ("str_replace", "reemplazar", Some("texto"), &[2, 0, 1]),
    ("explode", "partir", Some("texto"), &[1, 0]),
    ("implode", "unir", Some("texto"), &[1, 0]),
    ("str_repeat", "repetir", Some("texto"), &[0, 1]),
    ("ucfirst", "primera_mayuscula", Some("texto"), &[0]),
    ("print_r", "texto_completo", Some("texto"), &[0]),
    ("var_dump", "texto_completo", Some("texto"), &[0]),
    ("in_array", "incluye", None, &[1, 0]),
    ("array_reverse", "invertir", None, &[0]),
    ("strrev", "invertir", None, &[0]),
    ("array_sum", "sumar_lista", None, &[0]),
    ("range", "rango", None, &[0, 1]),
    ("rand", "entero_al_azar", Some("matematicas"), &[0, 1]),
    ("mt_rand", "entero_al_azar", Some("matematicas"), &[0, 1]),
    ("random_int", "entero_al_azar", Some("matematicas"), &[0, 1]),
    ("json_encode", "a_json", Some("json"), &[0]),
    ("sqrt", "raiz", Some("matematicas"), &[0]),
    ("pow", "potencia", Some("matematicas"), &[0, 1]),
    ("abs", "absoluto", Some("matematicas"), &[0]),
    ("round", "redondear", Some("matematicas"), &[0]),
    ("floor", "suelo", Some("matematicas"), &[0]),
    ("ceil", "techo", Some("matematicas"), &[0]),
    ("min", "minimo", Some("matematicas"), &[0, 1]),
    ("max", "maximo", Some("matematicas"), &[0, 1]),
];

struct Escritor {
    salida: Vec<String>,
    nivel: usize,
    modulos: BTreeSet<&'static str>,
    pendientes: Vec<(usize, String)>,
    avisos: Vec<(usize, String)>,
    funciones: HashSet<String>,
    /// Variables del ámbito actual que se declaran con `sea` en su primera asignación
    /// (las que se asignan por primera vez fuera de cualquier si/bucle).
    por_declarar: HashSet<String>,
    nivel_ambito: usize,
    /// Página web: echo siempre va a la página y exit/header terminan la ruta.
    web: bool,
    /// Variables que guardan un campo() (ya protegido como HTML).
    desde_campo: HashSet<String>,
    /// Paso de cada bucle abierto: un `for` de PHP hace su paso antes de `continue`.
    pasos: Vec<Vec<Instr>>,
    /// Zona horaria de `date_default_timezone_set` (o "UTC" si el PHP no la cambia).
    zona: String,
}

const PRECEDENCIA_O: u8 = 1;
const PRECEDENCIA_Y: u8 = 2;
const PRECEDENCIA_NO: u8 = 3;
const PRECEDENCIA_COMPARACION: u8 = 4;
const PRECEDENCIA_SUMA: u8 = 5;
const PRECEDENCIA_PRODUCTO: u8 = 6;
const PRECEDENCIA_UNARIA: u8 = 7;
const PRECEDENCIA_ATOMO: u8 = 9;


/// Zona horaria que fija `date_default_timezone_set('...')` en el PHP (o "UTC").
fn zona_de_php(php: &str) -> String {
    let minusculas = php.to_ascii_lowercase();
    let buscado = "date_default_timezone_set";
    let mut desde = 0;
    while let Some(posicion) = minusculas[desde..].find(buscado) {
        let mut i = desde + posicion + buscado.len();
        while i < php.len() && php.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if php.as_bytes().get(i) != Some(&b'(') {
            desde = i;
            continue;
        }
        i += 1;
        while i < php.len() && php.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        let comilla = match php.as_bytes().get(i) {
            Some(b'\'') => '\'',
            Some(b'"') => '"',
            _ => {
                desde = i;
                continue;
            }
        };
        i += 1;
        let inicio = i;
        while i < php.len() && php.as_bytes()[i] != comilla as u8 {
            i += 1;
        }
        if i > php.len() {
            break;
        }
        let valor = php[inicio..i].trim();
        if !valor.is_empty() {
            return valor.to_string();
        }
        desde = i + 1;
    }
    "UTC".to_string()
}

fn texto_literal_php(expresion: &Expr) -> Option<String> {
    match expresion {
        Expr::Texto(trozos) if trozos.len() == 1 => match &trozos[0] {
            Trozo::Literal(texto) => Some(texto.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Dice si el texto parece una de las fechas absolutas que acepta `leer_fecha`.
fn parece_fecha_absoluta_php(texto: &str) -> bool {
    let texto = texto.trim();
    let bytes = texto.as_bytes();
    if bytes.len() >= 10
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
    {
        return true;
    }
    let partes: Vec<&str> = texto.split('/').collect();
    partes.len() == 3
        && partes
            .iter()
            .all(|parte| !parte.is_empty() && parte.bytes().all(|b| b.is_ascii_digit()))
}

/// Lee "+N day(s)", "-N week(s)", "+N month(s)" o "+N year(s)" de `strtotime`.
fn relativo_php(texto: &str) -> Option<(i64, &'static str)> {
    let texto = texto.trim().to_ascii_lowercase();
    if texto == "now" {
        return Some((0, "segundos"));
    }
    let (signo, resto) = if let Some(resto) = texto.strip_prefix('+') {
        (1, resto)
    } else if let Some(resto) = texto.strip_prefix('-') {
        (-1, resto)
    } else {
        (1, texto.as_str())
    };
    let partes: Vec<&str> = resto.split_whitespace().collect();
    if partes.len() != 2 {
        return None;
    }
    let cantidad: i64 = partes[0].parse().ok()?;
    let unidad = match partes[1].trim_end_matches('s') {
        "second" => "segundos",
        "minute" => "minutos",
        "hour" => "horas",
        "day" => "dias",
        "week" => "semanas",
        "month" => "meses",
        "year" => "años",
        _ => return None,
    };
    Some((signo * cantidad, unidad))
}

fn nombre_libre(nombre: &str) -> String {
    let limpio: String = nombre.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let ocupado = lexer::PALABRAS_RESERVADAS.contains(&limpio.as_str())
        || integradas::buscar(&limpio).is_some()
        || EQUIVALENCIAS.iter().any(|(_, deimon, _, _)| *deimon == limpio)
        || ["verdadero", "falso", "nada"].contains(&limpio.as_str());
    if ocupado {
        format!("{}_php", limpio)
    } else {
        limpio
    }
}

fn texto_deimon(texto: &str) -> String {
    let mut salida = String::from("\"");
    for c in texto.chars() {
        match c {
            '"' => salida.push_str("\\\""),
            '\\' => salida.push_str("\\\\"),
            '\n' => salida.push_str("\\n"),
            '\t' => salida.push_str("\\t"),
            '\r' => {}
            otro => salida.push(otro),
        }
    }
    salida.push('"');
    salida
}

impl Escritor {
    fn linea(&mut self, texto: &str) {
        self.salida.push(format!("{}{}", "    ".repeat(self.nivel), texto));
    }

    fn con_parentesis(&self, (texto, precedencia): (String, u8), minimo: u8) -> String {
        if precedencia < minimo {
            format!("({})", texto)
        } else {
            texto
        }
    }

    /// Devuelve el código y su precedencia.
    fn expr(&mut self, e: &Expr) -> (String, u8) {
        match e {
            Expr::Numero(n) => (n.clone(), PRECEDENCIA_ATOMO),
            Expr::Logico(b) => ((if *b { "verdadero" } else { "falso" }).to_string(), PRECEDENCIA_ATOMO),
            Expr::Nulo => ("nada".to_string(), PRECEDENCIA_ATOMO),
            Expr::Variable(v) => (nombre_libre(v), PRECEDENCIA_ATOMO),
            Expr::SinTraducir(t) => {
                self.pendientes.push((0, format!("la constante '{}' no tiene equivalente (se puso nada)", t)));
                ("nada".to_string(), PRECEDENCIA_ATOMO)
            }
            Expr::Texto(trozos) => {
                let partes: Vec<String> = trozos
                    .iter()
                    .filter(|t| !matches!(t, Trozo::Literal(l) if l.is_empty()) || trozos.len() == 1)
                    .map(|t| match t {
                        Trozo::Literal(l) => texto_deimon(l),
                        Trozo::Variable(v) => nombre_libre(v),
                        Trozo::Indice(v, clave, true) => format!("{}[{}]", nombre_libre(v), clave),
                        Trozo::Indice(v, clave, false) => format!("{}[{}]", nombre_libre(v), texto_deimon(clave)),
                    })
                    .collect();
                if partes.len() == 1 && matches!(trozos[0], Trozo::Literal(_)) {
                    (partes[0].clone(), PRECEDENCIA_ATOMO)
                } else {
                    // Un texto con variables: se empieza por "" para que + siempre una textos.
                    let mut codigo = if matches!(trozos.first(), Some(Trozo::Literal(_))) { String::new() } else { "\"\" + ".to_string() };
                    codigo.push_str(&partes.join(" + "));
                    (codigo, PRECEDENCIA_SUMA)
                }
            }
            Expr::Lista(elementos) => {
                let partes: Vec<String> = elementos.iter().map(|e| self.expr(e).0).collect();
                (format!("[{}]", partes.join(", ")), PRECEDENCIA_ATOMO)
            }
            Expr::Mapa(pares) => {
                let partes: Vec<String> = pares.iter().map(|(c, v)| format!("{}: {}", self.expr(c).0, self.expr(v).0)).collect();
                (format!("{{{}}}", partes.join(", ")), PRECEDENCIA_ATOMO)
            }
            Expr::Indice(base, indice) => {
                let base = self.expr(base);
                let base = self.con_parentesis(base, PRECEDENCIA_ATOMO);
                let indice = self.expr(indice).0;
                (format!("{}[{}]", base, indice), PRECEDENCIA_ATOMO)
            }
            Expr::No(interior) => {
                let interior = self.expr(interior);
                (format!("no {}", self.con_parentesis(interior, PRECEDENCIA_COMPARACION)), PRECEDENCIA_NO)
            }
            Expr::Menos(interior) => {
                let interior = self.expr(interior);
                (format!("-{}", self.con_parentesis(interior, PRECEDENCIA_UNARIA)), PRECEDENCIA_UNARIA)
            }
            Expr::Binaria(op, izquierda, derecha) if es_metodo(izquierda) || es_metodo(derecha) => {
                let otro = if es_metodo(izquierda) { derecha } else { izquierda };
                let metodo = match otro.as_ref() {
                    Expr::Texto(t) => match t.as_slice() {
                        [Trozo::Literal(m)] => m.to_uppercase(),
                        _ => String::new(),
                    },
                    _ => String::new(),
                };
                let igual = matches!(op.as_str(), "==" | "===");
                let distinto = matches!(op.as_str(), "!=" | "!==" | "<>");
                match (metodo.as_str(), igual, distinto) {
                    ("POST", true, _) | ("GET", _, true) => ("es_envio()".to_string(), PRECEDENCIA_ATOMO),
                    ("POST", _, true) | ("GET", true, _) => ("no es_envio()".to_string(), PRECEDENCIA_NO),
                    _ => {
                        self.pendientes.push((0, "una comparación con $_SERVER['REQUEST_METHOD'] que no es con 'POST' o 'GET'".to_string()));
                        ("falso".to_string(), PRECEDENCIA_ATOMO)
                    }
                }
            }
            Expr::Binaria(op, izquierda, derecha) => {
                let (deimon, precedencia) = match op.as_str() {
                    "||" | "or" => ("o", PRECEDENCIA_O),
                    "&&" | "and" => ("y", PRECEDENCIA_Y),
                    "==" | "===" => ("==", PRECEDENCIA_COMPARACION),
                    "!=" | "!==" | "<>" => ("!=", PRECEDENCIA_COMPARACION),
                    "<" | "<=" | ">" | ">=" => (op.as_str(), PRECEDENCIA_COMPARACION),
                    "+" | "-" | "." => (if op == "." { "+" } else { op.as_str() }, PRECEDENCIA_SUMA),
                    _ => (op.as_str(), PRECEDENCIA_PRODUCTO),
                };
                let deimon = deimon.to_string();
                let mut izq = self.expr(izquierda);
                let der = self.expr(derecha);
                if op == "." && !es_texto(izquierda) && !es_texto(derecha) {
                    // En PHP '.' siempre une textos; en Deimon '+' sumaría dos números.
                    izq = (format!("a_texto({})", izq.0), PRECEDENCIA_ATOMO);
                }
                let izq = self.con_parentesis(izq, precedencia);
                let der = self.con_parentesis(der, precedencia + 1);
                (format!("{} {} {}", izq, deimon, der), precedencia)
            }
            Expr::Llamada(nombre, argumentos, linea) => self.llamada(nombre, argumentos, *linea),
        }
    }

    fn llamada(&mut self, nombre: &str, argumentos: &[Expr], linea: usize) -> (String, u8) {
        let minuscula = nombre.to_lowercase();
        if let Some((_, deimon, modulo, orden)) = EQUIVALENCIAS.iter().find(|(php, _, _, orden)| *php == minuscula && orden.len() == argumentos.len()) {
            if let Some(modulo) = modulo {
                self.modulos.insert(modulo);
            }
            let partes: Vec<String> = orden.iter().map(|i| self.expr(&argumentos[*i]).0).collect();
            return (format!("{}({})", deimon, partes.join(", ")), PRECEDENCIA_ATOMO);
        }
        match (minuscula.as_str(), argumentos.len()) {
            ("__provocar_error", 1) => {
                let mensaje = self.expr(&argumentos[0]).0;
                return (format!("provocar_error({})", mensaje), PRECEDENCIA_ATOMO);
            }
            ("time", 0) => {
                self.modulos.insert("tiempo");
                self.modulos.insert("matematicas");
                return ("suelo(ahora())".to_string(), PRECEDENCIA_ATOMO);
            }
            ("date", 1) | ("date", 2) => {
                self.modulos.insert("tiempo");
                let formato = self.expr(&argumentos[0]).0;
                let segundos = if argumentos.len() == 2 {
                    self.expr(&argumentos[1]).0
                } else {
                    "ahora()".to_string()
                };
                let zona = texto_deimon(&self.zona);
                return (
                    format!("formatear_fecha({}, {}, {})", segundos, formato, zona),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("mktime", 6) => {
                self.modulos.insert("tiempo");
                let hora = self.expr(&argumentos[0]).0;
                let minuto = self.expr(&argumentos[1]).0;
                let segundo = self.expr(&argumentos[2]).0;
                let mes = self.expr(&argumentos[3]).0;
                let dia = self.expr(&argumentos[4]).0;
                let anio = self.expr(&argumentos[5]).0;
                return (
                    format!(
                        "crear_fecha({}, {}, {}, {}, {}, {})",
                        anio, mes, dia, hora, minuto, segundo
                    ),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("checkdate", 3) => {
                self.modulos.insert("tiempo");
                let mes = self.expr(&argumentos[0]).0;
                let dia = self.expr(&argumentos[1]).0;
                let anio = self.expr(&argumentos[2]).0;
                return (
                    format!(
                        "({} >= 1 y {} <= 12 y {} >= 1 y {} <= dias_del_mes({}, {}))",
                        mes, mes, dia, dia, anio, mes
                    ),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("cal_days_in_month", 3) => {
                self.modulos.insert("tiempo");
                let mes = self.expr(&argumentos[1]).0;
                let anio = self.expr(&argumentos[2]).0;
                return (
                    format!("dias_del_mes({}, {})", anio, mes),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("strtotime", 1) | ("strtotime", 2) => {
                self.modulos.insert("tiempo");
                let base = if argumentos.len() == 2 {
                    self.expr(&argumentos[1]).0
                } else {
                    "ahora()".to_string()
                };
                return match texto_literal_php(&argumentos[0]) {
                    Some(texto) => match relativo_php(&texto) {
                        Some((cantidad, unidad)) => (
                            format!("sumar_tiempo({}, {}, \"{}\")", base, cantidad, unidad),
                            PRECEDENCIA_ATOMO,
                        ),
                        None if parece_fecha_absoluta_php(&texto) => (
                            format!("leer_fecha({})", texto_deimon(&texto)),
                            PRECEDENCIA_ATOMO,
                        ),
                        None => {
                            self.pendientes.push((
                                linea,
                                format!(
                                    "strtotime('{}') no está soportado; usa una fecha AAAA-MM-DD, AAAA-MM-DD HH:MM[:SS], AAAA-MM-DDTHH:MM:SS[Z o ±HH:MM] o \"+N day(s)/week(s)/month(s)/year(s)\"",
                                    texto
                                ),
                            ));
                            ("nada".to_string(), PRECEDENCIA_ATOMO)
                        }
                    },
                    None => {
                        self.avisos.push((
                            linea,
                            "strtotime con un texto que no es fijo: solo se traducen fechas absolutas (los \"+N días\" necesitan un texto literal)".to_string(),
                        ));
                        let texto = self.expr(&argumentos[0]).0;
                        (format!("leer_fecha({})", texto), PRECEDENCIA_ATOMO)
                    }
                };
            }
            ("date_default_timezone_set", 1) => {
                if let Some(zona) = texto_literal_php(&argumentos[0]) {
                    self.zona = zona;
                } else {
                    self.avisos.push((
                        linea,
                        "date_default_timezone_set con un valor que no es fijo: se usa la zona detectada al traducir".to_string(),
                    ));
                }
                return ("nada".to_string(), PRECEDENCIA_ATOMO);
            }
            ("__sesion", 1) | ("__sesion_borrar", 1) => {
                let clave = self.expr(&argumentos[0]).0;
                let funcion = if minuscula == "__sesion" { "leer_de_sesion" } else { "borrar_de_sesion" };
                return (format!("{}({})", funcion, clave), PRECEDENCIA_ATOMO);
            }
            ("__sesion_agregar", 2) => {
                let clave = self.expr(&argumentos[0]).0;
                let valor = self.expr(&argumentos[1]).0;
                return (format!("agregar_a_sesion({}, {})", clave, valor), PRECEDENCIA_ATOMO);
            }
            ("session_destroy", 0) | ("session_unset", 0) => {
                return ("vaciar_sesion()".to_string(), PRECEDENCIA_ATOMO);
            }
            ("__campo", 1) => {
                let nombre = self.expr(&argumentos[0]).0;
                return (format!("campo({})", nombre), PRECEDENCIA_ATOMO);
            }
            ("__metodo", 0) => {
                self.pendientes.push((linea, "$_SERVER['REQUEST_METHOD'] solo se traduce comparado con 'POST' o 'GET'".to_string()));
                return ("nada".to_string(), PRECEDENCIA_ATOMO);
            }
            ("__vacio", 1) => {
                let interior = self.expr(&argumentos[0]);
                let interior = self.con_parentesis(interior, PRECEDENCIA_SUMA);
                return (format!("({} == nada o {} == \"\")", interior, interior), PRECEDENCIA_ATOMO);
            }
            ("htmlspecialchars", 1) | ("htmlentities", 1) => {
                let ya_protegido = match &argumentos[0] {
                    Expr::Llamada(n, _, _) => n == "__campo",
                    Expr::Variable(v) => self.desde_campo.contains(v),
                    _ => false,
                };
                if ya_protegido {
                    return self.expr(&argumentos[0]);
                }
                let interior = self.expr(&argumentos[0]).0;
                return (format!("proteger_html({})", interior), PRECEDENCIA_ATOMO);
            }
            ("intval", 1) => {
                self.modulos.insert("matematicas");
                self.avisos.push((linea, "intval: se traduce como suelo(a_numero(...)); con números negativos PHP redondea hacia 0".to_string()));
                let interior = self.expr(&argumentos[0]).0;
                return (format!("suelo(a_numero({}))", interior), PRECEDENCIA_ATOMO);
            }
            ("substr", 2) | ("mb_substr", 2) | ("array_slice", 2) => {
                let valor = self.expr(&argumentos[0]).0;
                let inicio = self.expr(&argumentos[1]).0;
                return (format!("cortar({}, {}, nada)", valor, inicio), PRECEDENCIA_ATOMO);
            }
            ("substr", 3) | ("mb_substr", 3) | ("array_slice", 3) => {
                let partes: Vec<String> = argumentos.iter().map(|a| self.expr(a).0).collect();
                return (format!("cortar({})", partes.join(", ")), PRECEDENCIA_ATOMO);
            }
            ("json_decode", 2) if matches!(argumentos[1], Expr::Logico(true)) => {
                self.modulos.insert("json");
                let texto = self.expr(&argumentos[0]).0;
                return (format!("desde_json({})", texto), PRECEDENCIA_ATOMO);
            }
            ("array_merge", 2) => {
                let a = self.expr(&argumentos[0]);
                let b = self.expr(&argumentos[1]);
                let a = self.con_parentesis(a, PRECEDENCIA_SUMA);
                let b = self.con_parentesis(b, PRECEDENCIA_SUMA + 1);
                return (format!("{} + {}", a, b), PRECEDENCIA_SUMA);
            }
            ("sort", 1) | ("rsort", 1) => {
                // sort() cambia la lista que recibe: se reasigna la variable.
                if let Expr::Variable(v) = &argumentos[0] {
                    let nombre = nombre_libre(v);
                    let ordenada = if minuscula == "sort" { format!("ordenar({})", nombre) } else { format!("invertir(ordenar({}))", nombre) };
                    return (format!("{} = {}", nombre, ordenada), PRECEDENCIA_ATOMO);
                }
            }
            ("__galleta", 1) => {
                let nombre = self.expr(&argumentos[0]).0;
                return (format!("leer_galleta({})", nombre), PRECEDENCIA_ATOMO);
            }
            ("setcookie", 2) | ("setcookie", 3) => {
                let nombre = self.expr(&argumentos[0]).0;
                let valor = self.expr(&argumentos[1]).0;
                // El tercer argumento suele ser time() + segundos; time() - x (o un valor vacío) la borra.
                let caducidad = argumentos.get(2).map(|tercero| match tercero {
                    Expr::Binaria(op, a, b) if matches!(a.as_ref(), Expr::Llamada(n, x, _) if n.eq_ignore_ascii_case("time") && x.is_empty()) => match op.as_str() {
                        "+" => numero_constante(b).map(|s| Some(s / 86400.0)),
                        "-" => Some(None),
                        _ => None,
                    },
                    Expr::Numero(cero) if cero == "0" => Some(Some(0.0)),
                    _ => None,
                });
                // caducidad: None sin tercer argumento; Some(None) si no se entiende;
                // Some(Some(None)) para borrar; Some(Some(Some(días))) para guardar.
                match caducidad {
                    Some(Some(None)) => return (format!("borrar_galleta({})", nombre), PRECEDENCIA_ATOMO),
                    None => return (format!("guardar_galleta({}, {}, 0)", nombre, valor), PRECEDENCIA_ATOMO),
                    Some(Some(Some(dias))) => {
                        let dias = if dias.fract() == 0.0 { format!("{}", dias as i64) } else { format!("{}", dias) };
                        return (format!("guardar_galleta({}, {}, {})", nombre, valor, dias), PRECEDENCIA_ATOMO);
                    }
                    Some(None) => {
                        self.pendientes.push((linea, "setcookie con una caducidad que no es time() + segundos (se guarda hasta cerrar el navegador)".to_string()));
                        return (format!("guardar_galleta({}, {}, 0)", nombre, valor), PRECEDENCIA_ATOMO);
                    }
                }
            }
            ("__claves", 1) | ("array_keys", 1) => {
                let interior = self.expr(&argumentos[0]).0;
                return (format!("listar_claves({})", interior), PRECEDENCIA_ATOMO);
            }
            ("__quitar_clave", 2) => {
                let mapa = self.expr(&argumentos[0]).0;
                let clave = self.expr(&argumentos[1]).0;
                return (format!("quitar_clave({}, {})", mapa, clave), PRECEDENCIA_ATOMO);
            }
            ("array_key_exists", 2) => {
                let clave = self.expr(&argumentos[0]).0;
                let mapa = self.expr(&argumentos[1]).0;
                return (format!("tiene_clave({}, {})", mapa, clave), PRECEDENCIA_ATOMO);
            }
            ("__elegir", 3) => {
                let partes: Vec<String> = argumentos.iter().map(|a| self.expr(a).0).collect();
                return (format!("elegir({})", partes.join(", ")), PRECEDENCIA_ATOMO);
            }
            ("number_format", n) if (1..=4).contains(&n) => {
                self.modulos.insert("texto");
                let numero = self.expr(&argumentos[0]).0;
                let decimales = if n >= 2 { self.expr(&argumentos[1]).0 } else { "0".to_string() };
                let separador_decimal = if n >= 3 { self.expr(&argumentos[2]).0 } else { "\".\"".to_string() };
                let separador_miles = if n >= 4 { self.expr(&argumentos[3]).0 } else { "\",\"".to_string() };
                return (
                    format!(
                        "formatear_numero({}, {}, {}, {})",
                        numero, decimales, separador_decimal, separador_miles
                    ),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("str_pad", n) if (2..=4).contains(&n) => {
                self.modulos.insert("texto");
                let texto = self.expr(&argumentos[0]).0;
                let largo = self.expr(&argumentos[1]).0;
                let relleno = if n >= 3 { self.expr(&argumentos[2]).0 } else { "\" \"".to_string() };
                let lado = if n >= 4 {
                    match &argumentos[3] {
                        Expr::SinTraducir(constante) => match constante.to_uppercase().as_str() {
                            "STR_PAD_LEFT" => "\"izquierda\"".to_string(),
                            "STR_PAD_BOTH" => "\"ambos\"".to_string(),
                            "STR_PAD_RIGHT" => "\"derecha\"".to_string(),
                            _ => {
                                self.pendientes.push((linea, format!("la constante '{}' de str_pad no tiene equivalente (se usa \"derecha\")", constante)));
                                "\"derecha\"".to_string()
                            }
                        },
                        otro => self.expr(otro).0,
                    }
                } else {
                    "\"derecha\"".to_string()
                };
                return (
                    format!("rellenar_texto({}, {}, {}, {})", texto, largo, relleno, lado),
                    PRECEDENCIA_ATOMO,
                );
            }
            ("sprintf", n) | ("printf", n) if n >= 1 => {
                self.modulos.insert("texto");
                let plantilla = self.expr(&argumentos[0]).0;
                let valores: Vec<String> = argumentos[1..].iter().map(|a| self.expr(a).0).collect();
                let codigo = format!("formatear_texto({}, [{}])", plantilla, valores.join(", "));
                if minuscula == "printf" {
                    return (format!("escribir({})", codigo), PRECEDENCIA_ATOMO);
                }
                return (codigo, PRECEDENCIA_ATOMO);
            }
            ("preg_match", 2) | ("preg_match", 3) | ("preg_match_all", 2) | ("preg_match_all", 3) | ("preg_replace", 3) | ("preg_split", 2) => {
                return self.patrones(&minuscula, argumentos, linea);
            }
            ("is_null", 1) => {
                let interior = self.expr(&argumentos[0]);
                return (format!("{} == nada", self.con_parentesis(interior, PRECEDENCIA_SUMA)), PRECEDENCIA_COMPARACION);
            }
            _ => {}
        }
        if self.funciones.contains(&minuscula) {
            let partes: Vec<String> = argumentos.iter().map(|a| self.expr(a).0).collect();
            return (format!("{}({})", nombre_libre(&minuscula), partes.join(", ")), PRECEDENCIA_ATOMO);
        }
        self.pendientes.push((linea, format!("la función de PHP '{}' con {} argumentos no tiene equivalente en Deimon", nombre, argumentos.len())));
        ("nada".to_string(), PRECEDENCIA_ATOMO)
    }

    fn instrucciones(&mut self, lista: &[Instr]) {
        for instruccion in lista {
            self.instruccion(instruccion);
        }
    }

    fn instruccion(&mut self, instruccion: &Instr) {
        match instruccion {
            Instr::Escribir(partes) => {
                for parte in partes {
                    // echo "texto\n" se lee mejor como mostrar("texto") (en un script).
                    if let (false, Expr::Texto(trozos)) = (self.web, parte) {
                        if let Some(Trozo::Literal(ultimo)) = trozos.last() {
                            if ultimo.ends_with('\n') {
                                let mut sin_salto = trozos.clone();
                                if let Some(Trozo::Literal(l)) = sin_salto.last_mut() {
                                    l.pop();
                                }
                                let codigo = self.expr(&Expr::Texto(sin_salto)).0;
                                self.linea(&format!("mostrar({})", codigo));
                                continue;
                            }
                        }
                    }
                    let codigo = self.expr(parte).0;
                    self.linea(&format!("escribir({})", codigo));
                }
            }
            Instr::Html(html) => {
                if !html.is_empty() {
                    let codigo = texto_deimon(html);
                    self.linea(&format!("escribir({})", codigo));
                }
            }
            Instr::Asignar(variable, valor) => {
                if matches!(valor, Expr::Llamada(n, _, _) if n == "__campo") {
                    self.desde_campo.insert(variable.clone());
                } else {
                    self.desde_campo.remove(variable);
                }
                let valor = self.expr(valor).0;
                let sea = if self.nivel == self.nivel_ambito && self.por_declarar.remove(variable) { "sea " } else { "" };
                self.linea(&format!("{}{} = {}", sea, nombre_libre(variable), valor));
            }
            Instr::AsignarIndice(variable, indice, valor) if variable == "_SESSION" => {
                let indice = self.expr(indice).0;
                let valor = self.expr(valor).0;
                self.linea(&format!("guardar_en_sesion({}, {})", indice, valor));
            }
            Instr::AsignarIndice(variable, indice, valor) => {
                let indice = self.expr(indice).0;
                let valor = self.expr(valor).0;
                self.linea(&format!("{}[{}] = {}", nombre_libre(variable), indice, valor));
            }
            Instr::Agregar(variable, valor) => {
                let valor = self.expr(valor).0;
                let nombre = nombre_libre(variable);
                if self.nivel == self.nivel_ambito && self.por_declarar.remove(variable) {
                    self.linea(&format!("sea {} = [{}]", nombre, valor));
                } else {
                    self.linea(&format!("{} = agregar({}, {})", nombre, nombre, valor));
                }
            }
            Instr::Si(ramas, otro) => {
                for (indice, (condicion, cuerpo)) in ramas.iter().enumerate() {
                    let condicion = self.expr(condicion).0;
                    if indice == 0 {
                        self.linea(&format!("si {} {{", condicion));
                    } else {
                        self.nivel -= 1;
                        self.linea(&format!("}} sino si {} {{", condicion));
                    }
                    self.nivel += 1;
                    self.instrucciones(cuerpo);
                }
                if let Some(cuerpo) = otro {
                    self.nivel -= 1;
                    self.linea("} sino {");
                    self.nivel += 1;
                    self.instrucciones(cuerpo);
                }
                self.nivel -= 1;
                self.linea("}");
            }
            Instr::Mientras(condicion, cuerpo) => {
                let condicion = self.expr(condicion).0;
                self.linea(&format!("mientras {} {{", condicion));
                self.nivel += 1;
                self.pasos.push(Vec::new());
                self.instrucciones(cuerpo);
                self.pasos.pop();
                self.nivel -= 1;
                self.linea("}");
            }
            Instr::Para(inicio, condicion, paso, cuerpo) => {
                self.instrucciones(inicio);
                let condicion = condicion.as_ref().map(|c| self.expr(c).0).unwrap_or_else(|| "verdadero".to_string());
                self.linea(&format!("mientras {} {{", condicion));
                self.nivel += 1;
                self.pasos.push(paso.clone());
                self.instrucciones(cuerpo);
                self.pasos.pop();
                self.instrucciones(paso);
                self.nivel -= 1;
                self.linea("}");
            }
            Instr::ParaCada(lista, variable, cuerpo) => {
                let lista = self.expr(lista).0;
                self.linea(&format!("para {} en {} {{", nombre_libre(variable), lista));
                self.nivel += 1;
                self.pasos.push(Vec::new());
                self.instrucciones(cuerpo);
                self.pasos.pop();
                self.nivel -= 1;
                self.linea("}");
            }
            Instr::Intentar(cuerpo, variable, fallo) => {
                self.linea("intentar {");
                self.nivel += 1;
                self.instrucciones(cuerpo);
                self.nivel -= 1;
                self.linea(&format!("}} si falla {} {{", nombre_libre(variable)));
                self.nivel += 1;
                self.instrucciones(fallo);
                self.nivel -= 1;
                self.linea("}");
            }
            Instr::Funcion(..) => {} // las funciones se escriben aparte, al nivel principal
            Instr::Romper => self.linea("romper"),
            Instr::Continuar => {
                let paso = self.pasos.last().cloned().unwrap_or_default();
                self.instrucciones(&paso);
                self.linea("continuar");
            }
            Instr::Devolver(valor) => {
                let valor = valor.as_ref().map(|v| self.expr(v).0).unwrap_or_else(|| "nada".to_string());
                self.linea(&format!("devolver {}", valor));
            }
            Instr::Expresion(Expr::Nulo) => {}
            Instr::Expresion(Expr::Llamada(nombre, _, _))
                if nombre.eq_ignore_ascii_case("date_default_timezone_set") => {}
            Instr::Expresion(expresion @ Expr::Llamada(nombre, argumentos, _))
                if matches!(nombre.to_lowercase().as_str(), "print_r" | "var_dump") && argumentos.len() == 1 =>
            {
                let codigo = self.expr(expresion).0;
                self.linea(&format!("escribir({})", codigo));
            }
            Instr::Expresion(expresion) => {
                let codigo = self.expr(expresion).0;
                self.linea(&codigo);
            }
            Instr::Varias(lista) => self.instrucciones(lista),
            Instr::Salir(mensaje) => {
                let tras_devolver = self.salida.last().map(|l| l.trim_start().starts_with("devolver ")).unwrap_or(false);
                if mensaje.is_none() && tras_devolver {
                    return; // header("Location: ...") seguido de exit: ya se devolvió.
                }
                if let Some(mensaje) = mensaje {
                    let codigo = self.expr(mensaje).0;
                    self.linea(&format!("escribir({})", codigo));
                }
                self.linea("devolver \"\"");
            }
            Instr::Redirigir(destino) => {
                let codigo = self.expr(destino).0;
                self.linea(&format!("devolver redirigir({})", codigo));
            }
            Instr::SinTraducir(linea, texto) => {
                self.linea(&format!("# PHP sin traducir (línea {}): {}", linea, texto));
            }
        }
    }

    /// Declara con `sea ... = nada` todas las variables que se asignan en un ámbito, para
    /// que existan en todo él (en PHP una variable creada dentro de un if sigue viva fuera).
    fn declarar_variables(&mut self, cuerpo: &[Instr], parametros: &[String]) {
        // Las que se asignan por primera vez al nivel del ámbito se declaran ahí mismo con
        // `sea`; las que aparecen antes dentro de un si o un bucle se declaran arriba.
        let mut vistas: Vec<String> = parametros.to_vec();
        let mut en_linea = HashSet::new();
        let mut arriba = Vec::new();
        for instruccion in cuerpo {
            match instruccion {
                Instr::Asignar(v, valor) | Instr::Agregar(v, valor) if !vistas.contains(v) => {
                    let mut de_preg = Vec::new();
                    recoger_preg(valor, &mut de_preg);
                    for p in de_preg {
                        if !vistas.contains(&p) {
                            vistas.push(p.clone());
                            en_linea.insert(p);
                        }
                    }
                    vistas.push(v.clone());
                    en_linea.insert(v.clone());
                }
                otra => {
                    let mut interiores = Vec::new();
                    recoger_asignadas(std::slice::from_ref(otra), &mut interiores);
                    for v in interiores {
                        if !vistas.contains(&v) {
                            vistas.push(v.clone());
                            arriba.push(v);
                        }
                    }
                }
            }
        }
        for variable in arriba {
            self.linea(&format!("sea {} = nada", nombre_libre(&variable)));
        }
        self.por_declarar = en_linea;
        self.nivel_ambito = self.nivel;
    }
}

/// Valor de una cuenta hecha solo con números (86400 * 30), si lo es.
fn numero_constante(e: &Expr) -> Option<f64> {
    match e {
        Expr::Numero(n) => n.parse().ok(),
        Expr::Binaria(op, a, b) => {
            let (a, b) = (numero_constante(a)?, numero_constante(b)?);
            match op.as_str() {
                "+" => Some(a + b),
                "-" => Some(a - b),
                "*" => Some(a * b),
                "/" if b != 0.0 => Some(a / b),
                _ => None,
            }
        }
        _ => None,
    }
}

fn es_metodo(e: &Expr) -> bool {
    matches!(e, Expr::Llamada(n, _, _) if n == "__metodo")
}

impl Escritor {
    /// preg_match, preg_match_all, preg_replace y preg_split -> módulo `patrones`.
    fn patrones(&mut self, funcion: &str, argumentos: &[Expr], linea: usize) -> (String, u8) {
        let patron = match patron_de_php(&argumentos[0]) {
            Ok(p) => texto_deimon(&p),
            Err(motivo) => {
                self.pendientes.push((linea, format!("{}: {}", funcion, motivo)));
                return ("nada".to_string(), PRECEDENCIA_ATOMO);
            }
        };
        self.modulos.insert("patrones");
        match funcion {
            "preg_replace" => {
                let nuevo = self.expr(&argumentos[1]).0;
                let texto = self.expr(&argumentos[2]).0;
                (format!("reemplazar_patron({}, {}, {})", texto, patron, nuevo), PRECEDENCIA_ATOMO)
            }
            "preg_split" => {
                let texto = self.expr(&argumentos[1]).0;
                (format!("partir_patron({}, {})", texto, patron), PRECEDENCIA_ATOMO)
            }
            _ => {
                let todos = funcion == "preg_match_all";
                let texto = self.expr(&argumentos[1]).0;
                match argumentos.get(2) {
                    None if todos => (format!("medir(buscar_todos_patron({}, {}))", texto, patron), PRECEDENCIA_ATOMO),
                    None => (format!("coincide_patron({}, {})", texto, patron), PRECEDENCIA_ATOMO),
                    Some(Expr::Variable(v)) => {
                        // El grupo se guarda en la variable justo antes de la instrucción que lo usa.
                        let nombre = nombre_libre(v);
                        let sea = if self.nivel == self.nivel_ambito && self.por_declarar.remove(v) { "sea " } else { "" };
                        let buscar = if todos { "buscar_columnas_patron" } else { "buscar_patron" };
                        self.linea(&format!("{}{} = {}({}, {})", sea, nombre, buscar, texto, patron));
                        if todos {
                            (format!("medir({}[0])", nombre), PRECEDENCIA_ATOMO)
                        } else {
                            (format!("{} != nada", nombre), PRECEDENCIA_COMPARACION)
                        }
                    }
                    Some(_) => {
                        self.pendientes.push((linea, format!("{}: el tercer argumento debe ser una variable", funcion)));
                        ("nada".to_string(), PRECEDENCIA_ATOMO)
                    }
                }
            }
        }
    }
}

fn es_texto(e: &Expr) -> bool {
    matches!(e, Expr::Texto(_))
        || matches!(e, Expr::Binaria(op, _, _) if op == ".")
        || matches!(e, Expr::Llamada(n, _, _) if ["strval", "strtoupper", "strtolower", "trim", "str_replace", "implode", "str_repeat", "mb_strtoupper", "mb_strtolower"].contains(&n.to_lowercase().as_str()))
}

/// Variables que preg_match / preg_match_all rellenan (tercer argumento) dentro de una expresión.
fn recoger_preg(e: &Expr, vistas: &mut Vec<String>) {
    match e {
        Expr::Llamada(n, argumentos, _) => {
            if matches!(n.to_lowercase().as_str(), "preg_match" | "preg_match_all") {
                if let Some(Expr::Variable(v)) = argumentos.get(2) {
                    if !vistas.contains(v) {
                        vistas.push(v.clone());
                    }
                }
            }
            for a in argumentos {
                recoger_preg(a, vistas);
            }
        }
        Expr::No(i) | Expr::Menos(i) => recoger_preg(i, vistas),
        Expr::Binaria(_, i, d) => {
            recoger_preg(i, vistas);
            recoger_preg(d, vistas);
        }
        _ => {}
    }
}

/// Quita los delimitadores del patrón de PHP (/.../, #...#, ~...~) y pasa el modificador i a (?i).
/// Devuelve el motivo si el patrón usa algo que el módulo `patrones` no sabe hacer.
fn patron_de_php(e: &Expr) -> Result<String, String> {
    let literal = match e {
        Expr::Texto(trozos) => match trozos.as_slice() {
            [Trozo::Literal(l)] => l.clone(),
            _ => return Err("el patrón no es un texto fijo (se traduce a mano)".to_string()),
        },
        _ => return Err("el patrón no es un texto fijo (se traduce a mano)".to_string()),
    };
    let delimitador = match literal.chars().next() {
        Some(d @ ('/' | '#' | '~')) => d,
        _ => return Err("el patrón no empieza por un delimitador /, # o ~".to_string()),
    };
    let fin = match literal.rfind(delimitador) {
        Some(f) if f > 0 => f,
        _ => return Err("al patrón le falta el delimitador del final".to_string()),
    };
    let cuerpo = &literal[1..fin];
    let mut ignorar = false;
    for m in literal[fin + 1..].chars() {
        match m {
            'i' => ignorar = true,
            'u' => {}
            otro => return Err(format!("el modificador '{}' del patrón no está soportado", otro)),
        }
    }
    let letras: Vec<char> = cuerpo.chars().collect();
    let mut i = 0;
    while i < letras.len() {
        match letras[i] {
            '\\' => {
                match letras.get(i + 1) {
                    Some(c) if c.is_ascii_digit() => return Err("las referencias a grupos (\\1) no están soportadas".to_string()),
                    Some(c) if c.is_ascii_alphabetic() && !"dDwWsSbBntr".contains(*c) => {
                        return Err(format!("el escape \\{} del patrón no está soportado", c));
                    }
                    _ => {}
                }
                i += 2;
                continue;
            }
            '(' if letras.get(i + 1) == Some(&'?') && letras.get(i + 2) != Some(&':') => {
                return Err("los grupos especiales (?...), como mirar adelante o atrás, no están soportados".to_string());
            }
            _ => {}
        }
        i += 1;
    }
    Ok(if ignorar { format!("(?i){}", cuerpo) } else { cuerpo.to_string() })
}

fn recoger_asignadas(lista: &[Instr], vistas: &mut Vec<String>) {
    for instruccion in lista {
        match instruccion {
            Instr::AsignarIndice(v, _, _) if v == "_SESSION" => {}
            Instr::Asignar(v, valor) | Instr::AsignarIndice(v, _, valor) | Instr::Agregar(v, valor) => {
                recoger_preg(valor, vistas);
                if !vistas.contains(v) {
                    vistas.push(v.clone());
                }
            }
            Instr::Expresion(e) => recoger_preg(e, vistas),
            Instr::Si(ramas, otro) => {
                for (condicion, cuerpo) in ramas {
                    recoger_preg(condicion, vistas);
                    recoger_asignadas(cuerpo, vistas);
                }
                if let Some(cuerpo) = otro {
                    recoger_asignadas(cuerpo, vistas);
                }
            }
            Instr::Mientras(condicion, cuerpo) => {
                recoger_preg(condicion, vistas);
                recoger_asignadas(cuerpo, vistas);
            }
            Instr::Varias(cuerpo) => recoger_asignadas(cuerpo, vistas),
            Instr::Intentar(cuerpo, variable, fallo) => {
                recoger_asignadas(cuerpo, vistas);
                let mut interiores = Vec::new();
                recoger_asignadas(fallo, &mut interiores);
                for v in interiores {
                    if v != *variable && !vistas.contains(&v) {
                        vistas.push(v);
                    }
                }
            }
            Instr::Para(inicio, _, paso, cuerpo) => {
                recoger_asignadas(inicio, vistas);
                recoger_asignadas(cuerpo, vistas);
                recoger_asignadas(paso, vistas);
            }
            Instr::ParaCada(_, variable, cuerpo) => {
                // La variable del bucle la crea 'para'; no se declara aparte.
                let mut interiores = Vec::new();
                recoger_asignadas(cuerpo, &mut interiores);
                for v in interiores {
                    if v != *variable && !vistas.contains(&v) {
                        vistas.push(v);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Traduce un programa PHP a Deimon.
/// Lee un archivo PHP y devuelve sus instrucciones (y lo que no se pudo leer).
fn analizar(php: &str, web: bool) -> Result<(Vec<Instr>, Vec<(usize, String)>), String> {
    let piezas = leer_piezas(php)?;
    let mut analizador = Analizador { piezas, pos: 0, pendientes: Vec::new(), web, saltos: Vec::new() };
    let mut programa = Vec::new();
    while *analizador.actual() != Pieza::Fin {
        programa.push(analizador.instruccion());
    }
    Ok((programa, analizador.pendientes))
}

fn nombres_de_funciones(programa: &[Instr]) -> HashSet<String> {
    programa
        .iter()
        .filter_map(|i| match i {
            Instr::Funcion(nombre, _, _) => Some(nombre.to_lowercase()),
            _ => None,
        })
        .collect()
}

impl Escritor {
    fn nuevo(funciones: HashSet<String>, pendientes: Vec<(usize, String)>, web: bool) -> Self {
        Escritor {
            salida: Vec::new(),
            nivel: 0,
            modulos: BTreeSet::new(),
            pendientes,
            avisos: Vec::new(),
            funciones,
            por_declarar: HashSet::new(),
            nivel_ambito: 0,
            web,
            desde_campo: HashSet::new(),
            pasos: Vec::new(),
            zona: "UTC".to_string(),
        }
    }

    fn funciones_de(&mut self, programa: &[Instr]) {
        for instruccion in programa {
            if let Instr::Funcion(nombre, parametros, cuerpo) = instruccion {
                let parametros_deimon: Vec<String> = parametros.iter().map(|p| nombre_libre(p)).collect();
                self.linea(&format!("funcion {}({}) {{", nombre_libre(&nombre.to_lowercase()), parametros_deimon.join(", ")));
                self.nivel += 1;
                self.declarar_variables(cuerpo, parametros);
                self.desde_campo.clear();
                self.instrucciones(cuerpo);
                self.nivel -= 1;
                self.linea("}");
                self.linea("");
            }
        }
    }

    fn terminar(mut self) -> Traduccion {
        let mut codigo = String::from("# Traducido de PHP con 'deimon desde-php'.\n");
        self.pendientes.sort();
        for (linea, motivo) in &self.pendientes {
            if *linea > 0 {
                codigo.push_str(&format!("# PENDIENTE (línea {} del PHP): {}\n", linea, motivo));
            } else {
                codigo.push_str(&format!("# PENDIENTE: {}\n", motivo));
            }
        }
        for modulo in &self.modulos {
            codigo.push_str(&format!("usar modulo \"{}\"\n", modulo));
        }
        codigo.push('\n');
        for linea in &self.salida {
            codigo.push_str(linea.trim_end());
            codigo.push('\n');
        }
        Traduccion {
            codigo,
            pendientes: self.pendientes,
            avisos: self.avisos,
        }
    }
}

/// Traduce un programa PHP (un script que se ejecuta y escribe) a Deimon.
pub fn traducir(php: &str) -> Result<Traduccion, String> {
    let (programa, pendientes) = analizar(php, false)?;
    let mut escritor = Escritor::nuevo(nombres_de_funciones(&programa), pendientes, false);
    escritor.zona = zona_de_php(php);
    escritor.funciones_de(&programa);
    let principal: Vec<Instr> = programa.into_iter().filter(|i| !matches!(i, Instr::Funcion(..))).collect();
    escritor.declarar_variables(&principal, &[]);
    escritor.instrucciones(&principal);
    Ok(escritor.terminar())
}

/// Traduce un sitio web de PHP: cada archivo (camino, código) pasa a ser una página
/// (`ruta`); index.php responde también en "/". Al final se añade `servir(puerto)`.
pub fn traducir_web(paginas: &[(String, String)], puerto: u16) -> Result<Traduccion, String> {
    let mut programas = Vec::new();
    let mut pendientes = Vec::new();
    for (camino, php) in paginas {
        let (programa, mut suyos) = analizar(php, true).map_err(|e| format!("{}: {}", camino, e))?;
        for (_, motivo) in suyos.iter_mut() {
            *motivo = format!("{}: {}", camino, motivo);
        }
        pendientes.append(&mut suyos);
        programas.push((camino.clone(), programa));
    }
    let mut funciones = HashSet::new();
    for (camino, programa) in &programas {
        for nombre in nombres_de_funciones(programa) {
            if !funciones.insert(nombre.clone()) {
                pendientes.push((0, format!("{}: la función '{}' está definida en dos páginas", camino, nombre)));
            }
        }
    }
    let mut escritor = Escritor::nuevo(funciones, pendientes, true);
    escritor.zona = paginas
        .iter()
        .map(|(_, php)| zona_de_php(php))
        .find(|zona| zona != "UTC")
        .unwrap_or_else(|| "UTC".to_string());
    for (_, programa) in &programas {
        escritor.funciones_de(programa);
    }
    for (camino, programa) in &programas {
        let base: String = camino
            .trim_start_matches('/')
            .trim_end_matches(".php")
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let funcion = format!("pagina_{}", base);
        escritor.linea(&format!("# Página {}", camino));
        escritor.linea(&format!("funcion {}() {{", funcion));
        escritor.nivel += 1;
        let principal: Vec<Instr> = programa.iter().filter(|i| !matches!(i, Instr::Funcion(..))).cloned().collect();
        escritor.declarar_variables(&principal, &[]);
        escritor.desde_campo.clear();
        escritor.instrucciones(&principal);
        escritor.linea("devolver \"\"");
        escritor.nivel -= 1;
        escritor.linea("}");
        escritor.linea("");
        escritor.linea(&format!("ruta \"{}\" {{", camino));
        escritor.linea(&format!("    devolver {}()", funcion));
        escritor.linea("}");
        escritor.linea("");
        if camino == "/index.php" {
            escritor.linea("ruta \"/\" {");
            escritor.linea(&format!("    devolver {}()", funcion));
            escritor.linea("}");
            escritor.linea("");
        }
    }
    escritor.linea(&format!("servir({})", puerto));
    Ok(escritor.terminar())
}
