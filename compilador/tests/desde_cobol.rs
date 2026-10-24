use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_desde_cobol")
        .join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

fn hay_cobc() -> bool {
    Command::new("cobc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Programa mínimo con los datos y las instrucciones que se le pasan (formato fijo).
fn programa(datos: &str, instrucciones: &str) -> String {
    let mut p = String::from("       IDENTIFICATION DIVISION.\n       PROGRAM-ID. PRUEBA.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n");
    for l in datos.lines() {
        p.push_str(&format!("       {}\n", l));
    }
    p.push_str("       PROCEDURE DIVISION.\n");
    for l in instrucciones.lines() {
        p.push_str(&format!("           {}\n", l));
    }
    p
}

#[test]
fn los_ejemplos_de_cobol_se_traducen_y_dan_la_misma_salida() {
    if !hay_cobc() {
        eprintln!("se salta: no hay GnuCOBOL (cobc) instalado");
        return;
    }
    for nombre in ["hola", "nomina", "tabla", "intereses", "clasificar"] {
        let c = carpeta(nombre);
        let cob = c.join(format!("{}.cob", nombre));
        fs::copy(raiz().join("ejemplos-cobol").join(format!("{}.cob", nombre)), &cob).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
            .args(["desde-cobol", cob.to_str().unwrap(), "--comprobar"])
            .env("TMPDIR", &c)
            .output()
            .unwrap();
        let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(0), "{}: {}", nombre, texto);
        assert!(texto.contains("escriben exactamente lo mismo"), "{}: {}", nombre, texto);
    }
}

#[test]
fn las_reglas_de_formato_y_truncado_coinciden_con_cobc() {
    if !hay_cobc() {
        eprintln!("se salta: no hay GnuCOBOL (cobc) instalado");
        return;
    }
    let c = carpeta("reglas");
    let cob = c.join("reglas.cob");
    fs::write(
        &cob,
        programa(
            "77 A PIC S9(3)V99 VALUE -7.5.\n77 B PIC 9(3) VALUE 999.\n77 C PIC S99 VALUE 0.\n77 D PIC X(5) VALUE \"abcdefgh\".\n77 F PIC X(8).\n77 G PIC 9(4)V9.\n77 H PIC 9(5) VALUE 42.",
            "DISPLAY A \"|\" B \"|\" C \"|\" D \"|\" F \"|\" G \"|\" H.\nMOVE 1234 TO B. DISPLAY B.\nMOVE -5 TO B. DISPLAY B.\nADD 999 TO B. DISPLAY B.\nCOMPUTE C = A / 4. DISPLAY C.\nCOMPUTE C ROUNDED = A / -4. DISPLAY C.\nCOMPUTE G = 2.55 * 3.3 - A. DISPLAY G.\nMOVE H TO D. DISPLAY D.\nMOVE D TO F. DISPLAY F \"|\".\nMOVE SPACES TO D. DISPLAY D \"|\".\nIF D = SPACES DISPLAY \"vacio\" END-IF.\nDIVIDE 17 BY 5 GIVING B REMAINDER C. DISPLAY B \" \" C.\nSTOP RUN.",
        ),
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["desde-cobol", cob.to_str().unwrap(), "--comprobar"])
        .env("TMPDIR", &c)
        .output()
        .unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("escriben exactamente lo mismo"), "{}", texto);
}

fn error_de(datos: &str, instrucciones: &str) -> String {
    match deimon::desde_cobol::traducir(&programa(datos, instrucciones), "x.cob") {
        Ok(t) => panic!("debía fallar y tradujo:\n{}", t.codigo),
        Err(e) => e,
    }
}

#[test]
fn lo_que_no_esta_soportado_da_un_error_claro_con_la_linea() {
    let e = error_de("77 A PIC 9.", "DISPLAY A.\nGO TO FIN.\nSTOP RUN.");
    assert!(e.contains("línea 8") && e.contains("GO") && e.contains("no está soportada"), "{}", e);
    let e = error_de("77 A PIC 9.", "ADD 1 TO A ON SIZE ERROR DISPLAY \"mal\".");
    assert!(e.contains("SIZE ERROR") && e.contains("línea 7"), "{}", e);
    let e = error_de("77 A PIC 9(4) COMP.", "STOP RUN.");
    assert!(e.contains("COMP") && e.contains("línea 5"), "{}", e);
    let e = error_de("77 A PIC ZZ9.", "STOP RUN.");
    assert!(e.contains("PIC ZZ9") && e.contains("línea 5"), "{}", e);
    let e = error_de("77 T PIC X OCCURS 3.", "STOP RUN.");
    assert!(e.contains("OCCURS"), "{}", e);
    let e = error_de("01 G.\n   05 A PIC 9.", "DISPLAY G.");
    assert!(e.contains("grupo"), "{}", e);
    let e = error_de("77 A PIC 9.", "DISPLAY B.\n");
    assert!(e.contains("'B'") || e.contains("B"), "{}", e);
    let e = error_de("77 A PIC 9.", "COMPUTE A = FUNCTION SQRT(4).");
    assert!(e.contains("FUNCTION"), "{}", e);
    let e = error_de("77 A PIC 9.", "COMPUTE A = 2 ** 3.");
    assert!(e.contains("potencia"), "{}", e);
    let e = error_de("77 A PIC X(3).\n77 B PIC X(3).", "IF A > B DISPLAY \"x\" END-IF.");
    assert!(e.contains("textos"), "{}", e);
    let e = error_de("77 A PIC 9.", "PERFORM VARYING A FROM 1 BY 1 UNTIL A > 3 AFTER A FROM 1 BY 1 UNTIL A > 3 DISPLAY A END-PERFORM.");
    assert!(e.contains("AFTER"), "{}", e);
    let e = error_de("77 A PIC 9.", "PRINCIPAL.\n    PERFORM PRINCIPAL.");
    assert!(e.contains("recursión"), "{}", e);
}

#[test]
fn un_error_no_crea_ningun_archivo() {
    let c = carpeta("sin_archivo");
    let cob = c.join("malo.cob");
    fs::write(&cob, programa("77 A PIC 9.", "ACCEPT A.\nSTOP RUN.")).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_deimon")).args(["desde-cobol", cob.to_str().unwrap()]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let texto = String::from_utf8_lossy(&o.stderr);
    assert!(texto.contains("ACCEPT") && texto.contains("línea 7"), "{}", texto);
    assert!(!c.join("malo.dmn").exists());
}

#[test]
fn el_formato_libre_tambien_se_entiende() {
    let t = deimon::desde_cobol::traducir(
        "IDENTIFICATION DIVISION.\nPROGRAM-ID. LIBRE.\n*> comentario\nDATA DIVISION.\nWORKING-STORAGE SECTION.\n01 N PIC 9(3) VALUE 7.\nPROCEDURE DIVISION.\n    DISPLAY \"N=\" N *> otro\n    STOP RUN.\n",
        "libre.cob",
    )
    .unwrap();
    assert!(deimon::traducir(&t.codigo).is_ok(), "{}", t.codigo);
    assert!(t.codigo.contains("sea n = 7"), "{}", t.codigo);
}
