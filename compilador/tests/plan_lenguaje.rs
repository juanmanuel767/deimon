//! Puntos del grupo 1 de docs/PLAN.md: cada programa tiene que dar la misma salida con el
//! intérprete (`deimon ejecutar`) y con el programa compilado (`deimon compilar`).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("plan_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

/// Ejecuta el programa con los dos motores y exige la salida esperada en ambos.
fn igual_en_los_dos_motores(nombre: &str, programa: &str, esperado: &str) {
    let carpeta = carpeta(nombre);
    let archivo = carpeta.join(format!("{}.dmn", nombre));
    fs::write(&archivo, programa).expect("no se pudo escribir el programa");

    let interprete = Command::new(binario()).arg("ejecutar").arg(&archivo).output().expect("deimon ejecutar");
    assert!(interprete.status.success(), "{}", String::from_utf8_lossy(&interprete.stderr));
    assert_eq!(String::from_utf8_lossy(&interprete.stdout), esperado, "intérprete");

    let ejecutable = carpeta.join(nombre);
    let compilacion = Command::new(binario())
        .arg("compilar")
        .arg(&archivo)
        .arg("--salida")
        .arg(&ejecutable)
        .output()
        .expect("deimon compilar");
    assert!(compilacion.status.success(), "{}", String::from_utf8_lossy(&compilacion.stderr));
    let compilado = Command::new(&ejecutable).output().expect("programa compilado");
    assert_eq!(String::from_utf8_lossy(&compilado.stdout), esperado, "compilado");
}

#[test]
fn ordenar_por_varios_campos_con_errores_claros() {
    igual_en_los_dos_motores(
        "ordenar_por",
        r#"sea gente = [{"nombre": "Lucia", "edad": 34}, {"nombre": "Pedro", "edad": 28}, {"nombre": "Ana", "edad": 34}, {"nombre": "Zoe", "edad": 19}]
para p en ordenar_por(gente, ["-edad", "nombre"]) {
    mostrar(p["nombre"] + " " + p["edad"])
}
mostrar(ordenar_por(gente, "nombre")[0]["nombre"])
mostrar(medir(ordenar_por([], "x")))
mostrar(gente[0]["nombre"])
intentar {
    ordenar_por([{"a": 1}, {"b": 2}], "a")
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    ordenar_por([{"a": 1}, {"a": "x"}], "a")
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    ordenar_por([1, 2], "a")
} si falla e {
    mostrar(e["mensaje"])
}
"#,
        "Ana 34\nLucia 34\nPedro 28\nZoe 19\nAna\n0\nLucia\n\
ordenar_por(): el elemento 1 no tiene el campo 'a'\n\
ordenar_por(): el campo 'a' mezcla números y textos (mira el elemento 1)\n\
ordenar_por() necesita una lista de listas con claves, pero el elemento 0 es número\n",
    );
}
