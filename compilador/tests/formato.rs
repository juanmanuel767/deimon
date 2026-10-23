use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("formato_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn escribir(carpeta: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo");
    ruta
}

fn ejecutar(argumentos: &[&str]) -> Output {
    Command::new(binario())
        .args(argumentos)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn salida(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn ejemplos_ln(carpeta: &Path, encontrados: &mut Vec<PathBuf>) {
    let Ok(entradas) = fs::read_dir(carpeta) else {
        return;
    };
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.is_dir() {
            ejemplos_ln(&ruta, encontrados);
        } else if ruta.extension().map_or(false, |extension| extension == "dmn") {
            encontrados.push(ruta);
        }
    }
}

#[test]
fn un_programa_desordenado_queda_con_el_estilo_unico() {
    let fuente = "# prueba de formato\n\n\nfuncion suma( a,b ){\n        devolver   a+b    # suma\n}\n\n\n\nsi suma(2,3)>4{\n mostrar(\"hola  # mundo\")\n}\nsino{\n   mostrar(1)\n}\n";
    let esperado = "# prueba de formato\n\nfuncion suma(a, b) {\n    devolver a + b  # suma\n}\n\nsi suma(2, 3) > 4 {\n    mostrar(\"hola  # mundo\")\n} sino {\n    mostrar(1)\n}\n";
    assert_eq!(deimon::formato::formatear(fuente).unwrap(), esperado);
}

#[test]
fn formatear_dos_veces_da_lo_mismo_en_todos_los_ejemplos() {
    let mut archivos = Vec::new();
    for carpeta in ["ejemplos", "ejemplos-web", "rendimiento", "biblioteca"] {
        ejemplos_ln(&raiz().join(carpeta), &mut archivos);
    }
    assert!(!archivos.is_empty(), "no se encontraron ejemplos .dmn");
    for ruta in archivos {
        let fuente = fs::read_to_string(&ruta).expect("no se pudo leer el ejemplo");
        let una = deimon::formato::formatear(&fuente)
            .unwrap_or_else(|error| panic!("{}: {}", ruta.display(), error));
        let dos = deimon::formato::formatear(&una)
            .unwrap_or_else(|error| panic!("{}: {}", ruta.display(), error));
        assert_eq!(
            una,
            dos,
            "formatear dos veces cambia {}",
            ruta.display()
        );
    }
}

#[test]
fn comprobar_sale_con_uno_si_falta_formatear_y_con_cero_si_no() {
    let c = carpeta("comprobar");
    let ruta = escribir(&c, "desorden.dmn", "sea   x=1\nmostrar( x )\n");
    let antes = fs::read_to_string(&ruta).unwrap();

    let o = ejecutar(&["formatear", ruta.to_str().unwrap(), "--comprobar"]);
    assert_eq!(o.status.code(), Some(1), "{}", salida(&o));
    assert!(salida(&o).contains("no está formateado"), "{}", salida(&o));
    assert_eq!(antes, fs::read_to_string(&ruta).unwrap());

    let o = ejecutar(&["formatear", ruta.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", salida(&o));

    let o = ejecutar(&["formatear", ruta.to_str().unwrap(), "--comprobar"]);
    assert_eq!(o.status.code(), Some(0), "{}", salida(&o));
    assert!(salida(&o).contains("ya está formateado"), "{}", salida(&o));
}

#[test]
fn los_textos_con_espacios_y_almohadillas_no_se_tocan() {
    let fuente = "sea mensaje=  \"hola  #  mundo  \"\nmostrar( mensaje )\n";
    let formateado = deimon::formato::formatear(fuente).unwrap();
    assert!(
        formateado.contains("\"hola  #  mundo  \""),
        "el texto cambió: {}",
        formateado
    );
    assert_eq!(
        formateado,
        "sea mensaje = \"hola  #  mundo  \"\nmostrar(mensaje)\n"
    );
}

#[test]
fn un_archivo_con_error_de_sintaxis_no_se_toca() {
    let c = carpeta("error");
    let fuente = "si verdadero {\nmostrar(1)\n";
    let ruta = escribir(&c, "roto.dmn", fuente);

    let o = ejecutar(&["formatear", ruta.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}", salida(&o));
    assert!(salida(&o).contains("Error en la línea"), "{}", salida(&o));
    assert_eq!(fuente, fs::read_to_string(&ruta).unwrap());
}

#[test]
fn todas_las_palabras_reservadas_aparecen_en_la_gramatica() {
    let gramatica = fs::read_to_string(raiz().join("docs").join("GRAMATICA.md"))
        .expect("no se pudo leer docs/GRAMATICA.md");
    for palabra in deimon::lexer::PALABRAS_RESERVADAS {
        let tokens = deimon::lexer::tokenizar(palabra).expect("la palabra no se pudo leer");
        assert!(
            !matches!(tokens[0].tipo, deimon::lexer::Tipo::Ident(_)),
            "'{}' debería ser una palabra reservada",
            palabra
        );
        assert!(
            gramatica.contains(&format!("`{}`", palabra)),
            "GRAMATICA.md no menciona la palabra reservada '{}'",
            palabra
        );
    }
}
