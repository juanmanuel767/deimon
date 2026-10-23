use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("el compilador siempre está dentro de deimon")
        .to_path_buf()
}

/// Ejecuta un programa de rendimiento con `deimon ejecutar` y devuelve lo que tarda.
fn ejecutar(nombre: &str) -> Duration {
    let programa = raiz().join("rendimiento").join(format!("{}.dmn", nombre));
    let temporales = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp");
    std::fs::create_dir_all(&temporales).expect("no se pudo crear la carpeta temporal");

    let inicio = Instant::now();
    let salida = Command::new(binario())
        .arg("ejecutar")
        .arg(&programa)
        .env("TMPDIR", &temporales)
        .env_remove("DEIMON_PUERTO")
        .env_remove("DEIMON_DATOS")
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'");
    let tiempo = inicio.elapsed();

    assert!(
        salida.status.success(),
        "'{}' falló:\n{}",
        nombre,
        String::from_utf8_lossy(&salida.stderr)
    );
    tiempo
}

#[test]
fn lista_y_texto_terminan_en_menos_de_dos_segundos() {
    for nombre in ["lista", "texto"] {
        let tiempo = ejecutar(nombre);
        assert!(
            tiempo < Duration::from_secs(2),
            "'{}' tardó {:?} (más de 2 s)",
            nombre,
            tiempo
        );
    }
}
