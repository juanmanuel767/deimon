//! Funciones numéricas: si una función del programa solo hace cuentas con números (sus
//! parámetros, sus variables y lo que devuelve son siempre números), se genera además una
//! versión rápida que trabaja con `f64` directos, sin el `Valor` que puede ser cualquier
//! cosa. La función normal la usa cuando recibe números; con otros valores sigue igual
//! que siempre, así que el comportamiento no cambia.

use std::collections::{HashMap, HashSet};

use crate::ast::{Expresion, OperadorBinario, OperadorUnario, Sentencia};

/// Nombres (y número de parámetros) de las funciones que tienen versión numérica.
pub fn funciones_numericas(sentencias: &[Sentencia]) -> HashMap<String, usize> {
    let mut funciones: HashMap<String, (&Vec<String>, &Vec<Sentencia>)> = HashMap::new();
    let mut repetidas = HashSet::new();
    for sentencia in sentencias {
        if let Sentencia::Funcion { nombre, parametros, cuerpo, .. } = sentencia {
            if funciones.insert(nombre.clone(), (parametros, cuerpo)).is_some() {
                repetidas.insert(nombre.clone());
            }
        }
    }
    for nombre in repetidas {
        funciones.remove(&nombre);
    }
    let mut candidatas: HashMap<String, usize> = funciones.iter().map(|(n, (p, _))| (n.clone(), p.len())).collect();
    // Punto fijo: se quitan las que no cumplen hasta que no cambie nada.
    loop {
        let quitar: Vec<String> = candidatas
            .keys()
            .filter(|nombre| {
                let (parametros, cuerpo) = funciones[*nombre];
                let mut ambito = Ambito::nuevo(parametros);
                let mut repetidos = HashSet::new();
                parametros.iter().any(|p| !repetidos.insert(p.clone()))
                    || !sentencias_numericas(cuerpo, &mut ambito, &candidatas)
                    || !siempre_devuelve(cuerpo)
            })
            .cloned()
            .collect();
        if quitar.is_empty() {
            return candidatas;
        }
        for nombre in quitar {
            candidatas.remove(&nombre);
        }
    }
}

/// Variables visibles, por bloques (como `let` en Rust y `sea` en Deimon).
struct Ambito {
    bloques: Vec<HashSet<String>>,
}

impl Ambito {
    fn nuevo(parametros: &[String]) -> Self {
        Ambito { bloques: vec![parametros.iter().cloned().collect()] }
    }
    fn existe(&self, nombre: &str) -> bool {
        self.bloques.iter().any(|b| b.contains(nombre))
    }
}

fn sentencias_numericas(lista: &[Sentencia], ambito: &mut Ambito, numericas: &HashMap<String, usize>) -> bool {
    lista.iter().all(|s| sentencia_numerica(s, ambito, numericas))
}

fn sentencia_numerica(sentencia: &Sentencia, ambito: &mut Ambito, numericas: &HashMap<String, usize>) -> bool {
    match sentencia {
        Sentencia::Declarar { nombre, valor, .. } => {
            if !es_numero(valor, ambito, numericas) {
                return false;
            }
            ambito.bloques.last_mut().expect("un bloque").insert(nombre.clone());
            true
        }
        Sentencia::Asignar { nombre, valor, .. } => ambito.existe(nombre) && es_numero(valor, ambito, numericas),
        Sentencia::Si { condicion, entonces, sino, .. } => {
            if !es_logico(condicion, ambito, numericas) {
                return false;
            }
            for bloque in [entonces, sino] {
                ambito.bloques.push(HashSet::new());
                let bien = sentencias_numericas(bloque, ambito, numericas);
                ambito.bloques.pop();
                if !bien {
                    return false;
                }
            }
            true
        }
        Sentencia::Mientras { condicion, cuerpo, .. } => {
            if !es_logico(condicion, ambito, numericas) {
                return false;
            }
            ambito.bloques.push(HashSet::new());
            let bien = sentencias_numericas(cuerpo, ambito, numericas);
            ambito.bloques.pop();
            bien
        }
        Sentencia::Devolver { valor: Some(valor), .. } => es_numero(valor, ambito, numericas),
        Sentencia::Romper { .. } | Sentencia::Continuar { .. } => true,
        _ => false,
    }
}

fn siempre_devuelve(lista: &[Sentencia]) -> bool {
    match lista.last() {
        Some(Sentencia::Devolver { valor: Some(_), .. }) => true,
        Some(Sentencia::Si { entonces, sino, .. }) => siempre_devuelve(entonces) && siempre_devuelve(sino),
        _ => false,
    }
}

fn es_numero(e: &Expresion, ambito: &Ambito, numericas: &HashMap<String, usize>) -> bool {
    match e {
        Expresion::Numero(_) => true,
        Expresion::Variable { nombre, .. } => ambito.existe(nombre),
        Expresion::Unaria { operador: OperadorUnario::Negativo, valor, .. } => es_numero(valor, ambito, numericas),
        Expresion::Binaria { izquierda, operador, derecha, .. } => {
            matches!(
                operador,
                OperadorBinario::Suma | OperadorBinario::Resta | OperadorBinario::Multiplicacion | OperadorBinario::Division | OperadorBinario::Resto
            ) && es_numero(izquierda, ambito, numericas)
                && es_numero(derecha, ambito, numericas)
        }
        Expresion::Llamada { nombre, argumentos, .. } => {
            numericas.get(nombre) == Some(&argumentos.len()) && argumentos.iter().all(|a| es_numero(a, ambito, numericas))
        }
        _ => false,
    }
}

fn es_logico(e: &Expresion, ambito: &Ambito, numericas: &HashMap<String, usize>) -> bool {
    match e {
        Expresion::Logico(_) => true,
        Expresion::Unaria { operador: OperadorUnario::No, valor, .. } => es_logico(valor, ambito, numericas),
        Expresion::Binaria { izquierda, operador, derecha, .. } => match operador {
            OperadorBinario::Y | OperadorBinario::O => es_logico(izquierda, ambito, numericas) && es_logico(derecha, ambito, numericas),
            OperadorBinario::Igual
            | OperadorBinario::Distinto
            | OperadorBinario::Menor
            | OperadorBinario::MenorIgual
            | OperadorBinario::Mayor
            | OperadorBinario::MayorIgual => es_numero(izquierda, ambito, numericas) && es_numero(derecha, ambito, numericas),
            _ => false,
        },
        _ => false,
    }
}

/// Nombre de la versión numérica de una función.
pub fn nombre_numerico(nombre: &str) -> String {
    format!("num_{}", crate::generador::nombre_funcion(nombre))
}

/// Código Rust de la versión numérica de una función (que ya se sabe que cumple).
pub fn generar(nombre: &str, parametros: &[String], cuerpo: &[Sentencia]) -> String {
    let parametros_rust: Vec<String> = parametros
        .iter()
        .map(|p| format!("{}: f64", crate::generador::nombre_variable(p)))
        .collect();
    let mut salida = format!(
        "\n#[allow(unreachable_code, unused_mut, unused_variables, non_snake_case)]\nfn {}({}) -> f64 {{\n",
        nombre_numerico(nombre),
        parametros_rust.join(", ")
    );
    sentencias_rust(cuerpo, 1, &mut salida);
    salida.push_str("    unreachable!()\n}\n");
    salida
}

fn sentencias_rust(lista: &[Sentencia], nivel: usize, salida: &mut String) {
    let sangria = "    ".repeat(nivel);
    for sentencia in lista {
        match sentencia {
            Sentencia::Declarar { nombre, valor, .. } => {
                salida.push_str(&format!("{}let mut {}: f64 = {};\n", sangria, crate::generador::nombre_variable(nombre), numero_rust(valor)));
            }
            Sentencia::Asignar { nombre, valor, .. } => {
                salida.push_str(&format!("{}{} = {};\n", sangria, crate::generador::nombre_variable(nombre), numero_rust(valor)));
            }
            Sentencia::Si { condicion, entonces, sino, .. } => {
                salida.push_str(&format!("{}if {} {{\n", sangria, logico_rust(condicion)));
                sentencias_rust(entonces, nivel + 1, salida);
                salida.push_str(&format!("{}}} else {{\n", sangria));
                sentencias_rust(sino, nivel + 1, salida);
                salida.push_str(&format!("{}}}\n", sangria));
            }
            Sentencia::Mientras { condicion, cuerpo, .. } => {
                salida.push_str(&format!("{}while {} {{\n", sangria, logico_rust(condicion)));
                sentencias_rust(cuerpo, nivel + 1, salida);
                salida.push_str(&format!("{}}}\n", sangria));
            }
            Sentencia::Devolver { valor: Some(valor), .. } => {
                salida.push_str(&format!("{}return {};\n", sangria, numero_rust(valor)));
            }
            Sentencia::Romper { .. } => salida.push_str(&format!("{}break;\n", sangria)),
            Sentencia::Continuar { .. } => salida.push_str(&format!("{}continue;\n", sangria)),
            _ => unreachable!("sentencia que no es numérica"),
        }
    }
}

fn numero_rust(e: &Expresion) -> String {
    match e {
        Expresion::Numero(n) => format!("({:?}_f64)", n),
        Expresion::Variable { nombre, .. } => crate::generador::nombre_variable(nombre),
        Expresion::Unaria { valor, .. } => format!("(-{})", numero_rust(valor)),
        Expresion::Binaria { izquierda, operador, derecha, linea } => {
            let (a, b) = (numero_rust(izquierda), numero_rust(derecha));
            match operador {
                OperadorBinario::Suma => format!("({} + {})", a, b),
                OperadorBinario::Resta => format!("({} - {})", a, b),
                OperadorBinario::Multiplicacion => format!("({} * {})", a, b),
                OperadorBinario::Division => format!("division_numerica({}, {}, {})", a, b, linea),
                _ => format!("resto_numerico({}, {}, {})", a, b, linea),
            }
        }
        Expresion::Llamada { nombre, argumentos, .. } => {
            let partes: Vec<String> = argumentos.iter().map(numero_rust).collect();
            format!("{}({})", nombre_numerico(nombre), partes.join(", "))
        }
        _ => unreachable!("expresión que no es numérica"),
    }
}

fn logico_rust(e: &Expresion) -> String {
    match e {
        Expresion::Logico(b) => b.to_string(),
        Expresion::Unaria { valor, .. } => format!("(!{})", logico_rust(valor)),
        Expresion::Binaria { izquierda, operador, derecha, .. } => match operador {
            OperadorBinario::Y => format!("({} && {})", logico_rust(izquierda), logico_rust(derecha)),
            OperadorBinario::O => format!("({} || {})", logico_rust(izquierda), logico_rust(derecha)),
            otro => {
                let signo = match otro {
                    OperadorBinario::Igual => "==",
                    OperadorBinario::Distinto => "!=",
                    OperadorBinario::Menor => "<",
                    OperadorBinario::MenorIgual => "<=",
                    OperadorBinario::Mayor => ">",
                    _ => ">=",
                };
                format!("({} {} {})", numero_rust(izquierda), signo, numero_rust(derecha))
            }
        },
        _ => unreachable!("condición que no es lógica"),
    }
}

/// Si un bucle `mientras` solo hace cuentas, devuelve las variables de fuera que usa (en
/// orden): el bucle se puede recorrer con `f64` directos cuando todas valen números.
pub fn bucle_numerico(
    condicion: &Expresion,
    cuerpo: &[Sentencia],
    numericas: &HashMap<String, usize>,
) -> Option<Vec<String>> {
    if contiene_devolver(cuerpo) {
        return None;
    }
    let mut libres = Vec::new();
    let mut locales = vec![HashSet::new()];
    variables_de_expresion(condicion, &locales, &mut libres);
    variables_de_sentencias(cuerpo, &mut locales, &mut libres);
    let mut ambito = Ambito::nuevo(&libres);
    let bien = es_logico(condicion, &ambito, numericas) && {
        ambito.bloques.push(HashSet::new());
        sentencias_numericas(cuerpo, &mut ambito, numericas)
    };
    bien.then_some(libres)
}

/// Código Rust del bucle rápido; las variables de fuera ya existen como `f64` mutables.
pub fn generar_bucle(condicion: &Expresion, cuerpo: &[Sentencia], nivel: usize) -> String {
    let sangria = "    ".repeat(nivel);
    let mut salida = format!("{}while {} {{\n", sangria, logico_rust(condicion));
    sentencias_rust(cuerpo, nivel + 1, &mut salida);
    salida.push_str(&format!("{}}}\n", sangria));
    salida
}

fn contiene_devolver(lista: &[Sentencia]) -> bool {
    lista.iter().any(|s| match s {
        Sentencia::Devolver { .. } => true,
        Sentencia::Si { entonces, sino, .. } => contiene_devolver(entonces) || contiene_devolver(sino),
        Sentencia::Mientras { cuerpo, .. } | Sentencia::Para { cuerpo, .. } => contiene_devolver(cuerpo),
        _ => false,
    })
}

fn anotar_libre(nombre: &str, locales: &[HashSet<String>], libres: &mut Vec<String>) {
    if !locales.iter().any(|b| b.contains(nombre)) && !libres.iter().any(|l| l == nombre) {
        libres.push(nombre.to_string());
    }
}

fn variables_de_expresion(e: &Expresion, locales: &[HashSet<String>], libres: &mut Vec<String>) {
    match e {
        Expresion::Variable { nombre, .. } => anotar_libre(nombre, locales, libres),
        Expresion::Unaria { valor, .. } => variables_de_expresion(valor, locales, libres),
        Expresion::Binaria { izquierda, derecha, .. } => {
            variables_de_expresion(izquierda, locales, libres);
            variables_de_expresion(derecha, locales, libres);
        }
        Expresion::Llamada { argumentos, .. } | Expresion::Lista(argumentos) => {
            for a in argumentos {
                variables_de_expresion(a, locales, libres);
            }
        }
        Expresion::Indice { coleccion, indice, .. } => {
            variables_de_expresion(coleccion, locales, libres);
            variables_de_expresion(indice, locales, libres);
        }
        _ => {}
    }
}

fn variables_de_sentencias(lista: &[Sentencia], locales: &mut Vec<HashSet<String>>, libres: &mut Vec<String>) {
    for sentencia in lista {
        match sentencia {
            Sentencia::Declarar { nombre, valor, .. } => {
                variables_de_expresion(valor, locales, libres);
                locales.last_mut().expect("un bloque").insert(nombre.clone());
            }
            Sentencia::Asignar { nombre, valor, .. } => {
                anotar_libre(nombre, locales, libres);
                variables_de_expresion(valor, locales, libres);
            }
            Sentencia::Si { condicion, entonces, sino, .. } => {
                variables_de_expresion(condicion, locales, libres);
                for bloque in [entonces, sino] {
                    locales.push(HashSet::new());
                    variables_de_sentencias(bloque, locales, libres);
                    locales.pop();
                }
            }
            Sentencia::Mientras { condicion, cuerpo, .. } => {
                variables_de_expresion(condicion, locales, libres);
                locales.push(HashSet::new());
                variables_de_sentencias(cuerpo, locales, libres);
                locales.pop();
            }
            Sentencia::Expresion(e) => variables_de_expresion(e, locales, libres),
            _ => {}
        }
    }
}
