//! Biblioteca estándar: los módulos de `deimon/biblioteca/` van dentro del binario
//! `deimon`, así que funcionan aunque se copie solo. La primera vez que se necesitan se escriben
//! en una carpeta temporal propia de esta versión y se cargan desde allí como cualquier módulo.

use std::env;
use std::fs;
use std::path::PathBuf;

/// (ruta dentro de la biblioteca, contenido).
pub const ARCHIVOS: &[(&str, &str)] = &[
    ("matematicas/modulo.txt", include_str!("../../biblioteca/matematicas/modulo.txt")),
    ("matematicas/funciones.txt", include_str!("../../biblioteca/matematicas/funciones.txt")),
    ("matematicas/codigo.rs", include_str!("../../biblioteca/matematicas/codigo.rs")),
    ("matematicas/enteros.dmn", include_str!("../../biblioteca/matematicas/enteros.dmn")),
    ("archivos/modulo.txt", include_str!("../../biblioteca/archivos/modulo.txt")),
    ("archivos/funciones.txt", include_str!("../../biblioteca/archivos/funciones.txt")),
    ("archivos/codigo.rs", include_str!("../../biblioteca/archivos/codigo.rs")),
    ("excel/modulo.txt", include_str!("../../biblioteca/excel/modulo.txt")),
    ("excel/funciones.txt", include_str!("../../biblioteca/excel/funciones.txt")),
    ("excel/codigo.rs", include_str!("../../biblioteca/excel/codigo.rs")),
    ("graficos/modulo.txt", include_str!("../../biblioteca/graficos/modulo.txt")),
    ("graficos/funciones.txt", include_str!("../../biblioteca/graficos/funciones.txt")),
    ("graficos/codigo.rs", include_str!("../../biblioteca/graficos/codigo.rs")),
    ("json/modulo.txt", include_str!("../../biblioteca/json/modulo.txt")),
    ("json/funciones.txt", include_str!("../../biblioteca/json/funciones.txt")),
    ("json/codigo.rs", include_str!("../../biblioteca/json/codigo.rs")),
    ("sql/modulo.txt", include_str!("../../biblioteca/sql/modulo.txt")),
    ("sql/funciones.txt", include_str!("../../biblioteca/sql/funciones.txt")),
    ("sql/codigo.rs", include_str!("../../biblioteca/sql/codigo.rs")),
    ("whatsapp/modulo.txt", include_str!("../../biblioteca/whatsapp/modulo.txt")),
    ("whatsapp/funciones.txt", include_str!("../../biblioteca/whatsapp/funciones.txt")),
    ("whatsapp/codigo.rs", include_str!("../../biblioteca/whatsapp/codigo.rs")),
    ("whatsapp/dependencias.dmn", include_str!("../../biblioteca/whatsapp/dependencias.dmn")),
    ("mercadopago/modulo.txt", include_str!("../../biblioteca/mercadopago/modulo.txt")),
    ("mercadopago/funciones.txt", include_str!("../../biblioteca/mercadopago/funciones.txt")),
    ("mercadopago/codigo.rs", include_str!("../../biblioteca/mercadopago/codigo.rs")),
    ("mercadopago/dependencias.dmn", include_str!("../../biblioteca/mercadopago/dependencias.dmn")),
    ("paypal/modulo.txt", include_str!("../../biblioteca/paypal/modulo.txt")),
    ("paypal/funciones.txt", include_str!("../../biblioteca/paypal/funciones.txt")),
    ("paypal/codigo.rs", include_str!("../../biblioteca/paypal/codigo.rs")),
    ("paypal/dependencias.dmn", include_str!("../../biblioteca/paypal/dependencias.dmn")),
    ("google/modulo.txt", include_str!("../../biblioteca/google/modulo.txt")),
    ("google/funciones.txt", include_str!("../../biblioteca/google/funciones.txt")),
    ("google/codigo.rs", include_str!("../../biblioteca/google/codigo.rs")),
    ("google/dependencias.dmn", include_str!("../../biblioteca/google/dependencias.dmn")),
    ("mysql/modulo.txt", include_str!("../../biblioteca/mysql/modulo.txt")),
    ("mysql/funciones.txt", include_str!("../../biblioteca/mysql/funciones.txt")),
    ("mysql/codigo.rs", include_str!("../../biblioteca/mysql/codigo.rs")),
    ("correo/modulo.txt", include_str!("../../biblioteca/correo/modulo.txt")),
    ("correo/funciones.txt", include_str!("../../biblioteca/correo/funciones.txt")),
    ("correo/codigo.rs", include_str!("../../biblioteca/correo/codigo.rs")),
    ("pagos/modulo.txt", include_str!("../../biblioteca/pagos/modulo.txt")),
    ("pagos/funciones.txt", include_str!("../../biblioteca/pagos/funciones.txt")),
    ("pagos/codigo.rs", include_str!("../../biblioteca/pagos/codigo.rs")),
    ("pagos/dependencias.dmn", include_str!("../../biblioteca/pagos/dependencias.dmn")),
    ("pantallas/modulo.txt", include_str!("../../biblioteca/pantallas/modulo.txt")),
    ("pantallas/funciones.txt", include_str!("../../biblioteca/pantallas/funciones.txt")),
    ("pantallas/codigo.rs", include_str!("../../biblioteca/pantallas/codigo.rs")),
    ("pdf/modulo.txt", include_str!("../../biblioteca/pdf/modulo.txt")),
    ("pdf/funciones.txt", include_str!("../../biblioteca/pdf/funciones.txt")),
    ("patrones/modulo.txt", include_str!("../../biblioteca/patrones/modulo.txt")),
    ("patrones/funciones.txt", include_str!("../../biblioteca/patrones/funciones.txt")),
    ("patrones/codigo.rs", include_str!("../../biblioteca/patrones/codigo.rs")),
    ("pdf/codigo.rs", include_str!("../../biblioteca/pdf/codigo.rs")),
    ("texto/modulo.txt", include_str!("../../biblioteca/texto/modulo.txt")),
    ("texto/funciones.txt", include_str!("../../biblioteca/texto/funciones.txt")),
    ("texto/codigo.rs", include_str!("../../biblioteca/texto/codigo.rs")),
    ("tiempo/modulo.txt", include_str!("../../biblioteca/tiempo/modulo.txt")),
    ("tiempo/funciones.txt", include_str!("../../biblioteca/tiempo/funciones.txt")),
    ("tiempo/codigo.rs", include_str!("../../biblioteca/tiempo/codigo.rs")),
    ("web/modulo.txt", include_str!("../../biblioteca/web/modulo.txt")),
    ("web/funciones.txt", include_str!("../../biblioteca/web/funciones.txt")),
    ("web/codigo.rs", include_str!("../../biblioteca/web/codigo.rs")),
];

/// Nombres de los módulos de la biblioteca estándar, en orden alfabético.
pub fn nombres() -> Vec<&'static str> {
    let mut nombres: Vec<&str> = ARCHIVOS
        .iter()
        .filter_map(|(ruta, _)| ruta.split('/').next())
        .collect();
    nombres.sort();
    nombres.dedup();
    nombres
}

/// Carpeta donde está escrita la biblioteca estándar (la escribe si todavía no existe).
pub fn carpeta() -> Option<PathBuf> {
    // Un programa empaquetado lee la biblioteca de dentro del propio ejecutable.
    if crate::fuentes::en_paquete() {
        return Some(PathBuf::from(crate::fuentes::RAIZ_BIBLIOTECA));
    }
    let carpeta = env::temp_dir().join(format!("deimon-biblioteca-{:016x}", huella()));
    if carpeta.join(".completa").is_file() {
        return Some(carpeta);
    }
    // Se escribe en una carpeta aparte y se renombra al final para que dos `deimon` a la vez
    // nunca vean una biblioteca a medias.
    let provisional = env::temp_dir().join(format!(
        "deimon-biblioteca-{:016x}-{}",
        huella(),
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&provisional);
    for (ruta, contenido) in ARCHIVOS {
        let destino = provisional.join(ruta);
        fs::create_dir_all(destino.parent()?).ok()?;
        fs::write(&destino, contenido).ok()?;
    }
    fs::write(provisional.join(".completa"), "").ok()?;
    if fs::rename(&provisional, &carpeta).is_err() {
        // Otro `deimon` la escribió antes: vale la suya.
        let _ = fs::remove_dir_all(&provisional);
    }
    carpeta.join(".completa").is_file().then_some(carpeta)
}

/// Huella FNV-1a del contenido, para que cada versión de la biblioteca tenga su carpeta.
fn huella() -> u64 {
    let mut huella: u64 = 0xcbf2_9ce4_8422_2325;
    for (ruta, contenido) in ARCHIVOS {
        for byte in ruta.bytes().chain(contenido.bytes()) {
            huella ^= byte as u64;
            huella = huella.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    huella
}
