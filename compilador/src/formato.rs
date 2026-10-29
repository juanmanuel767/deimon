//! Formateador del lenguaje `deimon`.
//!
//! Reescribe un programa con el estilo único del lenguaje: cuatro espacios por
//! nivel, un espacio alrededor de los operadores, sin espacios al final de
//! línea, como mucho una línea vacía seguida y una línea vacía entre las
//! definiciones de nivel superior (`funcion`, `ruta` y `prueba`).
//!
//! Trabaja sobre los tokens del lexer (incluidos los comentarios), así que no
//! cambia los textos entre comillas ni lo que el parser acepta.

use std::fs;
use std::path::Path;

use crate::error::ErrorTraduccion;
use crate::lexer::{tokenizar_con_comentarios, Tipo, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClaseBloque {
    Definicion,
    Otro,
}

/// Formatea `fuente` con el estilo único del lenguaje.
pub fn formatear(fuente: &str) -> Result<String, ErrorTraduccion> {
    let tokens = tokenizar_con_comentarios(fuente)?;
    Ok(Formateador::nuevo().formatear(&tokens))
}

fn es_palabra(tipo: &Tipo) -> bool {
    matches!(
        tipo,
        Tipo::Ident(_)
            | Tipo::Numero(_)
            | Tipo::EnteroGrande(_)
            | Tipo::Texto(_)
            | Tipo::TextoEspecial { .. }
            | Tipo::Verdadero
            | Tipo::Falso
            | Tipo::Nada
            | Tipo::Sea
            | Tipo::Usar
            | Tipo::Funcion
            | Tipo::Ruta
            | Tipo::Prueba
            | Tipo::Extension
            | Tipo::Modulo
            | Tipo::Devolver
            | Tipo::Romper
            | Tipo::Continuar
            | Tipo::Si
            | Tipo::Sino
            | Tipo::Intenta
            | Tipo::Falla
            | Tipo::Mientras
            | Tipo::Para
            | Tipo::En
    )
}

fn es_operador_binario(tipo: &Tipo) -> bool {
    matches!(
        tipo,
        Tipo::Mas
            | Tipo::Por
            | Tipo::Entre
            | Tipo::Resto
            | Tipo::Igual
            | Tipo::Distinto
            | Tipo::Menor
            | Tipo::MenorIgual
            | Tipo::Mayor
            | Tipo::MayorIgual
            | Tipo::Y
            | Tipo::O
    )
}

fn texto_token(tipo: &Tipo) -> String {
    match tipo {
        Tipo::Numero(n) => format!("{}", n),
        Tipo::EnteroGrande(cifras) => cifras.clone(),
        Tipo::Texto(texto) => escapar_texto(texto),
        Tipo::TextoEspecial { crudo, .. } => crudo.clone(),
        Tipo::Ident(nombre) => nombre.clone(),
        Tipo::Comentario(texto) => texto.clone(),
        Tipo::Sea => "sea".to_string(),
        Tipo::Usar => "usar".to_string(),
        Tipo::Funcion => "funcion".to_string(),
        Tipo::Ruta => "ruta".to_string(),
        Tipo::Prueba => "prueba".to_string(),
        Tipo::Extension => "extension".to_string(),
        Tipo::Modulo => "modulo".to_string(),
        Tipo::Devolver => "devolver".to_string(),
        Tipo::Romper => "romper".to_string(),
        Tipo::Continuar => "continuar".to_string(),
        Tipo::Si => "si".to_string(),
        Tipo::Sino => "sino".to_string(),
        Tipo::Intenta => "intentar".to_string(),
        Tipo::Falla => "falla".to_string(),
        Tipo::Mientras => "mientras".to_string(),
        Tipo::Para => "para".to_string(),
        Tipo::En => "en".to_string(),
        Tipo::Y => "y".to_string(),
        Tipo::O => "o".to_string(),
        Tipo::No => "no".to_string(),
        Tipo::Verdadero => "verdadero".to_string(),
        Tipo::Falso => "falso".to_string(),
        Tipo::Nada => "nada".to_string(),
        Tipo::Mas => "+".to_string(),
        Tipo::Menos => "-".to_string(),
        Tipo::Por => "*".to_string(),
        Tipo::Entre => "/".to_string(),
        Tipo::Resto => "%".to_string(),
        Tipo::Igual => "==".to_string(),
        Tipo::Distinto => "!=".to_string(),
        Tipo::Menor => "<".to_string(),
        Tipo::MenorIgual => "<=".to_string(),
        Tipo::Mayor => ">".to_string(),
        Tipo::MayorIgual => ">=".to_string(),
        Tipo::Asignacion => "=".to_string(),
        Tipo::LParen => "(".to_string(),
        Tipo::RParen => ")".to_string(),
        Tipo::LLave => "{".to_string(),
        Tipo::RLave => "}".to_string(),
        Tipo::LCorchete => "[".to_string(),
        Tipo::RCorchete => "]".to_string(),
        Tipo::Coma => ",".to_string(),
        Tipo::DosPuntos => ":".to_string(),
        Tipo::Ajeno(texto) => texto.clone(),
        Tipo::NuevaLinea => "\n".to_string(),
        Tipo::Fin => String::new(),
    }
}

/// Devuelve el texto de un texto entre comillas con sus escapes, sin tocar su contenido.
fn escapar_texto(texto: &str) -> String {
    let mut salida = String::from("\"");
    for caracter in texto.chars() {
        match caracter {
            '\\' => salida.push_str("\\\\"),
            '"' => salida.push_str("\\\""),
            '\n' => salida.push_str("\\n"),
            '\t' => salida.push_str("\\t"),
            otro => salida.push(otro),
        }
    }
    salida.push('"');
    salida
}

struct Formateador {
    salida: String,
    linea: String,
    sangria: usize,
    parentesis: usize,
    pila: Vec<ClaseBloque>,
    ultimo: Option<Tipo>,
    linea_ultimo: usize,
    pegado: bool,
    separar: bool,
    pendiente_salto: bool,
    pendiente_cierre: bool,
    pendiente_sino: bool,
    unir_siguiente: bool,
    ignorar_salto: bool,
    tras_definicion: bool,
    definicion_pendiente: bool,
    /// Por cada '{' abierta: verdadero si es una lista con claves y no un bloque.
    llaves: Vec<bool>,
}

impl Formateador {
    fn nuevo() -> Self {
        Formateador {
            salida: String::new(),
            linea: String::new(),
            sangria: 0,
            parentesis: 0,
            pila: Vec::new(),
            ultimo: None,
            linea_ultimo: 0,
            pegado: false,
            separar: false,
            pendiente_salto: false,
            pendiente_cierre: false,
            pendiente_sino: false,
            unir_siguiente: false,
            ignorar_salto: false,
            tras_definicion: false,
            definicion_pendiente: false,
            llaves: Vec::new(),
        }
    }

    fn formatear(mut self, tokens: &[Token]) -> String {
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            match &token.tipo {
                Tipo::NuevaLinea => self.salto_de_linea(),
                Tipo::Comentario(texto) => self.comentario(texto, token.linea),
                Tipo::Fin => {
                    self.preparar_codigo();
                    self.cerrar_linea();
                }
                tipo => {
                    let mut siguiente_lista = tokens[i + 1..]
                        .iter()
                        .filter(|t| !matches!(t.tipo, Tipo::NuevaLinea));
                    let siguiente = siguiente_lista.next();
                    let siguiente2 = siguiente_lista.next();
                    self.codigo(tipo, token.linea, siguiente, siguiente2);
                }
            }
            i += 1;
        }
        self.cerrar_linea();
        while self.salida.ends_with("\n\n") {
            self.salida.pop();
        }
        if self.salida.is_empty() {
            return String::new();
        }
        if !self.salida.ends_with('\n') {
            self.salida.push('\n');
        }
        self.salida
    }

    fn inicio_linea(&self) -> bool {
        self.linea.trim().is_empty()
    }

    fn escribir(&mut self, texto: &str, espacio: bool) {
        if self.inicio_linea() {
            self.linea.clear();
            self.linea.push_str(&"    ".repeat(self.sangria));
        }
        if espacio && !self.linea.ends_with(' ') {
            self.linea.push(' ');
        }
        self.linea.push_str(texto);
    }

    fn cerrar_linea(&mut self) {
        let limpia = self.linea.trim_end().to_string();
        if !limpia.is_empty() {
            self.salida.push_str(&limpia);
            self.salida.push('\n');
        }
        self.linea.clear();
    }

    fn linea_vacia(&mut self) {
        self.cerrar_linea();
        if !self.salida.is_empty() && !self.salida.ends_with("\n\n") {
            self.salida.push('\n');
        }
    }

    fn preparar_codigo(&mut self) {
        if self.pendiente_salto {
            self.cerrar_linea();
            self.pendiente_salto = false;
        }
        if self.pendiente_cierre {
            self.cerrar_linea();
            self.pendiente_cierre = false;
        }
        self.pendiente_sino = false;
        self.unir_siguiente = false;
    }

    fn ultima_linea_es_comentario(&self) -> bool {
        let texto = self.salida.trim_end_matches('\n');
        let ultima = texto.rsplit('\n').next().unwrap_or("");
        ultima.trim_start().starts_with('#')
    }

    fn separar_antes_de_definicion(&mut self) {
        if self.ultima_linea_es_comentario() {
            self.cerrar_linea();
        } else {
            self.linea_vacia();
        }
    }

    fn aplicar_tras_definicion(&mut self) {
        if self.tras_definicion && self.sangria == 0 {
            self.linea_vacia();
            self.tras_definicion = false;
        }
    }

    fn salto_de_linea(&mut self) {
        if self.parentesis > 0 {
            return;
        }
        if self.ignorar_salto {
            self.ignorar_salto = false;
            return;
        }
        if self.unir_siguiente && (self.pendiente_cierre || self.pendiente_sino) {
            return;
        }
        if self.pendiente_salto {
            self.cerrar_linea();
            self.pendiente_salto = false;
            return;
        }
        if self.pendiente_cierre {
            self.cerrar_linea();
            self.pendiente_cierre = false;
            return;
        }
        if !self.inicio_linea() {
            self.cerrar_linea();
            return;
        }
        if !self.salida.is_empty() && !self.salida.ends_with("\n\n") {
            self.salida.push('\n');
        }
    }

    fn comentario(&mut self, texto: &str, linea_fuente: usize) {
        let de_cola = !self.inicio_linea()
            && self.ultimo.is_some()
            && self.linea_ultimo == linea_fuente;
        if de_cola {
            self.linea.push_str("  ");
            self.linea.push_str(texto.trim_end());
            self.cerrar_linea();
            self.pendiente_salto = false;
            self.pendiente_cierre = false;
            self.pendiente_sino = false;
            self.unir_siguiente = false;
            self.ignorar_salto = true;
            return;
        }
        self.preparar_codigo();
        self.aplicar_tras_definicion();
        self.escribir(texto.trim_end(), false);
        self.cerrar_linea();
        self.ignorar_salto = true;
    }

    fn codigo(
        &mut self,
        tipo: &Tipo,
        linea: usize,
        siguiente: Option<&Token>,
        siguiente2: Option<&Token>,
    ) {
        if matches!(tipo, Tipo::Si)
            && self.pendiente_cierre
            && siguiente.map(|t| matches!(t.tipo, Tipo::Falla)).unwrap_or(false)
        {
            self.escribir("si", true);
            self.pendiente_cierre = false;
            self.pendiente_sino = true;
            self.unir_siguiente = true;
            self.pegado = false;
            self.separar = true;
            self.recordar(tipo, linea);
            return;
        }
        if matches!(tipo, Tipo::Sino) && self.pendiente_cierre {
            self.escribir("sino", true);
            self.pendiente_cierre = false;
            self.pendiente_sino = true;
            self.unir_siguiente = siguiente
                .map(|t| matches!(t.tipo, Tipo::Si | Tipo::LLave))
                .unwrap_or(false);
            self.pegado = false;
            self.separar = true;
            self.recordar(tipo, linea);
            return;
        }

        self.preparar_codigo();
        self.aplicar_tras_definicion();

        if self.sangria == 0
            && self.inicio_linea()
            && matches!(tipo, Tipo::Funcion | Tipo::Ruta | Tipo::Prueba)
        {
            self.separar_antes_de_definicion();
            self.definicion_pendiente = true;
        }

        match tipo {
            Tipo::LLave if self.parentesis > 0
                || matches!(
                    self.ultimo,
                    Some(
                        Tipo::Asignacion
                            | Tipo::LParen
                            | Tipo::LCorchete
                            | Tipo::Coma
                            | Tipo::DosPuntos
                            | Tipo::Devolver
                            | Tipo::Mas
                            | Tipo::Igual
                            | Tipo::Distinto
                    )
                ) =>
            {
                // Lista con claves: se escribe como una lista, en la misma línea.
                let espacio = !self.pegado && !self.inicio_linea();
                self.escribir("{", espacio);
                self.llaves.push(true);
                self.parentesis += 1;
                self.pegado = true;
                self.separar = false;
            }
            Tipo::RLave if self.llaves.last() == Some(&true) => {
                self.llaves.pop();
                self.escribir("}", false);
                self.parentesis = self.parentesis.saturating_sub(1);
                self.pegado = false;
                self.separar = false;
            }
            Tipo::DosPuntos => {
                self.escribir(":", false);
                self.pegado = false;
                self.separar = true;
            }
            Tipo::LLave => {
                self.llaves.push(false);
                self.escribir("{", true);
                self.sangria += 1;
                let clase = if self.definicion_pendiente {
                    self.definicion_pendiente = false;
                    ClaseBloque::Definicion
                } else {
                    ClaseBloque::Otro
                };
                self.pila.push(clase);
                self.pendiente_salto = true;
                self.pegado = false;
                self.separar = false;
            }
            Tipo::RLave => {
                self.llaves.pop();
                if !self.inicio_linea() {
                    self.cerrar_linea();
                }
                self.sangria = self.sangria.saturating_sub(1);
                self.escribir("}", false);
                let clase = self.pila.pop().unwrap_or(ClaseBloque::Otro);
                self.pendiente_cierre = true;
                self.unir_siguiente = siguiente
                    .map(|t| {
                        matches!(t.tipo, Tipo::Sino)
                            || (matches!(t.tipo, Tipo::Si)
                                && siguiente2
                                    .map(|segundo| matches!(segundo.tipo, Tipo::Falla))
                                    .unwrap_or(false))
                    })
                    .unwrap_or(false);
                self.pegado = false;
                self.separar = false;
                if clase == ClaseBloque::Definicion && self.sangria == 0 {
                    self.tras_definicion = true;
                }
            }
            Tipo::LParen => {
                let espacio = if self.pegado {
                    false
                } else if self.separar {
                    true
                } else {
                    !matches!(self.ultimo, Some(Tipo::Ident(_)))
                };
                self.escribir("(", espacio);
                self.parentesis += 1;
                self.pegado = true;
                self.separar = false;
            }
            Tipo::RParen => {
                self.escribir(")", false);
                self.parentesis = self.parentesis.saturating_sub(1);
                self.pegado = false;
                self.separar = false;
            }
            Tipo::LCorchete => {
                let espacio = if self.pegado {
                    false
                } else if self.separar {
                    true
                } else {
                    !matches!(
                        self.ultimo,
                        Some(
                            Tipo::Ident(_)
                                | Tipo::Numero(_)
                                | Tipo::EnteroGrande(_)
                                | Tipo::Texto(_)
                                | Tipo::TextoEspecial { .. }
                                | Tipo::Verdadero
                                | Tipo::Falso
                                | Tipo::Nada
                                | Tipo::RParen
                                | Tipo::RCorchete
                                | Tipo::LParen
                                | Tipo::LCorchete
                        )
                    )
                };
                self.escribir("[", espacio);
                self.parentesis += 1;
                self.pegado = true;
                self.separar = false;
            }
            Tipo::RCorchete => {
                self.escribir("]", false);
                self.parentesis = self.parentesis.saturating_sub(1);
                self.pegado = false;
                self.separar = false;
            }
            Tipo::Coma => {
                self.escribir(",", false);
                self.pegado = false;
                self.separar = true;
            }
            Tipo::Asignacion => {
                self.escribir("=", true);
                self.pegado = false;
                self.separar = true;
            }
            Tipo::Menos => {
                let unario = match &self.ultimo {
                    None => true,
                    Some(anterior) => {
                        matches!(
                            anterior,
                            Tipo::LParen
                                | Tipo::LCorchete
                                | Tipo::LLave
                                | Tipo::Coma
                                | Tipo::Asignacion
                                | Tipo::Mas
                                | Tipo::Menos
                                | Tipo::Por
                                | Tipo::Entre
                                | Tipo::Resto
                                | Tipo::Igual
                                | Tipo::Distinto
                                | Tipo::Menor
                                | Tipo::MenorIgual
                                | Tipo::Mayor
                                | Tipo::MayorIgual
                                | Tipo::Y
                                | Tipo::O
                                | Tipo::No
                                | Tipo::Devolver
                                | Tipo::Si
                                | Tipo::Mientras
                                | Tipo::En
                                | Tipo::Sino
                        )
                    }
                };
                if unario {
                    let espacio = !self.inicio_linea() && !self.pegado;
                    self.escribir("-", espacio);
                    self.pegado = true;
                    self.separar = false;
                } else {
                    self.escribir("-", true);
                    self.pegado = false;
                    self.separar = true;
                }
            }
            Tipo::No => {
                let espacio = !self.pegado && !self.inicio_linea();
                self.escribir("no", espacio);
                self.pegado = false;
                self.separar = true;
            }
            tipo if es_operador_binario(tipo) => {
                self.escribir(&texto_token(tipo), true);
                self.pegado = false;
                self.separar = true;
            }
            tipo if es_palabra(tipo) => {
                let espacio = !self.pegado && !self.inicio_linea();
                self.escribir(&texto_token(tipo), espacio);
                self.pegado = false;
                self.separar = false;
            }
            otra => {
                let espacio = !self.pegado && !self.inicio_linea();
                self.escribir(&texto_token(otra), espacio);
                self.pegado = false;
                self.separar = false;
            }
        }
        self.recordar(tipo, linea);
    }

    fn recordar(&mut self, tipo: &Tipo, linea: usize) {
        self.ultimo = Some(tipo.clone());
        self.linea_ultimo = linea;
    }
}

fn lineas_cambiadas(original: &str, nuevo: &str) -> Vec<usize> {
    let antes: Vec<&str> = original.lines().collect();
    let despues: Vec<&str> = nuevo.lines().collect();
    let mut cambiadas = Vec::new();
    for i in 0..antes.len().max(despues.len()) {
        if antes.get(i).copied().unwrap_or("") != despues.get(i).copied().unwrap_or("") {
            cambiadas.push(i + 1);
        }
    }
    if cambiadas.is_empty() && original != nuevo {
        cambiadas.push(antes.len().max(1));
    }
    cambiadas
}

fn lista_de_lineas(lineas: &[usize]) -> String {
    match lineas {
        [] => String::new(),
        [una] => format!("la línea {}", una),
        [a, b] => format!("las líneas {} y {}", a, b),
        _ => {
            let (ultima, resto) = lineas.split_last().expect("la lista no está vacía");
            let numeros: Vec<String> = resto.iter().map(usize::to_string).collect();
            format!("las líneas {} y {}", numeros.join(", "), ultima)
        }
    }
}

/// `deimon formatear archivo.dmn [--comprobar]`
pub fn comando(argumentos: &[String]) -> Result<(), (String, i32)> {
    let uso = "Uso: deimon formatear archivo.dmn [--comprobar]";
    let mut archivo = None;
    let mut comprobar = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--comprobar" => comprobar = true,
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => {
                if archivo.is_some() {
                    return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2));
                }
                archivo = Some(otro.to_string());
            }
        }
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;
    let fuente = fs::read_to_string(&archivo).map_err(|error| {
        (
            format!("Error: no se pudo leer el archivo '{}': {}", archivo, crate::error::explicar_lectura(&error)),
            1,
        )
    })?;

    let errores = crate::comprobar_archivo(&fuente, Path::new(&archivo));
    if !errores.is_empty() {
        for error in &errores {
            println!("{}\n", error);
        }
        println!(
            "✗ {} tiene {} {}",
            archivo,
            errores.len(),
            if errores.len() == 1 { "error" } else { "errores" }
        );
        return Err((String::new(), 1));
    }

    let formateado = formatear(&fuente).map_err(|error| (error.to_string(), 1))?;
    let cambiadas = lineas_cambiadas(&fuente, &formateado);

    if comprobar {
        if cambiadas.is_empty() {
            println!("✓ {} ya está formateado", archivo);
            Ok(())
        } else {
            println!("✗ {} no está formateado", archivo);
            println!("Se cambiarían {}.", lista_de_lineas(&cambiadas));
            Err((String::new(), 1))
        }
    } else if cambiadas.is_empty() {
        println!("✓ {} ya estaba formateado", archivo);
        Ok(())
    } else {
        fs::write(&archivo, &formateado).map_err(|error| {
            (
                format!("Error: no se pudo escribir el archivo '{}': {}", archivo, error),
                1,
            )
        })?;
        println!("✓ {} formateado", archivo);
        Ok(())
    }
}
