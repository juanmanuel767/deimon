use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

/// Carpeta de prueba dentro de target/tmp (nunca se escribe en /tmp).
fn carpeta(nombre: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_modulos_propios");
    let carpeta = base.join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta de prueba");
    carpeta
}

fn escribir(carpeta: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    if let Some(padre) = ruta.parent() {
        fs::create_dir_all(padre).expect("no se pudo crear la subcarpeta de prueba");
    }
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

fn deimon(carpeta: &Path, argumentos: &[&str]) -> Output {
    Command::new(binario())
        .args(argumentos)
        .current_dir(carpeta)
        .env("TMPDIR", carpeta)
        .env_remove("DEIMON_PUERTO")
        .env_remove("DEIMON_DATOS")
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn salida(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn errores(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

const CODIGO: &str = r#"
fn ext_mayusculas(args: Vec<Valor>) -> Result<Valor, String> {
    match args.into_iter().next() {
        Some(Valor::Texto(texto)) => Ok(Valor::texto(texto.to_uppercase())),
        Some(otro) => Err(format!("mayusculas() espera un texto, no {}", otro.tipo())),
        None => Err("mayusculas() espera un texto".to_string()),
    }
}

fn ext_fallar(_args: Vec<Valor>) -> Result<Valor, String> {
    Err("esta extensión ha fallado a propósito".to_string())
}

fn ext_sumar(args: Vec<Valor>) -> Result<Valor, String> {
    match args.as_slice() {
        [Valor::Numero(a), Valor::Numero(b)] => Ok(Valor::Numero(a + b)),
        _ => Err("sumar() espera dos números".to_string()),
    }
}
"#;

const FUNCIONES: &str = "# Extensión de prueba
mayusculas | 1 | devuelve el texto en mayúsculas | mayusculas(\"hola\")
fallar | 0 | falla siempre a propósito | fallar()
sumar | 2 | suma dos números | sumar(2, 3)
";

/// Crea `modulos/<nombre>/` con su modulo.txt y el código y las funciones indicadas.
fn extension(carpeta: &Path, nombre: &str, funciones: &str, codigo: &str) {
    escribir(
        carpeta,
        &format!("modulos/{}/modulo.txt", nombre),
        &format!("nombre: {}\ndescripcion: módulo de prueba\n", nombre),
    );
    escribir(
        carpeta,
        &format!("modulos/{}/funciones.txt", nombre),
        funciones,
    );
    escribir(
        carpeta,
        &format!("modulos/{}/codigo.rs", nombre),
        codigo,
    );
}

fn extension_completa(carpeta: &Path, nombre: &str) {
    extension(carpeta, nombre, FUNCIONES, CODIGO);
}

#[test]
fn usar_extension_ejecuta_sus_funciones() {
    let c = carpeta("correcto");
    extension_completa(&c, "prueba");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"prueba\"\nmostrar(mayusculas(\"hola\"))\nmostrar(sumar(2, 3))\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "HOLA\n5\n");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn los_argumentos_incorrectos_dan_e007() {
    let c = carpeta("argumentos");
    extension_completa(&c, "prueba");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"prueba\"\nmostrar(sumar(1))\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E007"), "{}", texto);
    assert!(
        texto.contains("la función 'sumar' espera 2 argumentos y se han pasado 1"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn una_extension_que_devuelve_err_da_error_en_ejecucion_con_linea() {
    let c = carpeta("fallo");
    extension_completa(&c, "prueba");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"prueba\"\nmostrar(fallar())\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}", salida(&o));
    let texto = errores(&o);
    assert!(
        texto.contains("Error en la línea 2 (en ejecución): esta extensión ha fallado a propósito"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn una_extension_que_no_existe_da_e017() {
    let c = carpeta("no_existe");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"no_existe\"\nmostrar(1)\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E017"), "{}", texto);
    assert!(
        texto.contains("no se encontró el módulo 'no_existe'"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_funciones_txt_mal_formado_da_e018() {
    let c = carpeta("mal_formado");
    extension(
        &c,
        "rota",
        "mayusculas | 1 texto | descripción | ejemplo\n",
        CODIGO,
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"rota\"\nmostrar(1)\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E018"), "{}", texto);
    assert!(
        texto.contains("el módulo 'rota' tiene funciones.txt mal formado"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn una_funcion_que_choca_con_una_integrada_da_e020() {
    let c = carpeta("choque");
    extension(
        &c,
        "choca",
        "mostrar | 1 | intenta tapar una integrada | mostrar(\"hola\")\n",
        "fn ext_mostrar(_args: Vec<Valor>) -> Result<Valor, String> {\n    Err(\"no\".to_string())\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"choca\"\nmostrar(1)\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E020"), "{}", texto);
    assert!(
        texto.contains("choca con una función integrada"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_codigo_rust_que_no_compila_lo_dice_la_extension() {
    let c = carpeta("rust_roto");
    extension(
        &c,
        "roto",
        "mayusculas | 1 | no compila | mayusculas(\"hola\")\n",
        "fn ext_mayusculas(_args: Vec<Valor>) -> Result<Valor, String> {\n    esto no es Rust\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"roto\"\nmostrar(mayusculas(\"hola\"))\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}", salida(&o));
    assert!(
        errores(&o).contains("El módulo 'roto' tiene un error en su código Rust:"),
        "{}",
        errores(&o)
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn probar_y_comprobar_funcionan_con_extensiones() {
    let c = carpeta("probar_comprobar");
    extension_completa(&c, "prueba");
    let principal = escribir(
        &c,
        "principal.dmn",
        concat!(
            "usar modulo \"prueba\"\n",
            "funcion saludo(nombre) {\n  devolver mayusculas(nombre)\n}\n\n",
            "prueba \"saludo en mayúsculas\" {\n",
            "  comprobar_igual(saludo(\"ana\"), \"ANA\")\n",
            "}\n\n",
            "mostrar(\"no se ejecuta al probar\")\n"
        ),
    );
    let comprobado = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    assert_eq!(
        comprobado.status.code(),
        Some(0),
        "{}{}",
        salida(&comprobado),
        errores(&comprobado)
    );
    assert!(
        salida(&comprobado).contains("no tiene errores"),
        "{}",
        salida(&comprobado)
    );

    let probado = deimon(&c, &["probar", principal.to_str().unwrap()]);
    let texto = salida(&probado);
    assert_eq!(probado.status.code(), Some(0), "{}{}", texto, errores(&probado));
    assert!(texto.contains("✓ saludo en mayúsculas"), "{}", texto);
    assert!(
        texto.contains("Pruebas: 1 · correctas: 1 · fallidas: 0"),
        "{}",
        texto
    );
    assert!(!texto.contains("no se ejecuta al probar"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn la_guia_muestra_las_extensiones_usadas() {
    let c = carpeta("guia");
    extension_completa(&c, "prueba");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar modulo \"prueba\"\nmostrar(mayusculas(\"hola\"))\nmostrar(sumar(1, 2))\n",
    );
    let o = deimon(&c, &["guia", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert!(texto.contains("Módulos usados"), "{}", texto);
    assert!(texto.contains("mayusculas(argumento) [prueba]"), "{}", texto);
    assert!(
        texto.contains("sumar(argumento1, argumento2) [prueba]"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn una_extension_usada_en_dos_archivos_se_incluye_una_sola_vez() {
    let c = carpeta("una_vez");
    extension_completa(&c, "prueba");
    escribir(
        &c,
        "apoyo.dmn",
        "usar modulo \"prueba\"\nfuncion grito(texto) {\n  devolver mayusculas(texto)\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        concat!(
            "usar modulo \"prueba\"\n",
            "usar \"apoyo.dmn\"\n",
            "mostrar(grito(\"hola\"))\n",
            "mostrar(sumar(1, 2))\n"
        ),
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "HOLA\n3\n");
    let _ = fs::remove_dir_all(&c);
}
