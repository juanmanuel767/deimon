use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Arreglo, ErrorTraduccion};
use crate::integradas;

/// Icono de Deimon para la pestaña del navegador (48×48), servido en /favicon.ico.
const ICONO_WEB: &[u8] = include_bytes!("../recursos/favicon.png");

/// Código base que se pega en cada programa traducido. Vive en `nucleo/` y es el mismo que
/// usa el intérprete (`interprete/`), para que los dos motores se comporten igual.
/// Marcas que sustituye `Generador::nuevo`: `@PERFIL@` y `&[]; //@ICONO_WEB@`.
const RUNTIME: &str = concat!(
    "\n",
    include_str!("nucleo/base.rs"),
    include_str!("plantilla_lectura.rs"),
    "\n",
    include_str!("nucleo/servidor.rs")
);

/// Código de apoyo del destino `pc-sin-sistema` (el programa es el sistema operativo).
const RUNTIME_SIN_SISTEMA: &str = include_str!("nucleo/sin_sistema.rs");

/// Módulos con código Rust que también funcionan sin sistema (los escritos solo en Deimon
/// valen todos).
const MODULOS_SIN_SISTEMA: &[&str] = &["texto"];

/// Integradas fuera de las categorías básicas que el destino `pc-sin-sistema` también tiene.
const INTEGRADAS_SIN_SISTEMA: &[&str] = &["proteger_html", "listar_claves", "comprobar", "comprobar_igual"];

fn permitida_sin_sistema(integrada: &integradas::Integrada) -> bool {
    matches!(integrada.categoria, "básicas" | "conversiones" | "listas y textos" | "máquina")
        || INTEGRADAS_SIN_SISTEMA.contains(&integrada.nombre)
}

/// Principio del programa para `pc-sin-sistema`: su código de apoyo y las partes de
/// `nucleo/base.rs` que no usan el sistema (entre `@PURO_INICIO@` y `@PURO_FIN@`), para que
/// los valores se comporten igual que en los demás destinos.
fn preludio_sin_sistema() -> String {
    let mut salida = String::from(
        "#![no_std]\n#![no_main]\n#![allow(dead_code, unused_variables, unused_mut, unused_assignments, unreachable_code, unused_imports)]\n",
    );
    salida.push_str(RUNTIME_SIN_SISTEMA);
    let mut resto = include_str!("nucleo/base.rs");
    while let Some(inicio) = resto.find("// @PURO_INICIO@") {
        let tras_marca = &resto[inicio..];
        let desde = tras_marca.find('\n').map(|i| i + 1).unwrap_or(tras_marca.len());
        let fin = tras_marca.find("// @PURO_FIN@").unwrap_or(tras_marca.len());
        salida.push('\n');
        salida.push_str(&tras_marca[desde..fin]);
        resto = &tras_marca[fin..];
        resto = &resto[resto.find('\n').map(|i| i + 1).unwrap_or(resto.len())..];
    }
    salida.push('\n');
    salida
}

fn error_sin_sistema(linea: usize, columna: usize, que: String) -> ErrorTraduccion {
    ErrorTraduccion::nuevo(linea, format!("{} no funciona en el destino pc-sin-sistema", que))
        .con_codigo("E026")
        .en_columna(columna)
        .con_sugerencia("Sin sistema no hay disco, red ni navegador: usa listas y listas con claves en memoria, la pantalla (mostrar, escribir_en) y el teclado (leer_tecla, pedir_texto)")
}

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
    /// Destino `pc-sin-sistema`: el programa arranca solo en un PC (sin Linux debajo).
    SinSistema,
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
    if modo == Modo::SinSistema {
        generador.salida = preludio_sin_sistema();
        for extension in &programa.extensiones {
            if !extension.codigo.trim().is_empty() && !MODULOS_SIN_SISTEMA.contains(&extension.nombre.as_str()) {
                let error = error_sin_sistema(
                    extension.linea,
                    extension.columna,
                    format!("el módulo '{}' (tiene código Rust que necesita el sistema)", extension.nombre),
                );
                generador.errores.push(marcar_archivo(error, &extension.archivo));
            }
        }
    }
    generador.recoger_funciones(&programa.sentencias);
    generador.recoger_extensiones(&programa.extensiones);
    generador.recoger_rutas(&programa.sentencias);
    if modo == Modo::SinSistema {
        let rutas = generador.rutas.clone();
        if let Some(ruta) = rutas.first() {
            let linea = programa
                .sentencias
                .iter()
                .find_map(|sentencia| match sentencia {
                    Sentencia::Ruta { camino, linea, columna, .. } if *camino == ruta.camino => Some((*linea, *columna)),
                    _ => None,
                })
                .unwrap_or((1, 1));
            let error = error_sin_sistema(linea.0, linea.1, format!("la ruta web \"{}\"", ruta.camino));
            generador.errores.push(marcar_archivo(error, &ruta.archivo));
            generador.rutas.clear();
        }
    }
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

#[derive(Clone, Copy, Default)]
struct ControlesIntento {
    romper: bool,
    continuar: bool,
    devolver: bool,
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
    /// Bloques `intentar` abiertos y los bucles que había al abrir cada uno.
    intentos: usize,
    bases_intentar: Vec<usize>,
    controles_intentar: Vec<ControlesIntento>,
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
                .replace("&[]; //@ICONO_WEB@", &format!("&[{}];", icono_web)),
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
            intentos: 0,
            bases_intentar: Vec::new(),
            controles_intentar: Vec::new(),
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
                "rutas.push(Ruta {{ camino: {:?}.to_string(), manejador: std::sync::Arc::new(ruta_{}) }});",
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
            parametros_rust.push(format!("mut {}: Valor", nombre_variable(parametro)));
        }

        self.escribir_linea_vacia();
        self.escribir(&format!(
            "fn {}({}) -> Valor {{",
            nombre_funcion(nombre),
            parametros_rust.join(", ")
        ));
        self.nivel += 1;
        self.escribir("let _llamada = entrar_llamada();");
        let numerica = self.numericas.get(nombre) == Some(&parametros.len());
        if numerica {
            // Con números, la versión rápida; con cualquier otra cosa, el cuerpo de siempre.
            let mut argumentos: Vec<String> = (0..parametros.len()).map(|i| format!("*n{}", i)).collect();
            argumentos.push("profundidad_actual()".to_string());
            let llamada = format!("{}({})", crate::numerico::nombre_numerico(nombre), argumentos.join(", "));
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

    /// Funciones del programa en el orden de su número (el de `valor_funcion`).
    fn funciones_numeradas(&self) -> Vec<(String, usize)> {
        let mut nombres: Vec<(String, usize)> = self
            .funciones
            .iter()
            .map(|(nombre, parametros)| (nombre.clone(), parametros.len()))
            .collect();
        nombres.sort();
        nombres
    }

    /// `doble` escrito sin paréntesis: el valor de esa función del programa.
    fn valor_de_funcion(&self, nombre: &str) -> Option<String> {
        let numeradas = self.funciones_numeradas();
        let numero = numeradas.iter().position(|(otra, _)| otra == nombre)?;
        Some(format!("valor_funcion({:?}, {}, {})", nombre, numeradas[numero].1, numero))
    }

    /// Tabla que usa `llamar_funcion` para ejecutar una función guardada en un valor.
    fn generar_tabla_funciones(&mut self) {
        self.escribir_linea_vacia();
        self.escribir("fn despachar_tabla(numero: usize, argumentos: Vec<Valor>) -> Valor {");
        self.nivel += 1;
        self.escribir("let mut argumentos = argumentos.into_iter();");
        self.escribir("let mut siguiente = move || argumentos.next().unwrap_or(Valor::Nada);");
        self.escribir("match numero {");
        self.nivel += 1;
        for (numero, (nombre, parametros)) in self.funciones_numeradas().into_iter().enumerate() {
            let argumentos = vec!["siguiente()"; parametros].join(", ");
            self.escribir(&format!("{} => {}({}),", numero, nombre_funcion(&nombre), argumentos));
        }
        self.escribir("_ => Valor::Nada,");
        self.nivel -= 1;
        self.escribir("}");
        self.nivel -= 1;
        self.escribir("}");
    }

    fn generar_principal(&mut self, sentencias: &[Sentencia]) -> Result<(), ErrorTraduccion> {
        self.generar_tabla_funciones();
        self.escribir_linea_vacia();
        if self.modo == Modo::Pruebas {
            // Variante para `deimon probar`: el programa normal se traduce (para validarlo) pero
            // no se ejecuta; `main` solo corre las pruebas.
            self.escribir("fn programa_principal() {");
            self.generar_cuerpo_principal(sentencias)?;
            self.escribir("}");
            self.escribir_linea_vacia();
            self.escribir("fn correr_pruebas() {");
            self.nivel += 1;
            self.escribir("ejecutar_pruebas(pruebas_definidas());");
            self.nivel -= 1;
            self.escribir("}");
            self.escribir_linea_vacia();
            self.escribir("fn main() {");
            self.nivel += 1;
            self.escribir("fijar_despachador(despachar_tabla);");
            self.escribir("con_pila_grande(correr_pruebas);");
            self.nivel -= 1;
            self.escribir("}");
            return Ok(());
        }
        self.escribir("fn programa_principal() {");
        self.generar_cuerpo_principal(sentencias)?;
        self.escribir("}");
        self.escribir_linea_vacia();
        if self.modo == Modo::SinSistema {
            // Lo llama el arranque (nucleo/sin_sistema.rs) ya en 64 bits.
            self.escribir("#[no_mangle]");
            self.escribir("pub extern \"C\" fn principal_nucleo() -> ! {");
            self.nivel += 1;
            self.escribir("fijar_despachador(despachar_tabla);");
            self.escribir("arrancar_nucleo(programa_principal)");
            self.nivel -= 1;
            self.escribir("}");
            return Ok(());
        }
        self.escribir("fn main() {");
        self.nivel += 1;
        self.escribir("fijar_despachador(despachar_tabla);");
        self.escribir("con_pila_grande(programa_principal);");
        self.nivel -= 1;
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
            Sentencia::Intentar {
                cuerpo,
                nombre,
                captura,
                linea,
                columna,
            } => {
                self.escribir("intentar_entrar();");
                self.escribir("let __ln_intento = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {");
                self.nivel += 1;
                self.abrir_scope();
                self.intentos += 1;
                self.bases_intentar.push(self.bucles);
                self.controles_intentar.push(ControlesIntento::default());
                self.generar_sentencias(cuerpo);
                let controles = self.controles_intentar.pop().unwrap_or_default();
                self.bases_intentar.pop();
                self.intentos -= 1;
                self.cerrar_scope();
                self.nivel -= 1;
                self.escribir("}));");
                self.escribir("intentar_salir();");
                self.escribir("match __ln_intento {");
                self.nivel += 1;
                self.escribir("Ok(_) => {}");
                self.escribir("Err(__ln_causa) => {");
                self.nivel += 1;
                self.escribir("if let Some(__ln_error) = __ln_causa.downcast_ref::<ErrorIntento>() {");
                self.nivel += 1;
                self.abrir_scope();
                self.declarar(nombre, *linea, *columna)?;
                self.escribir(&format!(
                    "let mut {}: Valor = error_intento(__ln_error);",
                    nombre_variable(nombre)
                ));
                self.generar_sentencias(captura);
                self.cerrar_scope();
                self.nivel -= 1;
                let mut cierres = Vec::new();
                if controles.romper {
                    cierres.push("} else if __ln_causa.is::<ControlRomper>() {");
                    cierres.push("    break;");
                }
                if controles.continuar {
                    cierres.push("} else if __ln_causa.is::<ControlContinuar>() {");
                    cierres.push("    continue;");
                }
                if controles.devolver {
                    cierres.push("} else if __ln_causa.is::<ControlDevolver>() {");
                    cierres.push("    return tomar_valor_devuelto();");
                }
                if cierres.is_empty() {
                    self.escribir("} else {");
                } else {
                    for linea_cierre in &cierres {
                        self.escribir(linea_cierre);
                    }
                    self.escribir("} else {");
                }
                self.nivel += 1;
                self.escribir("std::panic::resume_unwind(__ln_causa);");
                self.nivel -= 1;
                self.escribir("}");
                self.nivel -= 1;
                self.escribir("}");
                self.nivel -= 1;
                self.escribir("}");
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
                    let nivel = self.nivel + 2;
                    let bucle = crate::numerico::generar_bucle(condicion, cuerpo, nivel);
                    self.abrir_bucle_rapido(&libres, &bucle);
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
                    self.cerrar_bucle_rapido();
                }
            }
            Sentencia::Para {
                nombre,
                coleccion,
                cuerpo,
                linea,
            } => {
                let codigo = self.generar_expresion(coleccion)?;
                // `para` sobre un rango que solo hace cuentas: igual que el `mientras` rápido.
                let numerico = crate::numerico::para_numerico(sentencia, &self.numericas)
                    .filter(|libres| !libres.is_empty() && libres.iter().all(|v| self.variable_visible(v)));
                let numerico_abierto = numerico.is_some();
                if let Some(libres) = numerico {
                    let bucle = crate::numerico::generar_para(sentencia, self.nivel + 2);
                    self.abrir_bucle_rapido(&libres, &bucle);
                }
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
                if numerico_abierto {
                    self.cerrar_bucle_rapido();
                }
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
                let dentro = self.intentos > 0
                    && self.bucles == *self.bases_intentar.last().unwrap_or(&0);
                if dentro {
                    if let Some(controles) = self.controles_intentar.last_mut() {
                        if romper {
                            controles.romper = true;
                        } else {
                            controles.continuar = true;
                        }
                    }
                    self.escribir(if romper {
                        "std::panic::panic_any(ControlRomper);"
                    } else {
                        "std::panic::panic_any(ControlContinuar);"
                    });
                } else {
                    self.escribir(if romper { "break;" } else { "continue;" });
                }
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
                if self.intentos > 0 {
                    if let Some(controles) = self.controles_intentar.last_mut() {
                        controles.devolver = true;
                    }
                    self.escribir(&format!("devolver_de_intentar({});", codigo));
                } else {
                    self.escribir(&format!("return {};", codigo));
                }
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
                    if let Some(valor) = self.valor_de_funcion(nombre) {
                        return Ok(valor);
                    }
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
                // `f(x)` con `f` una variable que guarda una función (y que no tapa a otra).
                if self.variable_visible(nombre)
                    && integradas::buscar(nombre).is_none()
                    && !self.extensiones.contains_key(nombre)
                    && !self.funciones.contains_key(nombre)
                {
                    let mut partes = Vec::new();
                    for argumento in argumentos {
                        partes.push(self.generar_expresion(argumento)?);
                    }
                    return Ok(format!(
                        "{{ en_linea({}); llamar_funcion(&{}, vec![{}]) }}",
                        linea,
                        nombre_variable(nombre),
                        partes.join(", ")
                    ));
                }
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
                if self.modo == Modo::SinSistema {
                    if let Some(integrada) = integradas::buscar(nombre) {
                        if !permitida_sin_sistema(integrada) {
                            let error = error_sin_sistema(*linea, *columna, format!("'{}'", nombre));
                            self.registrar(error);
                            return Ok("Valor::Nada".to_string());
                        }
                    }
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
                    if let Some(valor) = self.valor_de_funcion(nombre) {
                        return Ok(format!("&({})", valor));
                    }
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

    /// Abre `if let (números..) = (variables..) { bucle rápido } else {`: el resto de la
    /// sentencia (el camino de siempre) se escribe a continuación y se cierra con `cerrar_bucle_rapido`.
    fn abrir_bucle_rapido(&mut self, libres: &[String], bucle: &str) {
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
        self.salida.push_str(bucle);
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

    fn cerrar_bucle_rapido(&mut self) {
        self.nivel -= 1;
        self.escribir("}");
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
        if let Some(modulo) = crate::biblioteca::modulo_de_funcion(nombre) {
            error = error.con_sugerencia(format!(
                "'{}' es del módulo '{}': añade usar modulo \"{}\" al principio de este archivo",
                nombre, modulo, modulo
            ));
            self.registrar(error);
            return;
        }
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

pub(crate) fn linea_de_sentencia(sentencia: &Sentencia) -> Option<usize> {
    match sentencia {
        Sentencia::Declarar { linea, .. }
        | Sentencia::Asignar { linea, .. }
        | Sentencia::AsignarIndice { linea, .. }
        | Sentencia::Si { linea, .. }
        | Sentencia::Intentar { linea, .. }
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
