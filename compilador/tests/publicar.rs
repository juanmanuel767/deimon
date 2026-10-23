use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("publicar_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

fn hay_caddy() -> bool {
    Command::new("caddy").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn publicar_con_dominio_prepara_la_carpeta_con_https() {
    if !hay_caddy() {
        eprintln!("se salta: no hay Caddy instalado");
        return;
    }
    let c = carpeta("dominio");
    let programa = c.join("sitio.dmn");
    fs::write(&programa, "ruta \"/\" {\n    devolver \"hola\"\n}\n\nservir(8000)\n").unwrap();
    let salida = c.join("publicar");
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["publicar", programa.to_str().unwrap(), "--dominio", "ejemplo.com", "--salida", salida.to_str().unwrap()])
        .env("TMPDIR", &c)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let caddyfile = fs::read_to_string(salida.join("Caddyfile")).unwrap();
    assert!(caddyfile.contains("ejemplo.com {") && caddyfile.contains("reverse_proxy 127.0.0.1:8000"));
    assert!(salida.join("sitio").is_file() && salida.join("arrancar.sh").is_file());
    let valida = Command::new("caddy")
        .args(["validate", "--adapter", "caddyfile", "--config"])
        .arg(salida.join("Caddyfile"))
        .output()
        .unwrap();
    assert!(valida.status.success(), "{}", String::from_utf8_lossy(&valida.stderr));
}

#[test]
fn publicar_rechaza_un_dominio_que_no_lo_es_y_pide_un_modo() {
    let c = carpeta("mal");
    let programa = c.join("sitio.dmn");
    fs::write(&programa, "ruta \"/\" {\n    devolver \"hola\"\n}\n\nservir(8000)\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["publicar", programa.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("elige --prueba"));
    if hay_caddy() {
        let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
            .args(["publicar", programa.to_str().unwrap(), "--dominio", "no vale"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&o.stderr).contains("no parece un dominio"));
    }
}
