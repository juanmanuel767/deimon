use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const PLANTILLA_MANIFIESTO: &str = include_str!("../plantilla-android/AndroidManifest.xml");
const PLANTILLA_ACTIVIDAD: &str = include_str!("../plantilla-android/MainActivity.java");
/// Icono de la app (la cara del logo de Deimon), 192×192.
const ICONO: &[u8] = include_bytes!("../plantilla-android/icono.png");

type Fallo = (String, i32);

struct Opciones {
    archivo: String,
    nombre: Option<String>,
    paquete: Option<String>,
    salida: Option<String>,
    perfil: deimon::perfil::Perfil,
}

struct Herramientas {
    aapt2: PathBuf,
    d8: PathBuf,
    zipalign: PathBuf,
    apksigner: PathBuf,
    android_jar: PathBuf,
    nivel_api: u32,
    clang: PathBuf,
}

pub fn app(argumentos: &[String]) -> Result<(), Fallo> {
    let opciones = leer_opciones(argumentos)?;
    let ruta_fuente = PathBuf::from(&opciones.archivo);
    let fuente = fs::read_to_string(&ruta_fuente).map_err(|error| {
        (
            format!(
                "Error: no se pudo leer el archivo '{}': {}",
                opciones.archivo, deimon::error::explicar_lectura(&error)
            ),
            1,
        )
    })?;

    let rust = deimon::traducir_archivo_varios_con_perfil(&fuente, &ruta_fuente, opciones.perfil)
        .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;
    if let Some(destino) = crate::destinos::buscar("android-arm64") {
        crate::destinos::comprobar_capacidades(destino, &fuente, &ruta_fuente)?;
    }
    let usa_servir = fuente
        .lines()
        .any(|linea| !linea.trim_start().starts_with('#') && linea.contains("servir("));
    if !usa_servir {
        return Err((
            "Error: el programa no contiene 'servir(...)'; una app necesita al menos una ruta web."
                .to_string(),
            1,
        ));
    }

    // Antes de crear el APK se ejecutan las pruebas del programa en el ordenador.
    let pruebas = crate::probar::correr_archivo(&fuente, &ruta_fuente)
        .map_err(|mensaje| (mensaje, 1))?;
    if pruebas.is_empty() {
        println!("! Aviso: este programa no tiene pruebas.");
    } else {
        crate::probar::imprimir(&pruebas);
        let falladas = pruebas.iter().filter(|p| !p.correcta).count();
        if falladas > 0 {
            return Err((
                format!(
                    "No se crea el APK porque {} {} fallando. Arregla lo que falla y vuelve a intentarlo.",
                    falladas,
                    if falladas == 1 { "prueba está" } else { "pruebas están" }
                ),
                1,
            ));
        }
    }

    let estudio = ruta_fuente
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    let saneado = sanear(&estudio);
    let nombre = opciones.nombre.clone().unwrap_or(estudio);
    let paquete = match &opciones.paquete {
        Some(paquete) => {
            validar_paquete(paquete)?;
            paquete.clone()
        }
        None => format!("com.deimon.{}", saneado),
    };
    let salida = match &opciones.salida {
        Some(ruta) => PathBuf::from(ruta),
        None => ruta_fuente.with_file_name(format!("{}.apk", saneado_archivo(&ruta_fuente))),
    };

    let herramientas = localizar_herramientas()?;
    let trabajo = directorio_trabajo();
    fs::create_dir_all(&trabajo).map_err(|error| {
        (
            format!(
                "Error: no se pudo crear la carpeta temporal '{}': {}",
                trabajo.display(),
                error
            ),
            1,
        )
    })?;
    let resultado = construir(
        &herramientas,
        &trabajo,
        &rust,
        opciones.perfil,
        &nombre,
        &paquete,
        &salida,
    );
    let _ = fs::remove_dir_all(&trabajo);
    resultado?;
    println!("APK creado: {}", salida.display());
    Ok(())
}

fn leer_opciones(argumentos: &[String]) -> Result<Opciones, Fallo> {
    let uso = "Uso: deimon app archivo.dmn [--nombre \"Mi App\"] [--paquete com.ejemplo.miapp] [--salida ruta.apk] [--perfil pequeño|equilibrado|servidor]";
    let mut archivo = None;
    let mut nombre = None;
    let mut paquete = None;
    let mut salida = None;
    let mut perfil = deimon::perfil::Perfil::Equilibrado;
    let mut i = 0;
    while i < argumentos.len() {
        let actual = argumentos[i].as_str();
        match actual {
            "--nombre" | "--paquete" | "--salida" | "--perfil" => {
                i += 1;
                let valor = argumentos.get(i).cloned().ok_or_else(|| {
                    (format!("Error: falta el valor de '{}'.\n{}", actual, uso), 2)
                })?;
                match actual {
                    "--nombre" => nombre = Some(valor),
                    "--paquete" => paquete = Some(valor),
                    "--perfil" => {
                        perfil = deimon::perfil::Perfil::desde_texto(&valor)
                            .ok_or_else(|| deimon::perfil::error_desconocido(&valor))?
                    }
                    _ => salida = Some(valor),
                }
            }
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2));
            }
            otro => {
                if archivo.is_some() {
                    return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2));
                }
                archivo = Some(otro.to_string());
            }
        }
        i += 1;
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;
    Ok(Opciones {
        archivo,
        nombre,
        paquete,
        salida,
        perfil,
    })
}

fn saneado_archivo(ruta: &Path) -> String {
    ruta.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string())
}

/// Deja solo letras minúsculas sin acentos, dígitos y `_`, y que empiece por letra.
fn sanear(texto: &str) -> String {
    let mut resultado: String = texto
        .chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !resultado.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        resultado.insert_str(0, "app_");
    }
    resultado
}

fn validar_paquete(paquete: &str) -> Result<(), Fallo> {
    let partes: Vec<&str> = paquete.split('.').collect();
    let valido = partes.len() >= 2
        && partes.iter().all(|parte| {
            let mut letras = parte.chars();
            letras.next().is_some_and(|c| c.is_ascii_alphabetic())
                && letras.all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    if valido {
        Ok(())
    } else {
        Err((
            format!(
                "Error: el paquete '{}' no es válido. Usa por ejemplo com.ejemplo.miapp (al menos dos partes con letras sin acentos, dígitos o _).",
                paquete
            ),
            2,
        ))
    }
}

fn escapar_xml(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn directorio_trabajo() -> PathBuf {
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("ln_app_{}_{}", process::id(), marca))
}

pub(crate) fn carpeta_sdk() -> Result<PathBuf, Fallo> {
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(valor) = env::var_os(variable) {
            let ruta = PathBuf::from(valor);
            if ruta.is_dir() {
                return Ok(ruta);
            }
        }
    }
    if let Some(casa) = env::var_os("HOME") {
        let ruta = PathBuf::from(casa).join("Android").join("Sdk");
        if ruta.is_dir() {
            return Ok(ruta);
        }
    }
    Err((
        "Error: no se encontró el SDK de Android. Instálalo o indica su carpeta con la variable ANDROID_HOME (por defecto se busca en ~/Android/Sdk).".to_string(),
        1,
    ))
}

pub(crate) fn subcarpetas(ruta: &Path) -> Vec<String> {
    match fs::read_dir(ruta) {
        Ok(entradas) => entradas
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Convierte "34.0.0" o "27.1.12297006" en números para comparar versiones.
pub(crate) fn version(texto: &str) -> Vec<u64> {
    texto
        .split('.')
        .map(|parte| parte.parse::<u64>().unwrap_or(0))
        .collect()
}

/// Versión más nueva de `build-tools` instalada en el SDK.
pub(crate) fn ultimas_build_tools(sdk: &Path) -> Option<String> {
    let mut build_tools = subcarpetas(&sdk.join("build-tools"));
    build_tools.retain(|v| v.chars().all(|c| c.is_ascii_digit() || c == '.'));
    build_tools.sort_by_key(|v| version(v));
    build_tools.pop()
}

/// Solo plataformas estables "android-N" (sin puntos) con android.jar, desde la 35.
pub(crate) fn ultima_plataforma(sdk: &Path) -> Option<u32> {
    let mut niveles: Vec<u32> = subcarpetas(&sdk.join("platforms"))
        .iter()
        .filter_map(|nombre| nombre.strip_prefix("android-")?.parse::<u32>().ok())
        .filter(|&n| n >= 35)
        .filter(|n| {
            sdk.join("platforms")
                .join(format!("android-{}", n))
                .join("android.jar")
                .exists()
        })
        .collect();
    niveles.sort();
    niveles.pop()
}

/// Versión más nueva del NDK instalada en el SDK.
pub(crate) fn ultimo_ndk(sdk: &Path) -> Option<String> {
    let mut ndks = subcarpetas(&sdk.join("ndk"));
    ndks.sort_by_key(|v| version(v));
    ndks.pop()
}

fn localizar_herramientas() -> Result<Herramientas, Fallo> {
    let sdk = carpeta_sdk()?;

    let build_tools = ultimas_build_tools(&sdk).ok_or_else(|| {
        (
            format!(
                "Error: no hay 'build-tools' en el SDK ({}). Instálalas con el gestor del SDK de Android.",
                sdk.join("build-tools").display()
            ),
            1,
        )
    })?;
    let carpeta_bt = sdk.join("build-tools").join(&build_tools);
    let herramienta = |nombre: &str| -> Result<PathBuf, Fallo> {
        let ruta = carpeta_bt.join(nombre);
        if ruta.exists() {
            Ok(ruta)
        } else {
            Err((
                format!("Error: falta '{}' en {}.", nombre, carpeta_bt.display()),
                1,
            ))
        }
    };

    let nivel_api = ultima_plataforma(&sdk).ok_or_else(|| {
        (
            "Error: no hay una plataforma de Android 35 o superior con android.jar en el SDK (carpeta 'platforms')."
                .to_string(),
            1,
        )
    })?;
    let android_jar = sdk
        .join("platforms")
        .join(format!("android-{}", nivel_api))
        .join("android.jar");

    let ndk = ultimo_ndk(&sdk).ok_or_else(|| {
        (
            format!(
                "Error: no hay NDK en el SDK ({}). Instálalo con el gestor del SDK de Android.",
                sdk.join("ndk").display()
            ),
            1,
        )
    })?;
    let clang = sdk
        .join("ndk")
        .join(&ndk)
        .join("toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang");
    if !clang.exists() {
        return Err((
            format!(
                "Error: no se encontró el compilador del NDK: {}",
                clang.display()
            ),
            1,
        ));
    }

    Ok(Herramientas {
        aapt2: herramienta("aapt2")?,
        d8: herramienta("d8")?,
        zipalign: herramienta("zipalign")?,
        apksigner: herramienta("apksigner")?,
        android_jar,
        nivel_api,
        clang,
    })
}

pub(crate) fn existe_en_path(programa: &str) -> bool {
    env::var_os("PATH")
        .map(|rutas| env::split_paths(&rutas).any(|r| r.join(programa).is_file()))
        .unwrap_or(false)
}

fn ejecutar(descripcion: &str, orden: &mut Command) -> Result<(), Fallo> {
    match orden.output() {
        Ok(salida) if salida.status.success() => Ok(()),
        Ok(salida) => {
            let mut texto = String::from_utf8_lossy(&salida.stderr).to_string();
            texto.push_str(&String::from_utf8_lossy(&salida.stdout));
            Err((
                format!("Error: falló {}:\n{}", descripcion, texto.trim()),
                1,
            ))
        }
        Err(error) => Err((
            format!("Error: no se pudo ejecutar {}: {}", descripcion, error),
            1,
        )),
    }
}

fn escribir(ruta: &Path, contenido: &str) -> Result<(), Fallo> {
    fs::write(ruta, contenido).map_err(|error| {
        (
            format!("Error: no se pudo escribir '{}': {}", ruta.display(), error),
            1,
        )
    })
}

fn archivos_con_extension(carpeta: &Path, extension: &str, destino: &mut Vec<PathBuf>) {
    if let Ok(entradas) = fs::read_dir(carpeta) {
        for entrada in entradas.flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                archivos_con_extension(&ruta, extension, destino);
            } else if ruta.extension().is_some_and(|e| e == extension) {
                destino.push(ruta);
            }
        }
    }
}

fn asegurar_clave() -> Result<PathBuf, Fallo> {
    let casa = env::var_os("HOME").ok_or_else(|| {
        (
            "Error: no se pudo localizar la carpeta personal (variable HOME).".to_string(),
            1,
        )
    })?;
    let carpeta = PathBuf::from(casa).join(".deimon");
    let clave = carpeta.join("depuracion.keystore");
    if clave.exists() {
        return Ok(clave);
    }
    fs::create_dir_all(&carpeta).map_err(|error| {
        (
            format!("Error: no se pudo crear '{}': {}", carpeta.display(), error),
            1,
        )
    })?;
    ejecutar(
        "'keytool' (crear la clave de depuración)",
        Command::new("keytool")
            .args(["-genkeypair", "-keystore"])
            .arg(&clave)
            .args([
                "-alias",
                "depuracion",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
                "-storepass",
                "android",
                "-keypass",
                "android",
                "-dname",
                "CN=Deimon Depuracion,O=Deimon,C=ES",
            ]),
    )?;
    Ok(clave)
}

fn construir(
    h: &Herramientas,
    trabajo: &Path,
    rust: &str,
    perfil: deimon::perfil::Perfil,
    nombre: &str,
    paquete: &str,
    salida: &Path,
) -> Result<(), Fallo> {
    for programa in ["javac", "keytool"] {
        if !existe_en_path(programa) {
            return Err((
                format!("Error: no se encontró '{}'. Instala un JDK (OpenJDK 17 o superior).", programa),
                1,
            ));
        }
    }
    if !existe_en_path("zip") && !existe_en_path("jar") {
        return Err((
            "Error: no se encontró ni 'zip' ni 'jar' para empaquetar el APK.".to_string(),
            1,
        ));
    }

    // 1. Compilar el programa para Android.
    let ruta_rust = trabajo.join("programa.rs");
    let ruta_bin = trabajo.join("liblnprograma.so");
    escribir(&ruta_rust, rust)?;
    let mut orden = Command::new("rustc");
    for opcion in perfil.argumentos_rustc() {
        orden.arg(opcion);
    }
    if !perfil.argumentos_rustc().contains(&"strip=symbols") {
        orden.args(["-C", "strip=symbols"]);
    }
    orden
        .args(["--target", "aarch64-linux-android"])
        .arg("-C")
        .arg(format!("linker={}", h.clang.display()))
        .arg(&ruta_rust)
        .arg("-o")
        .arg(&ruta_bin);
    ejecutar(
        "'rustc' para Android (¿está instalado el destino con 'rustup target add aarch64-linux-android'?)",
        &mut orden,
    )?;

    // 2. Manifiesto y actividad a partir de las plantillas.
    let manifiesto = trabajo.join("AndroidManifest.xml");
    escribir(
        &manifiesto,
        &PLANTILLA_MANIFIESTO
            .replace("{{PAQUETE}}", paquete)
            .replace("{{NOMBRE}}", &escapar_xml(nombre)),
    )?;
    let fuentes = trabajo.join("java");
    fs::create_dir_all(&fuentes).map_err(|e| (format!("Error: {}", e), 1))?;
    let ruta_java = fuentes.join("MainActivity.java");
    escribir(&ruta_java, &PLANTILLA_ACTIVIDAD.replace("{{PAQUETE}}", paquete))?;

    // 3. javac -> d8.
    let clases = trabajo.join("clases");
    fs::create_dir_all(&clases).map_err(|e| (format!("Error: {}", e), 1))?;
    ejecutar(
        "'javac'",
        Command::new("javac")
            .args(["--release", "11", "-nowarn", "-cp"])
            .arg(&h.android_jar)
            .arg("-d")
            .arg(&clases)
            .arg(&ruta_java),
    )?;
    let mut archivos_class = Vec::new();
    archivos_con_extension(&clases, "class", &mut archivos_class);
    let dex = trabajo.join("dex");
    fs::create_dir_all(&dex).map_err(|e| (format!("Error: {}", e), 1))?;
    ejecutar(
        "'d8'",
        Command::new(&h.d8)
            .args(["--release", "--min-api", "24", "--lib"])
            .arg(&h.android_jar)
            .arg("--output")
            .arg(&dex)
            .args(&archivos_class),
    )?;

    // 4. aapt2 compile (icono) y aapt2 link -> APK base; se añaden classes.dex y la librería.
    let carpeta_icono = trabajo.join("res").join("mipmap-xxxhdpi");
    fs::create_dir_all(&carpeta_icono).map_err(|e| (format!("Error: {}", e), 1))?;
    fs::write(carpeta_icono.join("ic_launcher.png"), ICONO)
        .map_err(|e| (format!("Error: no se pudo escribir el icono: {}", e), 1))?;
    let recursos = trabajo.join("recursos.zip");
    ejecutar(
        "'aapt2 compile'",
        Command::new(&h.aapt2)
            .arg("compile")
            .arg("--dir")
            .arg(trabajo.join("res"))
            .arg("-o")
            .arg(&recursos),
    )?;
    let base = trabajo.join("base.apk");
    ejecutar(
        "'aapt2 link'",
        Command::new(&h.aapt2)
            .arg("link")
            .arg(&recursos)
            .arg("-I")
            .arg(&h.android_jar)
            .arg("--manifest")
            .arg(&manifiesto)
            .args(["--min-sdk-version", "24", "--target-sdk-version"])
            .arg(h.nivel_api.to_string())
            .args(["--version-code", "1", "--version-name", "1.0", "-o"])
            .arg(&base),
    )?;
    let contenido = trabajo.join("contenido");
    let carpeta_lib = contenido.join("lib").join("arm64-v8a");
    fs::create_dir_all(&carpeta_lib).map_err(|e| (format!("Error: {}", e), 1))?;
    fs::copy(dex.join("classes.dex"), contenido.join("classes.dex"))
        .map_err(|e| (format!("Error: no se encontró classes.dex: {}", e), 1))?;
    fs::copy(&ruta_bin, carpeta_lib.join("liblnprograma.so"))
        .map_err(|e| (format!("Error: no se pudo copiar el programa compilado: {}", e), 1))?;
    if existe_en_path("zip") {
        ejecutar(
            "'zip'",
            Command::new("zip")
                .current_dir(&contenido)
                .arg("-q")
                .arg("-X")
                .arg(&base)
                .args(["classes.dex", "lib/arm64-v8a/liblnprograma.so"]),
        )?;
    } else {
        ejecutar(
            "'jar'",
            Command::new("jar")
                .current_dir(&contenido)
                .arg("uf")
                .arg(&base)
                .args(["classes.dex", "lib/arm64-v8a/liblnprograma.so"]),
        )?;
    }

    // 5. Alinear y firmar.
    let alineado = trabajo.join("alineado.apk");
    ejecutar(
        "'zipalign'",
        Command::new(&h.zipalign)
            .args(["-f", "-p", "4"])
            .arg(&base)
            .arg(&alineado),
    )?;
    let clave = asegurar_clave()?;
    let firmado = trabajo.join("firmado.apk");
    ejecutar(
        "'apksigner'",
        Command::new(&h.apksigner)
            .arg("sign")
            .arg("--ks")
            .arg(&clave)
            .args([
                "--ks-key-alias",
                "depuracion",
                "--ks-pass",
                "pass:android",
                "--key-pass",
                "pass:android",
                "--out",
            ])
            .arg(&firmado)
            .arg(&alineado),
    )?;

    if let Some(padre) = salida.parent() {
        if !padre.as_os_str().is_empty() {
            let _ = fs::create_dir_all(padre);
        }
    }
    fs::copy(&firmado, salida).map_err(|error| {
        (
            format!("Error: no se pudo escribir '{}': {}", salida.display(), error),
            1,
        )
    })?;
    Ok(())
}
