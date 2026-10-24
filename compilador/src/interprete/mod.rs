//! Intérprete propio de Deimon: ejecuta el programa al instante, sin pasar por `rustc`.
//!
//! Reutiliza el lexer, el parser y el AST, y el mismo núcleo (`nucleo/`) que se pega en
//! los programas traducidos a Rust: así los dos motores comparten `Valor`, operaciones,
//! formato de números y mensajes de error.

mod codigo;
mod compilador;

pub use codigo::Codigo;

mod motor {
    #![allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code, unused_imports)]

    // El mismo núcleo que el modo Rust, con sus marcas ya puestas (perfil equilibrado e icono
    // de la web); lo prepara `build.rs` a partir de `nucleo/base.rs`.
    include!(concat!(env!("OUT_DIR"), "/nucleo_interprete.rs"));
    include!("../plantilla_lectura.rs");
    include!(concat!(env!("OUT_DIR"), "/servidor_interprete.rs"));

    pub fn elegir_perfil(nombre: &'static str) {
        let _ = PERFIL_ELEGIDO.set(nombre);
    }

    /// Lee un JSON con el mismo lector del módulo `json` (`desde_json`).
    pub fn leer_json(texto: &str) -> Result<super::Json, String> {
        fn convertir(valor: &Valor) -> super::Json {
            match valor {
                Valor::Nada => super::Json::Nulo,
                Valor::Logico(b) => super::Json::Logico(*b),
                Valor::Numero(n) => super::Json::Numero(*n),
                Valor::Texto(t) => super::Json::Texto(t.to_string()),
                Valor::Lista(elementos) => super::Json::Lista(elementos.iter().map(convertir).collect()),
                Valor::Mapa(mapa) => super::Json::Objeto(
                    mapa.claves
                        .iter()
                        .map(|clave| (clave.clone(), mapa.valores.get(clave).map(convertir).unwrap_or(super::Json::Nulo)))
                        .collect(),
                ),
            }
        }
        ext_desde_json(vec![Valor::Texto(Rc::new(texto.to_string()))]).map(|valor| convertir(&valor))
    }
    include!("vm.rs");
    include!("biblioteca.rs");
    include!("integradas.rs");
}

use crate::ast::Programa;

/// Compila el programa. `Err` lleva el motivo por el que el intérprete aún no puede con él
/// (entonces se usa el modo Rust).
pub fn compilar(programa: &Programa) -> Result<Codigo, String> {
    compilador::compilar(programa)
}

/// `deimon probar` con el intérprete: corre las pruebas (no el programa principal) y escribe
/// sus resultados igual que el programa de pruebas del modo Rust.
pub fn ejecutar_pruebas(codigo: &Codigo) {
    motor::ejecutar_pruebas_codigo(&std::sync::Arc::new(codigo.clone()))
}

/// Un valor JSON leído con `leer_json`.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Nulo,
    Logico(bool),
    Numero(f64),
    Texto(String),
    Lista(Vec<Json>),
    /// Las claves en el orden en que venían.
    Objeto(Vec<(String, Json)>),
}

impl Json {
    pub fn campo(&self, nombre: &str) -> Option<&Json> {
        match self {
            Json::Objeto(pares) => pares.iter().find(|(clave, _)| clave == nombre).map(|(_, valor)| valor),
            _ => None,
        }
    }

    pub fn como_texto(&self) -> Option<&str> {
        match self {
            Json::Texto(texto) => Some(texto),
            _ => None,
        }
    }

    /// El valor escrito otra vez como JSON.
    pub fn escribir(&self) -> String {
        match self {
            Json::Nulo => "null".to_string(),
            Json::Logico(b) => b.to_string(),
            Json::Numero(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64),
            Json::Numero(n) => n.to_string(),
            Json::Texto(t) => crate::json::texto(t),
            Json::Lista(elementos) => format!("[{}]", elementos.iter().map(Json::escribir).collect::<Vec<_>>().join(",")),
            Json::Objeto(pares) => format!(
                "{{{}}}",
                pares.iter().map(|(c, v)| format!("{}:{}", crate::json::texto(c), v.escribir())).collect::<Vec<_>>().join(",")
            ),
        }
    }
}

/// Lee un texto JSON con el lector del módulo `json` de la biblioteca (una sola versión).
pub fn leer_json(texto: &str) -> Result<Json, String> {
    motor::leer_json(texto)
}

/// Perfil de recursos del intérprete (`pequeño`, `equilibrado` o `servidor`), como
/// `--perfil` en el modo Rust. Se elige una vez, antes de ejecutar.
pub fn elegir_perfil(perfil: crate::perfil::Perfil) {
    motor::elegir_perfil(perfil.nombre())
}

/// Ejecuta el programa compilado en la máquina virtual. Los errores en tiempo de ejecución
/// escriben el mismo mensaje que el modo Rust y terminan el proceso con el código 1.
pub fn ejecutar(codigo: &Codigo) {
    // `DEIMON_VER_CODIGO=1` escribe el código intermedio (para estudiar la velocidad).
    if std::env::var_os("DEIMON_VER_CODIGO").is_some() {
        for (posicion, op) in codigo.ops.iter().enumerate() {
            eprintln!("{:5} {:?}", posicion, op);
        }
    }
    // Las rutas web se atienden en otros hilos, que comparten el código compilado.
    motor::ejecutar_codigo(&std::sync::Arc::new(codigo.clone()))
}
