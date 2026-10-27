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

#[test]
fn funciones_como_valores() {
    igual_en_los_dos_motores(
        "funciones",
        r#"funcion doble(n) {
    devolver n * 2
}

funcion es_par(n) {
    devolver n % 2 == 0
}

funcion aplicar_dos_veces(f, x) {
    devolver f(f(x))
}

funcion edad_y_nombre(p) {
    devolver [-p["edad"], p["nombre"]]
}

funcion suma(a, b) {
    devolver a + b
}

funcion falla_con_tres(n) {
    si n == 3 {
        provocar_error("tres no vale")
    }
    devolver verdadero
}

funcion para_siempre(n) {
    devolver medir(filtrar([n], para_siempre))
}

sea f = doble
mostrar(f(21))
mostrar(f)
mostrar(aplicar_dos_veces(doble, 5))
mostrar(transformar([1, 2, 3], doble))
mostrar(filtrar(rango(1, 10), es_par))
sea gente = [{"nombre": "Lucia", "edad": 34}, {"nombre": "Pedro", "edad": 28}, {"nombre": "Ana", "edad": 34}]
para p en ordenar_por(gente, edad_y_nombre) {
    mostrar(p["nombre"])
}
sea operaciones = {"doble": doble, "suma": suma}
sea op = operaciones["suma"]
mostrar(op(2, 3))
mostrar(f == doble)
intentar {
    f(1, 2)
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    sea x = 5
    x(1)
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    filtrar([1], doble)
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    transformar([1], suma)
} si falla e {
    mostrar(e["mensaje"])
}
intentar {
    filtrar([1, 2, 3, 4], falla_con_tres)
} si falla e {
    mostrar("atrapado: " + e["mensaje"] + " en la línea " + e["linea"])
}
intentar {
    para_siempre(1)
} si falla e {
    mostrar(cortar(e["mensaje"], 0, 40))
}
intentar {
    guardar("cosas", "f", doble)
} si falla e {
    mostrar(e["mensaje"])
}
"#,
        "42\nfunción doble\n20\n[2, 4, 6]\n[2, 4, 6, 8, 10]\nAna\nLucia\nPedro\n5\nverdadero\n\
la función 'doble' espera 1 argumento y se han pasado 2\n\
se intenta llamar a número como si fuera una función\n\
filtrar(): la función tiene que devolver verdadero o falso, no número\n\
transformar() necesita una función de 1 parámetro, y 'suma' tiene 2\n\
atrapado: tres no vale en la línea 23\n\
hay más de 10000 llamadas a funciones un\n\
no se puede guardar la función 'doble': guarda los datos y vuelve a elegir la función al leerlos\n",
    );
}

/// Escribe el programa y lo ejecuta con `deimon ejecutar`; devuelve el texto del error.
fn error_de_ejecutar(nombre: &str, programa: &str) -> String {
    let carpeta = carpeta(nombre);
    let archivo = carpeta.join(format!("{}.dmn", nombre));
    fs::write(&archivo, programa).expect("no se pudo escribir el programa");
    let salida = Command::new(binario())
        .arg("ejecutar")
        .arg(&archivo)
        .output()
        .expect("deimon ejecutar");
    assert!(!salida.status.success(), "el programa debía fallar");
    String::from_utf8_lossy(&salida.stderr).to_string()
}

/// Escribe el programa, lo compila y ejecuta el ejecutable; devuelve el texto del error.
fn error_del_compilado(nombre: &str, programa: &str) -> String {
    let carpeta = carpeta(nombre);
    let archivo = carpeta.join(format!("{}.dmn", nombre));
    fs::write(&archivo, programa).expect("no se pudo escribir el programa");
    let ejecutable = carpeta.join(nombre);
    let compilacion = Command::new(binario())
        .arg("compilar")
        .arg(&archivo)
        .arg("--salida")
        .arg(&ejecutable)
        .output()
        .expect("deimon compilar");
    assert!(compilacion.status.success(), "{}", String::from_utf8_lossy(&compilacion.stderr));
    let salida = Command::new(&ejecutable).output().expect("programa compilado");
    assert!(!salida.status.success(), "el programa debía fallar");
    String::from_utf8_lossy(&salida.stderr).to_string()
}

/// Revisa el programa con `deimon comprobar` y devuelve todo lo que escribe.
fn revisar(nombre: &str, programa: &str) -> String {
    let carpeta = carpeta(nombre);
    let archivo = carpeta.join(format!("{}.dmn", nombre));
    fs::write(&archivo, programa).expect("no se pudo escribir el programa");
    let salida = Command::new(binario())
        .arg("comprobar")
        .arg(&archivo)
        .output()
        .expect("deimon comprobar");
    assert!(!salida.status.success(), "el programa debía tener errores");
    format!(
        "{}{}",
        String::from_utf8_lossy(&salida.stdout),
        String::from_utf8_lossy(&salida.stderr)
    )
}

#[test]
fn tipo_propio_ordena_los_campos_como_el_tipo() {
    igual_en_los_dos_motores(
        "tipo_orden",
        r#"tipo Persona {
    nombre: texto
    edad: numero
}
sea p = Persona({"edad": 30, "nombre": "Luis"})
mostrar(p)
mostrar(p["nombre"])
"#,
        "{nombre: Luis, edad: 30}\nLuis\n",
    );
}

#[test]
fn tipo_propio_avisa_del_campo_que_falta_sin_ejecutar() {
    let texto = revisar(
        "tipo_falta",
        r#"tipo Persona {
    nombre: texto
    edad: numero
}
sea p = Persona({"nombre": "Luis"})
"#,
    );
    assert!(texto.contains("falta el campo 'edad' de Persona"), "{}", texto);
    assert!(texto.contains("E027"), "{}", texto);
}

#[test]
fn tipo_propio_sugiere_el_campo_parecido_cuando_sobra() {
    let texto = revisar(
        "tipo_sobra",
        r#"tipo Persona {
    nombre: texto
    edad: numero
}
sea p = Persona({"nombre": "Luis", "edda": 30})
"#,
    );
    assert!(texto.contains("¿Quisiste decir 'edad'?"), "{}", texto);
}

#[test]
fn tipo_propio_falla_en_la_linea_de_la_llamada_con_datos_dinamicos() {
    let programa = r#"tipo Persona {
    nombre: texto
    edad: numero
    notas
}
sea datos = {"nombre": "Luis", "edad": "mucha", "notas": 0}
sea q = Persona(datos)
"#;
    for error in [error_de_ejecutar("tipo_linea_ejecutar", programa), error_del_compilado("tipo_linea_compilado", programa)] {
        assert!(error.contains("línea 7"), "{}", error);
        assert!(error.contains("el campo 'edad' de Persona"), "{}", error);
    }
}

#[test]
fn tipo_sigue_siendo_un_nombre_normal_fuera_de_la_declaracion() {
    igual_en_los_dos_motores("tipo_variable", "sea tipo = 3\nmostrar(tipo)\n", "3\n");
}

#[test]
fn enteros_grandes_exactos() {
    let programa = r#"funcion fact(n) {
    si n <= 1 {
        devolver 1
    }
    devolver n * fact(n - 1)
}
mostrar(fact(30))
sea x = 1
para i en rango(0, 100) {
    x = x * 2
}
mostrar(x)
mostrar(x % 7)
mostrar(9007199254740993)
mostrar(9007199254740993 - 1)
mostrar(x / 2)
mostrar(x > 5)
mostrar(ordenar([x, 3, 9007199254740993]))
mostrar(7 / 2)
mostrar(0.5 * 3)
"#;
    let esperado = "265252859812191058636308480000000\n2535301200456458802993406410752\n4\n9007199254740993\n9007199254740992\n1267650600228229401496703205376\nverdadero\n[3, 9007199254740993, 2535301200456458802993406410752]\n3.5\n1.5\n";
    igual_en_los_dos_motores("enteros_grandes_exactos", programa, esperado);
}
