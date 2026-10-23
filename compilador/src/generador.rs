use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Arreglo, ErrorTraduccion};
use crate::integradas;

/// Icono de Deimon para la pestaña del navegador (48×48), servido en /favicon.ico.
const ICONO_WEB: &[u8] = include_bytes!("../recursos/favicon.png");

const RUNTIME: &str = concat!(r##"
const PERFIL_LN: &str = "@PERFIL@";

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
enum Valor {
    Numero(f64),
    Texto(Rc<String>),
    Logico(bool),
    Nada,
    Lista(Rc<Vec<Valor>>),
    Mapa(Rc<MapaValores>),
}

/// Lista con claves (como los arrays con nombre de PHP). Recuerda el orden en que se
/// añadieron las claves; los números como clave se guardan como texto ("1").
#[derive(Debug, Clone, PartialEq, Default)]
struct MapaValores {
    claves: Vec<String>,
    valores: std::collections::HashMap<String, Valor>,
}

impl MapaValores {
    fn poner(&mut self, clave: String, valor: Valor) {
        if !self.valores.contains_key(&clave) {
            self.claves.push(clave.clone());
        }
        self.valores.insert(clave, valor);
    }

    fn quitar(&mut self, clave: &str) {
        if self.valores.remove(clave).is_some() {
            self.claves.retain(|c| c != clave);
        }
    }

    fn pares(&self) -> impl Iterator<Item = (&String, &Valor)> {
        self.claves.iter().map(move |c| (c, &self.valores[c]))
    }
}

impl Valor {
    fn tipo(&self) -> &'static str {
        match self {
            Valor::Numero(_) => "número",
            Valor::Texto(_) => "texto",
            Valor::Logico(_) => "lógico",
            Valor::Nada => "nada",
            Valor::Lista(_) => "lista",
            Valor::Mapa(_) => "lista con claves",
        }
    }

    /// Crea un texto a partir de algo que se pueda convertir en `String`
    /// (`&str`, `String`, el resultado de `to_uppercase()`...).
    #[allow(dead_code)]
    fn texto(contenido: impl Into<String>) -> Valor {
        Valor::Texto(Rc::new(contenido.into()))
    }

    /// Crea una lista a partir de un `Vec<Valor>`.
    #[allow(dead_code)]
    fn lista(elementos: Vec<Valor>) -> Valor {
        Valor::Lista(Rc::new(elementos))
    }

    /// Devuelve el texto que contiene el valor, si es un texto.
    #[allow(dead_code)]
    fn como_texto(&self) -> Option<&str> {
        match self {
            Valor::Texto(texto) => Some(texto.as_str()),
            _ => None,
        }
    }

    /// Devuelve los elementos del valor, si es una lista.
    #[allow(dead_code)]
    fn como_lista(&self) -> Option<&[Valor]> {
        match self {
            Valor::Lista(elementos) => Some(elementos.as_slice()),
            _ => None,
        }
    }

    /// Crea una lista con claves a partir de sus pares (clave, valor) en orden.
    #[allow(dead_code)]
    fn mapa(pares: Vec<(String, Valor)>) -> Valor {
        let mut mapa = MapaValores::default();
        for (clave, valor) in pares {
            mapa.poner(clave, valor);
        }
        Valor::Mapa(Rc::new(mapa))
    }

    /// Devuelve los pares (clave, valor) en orden, si es una lista con claves.
    #[allow(dead_code)]
    fn como_mapa(&self) -> Option<Vec<(String, Valor)>> {
        match self {
            Valor::Mapa(mapa) => Some(mapa.pares().map(|(c, v)| (c.clone(), v.clone())).collect()),
            _ => None,
        }
    }

    /// Devuelve el número que contiene el valor, si es un número.
    #[allow(dead_code)]
    fn como_numero(&self) -> Option<f64> {
        match self {
            Valor::Numero(numero) => Some(*numero),
            _ => None,
        }
    }
}

#[inline]
fn texto_de(valor: &Valor) -> String {
    match valor {
        Valor::Numero(n) => {
            if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Valor::Texto(s) => (**s).clone(),
        Valor::Logico(b) => {
            if *b {
                "verdadero".to_string()
            } else {
                "falso".to_string()
            }
        }
        Valor::Nada => "nada".to_string(),
        Valor::Lista(elementos) => {
            let textos: Vec<String> = elementos.iter().map(texto_de).collect();
            format!("[{}]", textos.join(", "))
        }
        Valor::Mapa(mapa) => {
            let textos: Vec<String> = mapa.pares().map(|(c, v)| format!("{}: {}", c, texto_de(v))).collect();
            format!("{{{}}}", textos.join(", "))
        }
    }
}

/// Clave de una lista con claves: un texto, o un número entero (que se guarda como texto).
fn clave_de_mapa(posicion: &Valor) -> String {
    match posicion {
        Valor::Texto(texto) => (**texto).clone(),
        Valor::Numero(n) if n.is_finite() && n.fract() == 0.0 => texto_de(posicion),
        otro => fallo(format!(
            "la clave debe ser un texto o un número entero, no {}",
            otro.tipo()
        )),
    }
}

/// Crea una lista con claves: `{"a": 1}`. Si una clave se repite, gana el último valor.
fn crear_mapa(pares: Vec<(Valor, Valor)>) -> Valor {
    let mut mapa = MapaValores::default();
    for (clave, valor) in pares {
        mapa.poner(clave_de_mapa(&clave), valor);
    }
    Valor::Mapa(Rc::new(mapa))
}

fn tiene_clave(mapa: &Valor, clave: &Valor) -> Valor {
    match mapa {
        Valor::Mapa(mapa) => Valor::Logico(mapa.valores.contains_key(&clave_de_mapa(clave))),
        otro => fallo(format!("tiene_clave() espera una lista con claves, no {}", otro.tipo())),
    }
}

fn quitar_clave(mapa: &Valor, clave: &Valor) -> Valor {
    match mapa {
        Valor::Mapa(mapa) => {
            let mut nuevo = Rc::clone(mapa);
            Rc::make_mut(&mut nuevo).quitar(&clave_de_mapa(clave));
            Valor::Mapa(nuevo)
        }
        otro => fallo(format!("quitar_clave() espera una lista con claves, no {}", otro.tipo())),
    }
}

/// Una lista vacía que recibe una clave de texto pasa a ser una lista con claves.
fn convertir_en_mapa_si_vacia(coleccion: &mut Valor, posicion: &Valor) {
    if let (Valor::Lista(elementos), Valor::Texto(_)) = (&*coleccion, posicion) {
        if elementos.is_empty() {
            *coleccion = Valor::Mapa(Rc::new(MapaValores::default()));
        }
    }
}

/// Lista con las claves de una lista con claves, en su orden.
fn claves_de_mapa(mapa: &MapaValores) -> Valor {
    Valor::Lista(Rc::new(mapa.claves.iter().map(|c| Valor::Texto(Rc::new(c.clone()))).collect()))
}

#[inline]
fn a_texto(valor: &Valor) -> Valor {
    Valor::Texto(Rc::new(texto_de(valor)))
}

thread_local! {
    static EN_RUTA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EN_PRUEBA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LINEA: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[inline]
fn en_linea(linea: usize) {
    LINEA.with(|actual| actual.set(linea));
}

#[inline]
fn con_linea(valor: Valor, linea: usize) -> Valor {
    en_linea(linea);
    valor
}

struct ErrorRuta(String);
struct FalloPrueba(String);

fn fallo(mensaje: String) -> ! {
    let en_prueba = EN_PRUEBA.with(|estado| estado.get());
    let linea = LINEA.with(|actual| actual.get());
    if !en_prueba {
        if linea > 0 {
            eprintln!("Error en la línea {} (en ejecución): {}", linea, mensaje);
        } else {
            eprintln!("Error en tiempo de ejecución: {}", mensaje);
        }
    }
    if en_prueba || EN_RUTA.with(|estado| estado.get()) {
        let detalle = if linea > 0 {
            format!("{} (línea {})", mensaje, linea)
        } else {
            mensaje
        };
        std::panic::panic_any(ErrorRuta(detalle));
    }
    std::process::exit(1);
}

#[inline]
fn texto_de_comprobacion(valor: &Valor) -> String {
    match valor {
        Valor::Texto(texto) => format!("{:?}", texto.as_str()),
        otro => texto_de(otro),
    }
}

fn fallar_comprobacion(linea: usize, mensaje: String) -> ! {
    if EN_PRUEBA.with(|estado| estado.get()) {
        std::panic::panic_any(FalloPrueba(format!("línea {}: {}", linea, mensaje)));
    }
    en_linea(linea);
    fallo(format!("línea {}: {}", linea, mensaje))
}

#[inline]
fn comprobar(condicion: &Valor, linea: usize) -> Valor {
    match condicion {
        Valor::Logico(true) => Valor::Nada,
        Valor::Logico(false) => fallar_comprobacion(linea, "la condición es falsa".to_string()),
        otro => fallo(format!(
            "comprobar() espera verdadero o falso, no {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn comprobar_igual(obtenido: &Valor, esperado: &Valor, linea: usize) -> Valor {
    if obtenido == esperado {
        Valor::Nada
    } else {
        fallar_comprobacion(
            linea,
            format!(
                "se esperaba {} y se obtuvo {}",
                texto_de_comprobacion(esperado),
                texto_de_comprobacion(obtenido)
            ),
        )
    }
}

fn en_una_linea(texto: &str) -> String {
    texto.replace(['\t', '\n', '\r'], " ")
}

fn ejecutar_pruebas(pruebas: Vec<(&'static str, fn() -> Valor)>) {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |informacion| {
        let carga = informacion.payload();
        if carga.downcast_ref::<ErrorRuta>().is_none()
            && carga.downcast_ref::<FalloPrueba>().is_none()
        {
            anterior(informacion);
        }
    }));
    let base = carpeta_datos();
    for (numero, (nombre, prueba)) in pruebas.into_iter().enumerate() {
        // Cada prueba empieza con datos vacíos y sin galletas (sin sesión abierta).
        *DATOS_DE_PRUEBA.lock().unwrap_or_else(|e| e.into_inner()) = Some(base.join(format!("prueba-{}", numero + 1)));
        GALLETAS_PRUEBA.with(|galletas| galletas.borrow_mut().clear());
        EN_PRUEBA.with(|estado| estado.set(true));
        en_linea(0);
        let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prueba();
        }));
        EN_PRUEBA.with(|estado| estado.set(false));
        match resultado {
            Ok(()) => println!("\u{1}LN-PRUEBA\tok\t{}\t", en_una_linea(nombre)),
            Err(causa) => {
                let detalle = if let Some(f) = causa.downcast_ref::<FalloPrueba>() {
                    f.0.clone()
                } else if let Some(e) = causa.downcast_ref::<ErrorRuta>() {
                    format!("error en ejecución: {}", e.0)
                } else {
                    "la prueba se detuvo de forma inesperada".to_string()
                };
                println!(
                    "\u{1}LN-PRUEBA\tfallo\t{}\t{}",
                    en_una_linea(nombre),
                    en_una_linea(&detalle)
                );
            }
        }
    }
}

#[inline]
fn mostrar(valor: &Valor) -> Valor {
    println!("{}", texto_de(valor));
    Valor::Nada
}

/// Como mostrar(), pero sin salto de línea al final (como echo en PHP).
fn escribir(valor: &Valor) -> Valor {
    use std::io::Write;
    let en_pagina = RESPUESTA.with(|respuesta| {
        let mut respuesta = respuesta.borrow_mut();
        if respuesta.capturando {
            respuesta.escrito.push_str(&texto_de(valor));
        }
        respuesta.capturando
    });
    if en_pagina {
        return Valor::Nada;
    }
    print!("{}", texto_de(valor));
    let _ = std::io::stdout().flush();
    Valor::Nada
}

#[inline]
fn a_logico(valor: &Valor) -> bool {
    match valor {
        Valor::Logico(b) => *b,
        otro => fallo(format!(
            "la condición debe ser verdadero o falso, no {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn negativo(valor: &Valor) -> Valor {
    match valor {
        Valor::Numero(n) => Valor::Numero(-n),
        otro => fallo(format!("no se puede negar {}", otro.tipo())),
    }
}

#[inline]
fn no_logico(valor: &Valor) -> Valor {
    Valor::Logico(!a_logico(valor))
}

#[inline]
fn suma(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        return Valor::Numero(x + y);
    }
    match (a, b) {
        (Valor::Lista(x), Valor::Lista(y)) => {
            let mut elementos = (**x).clone();
            elementos.extend(y.iter().cloned());
            Valor::Lista(Rc::new(elementos))
        }
        (Valor::Texto(_), _) | (_, Valor::Texto(_)) => {
            Valor::Texto(Rc::new(format!("{}{}", texto_de(a), texto_de(b))))
        }
        _ => fallo(format!(
            "no se puede sumar {} y {}",
            a.tipo(),
            b.tipo()
        )),
    }
}

#[inline]
fn suma_mover(a: Valor, b: &Valor) -> Valor {
    match a {
        Valor::Texto(mut texto) => {
            Rc::make_mut(&mut texto).push_str(&texto_de(b));
            Valor::Texto(texto)
        }
        Valor::Lista(mut elementos) => match b {
            Valor::Lista(otros) => {
                Rc::make_mut(&mut elementos).extend(otros.iter().cloned());
                Valor::Lista(elementos)
            }
            _ => suma(&Valor::Lista(elementos), b),
        },
        otro => suma(&otro, b),
    }
}

#[inline]
fn resta(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        return Valor::Numero(x - y);
    }
    fallo(format!(
        "no se puede restar {} y {}",
        a.tipo(),
        b.tipo()
    ))
}

#[inline]
fn multiplicacion(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        return Valor::Numero(x * y);
    }
    fallo(format!(
        "no se puede multiplicar {} y {}",
        a.tipo(),
        b.tipo()
    ))
}

#[inline]
fn division(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        if *y == 0.0 {
            fallo("no se puede dividir entre cero".to_string());
        }
        return Valor::Numero(x / y);
    }
    fallo(format!(
        "no se puede dividir {} y {}",
        a.tipo(),
        b.tipo()
    ))
}

/// División de la versión numérica de una función (mismo error que division()).
#[inline]
fn division_numerica(x: f64, y: f64, linea: usize) -> f64 {
    if y == 0.0 {
        en_linea(linea);
        fallo("no se puede dividir entre cero".to_string());
    }
    x / y
}

/// Resto de la versión numérica de una función (mismo error que resto()).
#[inline]
fn resto_numerico(x: f64, y: f64, linea: usize) -> f64 {
    if y == 0.0 {
        en_linea(linea);
        fallo("no se puede calcular el resto de una división entre cero".to_string());
    }
    x % y
}

#[inline]
fn resto(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        if *y == 0.0 {
            fallo("no se puede calcular el resto de una división entre cero".to_string());
        }
        return Valor::Numero(x % y);
    }
    fallo(format!(
        "no se puede calcular el resto de {} y {}",
        a.tipo(),
        b.tipo()
    ))
}

#[inline]
fn igual(a: &Valor, b: &Valor) -> Valor {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        return Valor::Logico(x == y);
    }
    match (a, b) {
        (Valor::Texto(x), Valor::Texto(y)) => Valor::Logico(x == y),
        (Valor::Logico(x), Valor::Logico(y)) => Valor::Logico(x == y),
        (Valor::Nada, Valor::Nada) => Valor::Logico(true),
        (Valor::Nada, _) | (_, Valor::Nada) => Valor::Logico(false),
        (Valor::Lista(x), Valor::Lista(y)) => Valor::Logico(x == y),
        (Valor::Mapa(x), Valor::Mapa(y)) => Valor::Logico(x == y),
        _ => fallo(format!(
            "no se pueden comparar {} y {}",
            a.tipo(),
            b.tipo()
        )),
    }
}

#[inline]
fn distinto(a: &Valor, b: &Valor) -> Valor {
    match igual(a, b) {
        Valor::Logico(v) => Valor::Logico(!v),
        _ => fallo("la comparación no produjo un valor lógico".to_string()),
    }
}

#[inline]
fn comparar_numeros(a: &Valor, b: &Valor, operacion: &str) -> (f64, f64) {
    match (a, b) {
        (Valor::Numero(x), Valor::Numero(y)) => (*x, *y),
        _ => fallo(format!(
            "no se pueden comparar {} y {} con {}",
            a.tipo(),
            b.tipo(),
            operacion
        )),
    }
}

#[inline]
fn menor(a: &Valor, b: &Valor) -> Valor {
    let (x, y) = comparar_numeros(a, b, "menor que");
    Valor::Logico(x < y)
}

#[inline]
fn menor_igual(a: &Valor, b: &Valor) -> Valor {
    let (x, y) = comparar_numeros(a, b, "menor o igual que");
    Valor::Logico(x <= y)
}

#[inline]
fn mayor(a: &Valor, b: &Valor) -> Valor {
    let (x, y) = comparar_numeros(a, b, "mayor que");
    Valor::Logico(x > y)
}

#[inline]
fn mayor_igual(a: &Valor, b: &Valor) -> Valor {
    let (x, y) = comparar_numeros(a, b, "mayor o igual que");
    Valor::Logico(x >= y)
}

#[inline]
fn a_numero(valor: &Valor) -> Valor {
    match valor {
        Valor::Numero(n) => Valor::Numero(*n),
        Valor::Texto(texto) => match texto.trim().parse::<f64>() {
            Ok(n) if n.is_finite() => Valor::Numero(n),
            _ => fallo(format!("'{}' no es un número válido", texto)),
        },
        otro => fallo(format!("a_numero() espera un texto, no {}", otro.tipo())),
    }
}

#[inline]
fn indice_entero(posicion: &Valor) -> f64 {
    match posicion {
        Valor::Numero(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                fallo(format!(
                    "el índice debe ser un número entero, no {}",
                    texto_de(&Valor::Numero(*n))
                ));
            }
            *n
        }
        otro => fallo(format!(
            "el índice debe ser un número entero, no {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn indice(coleccion: &Valor, posicion: &Valor) -> Valor {
    if let Valor::Mapa(mapa) = coleccion {
        // Una clave que no existe da nada (así se puede comprobar con '!= nada').
        return mapa.valores.get(&clave_de_mapa(posicion)).cloned().unwrap_or(Valor::Nada);
    }
    if let (Valor::Lista(elementos), Valor::Texto(_)) = (coleccion, posicion) {
        if elementos.is_empty() {
            return Valor::Nada; // una lista vacía es también una lista con claves vacía
        }
    }
    let numero = indice_entero(posicion);
    match coleccion {
        Valor::Lista(elementos) => {
            if numero < 0.0 || numero as usize >= elementos.len() {
                fallo(format!(
                    "el índice {} está fuera de rango para una lista de {} elementos",
                    texto_de(&Valor::Numero(numero)),
                    elementos.len()
                ));
            }
            elementos[numero as usize].clone()
        }
        Valor::Texto(texto) => {
            let total = texto.chars().count();
            if numero < 0.0 || numero as usize >= total {
                fallo(format!(
                    "el índice {} está fuera de rango para un texto de {} caracteres",
                    texto_de(&Valor::Numero(numero)),
                    total
                ));
            }
            let caracter = texto.chars().nth(numero as usize).unwrap_or('\0');
            Valor::Texto(Rc::new(caracter.to_string()))
        }
        otro => fallo(format!(
            "solo se puede acceder con corchetes a una lista o a un texto, no a {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn poner_indice(coleccion: &mut Valor, posicion: &Valor, valor: Valor) {
    convertir_en_mapa_si_vacia(coleccion, posicion);
    if let Valor::Mapa(mapa) = coleccion {
        Rc::make_mut(mapa).poner(clave_de_mapa(posicion), valor);
        return;
    }
    let numero = indice_entero(posicion);
    match coleccion {
        Valor::Lista(elementos) => {
            let longitud = elementos.len();
            if numero < 0.0 || numero as usize >= longitud {
                fallo(format!(
                    "el índice {} está fuera de rango para una lista de {} elementos",
                    texto_de(&Valor::Numero(numero)),
                    longitud
                ));
            }
            Rc::make_mut(elementos)[numero as usize] = valor;
        }
        Valor::Texto(_) => fallo(
            "los textos no se pueden cambiar por partes; crea uno nuevo".to_string(),
        ),
        otro => fallo(format!(
            "solo se puede asignar con corchetes a una lista, no a {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn indice_mut<'a>(coleccion: &'a mut Valor, posicion: &Valor) -> &'a mut Valor {
    convertir_en_mapa_si_vacia(coleccion, posicion);
    let numero = if matches!(coleccion, Valor::Mapa(_)) { 0.0 } else { indice_entero(posicion) };
    match coleccion {
        Valor::Mapa(mapa) => {
            let clave = clave_de_mapa(posicion);
            let mapa = Rc::make_mut(mapa);
            if !mapa.valores.contains_key(&clave) {
                mapa.poner(clave.clone(), Valor::Nada);
            }
            mapa.valores.get_mut(&clave).expect("la clave se acaba de poner")
        }
        Valor::Lista(elementos) => {
            let longitud = elementos.len();
            if numero < 0.0 || numero as usize >= longitud {
                fallo(format!(
                    "el índice {} está fuera de rango para una lista de {} elementos",
                    texto_de(&Valor::Numero(numero)),
                    longitud
                ));
            }
            &mut Rc::make_mut(elementos)[numero as usize]
        }
        Valor::Texto(_) => fallo(
            "los textos no se pueden cambiar por partes; crea uno nuevo".to_string(),
        ),
        otro => fallo(format!(
            "solo se puede asignar con corchetes a una lista, no a {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn medir(valor: &Valor) -> Valor {
    match valor {
        Valor::Lista(elementos) => Valor::Numero(elementos.len() as f64),
        Valor::Mapa(mapa) => Valor::Numero(mapa.claves.len() as f64),
        Valor::Texto(texto) => Valor::Numero(texto.chars().count() as f64),
        otro => fallo(format!(
            "medir() espera una lista o un texto, no {}",
            otro.tipo()
        )),
    }
}

/// Dice si una lista (o los valores de una lista con claves) tiene ese valor,
/// o si un texto contiene otro texto.
fn incluye(coleccion: &Valor, buscado: &Valor) -> Valor {
    let mismo = |a: &Valor| match (a, buscado) {
        (Valor::Numero(x), Valor::Numero(y)) => x == y,
        _ => a == buscado,
    };
    match coleccion {
        Valor::Lista(elementos) => Valor::Logico(elementos.iter().any(mismo)),
        Valor::Mapa(mapa) => Valor::Logico(mapa.valores.values().any(mismo)),
        Valor::Texto(texto) => match buscado {
            Valor::Texto(parte) => Valor::Logico(texto.contains(parte.as_str())),
            otro => fallo(format!("incluye() en un texto busca otro texto, no {}", otro.tipo())),
        },
        otro => fallo(format!("incluye() espera una lista o un texto, no {}", otro.tipo())),
    }
}

/// Lista nueva ordenada de menor a mayor: todo números o todo textos.
fn ordenar(lista: &Valor) -> Valor {
    let elementos = match lista {
        Valor::Lista(elementos) => elementos,
        otro => fallo(format!("ordenar() espera una lista, no {}", otro.tipo())),
    };
    let mut nuevos: Vec<Valor> = elementos.iter().cloned().collect();
    if nuevos.iter().all(|v| matches!(v, Valor::Numero(_))) {
        nuevos.sort_by(|a, b| match (a, b) {
            (Valor::Numero(x), Valor::Numero(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        });
    } else if nuevos.iter().all(|v| matches!(v, Valor::Texto(_))) {
        nuevos.sort_by(|a, b| texto_de(a).cmp(&texto_de(b)));
    } else {
        fallo("ordenar() necesita una lista solo de números o solo de textos".to_string());
    }
    Valor::Lista(Rc::new(nuevos))
}

/// Lista o texto al revés.
fn invertir(valor: &Valor) -> Valor {
    match valor {
        Valor::Lista(elementos) => Valor::Lista(Rc::new(elementos.iter().rev().cloned().collect())),
        Valor::Texto(texto) => Valor::Texto(Rc::new(texto.chars().rev().collect())),
        otro => fallo(format!("invertir() espera una lista o un texto, no {}", otro.tipo())),
    }
}

/// Trozo de una lista o de un texto: desde `inicio` (negativo cuenta desde el final),
/// `cantidad` elementos (nada = hasta el final; negativo = deja fuera esos del final).
fn cortar(valor: &Valor, inicio: &Valor, cantidad: &Valor) -> Valor {
    let total = match valor {
        Valor::Lista(elementos) => elementos.len() as i64,
        Valor::Texto(texto) => texto.chars().count() as i64,
        otro => fallo(format!("cortar() espera una lista o un texto, no {}", otro.tipo())),
    };
    let entero = |v: &Valor, que: &str| match v {
        Valor::Numero(n) if n.is_finite() && n.fract() == 0.0 => *n as i64,
        otro => fallo(format!("cortar() espera un número entero como {}, no {}", que, texto_de(otro))),
    };
    let mut desde = entero(inicio, "inicio");
    if desde < 0 {
        desde = (total + desde).max(0);
    }
    let desde = desde.min(total);
    let hasta = match cantidad {
        Valor::Nada => total,
        otro => {
            let n = entero(otro, "cantidad");
            if n < 0 { (total + n).max(desde) } else { (desde + n).min(total) }
        }
    };
    let (desde, hasta) = (desde as usize, hasta as usize);
    match valor {
        Valor::Lista(elementos) => Valor::Lista(Rc::new(elementos[desde..hasta].to_vec())),
        Valor::Texto(texto) => Valor::Texto(Rc::new(texto.chars().skip(desde).take(hasta - desde).collect())),
        _ => unreachable!(),
    }
}

/// Lista de números enteros desde `inicio` hasta `fin`, los dos incluidos (al revés si fin < inicio).
fn rango(inicio: &Valor, fin: &Valor) -> Valor {
    let entero = |v: &Valor| match v {
        Valor::Numero(n) if n.is_finite() && n.fract() == 0.0 => *n as i64,
        otro => fallo(format!("rango() espera números enteros, no {}", texto_de(otro))),
    };
    let (a, b) = (entero(inicio), entero(fin));
    if (a - b).abs() > 10_000_000 {
        fallo("rango() no puede crear más de 10 millones de números".to_string());
    }
    let numeros: Vec<Valor> = if a <= b {
        (a..=b).map(|n| Valor::Numero(n as f64)).collect()
    } else {
        (b..=a).rev().map(|n| Valor::Numero(n as f64)).collect()
    };
    Valor::Lista(Rc::new(numeros))
}

/// Suma los números de una lista.
fn sumar_lista(lista: &Valor) -> Valor {
    match lista {
        Valor::Lista(elementos) => Valor::Numero(elementos.iter().map(|v| match v {
            Valor::Numero(n) => *n,
            otro => fallo(format!("sumar_lista() solo suma números, y la lista tiene {}", otro.tipo())),
        }).sum()),
        otro => fallo(format!("sumar_lista() espera una lista, no {}", otro.tipo())),
    }
}

#[inline]
fn agregar(lista: &Valor, valor: &Valor) -> Valor {
    match lista {
        Valor::Lista(elementos) => {
            let mut nuevos = Rc::clone(elementos);
            Rc::make_mut(&mut nuevos).push(valor.clone());
            Valor::Lista(nuevos)
        }
        otro => fallo(format!(
            "agregar() espera una lista como primer argumento, no {}",
            otro.tipo()
        )),
    }
}

#[inline]
fn agregar_mover(lista: Valor, valor: Valor) -> Valor {
    match lista {
        Valor::Lista(mut elementos) => {
            Rc::make_mut(&mut elementos).push(valor);
            Valor::Lista(elementos)
        }
        otro => fallo(format!(
            "agregar() espera una lista como primer argumento, no {}",
            otro.tipo()
        )),
    }
}

// ---------------------------------------------------------------------------
// Datos permanentes: un registro por colección (`<colección>.registro`) en el que cada
// cambio se AÑADE al final y se fuerza al disco. Cada línea lleva una suma de control:
// una línea a medias por un corte de luz se descarta sin perder lo anterior. Un candado
// de archivo coordina a varios programas que usen la misma carpeta, y una copia en
// memoria hace que leer no tenga que releer todo el archivo.
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Dato {
    Numero(f64),
    Texto(String),
    Logico(bool),
    Nada,
    Lista(Vec<Dato>),
    Mapa(Vec<(String, Dato)>),
}

impl Dato {
    fn desde(valor: &Valor) -> Dato {
        match valor {
            Valor::Numero(numero) => Dato::Numero(*numero),
            Valor::Texto(texto) => Dato::Texto((**texto).clone()),
            Valor::Logico(valor) => Dato::Logico(*valor),
            Valor::Nada => Dato::Nada,
            Valor::Lista(elementos) => Dato::Lista(elementos.iter().map(Dato::desde).collect()),
            Valor::Mapa(mapa) => Dato::Mapa(mapa.pares().map(|(c, v)| (c.clone(), Dato::desde(v))).collect()),
        }
    }

    fn a_valor(&self) -> Valor {
        match self {
            Dato::Numero(numero) => Valor::Numero(*numero),
            Dato::Texto(texto) => Valor::Texto(Rc::new(texto.clone())),
            Dato::Logico(valor) => Valor::Logico(*valor),
            Dato::Nada => Valor::Nada,
            Dato::Lista(elementos) => Valor::Lista(Rc::new(elementos.iter().map(Dato::a_valor).collect())),
            Dato::Mapa(pares) => Valor::mapa(pares.iter().map(|(c, d)| (c.clone(), d.a_valor())).collect()),
        }
    }

    fn codificar(&self) -> (char, String) {
        match self {
            Dato::Numero(numero) => ('n', texto_de(&Valor::Numero(*numero))),
            Dato::Texto(texto) => ('t', texto.clone()),
            Dato::Logico(valor) => ('b', if *valor { "verdadero" } else { "falso" }.to_string()),
            Dato::Nada => ('z', String::new()),
            // Una lista: cada elemento es <tipo><bytes>:<contenido>, así se pueden anidar.
            Dato::Lista(elementos) => ('l', elementos
                .iter()
                .map(|elemento| {
                    let (tipo, texto) = elemento.codificar();
                    format!("{}{}:{}", tipo, texto.len(), texto)
                })
                .collect()),
            // Una lista con claves: clave y valor alternados, con el mismo formato.
            Dato::Mapa(pares) => ('m', pares
                .iter()
                .map(|(clave, valor)| {
                    let (tipo, texto) = valor.codificar();
                    format!("t{}:{}{}{}:{}", clave.len(), clave, tipo, texto.len(), texto)
                })
                .collect()),
        }
    }

    fn decodificar(tipo: &str, texto: String) -> Option<Dato> {
        if tipo == "l" || tipo == "m" {
            let mut elementos = Vec::new();
            let mut resto = texto.as_str();
            while !resto.is_empty() {
                let tipo = resto.get(..1)?;
                let (largo, despues) = resto[1..].split_once(':')?;
                let largo: usize = largo.parse().ok()?;
                let contenido = despues.get(..largo)?;
                elementos.push(Dato::decodificar(tipo, contenido.to_string())?);
                resto = &despues[largo..];
            }
            if tipo == "m" {
                if elementos.len() % 2 != 0 {
                    return None;
                }
                let mut pares = Vec::new();
                let mut piezas = elementos.into_iter();
                while let (Some(Dato::Texto(clave)), Some(valor)) = (piezas.next(), piezas.next()) {
                    pares.push((clave, valor));
                }
                return Some(Dato::Mapa(pares));
            }
            return Some(Dato::Lista(elementos));
        }
        match tipo {
            "n" => texto.parse::<f64>().ok().filter(|n| n.is_finite()).map(Dato::Numero),
            "t" => Some(Dato::Texto(texto)),
            "b" => match texto.as_str() {
                "verdadero" => Some(Dato::Logico(true)),
                "falso" => Some(Dato::Logico(false)),
                _ => None,
            },
            "z" => Some(Dato::Nada),
            _ => None,
        }
    }
}

struct Coleccion {
    /// Generación del registro (cambia al compactarlo) y bytes ya aplicados.
    generacion: String,
    leido: u64,
    lineas: usize,
    datos: BTreeMap<String, Dato>,
}

static DATOS_CANDADO: Mutex<Option<std::collections::HashMap<PathBuf, Coleccion>>> = Mutex::new(None);
static DATOS_SECUENCIA: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Durante `deimon probar`, cada prueba usa su propia carpeta de datos (empieza vacía).
static DATOS_DE_PRUEBA: Mutex<Option<PathBuf>> = Mutex::new(None);

fn carpeta_datos() -> PathBuf {
    if let Some(carpeta) = DATOS_DE_PRUEBA.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return carpeta;
    }
    match std::env::var("DEIMON_DATOS") {
        Ok(carpeta) if !carpeta.is_empty() => PathBuf::from(carpeta),
        _ => PathBuf::from("./datos"),
    }
}

fn nombre_coleccion(valor: &Valor) -> String {
    let nombre = match valor {
        Valor::Texto(texto) => (**texto).clone(),
        otro => fallo(format!(
            "el nombre de la colección debe ser un texto, no {}",
            otro.tipo()
        )),
    };
    let valido = !nombre.is_empty()
        && nombre.chars().count() <= 64
        && nombre
            .chars()
            .all(|caracter| caracter.is_ascii_alphanumeric() || caracter == '_' || caracter == '-');
    if !valido {
        fallo(
            "el nombre de la colección solo puede tener letras, dígitos, '_' y '-', y medir entre 1 y 64 caracteres"
                .to_string(),
        );
    }
    nombre
}

fn clave_texto(valor: &Valor) -> String {
    match valor {
        Valor::Texto(texto) => (**texto).clone(),
        otro => fallo(format!(
            "la clave debe ser un texto, no {}",
            otro.tipo()
        )),
    }
}

fn escapar_dato(texto: &str) -> String {
    let mut salida = String::new();
    for caracter in texto.chars() {
        match caracter {
            '\\' => salida.push_str("\\\\"),
            '\t' => salida.push_str("\\t"),
            '\n' => salida.push_str("\\n"),
            '\r' => salida.push_str("\\r"),
            otro => salida.push(otro),
        }
    }
    salida
}

fn desescapar_dato(texto: &str) -> Result<String, ()> {
    let mut salida = String::new();
    let mut caracteres = texto.chars();
    while let Some(caracter) = caracteres.next() {
        if caracter == '\\' {
            match caracteres.next() {
                Some('\\') => salida.push('\\'),
                Some('t') => salida.push('\t'),
                Some('n') => salida.push('\n'),
                Some('r') => salida.push('\r'),
                _ => return Err(()),
            }
        } else {
            salida.push(caracter);
        }
    }
    Ok(salida)
}

/// Suma de control FNV-1a de 32 bits de una línea del registro.
fn suma_control(texto: &str) -> String {
    let mut suma: u32 = 0x811c_9dc5;
    for byte in texto.bytes() {
        suma ^= byte as u32;
        suma = suma.wrapping_mul(0x0100_0193);
    }
    format!("{:08x}", suma)
}

fn linea_registro(operacion: char, clave: &str, dato: Option<&Dato>) -> String {
    let (tipo, texto) = match dato {
        Some(dato) => dato.codificar(),
        None => ('-', String::new()),
    };
    let cuerpo = format!(
        "{}\t{}\t{}\t{}",
        operacion,
        escapar_dato(clave),
        tipo,
        escapar_dato(&texto)
    );
    format!("{}\t{}\n", cuerpo, suma_control(&cuerpo))
}

/// Aplica una línea del registro. `None` si la línea está dañada.
fn aplicar_linea(linea: &str, datos: &mut BTreeMap<String, Dato>) -> Option<()> {
    let (cuerpo, suma) = linea.rsplit_once('\t')?;
    if suma_control(cuerpo) != suma {
        return None;
    }
    let partes: Vec<&str> = cuerpo.splitn(4, '\t').collect();
    if partes.len() != 4 {
        return None;
    }
    let clave = desescapar_dato(partes[1]).ok()?;
    match partes[0] {
        "s" => {
            let texto = desescapar_dato(partes[3]).ok()?;
            datos.insert(clave, Dato::decodificar(partes[2], texto)?);
        }
        "b" => {
            datos.remove(&clave);
        }
        _ => return None,
    }
    Some(())
}

fn ruta_registro(carpeta: &Path, coleccion: &str) -> PathBuf {
    carpeta.join(format!("{}.registro", coleccion))
}

fn error_datos(coleccion: &str, detalle: String) -> ! {
    fallo(format!("colección '{}': {}", coleccion, detalle))
}

/// Ejecuta `operacion` con la colección al día, con el candado del proceso y el del archivo.
fn con_coleccion<T>(
    coleccion: &str,
    operacion: impl FnOnce(&mut Coleccion, &Path, &mut std::fs::File) -> T,
) -> T {
    use std::io::{Read, Seek, SeekFrom, Write};
    let carpeta = carpeta_datos();
    if let Err(error) = std::fs::create_dir_all(&carpeta) {
        fallo(format!(
            "no se pudo crear la carpeta de datos '{}': {}",
            carpeta.display(),
            error
        ));
    }
    let mut cache = DATOS_CANDADO
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let cache = cache.get_or_insert_with(std::collections::HashMap::new);
    let candado = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(carpeta.join(".candado"))
    {
        Ok(archivo) => archivo,
        Err(error) => error_datos(coleccion, format!("no se pudo abrir el candado: {}", error)),
    };
    let _ = candado.lock();

    let ruta = ruta_registro(&carpeta, coleccion);
    migrar_formato_antiguo(&carpeta, coleccion, &ruta);
    let mut archivo = match std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&ruta)
    {
        Ok(archivo) => archivo,
        Err(error) => error_datos(coleccion, format!("no se pudo abrir: {}", error)),
    };
    let mut contenido = String::new();
    let _ = archivo.seek(SeekFrom::Start(0));
    let mut primera = String::new();
    {
        let mut bytes = [0u8; 64];
        let leidos = archivo.read(&mut bytes).unwrap_or(0);
        primera.push_str(&String::from_utf8_lossy(&bytes[..leidos]));
    }
    let generacion = primera.lines().next().unwrap_or("").to_string();
    if !generacion.starts_with("#deimon-registro ") {
        // Registro nuevo (o vacío): se le pone cabecera con una generación nueva.
        let nueva = format!(
            "#deimon-registro {}-{}\n",
            std::process::id(),
            DATOS_SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        if archivo.set_len(0).is_err() || archivo.write_all(nueva.as_bytes()).is_err() {
            error_datos(coleccion, "no se pudo preparar el registro".to_string());
        }
        let _ = archivo.sync_data();
        cache.remove(&ruta);
        return con_coleccion_preparada(cache, coleccion, &ruta, archivo, operacion);
    }
    let largo = archivo.metadata().map(|m| m.len()).unwrap_or(0);
    let al_dia = matches!(cache.get(&ruta), Some(c) if c.generacion == generacion && c.leido <= largo);
    if !al_dia {
        cache.remove(&ruta);
    }
    let desde = cache.get(&ruta).map(|c| c.leido).unwrap_or(0);
    if desde < largo {
        let _ = archivo.seek(SeekFrom::Start(desde));
        let mut nuevo = Vec::new();
        if archivo.read_to_end(&mut nuevo).is_err() {
            error_datos(coleccion, "no se pudo leer".to_string());
        }
        contenido.push_str(&String::from_utf8_lossy(&nuevo));
    }
    let entrada = cache.entry(ruta.clone()).or_insert_with(|| Coleccion {
        generacion: generacion.clone(),
        leido: 0,
        lineas: 0,
        datos: BTreeMap::new(),
    });
    // Solo se aplican las líneas completas; una última línea a medias (corte de luz) se
    // descarta y se recorta del archivo para que la siguiente escritura empiece limpia.
    let completo = match contenido.rfind('\n') {
        Some(fin) => fin + 1,
        None => 0,
    };
    let mut aplicado = 0usize;
    for linea in contenido[..completo].lines() {
        aplicado += linea.len() + 1;
        if linea.starts_with('#') || linea.is_empty() {
            continue;
        }
        if aplicar_linea(linea, &mut entrada.datos).is_none() {
            let resto_vacio = contenido[aplicado..completo].trim().is_empty();
            if resto_vacio {
                // Última línea dañada: se trata igual que una escritura a medias.
                aplicado -= linea.len() + 1;
                break;
            }
            error_datos(coleccion, "el registro está dañado a mitad del archivo".to_string());
        }
        entrada.lineas += 1;
    }
    entrada.leido = desde + aplicado as u64;
    if entrada.leido < largo {
        let _ = archivo.set_len(entrada.leido);
    }
    con_coleccion_preparada(cache, coleccion, &ruta, archivo, operacion)
}

fn con_coleccion_preparada<T>(
    cache: &mut std::collections::HashMap<PathBuf, Coleccion>,
    coleccion: &str,
    ruta: &Path,
    mut archivo: std::fs::File,
    operacion: impl FnOnce(&mut Coleccion, &Path, &mut std::fs::File) -> T,
) -> T {
    if !cache.contains_key(ruta) {
        // Se acaba de crear la cabecera: se vuelve a leer para tener su generación.
        use std::io::{Read, Seek, SeekFrom};
        let _ = archivo.seek(SeekFrom::Start(0));
        let mut texto = String::new();
        let _ = archivo.read_to_string(&mut texto);
        cache.insert(
            ruta.to_path_buf(),
            Coleccion {
                generacion: texto.lines().next().unwrap_or("").to_string(),
                leido: texto.len() as u64,
                lineas: 0,
                datos: BTreeMap::new(),
            },
        );
    }
    let entrada = cache.get_mut(ruta).expect("colección en memoria");
    let _ = coleccion;
    operacion(entrada, ruta, &mut archivo)
}

/// Añade una línea al registro y la fuerza al disco antes de dar el cambio por hecho.
fn anotar(entrada: &mut Coleccion, archivo: &mut std::fs::File, coleccion: &str, linea: &str) {
    use std::io::Write;
    if archivo.write_all(linea.as_bytes()).is_err() || archivo.sync_data().is_err() {
        error_datos(coleccion, "no se pudo escribir en el disco".to_string());
    }
    entrada.leido += linea.len() as u64;
    entrada.lineas += 1;
}

/// Si el registro tiene muchas más líneas que datos vivos, se reescribe compacto.
fn compactar_si_hace_falta(entrada: &mut Coleccion, ruta: &Path, coleccion: &str) {
    if entrada.lineas < 1000 || entrada.lineas < entrada.datos.len() * 2 {
        return;
    }
    let generacion = format!(
        "#deimon-registro {}-{}-c",
        std::process::id(),
        DATOS_SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let mut contenido = format!("{}\n", generacion);
    for (clave, dato) in &entrada.datos {
        contenido.push_str(&linea_registro('s', clave, Some(dato)));
    }
    let temporal = ruta.with_extension(format!("compactando-{}", std::process::id()));
    let escrito = std::fs::File::create(&temporal).and_then(|mut archivo| {
        use std::io::Write;
        archivo.write_all(contenido.as_bytes())?;
        archivo.sync_all()
    });
    if escrito.is_err() || std::fs::rename(&temporal, ruta).is_err() {
        let _ = std::fs::remove_file(&temporal);
        error_datos(coleccion, "no se pudo compactar el registro".to_string());
    }
    entrada.generacion = generacion;
    entrada.leido = contenido.len() as u64;
    entrada.lineas = entrada.datos.len();
}

/// Importa una colección del formato antiguo (`<colección>.datos`) la primera vez.
fn migrar_formato_antiguo(carpeta: &Path, coleccion: &str, registro: &Path) {
    let antiguo = carpeta.join(format!("{}.datos", coleccion));
    if registro.exists() || !antiguo.is_file() {
        return;
    }
    let texto = std::fs::read_to_string(&antiguo).unwrap_or_default();
    let mut contenido = format!("#deimon-registro migrado-{}\n", std::process::id());
    for linea in texto.lines().filter(|linea| !linea.is_empty()) {
        let partes: Vec<&str> = linea.splitn(3, '\t').collect();
        if partes.len() != 3 {
            continue;
        }
        if let (Ok(clave), Ok(valor)) = (desescapar_dato(partes[0]), desescapar_dato(partes[2])) {
            if let Some(dato) = Dato::decodificar(partes[1], valor) {
                contenido.push_str(&linea_registro('s', &clave, Some(&dato)));
            }
        }
    }
    if std::fs::write(registro, contenido).is_ok() {
        let _ = std::fs::rename(&antiguo, carpeta.join(format!("{}.datos.migrado", coleccion)));
    }
}

fn guardar(coleccion: &Valor, clave: &Valor, valor: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let clave = clave_texto(clave);
    let dato = Dato::desde(valor);
    guardar_dato(&coleccion, &clave, dato);
    Valor::Nada
}

fn guardar_dato(coleccion: &str, clave: &str, dato: Dato) {
    con_coleccion(coleccion, |entrada, ruta, archivo| {
        anotar(entrada, archivo, coleccion, &linea_registro('s', clave, Some(&dato)));
        entrada.datos.insert(clave.to_string(), dato);
        compactar_si_hace_falta(entrada, ruta, coleccion);
    });
}

fn leer_dato(coleccion: &str, clave: &str) -> Option<Dato> {
    con_coleccion(coleccion, |entrada, _, _| entrada.datos.get(clave).cloned())
}

fn leer(coleccion: &Valor, clave: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let clave = clave_texto(clave);
    leer_dato(&coleccion, &clave)
        .map(|dato| dato.a_valor())
        .unwrap_or(Valor::Nada)
}

fn borrar(coleccion: &Valor, clave: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let clave = clave_texto(clave);
    con_coleccion(&coleccion, |entrada, ruta, archivo| {
        if entrada.datos.remove(&clave).is_some() {
            anotar(entrada, archivo, &coleccion, &linea_registro('b', &clave, None));
            compactar_si_hace_falta(entrada, ruta, &coleccion);
        }
    });
    Valor::Nada
}

fn contar(coleccion: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let total = con_coleccion(&coleccion, |entrada, _, _| entrada.datos.len());
    Valor::Numero(total as f64)
}

fn listar_claves(coleccion: &Valor) -> Valor {
    match coleccion {
        Valor::Mapa(mapa) => return claves_de_mapa(mapa),
        // Las claves de una lista son sus posiciones: 0, 1, 2...
        Valor::Lista(elementos) => {
            return Valor::Lista(Rc::new((0..elementos.len()).map(|i| Valor::Numero(i as f64)).collect()))
        }
        _ => {}
    }
    let coleccion = nombre_coleccion(coleccion);
    let claves: Vec<String> = con_coleccion(&coleccion, |entrada, _, _| {
        entrada.datos.keys().cloned().collect()
    });
    Valor::Lista(Rc::new(
        claves
            .into_iter()
            .map(|clave| Valor::Texto(Rc::new(clave)))
            .collect(),
    ))
}

struct Ruta {
    camino: String,
    manejador: fn(Vec<String>) -> Valor,
}

fn escapar_html(texto: &str) -> String {
    let mut salida = String::new();
    for caracter in texto.chars() {
        match caracter {
            '&' => salida.push_str("&amp;"),
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#39;"),
            otro => salida.push(otro),
        }
    }
    salida
}

fn valor_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decodificar(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut salida = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(alto), Some(bajo)) =
                (valor_hex(bytes[i + 1]), valor_hex(bytes[i + 2]))
            {
                salida.push(alto * 16 + bajo);
                i += 3;
                continue;
            }
        }
        salida.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&salida).into_owned()
}

fn buscar_ruta<'a>(rutas: &'a [Ruta], camino: &str) -> Option<(&'a Ruta, Vec<String>)> {
    let partes: Vec<String> = camino
        .split('/')
        .filter(|parte| !parte.is_empty())
        .map(decodificar)
        .collect();
    for ruta in rutas {
        let patron: Vec<&str> = ruta
            .camino
            .split('/')
            .filter(|parte| !parte.is_empty())
            .collect();
        if patron.len() != partes.len() {
            continue;
        }
        let mut parametros = Vec::new();
        let mut coincide = true;
        for (esperado, recibido) in patron.iter().zip(partes.iter()) {
            if let Some(nombre) = esperado.strip_prefix(':') {
                if nombre.is_empty() {
                    coincide = false;
                    break;
                }
                parametros.push(escapar_html(recibido));
            } else if *esperado != recibido.as_str() {
                coincide = false;
                break;
            }
        }
        if coincide {
            return Some((ruta, parametros));
        }
    }
    None
}

const PAGINA_404: &str = "<h1>Página no encontrada</h1>";

/// Icono de Deimon que ve el navegador en la pestaña si el programa no define otro.
const ICONO_WEB: &[u8] = &[@ICONO_WEB@];

fn visitar(camino: &Valor, rutas: Vec<Ruta>) -> Valor {
    let camino = texto_argumento(camino, "visitar");
    atender_sin_red(&camino, false, Vec::new(), rutas)
}

// ---------------------------------------------------------------------------
// Petición y respuesta de la ruta que se está atendiendo (formularios y sesiones).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Peticion {
    /// Llegó por HTTPS a través de Caddy (cabecera X-Forwarded-Proto: https).
    segura: bool,
    envio: bool,
    campos: Vec<(String, String)>,
    galletas: Vec<(String, String)>,
}

#[derive(Default)]
struct Respuesta {
    cabeceras: Vec<String>,
    redireccion: Option<String>,
    /// Mientras se atiende una ruta, lo que escribe escribir() va a la página (como echo).
    capturando: bool,
    escrito: String,
}

/// Cuerpo de la página: lo escrito con escribir() más lo que devuelve la ruta.
fn cuerpo_pagina(escrito: &str, valor: &Valor) -> String {
    match valor {
        Valor::Nada => escrito.to_string(),
        otro => format!("{}{}", escrito, texto_de(otro)),
    }
}

thread_local! {
    static PETICION: std::cell::RefCell<Peticion> = std::cell::RefCell::new(Peticion::default());
    static RESPUESTA: std::cell::RefCell<Respuesta> = std::cell::RefCell::new(Respuesta::default());
    /// Galletas (cookies) que guardan `visitar` y `enviar` en las pruebas, como un navegador.
    static GALLETAS_PRUEBA: std::cell::RefCell<Vec<(String, String)>> = std::cell::RefCell::new(Vec::new());
}

/// Lee `a=1&b=dos+palabras` como lo envía un formulario.
fn decodificar_formulario(texto: &str) -> Vec<(String, String)> {
    texto
        .split('&')
        .filter(|parte| !parte.is_empty())
        .map(|parte| {
            let (nombre, valor) = parte.split_once('=').unwrap_or((parte, ""));
            (
                decodificar(&nombre.replace('+', " ")),
                decodificar(&valor.replace('+', " ")),
            )
        })
        .collect()
}

fn leer_galletas(cabecera: &str) -> Vec<(String, String)> {
    cabecera
        .split(';')
        .filter_map(|parte| {
            let (nombre, valor) = parte.trim().split_once('=')?;
            Some((nombre.trim().to_string(), valor.trim().to_string()))
        })
        .collect()
}

fn galleta(nombre: &str) -> Option<String> {
    PETICION.with(|peticion| {
        peticion
            .borrow()
            .galletas
            .iter()
            .find(|(clave, _)| clave == nombre)
            .map(|(_, valor)| valor.clone())
    })
}

fn texto_argumento(valor: &Valor, funcion: &str) -> String {
    match valor {
        Valor::Texto(texto) => (**texto).clone(),
        otro => fallo(format!("{}() espera un texto, no {}", funcion, otro.tipo())),
    }
}

// ---------------------------------------------------------------------------
// Plantillas HTML: {{ dato }} (protegido como HTML), {{{ dato }}} (sin proteger),
// {% para x en lista %}...{% fin %}, {% si dato %}...{% sino %}...{% fin %} y {# nota #}.
// ---------------------------------------------------------------------------

/// Protege el texto como HTML sin volver a proteger lo que ya lo estaba (&amp; sigue igual),
/// porque los campos de los formularios llegan ya protegidos.
fn proteger_una_vez(texto: &str) -> String {
    let mut salida = String::new();
    let mut resto = texto;
    while let Some(caracter) = resto.chars().next() {
        match caracter {
            '&' => {
                let entidad = resto[1..].find(';').filter(|fin| *fin <= 8).map(|fin| &resto[1..1 + fin]);
                let valida = entidad.map(|e| {
                    matches!(e, "amp" | "lt" | "gt" | "quot" | "apos")
                        || (e.starts_with('#') && e.len() > 1 && e[1..].chars().all(|c| c.is_ascii_digit()))
                }).unwrap_or(false);
                salida.push_str(if valida { "&" } else { "&amp;" });
            }
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#39;"),
            otro => salida.push(otro),
        }
        resto = &resto[caracter.len_utf8()..];
    }
    salida
}

fn buscar_en_plantilla(camino: &[String], ambitos: &[(String, Valor)], datos: &Valor) -> Result<Valor, String> {
    let primero = &camino[0];
    let mut valor = match ambitos.iter().rev().find(|(nombre, _)| nombre == primero) {
        Some((_, valor)) => valor.clone(),
        None => match datos {
            Valor::Mapa(mapa) if mapa.valores.contains_key(primero) => mapa.valores[primero].clone(),
            _ => return Err(format!("la plantilla usa '{}' y no está en los datos", primero)),
        },
    };
    for parte in &camino[1..] {
        valor = match &valor {
            Valor::Mapa(mapa) => mapa.valores.get(parte).cloned().unwrap_or(Valor::Nada),
            Valor::Lista(elementos) => parte.parse::<usize>().ok().and_then(|i| elementos.get(i).cloned()).unwrap_or(Valor::Nada),
            _ => Valor::Nada,
        };
    }
    Ok(valor)
}

fn es_cierto_en_plantilla(valor: &Valor) -> bool {
    match valor {
        Valor::Nada => false,
        Valor::Logico(b) => *b,
        Valor::Numero(n) => *n != 0.0,
        Valor::Texto(t) => !t.is_empty(),
        Valor::Lista(l) => !l.is_empty(),
        Valor::Mapa(m) => !m.claves.is_empty(),
    }
}

fn pintar_plantilla(trozos: &[TrozoPlantilla], ambitos: &mut Vec<(String, Valor)>, datos: &Valor, salida: &mut String) -> Result<(), String> {
    for trozo in trozos {
        match trozo {
            TrozoPlantilla::Texto(texto) => salida.push_str(texto),
            TrozoPlantilla::Dato { camino, proteger } => {
                let valor = buscar_en_plantilla(camino, ambitos, datos)?;
                let texto = if valor == Valor::Nada { String::new() } else { texto_de(&valor) };
                salida.push_str(&if *proteger { proteger_una_vez(&texto) } else { texto });
            }
            TrozoPlantilla::Para { variable, camino, cuerpo } => {
                let elementos: Vec<Valor> = match buscar_en_plantilla(camino, ambitos, datos)? {
                    Valor::Lista(elementos) => elementos.iter().cloned().collect(),
                    Valor::Mapa(mapa) => mapa.claves.iter().map(|c| Valor::Texto(Rc::new(c.clone()))).collect(),
                    Valor::Nada => Vec::new(),
                    otro => return Err(format!("'para' en la plantilla necesita una lista, y '{}' es {}", camino.join("."), otro.tipo())),
                };
                for elemento in elementos {
                    ambitos.push((variable.clone(), elemento));
                    let resultado = pintar_plantilla(cuerpo, ambitos, datos, salida);
                    ambitos.pop();
                    resultado?;
                }
            }
            TrozoPlantilla::Si { negar, camino, entonces, sino } => {
                let cierto = es_cierto_en_plantilla(&buscar_en_plantilla(camino, ambitos, datos)?) != *negar;
                pintar_plantilla(if cierto { entonces } else { sino }, ambitos, datos, salida)?;
            }
        }
    }
    Ok(())
}

/// Rellena el texto de una plantilla con los datos (una lista con claves).
fn rellenar_plantilla(plantilla: &Valor, datos: &Valor) -> Valor {
    let texto = texto_argumento(plantilla, "rellenar_plantilla");
    if !matches!(datos, Valor::Mapa(_) | Valor::Nada) {
        fallo(format!("los datos de una plantilla van en una lista con claves, como {{\"nombre\": \"Ana\"}}, no en {}", datos.tipo()));
    }
    let trozos = analizar_plantilla(&texto).unwrap_or_else(|e| fallo(format!("plantilla mal escrita, {}", e)));
    let mut salida = String::new();
    if let Err(e) = pintar_plantilla(&trozos, &mut Vec::new(), datos, &mut salida) {
        fallo(e);
    }
    Valor::Texto(Rc::new(salida))
}

/// plantilla("archivo.html", datos): si el nombre va entre comillas, Deimon incluye el archivo
/// al traducir (y lo revisa); si no, lo lee aquí, al ejecutar.
fn plantilla(archivo: &Valor, datos: &Valor) -> Valor {
    let ruta = texto_argumento(archivo, "plantilla");
    match std::fs::read_to_string(&ruta) {
        Ok(texto) => rellenar_plantilla(&Valor::Texto(Rc::new(texto)), datos),
        Err(error) => fallo(format!("no se pudo leer la plantilla '{}': {}", ruta, error)),
    }
}

"##, include_str!("plantilla_lectura.rs"), r##"
/// Valor de un campo del formulario (o de `?campo=` en la dirección), escapado como HTML.
fn campo(nombre: &Valor) -> Valor {
    let nombre = texto_argumento(nombre, "campo");
    PETICION.with(|peticion| {
        peticion
            .borrow()
            .campos
            .iter()
            .find(|(clave, _)| *clave == nombre)
            .map(|(_, valor)| Valor::Texto(Rc::new(escapar_html(valor))))
            .unwrap_or(Valor::Nada)
    })
}

/// El texto con los caracteres especiales de HTML protegidos.
fn proteger_html(valor: &Valor) -> Valor {
    Valor::Texto(Rc::new(escapar_html(&texto_de(valor))))
}

/// Verdadero si la ruta se está atendiendo porque se envió un formulario.
fn es_envio() -> Valor {
    Valor::Logico(PETICION.with(|peticion| peticion.borrow().envio))
}

/// Hace que el navegador vaya a otra página de este mismo sitio.
fn redirigir(camino: &Valor) -> Valor {
    let camino = texto_argumento(camino, "redirigir");
    if !camino.starts_with('/') || camino.starts_with("//") || camino.contains(['\r', '\n']) {
        fallo("redirigir() solo acepta caminos de este sitio que empiecen por '/', por ejemplo \"/inicio\"".to_string());
    }
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().redireccion = Some(camino));
    Valor::Texto(Rc::new(String::new()))
}

fn preparar_peticion(envio: bool, campos: Vec<(String, String)>, galletas: Vec<(String, String)>) {
    PETICION.with(|peticion| {
        *peticion.borrow_mut() = Peticion {
            segura: false,
            envio,
            campos,
            galletas,
        }
    });
    RESPUESTA.with(|respuesta| {
        *respuesta.borrow_mut() = Respuesta {
            capturando: true,
            ..Respuesta::default()
        }
    });
}

fn tomar_respuesta() -> Respuesta {
    RESPUESTA.with(|respuesta| std::mem::take(&mut *respuesta.borrow_mut()))
}

// ---------------------------------------------------------------------------
// Contraseñas: PBKDF2-HMAC-SHA256 con sal aleatoria (sin dependencias).
// ---------------------------------------------------------------------------

fn sha256(datos: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut mensaje = datos.to_vec();
    let bits = (datos.len() as u64).wrapping_mul(8);
    mensaje.push(0x80);
    while mensaje.len() % 64 != 56 {
        mensaje.push(0);
    }
    mensaje.extend_from_slice(&bits.to_be_bytes());
    for bloque in mensaje.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([bloque[4 * i], bloque[4 * i + 1], bloque[4 * i + 2], bloque[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    let mut salida = [0u8; 32];
    for i in 0..8 {
        salida[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    salida
}

fn hmac_sha256(clave: &[u8], mensaje: &[u8]) -> [u8; 32] {
    let mut bloque = [0u8; 64];
    if clave.len() > 64 {
        bloque[..32].copy_from_slice(&sha256(clave));
    } else {
        bloque[..clave.len()].copy_from_slice(clave);
    }
    let mut interior: Vec<u8> = bloque.iter().map(|b| b ^ 0x36).collect();
    interior.extend_from_slice(mensaje);
    let mut exterior: Vec<u8> = bloque.iter().map(|b| b ^ 0x5c).collect();
    exterior.extend_from_slice(&sha256(&interior));
    sha256(&exterior)
}

fn pbkdf2_sha256(clave: &[u8], sal: &[u8], vueltas: u32) -> [u8; 32] {
    let mut primero = sal.to_vec();
    primero.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(clave, &primero);
    let mut resultado = u;
    for _ in 1..vueltas {
        u = hmac_sha256(clave, &u);
        for (r, b) in resultado.iter_mut().zip(u.iter()) {
            *r ^= b;
        }
    }
    resultado
}

fn hexadecimal(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn desde_hexadecimal(texto: &str) -> Option<Vec<u8>> {
    if texto.len() % 2 != 0 {
        return None;
    }
    (0..texto.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texto.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Bytes aleatorios del sistema (/dev/urandom); si no hay, se mezclan reloj, proceso y contador.
fn bytes_aleatorios(cantidad: usize) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = vec![0u8; cantidad];
    if let Ok(mut fuente) = std::fs::File::open("/dev/urandom") {
        if fuente.read_exact(&mut bytes).is_ok() {
            return bytes;
        }
    }
    let mut salida = Vec::new();
    while salida.len() < cantidad {
        let semilla = format!(
            "{:?}-{}-{}",
            std::time::SystemTime::now(),
            std::process::id(),
            DATOS_SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        salida.extend_from_slice(&sha256(semilla.as_bytes()));
    }
    salida.truncate(cantidad);
    salida
}

fn iguales_en_tiempo_constante(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acumulado, (x, y)| acumulado | (x ^ y)) == 0
}

const VUELTAS_CLAVE: u32 = 100_000;
/// Colección interna de usuarios: empieza por '.', así que leer()/guardar() no pueden tocarla.
const COLECCION_USUARIOS: &str = ".usuarios";

fn nombre_usuario(valor: &Valor, funcion: &str) -> String {
    let nombre = texto_argumento(valor, funcion).trim().to_string();
    if nombre.is_empty() || nombre.chars().count() > 64 || nombre.chars().any(char::is_control) {
        return String::new();
    }
    nombre
}

/// Crea un usuario con su contraseña cifrada. Falso si ya existe, si el nombre no vale o si
/// la contraseña tiene menos de 8 caracteres.
fn crear_usuario(nombre: &Valor, clave: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "crear_usuario");
    let clave = texto_argumento(clave, "crear_usuario");
    if nombre.is_empty() || clave.chars().count() < 8 {
        return Valor::Logico(false);
    }
    let ficha = cifrar_clave(&clave);
    let creado = con_coleccion(COLECCION_USUARIOS, |entrada, ruta, archivo| {
        if entrada.datos.contains_key(&nombre) {
            return false;
        }
        let dato = Dato::Texto(ficha);
        anotar(entrada, archivo, COLECCION_USUARIOS, &linea_registro('s', &nombre, Some(&dato)));
        entrada.datos.insert(nombre.clone(), dato);
        compactar_si_hace_falta(entrada, ruta, COLECCION_USUARIOS);
        true
    });
    Valor::Logico(creado)
}

/// Verdadero si el usuario existe y la contraseña es la suya. Tras 5 fallos seguidos la
/// cuenta queda bloqueada 15 minutos y devuelve falso aunque la contraseña sea buena.
fn comprobar_clave(nombre: &Valor, clave: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "comprobar_clave");
    let clave = texto_argumento(clave, "comprobar_clave");
    if bloqueada(&nombre) {
        return Valor::Logico(false);
    }
    let correcta = clave_correcta(&nombre, &clave);
    con_fallos(|fallos| {
        if correcta {
            fallos.remove(&nombre);
        } else {
            let entrada = fallos.entry(nombre.clone()).or_insert((0, segundos_ahora()));
            entrada.0 += 1;
            entrada.1 = segundos_ahora();
        }
    });
    Valor::Logico(correcta)
}

// ---------------------------------------------------------------------------
// Sesiones: una galleta con un código aleatorio. En la base de datos solo se guarda la
// huella SHA-256 del código (quien copie el archivo no puede hacerse pasar por nadie),
// así que las sesiones siguen abiertas aunque el servidor se reinicie.
// ---------------------------------------------------------------------------

const GALLETA_SESION: &str = "deimon_sesion";
const DURACION_SESION: u64 = 7 * 24 * 3600;
const COLECCION_SESIONES: &str = ".sesiones";
const COLECCION_RECUPERACION: &str = ".recuperacion";
const DURACION_RECUPERACION: u64 = 3600;
const MAXIMO_FALLOS: u32 = 5;
const DURACION_BLOQUEO: u64 = 15 * 60;

fn segundos_ahora() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn huella(codigo: &str) -> String {
    hexadecimal(&sha256(codigo.as_bytes()))
}

fn borrar_dato(coleccion: &str, clave: &str) {
    con_coleccion(coleccion, |entrada, ruta, archivo| {
        if entrada.datos.remove(clave).is_some() {
            anotar(entrada, archivo, coleccion, &linea_registro('b', clave, None));
            compactar_si_hace_falta(entrada, ruta, coleccion);
        }
    });
}

/// Borra las fichas (sesiones o códigos) de un usuario, o todas las caducadas.
fn limpiar_fichas(coleccion: &str, de_usuario: Option<&str>) {
    let ahora = segundos_ahora();
    con_coleccion(coleccion, |entrada, ruta, archivo| {
        let sobran: Vec<String> = entrada
            .datos
            .iter()
            .filter(|(_, dato)| match dato {
                Dato::Texto(ficha) => {
                    let (nombre, caduca) = ficha.rsplit_once('\t').unwrap_or(("", "0"));
                    de_usuario.map(|usuario| usuario == nombre).unwrap_or(false)
                        || caduca.parse::<u64>().unwrap_or(0) <= ahora
                }
                _ => true,
            })
            .map(|(clave, _)| clave.clone())
            .collect();
        for clave in sobran {
            entrada.datos.remove(&clave);
            anotar(entrada, archivo, coleccion, &linea_registro('b', &clave, None));
        }
        compactar_si_hace_falta(entrada, ruta, coleccion);
    });
}

/// Nombre dueño de una ficha si no ha caducado.
fn dueno_de_ficha(coleccion: &str, codigo: &str) -> Option<String> {
    match leer_dato(coleccion, &huella(codigo)) {
        Some(Dato::Texto(ficha)) => {
            let (nombre, caduca) = ficha.rsplit_once('\t')?;
            (caduca.parse::<u64>().ok()? > segundos_ahora()).then(|| nombre.to_string())
        }
        _ => None,
    }
}

fn poner_galleta(mut cabecera: String) {
    // Por HTTPS, la galleta solo viaja cifrada.
    if PETICION.with(|peticion| peticion.borrow().segura) {
        cabecera.push_str("; Secure");
    }
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().cabeceras.push(cabecera));
}

/// Abre una sesión para un usuario existente: desde ahora usuario_actual() lo devuelve.
fn iniciar_sesion(nombre: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "iniciar_sesion");
    if leer_dato(COLECCION_USUARIOS, &nombre).is_none() {
        fallo(format!("iniciar_sesion(): el usuario '{}' no existe; créalo antes con crear_usuario()", nombre));
    }
    limpiar_fichas(COLECCION_SESIONES, None);
    let codigo = hexadecimal(&bytes_aleatorios(32));
    guardar_dato(
        COLECCION_SESIONES,
        &huella(&codigo),
        Dato::Texto(format!("{}\t{}", nombre, segundos_ahora() + DURACION_SESION)),
    );
    poner_galleta(format!(
        "Set-Cookie: {}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        GALLETA_SESION, codigo, DURACION_SESION
    ));
    Valor::Nada
}

/// Nombre del usuario con la sesión abierta, o nada si nadie ha entrado.
fn usuario_actual() -> Valor {
    galleta(GALLETA_SESION)
        .filter(|codigo| codigo.len() == 64 && codigo.chars().all(|c| c.is_ascii_hexdigit()))
        .and_then(|codigo| dueno_de_ficha(COLECCION_SESIONES, &codigo))
        .map(|nombre| Valor::Texto(Rc::new(nombre)))
        .unwrap_or(Valor::Nada)
}

/// Cierra la sesión de quien hace la petición.
fn cerrar_sesion() -> Valor {
    if let Some(codigo) = galleta(GALLETA_SESION) {
        borrar_dato(COLECCION_SESIONES, &huella(&codigo));
    }
    poner_galleta(format!(
        "Set-Cookie: {}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        GALLETA_SESION
    ));
    Valor::Nada
}

// ---------------------------------------------------------------------------
// Sesión del visitante (como $_SESSION de PHP): datos de cada navegador, aunque no haya
// entrado ningún usuario. Una galleta con un código aleatorio; en la base de datos solo
// se guarda su huella. Caduca 24 horas después del último cambio.
// ---------------------------------------------------------------------------

const GALLETA_VISITA: &str = "deimon_visita";
const COLECCION_VISITAS: &str = ".visitas";
const DURACION_VISITA: u64 = 24 * 3600;

fn codigo_de_visita(crear: bool) -> Option<String> {
    let valida = |c: &String| c.len() == 64 && c.chars().all(|x| x.is_ascii_hexdigit());
    if let Some(codigo) = galleta(GALLETA_VISITA).filter(valida) {
        return Some(codigo);
    }
    if !crear {
        return None;
    }
    let codigo = hexadecimal(&bytes_aleatorios(32));
    poner_galleta(format!(
        "Set-Cookie: {}={}; Path=/; HttpOnly; SameSite=Lax",
        GALLETA_VISITA, codigo
    ));
    // Para que el resto de esta misma petición ya la vea.
    PETICION.with(|peticion| peticion.borrow_mut().galletas.push((GALLETA_VISITA.to_string(), codigo.clone())));
    // De paso se borran las visitas caducadas.
    let ahora = segundos_ahora();
    con_coleccion(COLECCION_VISITAS, |entrada, ruta, archivo| {
        let caducadas: Vec<String> = entrada
            .datos
            .iter()
            .filter_map(|(clave, dato)| match (clave.strip_suffix('\t'), dato) {
                (Some(prefijo), Dato::Numero(caduca)) if (*caduca as u64) <= ahora => Some(prefijo.to_string()),
                _ => None,
            })
            .collect();
        for prefijo in caducadas {
            let sobran: Vec<String> = entrada
                .datos
                .keys()
                .filter(|clave| clave.starts_with(&format!("{}\t", prefijo)))
                .cloned()
                .collect();
            for clave in sobran {
                entrada.datos.remove(&clave);
                anotar(entrada, archivo, COLECCION_VISITAS, &linea_registro('b', &clave, None));
            }
        }
        compactar_si_hace_falta(entrada, ruta, COLECCION_VISITAS);
    });
    Some(codigo)
}

fn clave_de_visita(codigo: &str, clave: &str) -> String {
    format!("{}\t{}", huella(codigo), clave)
}

fn visita_viva(codigo: &str) -> bool {
    matches!(leer_dato(COLECCION_VISITAS, &format!("{}\t", huella(codigo))), Some(Dato::Numero(caduca)) if (caduca as u64) > segundos_ahora())
}

/// Guarda un valor (también listas) en la sesión de este visitante.
fn guardar_en_sesion(clave: &Valor, valor: &Valor) -> Valor {
    let clave = texto_argumento(clave, "guardar_en_sesion");
    let dato = Dato::desde(valor);
    let codigo = codigo_de_visita(true).expect("se acaba de crear");
    guardar_dato(COLECCION_VISITAS, &format!("{}\t", huella(&codigo)), Dato::Numero((segundos_ahora() + DURACION_VISITA) as f64));
    guardar_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave), dato);
    Valor::Nada
}

/// Lee un valor de la sesión de este visitante, o nada.
fn leer_de_sesion(clave: &Valor) -> Valor {
    let clave = texto_argumento(clave, "leer_de_sesion");
    match codigo_de_visita(false) {
        Some(codigo) if visita_viva(&codigo) => leer_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave))
            .map(|dato| dato.a_valor())
            .unwrap_or(Valor::Nada),
        _ => Valor::Nada,
    }
}

/// Añade un valor a una lista de la sesión (la crea si no existe), como $_SESSION['x'][] = v.
fn agregar_a_sesion(clave: &Valor, valor: &Valor) -> Valor {
    let actual = leer_de_sesion(clave);
    let lista = match actual {
        Valor::Nada => Valor::Lista(Rc::new(vec![valor.clone()])),
        otra => agregar(&otra, valor),
    };
    guardar_en_sesion(clave, &lista)
}

/// Borra un valor de la sesión de este visitante.
fn borrar_de_sesion(clave: &Valor) -> Valor {
    let clave = texto_argumento(clave, "borrar_de_sesion");
    if let Some(codigo) = codigo_de_visita(false) {
        borrar_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave));
    }
    Valor::Nada
}

/// Borra todos los datos de la sesión de este visitante (como session_destroy()).
fn vaciar_sesion() -> Valor {
    if let Some(codigo) = codigo_de_visita(false) {
        let prefijo = format!("{}\t", huella(&codigo));
        con_coleccion(COLECCION_VISITAS, |entrada, ruta, archivo| {
            let sobran: Vec<String> = entrada.datos.keys().filter(|c| c.starts_with(&prefijo)).cloned().collect();
            for clave in sobran {
                entrada.datos.remove(&clave);
                anotar(entrada, archivo, COLECCION_VISITAS, &linea_registro('b', &clave, None));
            }
            compactar_si_hace_falta(entrada, ruta, COLECCION_VISITAS);
        });
    }
    Valor::Nada
}

// ---------------------------------------------------------------------------
// Límite de intentos: tras 5 contraseñas equivocadas seguidas, la cuenta se bloquea
// 15 minutos (aunque la contraseña sea la buena), para frenar a quien prueba claves.
// ---------------------------------------------------------------------------

static FALLOS: Mutex<Option<std::collections::HashMap<String, (u32, u64)>>> = Mutex::new(None);

fn con_fallos<T>(operacion: impl FnOnce(&mut std::collections::HashMap<String, (u32, u64)>) -> T) -> T {
    let mut fallos = FALLOS.lock().unwrap_or_else(|error| error.into_inner());
    operacion(fallos.get_or_insert_with(std::collections::HashMap::new))
}

fn bloqueada(nombre: &str) -> bool {
    let ahora = segundos_ahora();
    con_fallos(|fallos| match fallos.get(nombre) {
        Some((veces, desde)) if *veces >= MAXIMO_FALLOS && ahora < desde + DURACION_BLOQUEO => true,
        Some((_, desde)) if ahora >= desde + DURACION_BLOQUEO => {
            fallos.remove(nombre);
            false
        }
        _ => false,
    })
}

/// Verdadero si la cuenta está bloqueada por demasiados intentos fallidos.
fn cuenta_bloqueada(nombre: &Valor) -> Valor {
    Valor::Logico(bloqueada(&nombre_usuario(nombre, "cuenta_bloqueada")))
}

fn cifrar_clave(clave: &str) -> String {
    let sal = bytes_aleatorios(16);
    let cifrada = pbkdf2_sha256(clave.as_bytes(), &sal, VUELTAS_CLAVE);
    format!("pbkdf2-sha256${}${}${}", VUELTAS_CLAVE, hexadecimal(&sal), hexadecimal(&cifrada))
}

fn clave_correcta(nombre: &str, clave: &str) -> bool {
    let ficha = match leer_dato(COLECCION_USUARIOS, nombre) {
        Some(Dato::Texto(ficha)) => ficha,
        _ => {
            // Se calcula igual para que no se note por el tiempo si el usuario existe.
            pbkdf2_sha256(clave.as_bytes(), b"deimon-sin-usuario", VUELTAS_CLAVE);
            return false;
        }
    };
    let partes: Vec<&str> = ficha.split('$').collect();
    match partes.as_slice() {
        ["pbkdf2-sha256", vueltas, sal, cifrada] => {
            match (vueltas.parse::<u32>(), desde_hexadecimal(sal), desde_hexadecimal(cifrada)) {
                (Ok(vueltas), Some(sal), Some(cifrada)) => {
                    iguales_en_tiempo_constante(&pbkdf2_sha256(clave.as_bytes(), &sal, vueltas), &cifrada)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Cambia la contraseña guardada y cierra todas las sesiones abiertas del usuario.
fn poner_clave(nombre: &str, nueva: &str) {
    guardar_dato(COLECCION_USUARIOS, nombre, Dato::Texto(cifrar_clave(nueva)));
    limpiar_fichas(COLECCION_SESIONES, Some(nombre));
    limpiar_fichas(COLECCION_RECUPERACION, Some(nombre));
}

/// Cambia la contraseña sabiendo la actual. Falso si la actual no es correcta o la nueva
/// tiene menos de 8 caracteres.
fn cambiar_clave(nombre: &Valor, actual: &Valor, nueva: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "cambiar_clave");
    let actual = texto_argumento(actual, "cambiar_clave");
    let nueva = texto_argumento(nueva, "cambiar_clave");
    if nueva.chars().count() < 8 || bloqueada(&nombre) || !clave_correcta(&nombre, &actual) {
        return Valor::Logico(false);
    }
    poner_clave(&nombre, &nueva);
    Valor::Logico(true)
}

/// Crea un código de un solo uso (válido 1 hora) para recuperar la contraseña; el programa
/// decide cómo hacérselo llegar al usuario. Nada si el usuario no existe.
fn crear_codigo_recuperacion(nombre: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "crear_codigo_recuperacion");
    if leer_dato(COLECCION_USUARIOS, &nombre).is_none() {
        return Valor::Nada;
    }
    limpiar_fichas(COLECCION_RECUPERACION, None);
    let codigo = hexadecimal(&bytes_aleatorios(16));
    guardar_dato(
        COLECCION_RECUPERACION,
        &huella(&codigo),
        Dato::Texto(format!("{}\t{}", nombre, segundos_ahora() + DURACION_RECUPERACION)),
    );
    Valor::Texto(Rc::new(codigo))
}

/// Pone una contraseña nueva con un código de recuperación (que deja de valer).
fn restablecer_clave(nombre: &Valor, codigo: &Valor, nueva: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "restablecer_clave");
    let codigo = texto_argumento(codigo, "restablecer_clave");
    let nueva = texto_argumento(nueva, "restablecer_clave");
    if nueva.chars().count() < 8 || dueno_de_ficha(COLECCION_RECUPERACION, &codigo).as_deref() != Some(nombre.as_str()) {
        return Valor::Logico(false);
    }
    borrar_dato(COLECCION_RECUPERACION, &huella(&codigo));
    poner_clave(&nombre, &nueva);
    con_fallos(|fallos| {
        fallos.remove(&nombre);
    });
    Valor::Logico(true)
}

/// Suma `cantidad` a un número guardado de forma atómica (nadie más lo cambia a la vez).
fn incrementar(coleccion: &Valor, clave: &Valor, cantidad: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let clave = clave_texto(clave);
    let cantidad = match cantidad {
        Valor::Numero(numero) => *numero,
        otro => fallo(format!("incrementar() espera un número como cantidad, no {}", otro.tipo())),
    };
    let nuevo = con_coleccion(&coleccion, |entrada, ruta, archivo| {
        let actual = match entrada.datos.get(&clave) {
            None | Some(Dato::Nada) => 0.0,
            Some(Dato::Numero(numero)) => *numero,
            Some(_) => fallo(format!(
                "incrementar(): la clave '{}' no guarda un número",
                clave
            )),
        };
        let dato = Dato::Numero(actual + cantidad);
        anotar(entrada, archivo, &coleccion, &linea_registro('s', &clave, Some(&dato)));
        entrada.datos.insert(clave.clone(), dato);
        compactar_si_hace_falta(entrada, ruta, &coleccion);
        actual + cantidad
    });
    Valor::Numero(nuevo)
}

/// Atiende una ruta sin red (para visitar/enviar en las pruebas), guardando las galletas.
fn atender_sin_red(camino: &str, envio: bool, mut campos: Vec<(String, String)>, rutas: Vec<Ruta>) -> Valor {
    let (camino, consulta) = camino.split_once('?').unwrap_or((camino, ""));
    campos.extend(decodificar_formulario(consulta));
    let galletas = GALLETAS_PRUEBA.with(|galletas| galletas.borrow().clone());
    preparar_peticion(envio, campos, galletas);
    let valor = match buscar_ruta(&rutas, camino) {
        Some((ruta, parametros)) => Some((ruta.manejador)(parametros)),
        None => None,
    };
    let respuesta = tomar_respuesta();
    let cuerpo = match valor {
        Some(valor) => cuerpo_pagina(&respuesta.escrito, &valor),
        None => PAGINA_404.to_string(),
    };
    for cabecera in &respuesta.cabeceras {
        if let Some(valor) = cabecera.strip_prefix("Set-Cookie: ") {
            let primera = valor.split(';').next().unwrap_or("");
            if let Some((nombre, contenido)) = primera.split_once('=') {
                GALLETAS_PRUEBA.with(|galletas| {
                    let mut galletas = galletas.borrow_mut();
                    galletas.retain(|(clave, _)| clave != nombre);
                    if !contenido.is_empty() && !valor.contains("Max-Age=0") {
                        galletas.push((nombre.to_string(), contenido.to_string()));
                    }
                });
            }
        }
    }
    match respuesta.redireccion {
        Some(destino) => Valor::Texto(Rc::new(format!("redirige a {}", destino))),
        None => Valor::Texto(Rc::new(cuerpo)),
    }
}

/// Como visitar(), pero enviando un formulario: datos es un texto como "nombre=Ana&clave=123".
fn enviar(camino: &Valor, datos: &Valor, rutas: Vec<Ruta>) -> Valor {
    let camino = texto_argumento(camino, "enviar");
    let datos = texto_argumento(datos, "enviar");
    atender_sin_red(&camino, true, decodificar_formulario(&datos), rutas)
}

fn responder(conexion: &mut std::net::TcpStream, estado: &str, cuerpo: &str) {
    responder_con(conexion, estado, cuerpo, &[]);
}

fn responder_con(conexion: &mut std::net::TcpStream, estado: &str, cuerpo: &str, extra: &[String]) {
    use std::io::{Read, Write};
    let mut cabeceras = String::new();
    for cabecera in extra {
        cabeceras.push_str(cabecera);
        cabeceras.push_str("\r\n");
    }
    let respuesta = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
        estado,
        cuerpo.len(),
        cabeceras,
        cuerpo
    );
    let _ = conexion.write_all(respuesta.as_bytes());
    let _ = conexion.flush();
    // Se cierra el lado de escritura y se descarta lo que quede sin leer: así el cliente
    // recibe la respuesta completa en vez de un corte (RST) por datos pendientes.
    let _ = conexion.shutdown(std::net::Shutdown::Write);
    let _ = conexion.set_read_timeout(Some(std::time::Duration::from_millis(1)));
    let mut descarte = [0u8; 4096];
    loop {
        match conexion.read(&mut descarte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

/// Límites del servidor; salen del perfil grabado al compilar y de las variables DEIMON_*.
#[derive(Clone, Copy)]
struct Limites {
    hilos: usize,
    max_conexiones: usize,
    max_peticion: usize,
    tiempo_espera: std::time::Duration,
}

fn numero_de_entorno(nombre: &str) -> Option<usize> {
    std::env::var(nombre)
        .ok()
        .and_then(|texto| texto.trim().parse::<usize>().ok())
        .filter(|numero| *numero > 0)
}

fn limites() -> Limites {
    let por_perfil = |pequeno: usize, equilibrado: usize, servidor: usize| match PERFIL_LN {
        "pequeño" | "pequeno" => pequeno,
        "servidor" => servidor,
        _ => equilibrado,
    };
    let hilos = numero_de_entorno("DEIMON_HILOS").unwrap_or_else(|| {
        let nucleos = std::thread::available_parallelism()
            .map(|cantidad| cantidad.get())
            .unwrap_or(1);
        // Casi todo el tiempo de un hilo es esperar a la red, no calcular: con pocos hilos,
        // unos pocos clientes lentos bastarían para ocuparlos todos.
        match PERFIL_LN {
            "pequeño" | "pequeno" => 4,
            "servidor" => (nucleos * 8).max(64),
            _ => (nucleos * 4).max(16),
        }
    });
    let max_conexiones =
        numero_de_entorno("DEIMON_MAX_CONEXIONES").unwrap_or_else(|| por_perfil(16, 128, 1024));
    let max_peticion = numero_de_entorno("DEIMON_MAX_PETICION")
        .unwrap_or_else(|| por_perfil(16 * 1024, 256 * 1024, 1024 * 1024));
    let segundos = numero_de_entorno("DEIMON_TIEMPO_ESPERA").unwrap_or(10);
    Limites {
        hilos,
        max_conexiones,
        max_peticion,
        tiempo_espera: std::time::Duration::from_secs(segundos as u64),
    }
}

enum Lectura {
    /// Cabeceras y los bytes del cuerpo que ya llegaron con ellas.
    Peticion(String, Vec<u8>),
    DemasiadoGrande,
    Cerrada,
}

fn fin_de_cabecera(datos: &[u8]) -> Option<usize> {
    datos
        .windows(4)
        .position(|ventana| ventana == b"\r\n\r\n")
        .map(|posicion| posicion + 4)
        .or_else(|| {
            datos
                .windows(2)
                .position(|ventana| ventana == b"\n\n")
                .map(|posicion| posicion + 2)
        })
}

/// Antes de cada lectura, ajusta la espera a lo que queda del plazo total de la petición.
/// Devuelve falso si el plazo ya terminó (un cliente que gotea datos no ocupa el hilo para siempre).
fn quedar_hasta(conexion: &std::net::TcpStream, limite: std::time::Instant) -> bool {
    match limite.checked_duration_since(std::time::Instant::now()) {
        Some(resto) if !resto.is_zero() => conexion.set_read_timeout(Some(resto)).is_ok(),
        _ => false,
    }
}

/// Lee solo las cabeceras, sin pasar del máximo ni del plazo: así un cliente lento o enorme
/// no agota la memoria ni retiene el hilo.
fn leer_cabecera(conexion: &mut std::net::TcpStream, maximo: usize, limite: std::time::Instant) -> Lectura {
    use std::io::Read;
    let mut datos = Vec::new();
    let mut bloque = [0u8; 4096];
    loop {
        if let Some(fin) = fin_de_cabecera(&datos) {
            if fin > maximo {
                return Lectura::DemasiadoGrande;
            }
            let resto = datos.split_off(fin);
            return Lectura::Peticion(String::from_utf8_lossy(&datos).into_owned(), resto);
        }
        if datos.len() > maximo {
            return Lectura::DemasiadoGrande;
        }
        let disponibles = maximo + 1 - datos.len();
        let tamano = bloque.len().min(disponibles);
        if !quedar_hasta(conexion, limite) {
            return Lectura::Cerrada;
        }
        match conexion.read(&mut bloque[..tamano]) {
            Ok(0) => return Lectura::Cerrada,
            Ok(leidos) => datos.extend_from_slice(&bloque[..leidos]),
            Err(_) => return Lectura::Cerrada,
        }
    }
}

fn atender(conexion: std::net::TcpStream, rutas: &[Ruta], limites: &Limites) {
    // Plazo total para recibir la petición entera (cabeceras y cuerpo), no por cada lectura.
    let limite = std::time::Instant::now() + limites.tiempo_espera;
    let _ = conexion.set_read_timeout(Some(limites.tiempo_espera));
    let _ = conexion.set_write_timeout(Some(std::time::Duration::from_secs(10)));
    let mut escritura = match conexion.try_clone() {
        Ok(copia) => copia,
        Err(_) => return,
    };
    let mut lectura = conexion;
    let (peticion, mut cuerpo) = match leer_cabecera(&mut lectura, limites.max_peticion, limite) {
        Lectura::Peticion(texto, resto) => (texto, resto),
        Lectura::DemasiadoGrande => {
            responder(
                &mut escritura,
                "413 Payload Too Large",
                "<h1>Petición demasiado grande</h1>",
            );
            return;
        }
        Lectura::Cerrada => return,
    };
    let primera = peticion.lines().next().unwrap_or("");
    let mut partes = primera.split_whitespace();
    let metodo = partes.next().unwrap_or("");
    let objetivo = partes.next().unwrap_or("");
    if metodo != "GET" && metodo != "POST" {
        responder(
            &mut escritura,
            "405 Method Not Allowed",
            "<h1>Método no permitido</h1>",
        );
        return;
    }
    let cabecera = |nombre: &str| -> Option<String> {
        peticion.lines().skip(1).find_map(|linea| {
            let (clave, valor) = linea.split_once(':')?;
            clave.trim().eq_ignore_ascii_case(nombre).then(|| valor.trim().to_string())
        })
    };
    let (camino, consulta) = objetivo.split_once('?').unwrap_or((objetivo, ""));
    let mut campos = decodificar_formulario(consulta);
    let envio = metodo == "POST";
    if envio {
        // El cuerpo del formulario cuenta para el tamaño máximo de la petición.
        // Un tamaño que no es un número, dos tamaños distintos o el envío por trozos (chunked,
        // que Deimon no entiende) se rechazan en vez de adivinar dónde termina el cuerpo.
        let tamanos: Vec<&str> = peticion
            .lines()
            .skip(1)
            .filter_map(|linea| linea.split_once(':'))
            .filter(|(clave, _)| clave.trim().eq_ignore_ascii_case("Content-Length"))
            .map(|(_, valor)| valor.trim())
            .collect();
        let largo: Option<usize> = match tamanos.as_slice() {
            [] => Some(0),
            [primero, resto @ ..] if resto.iter().all(|otro| otro == primero) => primero.parse().ok(),
            _ => None,
        };
        let largo = match largo {
            Some(largo) if cabecera("Transfer-Encoding").is_none() => largo,
            _ => {
                responder(&mut escritura, "400 Bad Request", "<h1>Petición mal formada</h1>");
                return;
            }
        };
        if largo > limites.max_peticion.saturating_sub(peticion.len()) {
            responder(&mut escritura, "413 Payload Too Large", "<h1>Petición demasiado grande</h1>");
            return;
        }
        let mut bloque = [0u8; 4096];
        while cuerpo.len() < largo {
            use std::io::Read;
            if !quedar_hasta(&lectura, limite) {
                responder(&mut escritura, "408 Request Timeout", "<h1>La petición tardó demasiado</h1>");
                return;
            }
            match lectura.read(&mut bloque) {
                Ok(0) | Err(_) => return,
                Ok(leidos) => cuerpo.extend_from_slice(&bloque[..leidos]),
            }
        }
        cuerpo.truncate(largo);
        campos.extend(decodificar_formulario(&String::from_utf8_lossy(&cuerpo)));
    }
    let galletas = cabecera("Cookie").map(|v| leer_galletas(&v)).unwrap_or_default();
    match buscar_ruta(rutas, camino) {
        Some((ruta, parametros)) => {
            preparar_peticion(envio, campos, galletas);
            let segura = cabecera("X-Forwarded-Proto").map(|v| v.eq_ignore_ascii_case("https")).unwrap_or(false);
            PETICION.with(|peticion| peticion.borrow_mut().segura = segura);
            EN_RUTA.with(|estado| estado.set(true));
            let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (ruta.manejador)(parametros)
            }));
            EN_RUTA.with(|estado| estado.set(false));
            let respuesta = tomar_respuesta();
            match resultado {
                Ok(valor) => match respuesta.redireccion {
                    Some(destino) => {
                        let mut extra = respuesta.cabeceras;
                        extra.push(format!("Location: {}", destino));
                        responder_con(&mut escritura, "303 See Other", "", &extra);
                    }
                    None => {
                        let cuerpo = cuerpo_pagina(&respuesta.escrito, &valor);
                        responder_con(&mut escritura, "200 OK", &cuerpo, &respuesta.cabeceras);
                    }
                },
                Err(_) => {
                    responder(
                        &mut escritura,
                        "500 Internal Server Error",
                        "<h1>Error interno</h1>",
                    );
                }
            }
        }
        None if camino == "/favicon.ico" || camino == "/favicon.png" => {
            responder_icono(&mut escritura);
        }
        None => responder(
            &mut escritura,
            "404 Not Found",
            PAGINA_404,
        ),
    }
}

fn responder_icono(conexion: &mut std::net::TcpStream) {
    use std::io::Write;
    let cabecera = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nCache-Control: max-age=86400\r\nConnection: close\r\n\r\n",
        ICONO_WEB.len()
    );
    let _ = conexion.write_all(cabecera.as_bytes());
    let _ = conexion.write_all(ICONO_WEB);
    let _ = conexion.flush();
    let _ = conexion.shutdown(std::net::Shutdown::Write);
}

fn servir(puerto: &Valor, rutas: Vec<Ruta>) -> Valor {
    let puerto = match puerto {
        Valor::Numero(n) => *n as u16,
        otro => fallo(format!(
            "servir() espera un número de puerto, no {}",
            otro.tipo()
        )),
    };
    let puerto = match std::env::var("DEIMON_PUERTO") {
        Ok(texto) => match texto.trim().parse::<u16>() {
            Ok(numero) => numero,
            Err(_) => fallo(format!(
                "la variable DEIMON_PUERTO no es un puerto válido: '{}'",
                texto
            )),
        },
        Err(_) => puerto,
    };
    let escucha = match std::net::TcpListener::bind(("127.0.0.1", puerto)) {
        Ok(escucha) => escucha,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let libre = (puerto.saturating_add(1)..=puerto.saturating_add(200))
                .find(|candidato| std::net::TcpListener::bind(("127.0.0.1", *candidato)).is_ok());
            let archivo = std::env::var("DEIMON_ARCHIVO").unwrap_or_else(|_| "tu_programa.dmn".to_string());
            let archivo = if archivo.chars().any(|c| c.is_whitespace()) {
                format!("\"{}\"", archivo)
            } else {
                archivo
            };
            match libre {
                Some(libre) => fallo(format!(
                    "la puerta {} está ocupada por otro programa. Prueba con: DEIMON_PUERTO={} deimon ejecutar {}",
                    puerto, libre, archivo
                )),
                None => fallo(format!(
                    "la puerta {} está ocupada por otro programa y no se encontró una libre cerca",
                    puerto
                )),
            }
        }
        Err(error) => fallo(format!("no se pudo abrir el puerto {}: {}", puerto, error)),
    };
    let puerto = match escucha.local_addr() {
        Ok(direccion) => direccion.port(),
        Err(_) => puerto,
    };
    let limites = limites();
    println!(
        "Servidor listo en http://127.0.0.1:{} (perfil {}, {} hilos)",
        puerto, PERFIL_LN, limites.hilos
    );
    let rutas = std::sync::Arc::new(rutas);
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |informacion| {
        if informacion.payload().downcast_ref::<ErrorRuta>().is_none() {
            anterior(informacion);
        }
    }));

    // Grupo fijo de hilos trabajadores: las conexiones esperan en una cola acotada.
    let (envio, recepcion) =
        std::sync::mpsc::sync_channel::<std::net::TcpStream>(limites.max_conexiones);
    let recepcion = std::sync::Arc::new(std::sync::Mutex::new(recepcion));
    for _ in 0..limites.hilos {
        let recepcion = std::sync::Arc::clone(&recepcion);
        let rutas = std::sync::Arc::clone(&rutas);
        std::thread::spawn(move || loop {
            let conexion = {
                let guardia = match recepcion.lock() {
                    Ok(guardia) => guardia,
                    Err(_) => break,
                };
                match guardia.recv() {
                    Ok(conexion) => conexion,
                    Err(_) => break,
                }
            };
            atender(conexion, &rutas, &limites);
        });
    }

    for conexion in escucha.incoming() {
        match conexion {
            Ok(conexion) => match envio.try_send(conexion) {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(mut conexion)) => {
                    responder(
                        &mut conexion,
                        "503 Service Unavailable",
                        "<h1>Servidor ocupado, inténtalo de nuevo</h1>",
                    );
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
            },
            Err(_) => continue,
        }
    }
    Valor::Nada
}
"##);

#[derive(Clone)]
struct PruebaDef {
    nombre: String,
    cuerpo: Vec<Sentencia>,
    archivo: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Modo {
    Normal,
    Pruebas,
}

#[derive(Clone)]
struct RutaDef {
    camino: String,
    parametros: Vec<String>,
    cuerpo: Vec<Sentencia>,
    archivo: Option<String>,
}

pub fn generar(programa: &Programa) -> Result<String, Vec<ErrorTraduccion>> {
    generar_con_modo(programa, Modo::Normal)
}

/// Genera el Rust o devuelve todos los errores de nombres, argumentos y estructura que encuentre.
pub fn generar_con_modo(programa: &Programa, modo: Modo) -> Result<String, Vec<ErrorTraduccion>> {
    generar_con_modo_perfil(programa, modo, "equilibrado")
}

/// Como `generar_con_modo`, grabando el perfil de recursos en el programa generado.
pub fn generar_con_modo_perfil(
    programa: &Programa,
    modo: Modo,
    perfil: &str,
) -> Result<String, Vec<ErrorTraduccion>> {
    let mut generador = Generador::nuevo(perfil);
    generador.modo = modo;
    generador.recoger_funciones(&programa.sentencias);
    generador.recoger_extensiones(&programa.extensiones);
    generador.recoger_rutas(&programa.sentencias);
    generador.recoger_pruebas(&programa.sentencias);
    if let Err(error) = generador.generar_programa(&programa.sentencias) {
        generador.errores.push(error);
    }
    if generador.errores.is_empty() {
        Ok(generador.salida)
    } else {
        Err(generador.errores)
    }
}

fn marcar_archivo(error: ErrorTraduccion, archivo: &Option<String>) -> ErrorTraduccion {
    match archivo {
        Some(archivo) => error.con_archivo(archivo),
        None => error,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TipoConocido {
    Texto,
    Lista,
    Otro,
}

struct Generador {
    salida: String,
    nivel: usize,
    funciones: HashMap<String, Vec<String>>,
    extensiones: HashMap<String, Vec<String>>,
    errores: Vec<ErrorTraduccion>,
    rutas: Vec<RutaDef>,
    /// Funciones con versión numérica rápida (nombre → número de parámetros).
    numericas: HashMap<String, usize>,
    pruebas: Vec<PruebaDef>,
    modo: Modo,
    scopes: Vec<HashMap<String, ()>>,
    tipos: Vec<HashMap<String, TipoConocido>>,
    en_funcion: bool,
    /// Bucles abiertos alrededor de la sentencia actual (para 'romper' y 'continuar').
    bucles: usize,
    archivo_actual: Option<String>,
}

impl Generador {
    fn nuevo(perfil: &str) -> Self {
        let mut salida = String::new();
        salida.push_str(
            "#![allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code)]\n",
        );
        let icono_web = ICONO_WEB
            .iter()
            .map(|byte| byte.to_string())
            .collect::<Vec<String>>()
            .join(",");
        salida.push_str(
            &RUNTIME
                .replace("@PERFIL@", perfil)
                .replace("@ICONO_WEB@", &icono_web),
        );
        salida.push('\n');
        Generador {
            salida,
            nivel: 0,
            funciones: HashMap::new(),
            extensiones: HashMap::new(),
            errores: Vec::new(),
            rutas: Vec::new(),
            numericas: HashMap::new(),
            pruebas: Vec::new(),
            modo: Modo::Normal,
            scopes: Vec::new(),
            tipos: Vec::new(),
            en_funcion: false,
            bucles: 0,
            archivo_actual: None,
        }
    }

    /// Guarda un error marcándolo con el archivo que se está generando, si no es el principal.
    fn registrar(&mut self, error: ErrorTraduccion) {
        let error = match (&error.archivo, &self.archivo_actual) {
            (None, Some(archivo)) => error.con_archivo(archivo),
            _ => error,
        };
        self.errores.push(error);
    }

    /// Genera una lista de sentencias sin parar en el primer error: los guarda y sigue.
    fn generar_sentencias(&mut self, sentencias: &[Sentencia]) {
        for sentencia in sentencias {
            self.generar_sentencia_registrando(sentencia);
        }
    }

    fn generar_sentencia_registrando(&mut self, sentencia: &Sentencia) {
        if let Err(error) = self.generar_sentencia(sentencia) {
            self.registrar(error);
        }
    }

    fn escribir(&mut self, texto: &str) {
        for _ in 0..self.nivel {
            self.salida.push_str("    ");
        }
        self.salida.push_str(texto);
        self.salida.push('\n');
    }

    fn escribir_linea_vacia(&mut self) {
        self.salida.push('\n');
    }

    fn recoger_funciones(&mut self, sentencias: &[Sentencia]) {
        for sentencia in sentencias {
            if let Sentencia::Funcion {
                nombre,
                parametros,
                linea,
                columna,
                archivo,
                ..
            } = sentencia
            {
                if es_integrada(nombre) {
                    self.registrar(marcar_archivo(
                        ErrorTraduccion::nuevo(
                            *linea,
                            format!("no se puede redefinir la función integrada '{}'", nombre),
                        )
                        .con_codigo("E012")
                        .en_columna(columna + "funcion ".chars().count())
                        .con_sugerencia("Elige otro nombre para tu función"),
                        archivo,
                    ));
                    continue;
                }
                if self.funciones.contains_key(nombre) {
                    self.registrar(marcar_archivo(
                        ErrorTraduccion::nuevo(
                            *linea,
                            format!("la función '{}' ya existe", nombre),
                        )
                        .con_codigo("E010")
                        .en_columna(*columna)
                        .con_sugerencia("Cada función necesita un nombre distinto"),
                        archivo,
                    ));
                    continue;
                }
                self.funciones.insert(nombre.clone(), parametros.clone());
            }
        }
    }

    /// Añade al programa el código Rust de cada extensión (una sola vez) y apunta sus
    /// funciones para poder llamarlas como si fueran integradas.
    fn recoger_extensiones(&mut self, extensiones: &[Extension]) {
        for extension in extensiones {
            self.escribir_linea_vacia();
            self.escribir(&format!("// DEIMON_EXTENSION: {}", extension.nombre));
            self.salida.push_str(&extension.codigo);
            if !extension.codigo.ends_with('\n') {
                self.salida.push('\n');
            }
            for funcion in &extension.funciones {
                self.extensiones
                    .insert(funcion.nombre.clone(), funcion.parametros.clone());
            }
        }
    }

    fn recoger_rutas(&mut self, sentencias: &[Sentencia]) {
        for sentencia in sentencias {
            if let Sentencia::Ruta {
                camino,
                cuerpo,
                linea,
                columna,
                archivo,
            } = sentencia
            {
                let error_ruta = |codigo: &'static str, mensaje: String, sugerencia: &str| {
                    marcar_archivo(
                        ErrorTraduccion::nuevo(*linea, mensaje)
                            .con_codigo(codigo)
                            .en_columna(*columna)
                            .con_sugerencia(sugerencia.to_string()),
                        archivo,
                    )
                };
                if !camino.starts_with('/') {
                    self.registrar(error_ruta(
                        "E013",
                        "el camino de la ruta debe empezar por '/'".to_string(),
                        "Por ejemplo: ruta \"/inicio\" { ... }",
                    ));
                    continue;
                }
                let mut parametros = Vec::new();
                let mut valida = true;
                for parte in camino.split('/') {
                    if let Some(nombre) = parte.strip_prefix(':') {
                        if !es_identificador(nombre) {
                            self.registrar(error_ruta(
                                "E013",
                                format!(
                                    "el parámetro '{}' de la ruta no es un nombre válido",
                                    parte
                                ),
                                "Un parámetro es ':' seguido de un nombre, por ejemplo ':nombre'",
                            ));
                            valida = false;
                            break;
                        }
                        if parametros.contains(&nombre.to_string()) {
                            self.registrar(error_ruta(
                                "E010",
                                format!("el parámetro '{}' está repetido en la ruta", nombre),
                                "Cada parámetro de la ruta necesita un nombre distinto",
                            ));
                            valida = false;
                            break;
                        }
                        parametros.push(nombre.to_string());
                    }
                }
                if !valida {
                    continue;
                }
                if self
                    .rutas
                    .iter()
                    .any(|existente| existente.camino == *camino)
                {
                    self.registrar(error_ruta(
                        "E010",
                        format!("la ruta '{}' ya existe", camino),
                        "Cada ruta debe tener un camino distinto",
                    ));
                    continue;
                }
                self.rutas.push(RutaDef {
                    camino: camino.clone(),
                    parametros,
                    cuerpo: cuerpo.clone(),
                    archivo: archivo.clone(),
                });
            }
        }
    }

    fn recoger_pruebas(&mut self, sentencias: &[Sentencia]) {
        for sentencia in sentencias {
            if let Sentencia::Prueba {
                nombre,
                cuerpo,
                linea,
                columna,
                archivo,
            } = sentencia
            {
                if self.pruebas.iter().any(|existente| existente.nombre == *nombre) {
                    self.registrar(marcar_archivo(
                        ErrorTraduccion::nuevo(
                            *linea,
                            format!("la prueba '{}' ya existe", nombre),
                        )
                        .con_codigo("E010")
                        .en_columna(*columna)
                        .con_sugerencia("Cada prueba necesita un nombre distinto"),
                        archivo,
                    ));
                    continue;
                }
                self.pruebas.push(PruebaDef {
                    nombre: nombre.clone(),
                    cuerpo: cuerpo.clone(),
                    archivo: archivo.clone(),
                });
            }
        }
    }

    fn generar_programa(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorTraduccion> {
        self.numericas = crate::numerico::funciones_numericas(sentencias);
        for sentencia in sentencias {
            if matches!(sentencia, Sentencia::Funcion { .. }) {
                if let Err(error) = self.generar_funcion(sentencia) {
                    self.registrar(error);
                }
            }
        }
        let rutas = self.rutas.clone();
        for (indice, ruta) in rutas.iter().enumerate() {
            self.generar_ruta(indice, ruta)?;
        }
        if !self.rutas.is_empty() {
            self.generar_tabla_rutas();
        }
        let pruebas = self.pruebas.clone();
        for (indice, prueba) in pruebas.iter().enumerate() {
            self.generar_prueba(indice, prueba)?;
        }
        self.generar_tabla_pruebas();
        self.generar_principal(sentencias)
    }

    fn generar_ruta(
        &mut self,
        indice: usize,
        ruta: &RutaDef,
    ) -> Result<(), ErrorTraduccion> {
        self.escribir_linea_vacia();
        self.escribir(&format!(
            "fn ruta_{}(parametros: Vec<String>) -> Valor {{",
            indice
        ));
        self.nivel += 1;

        let mut mapa = HashMap::new();
        let mut tipos = HashMap::new();
        for (posicion, parametro) in ruta.parametros.iter().enumerate() {
            mapa.insert(parametro.clone(), ());
            tipos.insert(parametro.clone(), TipoConocido::Texto);
            self.escribir(&format!(
                "let mut {}: Valor = Valor::Texto(Rc::new(parametros.get({}).cloned().unwrap_or_default()));",
                nombre_variable(parametro),
                posicion
            ));
        }

        let scopes_previos = std::mem::replace(&mut self.scopes, vec![mapa]);
        let tipos_previos = std::mem::replace(&mut self.tipos, vec![tipos]);
        let en_funcion_previo = std::mem::replace(&mut self.en_funcion, true);
        let bucles_previos = std::mem::replace(&mut self.bucles, 0);
        let archivo_previo =
            std::mem::replace(&mut self.archivo_actual, ruta.archivo.clone());

        self.generar_sentencias(&ruta.cuerpo);
        self.escribir("Valor::Nada");

        self.scopes = scopes_previos;
        self.tipos = tipos_previos;
        self.en_funcion = en_funcion_previo;
        self.bucles = bucles_previos;
        self.archivo_actual = archivo_previo;
        self.nivel -= 1;
        self.escribir("}");
        Ok(())
    }

    fn generar_prueba(
        &mut self,
        indice: usize,
        prueba: &PruebaDef,
    ) -> Result<(), ErrorTraduccion> {
        self.escribir_linea_vacia();
        self.escribir(&format!("fn prueba_{}() -> Valor {{", indice));
        self.nivel += 1;

        let scopes_previos = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
        let tipos_previos = std::mem::replace(&mut self.tipos, vec![HashMap::new()]);
        let en_funcion_previo = std::mem::replace(&mut self.en_funcion, true);
        let bucles_previos = std::mem::replace(&mut self.bucles, 0);
        let archivo_previo =
            std::mem::replace(&mut self.archivo_actual, prueba.archivo.clone());

        self.generar_sentencias(&prueba.cuerpo);
        self.escribir("Valor::Nada");

        self.scopes = scopes_previos;
        self.tipos = tipos_previos;
        self.en_funcion = en_funcion_previo;
        self.bucles = bucles_previos;
        self.archivo_actual = archivo_previo;
        self.nivel -= 1;
        self.escribir("}");
        Ok(())
    }

    fn generar_tabla_pruebas(&mut self) {
        self.escribir_linea_vacia();
        self.escribir("fn pruebas_definidas() -> Vec<(&'static str, fn() -> Valor)> {");
        self.nivel += 1;
        self.escribir("let mut pruebas: Vec<(&'static str, fn() -> Valor)> = Vec::new();");
        let pruebas = self.pruebas.clone();
        for (indice, prueba) in pruebas.iter().enumerate() {
            self.escribir(&format!(
                "pruebas.push(({:?}, prueba_{}));",
                prueba.nombre, indice
            ));
        }
        self.escribir("pruebas");
        self.nivel -= 1;
        self.escribir("}");
    }

    fn generar_tabla_rutas(&mut self) {
        self.escribir_linea_vacia();
        self.escribir("fn rutas_definidas() -> Vec<Ruta> {");
        self.nivel += 1;
        self.escribir("let mut rutas = Vec::new();");
        let rutas = self.rutas.clone();
        for (indice, ruta) in rutas.iter().enumerate() {
            self.escribir(&format!(
                "rutas.push(Ruta {{ camino: {:?}.to_string(), manejador: ruta_{} }});",
                ruta.camino, indice
            ));
        }
        self.escribir("rutas");
        self.nivel -= 1;
        self.escribir("}");
    }

    fn generar_funcion(&mut self, sentencia: &Sentencia) -> Result<(), ErrorTraduccion> {
        let (nombre, parametros, cuerpo, linea, columna, archivo) = match sentencia {
            Sentencia::Funcion {
                nombre,
                parametros,
                cuerpo,
                linea,
                columna,
                archivo,
            } => (nombre, parametros, cuerpo, *linea, *columna, archivo),
            _ => return Ok(()),
        };

        let mut mapa = HashMap::new();
        let mut tipos = HashMap::new();
        let mut parametros_rust = Vec::new();
        for parametro in parametros {
            if mapa.insert(parametro.clone(), ()).is_some() {
                self.registrar(
                    ErrorTraduccion::nuevo(
                        linea,
                        format!("el parámetro '{}' está repetido", parametro),
                    )
                    .con_codigo("E010")
                    .en_columna(columna)
                    .con_sugerencia("Cada parámetro necesita un nombre distinto"),
                );
                continue;
            }
            tipos.insert(parametro.clone(), TipoConocido::Otro);
            parametros_rust.push(format!("{}: Valor", nombre_variable(parametro)));
        }

        self.escribir_linea_vacia();
        self.escribir(&format!(
            "fn {}({}) -> Valor {{",
            nombre_funcion(nombre),
            parametros_rust.join(", ")
        ));
        self.nivel += 1;
        let numerica = self.numericas.get(nombre) == Some(&parametros.len());
        if numerica {
            // Con números, la versión rápida; con cualquier otra cosa, el cuerpo de siempre.
            let llamada = format!(
                "{}({})",
                crate::numerico::nombre_numerico(nombre),
                (0..parametros.len()).map(|i| format!("*n{}", i)).collect::<Vec<_>>().join(", ")
            );
            if parametros.is_empty() {
                self.escribir(&format!("return Valor::Numero({});", llamada));
            } else {
                let patrones: Vec<String> = (0..parametros.len()).map(|i| format!("Valor::Numero(n{})", i)).collect();
                let valores: Vec<String> = parametros.iter().map(|p| format!("&{}", nombre_variable(p))).collect();
                self.escribir(&format!(
                    "if let ({},) = ({},) {{ return Valor::Numero({}); }}",
                    patrones.join(", "),
                    valores.join(", "),
                    llamada
                ));
            }
        }

        let scopes_previos = std::mem::replace(&mut self.scopes, vec![mapa]);
        let tipos_previos = std::mem::replace(&mut self.tipos, vec![tipos]);
        let en_funcion_previo = std::mem::replace(&mut self.en_funcion, true);
        let bucles_previos = std::mem::replace(&mut self.bucles, 0);
        let archivo_previo =
            std::mem::replace(&mut self.archivo_actual, archivo.clone());

        self.generar_sentencias(&cuerpo);
        self.escribir("Valor::Nada");

        self.scopes = scopes_previos;
        self.tipos = tipos_previos;
        self.en_funcion = en_funcion_previo;
        self.bucles = bucles_previos;
        self.archivo_actual = archivo_previo;
        self.nivel -= 1;
        self.escribir("}");
        if numerica {
            self.salida.push_str(&crate::numerico::generar(nombre, parametros, cuerpo));
        }
        Ok(())
    }

    fn generar_principal(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorTraduccion> {
        self.escribir_linea_vacia();
        if self.modo == Modo::Pruebas {
            // Variante para `deimon probar`: el programa normal se traduce (para validarlo) pero
            // no se ejecuta; `main` solo corre las pruebas.
            self.escribir("fn programa_principal() {");
            self.generar_cuerpo_principal(sentencias)?;
            self.escribir("}");
            self.escribir_linea_vacia();
            self.escribir("fn main() {");
            self.nivel += 1;
            self.escribir("ejecutar_pruebas(pruebas_definidas());");
            self.nivel -= 1;
            self.escribir("}");
            return Ok(());
        }
        self.escribir("fn main() {");
        self.generar_cuerpo_principal(sentencias)?;
        self.escribir("}");
        Ok(())
    }

    fn generar_cuerpo_principal(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorTraduccion> {
        self.nivel += 1;
        // `deimon ejecutar` reemplaza su proceso por este programa; el ejecutable temporal se borra a sí mismo.
        self.escribir("if std::env::var_os(\"DEIMON_BORRAR_EJECUTABLE\").is_some() { if let Ok(ruta) = std::env::current_exe() { let _ = std::fs::remove_file(ruta); } }");
        self.scopes = vec![HashMap::new()];
        self.tipos = vec![HashMap::new()];
        self.en_funcion = false;
        self.bucles = 0;
        self.archivo_actual = None;

        for sentencia in sentencias {
            if !matches!(
                sentencia,
                Sentencia::Funcion { .. }
                    | Sentencia::Ruta { .. }
                    | Sentencia::Prueba { .. }
                    | Sentencia::UsarExtension { .. }
            ) {
                self.generar_sentencia_registrando(sentencia);
            }
        }

        self.nivel -= 1;
        Ok(())
    }

    fn generar_sentencia(&mut self, sentencia: &Sentencia) -> Result<(), ErrorTraduccion> {
        if let Some(linea) = linea_de_sentencia(sentencia) {
            self.escribir(&format!("en_linea({});", linea));
        }
        match sentencia {
            Sentencia::Declarar {
                nombre,
                valor,
                linea,
                columna,
            } => {
                let tipo = self.tipo_de_expresion(valor);
                let codigo = self.generar_expresion(valor)?;
                self.declarar(nombre, *linea, *columna)?;
                self.escribir(&format!(
                    "let mut {}: Valor = {};",
                    nombre_variable(nombre),
                    codigo
                ));
                self.anotar_tipo(nombre, tipo);
            }
            Sentencia::Asignar {
                nombre,
                valor,
                linea,
                columna,
            } => {
                if !self.variable_visible(nombre) {
                    self.registrar_asignacion_sin_declarar(nombre, *linea, *columna);
                }
                let tipo = self.tipo_de_expresion(valor);
                let destino = nombre_variable(nombre);
                let mut codigo = None;
                if let Expresion::Llamada {
                    nombre: funcion,
                    argumentos,
                    ..
                } = valor
                {
                    if funcion == "agregar" && argumentos.len() == 2 {
                        if let Expresion::Variable {
                            nombre: primero, ..
                        } = &argumentos[0]
                        {
                            if primero == nombre {
                                let valor_nuevo = self.generar_expresion(&argumentos[1])?;
                                codigo = Some(format!(
                                    "{{ let __ln_valor = {}; agregar_mover(std::mem::replace(&mut {}, Valor::Nada), __ln_valor) }}",
                                    valor_nuevo, destino
                                ));
                            }
                        }
                    }
                }
                if codigo.is_none() {
                    if let Expresion::Binaria {
                        izquierda,
                        operador: OperadorBinario::Suma,
                        derecha,
                        ..
                    } = valor
                    {
                        if let Expresion::Variable {
                            nombre: primero, ..
                        } = izquierda.as_ref()
                        {
                            if primero == nombre {
                                let valor_nuevo = self.generar_expresion(derecha)?;
                                codigo = Some(format!(
                                    "{{ let __ln_valor = {}; suma_mover(std::mem::replace(&mut {}, Valor::Nada), &__ln_valor) }}",
                                    valor_nuevo, destino
                                ));
                            }
                        }
                    }
                }
                let codigo = match codigo {
                    Some(codigo) => codigo,
                    None => self.generar_expresion(valor)?,
                };
                self.escribir(&format!("{} = {};", destino, codigo));
                self.anotar_tipo(nombre, tipo);
            }
            Sentencia::AsignarIndice {
                nombre,
                indices,
                valor,
                linea,
                columna,
            } => {
                if !self.variable_visible(nombre) {
                    self.registrar_asignacion_sin_declarar(nombre, *linea, *columna);
                }
                if self.tipo_de(nombre) == TipoConocido::Texto {
                    return Err(ErrorTraduccion::nuevo(
                        *linea,
                        "los textos no se pueden cambiar por partes; crea uno nuevo",
                    )
                    .con_codigo("E023")
                    .en_columna(*columna)
                    .con_sugerencia(
                        "Para cambiar un carácter, crea un texto nuevo (por ejemplo con +)",
                    ));
                }
                let mut indices_codigo = Vec::new();
                for indice in indices {
                    let codigo = self.generar_expresion(indice)?;
                    indices_codigo.push(format!(
                        "let __ln_indice_{} = {};",
                        indices_codigo.len(),
                        codigo
                    ));
                }
                let valor_codigo = self.generar_expresion(valor)?;
                let destino = nombre_variable(nombre);
                let mut codigo = format!(
                    "{{ {} let __ln_valor = {};",
                    indices_codigo.join(" "),
                    valor_codigo
                );
                let ultimo = indices.len() - 1;
                if ultimo == 0 {
                    codigo.push_str(&format!(
                        " poner_indice(&mut {}, &__ln_indice_0, __ln_valor); }}",
                        destino
                    ));
                } else {
                    codigo.push_str(&format!(" let mut __ln_destino = &mut {};", destino));
                    for posicion in 0..ultimo {
                        codigo.push_str(&format!(
                            " __ln_destino = indice_mut(__ln_destino, &__ln_indice_{});",
                            posicion
                        ));
                    }
                    codigo.push_str(&format!(
                        " poner_indice(__ln_destino, &__ln_indice_{}, __ln_valor); }}",
                        ultimo
                    ));
                }
                self.escribir(&format!("{};", codigo));
            }
            Sentencia::Si {
                condicion,
                entonces,
                sino,
                ..
            } => {
                let codigo = self.generar_referencia(condicion)?;
                self.escribir(&format!("if a_logico({}) {{", codigo));
                self.nivel += 1;
                self.abrir_scope();
                self.generar_sentencias(&entonces);
                self.cerrar_scope();
                self.nivel -= 1;
                if sino.is_empty() {
                    self.escribir("}");
                } else {
                    self.escribir("} else {");
                    self.nivel += 1;
                    self.abrir_scope();
                    self.generar_sentencias(&sino);
                    self.cerrar_scope();
                    self.nivel -= 1;
                    self.escribir("}");
                }
            }
            Sentencia::Mientras {
                condicion, cuerpo, ..
            } => {
                let codigo = self.generar_referencia(condicion)?;
                let linea = linea_de_sentencia(sentencia).unwrap_or(0);
                // Bucle que solo hace cuentas: si sus variables valen números, se recorre
                // con f64 directos y al final se devuelven; si no, el bucle de siempre.
                let numerico = crate::numerico::bucle_numerico(condicion, cuerpo, &self.numericas)
                    .filter(|libres| !libres.is_empty() && libres.iter().all(|v| self.variable_visible(v)));
                let numerico_abierto = numerico.is_some();
                if let Some(libres) = numerico {
                    let patrones: Vec<String> = (0..libres.len()).map(|i| format!("Valor::Numero(m{})", i)).collect();
                    let valores: Vec<String> = libres.iter().map(|v| format!("&{}", nombre_variable(v))).collect();
                    self.escribir(&format!("if let ({},) = ({},) {{", patrones.join(", "), valores.join(", ")));
                    self.nivel += 1;
                    let resultados: Vec<String> = (0..libres.len()).map(|i| format!("r{}", i)).collect();
                    self.escribir(&format!("let ({},) = {{", resultados.join(", ")));
                    self.nivel += 1;
                    for (i, v) in libres.iter().enumerate() {
                        self.escribir(&format!("let mut {}: f64 = *m{};", nombre_variable(v), i));
                    }
                    let bucle = crate::numerico::generar_bucle(condicion, cuerpo, self.nivel);
                    self.salida.push_str(&bucle);
                    let finales: Vec<String> = libres.iter().map(|v| nombre_variable(v)).collect();
                    self.escribir(&format!("({},)", finales.join(", ")));
                    self.nivel -= 1;
                    self.escribir("};");
                    for (i, v) in libres.iter().enumerate() {
                        self.escribir(&format!("{} = Valor::Numero(r{});", nombre_variable(v), i));
                    }
                    self.nivel -= 1;
                    self.escribir("} else {");
                    self.nivel += 1;
                }
                self.escribir(&format!(
                    "while {{ en_linea({}); a_logico({}) }} {{",
                    linea, codigo
                ));
                self.nivel += 1;
                self.abrir_scope();
                self.bucles += 1;
                self.generar_sentencias(&cuerpo);
                self.bucles -= 1;
                self.cerrar_scope();
                self.nivel -= 1;
                self.escribir("}");
                if numerico_abierto {
                    self.nivel -= 1;
                    self.escribir("}");
                }
            }
            Sentencia::Para {
                nombre,
                coleccion,
                cuerpo,
                linea,
            } => {
                let codigo = self.generar_expresion(coleccion)?;
                let nivel = self.nivel;
                self.escribir("{");
                self.nivel += 1;
                self.escribir(&format!("let elementos_{} = match {} {{", nivel, codigo));
                self.nivel += 1;
                self.escribir("Valor::Lista(elementos) => elementos,");
                self.escribir("Valor::Mapa(mapa) => match claves_de_mapa(&mapa) { Valor::Lista(claves) => claves, _ => unreachable!() },");
                self.escribir(
                    "otro => fallo(format!(\"para necesita una lista, no {}\", otro.tipo())),",
                );
                self.nivel -= 1;
                self.escribir("};");
                self.escribir(&format!(
                    "for elemento_{} in elementos_{}.iter().cloned() {{",
                    nivel, nivel
                ));
                self.nivel += 1;
                self.abrir_scope();
                self.declarar(nombre, *linea, 0)?;
                self.escribir(&format!(
                    "let mut {}: Valor = elemento_{};",
                    nombre_variable(nombre),
                    nivel
                ));
                self.bucles += 1;
                self.generar_sentencias(&cuerpo);
                self.bucles -= 1;
                self.cerrar_scope();
                self.nivel -= 1;
                self.escribir("}");
                self.nivel -= 1;
                self.escribir("}");
            }
            Sentencia::Funcion { linea, columna, .. } => {
                return Err(ErrorTraduccion::nuevo(
                    *linea,
                    "una función solo puede declararse en el nivel principal",
                )
                .con_codigo("E011")
                .en_columna(*columna)
                .con_sugerencia("Mueve la función fuera de este bloque, al nivel principal"));
            }
            Sentencia::Ruta { linea, columna, .. } => {
                return Err(ErrorTraduccion::nuevo(
                    *linea,
                    "una ruta solo puede declararse en el nivel principal",
                )
                .con_codigo("E011")
                .en_columna(*columna)
                .con_sugerencia("Mueve la ruta fuera de este bloque, al nivel principal"));
            }
            Sentencia::Prueba { linea, columna, .. } => {
                return Err(ErrorTraduccion::nuevo(
                    *linea,
                    "una prueba solo puede declararse en el nivel principal",
                )
                .con_codigo("E011")
                .en_columna(*columna)
                .con_sugerencia("Mueve la prueba fuera de este bloque, al nivel principal"));
            }
            Sentencia::Usar { linea, columna, .. } => {
                return Err(ErrorTraduccion::nuevo(
                    *linea,
                    "'usar' solo puede escribirse en el nivel principal",
                )
                .con_codigo("E011")
                .en_columna(*columna)
                .con_sugerencia("Mueve 'usar' fuera de este bloque, al nivel principal"));
            }
            Sentencia::Romper { linea, columna } | Sentencia::Continuar { linea, columna } => {
                let romper = matches!(sentencia, Sentencia::Romper { .. });
                let palabra = if romper { "romper" } else { "continuar" };
                if self.bucles == 0 {
                    return Err(ErrorTraduccion::nuevo(
                        *linea,
                        format!("'{}' solo se puede usar dentro de un bucle", palabra),
                    )
                    .con_codigo("E011")
                    .en_columna(*columna)
                    .con_sugerencia(format!(
                        "Usa '{}' dentro de un 'mientras' o de un 'para'",
                        palabra
                    )));
                }
                self.escribir(if romper { "break;" } else { "continue;" });
            }
            Sentencia::Devolver {
                valor,
                linea,
                columna,
            } => {
                if !self.en_funcion {
                    return Err(ErrorTraduccion::nuevo(
                        *linea,
                        "'devolver' solo se puede usar dentro de una función",
                    )
                    .con_codigo("E011")
                    .en_columna(*columna)
                    .con_sugerencia(
                        "Usa 'devolver' dentro de una funcion o de una ruta; en el nivel principal usa mostrar(...)",
                    ));
                }
                let codigo = match valor {
                    Some(Expresion::Variable {
                        nombre,
                        linea,
                        columna,
                    }) => {
                        if !self.variable_visible(nombre) {
                            self.registrar_variable_inexistente(nombre, *linea, *columna);
                        }
                        // La función termina aquí: la variable se puede devolver movida
                        // en vez de clonarla.
                        nombre_variable(nombre)
                    }
                    Some(valor) => self.generar_expresion(valor)?,
                    None => "Valor::Nada".to_string(),
                };
                self.escribir(&format!("return {};", codigo));
            }
            Sentencia::Expresion(expresion) => {
                let codigo = self.generar_expresion(expresion)?;
                self.escribir(&format!("let _ = {};", codigo));
            }
            Sentencia::UsarExtension { .. } => {}
        }
        Ok(())
    }

    fn generar_expresion(&mut self, expresion: &Expresion) -> Result<String, ErrorTraduccion> {
        match expresion {
            Expresion::Numero(numero) => Ok(format!("Valor::Numero({:?})", numero)),
            Expresion::Texto(texto) => {
                Ok(format!("Valor::Texto(Rc::new({:?}.to_string()))", texto))
            }
            Expresion::Logico(valor) => Ok(format!("Valor::Logico({})", valor)),
            Expresion::Nada => Ok("Valor::Nada".to_string()),
            Expresion::Lista(elementos) => {
                let mut partes = Vec::new();
                for elemento in elementos {
                    partes.push(self.generar_expresion(elemento)?);
                }
                Ok(format!("Valor::Lista(Rc::new(vec![{}]))", partes.join(", ")))
            }
            Expresion::Mapa(pares) => {
                let mut partes = Vec::new();
                for (clave, valor) in pares {
                    let clave = self.generar_expresion(clave)?;
                    let valor = self.generar_expresion(valor)?;
                    partes.push(format!("({}, {})", clave, valor));
                }
                Ok(format!("crear_mapa(vec![{}])", partes.join(", ")))
            }
            Expresion::Indice {
                coleccion,
                indice,
                ..
            } => {
                let coleccion_codigo = self.generar_referencia(coleccion)?;
                let indice_codigo = self.generar_referencia(indice)?;
                Ok(format!("indice({}, {})", coleccion_codigo, indice_codigo))
            }
            Expresion::Variable {
                nombre,
                linea,
                columna,
            } => {
                if !self.variable_visible(nombre) {
                    self.registrar_variable_inexistente(nombre, *linea, *columna);
                }
                Ok(format!("{}.clone()", nombre_variable(nombre)))
            }
            Expresion::Llamada {
                nombre,
                argumentos,
                linea,
                columna,
            } => {
                let (parametros, es_extension): (Vec<String>, bool) =
                    match integradas::buscar(nombre) {
                        Some(integrada) => (
                            integrada.parametros.iter().map(|p| p.to_string()).collect(),
                            false,
                        ),
                        None => match self.extensiones.get(nombre) {
                            Some(parametros) => (parametros.clone(), true),
                            None => match self.funciones.get(nombre) {
                                Some(parametros) => (parametros.clone(), false),
                                None => {
                                    self.registrar_funcion_inexistente(nombre, *linea, *columna);
                                    // Se siguen revisando los argumentos para encontrar más errores.
                                    for argumento in argumentos {
                                        self.generar_expresion(argumento)?;
                                    }
                                    return Ok("Valor::Nada".to_string());
                                }
                            },
                        },
                    };

                if argumentos.len() != parametros.len() {
                    let texto_esperados = if parametros.len() == 1 {
                        "1 argumento".to_string()
                    } else {
                        format!("{} argumentos", parametros.len())
                    };
                    self.registrar(
                        ErrorTraduccion::nuevo(
                            *linea,
                            format!(
                                "la función '{}' espera {} y se han pasado {}",
                                nombre,
                                texto_esperados,
                                argumentos.len()
                            ),
                        )
                        .con_codigo("E007")
                        .en_columna(*columna)
                        .con_sugerencia(format!(
                            "La firma es: {}({})",
                            nombre,
                            parametros.join(", ")
                        )),
                    );
                    for argumento in argumentos {
                        self.generar_expresion(argumento)?;
                    }
                    return Ok("Valor::Nada".to_string());
                }

                if es_extension {
                    let mut partes = Vec::new();
                    for argumento in argumentos {
                        partes.push(self.generar_expresion(argumento)?);
                    }
                    return Ok(format!(
                        "{{ en_linea({}); match ext_{}(vec![{}]) {{ Ok(valor) => valor, Err(mensaje) => fallo(mensaje) }} }}",
                        linea,
                        nombre,
                        partes.join(", ")
                    ));
                }
                if nombre == "elegir" {
                    // Solo se calcula la opción elegida, como `c ? a : b` en otros lenguajes.
                    let condicion = self.generar_referencia(&argumentos[0])?;
                    let si_verdad = self.generar_expresion(&argumentos[1])?;
                    let si_falso = self.generar_expresion(&argumentos[2])?;
                    return Ok(format!(
                        "{{ en_linea({}); if a_logico({}) {{ {} }} else {{ {} }} }}",
                        linea, condicion, si_verdad, si_falso
                    ));
                }
                if es_integrada(nombre) {
                    let mut partes = Vec::new();
                    for argumento in argumentos {
                        partes.push(self.generar_referencia(argumento)?);
                    }
                    if nombre == "comprobar" || nombre == "comprobar_igual" {
                        partes.push(linea.to_string());
                    }
                    if nombre == "visitar" || nombre == "enviar" {
                        if self.rutas.is_empty() {
                            return Err(ErrorTraduccion::nuevo(
                                *linea,
                                format!("no se puede usar '{}' sin rutas: define al menos una ruta", nombre),
                            )
                            .con_codigo("E012")
                            .en_columna(*columna)
                            .con_sugerencia("Define antes una ruta, por ejemplo: ruta \"/\" { devolver \"hola\" }"));
                        }
                        partes.push("rutas_definidas()".to_string());
                    }
                    if nombre == "servir" {
                        if self.rutas.is_empty() {
                            return Err(ErrorTraduccion::nuevo(
                                *linea,
                                "no se puede usar 'servir' sin rutas: define al menos una ruta",
                            )
                            .con_codigo("E012")
                            .en_columna(*columna)
                            .con_sugerencia("Define antes una ruta, por ejemplo: ruta \"/\" { devolver \"hola\" }"));
                        }
                        return Ok(format!("servir({}, rutas_definidas())", partes[0]));
                    }
                    return Ok(format!("{}({})", nombre, partes.join(", ")));
                }
                let mut partes = Vec::new();
                for argumento in argumentos {
                    partes.push(self.generar_expresion(argumento)?);
                }
                // Tras volver de una función del usuario se recupera la línea actual.
                Ok(format!(
                    "con_linea({}({}), {})",
                    nombre_funcion(nombre),
                    partes.join(", "),
                    linea
                ))
            }
            Expresion::Unaria {
                operador,
                valor,
                ..
            } => {
                let codigo = self.generar_referencia(valor)?;
                let funcion = match operador {
                    OperadorUnario::Negativo => "negativo",
                    OperadorUnario::No => "no_logico",
                };
                Ok(format!("{}({})", funcion, codigo))
            }
            Expresion::Binaria {
                izquierda,
                operador,
                derecha,
                ..
            } => {
                let izquierda_codigo = self.generar_referencia(izquierda)?;
                let derecha_codigo = self.generar_referencia(derecha)?;
                let funcion = match operador {
                    OperadorBinario::Suma => "suma",
                    OperadorBinario::Resta => "resta",
                    OperadorBinario::Multiplicacion => "multiplicacion",
                    OperadorBinario::Division => "division",
                    OperadorBinario::Resto => "resto",
                    OperadorBinario::Igual => "igual",
                    OperadorBinario::Distinto => "distinto",
                    OperadorBinario::Menor => "menor",
                    OperadorBinario::MenorIgual => "menor_igual",
                    OperadorBinario::Mayor => "mayor",
                    OperadorBinario::MayorIgual => "mayor_igual",
                    OperadorBinario::Y => {
                        return Ok(format!(
                            "Valor::Logico(a_logico({}) && a_logico({}))",
                            izquierda_codigo, derecha_codigo
                        ));
                    }
                    OperadorBinario::O => {
                        return Ok(format!(
                            "Valor::Logico(a_logico({}) || a_logico({}))",
                            izquierda_codigo, derecha_codigo
                        ));
                    }
                };
                Ok(format!("{}({}, {})", funcion, izquierda_codigo, derecha_codigo))
            }
        }
    }

    /// Genera una expresión que se puede pasar prestada (`&Valor`). Las variables se prestan
    /// sin clonar; el resto se calcula en un temporal que vive hasta el final de la sentencia.
    fn generar_referencia(
        &mut self,
        expresion: &Expresion,
    ) -> Result<String, ErrorTraduccion> {
        match expresion {
            Expresion::Variable {
                nombre,
                linea,
                columna,
            } => {
                if !self.variable_visible(nombre) {
                    self.registrar_variable_inexistente(nombre, *linea, *columna);
                }
                Ok(format!("&{}", nombre_variable(nombre)))
            }
            _ => Ok(format!("&({})", self.generar_expresion(expresion)?)),
        }
    }

    fn abrir_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.tipos.push(HashMap::new());
    }

    fn cerrar_scope(&mut self) {
        self.scopes.pop();
        self.tipos.pop();
    }

    fn anotar_tipo(&mut self, nombre: &str, tipo: TipoConocido) {
        if let Some(scope) = self.tipos.last_mut() {
            scope.insert(nombre.to_string(), tipo);
        }
    }

    fn tipo_de(&self, nombre: &str) -> TipoConocido {
        for scope in self.tipos.iter().rev() {
            if let Some(tipo) = scope.get(nombre) {
                return *tipo;
            }
        }
        TipoConocido::Otro
    }

    fn tipo_de_expresion(&self, expresion: &Expresion) -> TipoConocido {
        match expresion {
            Expresion::Texto(_) => TipoConocido::Texto,
            Expresion::Lista(_) => TipoConocido::Lista,
            Expresion::Variable { nombre, .. } => self.tipo_de(nombre),
            Expresion::Llamada { nombre, .. } if nombre == "a_texto" => TipoConocido::Texto,
            Expresion::Llamada { nombre, .. } if nombre == "agregar" => TipoConocido::Lista,
            Expresion::Binaria {
                izquierda,
                operador: OperadorBinario::Suma,
                derecha,
                ..
            } => {
                let izquierda = self.tipo_de_expresion(izquierda);
                let derecha = self.tipo_de_expresion(derecha);
                if izquierda == TipoConocido::Texto || derecha == TipoConocido::Texto {
                    TipoConocido::Texto
                } else if izquierda == TipoConocido::Lista && derecha == TipoConocido::Lista {
                    TipoConocido::Lista
                } else {
                    TipoConocido::Otro
                }
            }
            _ => TipoConocido::Otro,
        }
    }

    fn declarar(&mut self, nombre: &str, linea: usize, columna: usize) -> Result<(), ErrorTraduccion> {
        let scope = self
            .scopes
            .last_mut()
            .expect("siempre hay al menos un ámbito abierto");
        if scope.contains_key(nombre) {
            return Err(ErrorTraduccion::nuevo(
                linea,
                format!("la variable '{}' ya existe en este ámbito", nombre),
            )
            .con_codigo("E010")
            .en_columna(columna)
            .con_sugerencia(format!(
                "Para cambiar su valor escribe: {} = ...; sin 'sea'",
                nombre
            )));
        }
        scope.insert(nombre.to_string(), ());
        Ok(())
    }

    fn variable_visible(&self, nombre: &str) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key(nombre))
    }

    fn variables_visibles(&self) -> Vec<String> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.keys().cloned())
            .collect()
    }

    fn registrar_variable_inexistente(&mut self, nombre: &str, linea: usize, columna: usize) {
        let mut error = ErrorTraduccion::nuevo(linea, format!("la variable '{}' no existe", nombre))
            .con_codigo("E005")
            .en_columna(columna);
        match nombre_parecido(nombre, &self.variables_visibles()) {
            Some(parecido) => {
                error = error
                    .con_sugerencia(format!("¿Quisiste decir '{}'?", parecido))
                    .con_arreglo(Arreglo {
                        linea,
                        columna_inicio: columna,
                        columna_fin: columna + nombre.chars().count(),
                        reemplazo: parecido,
                    });
            }
            None => {
                error = error.con_sugerencia(format!(
                    "Decláralo antes con: sea {} = ...",
                    nombre
                ));
            }
        }
        self.registrar(error);
    }

    fn registrar_asignacion_sin_declarar(&mut self, nombre: &str, linea: usize, columna: usize) {
        let mut error = ErrorTraduccion::nuevo(linea, format!("la variable '{}' no existe", nombre))
            .con_codigo("E009")
            .en_columna(columna);
        match nombre_parecido(nombre, &self.variables_visibles()) {
            Some(parecido) => {
                error = error
                    .con_sugerencia(format!(
                        "¿Quisiste decir '{}'? Si es una variable nueva, declárala con: sea {} = ...",
                        parecido, nombre
                    ))
                    .con_arreglo(Arreglo {
                        linea,
                        columna_inicio: columna,
                        columna_fin: columna + nombre.chars().count(),
                        reemplazo: parecido,
                    });
            }
            None => {
                error = error.con_sugerencia(format!(
                    "Una variable se declara con 'sea' antes de asignarla: sea {} = ...",
                    nombre
                ));
            }
        }
        self.registrar(error);
    }

    fn registrar_funcion_inexistente(&mut self, nombre: &str, linea: usize, columna: usize) {
        let mut candidatas: Vec<String> = self.funciones.keys().cloned().collect();
        candidatas.extend(self.extensiones.keys().cloned());
        candidatas.extend(integradas::INTEGRADAS.iter().map(|i| i.nombre.to_string()));
        let mut error = ErrorTraduccion::nuevo(linea, format!("la función '{}' no existe", nombre))
            .con_codigo("E006")
            .en_columna(columna);
        match nombre_parecido(nombre, &candidatas) {
            Some(parecido) => {
                error = error
                    .con_sugerencia(format!("¿Quisiste decir '{}'?", parecido))
                    .con_arreglo(Arreglo {
                        linea,
                        columna_inicio: columna,
                        columna_fin: columna + nombre.chars().count(),
                        reemplazo: parecido,
                    });
            }
            None => {
                error = error.con_sugerencia(
                    "Defínela con: funcion nombre(a, b) { ... } o mira las integradas con 'deimon guia'",
                );
            }
        }
        self.registrar(error);
    }
}

/// El nombre más parecido (distancia de edición <= 2) entre los visibles.
fn nombre_parecido(nombre: &str, candidatos: &[String]) -> Option<String> {
    let largo = nombre.chars().count();
    candidatos
        .iter()
        .map(|candidato| (distancia_edicion(nombre, candidato), candidato))
        .filter(|(distancia, candidato)| {
            *distancia > 0 && *distancia <= 2 && *distancia < largo.max(candidato.chars().count())
        })
        .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)))
        .map(|(_, candidato)| candidato.clone())
}

pub(crate) fn distancia_edicion(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut anterior: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut actual = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let costo = usize::from(a[i - 1] != b[j - 1]);
            actual[j] = (anterior[j] + 1)
                .min(actual[j - 1] + 1)
                .min(anterior[j - 1] + costo);
        }
        anterior = actual;
    }
    anterior[b.len()]
}

fn linea_de_sentencia(sentencia: &Sentencia) -> Option<usize> {
    match sentencia {
        Sentencia::Declarar { linea, .. }
        | Sentencia::Asignar { linea, .. }
        | Sentencia::AsignarIndice { linea, .. }
        | Sentencia::Si { linea, .. }
        | Sentencia::Mientras { linea, .. }
        | Sentencia::Para { linea, .. }
        | Sentencia::Devolver { linea, .. }
        | Sentencia::Romper { linea, .. }
        | Sentencia::Continuar { linea, .. } => Some(*linea),
        Sentencia::Expresion(expresion) => match expresion {
            Expresion::Variable { linea, .. }
            | Expresion::Llamada { linea, .. }
            | Expresion::Unaria { linea, .. }
            | Expresion::Binaria { linea, .. }
            | Expresion::Indice { linea, .. } => Some(*linea),
            _ => None,
        },
        Sentencia::Usar { linea, .. } | Sentencia::UsarExtension { linea, .. } => Some(*linea),
        Sentencia::Funcion { .. } | Sentencia::Ruta { .. } | Sentencia::Prueba { .. } => None,
    }
}

fn es_integrada(nombre: &str) -> bool {
    integradas::aridad(nombre).is_some()
}

fn es_identificador(nombre: &str) -> bool {
    let mut caracteres = nombre.chars();
    match caracteres.next() {
        Some(primero) if primero.is_alphabetic() || primero == '_' => {
            caracteres.all(|caracter| caracter.is_alphanumeric() || caracter == '_')
        }
        _ => false,
    }
}

fn codificar(nombre: &str) -> String {
    let mut resultado = String::new();
    for caracter in nombre.chars() {
        if caracter.is_ascii_alphanumeric() || caracter == '_' {
            resultado.push(caracter);
        } else {
            resultado.push_str(&format!("_u{:x}_", caracter as u32));
        }
    }
    resultado
}

pub(crate) fn nombre_variable(nombre: &str) -> String {
    format!("var_{}", codificar(nombre))
}

pub(crate) fn nombre_funcion(nombre: &str) -> String {
    format!("fn_{}", codificar(nombre))
}
