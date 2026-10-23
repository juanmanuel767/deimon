use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn carpeta_sdk() -> Option<PathBuf> {
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(valor) = env::var_os(variable) {
            let ruta = PathBuf::from(valor);
            if ruta.is_dir() {
                return Some(ruta);
            }
        }
    }
    let ruta = PathBuf::from(env::var_os("HOME")?).join("Android").join("Sdk");
    ruta.is_dir().then_some(ruta)
}

fn ultima_version(carpeta: &Path) -> Option<PathBuf> {
    let mut nombres: Vec<String> = fs::read_dir(carpeta)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    nombres.sort_by_key(|n| {
        n.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    });
    nombres.pop().map(|n| carpeta.join(n))
}

fn objetivo_android_instalado() -> bool {
    Command::new("rustc")
        .args(["--print", "target-libdir", "--target", "aarch64-linux-android"])
        .output()
        .map(|salida| salida.status.success() && Path::new(String::from_utf8_lossy(&salida.stdout).trim()).is_dir())
        .unwrap_or(false)
}

#[test]
fn genera_un_apk_de_android_valido() {
    let Some(sdk) = carpeta_sdk() else {
        eprintln!("SALTADA: no hay SDK de Android");
        return;
    };
    let Some(herramientas) = ultima_version(&sdk.join("build-tools")) else {
        eprintln!("SALTADA: no hay build-tools en el SDK");
        return;
    };
    if ultima_version(&sdk.join("ndk")).is_none() {
        eprintln!("SALTADA: no hay NDK en el SDK");
        return;
    }
    if !objetivo_android_instalado() {
        eprintln!("SALTADA: falta el destino aarch64-linux-android de rustup");
        return;
    }

    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ejemplo = raiz.join("..").join("ejemplos-web").join("app.dmn");
    let carpeta = env::temp_dir().join(format!("ln_prueba_apk_{}", std::process::id()));
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    let apk = carpeta.join("prueba.apk");

    let salida = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("app")
        .arg(&ejemplo)
        .args(["--nombre", "Prueba", "--paquete", "com.ejemplo.prueba", "--salida"])
        .arg(&apk)
        .output()
        .expect("no se pudo ejecutar deimon app");
    assert!(
        salida.status.success(),
        "deimon app falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    assert!(apk.is_file(), "no se creó el APK");

    let verificacion = Command::new(herramientas.join("apksigner"))
        .arg("verify")
        .arg(&apk)
        .output()
        .expect("no se pudo ejecutar apksigner");
    assert!(
        verificacion.status.success(),
        "apksigner verify falló: {}",
        String::from_utf8_lossy(&verificacion.stderr)
    );

    let informe = Command::new(herramientas.join("aapt2"))
        .args(["dump", "badging"])
        .arg(&apk)
        .output()
        .expect("no se pudo ejecutar aapt2");
    let texto = String::from_utf8_lossy(&informe.stdout);
    assert!(texto.contains("package: name='com.ejemplo.prueba'"), "{}", texto);
    assert!(texto.contains("minSdkVersion:'24'"), "{}", texto);

    let listado = Command::new("unzip")
        .arg("-l")
        .arg(&apk)
        .output()
        .expect("no se pudo ejecutar unzip");
    let listado = String::from_utf8_lossy(&listado.stdout);
    assert!(listado.contains("lib/arm64-v8a/liblnprograma.so"), "{}", listado);
    assert!(listado.contains("classes.dex"), "{}", listado);

    let extraido = Command::new("unzip")
        .args(["-p"])
        .arg(&apk)
        .arg("lib/arm64-v8a/liblnprograma.so")
        .output()
        .expect("no se pudo extraer la librería");
    let bytes = extraido.stdout;
    assert!(bytes.len() > 20, "la librería está vacía");
    assert_eq!(&bytes[0..4], b"\x7fELF", "no es un ELF");
    assert_eq!(bytes[4], 2, "no es de 64 bits");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183, "no es aarch64");

    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn rechaza_programas_sin_servir() {
    let carpeta = env::temp_dir().join(format!("ln_prueba_sin_servir_{}", std::process::id()));
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    let archivo = carpeta.join("sin.dmn");
    fs::write(&archivo, "mostrar(1)\n").expect("no se pudo escribir");
    let salida = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("app")
        .arg(&archivo)
        .output()
        .expect("no se pudo ejecutar deimon app");
    let _ = fs::remove_dir_all(&carpeta);
    assert!(!salida.status.success());
    assert!(String::from_utf8_lossy(&salida.stderr).contains("servir"));
}
