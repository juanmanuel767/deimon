#[derive(Debug, Clone, PartialEq)]
pub enum OperadorBinario {
    Suma,
    Resta,
    Multiplicacion,
    Division,
    Resto,
    Igual,
    Distinto,
    Menor,
    MenorIgual,
    Mayor,
    MayorIgual,
    Y,
    O,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperadorUnario {
    Negativo,
    No,
}

/// Trozo de un texto con valores: letras fijas o una expresión entre `{ }`.
#[derive(Debug, Clone, PartialEq)]
pub enum ParteTexto {
    Literal(String),
    Valor(Expresion),
}

impl Expresion {
    /// Baja un texto con valores a la suma equivalente: `"Hola " + a_texto(nombre) + "!"`.
    pub fn bajar_texto_con_valores(partes: &[ParteTexto], linea: usize) -> Expresion {
        let mut suma: Option<Expresion> = None;
        for parte in partes {
            let siguiente = match parte {
                ParteTexto::Literal(texto) => Expresion::Texto(texto.clone()),
                ParteTexto::Valor(valor) => Expresion::Llamada {
                    nombre: "a_texto".to_string(),
                    argumentos: vec![valor.clone()],
                    linea,
                    columna: 0,
                },
            };
            suma = Some(match suma {
                None => match parte {
                    ParteTexto::Literal(_) => siguiente,
                    // Empieza por "" para que la suma sea siempre de texto.
                    ParteTexto::Valor(_) => Expresion::Binaria {
                        izquierda: Box::new(Expresion::Texto(String::new())),
                        operador: OperadorBinario::Suma,
                        derecha: Box::new(siguiente),
                        linea,
                    },
                },
                Some(anterior) => Expresion::Binaria {
                    izquierda: Box::new(anterior),
                    operador: OperadorBinario::Suma,
                    derecha: Box::new(siguiente),
                    linea,
                },
            });
        }
        suma.unwrap_or_else(|| Expresion::Texto(String::new()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expresion {
    Numero(f64),
    /// Entero escrito con cifras que pasa de 2^53 (texto, para conservarlo exacto).
    EnteroGrande(String),
    Texto(String),
    /// Texto `f"Hola {nombre}"`: se convierte en la unión de sus partes (cada valor, como `a_texto`).
    TextoConValores {
        partes: Vec<ParteTexto>,
        linea: usize,
    },
    Logico(bool),
    Nada,
    Lista(Vec<Expresion>),
    /// Lista con claves: `{"nombre": "Ana", "edad": 30}`.
    Mapa(Vec<(Expresion, Expresion)>),
    Variable {
        nombre: String,
        linea: usize,
        columna: usize,
    },
    Llamada {
        nombre: String,
        argumentos: Vec<Expresion>,
        linea: usize,
        columna: usize,
    },
    Unaria {
        operador: OperadorUnario,
        valor: Box<Expresion>,
        linea: usize,
    },
    Binaria {
        izquierda: Box<Expresion>,
        operador: OperadorBinario,
        derecha: Box<Expresion>,
        linea: usize,
    },
    Indice {
        coleccion: Box<Expresion>,
        indice: Box<Expresion>,
        linea: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncionExtension {
    pub nombre: String,
    pub parametros: Vec<String>,
    pub descripcion: String,
    pub ejemplo: String,
    /// Línea de `funciones.txt` donde se declara (para los mensajes de error).
    pub linea_funciones: usize,
}

impl FuncionExtension {
    pub fn firma(&self) -> String {
        format!("{}({})", self.nombre, self.parametros.join(", "))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    pub nombre: String,
    /// Descripción y capacidades (`red`, `archivos`, `hilos`) tomadas de `modulo.txt`.
    pub descripcion: String,
    pub requiere: Vec<String>,
    pub funciones: Vec<FuncionExtension>,
    pub codigo: String,
    /// Línea y columna de la instrucción `usar modulo`.
    pub linea: usize,
    pub columna: usize,
    /// Archivo usado donde aparece la instrucción; `None` si es el principal.
    pub archivo: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sentencia {
    Declarar {
        nombre: String,
        valor: Expresion,
        linea: usize,
        columna: usize,
    },
    Usar {
        ruta: String,
        linea: usize,
        columna: usize,
    },
    UsarExtension {
        nombre: String,
        linea: usize,
        columna: usize,
    },
    Asignar {
        nombre: String,
        valor: Expresion,
        linea: usize,
        columna: usize,
    },
    AsignarIndice {
        nombre: String,
        indices: Vec<Expresion>,
        valor: Expresion,
        linea: usize,
        columna: usize,
    },
    Si {
        condicion: Expresion,
        entonces: Vec<Sentencia>,
        sino: Vec<Sentencia>,
        linea: usize,
    },
    /// `intentar { ... } si falla e { ... }`: captura los errores de ejecución del bloque.
    Intentar {
        cuerpo: Vec<Sentencia>,
        nombre: String,
        captura: Vec<Sentencia>,
        linea: usize,
        columna: usize,
    },
    Mientras {
        condicion: Expresion,
        cuerpo: Vec<Sentencia>,
        linea: usize,
    },
    Para {
        nombre: String,
        coleccion: Expresion,
        cuerpo: Vec<Sentencia>,
        linea: usize,
    },
    Funcion {
        nombre: String,
        parametros: Vec<String>,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
        /// Archivo donde se define cuando viene de un `usar`; `None` en el principal.
        archivo: Option<String>,
    },
    Ruta {
        camino: String,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
        /// Archivo donde se define cuando viene de un `usar`; `None` en el principal.
        archivo: Option<String>,
    },
    Prueba {
        nombre: String,
        cuerpo: Vec<Sentencia>,
        linea: usize,
        columna: usize,
        /// Archivo donde se define cuando viene de un `usar`; `None` en el principal.
        archivo: Option<String>,
    },
    Devolver {
        valor: Option<Expresion>,
        linea: usize,
        columna: usize,
    },
    Romper {
        linea: usize,
        columna: usize,
    },
    Continuar {
        linea: usize,
        columna: usize,
    },
    Expresion(Expresion),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Programa {
    pub sentencias: Vec<Sentencia>,
    pub extensiones: Vec<Extension>,
}
