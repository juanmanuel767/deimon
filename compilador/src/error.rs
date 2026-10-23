use std::fmt;

/// Tabla de códigos de error estables (la misma que está en el README y en `deimon guia`).
pub const CODIGOS: &[(&str, &str)] = &[
    ("E001", "un texto entre comillas no está cerrado"),
    ("E002", "carácter, escape o número que Deimon no reconoce"),
    ("E003", "error de sintaxis: se esperaba otra cosa"),
    ("E004", "llave, paréntesis o corchete sin cerrar (o sobrante)"),
    ("E005", "la variable no existe"),
    ("E006", "la función no existe"),
    ("E007", "número de argumentos incorrecto"),
    ("E008", "palabra u operador de otro lenguaje"),
    ("E009", "se asigna a una variable que no se ha declarado"),
    ("E010", "nombre repetido (variable, función, parámetro, ruta o prueba)"),
    ("E011", "sentencia en un lugar donde no se permite"),
    ("E012", "uso incorrecto de una función integrada"),
    ("E013", "ruta web no válida"),
    ("E014", "archivo usado con 'usar' no encontrado"),
    ("E015", "uso circular entre archivos"),
    ("E016", "función o ruta repetida entre archivos"),
    ("E017", "módulo no encontrado"),
    ("E018", "modulo.txt, funciones.txt o codigo.rs de un módulo mal formado o ausente"),
    ("E019", "nombre de módulo no válido"),
    ("E020", "función de un módulo repetida o que choca con otra función"),
    ("E021", "nombre antiguo de una función integrada"),
    ("E022", "palabra reservada usada como nombre"),
    ("E023", "se intenta cambiar un texto por partes"),
    ("E024", "plantilla HTML que no existe o está mal escrita"),
    ("E999", "otro error"),
];

/// Cambio automático seguro: sustituye las columnas `columna_inicio..columna_fin`
/// (se cuenta desde 1; el final no se incluye) de la línea `linea` por `reemplazo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arreglo {
    pub linea: usize,
    pub columna_inicio: usize,
    pub columna_fin: usize,
    pub reemplazo: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorTraduccion {
    pub codigo: &'static str,
    pub linea: usize,
    /// Columna (desde 1) donde empieza el problema; 0 si no se conoce.
    pub columna: usize,
    pub mensaje: String,
    pub sugerencia: Option<String>,
    pub arreglo: Option<Arreglo>,
    /// Texto de la línea del programa donde ocurre el error (se rellena al traducir).
    pub linea_codigo: Option<String>,
    /// Archivo usado donde ocurre el error; `None` si es el archivo principal.
    pub archivo: Option<String>,
}

impl ErrorTraduccion {
    pub fn nuevo(linea: usize, mensaje: impl Into<String>) -> Self {
        ErrorTraduccion {
            codigo: "E999",
            linea,
            columna: 0,
            mensaje: mensaje.into(),
            sugerencia: None,
            arreglo: None,
            linea_codigo: None,
            archivo: None,
        }
    }

    pub fn con_codigo(mut self, codigo: &'static str) -> Self {
        self.codigo = codigo;
        self
    }

    pub fn en_columna(mut self, columna: usize) -> Self {
        self.columna = columna;
        self
    }

    pub fn con_sugerencia(mut self, sugerencia: impl Into<String>) -> Self {
        self.sugerencia = Some(sugerencia.into());
        self
    }

    pub fn con_arreglo(mut self, arreglo: Arreglo) -> Self {
        self.arreglo = Some(arreglo);
        self
    }

    /// Marca que el error ocurre en un archivo usado (no en el principal).
    pub fn con_archivo(mut self, archivo: impl Into<String>) -> Self {
        self.archivo = Some(archivo.into());
        self
    }

    /// Guarda el texto de la línea del programa para poder dibujar el `^`.
    pub fn con_fuente(mut self, fuente: &str) -> Self {
        if self.linea >= 1 {
            self.linea_codigo = fuente
                .split('\n')
                .nth(self.linea - 1)
                .map(|texto| texto.trim_end_matches('\r').to_string());
        }
        self
    }

    /// Solo la primera línea: «Error en la línea N: ...» o, en un archivo usado,
    /// «Error en piezas/calculos.dmn, línea N: ...».
    pub fn primera_linea(&self) -> String {
        match &self.archivo {
            Some(archivo) => format!("Error en {}, línea {}: {}", archivo, self.linea, self.mensaje),
            None => format!("Error en la línea {}: {}", self.linea, self.mensaje),
        }
    }
}

impl fmt::Display for ErrorTraduccion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.primera_linea())?;
        if let Some(texto) = &self.linea_codigo {
            let numero = self.linea.to_string();
            write!(f, "\n  {} | {}", numero, texto)?;
            if self.columna > 0 {
                let relleno: String = texto
                    .chars()
                    .take(self.columna - 1)
                    .map(|c| if c == '\t' { '\t' } else { ' ' })
                    .chain(std::iter::repeat(' ').take(self.columna.saturating_sub(1 + texto.chars().count())))
                    .collect();
                write!(f, "\n  {} | {}^", " ".repeat(numero.len()), relleno)?;
            }
        }
        if let Some(sugerencia) = &self.sugerencia {
            write!(f, "\nSugerencia: {}", sugerencia)?;
        }
        write!(f, "\nCódigo: {}", self.codigo)
    }
}

impl std::error::Error for ErrorTraduccion {}

/// Varios errores, uno detrás de otro, para mostrarlos por pantalla.
pub fn texto_de_errores(errores: &[ErrorTraduccion]) -> String {
    errores
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
