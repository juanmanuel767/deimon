//! Revisión de tipos antes de ejecutar (código E025).
//!
//! Busca operaciones que fallarían SEGURO al llegar a ellas: restar un texto, comparar con `<`
//! algo que no es un número, poner un número como condición de `si`, recorrer un número con
//! `para`... Solo avisa cuando sabe con certeza el tipo de los dos lados (un número o un texto
//! escritos tal cual, una variable que solo ha recibido ese tipo, el resultado de `a_texto`...).
//! Si duda, calla: nunca da una falsa alarma. Los mensajes son los mismos que daría el programa
//! al fallar, para que se entiendan igual.

use std::collections::HashMap;

use crate::ast::{Expresion, OperadorBinario, OperadorUnario, Programa, Sentencia};
use crate::error::ErrorTraduccion;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tipo {
    Numero,
    Texto,
    Logico,
    Nada,
    Lista,
    Mapa,
    /// Lo que devuelve `Persona({...})` de un `tipo Persona { ... }` (posición en la lista de
    /// tipos propios). Es una lista con claves que además sabe sus campos.
    Registro(usize),
    /// Puede ser cualquier cosa: no se avisa de nada.
    Desconocido,
}

impl Tipo {
    /// El mismo nombre que usa el núcleo en sus mensajes (`Valor::tipo`).
    fn nombre(self) -> &'static str {
        match self {
            Tipo::Numero => "número",
            Tipo::Texto => "texto",
            Tipo::Logico => "lógico",
            Tipo::Nada => "nada",
            Tipo::Lista => "lista",
            Tipo::Mapa | Tipo::Registro(_) => "lista con claves",
            Tipo::Desconocido => "valor",
        }
    }

    fn conocido(self) -> bool {
        self != Tipo::Desconocido
    }
}

/// Tipo seguro del resultado de algunas integradas (comprobado en el núcleo: siempre
/// devuelven eso o fallan).
fn tipo_de_integrada(nombre: &str) -> Tipo {
    match nombre {
        "a_texto" | "proteger_html" => Tipo::Texto,
        "a_numero" | "medir" | "contar" | "sumar_lista" => Tipo::Numero,
        "ordenar" | "ordenar_por" | "filtrar" | "transformar" | "agregar" | "rango" | "listar_claves" => Tipo::Lista,
        "tiene_clave" | "incluye" => Tipo::Logico,
        _ => Tipo::Desconocido,
    }
}

type Ambito = HashMap<String, Tipo>;

/// Un `tipo Nombre { ... }`: su nombre y sus campos (nombre, tipo pedido o "" si vale todo).
struct TipoPropio {
    nombre: String,
    campos: Vec<(String, String)>,
}

/// Reconoce la función que crea el analizador para `tipo Nombre { ... }` (ver
/// `Analizador::tipo_propio`): un solo `devolver crear_con_tipo("Nombre", [[campo, tipo]...], datos)`.
fn tipo_propio_de(sentencia: &Sentencia) -> Option<TipoPropio> {
    let Sentencia::Funcion { nombre, parametros, cuerpo, .. } = sentencia else { return None };
    let [Sentencia::Devolver { valor: Some(Expresion::Llamada { nombre: llamada, argumentos, .. }), .. }] = cuerpo.as_slice() else {
        return None;
    };
    let [Expresion::Texto(nombre_tipo), Expresion::Lista(campos), Expresion::Variable { .. }] = argumentos.as_slice() else {
        return None;
    };
    if llamada != "crear_con_tipo" || nombre_tipo != nombre || parametros.len() != 1 {
        return None;
    }
    let campos = campos
        .iter()
        .map(|campo| match campo {
            Expresion::Lista(par) => match par.as_slice() {
                [Expresion::Texto(campo), Expresion::Texto(tipo)] => Some((campo.clone(), tipo.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TipoPropio { nombre: nombre.clone(), campos })
}

fn tipo_de_campo(tipo: &str) -> Tipo {
    match tipo {
        "numero" => Tipo::Numero,
        "texto" => Tipo::Texto,
        "logico" => Tipo::Logico,
        "lista" => Tipo::Lista,
        "lista_con_claves" => Tipo::Mapa,
        _ => Tipo::Desconocido,
    }
}

struct Revisor {
    tipos_propios: Vec<TipoPropio>,
    errores: Vec<ErrorTraduccion>,
    /// Archivo de la definición que se revisa (si viene de un `usar`).
    archivo: Option<String>,
}

/// Revisa los tipos del programa ya resuelto (con sus `usar`).
pub fn revisar(programa: &Programa) -> Vec<ErrorTraduccion> {
    let tipos_propios = programa.sentencias.iter().filter_map(tipo_propio_de).collect();
    let mut revisor = Revisor { tipos_propios, errores: Vec::new(), archivo: None };
    let mut principal = Ambito::new();
    for sentencia in &programa.sentencias {
        match sentencia {
            Sentencia::Funcion { parametros, cuerpo, archivo, .. } => {
                revisor.archivo = archivo.clone();
                let mut ambito: Ambito = parametros.iter().map(|p| (p.clone(), Tipo::Desconocido)).collect();
                revisor.sentencias(cuerpo, &mut ambito);
            }
            Sentencia::Ruta { camino, cuerpo, archivo, .. } => {
                revisor.archivo = archivo.clone();
                // Los parámetros del camino (`:nombre`) llegan siempre como texto.
                let mut ambito: Ambito = camino
                    .split('/')
                    .filter_map(|parte| parte.strip_prefix(':'))
                    .map(|nombre| (nombre.to_string(), Tipo::Texto))
                    .collect();
                revisor.sentencias(cuerpo, &mut ambito);
            }
            Sentencia::Prueba { cuerpo, archivo, .. } => {
                revisor.archivo = archivo.clone();
                revisor.sentencias(cuerpo, &mut Ambito::new());
            }
            otra => {
                revisor.archivo = None;
                revisor.sentencias(std::slice::from_ref(otra), &mut principal);
            }
        }
    }
    revisor.errores
}

impl Revisor {
    fn avisar(&mut self, linea: usize, mensaje: String, sugerencia: &str) {
        let mut error = ErrorTraduccion::nuevo(linea, format!("{} (se ve sin ejecutar el programa)", mensaje))
            .con_codigo("E025")
            .con_sugerencia(sugerencia);
        if let Some(archivo) = &self.archivo {
            error = error.con_archivo(archivo);
        }
        self.errores.push(error);
    }

    /// Campos de un tipo propio que faltan, sobran o no encajan (E027), con el mismo mensaje
    /// que daría el programa al ejecutarse.
    fn avisar_campo(&mut self, linea: usize, columna: usize, mensaje: String, sugerencia: String) {
        let mut error = ErrorTraduccion::nuevo(linea, format!("{} (se ve sin ejecutar el programa)", mensaje))
            .con_codigo("E027")
            .en_columna(columna)
            .con_sugerencia(sugerencia);
        if let Some(archivo) = &self.archivo {
            error = error.con_archivo(archivo);
        }
        self.errores.push(error);
    }

    fn campo_parecido(&self, tipo: usize, clave: &str) -> Option<String> {
        self.tipos_propios[tipo]
            .campos
            .iter()
            .map(|(campo, _)| (crate::generador::distancia_edicion(clave, campo), campo.clone()))
            .filter(|(distancia, _)| *distancia <= 2)
            .min()
            .map(|(_, campo)| campo)
    }

    /// `Persona({"nombre": ..., "edad": ...})` escrito tal cual: se revisan sus campos.
    fn revisar_creacion(&mut self, tipo: usize, pares: &[(Expresion, Expresion)], ambito: &Ambito, linea: usize, columna: usize) {
        let nombre = self.tipos_propios[tipo].nombre.clone();
        let campos = self.tipos_propios[tipo].campos.clone();
        let lista = campos.iter().map(|(campo, _)| campo.as_str()).collect::<Vec<_>>().join(", ");
        let mut puestas = Vec::new();
        for (clave, valor) in pares {
            let Expresion::Texto(clave) = clave else { return };
            puestas.push(clave.clone());
            match campos.iter().find(|(campo, _)| campo == clave) {
                None => {
                    let pista = match self.campo_parecido(tipo, clave) {
                        Some(campo) => format!("¿Quisiste decir '{}'?", campo),
                        None => format!("Los campos de {} son: {}", nombre, lista),
                    };
                    self.avisar_campo(linea, columna, format!("{} no tiene el campo '{}'", nombre, clave), pista);
                }
                Some((_, pedido)) => {
                    let esperado = tipo_de_campo(pedido);
                    let mut tiene = self.expresion(valor, ambito);
                    if let Tipo::Registro(_) = tiene {
                        tiene = Tipo::Mapa;
                    }
                    if esperado.conocido() && tiene.conocido() && tiene != esperado {
                        self.avisar_campo(
                            linea,
                            columna,
                            format!("el campo '{}' de {} tiene que ser {}, no {}", clave, nombre, esperado.nombre(), tiene.nombre()),
                            format!("Pon en '{}' un valor de tipo {}", clave, esperado.nombre()),
                        );
                    }
                }
            }
        }
        for (campo, _) in &campos {
            if !puestas.contains(campo) {
                self.avisar_campo(
                    linea,
                    columna,
                    format!("falta el campo '{}' de {} (sus campos son: {})", campo, nombre, lista),
                    format!("Añade \"{}\": ... dentro de las llaves", campo),
                );
            }
        }
    }

    fn sentencias(&mut self, lista: &[Sentencia], ambito: &mut Ambito) {
        for sentencia in lista {
            self.sentencia(sentencia, ambito);
        }
    }

    fn sentencia(&mut self, sentencia: &Sentencia, ambito: &mut Ambito) {
        match sentencia {
            Sentencia::Declarar { nombre, valor, .. } | Sentencia::Asignar { nombre, valor, .. } => {
                let tipo = self.expresion(valor, ambito);
                ambito.insert(nombre.clone(), tipo);
            }
            Sentencia::AsignarIndice { nombre, indices, valor, .. } => {
                for indice in indices {
                    self.expresion(indice, ambito);
                }
                self.expresion(valor, ambito);
                // Una lista puede pasar a ser una lista con claves: ya no se sabe.
                ambito.insert(nombre.clone(), Tipo::Desconocido);
            }
            Sentencia::Si { condicion, entonces, sino, linea } => {
                let tipo = self.expresion(condicion, ambito);
                self.condicion(tipo, *linea);
                let mut por_si = ambito.clone();
                let mut por_sino = ambito.clone();
                self.sentencias(entonces, &mut por_si);
                self.sentencias(sino, &mut por_sino);
                juntar(ambito, &por_si, &por_sino);
            }
            Sentencia::Mientras { condicion, cuerpo, linea } => {
                olvidar_asignadas(cuerpo, ambito);
                let tipo = self.expresion(condicion, ambito);
                self.condicion(tipo, *linea);
                let mut dentro = ambito.clone();
                self.sentencias(cuerpo, &mut dentro);
            }
            Sentencia::Para { nombre, coleccion, cuerpo, linea } => {
                olvidar_asignadas(cuerpo, ambito);
                let tipo = self.expresion(coleccion, ambito);
                if matches!(tipo, Tipo::Numero | Tipo::Texto | Tipo::Logico | Tipo::Nada) {
                    self.avisar(
                        *linea,
                        format!("para necesita una lista, no {}", tipo.nombre()),
                        "Recorre una lista, por ejemplo: para i en rango(1, 10) { ... }",
                    );
                }
                let mut dentro = ambito.clone();
                let elemento = match coleccion {
                    Expresion::Llamada { nombre: funcion, .. } if funcion == "rango" => Tipo::Numero,
                    _ => Tipo::Desconocido,
                };
                dentro.insert(nombre.clone(), elemento);
                self.sentencias(cuerpo, &mut dentro);
            }
            Sentencia::Intentar { cuerpo, nombre, captura, .. } => {
                olvidar_asignadas(cuerpo, ambito);
                olvidar_asignadas(captura, ambito);
                let mut dentro = ambito.clone();
                self.sentencias(cuerpo, &mut dentro);
                let mut si_falla = ambito.clone();
                si_falla.insert(nombre.clone(), Tipo::Mapa);
                self.sentencias(captura, &mut si_falla);
            }
            Sentencia::Devolver { valor, .. } => {
                if let Some(valor) = valor {
                    self.expresion(valor, ambito);
                }
            }
            Sentencia::Expresion(expresion) => {
                self.expresion(expresion, ambito);
            }
            _ => {}
        }
    }

    fn condicion(&mut self, tipo: Tipo, linea: usize) {
        if tipo.conocido() && tipo != Tipo::Logico {
            self.avisar(
                linea,
                format!("la condición debe ser verdadero o falso, no {}", tipo.nombre()),
                "Usa una comparación, por ejemplo: si total > 0 { ... } o si nombre != \"\" { ... }",
            );
        }
    }

    fn expresion(&mut self, expresion: &Expresion, ambito: &Ambito) -> Tipo {
        match expresion {
            Expresion::Numero(_) | Expresion::EnteroGrande(_) => Tipo::Numero,
            Expresion::Texto(_) => Tipo::Texto,
            Expresion::Logico(_) => Tipo::Logico,
            Expresion::Nada => Tipo::Nada,
            Expresion::Lista(elementos) => {
                for elemento in elementos {
                    self.expresion(elemento, ambito);
                }
                Tipo::Lista
            }
            Expresion::Mapa(pares) => {
                for (clave, valor) in pares {
                    self.expresion(clave, ambito);
                    self.expresion(valor, ambito);
                }
                Tipo::Mapa
            }
            Expresion::Variable { nombre, .. } => ambito.get(nombre).copied().unwrap_or(Tipo::Desconocido),
            Expresion::Llamada { nombre, argumentos, linea, columna } => {
                if let Some(tipo) = self.tipos_propios.iter().position(|t| t.nombre == *nombre) {
                    if let [Expresion::Mapa(pares)] = argumentos.as_slice() {
                        self.revisar_creacion(tipo, pares, ambito, *linea, *columna);
                    } else {
                        for argumento in argumentos {
                            self.expresion(argumento, ambito);
                        }
                    }
                    return Tipo::Registro(tipo);
                }
                for argumento in argumentos {
                    self.expresion(argumento, ambito);
                }
                tipo_de_integrada(nombre)
            }
            Expresion::Unaria { operador, valor, linea } => {
                let tipo = self.expresion(valor, ambito);
                match operador {
                    OperadorUnario::Negativo => {
                        if tipo.conocido() && tipo != Tipo::Numero {
                            self.avisar(*linea, format!("no se puede negar {}", tipo.nombre()), "Solo se pueden poner en negativo los números");
                        }
                        Tipo::Numero
                    }
                    OperadorUnario::No => {
                        self.condicion(tipo, *linea);
                        Tipo::Logico
                    }
                }
            }
            Expresion::Indice { coleccion, indice, linea } => {
                let tipo = self.expresion(coleccion, ambito);
                self.expresion(indice, ambito);
                if let (Tipo::Registro(registro), Expresion::Texto(clave)) = (tipo, indice.as_ref()) {
                    let campo = self.tipos_propios[registro].campos.iter().find(|(campo, _)| campo == clave).cloned();
                    return match campo {
                        Some((_, pedido)) => tipo_de_campo(&pedido),
                        None => {
                            let nombre = self.tipos_propios[registro].nombre.clone();
                            let pista = match self.campo_parecido(registro, clave) {
                                Some(campo) => format!("¿Quisiste decir '{}'?", campo),
                                None => format!(
                                    "Los campos de {} son: {}",
                                    nombre,
                                    self.tipos_propios[registro].campos.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>().join(", ")
                                ),
                            };
                            self.avisar_campo(*linea, 0, format!("{} no tiene el campo '{}'", nombre, clave), pista);
                            Tipo::Desconocido
                        }
                    };
                }
                if matches!(tipo, Tipo::Numero | Tipo::Logico | Tipo::Nada) {
                    self.avisar(
                        *linea,
                        format!("solo se puede acceder con corchetes a una lista o a un texto, no a {}", tipo.nombre()),
                        "Los corchetes valen para listas, listas con claves y textos: lista[0], datos[\"nombre\"]",
                    );
                }
                match tipo {
                    Tipo::Texto => Tipo::Texto,
                    _ => Tipo::Desconocido,
                }
            }
            Expresion::Binaria { izquierda, operador, derecha, linea } => {
                let a = self.expresion(izquierda, ambito);
                let b = self.expresion(derecha, ambito);
                self.binaria(operador, a, b, *linea)
            }
        }
    }

    fn binaria(&mut self, operador: &OperadorBinario, a: Tipo, b: Tipo, linea: usize) -> Tipo {
        use OperadorBinario::*;
        // Para las operaciones, un tipo propio es una lista con claves más.
        let como_mapa = |tipo: Tipo| if let Tipo::Registro(_) = tipo { Tipo::Mapa } else { tipo };
        let (a, b) = (como_mapa(a), como_mapa(b));
        let conocidos = a.conocido() && b.conocido();
        match operador {
            Suma => {
                if !conocidos {
                    return if a == Tipo::Texto || b == Tipo::Texto { Tipo::Texto } else { Tipo::Desconocido };
                }
                match (a, b) {
                    (Tipo::Numero, Tipo::Numero) => Tipo::Numero,
                    (Tipo::Lista, Tipo::Lista) => Tipo::Lista,
                    (Tipo::Texto, _) | (_, Tipo::Texto) => Tipo::Texto,
                    _ => {
                        self.avisar(
                            linea,
                            format!("no se puede sumar {} y {}", a.nombre(), b.nombre()),
                            "Se suman números, se juntan textos (con un texto se puede sumar cualquier cosa) y se juntan listas",
                        );
                        Tipo::Desconocido
                    }
                }
            }
            Resta | Multiplicacion | Division | Resto => {
                if conocidos && (a, b) != (Tipo::Numero, Tipo::Numero) {
                    let verbo = match operador {
                        Resta => "restar",
                        Multiplicacion => "multiplicar",
                        Division => "dividir",
                        _ => "calcular el resto de",
                    };
                    self.avisar(
                        linea,
                        format!("no se puede {} {} y {}", verbo, a.nombre(), b.nombre()),
                        "Solo se puede con números; si es un número escrito como texto, conviértelo con a_numero(...)",
                    );
                    return Tipo::Desconocido;
                }
                if conocidos { Tipo::Numero } else { Tipo::Desconocido }
            }
            Menor | MenorIgual | Mayor | MayorIgual => {
                if conocidos && (a, b) != (Tipo::Numero, Tipo::Numero) {
                    let como = match operador {
                        Menor => "menor que",
                        MenorIgual => "menor o igual que",
                        Mayor => "mayor que",
                        _ => "mayor o igual que",
                    };
                    self.avisar(
                        linea,
                        format!("no se pueden comparar {} y {} con {}", a.nombre(), b.nombre(), como),
                        "<, <=, > y >= comparan números; para ver si dos valores son iguales usa == o !=",
                    );
                }
                Tipo::Logico
            }
            Igual | Distinto => Tipo::Logico,
            Y | O => {
                self.condicion(a, linea);
                self.condicion(b, linea);
                Tipo::Logico
            }
        }
    }
}

/// Después de un `si`: una variable conserva su tipo solo si las dos ramas lo dejan igual.
fn juntar(ambito: &mut Ambito, por_si: &Ambito, por_sino: &Ambito) {
    for (nombre, tipo) in ambito.iter_mut() {
        let (uno, otro) = (por_si.get(nombre), por_sino.get(nombre));
        if uno != Some(tipo) || otro != Some(tipo) {
            *tipo = if uno == otro { uno.copied().unwrap_or(Tipo::Desconocido) } else { Tipo::Desconocido };
        }
    }
}

/// Antes de un bucle o un `intentar`: lo que se cambia dentro puede tener cualquier tipo en
/// cualquier vuelta, así que se olvida.
fn olvidar_asignadas(lista: &[Sentencia], ambito: &mut Ambito) {
    for sentencia in lista {
        match sentencia {
            Sentencia::Asignar { nombre, .. } | Sentencia::AsignarIndice { nombre, .. } => {
                if let Some(tipo) = ambito.get_mut(nombre) {
                    *tipo = Tipo::Desconocido;
                }
            }
            Sentencia::Si { entonces, sino, .. } => {
                olvidar_asignadas(entonces, ambito);
                olvidar_asignadas(sino, ambito);
            }
            Sentencia::Mientras { cuerpo, .. } | Sentencia::Para { cuerpo, .. } => olvidar_asignadas(cuerpo, ambito),
            Sentencia::Intentar { cuerpo, captura, .. } => {
                olvidar_asignadas(cuerpo, ambito);
                olvidar_asignadas(captura, ambito);
            }
            _ => {}
        }
    }
}
