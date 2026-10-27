//! Revisión de tipos antes de ejecutar (código E025).
//!
//! Busca operaciones que fallarían SEGURO al llegar a ellas: restar un texto, comparar con `<`
//! algo que no es un número, poner un número como condición de `si`, recorrer un número con
//! `para`... Solo avisa cuando sabe con certeza el tipo de los dos lados (un número o un texto
//! escritos tal cual, una variable que solo ha recibido ese tipo, el resultado de `a_texto`...).
//! Si duda, calla: nunca da una falsa alarma. Los mensajes son los mismos que daría el programa
//! al fallar, para que se entiendan igual.
//!
//! También mira más allá de cada función:
//! - El tipo que devuelven las funciones propias se conoce cuando TODOS sus `devolver` dan el
//!   mismo tipo y la función no puede terminar sin `devolver` (si no, vale cualquier cosa).
//!   Se calcula para todas antes de revisar, así que da igual dónde se definan.
//! - Cada llamada a una función propia con argumentos de tipo seguro revisa también el cuerpo
//!   de la función con esos tipos (una vez por función y combinación de tipos); el aviso dice
//!   la línea de dentro de la función y desde qué llamada se llega.
//! - Las integradas más usadas (`TABLA_ARGUMENTOS`) avisan del argumento que no admiten, con el
//!   mismo mensaje que da al ejecutar. Una prueba recorre la tabla y exige que sea verdad.
//! - Un índice que no es un número entero en una lista o un texto.
//!
//! Todo esto sigue el mismo criterio que el resto: se avisa del código dentro de `si`, bucles
//! y `intentar` igual que del que va suelto, y solo cuando el fallo es seguro.

use std::collections::{HashMap, HashSet};

use crate::ast::{Expresion, OperadorBinario, OperadorUnario, ParteTexto, Programa, Sentencia};
use crate::error::ErrorTraduccion;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Tipo {
    Numero,
    Texto,
    Logico,
    Nada,
    Lista,
    /// Una lista que seguro tiene algún elemento (`[1, 2]`, el resultado de `agregar`). Una lista
    /// vacía admite un índice de texto (es también una lista con claves vacía); esta no.
    ListaLlena,
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
            Tipo::Lista | Tipo::ListaLlena => "lista",
            Tipo::Mapa | Tipo::Registro(_) => "lista con claves",
            Tipo::Desconocido => "valor",
        }
    }

    fn conocido(self) -> bool {
        self != Tipo::Desconocido
    }

    /// El tipo sin los detalles que solo usa el revisor (una lista llena es una lista; un
    /// tipo propio, una lista con claves).
    fn base(self) -> Tipo {
        match self {
            Tipo::ListaLlena => Tipo::Lista,
            Tipo::Registro(_) => Tipo::Mapa,
            otro => otro,
        }
    }

    /// El tipo que queda al juntar dos posibles (dos `devolver` de una misma función).
    fn unir(self, otro: Tipo) -> Tipo {
        if self == otro {
            self
        } else if self.conocido() && self.base() == otro.base() {
            self.base()
        } else {
            Tipo::Desconocido
        }
    }
}

/// Tipo seguro del resultado de algunas integradas (comprobado en el núcleo: siempre
/// devuelven eso o fallan).
fn tipo_de_integrada(nombre: &str) -> Tipo {
    match nombre {
        "a_texto" | "proteger_html" | "rellenar_plantilla" => Tipo::Texto,
        "a_numero" | "medir" | "contar" | "sumar_lista" => Tipo::Numero,
        "agregar" => Tipo::ListaLlena,
        "ordenar" | "ordenar_por" | "filtrar" | "transformar" | "rango" | "listar_claves" => Tipo::Lista,
        "quitar_clave" => Tipo::Mapa,
        "tiene_clave" | "incluye" => Tipo::Logico,
        _ => Tipo::Desconocido,
    }
}

/// Una comprobación de un argumento de una integrada: qué tipos admite y el mensaje que da el
/// núcleo (`src/nucleo/base.rs`) cuando no es de esos; `{}` se cambia por el tipo recibido.
pub struct ReglaArgumento {
    /// Posición del argumento (0 es el primero).
    pub posicion: usize,
    pub permitidos: &'static [&'static str],
    pub mensaje: &'static str,
    /// La regla solo vale si el argumento de esa posición es de ese tipo.
    pub cuando: Option<(usize, &'static str)>,
}

/// Lo que admite cada argumento de las integradas más usadas, en el mismo orden en que las
/// comprueba el núcleo (si dos argumentos fallan, el aviso es el del primero que comprobaría).
/// Los tipos se escriben con los nombres de `Valor::tipo`. Una prueba (tests/plan_lenguaje.rs)
/// recorre la tabla y exige que cada aviso salga igual al ejecutar: así no puede mentir.
/// No están `filtrar`, `transformar` ni `rango`: comprueban primero otras cosas (la función, el
/// valor de los números) y no se podría asegurar qué mensaje saldría.
pub const TABLA_ARGUMENTOS: &[(&str, &[ReglaArgumento])] = &[
    (
        "a_numero",
        &[ReglaArgumento { posicion: 0, permitidos: &["texto", "número"], mensaje: "a_numero() espera un texto, no {}", cuando: None }],
    ),
    (
        "medir",
        &[ReglaArgumento {
            posicion: 0,
            permitidos: &["lista", "lista con claves", "texto"],
            mensaje: "medir() espera una lista o un texto, no {}",
            cuando: None,
        }],
    ),
    (
        "incluye",
        &[
            ReglaArgumento {
                posicion: 0,
                permitidos: &["lista", "lista con claves", "texto"],
                mensaje: "incluye() espera una lista o un texto, no {}",
                cuando: None,
            },
            ReglaArgumento { posicion: 1, permitidos: &["texto"], mensaje: "incluye() en un texto busca otro texto, no {}", cuando: Some((0, "texto")) },
        ],
    ),
    ("ordenar", &[ReglaArgumento { posicion: 0, permitidos: &["lista"], mensaje: "ordenar() espera una lista, no {}", cuando: None }]),
    (
        "ordenar_por",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista"], mensaje: "ordenar_por() espera una lista como primer argumento, no {}", cuando: None }],
    ),
    (
        "invertir",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista", "texto"], mensaje: "invertir() espera una lista o un texto, no {}", cuando: None }],
    ),
    (
        "cortar",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista", "texto"], mensaje: "cortar() espera una lista o un texto, no {}", cuando: None }],
    ),
    ("sumar_lista", &[ReglaArgumento { posicion: 0, permitidos: &["lista"], mensaje: "sumar_lista() espera una lista, no {}", cuando: None }]),
    (
        "agregar",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista"], mensaje: "agregar() espera una lista como primer argumento, no {}", cuando: None }],
    ),
    (
        "tiene_clave",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista con claves"], mensaje: "tiene_clave() espera una lista con claves, no {}", cuando: None }],
    ),
    (
        "quitar_clave",
        &[ReglaArgumento { posicion: 0, permitidos: &["lista con claves"], mensaje: "quitar_clave() espera una lista con claves, no {}", cuando: None }],
    ),
    (
        "rellenar_plantilla",
        &[ReglaArgumento { posicion: 0, permitidos: &["texto"], mensaje: "rellenar_plantilla() espera un texto, no {}", cuando: None }],
    ),
];

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

/// ¿Es la función que crea el analizador para un `tipo Nombre { ... }`?
pub(crate) fn es_tipo_propio(sentencia: &Sentencia) -> bool {
    tipo_propio_de(sentencia).is_some()
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

/// Una función propia: sus parámetros, su cuerpo y el tipo que seguro devuelve.
#[derive(Clone, Copy)]
struct Funcion<'a> {
    parametros: &'a [String],
    cuerpo: &'a [Sentencia],
    archivo: Option<&'a str>,
    devuelve: Tipo,
}

/// Una llamada a una función propia con tipos conocidos, por revisar con ellos.
struct Pendiente<'a> {
    funcion: &'a str,
    tipos: Vec<Tipo>,
    /// Cómo se llegó: `doble("a") de la línea 4`.
    llamada: String,
}

/// Tope de cuerpos revisados por tipos de llamada, para que un programa enorme no se haga lento.
const TOPE_REVISIONES: usize = 2000;

struct Revisor<'a> {
    tipos_propios: Vec<TipoPropio>,
    funciones: HashMap<&'a str, Funcion<'a>>,
    errores: Vec<ErrorTraduccion>,
    /// Archivo de la definición que se revisa (si viene de un `usar`).
    archivo: Option<String>,
    /// Calculando lo que devuelven las funciones: no se avisa ni se apuntan llamadas.
    silencioso: bool,
    /// Tipos de los `devolver` de la función que se calcula.
    retornos: Vec<Tipo>,
    pendientes: Vec<Pendiente<'a>>,
    vistas: HashSet<(&'a str, Vec<Tipo>)>,
    /// Llamada que se está siguiendo al revisar un cuerpo con los tipos de sus argumentos.
    contexto: Option<String>,
    /// Avisos ya dados (archivo, línea, mensaje), para no repetir el de dentro de una función.
    avisados: HashSet<(Option<String>, usize, String)>,
    /// Dentro de un bloque que puede no ejecutarse (o que atrapa sus fallos): no se avisa.
    callado: usize,
    /// Avisos dudosos (A008) que solo pide `deimon comprobar --estricto`.
    avisos: Vec<ErrorTraduccion>,
}

/// Revisa los tipos del programa ya resuelto (con sus `usar`).
pub fn revisar(programa: &Programa) -> Vec<ErrorTraduccion> {
    revisar_todo(programa).0
}

/// Como `revisar`, y además los avisos (A008) de `deimon comprobar --estricto`.
pub fn revisar_con_avisos(programa: &Programa) -> (Vec<ErrorTraduccion>, Vec<ErrorTraduccion>) {
    revisar_todo(programa)
}

fn revisar_todo(programa: &Programa) -> (Vec<ErrorTraduccion>, Vec<ErrorTraduccion>) {
    let tipos_propios = programa.sentencias.iter().filter_map(tipo_propio_de).collect();
    let de_extensiones: HashSet<&str> = programa.extensiones.iter().flat_map(|e| e.funciones.iter().map(|f| f.nombre.as_str())).collect();
    let mut funciones: HashMap<&str, Funcion> = HashMap::new();
    for sentencia in &programa.sentencias {
        if let Sentencia::Funcion { nombre, parametros, cuerpo, archivo, .. } = sentencia {
            // Las integradas y las de módulos mandan sobre una propia del mismo nombre, y los
            // tipos propios ya tienen su tratamiento.
            if tipo_propio_de(sentencia).is_some() || crate::integradas::buscar(nombre).is_some() || de_extensiones.contains(nombre.as_str()) {
                continue;
            }
            let funcion = Funcion { parametros, cuerpo, archivo: archivo.as_deref(), devuelve: Tipo::Desconocido };
            funciones.entry(nombre.as_str()).or_insert(funcion);
        }
    }
    let mut revisor = Revisor {
        tipos_propios,
        funciones,
        errores: Vec::new(),
        archivo: None,
        silencioso: false,
        retornos: Vec::new(),
        pendientes: Vec::new(),
        vistas: HashSet::new(),
        contexto: None,
        avisados: HashSet::new(),
        callado: 0,
        avisos: Vec::new(),
    };
    revisor.calcular_lo_que_devuelven();
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
    revisor.revisar_llamadas();
    (revisor.errores, revisor.avisos)
}

/// ¿Termina siempre con un `devolver` (sin poder llegar al final de la lista)?
fn siempre_devuelve(lista: &[Sentencia]) -> bool {
    lista.iter().any(|sentencia| match sentencia {
        Sentencia::Devolver { .. } => true,
        Sentencia::Si { entonces, sino, .. } => siempre_devuelve(entonces) && siempre_devuelve(sino),
        Sentencia::Intentar { cuerpo, captura, .. } => siempre_devuelve(cuerpo) && siempre_devuelve(captura),
        _ => false,
    })
}

/// ¿Puede esta sentencia (o algo dentro de ella) terminar la función con un `devolver` o un
/// `provocar_error`?
fn puede_devolver(sentencia: &Sentencia) -> bool {
    match sentencia {
        Sentencia::Devolver { .. } => true,
        Sentencia::Expresion(Expresion::Llamada { nombre, .. }) => nombre == "provocar_error",
        Sentencia::Si { entonces, sino, .. } => entonces.iter().chain(sino).any(puede_devolver),
        Sentencia::Mientras { cuerpo, .. } | Sentencia::Para { cuerpo, .. } => cuerpo.iter().any(puede_devolver),
        Sentencia::Intentar { cuerpo, captura, .. } => cuerpo.iter().chain(captura).any(puede_devolver),
        _ => false,
    }
}

/// Cómo se ve una llamada en el aviso: `doble("a")`.
fn texto_de_llamada(nombre: &str, argumentos: &[Expresion]) -> String {
    let partes: Vec<String> = argumentos
        .iter()
        .map(|argumento| match argumento {
            Expresion::Numero(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64),
            Expresion::Numero(n) => format!("{}", n),
            Expresion::EnteroGrande(cifras) => cifras.clone(),
            Expresion::Texto(texto) if texto.chars().count() <= 20 => format!("\"{}\"", texto),
            Expresion::Texto(texto) => format!("\"{}...\"", texto.chars().take(20).collect::<String>()),
            Expresion::Logico(true) => "verdadero".to_string(),
            Expresion::Logico(false) => "falso".to_string(),
            Expresion::Nada => "nada".to_string(),
            Expresion::Variable { nombre, .. } => nombre.clone(),
            Expresion::Lista(_) => "[...]".to_string(),
            Expresion::Mapa(_) => "{...}".to_string(),
            _ => "...".to_string(),
        })
        .collect();
    format!("{}({})", nombre, partes.join(", "))
}

impl<'a> Revisor<'a> {
    /// ¿Hay que callar este aviso? Mientras se calcula lo que devuelven las funciones, sí; y
    /// dentro de una función revisada con los tipos de una llamada, si ya se dio el mismo.
    fn callar(&mut self, linea: usize, mensaje: &str) -> bool {
        if self.silencioso || self.callado > 0 {
            return true;
        }
        let nuevo = self.avisados.insert((self.archivo.clone(), linea, mensaje.to_string()));
        !nuevo && self.contexto.is_some()
    }

    /// El final de todo aviso: «se ve sin ejecutar el programa», con la llamada que lleva hasta
    /// el fallo cuando se revisa el cuerpo de una función con los tipos de sus argumentos.
    fn cierre(&self) -> String {
        match &self.contexto {
            Some(llamada) => format!("con {}; se ve sin ejecutar el programa", llamada),
            None => "se ve sin ejecutar el programa".to_string(),
        }
    }

    fn avisar(&mut self, linea: usize, mensaje: String, sugerencia: &str) {
        if self.callar(linea, &mensaje) {
            return;
        }
        let mut error = ErrorTraduccion::nuevo(linea, format!("{} ({})", mensaje, self.cierre()))
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
        if self.callar(linea, &mensaje) {
            return;
        }
        let mut error = ErrorTraduccion::nuevo(linea, format!("{} ({})", mensaje, self.cierre()))
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
                    tiene = tiene.base();
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

    /// Calcula lo que devuelve cada función propia (ver `Funcion::devuelve`), mirando sus
    /// cuerpos sin avisar de nada. Se repite hasta que no cambie nada, porque una función puede
    /// devolver lo que devuelve otra; lo que no se sabe sigue siendo `Desconocido`.
    fn calcular_lo_que_devuelven(&mut self) {
        let mut nombres: Vec<&'a str> = self.funciones.keys().copied().collect();
        nombres.sort_unstable();
        self.silencioso = true;
        for _ in 0..=nombres.len() {
            let mut cambio = false;
            for nombre in &nombres {
                let funcion = self.funciones[nombre];
                self.archivo = funcion.archivo.map(String::from);
                self.retornos.clear();
                let mut ambito: Ambito = funcion.parametros.iter().map(|p| (p.clone(), Tipo::Desconocido)).collect();
                self.sentencias(funcion.cuerpo, &mut ambito);
                if !siempre_devuelve(funcion.cuerpo) {
                    self.retornos.push(Tipo::Nada);
                }
                let mut tipos = self.retornos.iter().copied();
                let primero = tipos.next().unwrap_or(Tipo::Desconocido);
                let devuelve = tipos.fold(primero, Tipo::unir);
                if devuelve != funcion.devuelve {
                    self.funciones.get_mut(nombre).expect("la función existe").devuelve = devuelve;
                    cambio = true;
                }
            }
            if !cambio {
                break;
            }
        }
        self.silencioso = false;
        self.retornos.clear();
    }

    /// Llamada a una función propia: si se sabe el tipo de algún argumento, se apunta para
    /// revisar su cuerpo con esos tipos (una vez por combinación).
    fn apuntar_llamada(&mut self, nombre: &'a str, funcion: &Funcion<'a>, tipos: Vec<Tipo>, argumentos: &[Expresion], linea: usize) {
        if self.silencioso || self.callado > 0 || tipos.len() != funcion.parametros.len() || !tipos.iter().any(|t| t.conocido()) {
            return;
        }
        if !self.vistas.insert((nombre, tipos.clone())) {
            return;
        }
        // Dentro de una función revisada así, el aviso sigue señalando la llamada de origen.
        let llamada = match &self.contexto {
            Some(origen) => origen.clone(),
            None => {
                let lugar = match &self.archivo {
                    Some(archivo) => format!("la línea {} de {}", linea, archivo),
                    None => format!("la línea {}", linea),
                };
                format!("{} de {}", texto_de_llamada(nombre, argumentos), lugar)
            }
        };
        self.pendientes.push(Pendiente { funcion: nombre, tipos, llamada });
    }

    /// Revisa el cuerpo de cada función propia llamada con tipos conocidos, con esos tipos en
    /// sus parámetros. Una combinación ya vista no se repite, así que la recursión termina.
    fn revisar_llamadas(&mut self) {
        let mut hechas = 0;
        while hechas < self.pendientes.len() && hechas < TOPE_REVISIONES {
            let (funcion, tipos, llamada) = {
                let pendiente = &self.pendientes[hechas];
                (self.funciones[pendiente.funcion], pendiente.tipos.clone(), pendiente.llamada.clone())
            };
            hechas += 1;
            self.archivo = funcion.archivo.map(String::from);
            self.contexto = Some(llamada);
            let mut ambito: Ambito = funcion.parametros.iter().cloned().zip(tipos).collect();
            self.sentencias(funcion.cuerpo, &mut ambito);
        }
        self.contexto = None;
    }

    /// Argumento de una integrada que seguro no admite (ver `TABLA_ARGUMENTOS`).
    fn revisar_integrada(&mut self, nombre: &str, tipos: &[Tipo], linea: usize) {
        let Some((_, reglas)) = TABLA_ARGUMENTOS.iter().find(|(integrada, _)| *integrada == nombre) else { return };
        for regla in reglas.iter() {
            if let Some((posicion, tipo)) = regla.cuando {
                if tipos.get(posicion).map(|t| t.base().nombre()) != Some(tipo) {
                    continue;
                }
            }
            let Some(tipo) = tipos.get(regla.posicion).copied() else { return };
            // Si no se sabe qué es, puede fallar antes otra cosa con otro mensaje: se calla.
            if !tipo.conocido() {
                return;
            }
            if !regla.permitidos.contains(&tipo.base().nombre()) {
                self.avisar(
                    linea,
                    regla.mensaje.replace("{}", tipo.nombre()),
                    &format!("El argumento {} de {}() admite: {}", regla.posicion + 1, nombre, regla.permitidos.join(", ")),
                );
                return;
            }
        }
    }

    fn sentencias(&mut self, lista: &[Sentencia], ambito: &mut Ambito) {
        for sentencia in lista {
            self.sentencia(sentencia, ambito);
            // Al revisar una función con los tipos de una llamada solo cuenta lo que se ejecuta
            // seguro: lo que sigue a un `devolver`, a un bloque que pueda salir antes o a un
            // `provocar_error` no se sabe si llega a ejecutarse.
            if self.contexto.is_some() && self.callado == 0 {
                if puede_devolver(sentencia) {
                    break;
                }
            }
        }
    }

    /// Revisa un bloque que puede no ejecutarse: al revisar con los tipos de una llamada no se
    /// avisa de lo de dentro (los tipos se siguen calculando).
    fn bloque_dudoso(&mut self, lista: &[Sentencia], ambito: &mut Ambito, siempre: bool) {
        let callar = siempre || self.contexto.is_some();
        if callar {
            self.callado += 1;
        }
        self.sentencias(lista, ambito);
        if callar {
            self.callado -= 1;
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
                self.bloque_dudoso(entonces, &mut por_si, false);
                self.bloque_dudoso(sino, &mut por_sino, false);
                juntar(ambito, &por_si, &por_sino);
            }
            Sentencia::Mientras { condicion, cuerpo, linea } => {
                olvidar_asignadas(cuerpo, ambito);
                let tipo = self.expresion(condicion, ambito);
                self.condicion(tipo, *linea);
                let mut dentro = ambito.clone();
                self.bloque_dudoso(cuerpo, &mut dentro, false);
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
                self.bloque_dudoso(cuerpo, &mut dentro, false);
            }
            Sentencia::Intentar { cuerpo, nombre, captura, .. } => {
                olvidar_asignadas(cuerpo, ambito);
                olvidar_asignadas(captura, ambito);
                // Lo que falla dentro de `intentar` lo atrapa `si falla`: no es un fallo seguro.
                let mut dentro = ambito.clone();
                self.bloque_dudoso(cuerpo, &mut dentro, true);
                let mut si_falla = ambito.clone();
                si_falla.insert(nombre.clone(), Tipo::Mapa);
                self.bloque_dudoso(captura, &mut si_falla, false);
            }
            Sentencia::Devolver { valor, .. } => {
                let tipo = match valor {
                    Some(valor) => self.expresion(valor, ambito),
                    None => Tipo::Nada,
                };
                if self.silencioso {
                    self.retornos.push(tipo);
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
            Expresion::TextoConValores { partes, .. } => {
                for parte in partes {
                    if let ParteTexto::Valor(valor) = parte {
                        self.expresion(valor, ambito);
                    }
                }
                Tipo::Texto
            }
            Expresion::Logico(_) => Tipo::Logico,
            Expresion::Nada => Tipo::Nada,
            Expresion::Lista(elementos) => {
                for elemento in elementos {
                    self.expresion(elemento, ambito);
                }
                if elementos.is_empty() { Tipo::Lista } else { Tipo::ListaLlena }
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
                let tipos: Vec<Tipo> = argumentos.iter().map(|argumento| self.expresion(argumento, ambito)).collect();
                if let Some((&clave, &funcion)) = self.funciones.get_key_value(nombre.as_str()) {
                    self.apuntar_llamada(clave, &funcion, tipos, argumentos, *linea);
                    return funcion.devuelve;
                }
                self.revisar_integrada(nombre, &tipos, *linea);
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
                let tipo_indice = self.expresion(indice, ambito);
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
                // El núcleo (`indice`) solo admite un número entero en una lista o un texto; una
                // lista vacía con un índice de texto es la excepción (es una lista con claves vacía).
                let falla = match (tipo.base(), tipo_indice.base()) {
                    (Tipo::Lista | Tipo::Texto, Tipo::Logico | Tipo::Nada | Tipo::Lista | Tipo::Mapa) => true,
                    (Tipo::Texto, Tipo::Texto) => true,
                    (Tipo::Lista, Tipo::Texto) => tipo == Tipo::ListaLlena,
                    _ => false,
                };
                if falla {
                    self.avisar(
                        *linea,
                        format!("el índice debe ser un número entero, no {}", tipo_indice.nombre()),
                        "Con corchetes se elige un elemento por su posición (0, 1, 2...); para buscar por nombre usa una lista con claves: {\"nombre\": ...}",
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

    /// A008: `==` o `!=` entre dos tipos distintos seguros. Se calla con `nada` (comparar con
    /// `nada` es normal), entre lista y lista con claves (una lista vacía es las dos) y al
    /// revisar una función con los tipos de una llamada concreta.
    fn comparacion_de_tipos_distintos(&mut self, operador: &OperadorBinario, a: Tipo, b: Tipo, linea: usize) {
        let lista_y_mapa = matches!((a, b), (Tipo::Lista, Tipo::Mapa) | (Tipo::Mapa, Tipo::Lista));
        if self.silencioso || self.callado > 0 || self.contexto.is_some() {
            return;
        }
        if !(a.conocido() && b.conocido()) || a == b || a == Tipo::Nada || b == Tipo::Nada || lista_y_mapa {
            return;
        }
        let signo = if *operador == OperadorBinario::Igual { "==" } else { "!=" };
        let mensaje = format!(
            "se compara {} con {} usando {}: son tipos distintos y el programa fallaría al llegar aquí con «no se pueden comparar {} y {}»",
            a.nombre(),
            b.nombre(),
            signo,
            a.nombre(),
            b.nombre()
        );
        if !self.avisados.insert((self.archivo.clone(), linea, mensaje.clone())) {
            return;
        }
        let mut aviso = ErrorTraduccion::nuevo(linea, mensaje)
            .con_codigo("A008")
            .con_sugerencia("Convierte uno de los dos para que sean del mismo tipo, por ejemplo a_numero(texto) o a_texto(numero)");
        if let Some(archivo) = &self.archivo {
            aviso = aviso.con_archivo(archivo);
        }
        self.avisos.push(aviso);
    }

    fn binaria(&mut self, operador: &OperadorBinario, a: Tipo, b: Tipo, linea: usize) -> Tipo {
        use OperadorBinario::*;
        // Para las operaciones, un tipo propio es una lista con claves más y una lista llena,
        // una lista.
        let (a, b) = (a.base(), b.base());
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
            Igual | Distinto => {
                self.comparacion_de_tipos_distintos(operador, a, b, linea);
                Tipo::Logico
            }
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
