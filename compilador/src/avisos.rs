//! Avisos de `deimon comprobar --estricto` (códigos A001 a A007 y A009; el A008 lo da `tipos`).
//!
//! Un aviso señala algo que probablemente es un fallo pero no es seguro: una variable que nunca
//! se lee, una función que nadie llama, código después de un `devolver`... Solo se revisa lo
//! escrito en el archivo principal (las funciones de los archivos usados y de los módulos no
//! se avisan, aunque sí cuentan como usos; y A003 solo se da en un programa con instrucciones
//! sueltas o rutas, no en una pieza de funciones). Una variable o un parámetro cuyo nombre empieza por
//! `_` no da A001 ni A002.

use std::collections::{HashMap, HashSet};

use crate::ast::{Expresion, OperadorBinario, ParteTexto, Programa, Sentencia};
use crate::error::ErrorTraduccion;
use crate::tipos::es_tipo_propio;

/// Integradas que solo devuelven un valor y no hacen nada más: descartar su resultado es un
/// fallo casi seguro. Ninguna toca disco, red, datos, sesión ni pantalla, y las que llaman a una
/// función del usuario (`filtrar`, `transformar`, `elegir`) quedan fuera.
const PURAS_INTEGRADAS: &[&str] = &[
    "a_numero",
    "a_texto",
    "medir",
    "agregar",
    "incluye",
    "ordenar",
    "ordenar_por",
    "invertir",
    "cortar",
    "rango",
    "sumar_lista",
    "tiene_clave",
    "quitar_clave",
    "proteger_html",
    "rellenar_plantilla",
];

/// Lo mismo para las funciones de los módulos `texto`, `matematicas`, `json`, `tiempo` y
/// `patrones` (sin `aleatorio`, `entero_al_azar`, `ahora` ni `esperar`).
const PURAS_MODULOS: &[&str] = &[
    "mayusculas",
    "minusculas",
    "recortar",
    "contiene",
    "reemplazar",
    "partir",
    "empieza_con",
    "termina_con",
    "repetir",
    "unir",
    "formatear_texto",
    "formatear_numero",
    "rellenar_texto",
    "primera_mayuscula",
    "texto_completo",
    "raiz",
    "potencia",
    "absoluto",
    "redondear",
    "suelo",
    "techo",
    "minimo",
    "maximo",
    "a_json",
    "leer_json",
    "listar_claves_json",
    "desde_json",
    "fecha",
    "crear_fecha",
    "leer_fecha",
    "formatear_fecha",
    "sumar_tiempo",
    "diferencia_fechas",
    "dia_de_la_semana",
    "es_bisiesto",
    "dias_del_mes",
    "coincide_patron",
    "buscar_patron",
    "buscar_todos_patron",
    "reemplazar_patron",
    "partir_patron",
    "buscar_columnas_patron",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Clase {
    Variable,
    Parametro,
    /// Variable de `para`, de `si falla` o parámetro de ruta: no se avisa.
    Otra,
}

struct Variable {
    nombre: String,
    linea: usize,
    columna: usize,
    leida: bool,
    clase: Clase,
}

struct Analisis<'a> {
    avisos: Vec<ErrorTraduccion>,
    variables: Vec<Variable>,
    ambitos: Vec<HashMap<String, usize>>,
    /// Nombres de las funciones escritas por el usuario (de cualquier archivo).
    propias: HashSet<&'a str>,
    /// Nombres de las funciones de los módulos cargados.
    de_modulos: HashSet<&'a str>,
    /// Dentro del cuerpo de un `intentar`: llamar a `a_numero(x)` solo para ver si falla es normal.
    en_intentar: usize,
}

/// Todos los avisos del archivo principal, sin ordenar.
pub fn revisar(programa: &Programa) -> Vec<ErrorTraduccion> {
    let propias = programa
        .sentencias
        .iter()
        .filter_map(|s| match s {
            Sentencia::Funcion { nombre, .. } => Some(nombre.as_str()),
            _ => None,
        })
        .collect();
    let de_modulos = programa
        .extensiones
        .iter()
        .flat_map(|e| e.funciones.iter().map(|f| f.nombre.as_str()))
        .collect();
    let mut analisis = Analisis {
        avisos: Vec::new(),
        variables: Vec::new(),
        ambitos: Vec::new(),
        propias,
        de_modulos,
        en_intentar: 0,
    };

    // Qué nombres usa cada pieza (cada función, ruta o prueba, y todo lo suelto junto).
    let mut usos: Vec<HashSet<String>> = Vec::new();
    let mut sueltas: Vec<Sentencia> = Vec::new();
    let mut piezas: Vec<Option<usize>> = Vec::new();
    for sentencia in &programa.sentencias {
        match sentencia {
            Sentencia::Funcion { cuerpo, .. } | Sentencia::Ruta { cuerpo, .. } | Sentencia::Prueba { cuerpo, .. } => {
                let mut nombres = HashSet::new();
                nombres_en(cuerpo, &mut nombres);
                piezas.push(Some(usos.len()));
                usos.push(nombres);
            }
            otra => {
                piezas.push(None);
                sueltas.push(otra.clone());
            }
        }
    }
    let mut de_lo_suelto = HashSet::new();
    nombres_en(&sueltas, &mut de_lo_suelto);
    usos.push(de_lo_suelto);

    // Un archivo que solo trae funciones y pruebas (una pieza para `usar`) no se avisa de A003:
    // sus funciones las llama otro archivo.
    let es_programa = !sueltas.is_empty()
        || programa
            .sentencias
            .iter()
            .any(|s| matches!(s, Sentencia::Ruta { archivo: None, .. }));

    for (sentencia, pieza) in programa.sentencias.iter().zip(&piezas) {
        match sentencia {
            Sentencia::Funcion { nombre, parametros, cuerpo, linea, columna, archivo: None } => {
                if es_tipo_propio(sentencia) {
                    continue;
                }
                analisis.funcion(parametros, cuerpo, *linea, *columna);
                let pieza = pieza.expect("las funciones tienen pieza");
                let usada = usos.iter().enumerate().any(|(i, nombres)| i != pieza && nombres.contains(nombre));
                if es_programa && !usada {
                    analisis.aviso(
                        "A003",
                        *linea,
                        *columna,
                        format!("la función '{}' nunca se llama", nombre),
                        "Llámala desde algún sitio, bórrala si ya no hace falta o prueba que funciona con una prueba \"...\" { ... }",
                    );
                }
                if devuelve_valor(cuerpo) && !termina(cuerpo) {
                    analisis.aviso(
                        "A006",
                        *linea,
                        *columna,
                        format!("la función '{}' devuelve un valor en unos casos, pero en otros llega al final sin devolver nada", nombre),
                        "Pon un 'devolver' al final de todos los caminos (o 'devolver nada' si es a propósito)",
                    );
                }
            }
            Sentencia::Ruta { camino, cuerpo, archivo: None, .. } => {
                analisis.abrir();
                for parte in camino.split('/') {
                    if let Some(nombre) = parte.strip_prefix(':') {
                        analisis.declarar(nombre, 0, 0, Clase::Otra);
                    }
                }
                analisis.sentencias(cuerpo);
                analisis.cerrar();
            }
            Sentencia::Prueba { cuerpo, archivo: None, .. } => {
                analisis.abrir();
                analisis.sentencias(cuerpo);
                analisis.cerrar();
            }
            _ => {}
        }
    }
    analisis.abrir();
    analisis.sentencias(&sueltas);
    analisis.cerrar();
    analisis.avisos
}

impl<'a> Analisis<'a> {
    fn aviso(&mut self, codigo: &'static str, linea: usize, columna: usize, mensaje: String, sugerencia: &str) {
        self.avisos.push(
            ErrorTraduccion::nuevo(linea, mensaje)
                .con_codigo(codigo)
                .en_columna(columna)
                .con_sugerencia(sugerencia),
        );
    }

    fn abrir(&mut self) {
        self.ambitos.push(HashMap::new());
    }

    /// Cierra el ámbito y avisa de lo que se declaró en él y nunca se leyó (A001 y A002).
    fn cerrar(&mut self) {
        let ambito = self.ambitos.pop().expect("hay un ámbito abierto");
        let mut sin_leer: Vec<usize> = ambito
            .into_values()
            .filter(|&i| {
                let v = &self.variables[i];
                !v.leida && v.clase != Clase::Otra && !v.nombre.starts_with('_')
            })
            .collect();
        sin_leer.sort_unstable();
        for i in sin_leer {
            let (nombre, linea, columna, clase) = {
                let v = &self.variables[i];
                (v.nombre.clone(), v.linea, v.columna, v.clase)
            };
            if clase == Clase::Parametro {
                self.aviso(
                    "A002",
                    linea,
                    columna,
                    format!("el parámetro '{}' nunca se usa", nombre),
                    &format!("Quítalo de la función o, si debe estar, ponle un nombre que empiece por _ (_{})", nombre),
                );
            } else {
                self.aviso(
                    "A001",
                    linea,
                    columna,
                    format!("la variable '{}' se declara pero nunca se lee", nombre),
                    &format!("Usa '{}' o bórrala; si es a propósito, ponle un nombre que empiece por _ (_{})", nombre, nombre),
                );
            }
        }
    }

    fn declarar(&mut self, nombre: &str, linea: usize, columna: usize, clase: Clase) {
        self.variables.push(Variable { nombre: nombre.to_string(), linea, columna, leida: false, clase });
        let indice = self.variables.len() - 1;
        self.ambitos.last_mut().expect("hay un ámbito abierto").insert(nombre.to_string(), indice);
    }

    fn leer(&mut self, nombre: &str) {
        for ambito in self.ambitos.iter().rev() {
            if let Some(&i) = ambito.get(nombre) {
                self.variables[i].leida = true;
                return;
            }
        }
    }

    fn funcion(&mut self, parametros: &[String], cuerpo: &[Sentencia], linea: usize, columna: usize) {
        self.abrir();
        for parametro in parametros {
            self.declarar(parametro, linea, columna, Clase::Parametro);
        }
        self.sentencias(cuerpo);
        self.cerrar();
    }

    fn sentencias(&mut self, lista: &[Sentencia]) {
        let mut avisado = false;
        for (i, sentencia) in lista.iter().enumerate() {
            if i > 0 && !avisado {
                if let Some(motivo) = terminador(&lista[i - 1]) {
                    let linea = linea_de(sentencia).or_else(|| linea_de(&lista[i - 1])).unwrap_or(0);
                    self.aviso(
                        "A004",
                        linea,
                        0,
                        format!("esta línea nunca se ejecuta: la sentencia anterior ({}) sale antes de llegar aquí", motivo),
                        "Borra el código que sobra o ponlo antes de esa sentencia",
                    );
                    avisado = true;
                }
            }
            self.sentencia(sentencia);
        }
    }

    fn bloque(&mut self, lista: &[Sentencia]) {
        self.abrir();
        self.sentencias(lista);
        self.cerrar();
    }

    fn sentencia(&mut self, sentencia: &Sentencia) {
        match sentencia {
            Sentencia::Declarar { nombre, valor, linea, columna } => {
                self.expresion(valor);
                let fuera = self.ambitos[..self.ambitos.len() - 1].iter().any(|a| a.contains_key(nombre));
                if fuera && !self.ambitos.last().is_some_and(|a| a.contains_key(nombre)) {
                    self.aviso(
                        "A005",
                        *linea,
                        *columna,
                        format!("'sea {}' dentro de este bloque crea otra variable distinta y tapa la '{}' de fuera, que no cambia", nombre, nombre),
                        &format!("Para cambiar la de fuera escribe {} = ...; si quieres otra distinta, ponle otro nombre", nombre),
                    );
                }
                self.declarar(nombre, *linea, *columna, Clase::Variable);
            }
            Sentencia::Asignar { valor, .. } => self.expresion(valor),
            Sentencia::AsignarIndice { nombre, indices, valor, .. } => {
                self.leer(nombre);
                for indice in indices {
                    self.expresion(indice);
                }
                self.expresion(valor);
            }
            Sentencia::Si { condicion, entonces, sino, linea } => {
                match constante(condicion) {
                    Some(true) => self.aviso(
                        "A009",
                        *linea,
                        0,
                        "la condición del 'si' siempre es verdadero: el 'sino' (si lo hay) nunca se ejecuta".to_string(),
                        "Quita el 'si' y deja el código directamente, o usa una condición que dependa de un valor",
                    ),
                    Some(false) => self.aviso(
                        "A009",
                        *linea,
                        0,
                        "la condición del 'si' siempre es falso: el bloque de dentro nunca se ejecuta".to_string(),
                        "Borra el bloque o usa una condición que dependa de un valor",
                    ),
                    None => {}
                }
                self.expresion(condicion);
                self.bloque(entonces);
                if !sino.is_empty() {
                    self.bloque(sino);
                }
            }
            Sentencia::Intentar { cuerpo, nombre, captura, .. } => {
                self.en_intentar += 1;
                self.bloque(cuerpo);
                self.en_intentar -= 1;
                self.abrir();
                self.declarar(nombre, 0, 0, Clase::Otra);
                self.sentencias(captura);
                self.cerrar();
            }
            Sentencia::Mientras { condicion, cuerpo, linea } => {
                if constante(condicion) == Some(false) {
                    self.aviso(
                        "A009",
                        *linea,
                        0,
                        "la condición del 'mientras' siempre es falso: el bucle nunca se ejecuta".to_string(),
                        "Borra el bucle o usa una condición que dependa de un valor",
                    );
                }
                self.expresion(condicion);
                self.bloque(cuerpo);
            }
            Sentencia::Para { nombre, coleccion, cuerpo, .. } => {
                self.expresion(coleccion);
                self.abrir();
                self.declarar(nombre, 0, 0, Clase::Otra);
                self.sentencias(cuerpo);
                self.cerrar();
            }
            Sentencia::Devolver { valor: Some(valor), .. } => self.expresion(valor),
            Sentencia::Expresion(expresion) => {
                if let Expresion::Llamada { nombre, argumentos, linea, columna } = expresion {
                    if self.en_intentar == 0 && self.es_pura(nombre) {
                        let sugerencia = match argumentos.first() {
                            Some(Expresion::Variable { nombre: variable, .. }) => {
                                let resto = if argumentos.len() > 1 { ", ..." } else { "" };
                                format!("Guarda el resultado: {} = {}({}{})", variable, nombre, variable, resto)
                            }
                            _ => format!("Guarda el resultado: sea resultado = {}(...)", nombre),
                        };
                        self.aviso(
                            "A007",
                            *linea,
                            *columna,
                            format!("el resultado de {}(...) se pierde: esta función no cambia nada, solo devuelve un valor", nombre),
                            &sugerencia,
                        );
                    }
                }
                self.expresion(expresion);
            }
            _ => {}
        }
    }

    fn es_pura(&self, nombre: &str) -> bool {
        if self.propias.contains(nombre) {
            return false;
        }
        PURAS_INTEGRADAS.contains(&nombre) || (PURAS_MODULOS.contains(&nombre) && self.de_modulos.contains(nombre))
    }

    fn expresion(&mut self, expresion: &Expresion) {
        match expresion {
            Expresion::Variable { nombre, .. } => self.leer(nombre),
            Expresion::Llamada { nombre, argumentos, .. } => {
                // `f(x)` puede llamar a la función que haya guardada en una variable `f`.
                self.leer(nombre);
                for argumento in argumentos {
                    self.expresion(argumento);
                }
            }
            Expresion::TextoConValores { partes, .. } => {
                for parte in partes {
                    if let ParteTexto::Valor(valor) = parte {
                        self.expresion(valor);
                    }
                }
            }
            Expresion::Lista(elementos) => elementos.iter().for_each(|e| self.expresion(e)),
            Expresion::Mapa(pares) => {
                for (clave, valor) in pares {
                    self.expresion(clave);
                    self.expresion(valor);
                }
            }
            Expresion::Unaria { valor, .. } => self.expresion(valor),
            Expresion::Binaria { izquierda, derecha, .. } => {
                self.expresion(izquierda);
                self.expresion(derecha);
            }
            Expresion::Indice { coleccion, indice, .. } => {
                self.expresion(coleccion);
                self.expresion(indice);
            }
            Expresion::Numero(_) | Expresion::EnteroGrande(_) | Expresion::Texto(_) | Expresion::Logico(_) | Expresion::Nada => {}
        }
    }
}

/// Qué sentencia deja sin ejecutar lo que le sigue en el mismo bloque.
fn terminador(sentencia: &Sentencia) -> Option<&'static str> {
    match sentencia {
        Sentencia::Devolver { .. } => Some("devolver"),
        Sentencia::Romper { .. } => Some("romper"),
        Sentencia::Continuar { .. } => Some("continuar"),
        Sentencia::Expresion(Expresion::Llamada { nombre, .. }) if nombre == "provocar_error" => Some("provocar_error"),
        _ => None,
    }
}

fn linea_de_expresion(expresion: &Expresion) -> Option<usize> {
    match expresion {
        Expresion::Variable { linea, .. }
        | Expresion::Llamada { linea, .. }
        | Expresion::Unaria { linea, .. }
        | Expresion::Binaria { linea, .. }
        | Expresion::Indice { linea, .. }
        | Expresion::TextoConValores { linea, .. } => Some(*linea),
        Expresion::Lista(elementos) => elementos.iter().find_map(linea_de_expresion),
        Expresion::Mapa(pares) => pares.iter().find_map(|(clave, valor)| linea_de_expresion(clave).or_else(|| linea_de_expresion(valor))),
        _ => None,
    }
}

fn linea_de(sentencia: &Sentencia) -> Option<usize> {
    match sentencia {
        Sentencia::Declarar { linea, .. }
        | Sentencia::Usar { linea, .. }
        | Sentencia::UsarExtension { linea, .. }
        | Sentencia::Asignar { linea, .. }
        | Sentencia::AsignarIndice { linea, .. }
        | Sentencia::Si { linea, .. }
        | Sentencia::Intentar { linea, .. }
        | Sentencia::Mientras { linea, .. }
        | Sentencia::Para { linea, .. }
        | Sentencia::Funcion { linea, .. }
        | Sentencia::Ruta { linea, .. }
        | Sentencia::Prueba { linea, .. }
        | Sentencia::Devolver { linea, .. }
        | Sentencia::Romper { linea, .. }
        | Sentencia::Continuar { linea, .. } => Some(*linea),
        Sentencia::Expresion(expresion) => linea_de_expresion(expresion),
    }
}

/// ¿Alguna sentencia de la lista (o de sus bloques) hace `devolver valor`?
fn devuelve_valor(lista: &[Sentencia]) -> bool {
    lista.iter().any(|sentencia| match sentencia {
        Sentencia::Devolver { valor: Some(valor), .. } => *valor != Expresion::Nada,
        Sentencia::Si { entonces, sino, .. } => devuelve_valor(entonces) || devuelve_valor(sino),
        Sentencia::Mientras { cuerpo, .. } | Sentencia::Para { cuerpo, .. } => devuelve_valor(cuerpo),
        Sentencia::Intentar { cuerpo, captura, .. } => devuelve_valor(cuerpo) || devuelve_valor(captura),
        _ => false,
    })
}

/// ¿Hay un `romper` que sale de este bucle (sin contar los de los bucles de dentro)?
fn tiene_romper(lista: &[Sentencia]) -> bool {
    lista.iter().any(|sentencia| match sentencia {
        Sentencia::Romper { .. } => true,
        Sentencia::Si { entonces, sino, .. } => tiene_romper(entonces) || tiene_romper(sino),
        Sentencia::Intentar { cuerpo, captura, .. } => tiene_romper(cuerpo) || tiene_romper(captura),
        _ => false,
    })
}

/// ¿Acaba siempre con `devolver` o `provocar_error` (o en un `mientras verdadero` sin salida),
/// sin poder llegar al final de la lista?
fn termina(lista: &[Sentencia]) -> bool {
    lista.iter().any(|sentencia| match sentencia {
        Sentencia::Devolver { .. } => true,
        Sentencia::Expresion(Expresion::Llamada { nombre, .. }) => nombre == "provocar_error",
        Sentencia::Si { entonces, sino, .. } => termina(entonces) && termina(sino),
        Sentencia::Intentar { cuerpo, captura, .. } => termina(cuerpo) && termina(captura),
        Sentencia::Mientras { condicion, cuerpo, .. } => constante(condicion) == Some(true) && !tiene_romper(cuerpo),
        _ => false,
    })
}

enum Literal<'a> {
    Numero(f64),
    Texto(&'a str),
    Logico(bool),
}

fn literal(expresion: &Expresion) -> Option<Literal<'_>> {
    match expresion {
        Expresion::Numero(n) => Some(Literal::Numero(*n)),
        Expresion::Texto(t) => Some(Literal::Texto(t)),
        Expresion::Logico(b) => Some(Literal::Logico(*b)),
        _ => None,
    }
}

/// El valor de una condición que no depende de nada: `verdadero`, `falso` o una comparación
/// entre dos valores escritos tal cual (`1 == 1`).
fn constante(condicion: &Expresion) -> Option<bool> {
    use OperadorBinario::*;
    match condicion {
        Expresion::Logico(b) => Some(*b),
        Expresion::Binaria { izquierda, operador, derecha, .. } => {
            let (a, b) = (literal(izquierda)?, literal(derecha)?);
            match (a, b) {
                (Literal::Numero(x), Literal::Numero(y)) => match operador {
                    Igual => Some(x == y),
                    Distinto => Some(x != y),
                    Menor => Some(x < y),
                    MenorIgual => Some(x <= y),
                    Mayor => Some(x > y),
                    MayorIgual => Some(x >= y),
                    _ => None,
                },
                (Literal::Texto(x), Literal::Texto(y)) => match operador {
                    Igual => Some(x == y),
                    Distinto => Some(x != y),
                    _ => None,
                },
                (Literal::Logico(x), Literal::Logico(y)) => match operador {
                    Igual => Some(x == y),
                    Distinto => Some(x != y),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Todos los nombres que aparecen como llamada o como variable (una función usada como valor
/// es una variable).
fn nombres_en(lista: &[Sentencia], nombres: &mut HashSet<String>) {
    for sentencia in lista {
        match sentencia {
            Sentencia::Declarar { valor, .. } | Sentencia::Asignar { valor, .. } => nombres_en_expresion(valor, nombres),
            Sentencia::AsignarIndice { indices, valor, .. } => {
                indices.iter().for_each(|i| nombres_en_expresion(i, nombres));
                nombres_en_expresion(valor, nombres);
            }
            Sentencia::Si { condicion, entonces, sino, .. } => {
                nombres_en_expresion(condicion, nombres);
                nombres_en(entonces, nombres);
                nombres_en(sino, nombres);
            }
            Sentencia::Intentar { cuerpo, captura, .. } => {
                nombres_en(cuerpo, nombres);
                nombres_en(captura, nombres);
            }
            Sentencia::Mientras { condicion, cuerpo, .. } => {
                nombres_en_expresion(condicion, nombres);
                nombres_en(cuerpo, nombres);
            }
            Sentencia::Para { coleccion, cuerpo, .. } => {
                nombres_en_expresion(coleccion, nombres);
                nombres_en(cuerpo, nombres);
            }
            Sentencia::Funcion { cuerpo, .. } | Sentencia::Ruta { cuerpo, .. } | Sentencia::Prueba { cuerpo, .. } => nombres_en(cuerpo, nombres),
            Sentencia::Devolver { valor: Some(valor), .. } => nombres_en_expresion(valor, nombres),
            Sentencia::Expresion(expresion) => nombres_en_expresion(expresion, nombres),
            _ => {}
        }
    }
}

fn nombres_en_expresion(expresion: &Expresion, nombres: &mut HashSet<String>) {
    match expresion {
        Expresion::Variable { nombre, .. } => {
            nombres.insert(nombre.clone());
        }
        Expresion::Llamada { nombre, argumentos, .. } => {
            nombres.insert(nombre.clone());
            argumentos.iter().for_each(|a| nombres_en_expresion(a, nombres));
        }
        Expresion::TextoConValores { partes, .. } => {
            for parte in partes {
                if let ParteTexto::Valor(valor) = parte {
                    nombres_en_expresion(valor, nombres);
                }
            }
        }
        Expresion::Lista(elementos) => elementos.iter().for_each(|e| nombres_en_expresion(e, nombres)),
        Expresion::Mapa(pares) => {
            for (clave, valor) in pares {
                nombres_en_expresion(clave, nombres);
                nombres_en_expresion(valor, nombres);
            }
        }
        Expresion::Unaria { valor, .. } => nombres_en_expresion(valor, nombres),
        Expresion::Binaria { izquierda, derecha, .. } => {
            nombres_en_expresion(izquierda, nombres);
            nombres_en_expresion(derecha, nombres);
        }
        Expresion::Indice { coleccion, indice, .. } => {
            nombres_en_expresion(coleccion, nombres);
            nombres_en_expresion(indice, nombres);
        }
        Expresion::Numero(_) | Expresion::EnteroGrande(_) | Expresion::Texto(_) | Expresion::Logico(_) | Expresion::Nada => {}
    }
}
