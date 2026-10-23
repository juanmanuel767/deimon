use std::fs;
use std::path::{Path, PathBuf};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Devuelve los destinos de los enlaces markdown `[texto](destino)` de un documento.
fn enlaces(texto: &str) -> Vec<String> {
    let mut encontrados = Vec::new();
    let mut resto = texto;
    while let Some(inicio) = resto.find("](") {
        resto = &resto[inicio + 2..];
        if let Some(fin) = resto.find(')') {
            encontrados.push(resto[..fin].to_string());
            resto = &resto[fin..];
        }
    }
    encontrados
}

fn comprobar_documento(ruta: &Path, rotos: &mut Vec<String>) {
    let texto = fs::read_to_string(ruta).expect("no se pudo leer el documento");
    let carpeta = ruta.parent().expect("el documento no tiene carpeta");
    for destino in enlaces(&texto) {
        if destino.starts_with("http") || destino.starts_with('#') || destino.starts_with("mailto:") {
            continue;
        }
        let sin_ancla = destino.split('#').next().unwrap_or("");
        if !sin_ancla.is_empty() && !carpeta.join(sin_ancla).exists() {
            rotos.push(format!("{} -> {}", ruta.display(), destino));
        }
    }
}

#[test]
fn los_enlaces_de_la_documentacion_apuntan_a_archivos_que_existen() {
    let mut documentos = vec![raiz().join("README.md")];
    for entrada in fs::read_dir(raiz().join("docs")).expect("no existe la carpeta docs") {
        let ruta = entrada.expect("entrada ilegible").path();
        if ruta.extension().map(|e| e == "md").unwrap_or(false) {
            documentos.push(ruta);
        }
    }
    let mut rotos = Vec::new();
    for documento in &documentos {
        comprobar_documento(documento, &mut rotos);
    }
    assert!(rotos.is_empty(), "enlaces rotos:\n{}", rotos.join("\n"));
}
