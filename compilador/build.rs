// Prepara la biblioteca estándar para el intérprete leyendo `biblioteca/*/funciones.txt`.
// Así la lista de funciones sale del mismo sitio que usa el modo Rust y no se puede
// quedar atrás cuando un módulo gana una función nueva.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Módulos que el intérprete no lleva dentro (siguen en el modo Rust); el motivo se da en
/// `src/interprete/compilador.rs`. `sql` entra si hay una SQLite para meter dentro (ver
/// `sqlite_para_meter_dentro`).
const FUERA_DEL_INTERPRETE: &[&str] = &["sql", "pantallas"];

/// Carpeta con `libsqlite3.a` (SQLite que se puede meter dentro de `deimon`), si la hay y se
/// prepara `deimon` para este mismo ordenador. Con ella, el intérprete usa el módulo `sql` sin
/// Rust y sin necesitar SQLite instalada donde se abra. `DEIMON_SQLITE_A` indica otra carpeta;
/// `DEIMON_SQLITE_A=no` la desactiva.
fn sqlite_para_meter_dentro() -> Option<std::path::PathBuf> {
    println!("cargo:rerun-if-env-changed=DEIMON_SQLITE_A");
    let objetivo = std::env::var("TARGET").ok()?;
    let anfitrion = std::env::var("HOST").ok()?;
    if objetivo != anfitrion || !objetivo.contains("linux") {
        return None;
    }
    let candidatas: Vec<std::path::PathBuf> = match std::env::var("DEIMON_SQLITE_A") {
        Ok(valor) if valor == "no" => return None,
        Ok(valor) => vec![valor.into()],
        Err(_) => ["/usr/lib/x86_64-linux-gnu", "/usr/lib/aarch64-linux-gnu", "/usr/lib64", "/usr/lib", "/usr/local/lib"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect(),
    };
    candidatas.into_iter().find(|carpeta| carpeta.join("libsqlite3.a").is_file())
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(deimon_sqlite)");
    let sqlite = sqlite_para_meter_dentro();
    if let Some(carpeta) = &sqlite {
        println!("cargo:rustc-link-search=native={}", carpeta.display());
        println!("cargo:rustc-cfg=deimon_sqlite");
    }
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("biblioteca");
    println!("cargo:rerun-if-changed={}", raiz.display());

    let mut modulos: Vec<String> = fs::read_dir(&raiz)
        .expect("no se pudo leer la carpeta biblioteca")
        .filter_map(|entrada| entrada.ok())
        .filter(|entrada| entrada.path().join("codigo.rs").is_file())
        .map(|entrada| entrada.file_name().to_string_lossy().into_owned())
        .filter(|nombre| !FUERA_DEL_INTERPRETE.contains(&nombre.as_str()) || (nombre == "sql" && sqlite.is_some()))
        .collect();
    modulos.sort();

    let mut inclusiones = String::new();
    let mut codigos = String::from("pub const CODIGOS_ESTANDAR: &[(&str, &str)] = &[\n");
    let mut externas = String::from("static EXTERNAS: &[(&str, FuncionExterna)] = &[\n");
    for modulo in &modulos {
        let carpeta = raiz.join(modulo);
        let codigo = carpeta.join("codigo.rs");
        let funciones = carpeta.join("funciones.txt");
        println!("cargo:rerun-if-changed={}", codigo.display());
        println!("cargo:rerun-if-changed={}", funciones.display());

        if modulo == "sql" {
            // El mismo código, pero con SQLite metida dentro de `deimon` (enlace estático).
            let texto = fs::read_to_string(&codigo).unwrap();
            const ENLACE: &str = "#[link(name = \"sqlite3\")]";
            assert!(texto.contains(ENLACE), "biblioteca/sql/codigo.rs ya no enlaza sqlite3 como se esperaba");
            let copia = Path::new(&std::env::var("OUT_DIR").unwrap()).join("sql_interprete.rs");
            fs::write(&copia, texto.replace(ENLACE, "#[link(name = \"sqlite3\", kind = \"static\")]")).unwrap();
            writeln!(inclusiones, "// ---- módulo sql (SQLite dentro)\ninclude!({:?});", copia.display().to_string()).unwrap();
        } else {
            writeln!(inclusiones, "// ---- módulo {}\ninclude!({:?});", modulo, codigo.display().to_string()).unwrap();
        }
        writeln!(codigos, "    ({:?}, include_str!({:?})),", modulo, codigo.display().to_string()).unwrap();

        let texto = fs::read_to_string(&funciones)
            .unwrap_or_else(|error| panic!("no se pudo leer {}: {}", funciones.display(), error));
        for linea in texto.lines().map(str::trim) {
            if linea.is_empty() || linea.starts_with('#') {
                continue;
            }
            let nombre = linea.split('|').next().unwrap_or("").trim();
            assert!(
                !nombre.is_empty() && nombre.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "nombre de función no válido en {}: {:?}",
                funciones.display(),
                nombre
            );
            writeln!(externas, "    ({:?}, ext_{}),", nombre, nombre).unwrap();
        }
    }
    codigos.push_str("];\n");
    externas.push_str("];\n");

    let destino = Path::new(&std::env::var("OUT_DIR").unwrap()).join("biblioteca_interprete.rs");
    fs::write(destino, format!("{}\n{}\n{}", inclusiones, codigos, externas)).unwrap();

    generar_integradas();
    generar_nucleo();
}

/// `nucleo/base.rs` y `nucleo/servidor.rs` con las marcas que el generador sustituye en cada
/// programa (`Generador::nuevo`), puestas para el intérprete. El perfil, que en el modo Rust
/// queda grabado en el programa, en el intérprete se elige al arrancar (`perfil_ln()`).
fn generar_nucleo() {
    println!("cargo:rerun-if-changed=src/nucleo/base.rs");
    println!("cargo:rerun-if-changed=src/nucleo/servidor.rs");
    println!("cargo:rerun-if-changed=src/nucleo/entero_grande.rs");
    println!("cargo:rerun-if-changed=recursos/favicon.png");
    const PERFIL: &str = "const PERFIL_LN: &str = \"@PERFIL@\";";
    let base = fs::read_to_string("src/nucleo/base.rs").expect("no se pudo leer src/nucleo/base.rs");
    let servidor = fs::read_to_string("src/nucleo/servidor.rs").expect("no se pudo leer src/nucleo/servidor.rs");
    let icono = Path::new(env!("CARGO_MANIFEST_DIR")).join("recursos").join("favicon.png");
    assert!(base.contains(PERFIL) && base.contains("&[]; //@ICONO_WEB@"), "faltan las marcas del núcleo");
    let entero_grande =
        fs::read_to_string("src/nucleo/entero_grande.rs").expect("no se pudo leer src/nucleo/entero_grande.rs");
    let nucleo = (base + "\n" + &entero_grande)
        .replace(
            PERFIL,
            "static PERFIL_ELEGIDO: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();\n\
             fn perfil_ln() -> &'static str { PERFIL_ELEGIDO.get().copied().unwrap_or(\"equilibrado\") }",
        )
        .replace("PERFIL_LN", "perfil_ln()")
        .replace("&[]; //@ICONO_WEB@", &format!("include_bytes!({:?});", icono.display().to_string()));
    let carpeta = std::env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&carpeta).join("nucleo_interprete.rs"), nucleo).unwrap();
    fs::write(Path::new(&carpeta).join("servidor_interprete.rs"), servidor.replace("PERFIL_LN", "perfil_ln()")).unwrap();
}

/// Tabla de integradas para el intérprete, sacada de la misma lista que usa el generador
/// (`src/integradas.rs`). El modo Rust traduce cada llamada a `nombre(&a, &b, ...)`, salvo
/// las de `ESPECIALES`, que el intérprete trata aparte igual que el generador.
fn generar_integradas() {
    const ESPECIALES: &[&str] = &["elegir", "comprobar", "comprobar_igual", "visitar", "enviar", "servir"];
    println!("cargo:rerun-if-changed=src/integradas.rs");
    let mut tabla = String::from("static INTEGRADAS_NUCLEO: &[(&str, usize, fn(&[Valor]) -> Valor)] = &[\n");
    for integrada in integradas::INTEGRADAS {
        if ESPECIALES.contains(&integrada.nombre) {
            continue;
        }
        let argumentos: Vec<String> = (0..integrada.parametros.len()).map(|i| format!("&a[{}]", i)).collect();
        writeln!(
            tabla,
            "    ({:?}, {}, |a: &[Valor]| {}({})),",
            integrada.nombre,
            integrada.parametros.len(),
            integrada.nombre,
            argumentos.join(", ")
        )
        .unwrap();
    }
    tabla.push_str("];\n");
    let destino = Path::new(&std::env::var("OUT_DIR").unwrap()).join("integradas_interprete.rs");
    fs::write(destino, tabla).unwrap();
}

#[allow(dead_code)]
#[path = "src/integradas.rs"]
mod integradas;
