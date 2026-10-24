// Biblioteca estándar dentro del intérprete. Este archivo se incluye (include!) dentro del
// módulo `motor` de `mod.rs`, donde ya están el núcleo (`Valor`, `fallo`, `en_linea`...):
// así los `codigo.rs` de `biblioteca/` son EXACTAMENTE los mismos que pega el generador en
// el modo Rust (una sola fuente de verdad) y se llaman directamente.
//
// `build.rs` genera, a partir de `biblioteca/*/funciones.txt`, los include! de cada
// codigo.rs, `CODIGOS_ESTANDAR` y la tabla `EXTERNAS` (nombre → `ext_<nombre>`).
//
// Quedan fuera (siguen en el modo Rust): `sql` (enlaza SQLite del sistema) y `pantallas`
// (Android). Los módulos propios o de la comunidad se detectan comparando su código con el
// de aquí (`es_estandar`).

type FuncionExterna = fn(Vec<Valor>) -> Result<Valor, String>;

include!(concat!(env!("OUT_DIR"), "/biblioteca_interprete.rs"));

/// ¿Es `codigo` el del módulo estándar `nombre`? (Un módulo propio con el mismo nombre no lo es.)
pub fn es_estandar(nombre: &str, codigo: &str) -> bool {
    CODIGOS_ESTANDAR.iter().any(|(n, c)| *n == nombre && *c == codigo)
}

/// Posición de la función externa en `EXTERNAS`.
pub fn indice_externa(nombre: &str) -> Option<usize> {
    EXTERNAS.iter().position(|(n, _)| *n == nombre)
}

/// Llama a la función externa `indice` con la línea anotada, como hace el modo Rust.
fn llamar_externa(indice: usize, linea: usize, argumentos: Vec<Valor>) -> Valor {
    en_linea(linea);
    match (EXTERNAS[indice].1)(argumentos) {
        Ok(valor) => valor,
        Err(mensaje) => fallo(mensaje),
    }
}
