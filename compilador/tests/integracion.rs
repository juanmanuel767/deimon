use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta_ejemplos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ejemplos")
}

fn carpeta_datos(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    carpeta
}

fn ejecutar_con_datos(ruta: &PathBuf, datos: &PathBuf) -> Output {
    Command::new(binario())
        .arg("ejecutar")
        .arg(ruta)
        .env("DEIMON_DATOS", datos)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn ejecutar(ruta: &PathBuf) -> Output {
    Command::new(binario())
        .arg("ejecutar")
        .arg(ruta)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn archivo_temporal(nombre: &str, contenido: &str) -> PathBuf {
    let carpeta = std::env::temp_dir();
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    let ruta = carpeta.join(format!("ln_prueba_{}_{}.dmn", std::process::id(), nombre));
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

#[test]
fn los_ejemplos_producen_la_salida_esperada() {
    let datos = carpeta_datos("ejemplos");
    let mut ejemplos: Vec<PathBuf> = fs::read_dir(carpeta_ejemplos())
        .expect("no se pudo leer la carpeta de ejemplos")
        .filter_map(|entrada| entrada.ok())
        .filter(|entrada| {
            entrada
                .file_type()
                .map(|tipo| tipo.is_file())
                .unwrap_or(false)
        })
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.extension().map_or(false, |extension| extension == "dmn"))
        .collect();
    ejemplos.sort();

    for ruta in ejemplos {
        let nombre = ruta
            .file_stem()
            .map(|nombre| nombre.to_string_lossy().to_string())
            .unwrap_or_default();
        let esperado = fs::read_to_string(carpeta_ejemplos().join(format!("{}.esperado", nombre)))
            .unwrap_or_else(|_| panic!("no se pudo leer la salida esperada de '{}'", nombre));

        let salida = ejecutar_con_datos(&ruta, &datos);
        let texto = String::from_utf8_lossy(&salida.stdout).to_string();
        let errores = String::from_utf8_lossy(&salida.stderr).to_string();

        assert!(
            salida.status.success(),
            "el ejemplo '{}' falló: {}",
            nombre,
            errores
        );
        assert!(
            errores.is_empty(),
            "el ejemplo '{}' escribió en stderr: {}",
            nombre,
            errores
        );
        assert_eq!(
            texto, esperado,
            "la salida del ejemplo '{}' no coincide",
            nombre
        );
    }
}

#[test]
fn traducir_emite_el_runtime_embebido() {
    let ruta = archivo_temporal("runtime", "mostrar(1)\n");
    let salida = Command::new(binario())
        .arg("traducir")
        .arg(&ruta)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'");
    let _ = fs::remove_file(&ruta);

    assert!(salida.status.success());
    let rust = String::from_utf8_lossy(&salida.stdout);
    assert!(rust.contains("enum Valor"));
    assert!(rust.contains("Valor::Numero"));
    assert!(rust.contains("fn main()"));
}

#[test]
fn error_al_usar_una_variable_no_declarada() {
    let ruta = archivo_temporal("variable", "mostrar(x)\n");
    let salida = Command::new(binario())
        .arg("traducir")
        .arg(&ruta)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'");
    let _ = fs::remove_file(&ruta);

    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(
        errores.contains("Error en la línea 1: la variable 'x' no existe"),
        "mensaje inesperado: {}",
        errores
    );
}

#[test]
fn error_al_asignar_una_variable_no_declarada() {
    let ruta = archivo_temporal("asignacion", "x = 5\n");
    let salida = Command::new(binario())
        .arg("traducir")
        .arg(&ruta)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'");
    let _ = fs::remove_file(&ruta);

    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(
        errores.contains("Error en la línea 1: la variable 'x' no existe"),
        "mensaje inesperado: {}",
        errores
    );
}

#[test]
fn error_al_pasar_un_numero_incorrecto_de_argumentos() {
    let fuente = "funcion suma(a, b) {\n    devolver a + b\n}\nmostrar(suma(1))\n";
    let ruta = archivo_temporal("argumentos", fuente);
    let salida = Command::new(binario())
        .arg("traducir")
        .arg(&ruta)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'");
    let _ = fs::remove_file(&ruta);

    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(
        errores.contains(
            "Error en la línea 4: la función 'suma' espera 2 argumentos y se han pasado 1"
        ),
        "mensaje inesperado: {}",
        errores
    );
}

#[test]
fn error_en_ejecucion_al_dividir_entre_cero() {
    let ruta = archivo_temporal("division", "mostrar(1 / 0)\n");
    let salida = ejecutar(&ruta);
    let _ = fs::remove_file(&ruta);

    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(
        errores.contains("Error en la línea 1 (en ejecución): no se puede dividir entre cero"),
        "mensaje inesperado: {}",
        errores
    );
}

#[test]
fn el_ejemplo_de_datos_coincide_ejecutado_dos_veces() {
    let datos = carpeta_datos("dos_veces");
    let ruta = carpeta_ejemplos().join("datos.dmn");
    let esperado = fs::read_to_string(carpeta_ejemplos().join("datos.esperado"))
        .expect("no se pudo leer la salida esperada");
    for vez in 1..=2 {
        let salida = ejecutar_con_datos(&ruta, &datos);
        assert!(
            salida.status.success(),
            "la ejecución {} falló: {}",
            vez,
            String::from_utf8_lossy(&salida.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&salida.stdout),
            esperado,
            "la salida de la ejecución {} no coincide",
            vez
        );
    }
    let _ = fs::remove_dir_all(&datos);
}

#[test]
fn los_datos_sobreviven_entre_dos_programas() {
    let datos = carpeta_datos("entre_programas");
    let escribe = archivo_temporal("escribe", "guardar(\"compartida\", \"saludo\", \"hola\")\n");
    let lee = archivo_temporal("lee", "mostrar(leer(\"compartida\", \"saludo\"))\n");
    let primera = ejecutar_con_datos(&escribe, &datos);
    assert!(
        primera.status.success(),
        "el primer programa falló: {}",
        String::from_utf8_lossy(&primera.stderr)
    );
    let segunda = ejecutar_con_datos(&lee, &datos);
    assert!(
        segunda.status.success(),
        "el segundo programa falló: {}",
        String::from_utf8_lossy(&segunda.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&segunda.stdout), "hola\n");
    let _ = fs::remove_dir_all(&datos);
}

#[test]
fn error_al_usar_un_nombre_de_coleccion_peligroso() {
    let datos = carpeta_datos("peligroso");
    let ruta = archivo_temporal("malo", "guardar(\"../malo\", \"a\", 1)\n");
    let salida = ejecutar_con_datos(&ruta, &datos);
    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(
        errores.contains("nombre de la colección"),
        "el error no es el esperado: {}",
        errores
    );
    assert!(!datos.join("..").join("malo.datos").exists());
    let _ = fs::remove_dir_all(&datos);
}
