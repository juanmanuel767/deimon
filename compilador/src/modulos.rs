//! Resuelve las instrucciones `usar`: lee los archivos indicados (relativos a la carpeta del
//! archivo que las contiene), carga sus funciones, rutas y pruebas, y devuelve un único programa.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ajenas;
use crate::antiguas;
use crate::ast::{Extension, FuncionExtension, Programa, Sentencia};
use crate::error::ErrorTraduccion;
use crate::{biblioteca, generador, integradas, lexer, parser};

/// Resuelve los `usar` de `programa` y devuelve el programa combinado.
pub fn resolver(
    programa: Programa,
    archivo: Option<&Path>,
) -> Result<Programa, Vec<ErrorTraduccion>> {
    resolver_con_fuentes(programa, archivo).map(|(programa, _)| programa)
}

/// Como `resolver`, pero también devuelve el texto de cada archivo usado para poder
/// mostrar la línea del código en los errores.
pub fn resolver_con_fuentes(
    programa: Programa,
    archivo: Option<&Path>,
) -> Result<(Programa, HashMap<String, String>), Vec<ErrorTraduccion>> {
    let mut resolutor = Resolutor::nuevo(archivo);
    let principal = resolutor.contexto_principal();
    let mut sentencias = Vec::new();
    for sentencia in programa.sentencias {
        match sentencia {
            Sentencia::Usar {
                ruta,
                linea,
                columna,
            } => {
                let mut definiciones = resolutor.cargar(&ruta, linea, columna, &principal);
                sentencias.append(&mut definiciones);
            }
            Sentencia::UsarExtension {
                nombre,
                linea,
                columna,
            } => {
                let mut definiciones = resolutor.cargar_modulo(&nombre, linea, columna, &principal);
                sentencias.append(&mut definiciones);
            }
            otra => {
                resolutor.registrar_definicion(&otra, &None, &principal);
                sentencias.push(otra);
            }
        }
    }
    resolutor.validar_extensiones();
    if resolutor.errores.is_empty() {
        let fuentes = resolutor.fuentes;
        Ok((
            Programa {
                sentencias,
                extensiones: resolutor.extensiones,
            },
            fuentes,
        ))
    } else {
        Err(resolutor.errores)
    }
}

struct Contexto {
    /// Carpeta canónica del archivo que contiene el `usar`.
    dir: PathBuf,
    /// Ruta visible del archivo (relativa a la carpeta del principal).
    mostrar: String,
    /// Archivo visible donde marcar los errores; `None` si es el principal.
    archivo: Option<String>,
    /// Texto del archivo, para dibujar la línea del error.
    fuente: String,
}

struct Resolutor {
    base: PathBuf,
    principal: String,
    cargados: HashSet<PathBuf>,
    pila: Vec<(PathBuf, String)>,
    fuentes: HashMap<String, String>,
    funciones: HashMap<String, String>,
    rutas: HashMap<String, String>,
    extensiones: Vec<Extension>,
    extensiones_vistas: HashSet<PathBuf>,
    errores: Vec<ErrorTraduccion>,
}

impl Resolutor {
    fn nuevo(archivo: Option<&Path>) -> Self {
        let canonico = archivo.and_then(|ruta| fs::canonicalize(ruta).ok());
        let base = canonico
            .as_ref()
            .and_then(|ruta| ruta.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let principal = archivo
            .and_then(|ruta| ruta.file_name())
            .map(|nombre| nombre.to_string_lossy().to_string())
            .unwrap_or_else(|| "programa.dmn".to_string());
        let mut pila = Vec::new();
        if let Some(canonico) = canonico {
            pila.push((canonico, principal.clone()));
        }
        Resolutor {
            base,
            principal,
            cargados: HashSet::new(),
            pila,
            fuentes: HashMap::new(),
            funciones: HashMap::new(),
            rutas: HashMap::new(),
            extensiones: Vec::new(),
            extensiones_vistas: HashSet::new(),
            errores: Vec::new(),
        }
    }

    fn contexto_principal(&self) -> Contexto {
        Contexto {
            dir: self.base.clone(),
            mostrar: self.principal.clone(),
            archivo: None,
            fuente: String::new(),
        }
    }

    fn marcar(&self, error: ErrorTraduccion, desde: &Contexto) -> ErrorTraduccion {
        match &desde.archivo {
            Some(archivo) => error.con_archivo(archivo).con_fuente(&desde.fuente),
            None => error,
        }
    }

    fn cargar(
        &mut self,
        ruta: &str,
        linea: usize,
        columna: usize,
        desde: &Contexto,
    ) -> Vec<Sentencia> {
        let mostrar = mostrar_para(desde, ruta);
        let candidato = if Path::new(ruta).is_absolute() {
            PathBuf::from(ruta)
        } else {
            desde.dir.join(ruta)
        };
        let canonico = match fs::canonicalize(&candidato) {
            Ok(canonico) => canonico,
            Err(error) => {
                let mensaje = if error.kind() == std::io::ErrorKind::NotFound {
                    format!("no se encontró el archivo '{}'", mostrar)
                } else {
                    format!("no se pudo leer el archivo '{}': {}", mostrar, error)
                };
                let error = ErrorTraduccion::nuevo(linea, mensaje)
                    .con_codigo("E014")
                    .en_columna(columna)
                    .con_sugerencia(
                        "Comprueba la ruta: se escribe entre comillas y es relativa a la carpeta del archivo que la contiene",
                    );
                let error = self.marcar(error, desde);
                self.errores.push(error);
                return Vec::new();
            }
        };

        if let Some(posicion) = self.pila.iter().position(|(cargado, _)| *cargado == canonico) {
            let cadena = self.pila[posicion..]
                .iter()
                .map(|(_, nombre)| nombre.clone())
                .chain(std::iter::once(mostrar.clone()))
                .collect::<Vec<String>>()
                .join(" → ");
            let error = ErrorTraduccion::nuevo(
                linea,
                format!("uso circular entre archivos: {}", cadena),
            )
            .con_codigo("E015")
            .en_columna(columna)
            .con_sugerencia("Quita uno de los 'usar' que forman el círculo para que los archivos no se carguen unos a otros");
            let error = self.marcar(error, desde);
            self.errores.push(error);
            return Vec::new();
        }

        if self.cargados.contains(&canonico) {
            return Vec::new();
        }
        self.cargados.insert(canonico.clone());

        let fuente = match fs::read_to_string(&canonico) {
            Ok(fuente) => fuente,
            Err(error) => {
                let mensaje = format!("no se pudo leer el archivo '{}': {}", mostrar, error);
                let error = ErrorTraduccion::nuevo(linea, mensaje)
                    .con_codigo("E014")
                    .en_columna(columna)
                    .con_sugerencia("Comprueba que el archivo existe y se puede leer");
                let error = self.marcar(error, desde);
                self.errores.push(error);
                return Vec::new();
            }
        };

        let contexto = Contexto {
            dir: canonico
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.base.clone()),
            mostrar: mostrar.clone(),
            archivo: Some(mostrar.clone()),
            fuente: fuente.clone(),
        };
        self.fuentes.insert(mostrar.clone(), fuente.clone());
        self.pila.push((canonico, mostrar.clone()));

        let definiciones = self.procesar_archivo(&fuente, &contexto);

        self.pila.pop();
        definiciones
    }

    fn procesar_archivo(&mut self, fuente: &str, contexto: &Contexto) -> Vec<Sentencia> {
        let tokens = match lexer::tokenizar(fuente) {
            Ok(tokens) => tokens,
            Err(error) => {
                let error = self.marcar(error, contexto);
                self.errores.push(error);
                return Vec::new();
            }
        };
        let (tokens, avisos) = ajenas::corregir(tokens);
        let (tokens, de_antiguas) = antiguas::corregir(tokens);
        for aviso in avisos.into_iter().chain(de_antiguas) {
            let aviso = self.marcar(aviso, contexto);
            self.errores.push(aviso);
        }
        let programa = match parser::analizar(tokens) {
            Ok(programa) => programa,
            Err(error) => {
                let error = self.marcar(error, contexto);
                self.errores.push(error);
                return Vec::new();
            }
        };

        let mut definiciones = Vec::new();
        for sentencia in programa.sentencias {
            match sentencia {
                Sentencia::Funcion { .. }
                | Sentencia::Ruta { .. }
                | Sentencia::Prueba { .. } => {
                    let mut sentencia = sentencia;
                    poner_archivo(&mut sentencia, &contexto.mostrar);
                    self.registrar_definicion(
                        &sentencia,
                        &Some(contexto.mostrar.clone()),
                        contexto,
                    );
                    definiciones.push(sentencia);
                }
                Sentencia::Usar {
                    ruta,
                    linea,
                    columna,
                } => {
                    let mut sub = self.cargar(&ruta, linea, columna, contexto);
                    definiciones.append(&mut sub);
                }
                Sentencia::UsarExtension {
                    nombre,
                    linea,
                    columna,
                } => {
                    let mut sub = self.cargar_modulo(&nombre, linea, columna, contexto);
                    definiciones.append(&mut sub);
                }
                _ => {}
            }
        }
        definiciones
    }

    fn registrar_definicion(
        &mut self,
        sentencia: &Sentencia,
        origen: &Option<String>,
        contexto: &Contexto,
    ) {
        let (nombre, es_funcion) = match sentencia {
            Sentencia::Funcion { nombre, .. } => (nombre.clone(), true),
            Sentencia::Ruta { camino, .. } => (camino.clone(), false),
            _ => return,
        };
        let etiqueta = origen
            .clone()
            .unwrap_or_else(|| self.principal.clone());
        let mapa = if es_funcion {
            &mut self.funciones
        } else {
            &mut self.rutas
        };
        match mapa.get(&nombre) {
            Some(anterior) if anterior != &etiqueta => {
                let anterior = anterior.clone();
                let linea = linea_de(sentencia);
                let columna = columna_de(sentencia);
                let (mensaje, sugerencia) = if es_funcion {
                    (
                        format!(
                            "la función '{}' está definida en '{}' y en '{}'",
                            nombre, anterior, etiqueta
                        ),
                        "Cambia el nombre en uno de los dos archivos o quita uno de los dos 'usar'",
                    )
                } else {
                    (
                        format!(
                            "la ruta '{}' está definida en '{}' y en '{}'",
                            nombre, anterior, etiqueta
                        ),
                        "Cambia el camino en uno de los dos archivos o quita uno de los dos 'usar'",
                    )
                };
                let error = ErrorTraduccion::nuevo(linea, mensaje)
                    .con_codigo("E016")
                    .en_columna(columna)
                    .con_sugerencia(sugerencia);
                let error = self.marcar(error, contexto);
                self.errores.push(error);
            }
            Some(_) => {}
            None => {
                mapa.insert(nombre, etiqueta);
            }
        }
    }

    /// Carga un módulo buscando `modulos/<nombre>/` junto al archivo, después en
    /// `~/.deimon/modulos/<nombre>/` y por último en la biblioteca estándar. Un módulo tiene
    /// `modulo.txt` y, opcionalmente, funciones en Rust (`funciones.txt` + `codigo.rs`) y
    /// archivos `.dmn`. Solo se incluye una vez aunque se use varias veces.
    fn cargar_modulo(
        &mut self,
        nombre: &str,
        linea: usize,
        columna: usize,
        desde: &Contexto,
    ) -> Vec<Sentencia> {
        let local = desde.dir.join("modulos").join(nombre);
        let personal = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".deimon")
            .join("modulos")
            .join(nombre);
        let estandar = biblioteca::carpeta().map(|carpeta| carpeta.join(nombre));
        let carpeta = [Some(local.clone()), Some(personal.clone()), estandar]
            .into_iter()
            .flatten()
            .find(|carpeta| carpeta.is_dir());
        let carpeta = match carpeta {
            Some(carpeta) => carpeta,
            None => {
                let disponibles = biblioteca::nombres();
                let parecido = disponibles
                    .iter()
                    .map(|disponible| (generador::distancia_edicion(nombre, disponible), *disponible))
                    .filter(|(distancia, _)| *distancia <= 3)
                    .min();
                let sugerencia = match parecido {
                    Some((_, parecido)) => format!("¿Quisiste decir \"{}\"?", parecido),
                    None => format!(
                        "Crea la carpeta 'modulos/{}/' junto al archivo .dmn (con modulo.txt) o usa uno de la biblioteca estándar: {}",
                        nombre,
                        disponibles.join(", ")
                    ),
                };
                let error = ErrorTraduccion::nuevo(
                    linea,
                    format!(
                        "no se encontró el módulo '{}'; se buscó en '{}', en '{}' y en la biblioteca estándar ({})",
                        nombre,
                        local.display(),
                        personal.display(),
                        disponibles.join(", ")
                    ),
                )
                .con_codigo("E017")
                .en_columna(columna)
                .con_sugerencia(sugerencia);
                let error = self.marcar(error, desde);
                self.errores.push(error);
                return Vec::new();
            }
        };

        let canonico = fs::canonicalize(&carpeta).unwrap_or_else(|_| carpeta.clone());
        if !self.extensiones_vistas.insert(canonico) {
            return Vec::new();
        }

        let (descripcion, requiere) = match leer_ficha(&carpeta, nombre) {
            Ok(ficha) => ficha,
            Err(mensaje) => {
                let error = ErrorTraduccion::nuevo(linea, mensaje)
                    .con_codigo("E018")
                    .en_columna(columna)
                    .con_sugerencia("modulo.txt tiene líneas 'nombre: x', 'descripcion: ...' y, si hace falta, 'requiere: red, archivos, hilos'");
                let error = self.marcar(error, desde);
                self.errores.push(error);
                return Vec::new();
            }
        };

        let ruta_funciones = carpeta.join("funciones.txt");
        let ruta_codigo = carpeta.join("codigo.rs");
        let (funciones, codigo, valida) = match (ruta_funciones.is_file(), ruta_codigo.is_file()) {
            (false, false) => (Vec::new(), String::new(), true),
            (true, true) => {
                let leidos = fs::read_to_string(&ruta_funciones)
                    .and_then(|texto| fs::read_to_string(&ruta_codigo).map(|codigo| (texto, codigo)));
                match leidos {
                    Ok((texto, codigo)) => {
                        let (funciones, mut errores, valida) =
                            leer_funciones(&texto, nombre, linea, columna);
                        for error in &mut errores {
                            *error = self.marcar(error.clone(), desde);
                        }
                        self.errores.append(&mut errores);
                        (funciones, codigo, valida)
                    }
                    Err(error) => {
                        let error = ErrorTraduccion::nuevo(
                            linea,
                            format!("no se pudieron leer las funciones en Rust del módulo '{}': {}", nombre, error),
                        )
                        .con_codigo("E018")
                        .en_columna(columna)
                        .con_sugerencia("Comprueba que funciones.txt y codigo.rs se pueden leer");
                        let error = self.marcar(error, desde);
                        self.errores.push(error);
                        return Vec::new();
                    }
                }
            }
            (tiene_funciones, _) => {
                let (presente, falta) = if tiene_funciones {
                    ("funciones.txt", "codigo.rs")
                } else {
                    ("codigo.rs", "funciones.txt")
                };
                let error = ErrorTraduccion::nuevo(
                    linea,
                    format!("el módulo '{}' tiene {} pero le falta {}", nombre, presente, falta),
                )
                .con_codigo("E018")
                .en_columna(columna)
                .con_sugerencia("Las funciones en Rust necesitan los dos archivos: funciones.txt (una línea por función) y codigo.rs (una 'fn ext_<nombre>' por función)");
                let error = self.marcar(error, desde);
                self.errores.push(error);
                return Vec::new();
            }
        };

        if valida {
            self.extensiones.push(Extension {
                nombre: nombre.to_string(),
                descripcion,
                requiere,
                funciones,
                codigo,
                linea,
                columna,
                archivo: desde.archivo.clone(),
            });
        }

        // Las funciones escritas en Deimon del módulo se cargan como con `usar "archivo.dmn"`.
        let mut archivos_ln: Vec<PathBuf> = fs::read_dir(&carpeta)
            .map(|entradas| {
                entradas
                    .filter_map(|entrada| entrada.ok().map(|entrada| entrada.path()))
                    .filter(|ruta| ruta.extension().map(|e| e == "dmn").unwrap_or(false))
                    .collect()
            })
            .unwrap_or_default();
        archivos_ln.sort();
        let mut definiciones = Vec::new();
        for archivo in archivos_ln {
            let mut sub = self.cargar(&archivo.to_string_lossy(), linea, columna, desde);
            definiciones.append(&mut sub);
        }
        definiciones
    }

    /// Comprueba que las funciones de las extensiones no chocan con las integradas ni con
    /// las funciones del programa (ni entre sí).
    fn validar_extensiones(&mut self) {
        let extensiones = self.extensiones.clone();
        let mut vistos: HashMap<String, String> = HashMap::new();
        for extension in &extensiones {
            for funcion in &extension.funciones {
                let mut error = None;
                if integradas::buscar(&funcion.nombre).is_some() {
                    error = Some(
                        ErrorTraduccion::nuevo(
                            extension.linea,
                            format!(
                                "la función '{}' del módulo '{}' (funciones.txt, línea {}) choca con una función integrada",
                                funcion.nombre, extension.nombre, funcion.linea_funciones
                            ),
                        )
                        .con_codigo("E020")
                        .en_columna(extension.columna)
                        .con_sugerencia(format!(
                            "Cambia el nombre de '{}' en funciones.txt por otro que no exista en el lenguaje",
                            funcion.nombre
                        )),
                    );
                } else if let Some(otra) = vistos.get(&funcion.nombre) {
                    error = Some(
                        ErrorTraduccion::nuevo(
                            extension.linea,
                            format!(
                                "la función '{}' del módulo '{}' ya existe en el módulo '{}'",
                                funcion.nombre, extension.nombre, otra
                            ),
                        )
                        .con_codigo("E020")
                        .en_columna(extension.columna)
                        .con_sugerencia(
                            "Cambia el nombre de una de las dos funciones para que no se repita",
                        ),
                    );
                } else if self.funciones.contains_key(&funcion.nombre) {
                    error = Some(
                        ErrorTraduccion::nuevo(
                            extension.linea,
                            format!(
                                "la función '{}' del módulo '{}' choca con una función del programa",
                                funcion.nombre, extension.nombre
                            ),
                        )
                        .con_codigo("E020")
                        .en_columna(extension.columna)
                        .con_sugerencia(format!(
                            "Cambia el nombre de '{}' en funciones.txt o el de tu función del programa",
                            funcion.nombre
                        )),
                    );
                }
                if let Some(error) = error {
                    let error = self.marcar_extension(error, extension);
                    self.errores.push(error);
                }
                vistos
                    .entry(funcion.nombre.clone())
                    .or_insert_with(|| extension.nombre.clone());
            }
        }
    }

    fn marcar_extension(&self, error: ErrorTraduccion, extension: &Extension) -> ErrorTraduccion {
        match &extension.archivo {
            Some(archivo) => error
                .con_archivo(archivo)
                .con_fuente(self.fuentes.get(archivo).map(String::as_str).unwrap_or("")),
            None => error,
        }
    }
}

/// Capacidades que un módulo puede pedir en la línea `requiere:` de `modulo.txt`.
pub const CAPACIDADES: &[&str] = &["red", "archivos", "hilos"];

/// Lee `modulo.txt` y devuelve la descripción y las capacidades que requiere el módulo.
fn leer_ficha(carpeta: &Path, nombre: &str) -> Result<(String, Vec<String>), String> {
    let ruta = carpeta.join("modulo.txt");
    let texto = fs::read_to_string(&ruta).map_err(|error| {
        format!(
            "el módulo '{}' no tiene un modulo.txt que se pueda leer ('{}'): {}",
            nombre,
            ruta.display(),
            error
        )
    })?;
    let mut nombre_ficha = None;
    let mut descripcion = None;
    let mut requiere = Vec::new();
    for (numero, linea) in texto.lines().enumerate() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let (clave, valor) = match linea.split_once(':') {
            Some((clave, valor)) => (clave.trim(), valor.trim()),
            None => {
                return Err(format!(
                    "modulo.txt del módulo '{}', línea {}: se esperaba 'clave: valor'",
                    nombre,
                    numero + 1
                ))
            }
        };
        match clave {
            "nombre" => nombre_ficha = Some(valor.to_string()),
            "descripcion" => descripcion = Some(valor.to_string()),
            "requiere" => {
                for capacidad in valor.split(',').map(str::trim).filter(|c| !c.is_empty()) {
                    if !CAPACIDADES.contains(&capacidad) {
                        return Err(format!(
                            "modulo.txt del módulo '{}', línea {}: la capacidad '{}' no existe (valen: {})",
                            nombre,
                            numero + 1,
                            capacidad,
                            CAPACIDADES.join(", ")
                        ));
                    }
                    requiere.push(capacidad.to_string());
                }
            }
            otra => {
                return Err(format!(
                    "modulo.txt del módulo '{}', línea {}: la clave '{}' no existe (valen: nombre, descripcion, requiere)",
                    nombre,
                    numero + 1,
                    otra
                ))
            }
        }
    }
    match (nombre_ficha, descripcion) {
        (Some(ficha), _) if ficha != nombre => Err(format!(
            "modulo.txt dice 'nombre: {}', pero la carpeta del módulo se llama '{}'",
            ficha, nombre
        )),
        (Some(_), Some(descripcion)) => Ok((descripcion, requiere)),
        _ => Err(format!(
            "modulo.txt del módulo '{}' necesita las líneas 'nombre: {}' y 'descripcion: ...'",
            nombre, nombre
        )),
    }
}

/// Lee `funciones.txt`. Devuelve las funciones, los errores encontrados y si el archivo
/// está bien formado (en cuyo caso se puede usar el módulo).
fn leer_funciones(
    texto: &str,
    extension: &str,
    linea: usize,
    columna: usize,
) -> (Vec<FuncionExtension>, Vec<ErrorTraduccion>, bool) {
    let mut funciones = Vec::new();
    let mut errores = Vec::new();
    let mut vistos = HashSet::new();
    for (indice, linea_texto) in texto.lines().enumerate() {
        let numero = indice + 1;
        let limpia = linea_texto.trim();
        if limpia.is_empty() || limpia.starts_with('#') {
            continue;
        }
        let partes: Vec<&str> = limpia.split('|').map(str::trim).collect();
        let mal = |detalle: String| {
            ErrorTraduccion::nuevo(
                linea,
                format!(
                    "el módulo '{}' tiene funciones.txt mal formado en la línea {}: {}",
                    extension, numero, detalle
                ),
            )
            .con_codigo("E018")
            .en_columna(columna)
            .con_sugerencia(
                "Cada línea debe ser: nombre | número_de_argumentos | descripción | ejemplo (las vacías y las que empiezan por # se ignoran)",
            )
        };
        if partes.len() != 4 {
            errores.push(mal(format!(
                "se esperaban 4 campos separados por '|' y se encontraron {}",
                partes.len()
            )));
            continue;
        }
        let nombre = partes[0];
        if !identificador_valido(nombre) {
            errores.push(mal(format!(
                "el nombre '{}' no es un identificador válido",
                nombre
            )));
            continue;
        }
        let aridad = match partes[1].parse::<usize>() {
            Ok(aridad) => aridad,
            Err(_) => {
                errores.push(mal(format!(
                    "el número de argumentos '{}' no es un número entero",
                    partes[1]
                )));
                continue;
            }
        };
        if partes[2].is_empty() {
            errores.push(mal("la descripción está vacía".to_string()));
            continue;
        }
        if !vistos.insert(nombre.to_string()) {
            errores.push(mal(format!("la función '{}' está repetida", nombre)));
            continue;
        }
        funciones.push(FuncionExtension {
            nombre: nombre.to_string(),
            parametros: parametros_extension(aridad),
            descripcion: partes[2].to_string(),
            ejemplo: partes[3].to_string(),
            linea_funciones: numero,
        });
    }
    let valida = errores.is_empty();
    (funciones, errores, valida)
}

fn parametros_extension(aridad: usize) -> Vec<String> {
    match aridad {
        0 => Vec::new(),
        1 => vec!["argumento".to_string()],
        total => (1..=total)
            .map(|indice| format!("argumento{}", indice))
            .collect(),
    }
}

fn identificador_valido(nombre: &str) -> bool {
    let mut caracteres = nombre.chars();
    match caracteres.next() {
        Some(primero) if primero.is_alphabetic() || primero == '_' => caracteres
            .all(|caracter| caracter.is_alphanumeric() || caracter == '_'),
        _ => false,
    }
}

fn poner_archivo(sentencia: &mut Sentencia, archivo: &str) {
    match sentencia {
        Sentencia::Funcion {
            archivo: destino, ..
        }
        | Sentencia::Ruta {
            archivo: destino, ..
        }
        | Sentencia::Prueba {
            archivo: destino, ..
        } => *destino = Some(archivo.to_string()),
        _ => {}
    }
}

fn linea_de(sentencia: &Sentencia) -> usize {
    match sentencia {
        Sentencia::Funcion { linea, .. }
        | Sentencia::Ruta { linea, .. }
        | Sentencia::Prueba { linea, .. } => *linea,
        _ => 0,
    }
}

fn columna_de(sentencia: &Sentencia) -> usize {
    match sentencia {
        Sentencia::Funcion { columna, .. }
        | Sentencia::Ruta { columna, .. }
        | Sentencia::Prueba { columna, .. } => *columna,
        _ => 0,
    }
}

fn mostrar_padre(mostrar: &str) -> &str {
    match mostrar.rfind('/') {
        Some(posicion) => &mostrar[..posicion],
        None => "",
    }
}

fn mostrar_para(desde: &Contexto, ruta: &str) -> String {
    if Path::new(ruta).is_absolute() {
        ruta.to_string()
    } else {
        unir_mostrar(mostrar_padre(&desde.mostrar), ruta)
    }
}

/// Une dos rutas visibles y resuelve `.` y `..` sin tocar el disco.
fn unir_mostrar(padre: &str, ruta: &str) -> String {
    let mut partes: Vec<&str> = Vec::new();
    for parte in padre.split('/').chain(ruta.split('/')) {
        match parte {
            "" | "." => {}
            ".." => {
                partes.pop();
            }
            otra => partes.push(otra),
        }
    }
    partes.join("/")
}

/// Descripción de un módulo para `deimon modulos`.
#[derive(Debug, Clone)]
pub struct Ficha {
    pub nombre: String,
    pub descripcion: String,
    pub requiere: Vec<String>,
    /// Dónde se encontró: "biblioteca estándar", "junto al programa" o "~/.deimon/modulos".
    pub origen: String,
    /// (firma, descripción) de cada función, primero las de Rust y después las de Deimon.
    pub funciones: Vec<(String, String)>,
}

/// Lee la ficha de cada módulo (subcarpeta con modulo.txt) de `carpeta`, por orden de nombre.
pub fn fichas_en(carpeta: &Path, origen: &str) -> Vec<Ficha> {
    let mut carpetas: Vec<PathBuf> = fs::read_dir(carpeta)
        .map(|entradas| {
            entradas
                .filter_map(|entrada| entrada.ok().map(|entrada| entrada.path()))
                .filter(|ruta| ruta.join("modulo.txt").is_file())
                .collect()
        })
        .unwrap_or_default();
    carpetas.sort();
    carpetas
        .iter()
        .filter_map(|carpeta| {
            let nombre = carpeta.file_name()?.to_string_lossy().to_string();
            let (descripcion, requiere) = leer_ficha(carpeta, &nombre).ok()?;
            let mut funciones = Vec::new();
            if let Ok(texto) = fs::read_to_string(carpeta.join("funciones.txt")) {
                for funcion in leer_funciones(&texto, &nombre, 1, 1).0 {
                    funciones.push((funcion.firma(), funcion.descripcion));
                }
            }
            let mut archivos_ln: Vec<PathBuf> = fs::read_dir(carpeta)
                .map(|entradas| {
                    entradas
                        .filter_map(|entrada| entrada.ok().map(|entrada| entrada.path()))
                        .filter(|ruta| ruta.extension().map(|e| e == "dmn").unwrap_or(false))
                        .collect()
                })
                .unwrap_or_default();
            archivos_ln.sort();
            for archivo in archivos_ln {
                let programa = fs::read_to_string(&archivo)
                    .ok()
                    .and_then(|fuente| lexer::tokenizar(&fuente).ok())
                    .and_then(|tokens| parser::analizar(tokens).ok());
                for sentencia in programa.map(|p| p.sentencias).unwrap_or_default() {
                    if let Sentencia::Funcion {
                        nombre, parametros, ..
                    } = sentencia
                    {
                        funciones.push((
                            format!("{}({})", nombre, parametros.join(", ")),
                            "escrita en Deimon".to_string(),
                        ));
                    }
                }
            }
            Some(Ficha {
                nombre,
                descripcion,
                requiere,
                origen: origen.to_string(),
                funciones,
            })
        })
        .collect()
}
