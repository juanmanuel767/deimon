use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ejemplo() -> PathBuf {
    raiz().join("..").join("ejemplos").join("hola.dmn")
}

fn esperado() -> String {
    fs::read_to_string(raiz().join("..").join("ejemplos").join("hola.esperado"))
        .expect("no se pudo leer hola.esperado")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = raiz()
        .join("target")
        .join("tmp")
        .join(format!("destinos_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn compilar(destino: &str, salida: &Path) -> Output {
    Command::new(binario())
        .arg("compilar")
        .arg(ejemplo())
        .args(["--destino", destino, "--salida"])
        .arg(salida)
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'")
}

fn ejecutar(ruta: &Path) -> Output {
    Command::new(ruta)
        .output()
        .expect("no se pudo ejecutar el programa compilado")
}

fn tiene_interprete(bytes: &[u8]) -> bool {
    if bytes.len() < 64 || &bytes[0..4] != b"\x7fELF" {
        return true;
    }
    let phoff = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;
    let phentsize = u16::from_le_bytes(bytes[54..56].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(bytes[56..58].try_into().unwrap()) as usize;
    for i in 0..phnum {
        let inicio = phoff + i * phentsize;
        if inicio + 4 > bytes.len() {
            break;
        }
        let tipo = u32::from_le_bytes(bytes[inicio..inicio + 4].try_into().unwrap());
        if tipo == 3 {
            return true;
        }
    }
    false
}

fn objetivo_instalado(triple: &str) -> bool {
    Command::new("rustc")
        .args(["--print", "target-libdir", "--target", triple])
        .output()
        .map(|salida| {
            salida.status.success()
                && Path::new(String::from_utf8_lossy(&salida.stdout).trim()).is_dir()
        })
        .unwrap_or(false)
}

fn sdk_android() -> Option<PathBuf> {
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

fn ndk_tiene(clang: &str) -> bool {
    let Some(sdk) = sdk_android() else {
        return false;
    };
    let Ok(entradas) = fs::read_dir(sdk.join("ndk")) else {
        return false;
    };
    for entrada in entradas.flatten() {
        let prebuilt = entrada
            .path()
            .join("toolchains")
            .join("llvm")
            .join("prebuilt");
        let Ok(carpetas) = fs::read_dir(prebuilt) else {
            continue;
        };
        for carpeta in carpetas.flatten() {
            if carpeta.path().join("bin").join(clang).is_file() {
                return true;
            }
        }
    }
    false
}

#[test]
fn compila_y_ejecuta_para_linux_x64() {
    let c = carpeta("linux_x64");
    let salida = c.join("hola");
    let o = compilar("linux-x64", &salida);
    assert!(
        o.status.success(),
        "falló linux-x64: {}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let ejecucion = ejecutar(&salida);
    assert!(
        ejecucion.status.success(),
        "el programa linux-x64 falló: {}",
        String::from_utf8_lossy(&ejecucion.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ejecucion.stdout), esperado());
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn compila_un_ejecutable_estatico_para_linux_x64() {
    let c = carpeta("linux_x64_estatico");
    let salida = c.join("hola");
    let o = compilar("linux-x64-estatico", &salida);
    assert!(
        o.status.success(),
        "falló linux-x64-estatico: {}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let ejecucion = ejecutar(&salida);
    assert!(
        ejecucion.status.success(),
        "el programa estático falló: {}",
        String::from_utf8_lossy(&ejecucion.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ejecucion.stdout), esperado());

    let bytes = fs::read(&salida).expect("no se pudo leer el ejecutable estático");
    assert_eq!(&bytes[0..4], b"\x7fELF", "no es un ELF");
    assert_eq!(bytes[4], 2, "no es de 64 bits");
    assert!(!tiene_interprete(&bytes), "el ejecutable estático tiene PT_INTERP");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn compila_para_android_arm64_con_la_cabecera_correcta() {
    if !objetivo_instalado("aarch64-linux-android")
        || !ndk_tiene("aarch64-linux-android24-clang")
    {
        eprintln!("SALTADA: falta el destino aarch64-linux-android o el NDK de Android");
        return;
    }
    let c = carpeta("android_arm64");
    let salida = c.join("hola");
    let o = compilar("android-arm64", &salida);
    assert!(
        o.status.success(),
        "falló android-arm64: {}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let bytes = fs::read(&salida).expect("no se pudo leer el ejecutable de Android");
    assert_eq!(&bytes[0..4], b"\x7fELF", "no es un ELF");
    assert_eq!(bytes[4], 2, "no es de 64 bits");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183, "no es aarch64");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn compila_para_android_arm32_con_la_cabecera_correcta() {
    if !objetivo_instalado("armv7-linux-androideabi")
        || !ndk_tiene("armv7a-linux-androideabi24-clang")
    {
        eprintln!("SALTADA: falta el destino armv7-linux-androideabi o el NDK de Android");
        return;
    }
    let c = carpeta("android_arm32");
    let salida = c.join("hola");
    let o = compilar("android-arm32", &salida);
    assert!(
        o.status.success(),
        "falló android-arm32: {}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let bytes = fs::read(&salida).expect("no se pudo leer el ejecutable de Android");
    assert_eq!(&bytes[0..4], b"\x7fELF", "no es un ELF");
    assert_eq!(bytes[4], 1, "no es de 32 bits");
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 40, "no es armv7");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_destino_desconocido_sugiere_el_parecido() {
    let o = Command::new(binario())
        .arg("compilar")
        .arg(ejemplo())
        .args(["--destino", "linux-x65"])
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");
    assert_eq!(o.status.code(), Some(2));
    let errores = String::from_utf8_lossy(&o.stderr);
    assert!(errores.contains("no existe"), "{}", errores);
    assert!(errores.contains("linux-x64"), "{}", errores);
    assert!(errores.contains("Querías decir 'linux-x64'"), "{}", errores);
}

#[test]
fn un_destino_no_disponible_explica_como_instalarlo() {
    let listado = Command::new(binario())
        .arg("destinos")
        .output()
        .expect("no se pudo ejecutar 'deimon destinos'");
    let texto = String::from_utf8_lossy(&listado.stdout);
    let Some(nombre) = texto
        .lines()
        .find_map(|linea| linea.strip_prefix("✗ ")?.split_whitespace().next())
    else {
        eprintln!("SALTADA: todos los destinos están disponibles");
        return;
    };
    let c = carpeta("no_disponible");
    let salida = c.join("hola");
    let o = compilar(nombre, &salida);
    assert_eq!(o.status.code(), Some(1), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let errores = String::from_utf8_lossy(&o.stderr);
    assert!(errores.contains("no está disponible"), "{}", errores);
    assert!(errores.contains("Instálalo con:"), "{}", errores);
    assert!(!salida.exists(), "se creó un ejecutable que no debía existir");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn destinos_json_es_basico_pero_valido() {
    let o = Command::new(binario())
        .args(["destinos", "--json"])
        .output()
        .expect("no se pudo ejecutar 'deimon destinos --json'");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let texto = String::from_utf8_lossy(&o.stdout).trim().to_string();
    assert!(texto.starts_with('{'), "{}", texto);
    assert!(texto.ends_with('}'), "{}", texto);
    assert!(texto.contains("\"destinos\":["), "{}", texto);
    assert!(texto.contains("\"nombre\":\"linux-x64\""), "{}", texto);
    assert!(texto.contains("\"disponible\":true"), "{}", texto);
    assert_eq!(
        texto.matches('{').count(),
        texto.matches('}').count(),
        "las llaves no están equilibradas: {}",
        texto
    );
}

#[test]
fn un_programa_con_una_prueba_fallida_no_se_compila() {
    let c = carpeta("prueba_fallida");
    let archivo = c.join("roto.dmn");
    fs::write(
        &archivo,
        "prueba \"rota\" {\n    comprobar(falso)\n}\nmostrar(\"hola\")\n",
    )
    .expect("no se pudo escribir el programa de prueba");
    let salida = c.join("roto-linux-x64");
    let o = Command::new(binario())
        .arg("compilar")
        .arg(&archivo)
        .args(["--destino", "linux-x64", "--salida"])
        .arg(&salida)
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");
    assert_eq!(
        o.status.code(),
        Some(1),
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("No se compila"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(!salida.exists(), "se compiló pese a que falló una prueba");
    let _ = fs::remove_dir_all(&c);
}

fn en_el_path(programa: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {}", programa)])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn compila_para_windows_y_funciona_con_wine() {
    if !objetivo_instalado("x86_64-pc-windows-gnu") || !en_el_path("x86_64-w64-mingw32-gcc") {
        eprintln!("se salta: falta el destino de Windows (deimon destinos dice cómo instalarlo)");
        return;
    }
    let c = carpeta("windows");
    let salida = c.join("hola");
    let o = compilar("windows-x64", &salida);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let exe = c.join("hola.exe");
    let bytes = fs::read(&exe).expect("no se creó hola.exe");
    assert_eq!(&bytes[..2], b"MZ", "no es un ejecutable de Windows");
    assert!(String::from_utf8_lossy(&o.stdout).contains("Tamaño del ejecutable: ") && !String::from_utf8_lossy(&o.stdout).contains("desconocido"));
    if en_el_path("wine") {
        let o = Command::new("wine").arg(&exe).env("WINEDEBUG", "-all").output().unwrap();
        assert_eq!(String::from_utf8_lossy(&o.stdout).replace('\r', ""), esperado());
    }
}

#[test]
fn compila_para_raspberry_pi_y_funciona_con_qemu() {
    if !objetivo_instalado("aarch64-unknown-linux-gnu") || !en_el_path("aarch64-linux-gnu-gcc") {
        eprintln!("se salta: falta el destino linux-arm64 (deimon destinos dice cómo instalarlo)");
        return;
    }
    let c = carpeta("arm64");
    let salida = c.join("hola");
    let o = compilar("linux-arm64", &salida);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let bytes = fs::read(&salida).unwrap();
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183, "no es ARM de 64 bits");
    if en_el_path("qemu-aarch64") && Path::new("/usr/aarch64-linux-gnu").is_dir() {
        let o = Command::new("qemu-aarch64").args(["-L", "/usr/aarch64-linux-gnu"]).arg(&salida).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&o.stdout), esperado());
    }
}
