use crate::ast::*;
use crate::error::ErrorTraduccion;
use crate::lexer::{self, Tipo, Token};

pub fn analizar(tokens: Vec<Token>) -> Result<Programa, ErrorTraduccion> {
    let mut analizador = Analizador { tokens, pos: 0, profundidad: 0 };
    let sentencias = analizador.programa()?;
    Ok(Programa {
        sentencias,
        extensiones: Vec::new(),
    })
}

struct Analizador {
    tokens: Vec<Token>,
    pos: usize,
    /// Expresiones y bloques abiertos uno dentro de otro (ver `PROFUNDIDAD_MAXIMA`).
    profundidad: usize,
}

/// Más anidado que esto da un error claro en vez de agotar la pila (5000 paréntesis
/// seguidos tumbaban `deimon` con «stack overflow»).
const PROFUNDIDAD_MAXIMA: usize = 1000;

impl Analizador {
    fn actual(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn tipo(&self) -> &Tipo {
        &self.actual().tipo
    }

    fn linea(&self) -> usize {
        self.actual().linea
    }

    fn avanzar(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn es(&self, tipo: &Tipo) -> bool {
        std::mem::discriminant(self.tipo()) == std::mem::discriminant(tipo)
    }

    fn saltar_nuevas_lineas(&mut self) {
        while matches!(self.tipo(), Tipo::NuevaLinea) {
            self.avanzar();
        }
    }

    fn entrar(&mut self) -> Result<(), ErrorTraduccion> {
        if self.profundidad >= PROFUNDIDAD_MAXIMA {
            return Err(self
                .error(
                    "E003",
                    format!("hay más de {} expresiones o bloques uno dentro de otro", PROFUNDIDAD_MAXIMA),
                )
                .con_sugerencia("Guarda las partes intermedias en variables con 'sea' o sepáralas en funciones"));
        }
        self.profundidad += 1;
        Ok(())
    }

    fn error(&self, codigo: &'static str, mensaje: impl Into<String>) -> ErrorTraduccion {
        ErrorTraduccion::nuevo(self.linea(), mensaje)
            .con_codigo(codigo)
            .en_columna(self.actual().columna)
    }

    fn error_reservada(&self, palabra: &str, token: &Token) -> ErrorTraduccion {
        ErrorTraduccion::nuevo(
            token.linea,
            format!(
                "'{}' es una palabra reservada y no se puede usar como nombre",
                palabra
            ),
        )
        .con_codigo("E022")
        .en_columna(token.columna)
        .con_sugerencia(format!(
            "Elige otro nombre que no sea '{}': es parte del lenguaje",
            palabra
        ))
    }

    fn esperar(&mut self, tipo: &Tipo, descripcion: &str) -> Result<Token, ErrorTraduccion> {
        if self.es(tipo) {
            Ok(self.avanzar())
        } else {
            let error = self.error(
                "E003",
                format!(
                    "se esperaba {} y se encontró {}",
                    descripcion,
                    describir(self.tipo())
                ),
            );
            Err(match tipo {
                Tipo::LLave => error.con_sugerencia(
                    "En Deimon el bloque se abre con '{' en la misma línea que la condición, el nombre o la ruta",
                ),
                Tipo::Asignacion => error.con_sugerencia("Escribe el '=' justo después del nombre"),
                Tipo::En => error.con_sugerencia("La forma es: para elemento en lista { ... }"),
                _ => error,
            })
        }
    }

    /// Como `esperar`, pero para cerrar algo abierto antes: el error dice dónde se abrió.
    fn esperar_cierre(
        &mut self,
        tipo: &Tipo,
        descripcion: &str,
        abierto: &Token,
    ) -> Result<Token, ErrorTraduccion> {
        if self.es(tipo) {
            return Ok(self.avanzar());
        }
        let (apertura, cierre) = match abierto.tipo {
            Tipo::LParen => ('(', ')'),
            Tipo::LCorchete => ('[', ']'),
            _ => ('{', '}'),
        };
        Err(self
            .error(
                "E004",
                format!(
                    "se esperaba {} y se encontró {}; el '{}' se abrió en la línea {}",
                    descripcion,
                    describir(self.tipo()),
                    apertura,
                    abierto.linea
                ),
            )
            .con_sugerencia(format!(
                "Añade '{}' para cerrar el '{}' de la línea {}, columna {}",
                cierre, apertura, abierto.linea, abierto.columna
            )))
    }

    fn programa(&mut self) -> Result<Vec<Sentencia>, ErrorTraduccion> {
        let mut sentencias = Vec::new();
        self.saltar_nuevas_lineas();
        while !matches!(self.tipo(), Tipo::Fin) {
            if matches!(self.tipo(), Tipo::RLave) {
                return Err(self
                    .error("E004", "se encontró '}' sin un bloque abierto")
                    .con_sugerencia(
                        "Sobra esta '}': bórrala o revisa que cada '{' tenga su '}'",
                    ));
            }
            sentencias.push(self.sentencia(false)?);
            self.fin_de_sentencia()?;
        }
        Ok(sentencias)
    }

    fn fin_de_sentencia(&mut self) -> Result<(), ErrorTraduccion> {
        let mut hubo_salto = false;
        while matches!(self.tipo(), Tipo::NuevaLinea) {
            hubo_salto = true;
            self.avanzar();
        }
        if hubo_salto || matches!(self.tipo(), Tipo::Fin | Tipo::RLave) {
            Ok(())
        } else {
            Err(self
                .error(
                    "E003",
                    format!(
                        "se esperaba un salto de línea y se encontró {}",
                        describir(self.tipo())
                    ),
                )
                .con_sugerencia(
                    "Cada sentencia va en su propia línea; separa esta con un salto de línea",
                ))
        }
    }

    fn bloque(&mut self) -> Result<Vec<Sentencia>, ErrorTraduccion> {
        self.entrar()?;
        let resultado = self.bloque_dentro();
        self.profundidad -= 1;
        resultado
    }

    fn bloque_dentro(&mut self) -> Result<Vec<Sentencia>, ErrorTraduccion> {
        let apertura = self.esperar(&Tipo::LLave, "'{'")?;
        let mut sentencias = Vec::new();
        self.saltar_nuevas_lineas();
        while !matches!(self.tipo(), Tipo::RLave) {
            if matches!(self.tipo(), Tipo::Fin) {
                return Err(self
                    .error(
                        "E004",
                        format!(
                            "se esperaba '}}' para cerrar el bloque abierto en la línea {}",
                            apertura.linea
                        ),
                    )
                    .con_sugerencia(format!(
                        "Añade '}}' para cerrar el '{{' de la línea {}, columna {}",
                        apertura.linea, apertura.columna
                    )));
            }
            sentencias.push(self.sentencia(true)?);
            self.fin_de_sentencia()?;
        }
        self.esperar(&Tipo::RLave, "'}'")?;
        Ok(sentencias)
    }

    fn sentencia(&mut self, en_bloque: bool) -> Result<Sentencia, ErrorTraduccion> {
        match self.tipo() {
            Tipo::Sea => self.declaracion(),
            Tipo::Usar if !en_bloque => self.usar(),
            Tipo::Usar => Err(self
                .error("E011", "'usar' solo se puede escribir en el nivel principal")
                .con_sugerencia("Mueve 'usar' fuera de este bloque, al nivel principal")),
            Tipo::Funcion => self.funcion(),
            Tipo::Ruta => self.ruta(),
            Tipo::Prueba => self.prueba(),
            Tipo::Si => self.condicional(),
            Tipo::Intenta => self.intentar_bloque(),
            Tipo::Mientras => self.bucle(),
            Tipo::Para => self.para_bucle(),
            Tipo::Devolver => self.devolucion(),
            Tipo::Romper | Tipo::Continuar => {
                let romper = matches!(self.tipo(), Tipo::Romper);
                let linea = self.linea();
                let columna = self.actual().columna;
                self.avanzar();
                Ok(if romper {
                    Sentencia::Romper { linea, columna }
                } else {
                    Sentencia::Continuar { linea, columna }
                })
            }
            Tipo::Sino => Err(self
                .error("E003", "se encontró 'sino' sin un 'si'")
                .con_sugerencia(
                    "'sino' va justo después de la '}' de un 'si', en la misma línea: } sino { ... }",
                )),
            Tipo::Ident(palabra) if palabra == "tipo" && self.empieza_tipo_propio() => {
                if en_bloque {
                    return Err(self
                        .error("E011", "un tipo solo se puede declarar en el nivel principal")
                        .con_sugerencia("Mueve 'tipo Nombre { ... }' fuera de este bloque, al nivel principal"));
                }
                self.tipo_propio()
            }
            Tipo::Ident(_) => self.expresion_o_asignacion(),
            _ => Ok(Sentencia::Expresion(self.expresion()?)),
        }
    }

    /// `tipo Nombre {`: «tipo» solo es especial así, para no romper variables llamadas tipo.
    fn empieza_tipo_propio(&self) -> bool {
        matches!(self.tokens.get(self.pos + 1).map(|t| &t.tipo), Some(Tipo::Ident(_)))
            && matches!(self.tokens.get(self.pos + 2).map(|t| &t.tipo), Some(Tipo::LLave))
    }

    /// `tipo Persona { nombre: texto  edad: numero  notas }`, un campo por línea y el tipo
    /// opcional. Se convierte en la función `Persona(datos)`, que comprueba los campos con
    /// `crear_con_tipo` y devuelve una lista con claves en el orden de los campos.
    fn tipo_propio(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        const TIPOS_DE_CAMPO: &[&str] = &["numero", "texto", "logico", "lista", "lista_con_claves", "funcion"];
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        let token = self.avanzar();
        let nombre = match &token.tipo {
            Tipo::Ident(nombre) => nombre.clone(),
            _ => unreachable!("empieza_tipo_propio ya miró que hay un nombre"),
        };
        let apertura = self.esperar(&Tipo::LLave, "'{'")?;
        let mut campos: Vec<(String, String, usize)> = Vec::new();
        self.saltar_nuevas_lineas();
        while !matches!(self.tipo(), Tipo::RLave) {
            if matches!(self.tipo(), Tipo::Fin) {
                return Err(self
                    .error("E004", format!("se esperaba '}}' para cerrar el tipo abierto en la línea {}", apertura.linea))
                    .con_sugerencia(format!("Añade '}}' para cerrar el '{{' de la línea {}", apertura.linea)));
            }
            let token = self.avanzar();
            let campo = match &token.tipo {
                Tipo::Ident(campo) => campo.clone(),
                otro => {
                    if let Some(palabra) = palabra_reservada(otro) {
                        return Err(self.error_reservada(palabra, &token));
                    }
                    return Err(ErrorTraduccion::nuevo(token.linea, "se esperaba el nombre de un campo")
                        .con_codigo("E003")
                        .en_columna(token.columna)
                        .con_sugerencia("Escribe un campo por línea: nombre o nombre: texto"));
                }
            };
            if campos.iter().any(|(otro, _, _)| *otro == campo) {
                return Err(ErrorTraduccion::nuevo(token.linea, format!("el campo '{}' está repetido en el tipo '{}'", campo, nombre))
                    .con_codigo("E010")
                    .en_columna(token.columna)
                    .con_sugerencia("Cada campo se escribe una sola vez"));
            }
            let mut tipo_campo = String::new();
            if matches!(self.tipo(), Tipo::DosPuntos) {
                self.avanzar();
                let token = self.avanzar();
                tipo_campo = match &token.tipo {
                    Tipo::Ident(t) if TIPOS_DE_CAMPO.contains(&t.as_str()) => t.clone(),
                    Tipo::Funcion => "funcion".to_string(),
                    _ => {
                        return Err(ErrorTraduccion::nuevo(token.linea, format!("tipo de campo desconocido en '{}'", campo))
                            .con_codigo("E003")
                            .en_columna(token.columna)
                            .con_sugerencia(format!("Usa uno de estos: {} (o ninguno, para aceptar cualquier valor)", TIPOS_DE_CAMPO.join(", "))))
                    }
                };
            }
            campos.push((campo, tipo_campo, token.linea));
            if !matches!(self.tipo(), Tipo::NuevaLinea | Tipo::RLave) {
                return Err(self
                    .error("E003", "después de cada campo va un salto de línea")
                    .con_sugerencia("Escribe un campo por línea: nombre o nombre: texto"));
            }
            self.saltar_nuevas_lineas();
        }
        self.esperar(&Tipo::RLave, "'}'")?;
        if campos.is_empty() {
            return Err(ErrorTraduccion::nuevo(linea, format!("el tipo '{}' no tiene campos", nombre))
                .con_codigo("E003")
                .en_columna(columna)
                .con_sugerencia("Escribe al menos un campo, uno por línea: nombre: texto"));
        }
        let descripcion = Expresion::Lista(
            campos
                .iter()
                .map(|(campo, tipo_campo, _)| {
                    Expresion::Lista(vec![Expresion::Texto(campo.clone()), Expresion::Texto(tipo_campo.clone())])
                })
                .collect(),
        );
        let llamada = Expresion::Llamada {
            nombre: "crear_con_tipo".to_string(),
            argumentos: vec![
                Expresion::Texto(nombre.clone()),
                descripcion,
                Expresion::Variable { nombre: "datos".to_string(), linea, columna },
            ],
            linea,
            columna,
        };
        Ok(Sentencia::Funcion {
            nombre,
            parametros: vec!["datos".to_string()],
            cuerpo: vec![Sentencia::Devolver { valor: Some(llamada), linea: 0, columna }],
            linea,
            columna,
            archivo: None,
        })
    }

    fn usar(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        if matches!(self.tipo(), Tipo::Modulo | Tipo::Extension) {
            self.avanzar();
            return self.usar_modulo(linea, columna);
        }
        let token = self.avanzar();
        let ruta = match token.tipo {
            Tipo::Texto(ruta) => ruta,
            _ => {
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba la ruta del archivo entre comillas",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: usar \"piezas/calculos.dmn\""))
            }
        };
        Ok(Sentencia::Usar {
            ruta,
            linea,
            columna,
        })
    }

    fn usar_modulo(
        &mut self,
        linea: usize,
        columna: usize,
    ) -> Result<Sentencia, ErrorTraduccion> {
        let token = self.avanzar();
        let nombre = match token.tipo {
            Tipo::Texto(nombre) => nombre,
            _ => {
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el nombre del módulo entre comillas",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: usar modulo \"texto\""))
            }
        };
        if !nombre_valido(&nombre) {
            return Err(ErrorTraduccion::nuevo(
                linea,
                format!(
                    "el nombre de módulo '{}' no es válido: solo puede tener letras, dígitos, '_' y '-'",
                    nombre
                ),
            )
            .con_codigo("E019")
            .en_columna(token.columna)
            .con_sugerencia("Usa un nombre como \"texto\""));
        }
        Ok(Sentencia::UsarExtension {
            nombre,
            linea,
            columna,
        })
    }

    fn declaracion(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        self.avanzar();
        let token = self.avanzar();
        let columna = token.columna;
        let nombre = match &token.tipo {
            Tipo::Ident(nombre) => nombre.clone(),
            otro => {
                if let Some(palabra) = palabra_reservada(otro) {
                    return Err(self.error_reservada(palabra, &token));
                }
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el nombre de la variable",
                )
                .con_codigo("E003")
                .en_columna(columna)
                .con_sugerencia("La forma es: sea nombre = valor"));
            }
        };
        self.esperar(
            &Tipo::Asignacion,
            "un '=' después del nombre de la variable",
        )?;
        let valor = self.expresion()?;
        Ok(Sentencia::Declarar {
            nombre,
            valor,
            linea,
            columna,
        })
    }

    /// Una sentencia que empieza por un nombre: una expresión suelta o una asignación
    /// (a una variable o a un elemento de una lista, con uno o varios índices).
    fn expresion_o_asignacion(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let expresion = self.expresion()?;
        if !matches!(self.tipo(), Tipo::Asignacion) {
            return Ok(Sentencia::Expresion(expresion));
        }
        self.avanzar();
        let valor = self.expresion()?;
        match expresion {
            Expresion::Variable {
                nombre,
                linea,
                columna,
            } => Ok(Sentencia::Asignar {
                nombre,
                valor,
                linea,
                columna,
            }),
            Expresion::Indice { .. } => {
                let (nombre, indices, linea, columna) = descomponer_asignacion(expresion)?;
                Ok(Sentencia::AsignarIndice {
                    nombre,
                    indices,
                    valor,
                    linea,
                    columna,
                })
            }
            otra => {
                let linea = match &otra {
                    Expresion::Binaria { linea, .. }
                    | Expresion::Unaria { linea, .. }
                    | Expresion::Llamada { linea, .. } => *linea,
                    _ => self.linea(),
                };
                Err(ErrorTraduccion::nuevo(
                    linea,
                    "solo se puede asignar a una variable o a un elemento de una lista",
                )
                .con_codigo("E003")
                .en_columna(self.actual().columna)
                .con_sugerencia(
                    "Para guardar un valor usa: x = valor, lista[i] = valor o lista[i][j] = valor",
                ))
            }
        }
    }

    fn funcion(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        let token = self.avanzar();
        let nombre = match &token.tipo {
            Tipo::Ident(nombre) => nombre.clone(),
            otro => {
                if let Some(palabra) = palabra_reservada(otro) {
                    return Err(self.error_reservada(palabra, &token));
                }
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el nombre de la función",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: funcion nombre(a, b) { ... }"));
            }
        };
        let apertura = self.esperar(&Tipo::LParen, "'(' después del nombre de la función")?;
        let mut parametros = Vec::new();
        self.saltar_nuevas_lineas();
        if !self.es(&Tipo::RParen) {
            loop {
                let token = self.avanzar();
                let parametro = match &token.tipo {
                    Tipo::Ident(nombre) => nombre.clone(),
                    otro => {
                        if let Some(palabra) = palabra_reservada(otro) {
                            return Err(self.error_reservada(palabra, &token));
                        }
                        return Err(self.error("E003", "se esperaba un nombre de parámetro"));
                    }
                };
                parametros.push(parametro);
                self.saltar_nuevas_lineas();
                if matches!(self.tipo(), Tipo::Coma) {
                    self.avanzar();
                    self.saltar_nuevas_lineas();
                } else {
                    break;
                }
            }
        }
        self.esperar_cierre(&Tipo::RParen, "')' para cerrar los parámetros", &apertura)?;
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Funcion {
            nombre,
            parametros,
            cuerpo,
            linea,
            columna,
            archivo: None,
        })
    }

    fn ruta(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        let token = self.avanzar();
        let camino = match token.tipo {
            Tipo::Texto(camino) => camino,
            _ => {
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el camino de la ruta entre comillas",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: ruta \"/camino\" { ... }"))
            }
        };
        for parte in camino.split('/') {
            if let Some(nombre) = parte.strip_prefix(':') {
                if let Some(palabra) = palabra_reservada_en_texto(nombre) {
                    return Err(ErrorTraduccion::nuevo(
                        linea,
                        format!(
                            "'{}' es una palabra reservada y no se puede usar como nombre",
                            palabra
                        ),
                    )
                    .con_codigo("E022")
                    .en_columna(columna)
                    .con_sugerencia(format!(
                        "Elige otro nombre que no sea '{}': es parte del lenguaje",
                        palabra
                    )));
                }
            }
        }
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Ruta {
            camino,
            cuerpo,
            linea,
            columna,
            archivo: None,
        })
    }

    fn prueba(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        let token = self.avanzar();
        let nombre = match token.tipo {
            Tipo::Texto(nombre) => nombre,
            _ => {
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el nombre de la prueba entre comillas",
                )
                .con_codigo("E003")
                .en_columna(token.columna))
            }
        };
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Prueba {
            nombre,
            cuerpo,
            linea,
            columna,
            archivo: None,
        })
    }

    fn condicional(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let (condicion, entonces, sino) = self.parte_si()?;
        Ok(Sentencia::Si {
            condicion,
            entonces,
            sino,
            linea,
        })
    }

    fn parte_si(
        &mut self,
    ) -> Result<(Expresion, Vec<Sentencia>, Vec<Sentencia>), ErrorTraduccion> {
        self.esperar(&Tipo::Si, "'si'")?;
        let condicion = self.expresion()?;
        self.saltar_nuevas_lineas();
        let entonces = self.bloque()?;
        let marca = self.pos;
        self.saltar_nuevas_lineas();
        let sino = if matches!(self.tipo(), Tipo::Sino) {
            self.avanzar();
            self.saltar_nuevas_lineas();
            if matches!(self.tipo(), Tipo::Si) {
                let linea_si = self.linea();
                let (condicion, entonces, sino) = self.parte_si()?;
                vec![Sentencia::Si {
                    condicion,
                    entonces,
                    sino,
                    linea: linea_si,
                }]
            } else {
                self.bloque()?
            }
        } else {
            self.pos = marca;
            Vec::new()
        };
        Ok((condicion, entonces, sino))
    }

    fn intentar_bloque(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        self.saltar_nuevas_lineas();
        if !matches!(self.tipo(), Tipo::Si) {
            return Err(self
                .error("E003", "después de 'intentar { ... }' falta 'si falla e { ... }'")
                .con_sugerencia(
                    "La forma es: intentar { ... } si falla e { mostrar(e[\"mensaje\"]) }",
                ));
        }
        self.avanzar();
        self.saltar_nuevas_lineas();
        if !matches!(self.tipo(), Tipo::Falla) {
            return Err(self
                .error("E003", "después de 'si' falta 'falla'")
                .con_sugerencia(
                    "Escribe 'si falla e { ... }' para recoger el error en la variable e",
                ));
        }
        self.avanzar();
        let token = self.avanzar();
        let nombre = match &token.tipo {
            Tipo::Ident(nombre) => nombre.clone(),
            otro => {
                if let Some(palabra) = palabra_reservada(otro) {
                    return Err(self.error_reservada(palabra, &token));
                }
                return Err(ErrorTraduccion::nuevo(
                    token.linea,
                    "se esperaba el nombre de la variable del error",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: si falla e { ... }"));
            }
        };
        self.saltar_nuevas_lineas();
        let captura = self.bloque()?;
        Ok(Sentencia::Intentar {
            cuerpo,
            nombre,
            captura,
            linea,
            columna,
        })
    }

    fn bucle(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        self.avanzar();
        let condicion = self.expresion()?;
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Mientras {
            condicion,
            cuerpo,
            linea,
        })
    }

    fn para_bucle(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        self.avanzar();
        let token = self.avanzar();
        let nombre = match &token.tipo {
            Tipo::Ident(nombre) => nombre.clone(),
            otro => {
                if let Some(palabra) = palabra_reservada(otro) {
                    return Err(self.error_reservada(palabra, &token));
                }
                return Err(ErrorTraduccion::nuevo(
                    linea,
                    "se esperaba el nombre de la variable después de 'para'",
                )
                .con_codigo("E003")
                .en_columna(token.columna)
                .con_sugerencia("La forma es: para elemento en lista { ... }"));
            }
        };
        self.esperar(&Tipo::En, "'en' después del nombre de la variable")?;
        let coleccion = self.expresion()?;
        self.saltar_nuevas_lineas();
        let cuerpo = self.bloque()?;
        Ok(Sentencia::Para {
            nombre,
            coleccion,
            cuerpo,
            linea,
        })
    }

    fn devolucion(&mut self) -> Result<Sentencia, ErrorTraduccion> {
        let linea = self.linea();
        let columna = self.actual().columna;
        self.avanzar();
        let valor = if matches!(self.tipo(), Tipo::NuevaLinea | Tipo::Fin | Tipo::RLave) {
            None
        } else {
            Some(self.expresion()?)
        };
        Ok(Sentencia::Devolver {
            valor,
            linea,
            columna,
        })
    }

    fn expresion(&mut self) -> Result<Expresion, ErrorTraduccion> {
        self.entrar()?;
        let resultado = self.expresion_o();
        self.profundidad -= 1;
        resultado
    }

    fn expresion_o(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut izquierda = self.expresion_y()?;
        let mut operadores = 0;
        while matches!(self.tipo(), Tipo::O) {
            let linea = self.linea();
            self.avanzar();
            let derecha = self.expresion_y()?;
            // Cada operador encadenado es un nivel más del árbol: cuenta para el límite.
            self.entrar()?;
            operadores += 1;
            izquierda = Expresion::Binaria {
                izquierda: Box::new(izquierda),
                operador: OperadorBinario::O,
                derecha: Box::new(derecha),
                linea,
            };
        }
        self.profundidad -= operadores;
        Ok(izquierda)
    }

    fn expresion_y(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut izquierda = self.expresion_no()?;
        let mut operadores = 0;
        while matches!(self.tipo(), Tipo::Y) {
            let linea = self.linea();
            self.avanzar();
            let derecha = self.expresion_no()?;
            // Cada operador encadenado es un nivel más del árbol: cuenta para el límite.
            self.entrar()?;
            operadores += 1;
            izquierda = Expresion::Binaria {
                izquierda: Box::new(izquierda),
                operador: OperadorBinario::Y,
                derecha: Box::new(derecha),
                linea,
            };
        }
        self.profundidad -= operadores;
        Ok(izquierda)
    }

    fn expresion_no(&mut self) -> Result<Expresion, ErrorTraduccion> {
        if matches!(self.tipo(), Tipo::No) {
            let linea = self.linea();
            self.avanzar();
            self.entrar()?;
            let valor = self.expresion_no();
            self.profundidad -= 1;
            let valor = valor?;
            return Ok(Expresion::Unaria {
                operador: OperadorUnario::No,
                valor: Box::new(valor),
                linea,
            });
        }
        self.expresion_comparacion()
    }

    fn expresion_comparacion(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut izquierda = self.expresion_suma()?;
        let mut operadores = 0;
        loop {
            let operador = match self.tipo() {
                Tipo::Igual => OperadorBinario::Igual,
                Tipo::Distinto => OperadorBinario::Distinto,
                Tipo::Menor => OperadorBinario::Menor,
                Tipo::MenorIgual => OperadorBinario::MenorIgual,
                Tipo::Mayor => OperadorBinario::Mayor,
                Tipo::MayorIgual => OperadorBinario::MayorIgual,
                _ => break,
            };
            let linea = self.linea();
            self.avanzar();
            let derecha = self.expresion_suma()?;
            // Cada operador encadenado es un nivel más del árbol: cuenta para el límite.
            self.entrar()?;
            operadores += 1;
            izquierda = Expresion::Binaria {
                izquierda: Box::new(izquierda),
                operador,
                derecha: Box::new(derecha),
                linea,
            };
        }
        self.profundidad -= operadores;
        Ok(izquierda)
    }

    fn expresion_suma(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut izquierda = self.expresion_producto()?;
        let mut operadores = 0;
        loop {
            let operador = match self.tipo() {
                Tipo::Mas => OperadorBinario::Suma,
                Tipo::Menos => OperadorBinario::Resta,
                _ => break,
            };
            let linea = self.linea();
            self.avanzar();
            let derecha = self.expresion_producto()?;
            // Cada operador encadenado es un nivel más del árbol: cuenta para el límite.
            self.entrar()?;
            operadores += 1;
            izquierda = Expresion::Binaria {
                izquierda: Box::new(izquierda),
                operador,
                derecha: Box::new(derecha),
                linea,
            };
        }
        self.profundidad -= operadores;
        Ok(izquierda)
    }

    fn expresion_producto(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut izquierda = self.expresion_unaria()?;
        let mut operadores = 0;
        loop {
            let operador = match self.tipo() {
                Tipo::Por => OperadorBinario::Multiplicacion,
                Tipo::Entre => OperadorBinario::Division,
                Tipo::Resto => OperadorBinario::Resto,
                _ => break,
            };
            let linea = self.linea();
            self.avanzar();
            let derecha = self.expresion_unaria()?;
            // Cada operador encadenado es un nivel más del árbol: cuenta para el límite.
            self.entrar()?;
            operadores += 1;
            izquierda = Expresion::Binaria {
                izquierda: Box::new(izquierda),
                operador,
                derecha: Box::new(derecha),
                linea,
            };
        }
        self.profundidad -= operadores;
        Ok(izquierda)
    }

    fn expresion_unaria(&mut self) -> Result<Expresion, ErrorTraduccion> {
        if matches!(self.tipo(), Tipo::Menos) {
            let linea = self.linea();
            self.avanzar();
            self.entrar()?;
            let valor = self.expresion_unaria();
            self.profundidad -= 1;
            let valor = valor?;
            return Ok(Expresion::Unaria {
                operador: OperadorUnario::Negativo,
                valor: Box::new(valor),
                linea,
            });
        }
        self.primaria()
    }

    fn primaria(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let mut expresion = self.primaria_base()?;
        while matches!(self.tipo(), Tipo::LCorchete) {
            let linea = self.linea();
            let apertura = self.avanzar();
            self.saltar_nuevas_lineas();
            let indice = self.expresion()?;
            self.saltar_nuevas_lineas();
            self.esperar_cierre(&Tipo::RCorchete, "']' para cerrar el índice", &apertura)?;
            expresion = Expresion::Indice {
                coleccion: Box::new(expresion),
                indice: Box::new(indice),
                linea,
            };
        }
        Ok(expresion)
    }

    fn primaria_base(&mut self) -> Result<Expresion, ErrorTraduccion> {
        let token = self.avanzar();
        match token.tipo {
            Tipo::Numero(numero) => Ok(Expresion::Numero(numero)),
            Tipo::EnteroGrande(cifras) => Ok(Expresion::EnteroGrande(cifras)),
            Tipo::Texto(texto) => Ok(Expresion::Texto(texto)),
            Tipo::TextoEspecial { partes, .. } => {
                if let [lexer::ParteTexto::Literal(texto)] = partes.as_slice() {
                    return Ok(Expresion::Texto(texto.clone()));
                }
                let mut convertidas = Vec::new();
                for parte in partes {
                    convertidas.push(match parte {
                        lexer::ParteTexto::Literal(texto) => ParteTexto::Literal(texto),
                        lexer::ParteTexto::Valor(tokens) => {
                            let mut interior = Analizador { tokens, pos: 0, profundidad: self.profundidad };
                            let valor = interior.expresion()?;
                            if !matches!(interior.tipo(), Tipo::Fin) {
                                return Err(interior.error(
                                    "E003",
                                    format!(
                                        "dentro de las llaves del texto sobra {}",
                                        describir(interior.tipo())
                                    ),
                                )
                                .con_sugerencia(
                                    "Dentro de { } va una sola expresión, por ejemplo {nombre} o {edad + 1}",
                                ));
                            }
                            ParteTexto::Valor(valor)
                        }
                    });
                }
                Ok(Expresion::TextoConValores {
                    partes: convertidas,
                    linea: token.linea,
                })
            }
            Tipo::Verdadero => Ok(Expresion::Logico(true)),
            Tipo::Falso => Ok(Expresion::Logico(false)),
            Tipo::Nada => Ok(Expresion::Nada),
            Tipo::LCorchete => {
                let apertura = token.clone();
                let mut elementos = Vec::new();
                self.saltar_nuevas_lineas();
                if !self.es(&Tipo::RCorchete) {
                    loop {
                        elementos.push(self.expresion()?);
                        self.saltar_nuevas_lineas();
                        if matches!(self.tipo(), Tipo::Coma) {
                            self.avanzar();
                            self.saltar_nuevas_lineas();
                        } else {
                            break;
                        }
                    }
                }
                self.esperar_cierre(&Tipo::RCorchete, "']' para cerrar la lista", &apertura)?;
                Ok(Expresion::Lista(elementos))
            }
            Tipo::LLave => {
                let apertura = token.clone();
                let mut pares = Vec::new();
                self.saltar_nuevas_lineas();
                if !self.es(&Tipo::RLave) {
                    loop {
                        let clave = self.expresion()?;
                        self.saltar_nuevas_lineas();
                        self.esperar(&Tipo::DosPuntos, "':' entre la clave y su valor")?;
                        self.saltar_nuevas_lineas();
                        let valor = self.expresion()?;
                        pares.push((clave, valor));
                        self.saltar_nuevas_lineas();
                        if matches!(self.tipo(), Tipo::Coma) {
                            self.avanzar();
                            self.saltar_nuevas_lineas();
                        } else {
                            break;
                        }
                    }
                }
                self.esperar_cierre(&Tipo::RLave, "'}' para cerrar la lista con claves", &apertura)?;
                Ok(Expresion::Mapa(pares))
            }
            Tipo::Ident(nombre) => {
                if matches!(self.tipo(), Tipo::LParen) {
                    let argumentos = self.argumentos()?;
                    Ok(Expresion::Llamada {
                        nombre,
                        argumentos,
                        linea: token.linea,
                        columna: token.columna,
                    })
                } else {
                    Ok(Expresion::Variable {
                        nombre,
                        linea: token.linea,
                        columna: token.columna,
                    })
                }
            }
            Tipo::LParen => {
                self.saltar_nuevas_lineas();
                let expresion = self.expresion()?;
                self.saltar_nuevas_lineas();
                self.esperar_cierre(&Tipo::RParen, "')' para cerrar el paréntesis", &token)?;
                Ok(expresion)
            }
            ref otro => {
                if let Some(palabra) = palabra_reservada(otro) {
                    return Err(self.error_reservada(palabra, &token));
                }
                Err(ErrorTraduccion::nuevo(
                    token.linea,
                    format!("se esperaba una expresión y se encontró {}", describir(otro)),
                )
                .con_codigo("E003")
                .en_columna(token.columna))
            }
        }
    }

    fn argumentos(&mut self) -> Result<Vec<Expresion>, ErrorTraduccion> {
        let apertura = self.esperar(&Tipo::LParen, "'('")?;
        let mut argumentos = Vec::new();
        self.saltar_nuevas_lineas();
        if !self.es(&Tipo::RParen) {
            loop {
                argumentos.push(self.expresion()?);
                self.saltar_nuevas_lineas();
                if matches!(self.tipo(), Tipo::Coma) {
                    self.avanzar();
                    self.saltar_nuevas_lineas();
                } else {
                    break;
                }
            }
        }
        self.esperar_cierre(&Tipo::RParen, "')' para cerrar los argumentos", &apertura)?;
        Ok(argumentos)
    }
}

fn palabra_reservada_en_texto(nombre: &str) -> Option<&'static str> {
    if nombre.is_empty() {
        return None;
    }
    let tokens = crate::lexer::tokenizar(nombre).ok()?;
    tokens.first().and_then(|token| palabra_reservada(&token.tipo))
}

fn palabra_reservada(tipo: &Tipo) -> Option<&'static str> {
    match tipo {
        Tipo::Sea => Some("sea"),
        Tipo::Usar => Some("usar"),
        Tipo::Funcion => Some("funcion"),
        Tipo::Ruta => Some("ruta"),
        Tipo::Prueba => Some("prueba"),
        Tipo::Extension => Some("extension"),
        Tipo::Modulo => Some("modulo"),
        Tipo::Devolver => Some("devolver"),
        Tipo::Romper => Some("romper"),
        Tipo::Continuar => Some("continuar"),
        Tipo::Si => Some("si"),
        Tipo::Sino => Some("sino"),
        Tipo::Intenta => Some("intentar"),
        Tipo::Falla => Some("falla"),
        Tipo::Mientras => Some("mientras"),
        Tipo::Para => Some("para"),
        Tipo::En => Some("en"),
        Tipo::No => Some("no"),
        Tipo::Verdadero => Some("verdadero"),
        Tipo::Falso => Some("falso"),
        Tipo::Nada => Some("nada"),
        _ => None,
    }
}

/// Convierte `a[i]` o `a[i][j]` en el nombre de la variable y sus índices.
fn descomponer_asignacion(
    expresion: Expresion,
) -> Result<(String, Vec<Expresion>, usize, usize), ErrorTraduccion> {
    match expresion {
        Expresion::Indice {
            coleccion,
            indice,
            linea,
        } => match *coleccion {
            Expresion::Variable {
                nombre, columna, ..
            } => Ok((nombre, vec![*indice], linea, columna)),
            Expresion::Indice { .. } => {
                let (nombre, mut indices, _, columna) = descomponer_asignacion(*coleccion)?;
                indices.push(*indice);
                Ok((nombre, indices, linea, columna))
            }
            _ => Err(ErrorTraduccion::nuevo(
                linea,
                "solo se puede asignar por índice a una variable de lista",
            )
            .con_codigo("E003")
            .con_sugerencia(
                "Escribe el nombre de la lista antes del corchete: lista[i] = valor",
            )),
        },
        _ => Err(ErrorTraduccion::nuevo(
            0,
            "solo se puede asignar a una variable o a un elemento de una lista",
        )
        .con_codigo("E003")),
    }
}

fn nombre_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && nombre
            .chars()
            .all(|caracter| caracter.is_alphabetic() || caracter.is_numeric() || caracter == '_' || caracter == '-')
}

fn describir(tipo: &Tipo) -> String {
    match tipo {
        Tipo::Numero(n) => format!("el número {}", n),
        Tipo::EnteroGrande(cifras) => format!("el número {}", cifras),
        Tipo::Texto(t) => format!("el texto \"{}\"", t),
        Tipo::TextoEspecial { crudo, .. } => format!("el texto {}", crudo),
        Tipo::Comentario(t) => format!("el comentario '{}'", t),
        Tipo::Ident(nombre) => format!("el identificador '{}'", nombre),
        Tipo::Ajeno(texto) => format!("'{}' (de otro lenguaje)", texto),
        Tipo::Sea => "'sea'".to_string(),
        Tipo::Usar => "'usar'".to_string(),
        Tipo::Funcion => "'funcion'".to_string(),
        Tipo::Ruta => "'ruta'".to_string(),
        Tipo::Prueba => "'prueba'".to_string(),
        Tipo::Extension => "'extension'".to_string(),
        Tipo::Modulo => "'modulo'".to_string(),
        Tipo::Devolver => "'devolver'".to_string(),
        Tipo::Romper => "'romper'".to_string(),
        Tipo::Continuar => "'continuar'".to_string(),
        Tipo::Si => "'si'".to_string(),
        Tipo::Sino => "'sino'".to_string(),
        Tipo::Intenta => "'intentar'".to_string(),
        Tipo::Falla => "'falla'".to_string(),
        Tipo::Mientras => "'mientras'".to_string(),
        Tipo::Para => "'para'".to_string(),
        Tipo::En => "'en'".to_string(),
        Tipo::Y => "'y'".to_string(),
        Tipo::O => "'o'".to_string(),
        Tipo::No => "'no'".to_string(),
        Tipo::Verdadero => "'verdadero'".to_string(),
        Tipo::Falso => "'falso'".to_string(),
        Tipo::Nada => "'nada'".to_string(),
        Tipo::Mas => "'+'".to_string(),
        Tipo::Menos => "'-'".to_string(),
        Tipo::Por => "'*'".to_string(),
        Tipo::Entre => "'/'".to_string(),
        Tipo::Resto => "'%'".to_string(),
        Tipo::Igual => "'=='".to_string(),
        Tipo::Distinto => "'!='".to_string(),
        Tipo::Menor => "'<'".to_string(),
        Tipo::MenorIgual => "'<='".to_string(),
        Tipo::Mayor => "'>'".to_string(),
        Tipo::MayorIgual => "'>='".to_string(),
        Tipo::Asignacion => "'='".to_string(),
        Tipo::LParen => "'('".to_string(),
        Tipo::RParen => "')'".to_string(),
        Tipo::LLave => "'{'".to_string(),
        Tipo::RLave => "'}'".to_string(),
        Tipo::LCorchete => "'['".to_string(),
        Tipo::RCorchete => "']'".to_string(),
        Tipo::Coma => "','".to_string(),
        Tipo::DosPuntos => "':'".to_string(),
        Tipo::NuevaLinea => "un salto de línea".to_string(),
        Tipo::Fin => "el final del archivo".to_string(),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::lexer::tokenizar;

    fn analizar_fuente(fuente: &str) -> Result<Programa, ErrorTraduccion> {
        analizar(tokenizar(fuente)?)
    }

    #[test]
    fn analiza_una_funcion_recursiva() {
        let programa =
            analizar_fuente("funcion f(n) {\n  si n <= 1 {\n    devolver n\n  }\n  devolver f(n - 1)\n}\n")
                .unwrap();
        assert_eq!(programa.sentencias.len(), 1);
        match &programa.sentencias[0] {
            Sentencia::Funcion {
                nombre,
                parametros,
                cuerpo,
                ..
            } => {
                assert_eq!(nombre, "f");
                assert_eq!(parametros, &vec!["n".to_string()]);
                assert_eq!(cuerpo.len(), 2);
            }
            otra => panic!("se esperaba una función, se encontró {:?}", otra),
        }
    }

    #[test]
    fn respeta_la_precedencia_de_operadores() {
        let programa = analizar_fuente("mostrar(1 + 2 * 3)\n").unwrap();
        let expresion = match &programa.sentencias[0] {
            Sentencia::Expresion(Expresion::Llamada { argumentos, .. }) => &argumentos[0],
            otra => panic!("se esperaba una llamada, se encontró {:?}", otra),
        };
        match expresion {
            Expresion::Binaria {
                operador: OperadorBinario::Suma,
                derecha,
                ..
            } => {
                assert!(matches!(
                    **derecha,
                    Expresion::Binaria {
                        operador: OperadorBinario::Multiplicacion,
                        ..
                    }
                ));
            }
            otra => panic!("se esperaba una suma, se encontró {:?}", otra),
        }
    }

    #[test]
    fn analiza_la_cadena_sino_si() {
        let programa = analizar_fuente(
            "si verdadero {\n  mostrar(1)\n} sino si falso {\n  mostrar(2)\n} sino {\n  mostrar(3)\n}\n",
        )
        .unwrap();
        match &programa.sentencias[0] {
            Sentencia::Si { sino, .. } => {
                assert_eq!(sino.len(), 1);
                assert!(matches!(sino[0], Sentencia::Si { .. }));
            }
            otra => panic!("se esperaba un condicional, se encontró {:?}", otra),
        }
    }

    #[test]
    fn informa_una_llave_sin_cerrar() {
        let error = analizar_fuente("si verdadero {\n  mostrar(1)\n").unwrap_err();
        assert_eq!(error.linea, 3);
        assert!(error.mensaje.contains("se esperaba '}'"));
    }

    #[test]
    fn analiza_una_ruta_con_parametro() {
        let programa =
            analizar_fuente("ruta \"/saludo/:nombre\" {\n  devolver \"Hola\"\n}\n").unwrap();
        match &programa.sentencias[0] {
            Sentencia::Ruta {
                camino, cuerpo, ..
            } => {
                assert_eq!(camino, "/saludo/:nombre");
                assert_eq!(cuerpo.len(), 1);
            }
            otra => panic!("se esperaba una ruta, se encontró {:?}", otra),
        }
    }

    #[test]
    fn informa_una_ruta_duplicada() {
        let fuente = "ruta \"/x\" {\n  devolver \"a\"\n}\nruta \"/x\" {\n  devolver \"b\"\n}\nservir(8080)\n";
        let error = crate::traducir(fuente).unwrap_err();
        assert_eq!(error.linea, 4);
        assert!(error.mensaje.contains("la ruta '/x' ya existe"));
    }

    #[test]
    fn informa_servir_sin_rutas() {
        let error = crate::traducir("servir(8080)\n").unwrap_err();
        assert_eq!(error.linea, 1);
        assert!(error.mensaje.contains("sin rutas"));
    }

    #[test]
    fn analiza_una_prueba() {
        let programa = analizar_fuente(
            "prueba \"suma básica\" {\n  comprobar(1 == 1)\n  comprobar_igual(2, 2)\n}\n",
        )
        .unwrap();
        match &programa.sentencias[0] {
            Sentencia::Prueba { nombre, cuerpo, .. } => {
                assert_eq!(nombre, "suma básica");
                assert_eq!(cuerpo.len(), 2);
            }
            otra => panic!("se esperaba una prueba, se encontró {:?}", otra),
        }
    }

    #[test]
    fn prueba_y_extension_son_palabras_reservadas() {
        for fuente in ["sea prueba = 1\n", "sea extension = 1\n"] {
            let error = analizar_fuente(fuente).unwrap_err();
            assert_eq!(error.codigo, "E022");
            assert!(error.mensaje.contains("palabra reservada"));
        }
    }

    #[test]
    fn informa_una_prueba_duplicada() {
        let fuente = "prueba \"a\" {\n  comprobar(verdadero)\n}\nprueba \"a\" {\n  comprobar(verdadero)\n}\n";
        let error = crate::traducir(fuente).unwrap_err();
        assert_eq!(error.linea, 4);
        assert!(error.mensaje.contains("la prueba 'a' ya existe"));
    }

    #[test]
    fn analiza_una_lista_literal() {
        let programa = analizar_fuente("sea cosas = [1, \"a\", verdadero]\n").unwrap();
        match &programa.sentencias[0] {
            Sentencia::Declarar {
                valor: Expresion::Lista(elementos),
                ..
            } => {
                assert_eq!(elementos.len(), 3);
                assert_eq!(elementos[0], Expresion::Numero(1.0));
                assert_eq!(elementos[1], Expresion::Texto("a".to_string()));
                assert_eq!(elementos[2], Expresion::Logico(true));
            }
            otra => panic!("se esperaba una lista, se encontró {:?}", otra),
        }
    }

    #[test]
    fn analiza_un_indice() {
        let programa = analizar_fuente("mostrar(cosas[0])\n").unwrap();
        match &programa.sentencias[0] {
            Sentencia::Expresion(Expresion::Llamada { argumentos, .. }) => {
                assert!(matches!(
                    &argumentos[0],
                    Expresion::Indice { indice, .. }
                        if **indice == Expresion::Numero(0.0)
                ));
            }
            otra => panic!("se esperaba una llamada, se encontró {:?}", otra),
        }
    }

    #[test]
    fn analiza_un_bucle_para() {
        let programa = analizar_fuente("para valor en [1, 2] {\n  mostrar(valor)\n}\n").unwrap();
        match &programa.sentencias[0] {
            Sentencia::Para {
                nombre,
                coleccion,
                cuerpo,
                ..
            } => {
                assert_eq!(nombre, "valor");
                assert!(matches!(coleccion, Expresion::Lista(elementos) if elementos.len() == 2));
                assert_eq!(cuerpo.len(), 1);
            }
            otra => panic!("se esperaba un bucle 'para', se encontró {:?}", otra),
        }
    }
}
