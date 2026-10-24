//! Código intermedio (bytecode) del intérprete. No depende del núcleo ni del AST: es solo
//! datos, para que el compilador y la máquina virtual se puedan leer por separado.

/// Valor fijo que aparece escrito en el programa.
#[derive(Debug, Clone, PartialEq)]
pub enum Constante {
    Numero(f64),
    Texto(String),
    Logico(bool),
    Nada,
}

/// Integradas que usan las rutas del programa (el generador les añade `rutas_definidas()`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Web {
    /// `visitar(camino)`
    Visitar,
    /// `enviar(camino, datos)`
    Enviar,
    /// `servir(puerto)`
    Servir,
}

/// Operadores de dos valores (sin `y` ni `o`, que no calculan siempre los dos lados).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bin {
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
}

impl Bin {
    pub fn es_comparacion(self) -> bool {
        !matches!(
            self,
            Bin::Suma | Bin::Resta | Bin::Multiplicacion | Bin::Division | Bin::Resto
        )
    }
}

/// Una instrucción. Las variables se nombran por su posición (ranura) dentro del marco de la
/// función; los saltos llevan la posición absoluta de la instrucción de destino.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    /// Apila la constante número `n`.
    Constante(usize),
    /// Apila una copia de la ranura.
    Cargar(usize),
    /// Desapila y guarda en la ranura.
    Guardar(usize),
    /// Desapila y descarta.
    Descartar,
    /// Anota la línea del programa que se está ejecutando (para los mensajes de error).
    EnLinea(usize),

    /// Operación de dos valores. Las variantes con `V` (variable) y `C` (constante) toman ese
    /// operando directamente en vez de apilarlo; el otro es el de arriba de la pila.
    Bin(Bin),
    BinV(Bin, usize),
    BinC(Bin, usize),
    BinVC(Bin, usize, usize),
    BinVV(Bin, usize, usize),
    Negativo,
    NoLogico,
    /// `y`: si el valor desapilado es falso, apila falso y salta; si no, sigue.
    YCorto(usize),
    /// `o`: si el valor desapilado es verdadero, apila verdadero y salta; si no, sigue.
    OCorto(usize),
    /// Desapila un valor y apila su versión lógica (falla si no es verdadero/falso).
    ALogico,

    /// Desapila `n` elementos y apila una lista.
    Lista(usize),
    /// Desapila `n` pares (clave, valor) y apila una lista con claves.
    Mapa(usize),
    /// Desapila posición y colección; apila el elemento.
    Indice,
    /// `nombre[i]... = valor`: desapila el valor y `n` posiciones y cambia la ranura.
    AsignarIndice { ranura: usize, indices: usize },

    Salto(usize),
    SaltoSiFalso(usize),

    /// Entra en un `intentar`: guarda dónde volver y la ranura del error `e`.
    IntentarEntrar { ranura: usize, destino: usize },
    /// Sale de un `intentar` sin error.
    IntentarSalir,
    /// Sale de los `n` `intentar` más internos (para `romper`, `continuar` o `devolver`).
    SalirIntentos(usize),
    /// Comparación y salto en un solo paso: salta si la comparación no se cumple.
    SaltoNo { operacion: Bin, destino: usize },
    SaltoNoC { operacion: Bin, constante: usize, destino: usize },
    SaltoNoVC { operacion: Bin, ranura: usize, constante: usize, destino: usize },
    SaltoNoVV { operacion: Bin, izquierda: usize, derecha: usize, destino: usize },

    /// `para`: desapila la colección y la deja preparada en `ranura` (y el contador en `ranura + 1`).
    ParaIniciar(usize),
    /// Pasa al siguiente elemento de `para` o salta a `fin` si se acabaron.
    ParaSiguiente { ranura: usize, variable: usize, fin: usize },
    /// `para x en rango(a, b)` sin crear la lista: desapila `b` y `a`, comprueba los límites
    /// como `rango()` y deja el contador en `ranura`, `ranura + 1` y `ranura + 2`.
    RangoIniciar { ranura: usize, linea: usize },
    /// Siguiente número del rango, o salta a `fin` si se acabó.
    RangoSiguiente { ranura: usize, variable: usize, fin: usize },

    /// `x = agregar(x, valor)`: añade sin copiar la lista cuando se puede.
    AgregarEnSitio(usize),
    /// `x = x + valor`: suma sin copiar el texto o la lista cuando se puede.
    SumaEnSitio(usize),
    /// Como `SumaEnSitio`, con la constante o la variable indicada como valor a sumar.
    SumaEnSitioC(usize, usize),
    SumaEnSitioV(usize, usize),

    /// Llama a la integrada `indice` de `INTEGRADAS_NUCLEO` (la misma función del núcleo que
    /// usa el modo Rust) con sus `argumentos` apilados.
    Nucleo { indice: usize, argumentos: usize },
    /// `visitar`, `enviar` o `servir` con las rutas del programa; argumentos apilados.
    Web { tipo: Web, argumentos: usize },
    /// `comprobar(condicion)` (`igual: false`) o `comprobar_igual(obtenido, esperado)`
    /// dentro de una `prueba`; `linea` es la de la comprobación.
    Comprobar { igual: bool, linea: usize },
    /// Llama a la función `n` con sus argumentos apilados; `linea` es la de la llamada.
    Llamar { funcion: usize, linea: usize },
    /// Llama a una función de la biblioteca estándar (`indice` en `EXTERNAS`) con sus
    /// `argumentos` apilados; `linea` es la de la llamada.
    Externa { indice: usize, argumentos: usize, linea: usize },
    /// Termina la función devolviendo el valor apilado.
    Devolver,
    /// Termina el programa principal.
    Fin,
}

/// Una función compilada (la posición 0 es el programa principal).
#[derive(Debug, Clone)]
pub struct FuncionCompilada {
    pub nombre: String,
    pub inicio: usize,
    pub parametros: usize,
    /// Ranuras totales del marco (parámetros incluidos).
    pub ranuras: usize,
}

#[derive(Debug, Clone)]
pub struct Codigo {
    pub ops: Vec<Op>,
    pub constantes: Vec<Constante>,
    pub funciones: Vec<FuncionCompilada>,
    /// Rutas web en el orden del programa: (camino, función que la atiende).
    pub rutas: Vec<(String, usize)>,
    /// Pruebas en el orden del programa: (nombre, función con su cuerpo).
    pub pruebas: Vec<(String, usize)>,
    /// Posición de un `Fin` al que vuelve la función de una ruta al terminar.
    pub fin_ruta: usize,
}
