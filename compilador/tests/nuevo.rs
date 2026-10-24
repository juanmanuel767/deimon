//! `deimon nuevo`: cada tipo de proyecto nace ordenado, sin errores, formateado y con sus
//! pruebas pasando. También el aviso de módulo que falta (E006 con «añade usar modulo»).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("nuevo_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn deimon(dentro: &Path, argumentos: &[&str]) -> Output {
    Command::new(binario())
        .args(argumentos)
        .current_dir(dentro)
        .output()
        .expect("no se pudo ejecutar deimon")
}

fn texto(salida: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&salida.stdout),
        String::from_utf8_lossy(&salida.stderr)
    )
}

#[test]
fn cada_tipo_nace_sin_errores_formateado_y_con_pruebas_que_pasan() {
    let base = carpeta("tipos");
    for (tipo, pieza) in [
        ("programa", "calculos.dmn"),
        ("web", "paginas.dmn"),
        ("app", "paginas.dmn"),
        ("sistema", "ordenes.dmn"),
    ] {
        let nombre = format!("mi-{}", tipo);
        let creado = deimon(&base, &["nuevo", tipo, &nombre]);
        assert!(creado.status.success(), "{}", texto(&creado));
        let proyecto = base.join(&nombre);
        for archivo in ["principal.dmn", "README.md", "arrancar.sh", ".gitignore"] {
            assert!(proyecto.join(archivo).is_file(), "{}: falta {}", tipo, archivo);
        }
        let pieza = format!("piezas/{}", pieza);
        for archivo in ["principal.dmn", pieza.as_str()] {
            let comprobado = deimon(&proyecto, &["comprobar", archivo]);
            assert!(comprobado.status.success(), "{} {}: {}", tipo, archivo, texto(&comprobado));
            let formato = deimon(&proyecto, &["formatear", "--comprobar", archivo]);
            assert!(formato.status.success(), "{} {}: {}", tipo, archivo, texto(&formato));
        }
        let pruebas = deimon(&proyecto, &["probar", "principal.dmn"]);
        assert!(pruebas.status.success(), "{}: {}", tipo, texto(&pruebas));
        assert!(texto(&pruebas).contains("fallidas: 0"), "{}: {}", tipo, texto(&pruebas));
        let readme = fs::read_to_string(proyecto.join("README.md")).expect("README.md");
        assert!(readme.contains("./arrancar.sh probar"), "{}", readme);
    }
    let programa = deimon(&base.join("mi-programa"), &["ejecutar", "principal.dmn"]);
    assert_eq!(String::from_utf8_lossy(&programa.stdout), "Hola, mundo!\nLa media de 2, 4 y 6 es 4\n");
}

#[test]
fn no_pisa_una_carpeta_que_ya_existe_ni_acepta_nombres_raros() {
    let base = carpeta("errores");
    fs::create_dir_all(base.join("ocupada")).expect("carpeta");
    fs::write(base.join("ocupada").join("mio.txt"), "no tocar").expect("archivo");
    let repetido = deimon(&base, &["nuevo", "web", "ocupada"]);
    assert_eq!(repetido.status.code(), Some(1));
    assert!(texto(&repetido).contains("ya existe 'ocupada'"), "{}", texto(&repetido));
    assert_eq!(fs::read_to_string(base.join("ocupada").join("mio.txt")).expect("archivo"), "no tocar");

    let tipo = deimon(&base, &["nuevo", "juego", "x"]);
    assert_eq!(tipo.status.code(), Some(2));
    assert!(texto(&tipo).contains("el tipo 'juego' no existe"), "{}", texto(&tipo));

    for raro in ["mi tienda", "../fuera", "año", "-x"] {
        let salida = deimon(&base, &["nuevo", "programa", raro]);
        assert_eq!(salida.status.code(), Some(2), "{}", raro);
        assert!(texto(&salida).contains("no vale para una carpeta"), "{}", texto(&salida));
    }
    assert!(!base.join("..").join("fuera").exists());
}

#[test]
fn una_funcion_de_modulo_sin_cargarlo_dice_que_modulo_falta() {
    let base = carpeta("modulo");
    fs::write(base.join("a.dmn"), "sea t = recortar(\" a \")\nsea r = raiz(9)\n").expect("programa");
    let salida = deimon(&base, &["comprobar", "a.dmn"]);
    assert_eq!(salida.status.code(), Some(1));
    let errores = texto(&salida);
    assert!(errores.contains("'recortar' es del módulo 'texto': añade usar modulo \"texto\""), "{}", errores);
    assert!(errores.contains("'raiz' es del módulo 'matematicas': añade usar modulo \"matematicas\""), "{}", errores);
    assert!(!errores.contains("¿Quisiste decir 'cortar'?"), "{}", errores);
}
