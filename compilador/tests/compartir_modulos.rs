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
        .join("pruebas_compartir_modulos");
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

fn deimon(carpeta: &Path, casa: &Path, argumentos: &[&str]) -> Output {
    Command::new(binario())
        .args(argumentos)
        .current_dir(carpeta)
        .env("TMPDIR", carpeta)
        .env("DEIMON_CASA", casa)
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

fn git(carpeta: &Path, argumentos: &[&str]) {
    let o = Command::new("git")
        .args(["-c", "user.name=x", "-c", "user.email=x@x", "-c", "commit.gpgsign=false"])
        .args(argumentos)
        .current_dir(carpeta)
        .output()
        .expect("no se pudo lanzar git");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

const CODIGO: &str = r#"
fn ext_gritar(args: Vec<Valor>) -> Result<Valor, String> {
    match args.into_iter().next() {
        Some(Valor::Texto(texto)) => Ok(Valor::texto(format!("{}!", texto.to_uppercase()))),
        _ => Err("gritar() espera un texto".to_string()),
    }
}
"#;

/// Repositorio git local con un módulo `gritos` en su raíz.
fn repositorio(c: &Path, nombre_dir: &str, nombre: &str) -> PathBuf {
    let repo = c.join(nombre_dir);
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    escribir(&repo, "modulo.txt", &format!("nombre: {}\ndescripcion: gritos de prueba\n", nombre));
    escribir(&repo, "funciones.txt", "gritar | 1 | grita un texto | gritar(\"hola\")\n");
    escribir(&repo, "codigo.rs", CODIGO);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "primero"]);
    repo
}

#[test]
fn instalar_usar_actualizar_y_quitar() {
    let c = carpeta("ciclo");
    let casa = carpeta("ciclo_casa");
    let repo = repositorio(&c, "origen", "gritos");
    let origen = repo.to_str().unwrap();

    let o = deimon(&c, &casa, &["modulo", "instalar", origen]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert!(salida(&o).contains("código Rust que se ejecuta en tu equipo"), "{}", salida(&o));
    assert!(casa.join(".deimon/modulos/gritos/modulo.txt").is_file());
    assert!(casa.join(".deimon/modulos/gritos/.git").exists());

    let principal = escribir(&c, "principal.dmn", "usar modulo \"gritos\"\nmostrar(gritar(\"hola\"))\n");
    let o = deimon(&c, &casa, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "HOLA!\n");

    // Repetido: error claro que manda a actualizar.
    let o = deimon(&c, &casa, &["modulo", "instalar", origen]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("ya está instalado"), "{}", errores(&o));
    assert!(errores(&o).contains("actualizar"), "{}", errores(&o));

    // Segundo commit y actualizar.
    escribir(&repo, "funciones.txt", "gritar | 1 | grita un texto | gritar(\"hola\")\n# nuevo\n");
    git(&repo, &["commit", "-q", "-am", "segundo"]);
    let o = deimon(&c, &casa, &["modulo", "actualizar", "gritos"]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    let texto = fs::read_to_string(casa.join(".deimon/modulos/gritos/funciones.txt")).unwrap();
    assert!(texto.contains("# nuevo"), "{}", texto);

    let o = deimon(&c, &casa, &["modulo", "quitar", "gritos"]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert!(!casa.join(".deimon/modulos/gritos").exists());
    let o = deimon(&c, &casa, &["modulo", "quitar", "gritos"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("no está instalado"), "{}", errores(&o));
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}

#[test]
fn con_aqui_instala_junto_al_programa() {
    let c = carpeta("aqui");
    let casa = carpeta("aqui_casa");
    let repo = repositorio(&c, "origen", "gritos");
    let o = deimon(&c, &casa, &["modulo", "instalar", repo.to_str().unwrap(), "--aqui"]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert!(c.join("modulos/gritos/modulo.txt").is_file());
    assert!(!casa.join(".deimon").exists());
    let o = deimon(&c, &casa, &["modulo", "quitar", "gritos", "--aqui"]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert!(!c.join("modulos/gritos").exists());
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}

#[test]
fn nombres_invalidos_no_borran_ni_instalan() {
    let c = carpeta("nombres");
    let casa = carpeta("nombres_casa");
    escribir(&casa, "importante.txt", "no tocar");
    for orden in ["quitar", "actualizar"] {
        let o = deimon(&c, &casa, &["modulo", orden, "../malo"]);
        assert_eq!(o.status.code(), Some(1));
        assert!(errores(&o).contains("no vale"), "{}", errores(&o));
    }
    assert!(casa.join("importante.txt").is_file());

    // El nombre declarado en modulo.txt también se valida.
    let repo = repositorio(&c, "malo", "../malo");
    let o = deimon(&c, &casa, &["modulo", "instalar", repo.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("no vale"), "{}", errores(&o));
    assert!(!casa.join(".deimon/modulos/malo").exists());
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}

#[test]
fn sin_modulo_txt_se_rechaza() {
    let c = carpeta("sin_ficha");
    let casa = carpeta("sin_ficha_casa");
    let repo = c.join("origen");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    escribir(&repo, "leeme.txt", "nada");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "primero"]);
    let o = deimon(&c, &casa, &["modulo", "instalar", repo.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("modulo.txt"), "{}", errores(&o));
    let vacia = fs::read_dir(casa.join(".deimon/modulos")).map(|mut d| d.next().is_none()).unwrap_or(true);
    assert!(vacia, "no debe quedar nada instalado");
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}

#[test]
fn nombre_de_la_biblioteca_estandar_se_rechaza() {
    let c = carpeta("estandar");
    let casa = carpeta("estandar_casa");
    let repo = repositorio(&c, "origen", "texto");
    let o = deimon(&c, &casa, &["modulo", "instalar", repo.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("biblioteca estándar"), "{}", errores(&o));
    assert!(!casa.join(".deimon/modulos/texto").exists());
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}

#[test]
fn origenes_peligrosos_se_rechazan() {
    let c = carpeta("origenes");
    let casa = carpeta("origenes_casa");
    let o = deimon(&c, &casa, &["modulo", "instalar", "http://ejemplo.com/modulo.git"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("http://"), "{}", errores(&o));
    let o = deimon(&c, &casa, &["modulo", "instalar", "--upload-pack=touch x"]);
    assert_eq!(o.status.code(), Some(2), "{}", errores(&o));
    let o = deimon(&c, &casa, &["modulo", "instalar", "-x"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("'-'"), "{}", errores(&o));
    let _ = fs::remove_dir_all(&c);
    let _ = fs::remove_dir_all(&casa);
}
