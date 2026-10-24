const PERFIL_LN: &str = "@PERFIL@";

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

// @PURO_INICIO@ (de aquí a @PURO_FIN@ no se usa std: el destino pc-sin-sistema copia estas partes)
#[derive(Debug, Clone, PartialEq)]
enum Valor {
    Numero(f64),
    Texto(Rc<String>),
    Logico(bool),
    Nada,
    Lista(Rc<Vec<Valor>>),
    Mapa(Rc<MapaValores>),
    /// Una función del programa usada como valor: `sea f = doble` y después `f(4)`.
    Funcion(Rc<FuncionValor>),
}

/// Función del programa guardada en un valor. `numero` es su posición en la tabla del motor
/// que la ejecuta (el programa traducido o el intérprete); `parametros`, cuántos recibe.
#[derive(Debug, Clone, PartialEq)]
struct FuncionValor {
    nombre: String,
    parametros: usize,
    numero: usize,
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
            Valor::Funcion(_) => "función",
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
        Valor::Funcion(funcion) => format!("función {}", funcion.nombre),
    }
}

/// Crea el valor de una función del programa (lo usa el código traducido y el intérprete).
fn valor_funcion(nombre: &str, parametros: usize, numero: usize) -> Valor {
    Valor::Funcion(Rc::new(FuncionValor { nombre: nombre.to_string(), parametros, numero }))
}

/// `f(a, b)` cuando `f` es una variable: comprueba que sea una función con esos parámetros y la
/// llama a través del motor que la ejecuta (`despachar_funcion`).
fn llamar_funcion(funcion: &Valor, argumentos: Vec<Valor>) -> Valor {
    let funcion = match funcion {
        Valor::Funcion(funcion) => funcion,
        otro => fallo(format!("se intenta llamar a {} como si fuera una función", otro.tipo())),
    };
    if funcion.parametros != argumentos.len() {
        fallo(format!(
            "la función '{}' espera {} {} y se han pasado {}",
            funcion.nombre,
            funcion.parametros,
            if funcion.parametros == 1 { "argumento" } else { "argumentos" },
            argumentos.len()
        ));
    }
    despachar_funcion(funcion.numero, argumentos)
}

/// La función que se pasa a filtrar(), transformar() u ordenar_por() tiene que recibir un valor.
fn funcion_de_un_parametro<'a>(valor: &'a Valor, quien: &str) -> &'a Valor {
    match valor {
        Valor::Funcion(funcion) if funcion.parametros == 1 => valor,
        Valor::Funcion(funcion) => fallo(format!(
            "{}() necesita una función de 1 parámetro, y '{}' tiene {}",
            quien, funcion.nombre, funcion.parametros
        )),
        otro => fallo(format!(
            "{}() espera una función como segundo argumento (su nombre, sin paréntesis), no {}",
            quien,
            otro.tipo()
        )),
    }
}

fn elementos_para<'a>(lista: &'a Valor, quien: &str) -> &'a [Valor] {
    match lista {
        Valor::Lista(elementos) => elementos,
        otro => fallo(format!("{}() espera una lista como primer argumento, no {}", quien, otro.tipo())),
    }
}

/// `filtrar(lista, f)`: los elementos para los que f devuelve verdadero, en su orden.
fn filtrar(lista: &Valor, funcion: &Valor) -> Valor {
    let funcion = funcion_de_un_parametro(funcion, "filtrar");
    let mut elegidos = Vec::new();
    for elemento in elementos_para(lista, "filtrar") {
        match llamar_funcion(funcion, vec![elemento.clone()]) {
            Valor::Logico(true) => elegidos.push(elemento.clone()),
            Valor::Logico(false) => {}
            otro => fallo(format!("filtrar(): la función tiene que devolver verdadero o falso, no {}", otro.tipo())),
        }
    }
    Valor::Lista(Rc::new(elegidos))
}

/// `transformar(lista, f)`: una lista nueva con f aplicada a cada elemento.
fn transformar(lista: &Valor, funcion: &Valor) -> Valor {
    let funcion = funcion_de_un_parametro(funcion, "transformar");
    let elementos = elementos_para(lista, "transformar");
    Valor::Lista(Rc::new(
        elementos.iter().map(|elemento| llamar_funcion(funcion, vec![elemento.clone()])).collect(),
    ))
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
// @PURO_FIN@

thread_local! {
    static EN_RUTA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EN_PRUEBA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LINEA: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Cuántos bloques `intentar` hay abiertos (para convertir los errores en algo atrapable).
    static INTENTAR: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Valor de un `devolver` que tiene que salir de dentro de un `intentar`.
    static VALOR_DEVOLVER: std::cell::RefCell<Option<Valor>> = const { std::cell::RefCell::new(None) };
    /// Enlace de pánico que había antes de entrar en `intentar` (se restaura al salir).
    static HOOK_ANTERIOR: std::cell::RefCell<Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send>>> = const { std::cell::RefCell::new(None) };
    /// Cuántas funciones del programa hay llamadas una dentro de otra ahora mismo.
    static PROFUNDIDAD: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Dirección donde empieza la pila de este hilo y su tamaño (0 si no se conoce).
    static PILA: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

/// Pila del hilo que ejecuta el programa. Es memoria reservada, no usada: solo se ocupa la
/// parte que de verdad hace falta. Con ella caben las 10 000 llamadas aunque cada función
/// guarde muchos datos.
const PILA_PROGRAMA: usize = 256 * 1024 * 1024;
/// Pila de cada hilo del servidor web (hay varios a la vez).
const PILA_HILO_WEB: usize = 64 * 1024 * 1024;
/// Espacio que se deja libre al final de la pila para poder escribir el error.
const MARGEN_PILA: usize = 4 * 1024 * 1024;

/// Apunta dónde empieza la pila del hilo actual (se llama al empezar cada hilo).
#[inline(never)]
fn marcar_pila(tamano: usize) {
    let marca = 0u8;
    PILA.with(|p| p.set((&marca as *const u8 as usize, tamano)));
}

/// Ejecuta el programa en un hilo con una pila grande. Si el sistema no da el hilo, lo
/// ejecuta aquí mismo con la pila normal (unos 8 MB), igualmente vigilada.
fn con_pila_grande(programa: fn()) {
    let hilo = std::thread::Builder::new()
        .name("main".to_string())
        .stack_size(PILA_PROGRAMA)
        .spawn(move || {
            marcar_pila(PILA_PROGRAMA);
            programa()
        });
    match hilo {
        Ok(hilo) => {
            if hilo.join().is_err() {
                std::process::exit(101);
            }
        }
        Err(_) => {
            marcar_pila(8 * 1024 * 1024);
            programa()
        }
    }
}

/// Máximo de llamadas una dentro de otra. Sin este límite, una función que se llama a sí
/// misma sin parar rompería el programa (modo Rust) o se comería la memoria (intérprete).
/// El intérprete usa el mismo número para que los dos motores se comporten igual.
const LIMITE_LLAMADAS: usize = 10_000;

#[cold]
fn demasiadas_llamadas() -> ! {
    fallo(format!(
        "hay más de {} llamadas a funciones una dentro de otra; revisa que la función tenga un caso que termine sin volver a llamarse",
        LIMITE_LLAMADAS
    ))
}

/// Mientras vive, cuenta una llamada en curso (al salir de la función, aunque sea por un
/// error atrapado con `intentar`, se descuenta sola).
struct Llamada;

impl Drop for Llamada {
    fn drop(&mut self) {
        PROFUNDIDAD.with(|p| p.set(p.get() - 1));
    }
}

#[inline]
fn entrar_llamada() -> Llamada {
    let n = PROFUNDIDAD.with(|p| {
        let n = p.get() + 1;
        p.set(n);
        n
    });
    let llamada = Llamada;
    if n > LIMITE_LLAMADAS {
        demasiadas_llamadas();
    }
    // Último seguro: si las funciones guardan tantos datos que la pila se acaba antes de
    // llegar al límite, se para con un error claro en vez de romper el programa.
    let (inicio, tamano) = PILA.with(|p| p.get());
    if inicio != 0 {
        let aqui = &n as *const usize as usize;
        if inicio.saturating_sub(aqui) + MARGEN_PILA > tamano {
            pila_llena();
        }
    }
    llamada
}

#[cold]
fn pila_llena() -> ! {
    fallo(
        "las llamadas a funciones una dentro de otra han llenado la memoria reservada para ellas; revisa que la función tenga un caso que termine sin volver a llamarse"
            .to_string(),
    )
}

#[inline]
fn profundidad_actual() -> usize {
    PROFUNDIDAD.with(|p| p.get())
}

/// Profundidad de una llamada hecha desde el camino rápido de números (que la lleva como
/// parámetro para no tocar memoria compartida en cada llamada).
#[inline(always)]
fn llamada_numerica(profundidad: usize, linea: usize) -> usize {
    if profundidad >= LIMITE_LLAMADAS {
        en_linea(linea);
        demasiadas_llamadas();
    }
    profundidad + 1
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
/// Error de ejecución atrapable por `intentar`. Guarda el mensaje y la línea por separado.
struct ErrorIntento {
    mensaje: String,
    linea: usize,
}
/// Señales para salir de un `intentar` con `romper`, `continuar` o `devolver`.
struct ControlRomper;
struct ControlContinuar;
struct ControlDevolver;

fn intentar_entrar() {
    INTENTAR.with(|profundidad| {
        let actual = profundidad.get();
        if actual == 0 {
            let anterior = std::panic::take_hook();
            HOOK_ANTERIOR.with(|guardado| *guardado.borrow_mut() = Some(anterior));
            std::panic::set_hook(Box::new(|informacion| {
                let carga = informacion.payload();
                if carga.downcast_ref::<ErrorIntento>().is_some()
                    || carga.downcast_ref::<ControlRomper>().is_some()
                    || carga.downcast_ref::<ControlContinuar>().is_some()
                    || carga.downcast_ref::<ControlDevolver>().is_some()
                {
                    return;
                }
                HOOK_ANTERIOR.with(|guardado| {
                    if let Some(anterior) = guardado.borrow().as_ref() {
                        anterior(informacion);
                    }
                });
            }));
        }
        profundidad.set(actual + 1);
    });
}

fn intentar_salir() {
    INTENTAR.with(|profundidad| {
        let nuevo = profundidad.get().saturating_sub(1);
        profundidad.set(nuevo);
        if nuevo == 0 {
            let _ = std::panic::take_hook();
            let anterior = HOOK_ANTERIOR.with(|guardado| guardado.borrow_mut().take());
            if let Some(anterior) = anterior {
                std::panic::set_hook(anterior);
            }
        }
    });
}

fn error_intento(valor: &ErrorIntento) -> Valor {
    Valor::mapa(vec![
        ("mensaje".to_string(), Valor::Texto(Rc::new(valor.mensaje.clone()))),
        ("linea".to_string(), Valor::Numero(valor.linea as f64)),
    ])
}

fn devolver_de_intentar(valor: Valor) -> ! {
    VALOR_DEVOLVER.with(|guardado| *guardado.borrow_mut() = Some(valor));
    std::panic::panic_any(ControlDevolver);
}

fn tomar_valor_devuelto() -> Valor {
    VALOR_DEVOLVER.with(|guardado| guardado.borrow_mut().take().unwrap_or(Valor::Nada))
}

fn fallo(mensaje: String) -> ! {
    let linea = LINEA.with(|actual| actual.get());
    if INTENTAR.with(|profundidad| profundidad.get()) > 0 {
        std::panic::panic_any(ErrorIntento { mensaje, linea });
    }
    let en_prueba = EN_PRUEBA.with(|estado| estado.get());
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

/// `provocar_error(mensaje)`: lanza un error de ejecución, como `throw` en otros lenguajes.
#[inline]
fn provocar_error(mensaje: &Valor) -> Valor {
    fallo(texto_de(mensaje))
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

/// Corre las pruebas y escribe el resultado de cada una para `deimon probar`. Cada prueba es
/// una función del programa traducido o una función compilada por el intérprete.
fn ejecutar_pruebas<N: AsRef<str>, F: Fn() -> Valor>(pruebas: Vec<(N, F)>) {
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
        let nombre = nombre.as_ref();
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

// --- Funciones como valores. El programa traducido fija una tabla (una función normal) al
// empezar; el intérprete fija en cada hilo cómo ejecutar una función suya.

static DESPACHADOR_TRADUCIDO: std::sync::OnceLock<fn(usize, Vec<Valor>) -> Valor> = std::sync::OnceLock::new();

thread_local! {
    static DESPACHADOR_LOCAL: std::cell::RefCell<Option<Rc<dyn Fn(usize, Vec<Valor>) -> Valor>>> = const { std::cell::RefCell::new(None) };
}

fn fijar_despachador(despachador: fn(usize, Vec<Valor>) -> Valor) {
    let _ = DESPACHADOR_TRADUCIDO.set(despachador);
}

#[allow(dead_code)]
fn fijar_despachador_local(despachador: Rc<dyn Fn(usize, Vec<Valor>) -> Valor>) {
    DESPACHADOR_LOCAL.with(|local| *local.borrow_mut() = Some(despachador));
}

fn despachar_funcion(numero: usize, argumentos: Vec<Valor>) -> Valor {
    let local = DESPACHADOR_LOCAL.with(|local| local.borrow().clone());
    if let Some(despachador) = local {
        return despachador(numero, argumentos);
    }
    match DESPACHADOR_TRADUCIDO.get() {
        Some(despachador) => despachador(numero, argumentos),
        None => fallo("error interno: no hay tabla de funciones".to_string()),
    }
}

// --- Funciones de la máquina. En un PC sin sistema (destino pc-sin-sistema) usan la pantalla
// y el teclado de verdad (nucleo/sin_sistema.rs); aquí hacen lo mismo en la terminal.

fn numero_de_maquina(valor: &Valor, funcion: &str) -> f64 {
    match valor {
        Valor::Numero(n) => *n,
        otro => fallo(format!("{}() espera un número, no {}", funcion, otro.tipo())),
    }
}

/// Número de color de la pantalla de texto (0 a 15), el mismo en los dos destinos.
fn codigo_color(valor: &Valor) -> u8 {
    match texto_de(valor).as_str() {
        "negro" => 0, "azul" => 1, "verde" => 2, "cian" => 3, "rojo" => 4,
        "morado" => 5, "marron" => 6, "gris" => 7, "gris_oscuro" => 8,
        "azul_claro" => 9, "verde_claro" => 10, "cian_claro" => 11,
        "rojo_claro" => 12, "rosa" => 13, "amarillo" => 14, "blanco" => 15,
        otro => fallo(format!(
            "color desconocido '{}'; usa negro, azul, verde, cian, rojo, morado, marron, gris, gris_oscuro, azul_claro, verde_claro, cian_claro, rojo_claro, rosa, amarillo o blanco",
            otro
        )),
    }
}

fn limpiar_pantalla() -> Valor {
    escribir(&Valor::Texto(Rc::new("\x1b[2J\x1b[H".to_string())))
}

fn escribir_en(fila: &Valor, columna: &Valor, texto: &Valor) -> Valor {
    let f = numero_de_maquina(fila, "escribir_en");
    let c = numero_de_maquina(columna, "escribir_en");
    if !(0.0..25.0).contains(&f) || !(0.0..80.0).contains(&c) {
        fallo("escribir_en(): la fila va de 0 a 24 y la columna de 0 a 79".to_string());
    }
    let destino = format!("\x1b[{};{}H", f as usize + 1, c as usize + 1);
    escribir(&Valor::Texto(Rc::new(destino + &texto_de(texto))))
}

fn poner_color(letra: &Valor, fondo: &Valor) -> Valor {
    // Orden de los colores de la pantalla de un PC → número de color de la terminal.
    const ANSI: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    let l = codigo_color(letra);
    let f = codigo_color(fondo);
    let codigo_letra = if l < 8 { 30 + ANSI[l as usize] } else { 90 + ANSI[l as usize - 8] };
    let codigo_fondo = if f < 8 { 40 + ANSI[f as usize] } else { 100 + ANSI[f as usize - 8] };
    escribir(&Valor::Texto(Rc::new(format!("\x1b[{};{}m", codigo_letra, codigo_fondo))))
}

/// Lee un byte de la entrada estándar (None si se acabó).
fn leer_byte_entrada() -> Option<u8> {
    use std::io::Read;
    let mut byte = [0u8; 1];
    match std::io::stdin().lock().read(&mut byte) {
        Ok(1) => Some(byte[0]),
        _ => None,
    }
}

fn leer_tecla() -> Valor {
    let primero = match leer_byte_entrada() {
        Some(byte) => byte,
        None => return Valor::Nada,
    };
    let nombre = match primero {
        b'\n' | b'\r' => "enter",
        0x7f | 0x08 => "borrar",
        b'\t' => "tab",
        0x1b => "escape",
        _ => "",
    };
    if !nombre.is_empty() {
        return Valor::Texto(Rc::new(nombre.to_string()));
    }
    let largo = match primero {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    };
    let mut bytes = vec![primero];
    while bytes.len() < largo {
        match leer_byte_entrada() {
            Some(byte) => bytes.push(byte),
            None => break,
        }
    }
    Valor::Texto(Rc::new(String::from_utf8_lossy(&bytes).into_owned()))
}

fn pedir_texto(aviso: &Valor) -> Valor {
    escribir(aviso);
    let mut linea = String::new();
    match std::io::stdin().read_line(&mut linea) {
        Ok(0) | Err(_) => Valor::Nada,
        Ok(_) => {
            while linea.ends_with('\n') || linea.ends_with('\r') {
                linea.pop();
            }
            Valor::Texto(Rc::new(linea))
        }
    }
}

fn apagar_maquina() -> Valor {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0)
}

fn reiniciar_maquina() -> Valor {
    apagar_maquina()
}

// @PURO_INICIO@
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

/// Límites de `rango(a, b)` para un `para` rápido: (desde, hasta, al revés), con desde <= hasta
/// y los mismos errores que rango().
#[inline]
fn rango_numerico(a: f64, b: f64, linea: usize) -> (i64, i64, bool) {
    en_linea(linea);
    let entero = |n: f64| -> i64 {
        if n.is_finite() && n.fract() == 0.0 {
            n as i64
        } else {
            fallo(format!("rango() espera números enteros, no {}", texto_de(&Valor::Numero(n))))
        }
    };
    let (x, y) = (entero(a), entero(b));
    match x.checked_sub(y).and_then(|d| d.checked_abs()) {
        Some(d) if d <= 10_000_000 => {}
        _ => fallo("rango() no puede crear más de 10 millones de números".to_string()),
    }
    if x <= y { (x, y, false) } else { (y, x, true) }
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
        (Valor::Funcion(x), Valor::Funcion(y)) => Valor::Logico(x.nombre == y.nombre),
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

/// `ordenar_por(lista, claves)`: ordena una lista de listas con claves por uno o varios
/// campos. `claves` es un texto o una lista de textos; "-campo" ordena de mayor a menor. Si dos
/// elementos empatan en todos los campos, quedan en el orden en que estaban.
fn ordenar_por(lista: &Valor, claves: &Valor) -> Valor {
    let elementos = match lista {
        Valor::Lista(elementos) => elementos,
        otro => fallo(format!("ordenar_por() espera una lista como primer argumento, no {}", otro.tipo())),
    };
    if let Valor::Funcion(_) = claves {
        return ordenar_con_funcion(elementos, claves);
    }
    let criterios: Vec<Valor> = match claves {
        Valor::Texto(_) => vec![claves.clone()],
        Valor::Lista(criterios) if !criterios.is_empty() => criterios.to_vec(),
        otro => fallo(format!(
            "ordenar_por() espera como segundo argumento un campo o una lista de campos, como [\"-edad\", \"nombre\"], no {}",
            otro.tipo()
        )),
    };
    let mut campos: Vec<(String, bool)> = Vec::new();
    for criterio in &criterios {
        match criterio {
            Valor::Texto(texto) => match texto.strip_prefix('-') {
                Some(campo) => campos.push((campo.to_string(), true)),
                None => campos.push((texto.to_string(), false)),
            },
            otro => fallo(format!("ordenar_por(): cada campo debe ser un texto, no {}", otro.tipo())),
        }
    }
    // Se sacan antes los valores de cada campo para dar un error claro si falta alguno.
    let mut filas: Vec<(Vec<Valor>, Valor)> = Vec::new();
    for (posicion, elemento) in elementos.iter().enumerate() {
        let mapa = match elemento {
            Valor::Mapa(mapa) => mapa,
            otro => fallo(format!(
                "ordenar_por() necesita una lista de listas con claves, pero el elemento {} es {}",
                posicion,
                otro.tipo()
            )),
        };
        let mut valores = Vec::new();
        for (campo, _) in &campos {
            match mapa.valores.get(campo) {
                Some(valor @ (Valor::Numero(_) | Valor::Texto(_))) => valores.push(valor.clone()),
                Some(otro) => fallo(format!(
                    "ordenar_por(): el campo '{}' del elemento {} es {}; solo se ordena por números o textos",
                    campo,
                    posicion,
                    otro.tipo()
                )),
                None => fallo(format!("ordenar_por(): el elemento {} no tiene el campo '{}'", posicion, campo)),
            }
        }
        filas.push((valores, elemento.clone()));
    }
    for (indice, (campo, _)) in campos.iter().enumerate() {
        let es_numero = filas.first().map(|(v, _)| matches!(v[indice], Valor::Numero(_)));
        if let Some(es_numero) = es_numero {
            if let Some((posicion, _)) = filas
                .iter()
                .enumerate()
                .find(|(_, (v, _))| matches!(v[indice], Valor::Numero(_)) != es_numero)
            {
                fallo(format!(
                    "ordenar_por(): el campo '{}' mezcla números y textos (mira el elemento {})",
                    campo, posicion
                ));
            }
        }
    }
    filas.sort_by(|(a, _), (b, _)| {
        for (indice, (_, al_reves)) in campos.iter().enumerate() {
            let orden = match (&a[indice], &b[indice]) {
                (Valor::Numero(x), Valor::Numero(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                (x, y) => texto_de(x).cmp(&texto_de(y)),
            };
            let orden = if *al_reves { orden.reverse() } else { orden };
            if orden != std::cmp::Ordering::Equal {
                return orden;
            }
        }
        std::cmp::Ordering::Equal
    });
    Valor::Lista(Rc::new(filas.into_iter().map(|(_, elemento)| elemento).collect()))
}

/// `ordenar_por(lista, f)`: ordena por lo que devuelve f para cada elemento (un número, un
/// texto o una lista de ellos para desempatar). Los empates quedan en su orden.
fn ordenar_con_funcion(elementos: &[Valor], funcion: &Valor) -> Valor {
    let funcion = funcion_de_un_parametro(funcion, "ordenar_por");
    let mut filas: Vec<(Vec<Valor>, Valor)> = Vec::new();
    for elemento in elementos {
        let clave = match llamar_funcion(funcion, vec![elemento.clone()]) {
            Valor::Lista(partes) => partes.to_vec(),
            otra => vec![otra],
        };
        for parte in &clave {
            if !matches!(parte, Valor::Numero(_) | Valor::Texto(_)) {
                fallo(format!(
                    "ordenar_por(): la función tiene que devolver un número, un texto o una lista de ellos, no {}",
                    parte.tipo()
                ));
            }
        }
        filas.push((clave, elemento.clone()));
    }
    let mut error = None;
    filas.sort_by(|(a, _), (b, _)| {
        for (x, y) in a.iter().zip(b.iter()) {
            let orden = match (x, y) {
                (Valor::Numero(x), Valor::Numero(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                (Valor::Texto(x), Valor::Texto(y)) => x.cmp(y),
                _ => {
                    error.get_or_insert("ordenar_por(): la función devuelve números para unos elementos y textos para otros");
                    std::cmp::Ordering::Equal
                }
            };
            if orden != std::cmp::Ordering::Equal {
                return orden;
            }
        }
        a.len().cmp(&b.len())
    });
    if let Some(mensaje) = error {
        fallo(mensaje.to_string());
    }
    Valor::Lista(Rc::new(filas.into_iter().map(|(_, elemento)| elemento).collect()))
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
    // Con límites enormes, `a - b` se saldría de los enteros: se comprueba sin desbordar.
    match a.checked_sub(b).and_then(|d| d.checked_abs()) {
        Some(d) if d <= 10_000_000 => {}
        _ => fallo("rango() no puede crear más de 10 millones de números".to_string()),
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

// @PURO_FIN@
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
            Valor::Funcion(funcion) => fallo(format!(
                "no se puede guardar la función '{}': guarda los datos y vuelve a elegir la función al leerlos",
                funcion.nombre
            )),
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
    /// Lo que atiende la ruta: una función del programa traducido (modo Rust) o una función
    /// compilada por el intérprete. Se comparte entre los hilos del servidor.
    manejador: std::sync::Arc<dyn Fn(Vec<String>) -> Valor + Send + Sync>,
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
const ICONO_WEB: &[u8] = &[]; //@ICONO_WEB@

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
    /// Archivos que llegaron en un formulario multipart/form-data.
    subidas: Vec<Subida>,
    /// Cuerpo tal cual llegó (para avisos de otros servicios, que mandan JSON) y cabeceras.
    cuerpo: String,
    cabeceras: Vec<(String, String)>,
}

struct Subida {
    campo: String,
    nombre: String,
    tipo: String,
    datos: Vec<u8>,
}

/// Archivo que una ruta devuelve en vez de una página: datos, tipo y si se descarga.
struct ArchivoRespuesta {
    datos: Vec<u8>,
    tipo: &'static str,
    descargar: Option<String>,
}

#[derive(Default)]
struct Respuesta {
    archivo: Option<ArchivoRespuesta>,
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
    /// Lo que manda enviar() en las pruebas, tal cual (para cuerpo_recibido()).
    static CUERPO_PRUEBA: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
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
        Valor::Funcion(_) => true,
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

