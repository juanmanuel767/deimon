//! Resuelve las instrucciones `usar`: lee los archivos indicados (relativos a la carpeta del
//! archivo que las contiene), carga sus funciones, rutas y pruebas, y devuelve un único programa.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ajenas;
use crate::antiguas;
use crate::ast::{Expresion, Extension, FuncionExtension, ParteTexto, Programa, Sentencia};
use crate::error::ErrorTraduccion;
use crate::{biblioteca, fuentes, generador, integradas, lexer, parser, plantillas};

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
                let mut otra = otra;
                for error in plantillas::incluir(std::slice::from_mut(&mut otra), &principal.dir) {
                    resolutor.errores.push(error);
                }
                resolutor.registrar_definicion(&otra, &None, &principal);
                sentencias.push(otra);
            }
        }
    }
    cargar_modulos_sin_requisitos(&mut resolutor, &mut sentencias, &principal);
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

/// Los módulos de la biblioteca estándar que no piden nada (texto, matematicas, json...) se
/// cargan solos cuando el programa llama a una de sus funciones sin escribir `usar modulo`:
/// olvidarlo era el fallo más común al escribir Deimon. Una función propia con el mismo nombre
/// gana, y los módulos con red o archivos se siguen pidiendo a mano.
fn cargar_modulos_sin_requisitos(resolutor: &mut Resolutor, sentencias: &mut Vec<Sentencia>, principal: &Contexto) {
    loop {
        let mut definidas: HashSet<String> = resolutor
            .extensiones
            .iter()
            .flat_map(|extension| extension.funciones.iter().map(|funcion| funcion.nombre.clone()))
            .collect();
        for sentencia in sentencias.iter() {
            if let Sentencia::Funcion { nombre, .. } = sentencia {
                definidas.insert(nombre.clone());
            }
        }
        let mut llamadas = Vec::new();
        llamadas_en(sentencias, &mut llamadas);
        let pendiente = llamadas.into_iter().find_map(|(nombre, linea, columna)| {
            if definidas.contains(&nombre) || integradas::buscar(&nombre).is_some() {
                return None;
            }
            biblioteca::modulo_de_funcion(&nombre)
                .filter(|modulo| biblioteca::modulo_sin_requisitos(modulo))
                .map(|modulo| (modulo, linea, columna))
        });
        let Some((modulo, linea, columna)) = pendiente else { return };
        let antes = resolutor.extensiones.len();
        let mut nuevas = resolutor.cargar_modulo(modulo, linea, columna, principal);
        if nuevas.is_empty() && resolutor.extensiones.len() == antes {
            // Ya estaba cargado (o no se pudo cargar y ya hay error): no repetir.
            return;
        }
        sentencias.append(&mut nuevas);
    }
}

fn llamadas_en(sentencias: &[Sentencia], salida: &mut Vec<(String, usize, usize)>) {
    for sentencia in sentencias {
        match sentencia {
            Sentencia::Declarar { valor, .. } | Sentencia::Asignar { valor, .. } | Sentencia::Expresion(valor) => {
                llamadas_de(valor, salida)
            }
            Sentencia::AsignarIndice { indices, valor, .. } => {
                indices.iter().for_each(|indice| llamadas_de(indice, salida));
                llamadas_de(valor, salida);
            }
            Sentencia::Si { condicion, entonces, sino, .. } => {
                llamadas_de(condicion, salida);
                llamadas_en(entonces, salida);
                llamadas_en(sino, salida);
            }
            Sentencia::Intentar { cuerpo, captura, .. } => {
                llamadas_en(cuerpo, salida);
                llamadas_en(captura, salida);
            }
            Sentencia::Mientras { condicion, cuerpo, .. } => {
                llamadas_de(condicion, salida);
                llamadas_en(cuerpo, salida);
            }
            Sentencia::Para { coleccion, cuerpo, .. } => {
                llamadas_de(coleccion, salida);
                llamadas_en(cuerpo, salida);
            }
            Sentencia::Funcion { cuerpo, .. } | Sentencia::Ruta { cuerpo, .. } | Sentencia::Prueba { cuerpo, .. } => {
                llamadas_en(cuerpo, salida)
            }
            Sentencia::Devolver { valor: Some(valor), .. } => llamadas_de(valor, salida),
            _ => {}
        }
    }
}

fn llamadas_de(expresion: &Expresion, salida: &mut Vec<(String, usize, usize)>) {
    match expresion {
        Expresion::Llamada { nombre, argumentos, linea, columna } => {
            salida.push((nombre.clone(), *linea, *columna));
            argumentos.iter().for_each(|argumento| llamadas_de(argumento, salida));
        }
        Expresion::Lista(elementos) => elementos.iter().for_each(|elemento| llamadas_de(elemento, salida)),
        Expresion::Mapa(pares) => pares.iter().for_each(|(clave, valor)| {
            llamadas_de(clave, salida);
            llamadas_de(valor, salida);
        }),
        Expresion::TextoConValores { partes, .. } => partes.iter().for_each(|parte| {
            if let ParteTexto::Valor(valor) = parte {
                llamadas_de(valor, salida);
            }
        }),
        Expresion::Unaria { valor, .. } => llamadas_de(valor, salida),
        Expresion::Binaria { izquierda, derecha, .. } => {
            llamadas_de(izquierda, salida);
            llamadas_de(derecha, salida);
        }
        Expresion::Indice { coleccion, indice, .. } => {
            llamadas_de(coleccion, salida);
            llamadas_de(indice, salida);
        }
        _ => {}
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
        let canonico = archivo.and_then(|ruta| fuentes::canonica(ruta).ok());
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
        let canonico = match fuentes::canonica(&candidato) {
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

        let fuente = match fuentes::leer_texto(&canonico) {
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
                    for error in plantillas::incluir(std::slice::from_mut(&mut sentencia), &contexto.dir) {
                        let error = self.marcar(error, contexto);
                        self.errores.push(error);
                    }
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
        let personal = carpeta_personal().join(nombre);
        let estandar = biblioteca::carpeta().map(|carpeta| carpeta.join(nombre));
        let carpeta = [Some(local.clone()), Some(personal.clone()), estandar]
            .into_iter()
            .flatten()
            .find(|carpeta| fuentes::es_carpeta(carpeta));
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

        let canonico = fuentes::canonica(&carpeta).unwrap_or_else(|_| carpeta.clone());
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
        let (funciones, codigo, valida) = match (fuentes::es_archivo(&ruta_funciones), fuentes::es_archivo(&ruta_codigo)) {
            (false, false) => (Vec::new(), String::new(), true),
            (true, true) => {
                let leidos = fuentes::leer_texto(&ruta_funciones)
                    .and_then(|texto| fuentes::leer_texto(&ruta_codigo).map(|codigo| (texto, codigo)));
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
        let mut archivos_ln: Vec<PathBuf> = fuentes::listar(&carpeta)
            .map(|rutas| {
                rutas
                    .into_iter()
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
    let texto = fuentes::leer_texto(&ruta).map_err(|error| {
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
                "Cada línea debe ser: nombre | parámetros separados por comas (o cuántos son; 0 si ninguno) | descripción | ejemplo (las vacías y las que empiezan por # se ignoran)",
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
        // Nombres de los parámetros ("texto, buscado") o, como antes, solo cuántos son ("2").
        let parametros = match partes[1].parse::<usize>() {
            Ok(aridad) => parametros_extension(aridad),
            Err(_) => {
                let nombres: Vec<String> = partes[1].split(',').map(|n| n.trim().to_string()).collect();
                if let Some(malo) = nombres.iter().find(|n| !identificador_valido(n)) {
                    errores.push(mal(format!(
                        "'{}' no es un nombre de parámetro válido (escribe los nombres separados por comas, o 0 si no tiene)",
                        malo
                    )));
                    continue;
                }
                nombres
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
            parametros,
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

/// Carpeta personal de módulos: `$DEIMON_CASA/.deimon/modulos` (por defecto `$HOME`).
pub fn carpeta_personal() -> PathBuf {
    // En un programa empaquetado, los módulos personales que usó viajan dentro del paquete.
    if crate::fuentes::en_paquete() {
        return PathBuf::from(crate::fuentes::RAIZ_PERSONAL);
    }
    env::var_os("DEIMON_CASA")
        .filter(|casa| !casa.is_empty())
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".deimon")
        .join("modulos")
}

/// Comprueba que `carpeta` es un módulo válido (modulo.txt y, si lo tiene, funciones.txt con
/// codigo.rs) y devuelve su nombre, que es el que dice modulo.txt.
pub fn validar_carpeta(carpeta: &Path) -> Result<String, String> {
    let texto = fuentes::leer_texto(&carpeta.join("modulo.txt"))
        .map_err(|_| "la carpeta no tiene un modulo.txt que se pueda leer".to_string())?;
    let nombre = texto
        .lines()
        .filter_map(|linea| linea.trim().strip_prefix("nombre:"))
        .map(|valor| valor.trim().to_string())
        .next()
        .ok_or_else(|| "modulo.txt no tiene la línea 'nombre: ...'".to_string())?;
    if nombre.is_empty() || !nombre.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_')) {
        return Err(format!(
            "el nombre '{}' no vale: usa solo letras minúsculas sin tilde, números y _",
            nombre
        ));
    }
    leer_ficha(carpeta, &nombre)?;
    let funciones = carpeta.join("funciones.txt");
    let codigo = carpeta.join("codigo.rs");
    match (fuentes::es_archivo(&funciones), fuentes::es_archivo(&codigo)) {
        (false, false) => {}
        (true, true) => {
            let texto = fuentes::leer_texto(&funciones)
                .map_err(|error| format!("no se pudo leer funciones.txt: {}", error))?;
            let (_, errores, valida) = leer_funciones(&texto, &nombre, 1, 1);
            if !valida {
                let detalle: Vec<String> = errores.iter().map(|e| e.mensaje.clone()).collect();
                return Err(format!("funciones.txt no es válido: {}", detalle.join("; ")));
            }
        }
        _ => return Err("el módulo necesita funciones.txt y codigo.rs juntos, o ninguno".to_string()),
    }
    Ok(nombre)
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
