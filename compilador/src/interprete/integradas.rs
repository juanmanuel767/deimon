// Integradas del núcleo para el intérprete. Este archivo se incluye (include!) dentro del
// módulo `motor` de `mod.rs`, junto al núcleo. `build.rs` genera `INTEGRADAS_NUCLEO` a partir
// de `src/integradas.rs`: cada entrada llama a la MISMA función del núcleo que el modo Rust
// (`nombre(&a, &b, ...)`), así que no hay que tocar nada aquí al añadir una integrada.

include!(concat!(env!("OUT_DIR"), "/integradas_interprete.rs"));

/// Posición en `INTEGRADAS_NUCLEO` y número de argumentos de una integrada.
pub fn integrada_del_nucleo(nombre: &str) -> Option<(usize, usize)> {
    INTEGRADAS_NUCLEO
        .iter()
        .position(|(n, _, _)| *n == nombre)
        .map(|posicion| (posicion, INTEGRADAS_NUCLEO[posicion].1))
}
