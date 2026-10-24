
// Máquina virtual. Se incluye dentro del módulo `motor`, junto al núcleo, para usar sus
// funciones privadas (suma, indice, fallo...) tal cual.

use super::codigo::{Bin, Codigo, Constante, FuncionCompilada, Op, Web};

struct Marco {
    /// Instrucción a la que se vuelve.
    retorno: usize,
    /// Base del marco de quien llamó.
    base: usize,
    /// Línea de la llamada (se recupera al volver, como `con_linea` en el modo Rust).
    linea: usize,
}

/// Un `intentar { ... }` que se está ejecutando: dónde volver si algo falla.
struct Intento {
    /// Marco al que pertenece el intento (para no salir de los del llamante).
    marcos: usize,
    /// Base del marco en el momento de entrar.
    base: usize,
    /// Altura de la pila al entrar (se descartan los temporales al recuperar).
    altura: usize,
    /// Ranura donde se guarda el mapa del error (`e`).
    ranura: usize,
    /// Instrucción del bloque `si falla e { ... }`.
    destino: usize,
}

#[inline(always)]
fn aplicar(operacion: Bin, a: &Valor, b: &Valor) -> Valor {
    // Atajo para dos números (lo más común): las mismas cuentas que el núcleo, sin pasar por
    // él. La división y el resto entre cero, y todo lo demás, siguen por el núcleo.
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        let (x, y) = (*x, *y);
        match operacion {
            Bin::Suma => return Valor::Numero(x + y),
            Bin::Resta => return Valor::Numero(x - y),
            Bin::Multiplicacion => return Valor::Numero(x * y),
            Bin::Division if y != 0.0 => return Valor::Numero(x / y),
            Bin::Resto if y != 0.0 => return Valor::Numero(x % y),
            Bin::Division | Bin::Resto => {}
            otra => return Valor::Logico(comparar(otra, x, y)),
        }
    }
    match operacion {
        Bin::Suma => suma(a, b),
        Bin::Resta => resta(a, b),
        Bin::Multiplicacion => multiplicacion(a, b),
        Bin::Division => division(a, b),
        Bin::Resto => resto(a, b),
        Bin::Igual => igual(a, b),
        Bin::Distinto => distinto(a, b),
        Bin::Menor => menor(a, b),
        Bin::MenorIgual => menor_igual(a, b),
        Bin::Mayor => mayor(a, b),
        Bin::MayorIgual => mayor_igual(a, b),
    }
}

/// Comparación de dos números (igual que `menor`, `igual`... del núcleo con números).
#[inline(always)]
fn comparar(operacion: Bin, x: f64, y: f64) -> bool {
    match operacion {
        Bin::Igual => x == y,
        Bin::Distinto => x != y,
        Bin::Menor => x < y,
        Bin::MenorIgual => x <= y,
        Bin::Mayor => x > y,
        _ => x >= y,
    }
}

/// ¿Se cumple la comparación? Con dos números, sin crear el valor lógico intermedio.
#[inline(always)]
fn se_cumple(operacion: Bin, a: &Valor, b: &Valor) -> bool {
    if let (Valor::Numero(x), Valor::Numero(y)) = (a, b) {
        return comparar(operacion, *x, *y);
    }
    a_logico(&aplicar(operacion, a, b))
}

/// Límites opcionales, idea tomada de `dvm` (útiles cuando una IA o la página de `deimon jugar`
/// prueban código): `DEIMON_MAX_PASOS` (instrucciones) y `DEIMON_MAX_SALIDA` (bytes escritos con
/// `mostrar`/`escribir`). Sin las variables no hay límite, como en el modo Rust. Al pasarse, el
/// programa se detiene con un error que `intentar` no puede atrapar.
fn limite_de_entorno(variable: &str) -> u64 {
    std::env::var(variable)
        .ok()
        .and_then(|texto| texto.trim().replace('_', "").parse::<u64>().ok())
        .filter(|numero| *numero > 0)
        .unwrap_or(u64::MAX)
}

fn maximo_de_pasos() -> u64 {
    static MAXIMO: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *MAXIMO.get_or_init(|| limite_de_entorno("DEIMON_MAX_PASOS"))
}

/// Bytes que aún se pueden escribir (para todo el programa).
static SALIDA_RESTANTE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
static SALIDA_PREPARADA: std::sync::Once = std::sync::Once::new();

#[cold]
fn limite_superado(mensaje: String) -> ! {
    // Ni `intentar` lo atrapa: es una protección, no un error del programa.
    INTENTAR.with(|profundidad| profundidad.set(0));
    fallo(mensaje)
}

fn contar_salida(valor: &Valor, salto_de_linea: bool) {
    SALIDA_PREPARADA.call_once(|| {
        SALIDA_RESTANTE.store(limite_de_entorno("DEIMON_MAX_SALIDA"), std::sync::atomic::Ordering::Relaxed)
    });
    if SALIDA_RESTANTE.load(std::sync::atomic::Ordering::Relaxed) == u64::MAX {
        return;
    }
    let bytes = texto_de(valor).len() as u64 + salto_de_linea as u64;
    let antes = SALIDA_RESTANTE.fetch_sub(bytes.min(SALIDA_RESTANTE.load(std::sync::atomic::Ordering::Relaxed)), std::sync::atomic::Ordering::Relaxed);
    if bytes > antes {
        limite_superado(format!(
            "el programa escribió más de lo permitido (DEIMON_MAX_SALIDA = {} bytes)",
            limite_de_entorno("DEIMON_MAX_SALIDA")
        ));
    }
}

/// Posiciones de `mostrar` y `escribir` en `INTEGRADAS_NUCLEO` (para contar lo que escriben).
fn integradas_que_escriben() -> (usize, usize) {
    static POSICIONES: std::sync::OnceLock<(usize, usize)> = std::sync::OnceLock::new();
    *POSICIONES.get_or_init(|| {
        let posicion = |nombre| integrada_del_nucleo(nombre).map(|(i, _)| i).unwrap_or(usize::MAX);
        (posicion("mostrar"), posicion("escribir"))
    })
}

struct Maquina<'a> {
    codigo: &'a std::sync::Arc<Codigo>,
    ops: &'a [Op],
    constantes: Rc<Vec<Valor>>,
    funciones: &'a [FuncionCompilada],
    /// Marcos que no son llamadas del programa (el de vuelta de una ruta): no cuentan para
    /// el límite de llamadas, igual que en el modo Rust.
    marcos_iniciales: usize,
    /// Instrucciones que aún puede ejecutar (`DEIMON_MAX_PASOS`; sin límite si no se pide).
    pasos: u64,
    pila: Vec<Valor>,
    marcos: Vec<Marco>,
    intentos: Vec<Intento>,
    base: usize,
    pc: usize,
}

impl<'a> Maquina<'a> {
    fn ejecutar(&mut self) {
        // Una sola red para todo el programa (no una por instrucción): si un error salta
        // dentro de un `intentar`, `recuperar` lleva al bloque `si falla` y se sigue; si no
        // hay `intentar`, el error sigue su camino como en el modo Rust.
        loop {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.correr())) {
                Ok(()) => return,
                Err(causa) => self.recuperar(causa),
            }
        }
    }

    /// Ejecuta instrucciones hasta llegar a `Fin`.
    fn correr(&mut self) {
        loop {
            if self.pasos == 0 {
                limite_superado(format!(
                    "el programa pasó de {} pasos (DEIMON_MAX_PASOS); ¿hay un bucle que no termina?",
                    maximo_de_pasos()
                ));
            }
            self.pasos -= 1;
            let actual = self.pc;
            self.pc += 1;
            if self.paso(actual) {
                return;
            }
        }
    }

    /// Ejecuta una instrucción. Devuelve `true` al llegar a `Fin`.
    #[inline(always)]
    fn paso(&mut self, actual: usize) -> bool {
        match self.ops[actual] {
            Op::Constante(i) => self.pila.push(self.constantes[i].clone()),
            Op::Cargar(ranura) => {
                let valor = self.pila[self.base + ranura].clone();
                self.pila.push(valor);
            }
            Op::Guardar(ranura) => {
                let valor = self.pila.pop().unwrap();
                self.pila[self.base + ranura] = valor;
            }
            Op::Descartar => {
                self.pila.pop();
            }
            Op::EnLinea(linea) => en_linea(linea),

            Op::Bin(operacion) => {
                let derecha = self.pila.pop().unwrap();
                let ultimo = self.pila.len() - 1;
                let resultado = aplicar(operacion, &self.pila[ultimo], &derecha);
                self.pila[ultimo] = resultado;
            }
            Op::BinV(operacion, ranura) => {
                let ultimo = self.pila.len() - 1;
                let resultado = aplicar(operacion, &self.pila[ultimo], &self.pila[self.base + ranura]);
                self.pila[ultimo] = resultado;
            }
            Op::BinC(operacion, constante) => {
                let ultimo = self.pila.len() - 1;
                let resultado = aplicar(operacion, &self.pila[ultimo], &self.constantes[constante]);
                self.pila[ultimo] = resultado;
            }
            Op::BinVC(operacion, ranura, constante) => {
                let resultado = aplicar(
                    operacion,
                    &self.pila[self.base + ranura],
                    &self.constantes[constante],
                );
                self.pila.push(resultado);
            }
            Op::BinVV(operacion, izquierda, derecha) => {
                let resultado = aplicar(
                    operacion,
                    &self.pila[self.base + izquierda],
                    &self.pila[self.base + derecha],
                );
                self.pila.push(resultado);
            }
            Op::SaltoNo { operacion, destino } => {
                let derecha = self.pila.pop().unwrap();
                let izquierda = self.pila.pop().unwrap();
                if !se_cumple(operacion, &izquierda, &derecha) {
                    self.pc = destino;
                }
            }
            Op::SaltoNoC {
                operacion,
                constante,
                destino,
            } => {
                let izquierda = self.pila.pop().unwrap();
                if !se_cumple(operacion, &izquierda, &self.constantes[constante]) {
                    self.pc = destino;
                }
            }
            Op::SaltoNoVC {
                operacion,
                ranura,
                constante,
                destino,
            } => {
                if !se_cumple(operacion, &self.pila[self.base + ranura], &self.constantes[constante]) {
                    self.pc = destino;
                }
            }
            Op::SaltoNoVV {
                operacion,
                izquierda,
                derecha,
                destino,
            } => {
                if !se_cumple(operacion, &self.pila[self.base + izquierda], &self.pila[self.base + derecha]) {
                    self.pc = destino;
                }
            }
            Op::Negativo => {
                let valor = self.pila.pop().unwrap();
                self.pila.push(negativo(&valor));
            }
            Op::NoLogico => {
                let valor = self.pila.pop().unwrap();
                self.pila.push(no_logico(&valor));
            }
            Op::YCorto(destino) => {
                let valor = self.pila.pop().unwrap();
                if !a_logico(&valor) {
                    self.pila.push(Valor::Logico(false));
                    self.pc = destino;
                }
            }
            Op::OCorto(destino) => {
                let valor = self.pila.pop().unwrap();
                if a_logico(&valor) {
                    self.pila.push(Valor::Logico(true));
                    self.pc = destino;
                }
            }
            Op::ALogico => {
                let valor = self.pila.pop().unwrap();
                self.pila.push(Valor::Logico(a_logico(&valor)));
            }

            Op::Lista(n) => {
                let elementos = self.pila.split_off(self.pila.len() - n);
                self.pila.push(Valor::Lista(Rc::new(elementos)));
            }
            Op::Mapa(n) => {
                let mut sueltos = self.pila.split_off(self.pila.len() - 2 * n).into_iter();
                let mut pares = Vec::with_capacity(n);
                while let (Some(clave), Some(valor)) = (sueltos.next(), sueltos.next()) {
                    pares.push((clave, valor));
                }
                self.pila.push(crear_mapa(pares));
            }
            Op::Indice => {
                let posicion = self.pila.pop().unwrap();
                let ultimo = self.pila.len() - 1;
                let resultado = indice(&self.pila[ultimo], &posicion);
                self.pila[ultimo] = resultado;
            }
            Op::AsignarIndice { ranura, indices } => {
                let valor = self.pila.pop().unwrap();
                if indices == 1 {
                    let posicion = self.pila.pop().unwrap();
                    poner_indice(&mut self.pila[self.base + ranura], &posicion, valor);
                } else {
                    let posiciones = self.pila.split_off(self.pila.len() - indices);
                    let mut destino = &mut self.pila[self.base + ranura];
                    for posicion in &posiciones[..indices - 1] {
                        destino = indice_mut(destino, posicion);
                    }
                    poner_indice(destino, &posiciones[indices - 1], valor);
                }
            }

            Op::Salto(destino) => self.pc = destino,
            Op::SaltoSiFalso(destino) => {
                let valor = self.pila.pop().unwrap();
                if !a_logico(&valor) {
                    self.pc = destino;
                }
            }

            Op::IntentarEntrar { ranura, destino } => {
                intentar_entrar();
                self.intentos.push(Intento {
                    marcos: self.marcos.len(),
                    base: self.base,
                    altura: self.pila.len(),
                    ranura,
                    destino,
                });
            }
            Op::IntentarSalir => {
                self.intentos.pop();
                intentar_salir();
            }
            Op::SalirIntentos(n) => {
                for _ in 0..n {
                    if self.intentos.pop().is_some() {
                        intentar_salir();
                    }
                }
            }

            Op::ParaIniciar(ranura) => {
                let valor = self.pila.pop().unwrap();
                let elementos = match valor {
                    Valor::Lista(elementos) => Valor::Lista(elementos),
                    Valor::Mapa(mapa) => claves_de_mapa(&mapa),
                    otro => fallo(format!("para necesita una lista, no {}", otro.tipo())),
                };
                self.pila[self.base + ranura] = elementos;
                self.pila[self.base + ranura + 1] = Valor::Numero(0.0);
            }
            Op::ParaSiguiente { ranura, variable, fin } => {
                let posicion = match &self.pila[self.base + ranura + 1] {
                    Valor::Numero(n) => *n as usize,
                    _ => 0,
                };
                let siguiente = match &self.pila[self.base + ranura] {
                    Valor::Lista(elementos) => elementos.get(posicion).cloned(),
                    _ => None,
                };
                match siguiente {
                    Some(elemento) => {
                        self.pila[self.base + variable] = elemento;
                        self.pila[self.base + ranura + 1] = Valor::Numero((posicion + 1) as f64);
                    }
                    None => {
                        self.pila[self.base + ranura] = Valor::Nada;
                        self.pc = fin;
                    }
                }
            }

            Op::RangoIniciar { ranura, linea } => {
                let fin = self.pila.pop().unwrap();
                let inicio = self.pila.pop().unwrap();
                let numero = |valor: &Valor| match valor {
                    Valor::Numero(n) => *n,
                    // Lo que no es un número da el mismo error que `rango()`.
                    otro => {
                        rango(otro, otro);
                        unreachable!()
                    }
                };
                let (a, b) = (numero(&inicio), numero(&fin));
                let (desde, hasta, al_reves) = rango_numerico(a, b, linea);
                // Contadores internos (el programa no los ve): el número actual (entero
                // guardado tal cual), cuántos faltan y el sentido.
                let actual = if al_reves { hasta } else { desde };
                self.pila[self.base + ranura] = Valor::Numero(f64::from_bits(actual as u64));
                self.pila[self.base + ranura + 1] = Valor::Numero((hasta - desde + 1) as f64);
                self.pila[self.base + ranura + 2] = Valor::Logico(al_reves);
            }
            Op::RangoSiguiente { ranura, variable, fin } => {
                let faltan = match self.pila[self.base + ranura + 1] {
                    Valor::Numero(n) => n,
                    _ => 0.0,
                };
                if faltan <= 0.0 {
                    self.pc = fin;
                } else {
                    let actual = match self.pila[self.base + ranura] {
                        Valor::Numero(n) => n.to_bits() as i64,
                        _ => 0,
                    };
                    let al_reves = matches!(self.pila[self.base + ranura + 2], Valor::Logico(true));
                    self.pila[self.base + variable] = Valor::Numero(actual as f64);
                    let siguiente = if al_reves { actual.wrapping_sub(1) } else { actual.wrapping_add(1) };
                    self.pila[self.base + ranura] = Valor::Numero(f64::from_bits(siguiente as u64));
                    self.pila[self.base + ranura + 1] = Valor::Numero(faltan - 1.0);
                }
            }
            Op::AgregarEnSitio(ranura) => {
                let nuevo = self.pila.pop().unwrap();
                let lista = std::mem::replace(&mut self.pila[self.base + ranura], Valor::Nada);
                self.pila[self.base + ranura] = agregar_mover(lista, nuevo);
            }
            Op::SumaEnSitioC(ranura, constante) => {
                if let (Valor::Numero(x), Valor::Numero(y)) =
                    (&mut self.pila[self.base + ranura], &self.constantes[constante])
                {
                    *x += *y;
                    return false;
                }
                let actual = std::mem::replace(&mut self.pila[self.base + ranura], Valor::Nada);
                self.pila[self.base + ranura] = suma_mover(actual, &self.constantes[constante]);
            }
            Op::SumaEnSitioV(ranura, otra) => {
                let sumando = self.pila[self.base + otra].clone();
                let actual = std::mem::replace(&mut self.pila[self.base + ranura], Valor::Nada);
                self.pila[self.base + ranura] = suma_mover(actual, &sumando);
            }
            Op::SumaEnSitio(ranura) => {
                let nuevo = self.pila.pop().unwrap();
                let actual = std::mem::replace(&mut self.pila[self.base + ranura], Valor::Nada);
                self.pila[self.base + ranura] = suma_mover(actual, &nuevo);
            }

            Op::Nucleo { indice, argumentos } => {
                let args = self.pila.split_off(self.pila.len() - argumentos);
                let (mostrar, escribir) = integradas_que_escriben();
                if indice == mostrar || indice == escribir {
                    contar_salida(&args[0], indice == mostrar);
                }
                let resultado = (INTEGRADAS_NUCLEO[indice].2)(&args);
                self.pila.push(resultado);
            }
            Op::Comprobar { igual, linea } => {
                let resultado = if igual {
                    let esperado = self.pila.pop().unwrap();
                    let obtenido = self.pila.pop().unwrap();
                    comprobar_igual(&obtenido, &esperado, linea)
                } else {
                    let condicion = self.pila.pop().unwrap();
                    comprobar(&condicion, linea)
                };
                self.pila.push(resultado);
            }
            Op::Web { tipo, argumentos } => {
                let args = self.pila.split_off(self.pila.len() - argumentos);
                let rutas = rutas_del_programa(self.codigo);
                let resultado = match tipo {
                    Web::Visitar => visitar(&args[0], rutas),
                    Web::Enviar => enviar(&args[0], &args[1], rutas),
                    Web::Servir => servir(&args[0], rutas),
                };
                self.pila.push(resultado);
            }
            Op::Llamar { funcion, linea } => {
                // Mismo límite y mismo mensaje que el modo Rust (`entrar_llamada` del núcleo).
                if self.marcos.len() - self.marcos_iniciales >= LIMITE_LLAMADAS {
                    demasiadas_llamadas();
                }
                let destino = &self.funciones[funcion];
                self.marcos.push(Marco { retorno: self.pc, base: self.base, linea });
                self.base = self.pila.len() - destino.parametros;
                for _ in destino.parametros..destino.ranuras {
                    self.pila.push(Valor::Nada);
                }
                self.pc = destino.inicio;
            }
            Op::Externa { indice, argumentos, linea } => {
                let args = self.pila.split_off(self.pila.len() - argumentos);
                let resultado = llamar_externa(indice, linea, args);
                self.pila.push(resultado);
            }
            Op::Devolver => {
                let resultado = self.pila.pop().unwrap();
                self.pila.truncate(self.base);
                self.pila.push(resultado);
                let marco = self.marcos.pop().unwrap();
                self.pc = marco.retorno;
                self.base = marco.base;
                en_linea(marco.linea);
            }
            Op::Fin => return true,
        }
        false
    }

    /// Un error de ejecución o una señal de control ha interrumpido una instrucción: se
    /// vuelve al `intentar` más interno del marco correspondiente.
    fn recuperar(&mut self, causa: Box<dyn std::any::Any + Send>) {
        if let Some(error) = causa.downcast_ref::<ErrorIntento>() {
            if let Some(intento) = self.intentos.pop() {
                self.pila.truncate(intento.altura);
                self.marcos.truncate(intento.marcos);
                self.base = intento.base;
                let mapa = error_intento(error);
                self.pila.push(mapa);
                self.pc = intento.destino;
                intentar_salir();
                return;
            }
        }
        std::panic::resume_unwind(causa);
    }
}

/// Las constantes del programa como valores. Cada hilo las prepara una sola vez (las rutas
/// web se atienden en varios hilos y los `Valor` no se pueden compartir entre ellos).
fn constantes_de(codigo: &std::sync::Arc<Codigo>) -> Rc<Vec<Valor>> {
    thread_local! {
        static PREPARADAS: std::cell::RefCell<Option<(usize, Rc<Vec<Valor>>)>> = const { std::cell::RefCell::new(None) };
    }
    let clave = std::sync::Arc::as_ptr(codigo) as usize;
    PREPARADAS.with(|preparadas| {
        if let Some((de, valores)) = preparadas.borrow().as_ref() {
            if *de == clave {
                return Rc::clone(valores);
            }
        }
        let valores: Rc<Vec<Valor>> = Rc::new(
            codigo
                .constantes
                .iter()
                .map(|constante| match constante {
                    Constante::Numero(n) => Valor::Numero(*n),
                    Constante::Texto(t) => Valor::Texto(Rc::new(t.clone())),
                    Constante::Logico(b) => Valor::Logico(*b),
                    Constante::Nada => Valor::Nada,
                })
                .collect(),
        );
        *preparadas.borrow_mut() = Some((clave, Rc::clone(&valores)));
        valores
    })
}

/// Las rutas del programa para el servidor del núcleo (el mismo que usa el modo Rust): cada
/// una ejecuta su función en una máquina propia del hilo que atiende la visita.
fn rutas_del_programa(codigo: &std::sync::Arc<Codigo>) -> Vec<Ruta> {
    codigo
        .rutas
        .iter()
        .map(|(camino, funcion)| {
            let codigo = std::sync::Arc::clone(codigo);
            let funcion = *funcion;
            Ruta {
                camino: camino.clone(),
                manejador: std::sync::Arc::new(move |parametros: Vec<String>| ejecutar_aparte(&codigo, funcion, parametros)),
            }
        })
        .collect()
}

/// Ejecuta aparte la función de una ruta o de una prueba (con los parámetros de la ruta, como
/// textos) y devuelve su resultado.
fn ejecutar_aparte(codigo: &std::sync::Arc<Codigo>, funcion: usize, parametros: Vec<String>) -> Valor {
    let destino = &codigo.funciones[funcion];
    let mut pila: Vec<Valor> = Vec::with_capacity(256);
    for posicion in 0..destino.parametros {
        pila.push(Valor::Texto(Rc::new(parametros.get(posicion).cloned().unwrap_or_default())));
    }
    for _ in destino.parametros..destino.ranuras {
        pila.push(Valor::Nada);
    }
    let mut maquina = Maquina {
        codigo,
        ops: &codigo.ops,
        constantes: constantes_de(codigo),
        funciones: &codigo.funciones,
        marcos_iniciales: 1,
        pasos: maximo_de_pasos(),
        pila,
        // Al devolver, la ruta vuelve a un `Fin` y la respuesta queda en la pila.
        marcos: vec![Marco { retorno: codigo.fin_ruta, base: 0, linea: 0 }],
        intentos: Vec::new(),
        base: 0,
        pc: destino.inicio,
    };
    maquina.ejecutar();
    maquina.pila.pop().unwrap_or(Valor::Nada)
}

/// `deimon probar`: corre las pruebas del programa con el mismo `ejecutar_pruebas` del núcleo
/// que el modo Rust (datos vacíos y sin sesión en cada una, mismo formato de resultado).
pub fn ejecutar_pruebas_codigo(codigo: &std::sync::Arc<Codigo>) {
    let pruebas: Vec<(String, _)> = codigo
        .pruebas
        .iter()
        .map(|(nombre, funcion)| {
            let funcion = *funcion;
            (nombre.clone(), move || ejecutar_aparte(codigo, funcion, Vec::new()))
        })
        .collect();
    ejecutar_pruebas(pruebas);
}

pub fn ejecutar_codigo(codigo: &std::sync::Arc<Codigo>) {
    let mut pila: Vec<Valor> = Vec::with_capacity(4096);
    for _ in 0..codigo.funciones[0].ranuras {
        pila.push(Valor::Nada);
    }
    let mut maquina = Maquina {
        codigo,
        ops: &codigo.ops,
        constantes: constantes_de(codigo),
        funciones: &codigo.funciones,
        marcos_iniciales: 0,
        pasos: maximo_de_pasos(),
        pila,
        marcos: Vec::new(),
        intentos: Vec::new(),
        base: 0,
        pc: 0,
    };
    maquina.ejecutar();
}
