//! Compila el AST a código intermedio. Si el programa usa algo que el intérprete aún no
//! sabe hacer (módulos con código Rust propio, sql, pantallas de Android...), devuelve `Err`
//! con el motivo y `deimon ejecutar` usa el modo Rust de siempre.
//!
//! Se asume que el programa ya pasó por el generador de Rust sin errores (mismos errores E0xx
//! que `traducir`); aquí solo se cubren los casos que el intérprete no soporta.

use std::collections::HashMap;

use super::codigo::*;
use crate::ast::*;
use crate::generador::linea_de_sentencia;

struct Bucle {
    /// Dónde se vuelve con `continuar`.
    continuar: usize,
    /// Saltos de `romper` por rellenar con el final del bucle.
    romper: Vec<usize>,
    /// Cuántos `intentar` había al empezar el bucle (los de dentro se salen al romper).
    intentos: usize,
}

struct Compilador {
    ops: Vec<Op>,
    constantes: Vec<Constante>,
    funciones: Vec<FuncionCompilada>,
    /// Nombre de función de usuario → (posición en `funciones`, parámetros).
    indices: HashMap<String, (usize, usize)>,
    ambitos: Vec<HashMap<String, usize>>,
    siguiente: usize,
    maximo: usize,
    bucles: Vec<Bucle>,
    /// Cuántos `intentar` rodean a la sentencia que se está compilando.
    intentos: usize,
    /// Funciones de la biblioteca estándar usadas: nombre → (posición en EXTERNAS, parámetros).
    externas: HashMap<String, (usize, usize)>,
    /// Se está compilando el cuerpo de una `prueba` (donde valen `comprobar` y `comprobar_igual`).
    en_prueba: bool,
}

type Resultado<T> = Result<T, String>;

pub fn compilar(programa: &Programa) -> Resultado<Codigo> {
    // Funciones de los módulos de la biblioteca estándar: nombre → (posición en EXTERNAS, parámetros).
    let mut externas: HashMap<String, (usize, usize)> = HashMap::new();
    for extension in &programa.extensiones {
        // Un módulo escrito solo en Deimon (sin funciones.txt ni codigo.rs) no trae nada de
        // Rust: sus funciones ya llegan como funciones normales del programa.
        if extension.funciones.is_empty() && extension.codigo.trim().is_empty() {
            continue;
        }
        match extension.nombre.as_str() {
            "sql" if !cfg!(deimon_sqlite) => {
                return Err("usa el módulo 'sql' (este deimon se preparó sin SQLite dentro)".to_string())
            }
            "pantallas" => return Err("usa el módulo 'pantallas' (es de Android)".to_string()),
            _ => {}
        }
        if !super::motor::es_estandar(&extension.nombre, &extension.codigo) {
            return Err(format!("usa el módulo propio '{}' (código Rust)", extension.nombre));
        }
        for funcion in &extension.funciones {
            match super::motor::indice_externa(&funcion.nombre) {
                Some(indice) => {
                    externas.insert(funcion.nombre.clone(), (indice, funcion.parametros.len()));
                }
                None => {
                    return Err(format!(
                        "usa el módulo '{}' de la biblioteca (necesita el modo Rust)",
                        extension.nombre
                    ))
                }
            }
        }
    }
    let mut definiciones: Vec<(&String, &Vec<String>, &Vec<Sentencia>)> = Vec::new();
    // Rutas web: (camino, parámetros sacados de los `:nombre` del camino, cuerpo).
    let mut rutas: Vec<(&String, Vec<String>, &Vec<Sentencia>)> = Vec::new();
    let mut pruebas: Vec<(&String, &Vec<Sentencia>)> = Vec::new();
    for sentencia in &programa.sentencias {
        match sentencia {
            Sentencia::Funcion {
                nombre,
                parametros,
                cuerpo,
                ..
            } => definiciones.push((nombre, parametros, cuerpo)),
            Sentencia::Ruta { camino, cuerpo, .. } => {
                // Igual que `recoger_rutas` del generador (que ya validó el camino).
                let parametros = camino
                    .split('/')
                    .filter_map(|parte| parte.strip_prefix(':'))
                    .map(str::to_string)
                    .collect();
                rutas.push((camino, parametros, cuerpo));
            }
            Sentencia::Prueba { nombre, cuerpo, .. } => pruebas.push((nombre, cuerpo)),
            // Las pruebas no se ejecutan con `deimon ejecutar` (solo con `deimon probar`).
            Sentencia::Usar { .. } | Sentencia::UsarExtension { .. } => {
                return Err("usa módulos".to_string())
            }
            _ => {}
        }
    }

    let mut c = Compilador {
        ops: Vec::new(),
        constantes: Vec::new(),
        funciones: Vec::new(),
        indices: HashMap::new(),
        ambitos: Vec::new(),
        siguiente: 0,
        maximo: 0,
        bucles: Vec::new(),
        intentos: 0,
        externas,
        en_prueba: false,
    };
    for (posicion, (nombre, parametros, _)) in definiciones.iter().enumerate() {
        c.indices
            .insert((*nombre).clone(), (posicion + 1, parametros.len()));
    }

    // Programa principal (posición 0).
    c.empezar_marco(&[]);
    for sentencia in &programa.sentencias {
        if !matches!(sentencia, Sentencia::Funcion { .. } | Sentencia::Ruta { .. } | Sentencia::Prueba { .. }) {
            c.sentencia(sentencia)?;
        }
    }
    c.ops.push(Op::Fin);
    c.cerrar_marco("<principal>", 0, 0);

    for (nombre, parametros, cuerpo) in &definiciones {
        let inicio = c.ops.len();
        c.empezar_marco(parametros);
        for sentencia in cuerpo.iter() {
            c.sentencia(sentencia)?;
        }
        let nada = c.constante(Constante::Nada);
        c.ops.push(nada);
        c.ops.push(Op::Devolver);
        c.cerrar_marco(nombre, inicio, parametros.len());
    }
    // Cada ruta se compila como una función más; sus parámetros llegan como textos.
    let mut tabla_rutas = Vec::new();
    for (camino, parametros, cuerpo) in &rutas {
        let inicio = c.ops.len();
        c.empezar_marco(parametros);
        for sentencia in cuerpo.iter() {
            c.sentencia(sentencia)?;
        }
        let nada = c.constante(Constante::Nada);
        c.ops.push(nada);
        c.ops.push(Op::Devolver);
        tabla_rutas.push(((*camino).clone(), c.funciones.len()));
        c.cerrar_marco(camino, inicio, parametros.len());
    }
    // Cada prueba también es una función sin parámetros (solo la ejecuta `deimon probar`).
    let mut tabla_pruebas = Vec::new();
    for (nombre, cuerpo) in &pruebas {
        let inicio = c.ops.len();
        c.empezar_marco(&[]);
        c.en_prueba = true;
        for sentencia in cuerpo.iter() {
            c.sentencia(sentencia)?;
        }
        c.en_prueba = false;
        let nada = c.constante(Constante::Nada);
        c.ops.push(nada);
        c.ops.push(Op::Devolver);
        tabla_pruebas.push(((*nombre).clone(), c.funciones.len()));
        c.cerrar_marco(nombre, inicio, 0);
    }
    let fin_ruta = c.ops.len();
    c.ops.push(Op::Fin);
    // La posición 0 empieza en 0 y las demás ya llevan su inicio.
    Ok(Codigo {
        ops: c.ops,
        constantes: c.constantes,
        funciones: c.funciones,
        rutas: tabla_rutas,
        pruebas: tabla_pruebas,
        fin_ruta,
    })
}

impl Compilador {
    fn empezar_marco(&mut self, parametros: &[String]) {
        self.ambitos = vec![HashMap::new()];
        self.siguiente = 0;
        self.maximo = 0;
        self.bucles.clear();
        self.intentos = 0;
        for parametro in parametros {
            let ranura = self.nueva_ranura();
            self.ambitos[0].insert(parametro.clone(), ranura);
        }
    }

    fn cerrar_marco(&mut self, nombre: &str, inicio: usize, parametros: usize) {
        self.funciones.push(FuncionCompilada {
            nombre: nombre.to_string(),
            inicio,
            parametros,
            ranuras: self.maximo,
        });
    }

    fn nueva_ranura(&mut self) -> usize {
        let ranura = self.siguiente;
        self.siguiente += 1;
        self.maximo = self.maximo.max(self.siguiente);
        ranura
    }

    fn abrir_ambito(&mut self) -> usize {
        self.ambitos.push(HashMap::new());
        self.siguiente
    }

    fn cerrar_ambito(&mut self, siguiente: usize) {
        self.ambitos.pop();
        self.siguiente = siguiente;
    }

    fn declarar(&mut self, nombre: &str) -> usize {
        let ranura = self.nueva_ranura();
        if let Some(ambito) = self.ambitos.last_mut() {
            ambito.insert(nombre.to_string(), ranura);
        }
        ranura
    }

    fn buscar(&self, nombre: &str) -> Resultado<usize> {
        self.ambitos
            .iter()
            .rev()
            .find_map(|ambito| ambito.get(nombre).copied())
            .ok_or_else(|| format!("variable '{}' sin declarar", nombre))
    }

    fn constante(&mut self, constante: Constante) -> Op {
        self.constantes.push(constante);
        Op::Constante(self.constantes.len() - 1)
    }

    fn emitir(&mut self, op: Op) -> usize {
        self.ops.push(op);
        self.ops.len() - 1
    }

    fn aqui(&self) -> usize {
        self.ops.len()
    }

    /// Rellena el destino de un salto emitido antes.
    fn parchear(&mut self, posicion: usize, destino: usize) {
        self.ops[posicion] = match self.ops[posicion] {
            Op::Salto(_) => Op::Salto(destino),
            Op::SaltoSiFalso(_) => Op::SaltoSiFalso(destino),
            Op::SaltoNo { operacion, .. } => Op::SaltoNo { operacion, destino },
            Op::SaltoNoC {
                operacion,
                constante,
                ..
            } => Op::SaltoNoC {
                operacion,
                constante,
                destino,
            },
            Op::SaltoNoVC {
                operacion,
                ranura,
                constante,
                ..
            } => Op::SaltoNoVC {
                operacion,
                ranura,
                constante,
                destino,
            },
            Op::SaltoNoVV {
                operacion,
                izquierda,
                derecha,
                ..
            } => Op::SaltoNoVV {
                operacion,
                izquierda,
                derecha,
                destino,
            },
            Op::YCorto(_) => Op::YCorto(destino),
            Op::OCorto(_) => Op::OCorto(destino),
            Op::ParaSiguiente {
                ranura, variable, ..
            } => Op::ParaSiguiente {
                ranura,
                variable,
                fin: destino,
            },
            Op::RangoSiguiente {
                ranura, variable, ..
            } => Op::RangoSiguiente {
                ranura,
                variable,
                fin: destino,
            },
            Op::IntentarEntrar { ranura, .. } => Op::IntentarEntrar {
                ranura,
                destino,
            },
            otro => otro,
        };
    }

    fn bloque(&mut self, sentencias: &[Sentencia]) -> Resultado<()> {
        let marca = self.abrir_ambito();
        for sentencia in sentencias {
            self.sentencia(sentencia)?;
        }
        self.cerrar_ambito(marca);
        Ok(())
    }

    fn sentencia(&mut self, sentencia: &Sentencia) -> Resultado<()> {
        if let Some(linea) = linea_de_sentencia(sentencia) {
            self.emitir(Op::EnLinea(linea));
        }
        match sentencia {
            Sentencia::Declarar { nombre, valor, .. } => {
                self.expresion(valor)?;
                let ranura = self.declarar(nombre);
                self.emitir(Op::Guardar(ranura));
            }
            Sentencia::Asignar { nombre, valor, .. } => {
                let ranura = self.buscar(nombre)?;
                if !self.asignacion_en_sitio(nombre, ranura, valor)? {
                    self.expresion(valor)?;
                    self.emitir(Op::Guardar(ranura));
                }
            }
            Sentencia::AsignarIndice {
                nombre,
                indices,
                valor,
                ..
            } => {
                let ranura = self.buscar(nombre)?;
                for indice in indices {
                    self.expresion(indice)?;
                }
                self.expresion(valor)?;
                self.emitir(Op::AsignarIndice {
                    ranura,
                    indices: indices.len(),
                });
            }
            Sentencia::Si {
                condicion,
                entonces,
                sino,
                ..
            } => {
                let salto_sino = self.condicion(condicion)?;
                self.bloque(entonces)?;
                if sino.is_empty() {
                    let fin = self.aqui();
                    self.parchear(salto_sino, fin);
                } else {
                    let salto_fin = self.emitir(Op::Salto(0));
                    let inicio_sino = self.aqui();
                    self.parchear(salto_sino, inicio_sino);
                    self.bloque(sino)?;
                    let fin = self.aqui();
                    self.parchear(salto_fin, fin);
                }
            }
            Sentencia::Intentar {
                cuerpo,
                nombre,
                captura,
                ..
            } => {
                let marca = self.abrir_ambito();
                self.intentos += 1;
                let entrar = self.emitir(Op::IntentarEntrar {
                    ranura: 0,
                    destino: 0,
                });
                self.bloque(cuerpo)?;
                self.emitir(Op::IntentarSalir);
                self.intentos -= 1;
                self.cerrar_ambito(marca);
                let salto_fin = self.emitir(Op::Salto(0));
                let inicio_captura = self.aqui();
                let marca_captura = self.abrir_ambito();
                let ranura = self.declarar(nombre);
                self.emitir(Op::Guardar(ranura));
                for sentencia in captura {
                    self.sentencia(sentencia)?;
                }
                self.cerrar_ambito(marca_captura);
                let fin = self.aqui();
                self.parchear(entrar, inicio_captura);
                self.ops[entrar] = match self.ops[entrar] {
                    Op::IntentarEntrar { destino, .. } => Op::IntentarEntrar {
                        ranura,
                        destino,
                    },
                    otro => otro,
                };
                self.parchear(salto_fin, fin);
            }
            Sentencia::Mientras {
                condicion,
                cuerpo,
                linea,
            } => {
                let inicio = self.aqui();
                self.emitir(Op::EnLinea(*linea));
                let salir = self.condicion(condicion)?;
                self.bucles.push(Bucle {
                    continuar: inicio,
                    romper: Vec::new(),
                    intentos: self.intentos,
                });
                self.bloque(cuerpo)?;
                self.emitir(Op::Salto(inicio));
                let fin = self.aqui();
                self.parchear(salir, fin);
                self.cerrar_bucle(fin);
            }
            Sentencia::Para {
                nombre,
                coleccion,
                cuerpo,
                linea,
            } => {
                let marca = self.abrir_ambito();
                let ranura = self.nueva_ranura();
                self.nueva_ranura();
                let inicio = match coleccion {
                    // `para x en rango(a, b)`: cuenta sin crear la lista (mismos números,
                    // mismos errores que `rango()`).
                    Expresion::Llamada { nombre: funcion, argumentos, .. }
                        if funcion == "rango" && argumentos.len() == 2 =>
                    {
                        self.nueva_ranura();
                        self.expresion(&argumentos[0])?;
                        self.expresion(&argumentos[1])?;
                        self.emitir(Op::RangoIniciar { ranura, linea: *linea });
                        let variable = self.declarar(nombre);
                        self.emitir(Op::RangoSiguiente { ranura, variable, fin: 0 })
                    }
                    _ => {
                        self.expresion(coleccion)?;
                        self.emitir(Op::ParaIniciar(ranura));
                        let variable = self.declarar(nombre);
                        self.emitir(Op::ParaSiguiente { ranura, variable, fin: 0 })
                    }
                };
                self.bucles.push(Bucle {
                    continuar: inicio,
                    romper: Vec::new(),
                    intentos: self.intentos,
                });
                for sentencia in cuerpo {
                    self.sentencia(sentencia)?;
                }
                self.emitir(Op::Salto(inicio));
                let fin = self.aqui();
                self.parchear(inicio, fin);
                self.cerrar_bucle(fin);
                self.cerrar_ambito(marca);
            }
            Sentencia::Romper { .. } => {
                let intentos = match self.bucles.last() {
                    Some(bucle) => self.intentos - bucle.intentos,
                    None => return Err("'romper' fuera de un bucle".to_string()),
                };
                if intentos > 0 {
                    self.emitir(Op::SalirIntentos(intentos));
                }
                let salto = self.emitir(Op::Salto(0));
                match self.bucles.last_mut() {
                    Some(bucle) => bucle.romper.push(salto),
                    None => return Err("'romper' fuera de un bucle".to_string()),
                }
            }
            Sentencia::Continuar { .. } => {
                let (destino, intentos) = match self.bucles.last() {
                    Some(bucle) => (bucle.continuar, self.intentos - bucle.intentos),
                    None => return Err("'continuar' fuera de un bucle".to_string()),
                };
                if intentos > 0 {
                    self.emitir(Op::SalirIntentos(intentos));
                }
                self.emitir(Op::Salto(destino));
            }
            Sentencia::Devolver { valor, .. } => {
                match valor {
                    Some(valor) => self.expresion(valor)?,
                    None => {
                        let nada = self.constante(Constante::Nada);
                        self.emitir(nada);
                    }
                }
                if self.intentos > 0 {
                    self.emitir(Op::SalirIntentos(self.intentos));
                }
                self.emitir(Op::Devolver);
            }
            Sentencia::Expresion(expresion) => {
                self.expresion(expresion)?;
                self.emitir(Op::Descartar);
            }
            Sentencia::Funcion { .. } => return Err("función dentro de un bloque".to_string()),
            Sentencia::Ruta { .. } => return Err("ruta dentro de un bloque".to_string()),
            Sentencia::Prueba { .. } => return Err("prueba dentro de un bloque".to_string()),
            Sentencia::Usar { .. } | Sentencia::UsarExtension { .. } => {
                return Err("usa módulos".to_string())
            }
        }
        Ok(())
    }

    fn cerrar_bucle(&mut self, fin: usize) {
        if let Some(bucle) = self.bucles.pop() {
            for salto in bucle.romper {
                self.parchear(salto, fin);
            }
        }
    }

    /// `x = agregar(x, v)` y `x = x + v`: igual que el modo Rust, trabajan sobre la propia
    /// variable para no copiar listas y textos grandes en cada vuelta.
    fn asignacion_en_sitio(
        &mut self,
        nombre: &str,
        ranura: usize,
        valor: &Expresion,
    ) -> Resultado<bool> {
        if let Expresion::Llamada {
            nombre: funcion,
            argumentos,
            ..
        } = valor
        {
            if funcion == "agregar" && argumentos.len() == 2 {
                if let Expresion::Variable { nombre: primero, .. } = &argumentos[0] {
                    if primero == nombre {
                        self.expresion(&argumentos[1])?;
                        self.emitir(Op::AgregarEnSitio(ranura));
                        return Ok(true);
                    }
                }
            }
        }
        if let Expresion::Binaria {
            izquierda,
            operador: OperadorBinario::Suma,
            derecha,
            ..
        } = valor
        {
            if let Expresion::Variable { nombre: primero, .. } = izquierda.as_ref() {
                if primero == nombre {
                    match self.operando_simple(derecha)? {
                        Some(Simple::Constante(c)) => {
                            self.emitir(Op::SumaEnSitioC(ranura, c));
                        }
                        Some(Simple::Variable(r)) => {
                            self.emitir(Op::SumaEnSitioV(ranura, r));
                        }
                        None => {
                            self.expresion(derecha)?;
                            self.emitir(Op::SumaEnSitio(ranura));
                        }
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn expresion(&mut self, expresion: &Expresion) -> Resultado<()> {
        match expresion {
            Expresion::Numero(n) => {
                let op = self.constante(Constante::Numero(*n));
                self.emitir(op);
            }
            Expresion::Texto(t) => {
                let op = self.constante(Constante::Texto(t.clone()));
                self.emitir(op);
            }
            Expresion::Logico(b) => {
                let op = self.constante(Constante::Logico(*b));
                self.emitir(op);
            }
            Expresion::Nada => {
                let op = self.constante(Constante::Nada);
                self.emitir(op);
            }
            Expresion::Lista(elementos) => {
                for elemento in elementos {
                    self.expresion(elemento)?;
                }
                self.emitir(Op::Lista(elementos.len()));
            }
            Expresion::Mapa(pares) => {
                for (clave, valor) in pares {
                    self.expresion(clave)?;
                    self.expresion(valor)?;
                }
                self.emitir(Op::Mapa(pares.len()));
            }
            Expresion::Variable { nombre, .. } => {
                let ranura = self.buscar(nombre)?;
                self.emitir(Op::Cargar(ranura));
            }
            Expresion::Indice {
                coleccion, indice, ..
            } => {
                self.expresion(coleccion)?;
                self.expresion(indice)?;
                self.emitir(Op::Indice);
            }
            Expresion::Unaria {
                operador, valor, ..
            } => {
                self.expresion(valor)?;
                self.emitir(match operador {
                    OperadorUnario::Negativo => Op::Negativo,
                    OperadorUnario::No => Op::NoLogico,
                });
            }
            Expresion::Binaria {
                izquierda,
                operador,
                derecha,
                ..
            } => {
                match operador {
                    OperadorBinario::Y | OperadorBinario::O => {
                        self.expresion(izquierda)?;
                        let salto = self.emitir(if *operador == OperadorBinario::Y {
                            Op::YCorto(0)
                        } else {
                            Op::OCorto(0)
                        });
                        self.expresion(derecha)?;
                        self.emitir(Op::ALogico);
                        let fin = self.aqui();
                        self.parchear(salto, fin);
                    }
                    _ => {
                        let operacion = bin_de(operador);
                        match self.operandos(izquierda, derecha)? {
                            Operandos::VariableConstante(r, c) => {
                                self.emitir(Op::BinVC(operacion, r, c));
                            }
                            Operandos::VariableVariable(r1, r2) => {
                                self.emitir(Op::BinVV(operacion, r1, r2));
                            }
                            Operandos::PilaVariable(r) => {
                                self.emitir(Op::BinV(operacion, r));
                            }
                            Operandos::PilaConstante(c) => {
                                self.emitir(Op::BinC(operacion, c));
                            }
                            Operandos::Pila => {
                                self.emitir(Op::Bin(operacion));
                            }
                        }
                    }
                }
            }
            Expresion::Llamada {
                nombre,
                argumentos,
                linea,
                ..
            } => {
                if nombre == "elegir" && argumentos.len() == 3 {
                    self.emitir(Op::EnLinea(*linea));
                    self.expresion(&argumentos[0])?;
                    let si_falso = self.emitir(Op::SaltoSiFalso(0));
                    self.expresion(&argumentos[1])?;
                    let fin = self.emitir(Op::Salto(0));
                    let inicio_falso = self.aqui();
                    self.parchear(si_falso, inicio_falso);
                    self.expresion(&argumentos[2])?;
                    let destino = self.aqui();
                    self.parchear(fin, destino);
                    return Ok(());
                }
                let web = match nombre.as_str() {
                    "visitar" => Some(Web::Visitar),
                    "enviar" => Some(Web::Enviar),
                    "servir" => Some(Web::Servir),
                    _ => None,
                };
                if let Some(tipo) = web {
                    for argumento in argumentos {
                        self.expresion(argumento)?;
                    }
                    self.emitir(Op::Web {
                        tipo,
                        argumentos: argumentos.len(),
                    });
                    return Ok(());
                }
                if nombre == "comprobar" || nombre == "comprobar_igual" {
                    // Solo tienen sentido dentro de `prueba`, que se ejecuta con `deimon probar`.
                    if !self.en_prueba {
                        return Err(format!("usa '{}' fuera de una prueba", nombre));
                    }
                    for argumento in argumentos {
                        self.expresion(argumento)?;
                    }
                    self.emitir(Op::Comprobar {
                        igual: nombre == "comprobar_igual",
                        linea: *linea,
                    });
                    return Ok(());
                }
                if let Some((indice, parametros)) = super::motor::integrada_del_nucleo(nombre) {
                    if parametros != argumentos.len() {
                        return Err(format!("llamada a '{}' con otros argumentos", nombre));
                    }
                    for argumento in argumentos {
                        self.expresion(argumento)?;
                    }
                    self.emitir(Op::Nucleo {
                        indice,
                        argumentos: argumentos.len(),
                    });
                    return Ok(());
                }
                if let Some((indice, parametros)) = self.externas.get(nombre).copied() {
                    if parametros != argumentos.len() {
                        return Err(format!("llamada a '{}' con otros argumentos", nombre));
                    }
                    for argumento in argumentos {
                        self.expresion(argumento)?;
                    }
                    self.emitir(Op::Externa {
                        indice,
                        argumentos: argumentos.len(),
                        linea: *linea,
                    });
                    return Ok(());
                }
                match self.indices.get(nombre).copied() {
                    Some((funcion, parametros)) if parametros == argumentos.len() => {
                        for argumento in argumentos {
                            self.expresion(argumento)?;
                        }
                        self.emitir(Op::Llamar {
                            funcion,
                            linea: *linea,
                        });
                    }
                    Some(_) => return Err(format!("llamada a '{}' con otros argumentos", nombre)),
                    None => return Err(format!("usa la función '{}'", nombre)),
                }
            }
        }
        Ok(())
    }

    /// Condición de `si` / `mientras`: deja emitido el salto que sale cuando no se cumple y
    /// devuelve su posición para parchearlo. Las comparaciones se funden con el salto.
    fn condicion(&mut self, condicion: &Expresion) -> Resultado<usize> {
        if let Expresion::Binaria {
            izquierda,
            operador,
            derecha,
            ..
        } = condicion
        {
            if !matches!(operador, OperadorBinario::Y | OperadorBinario::O) {
                let operacion = bin_de(operador);
                if operacion.es_comparacion() {
                    return Ok(match self.operandos(izquierda, derecha)? {
                        Operandos::VariableConstante(ranura, constante) => {
                            self.emitir(Op::SaltoNoVC {
                                operacion,
                                ranura,
                                constante,
                                destino: 0,
                            })
                        }
                        Operandos::VariableVariable(izquierda, derecha) => {
                            self.emitir(Op::SaltoNoVV {
                                operacion,
                                izquierda,
                                derecha,
                                destino: 0,
                            })
                        }
                        Operandos::PilaConstante(constante) => self.emitir(Op::SaltoNoC {
                            operacion,
                            constante,
                            destino: 0,
                        }),
                        Operandos::PilaVariable(ranura) => {
                            self.emitir(Op::Cargar(ranura));
                            self.emitir(Op::SaltoNo {
                                operacion,
                                destino: 0,
                            })
                        }
                        Operandos::Pila => self.emitir(Op::SaltoNo {
                            operacion,
                            destino: 0,
                        }),
                    });
                }
            }
        }
        self.expresion(condicion)?;
        Ok(self.emitir(Op::SaltoSiFalso(0)))
    }

    /// Deja apilado lo que haga falta de los dos operandos (el izquierdo siempre antes que el
    /// derecho) y dice cuáles se leen directamente de una variable o una constante.
    fn operandos(&mut self, izquierda: &Expresion, derecha: &Expresion) -> Resultado<Operandos> {
        let derecha_simple = self.operando_simple(derecha)?;
        if let Some(izquierda_simple) = self.operando_simple(izquierda)? {
            match (izquierda_simple, derecha_simple) {
                (Simple::Variable(r), Some(Simple::Constante(c))) => {
                    return Ok(Operandos::VariableConstante(r, c))
                }
                (Simple::Variable(r1), Some(Simple::Variable(r2))) => {
                    return Ok(Operandos::VariableVariable(r1, r2))
                }
                _ => {}
            }
        }
        self.expresion(izquierda)?;
        match derecha_simple {
            Some(Simple::Variable(r)) => Ok(Operandos::PilaVariable(r)),
            Some(Simple::Constante(c)) => Ok(Operandos::PilaConstante(c)),
            None => {
                self.expresion(derecha)?;
                Ok(Operandos::Pila)
            }
        }
    }

    /// Variable ya declarada o valor fijo (número, texto, lógico, nada), sin emitir nada.
    fn operando_simple(&mut self, expresion: &Expresion) -> Resultado<Option<Simple>> {
        Ok(match expresion {
            Expresion::Variable { nombre, .. } => Some(Simple::Variable(self.buscar(nombre)?)),
            Expresion::Numero(n) => Some(Simple::Constante(self.nueva_constante(Constante::Numero(*n)))),
            Expresion::Texto(t) => {
                Some(Simple::Constante(self.nueva_constante(Constante::Texto(t.clone()))))
            }
            Expresion::Logico(b) => Some(Simple::Constante(self.nueva_constante(Constante::Logico(*b)))),
            Expresion::Nada => Some(Simple::Constante(self.nueva_constante(Constante::Nada))),
            _ => None,
        })
    }

    fn nueva_constante(&mut self, constante: Constante) -> usize {
        self.constantes.push(constante);
        self.constantes.len() - 1
    }
}

#[derive(Clone, Copy)]
enum Simple {
    Variable(usize),
    Constante(usize),
}

enum Operandos {
    VariableConstante(usize, usize),
    VariableVariable(usize, usize),
    PilaVariable(usize),
    PilaConstante(usize),
    Pila,
}

fn bin_de(operador: &OperadorBinario) -> Bin {
    match operador {
        OperadorBinario::Suma => Bin::Suma,
        OperadorBinario::Resta => Bin::Resta,
        OperadorBinario::Multiplicacion => Bin::Multiplicacion,
        OperadorBinario::Division => Bin::Division,
        OperadorBinario::Resto => Bin::Resto,
        OperadorBinario::Igual => Bin::Igual,
        OperadorBinario::Distinto => Bin::Distinto,
        OperadorBinario::Menor => Bin::Menor,
        OperadorBinario::MenorIgual => Bin::MenorIgual,
        OperadorBinario::Mayor => Bin::Mayor,
        OperadorBinario::MayorIgual => Bin::MayorIgual,
        OperadorBinario::Y | OperadorBinario::O => unreachable!("y/o se compilan aparte"),
    }
}
