//! `deimon comprobar --estricto` (plan 2.2): avisos A001 a A009 de lo dudoso.
//! Para cada aviso hay un programa que avisa y uno parecido que no.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn carpeta_tmp(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("estricto_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn deimon(carpeta: &Path, argumentos: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(argumentos)
        .current_dir(carpeta)
        .env_remove("DEIMON_PUERTO")
        .env_remove("DEIMON_DATOS")
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn salida(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn avisos(fuente: &str) -> Vec<(&'static str, usize)> {
    assert!(
        deimon::comprobar(fuente).is_empty(),
        "el programa de prueba no debe tener errores: {:?}\n{}",
        deimon::comprobar(fuente),
        fuente
    );
    deimon::avisos_archivo(fuente, Path::new("estricto.dmn"))
        .iter()
        .map(|a| (a.codigo, a.linea))
        .collect()
}

fn codigos(fuente: &str) -> Vec<&'static str> {
    avisos(fuente).into_iter().map(|(codigo, _)| codigo).collect()
}

fn sin_avisos(fuente: &str) {
    assert_eq!(codigos(fuente), Vec::<&str>::new(), "no debía avisar:\n{}", fuente);
}

#[test]
fn a001_variable_que_nunca_se_lee() {
    let con = "funcion f() {\n    sea sobra = 1\n    devolver 2\n}\nmostrar(f())\n";
    assert_eq!(avisos(con), vec![("A001", 2)]);
    // Con _ delante se silencia; leerla también.
    sin_avisos("funcion f() {\n    sea _sobra = 1\n    devolver 2\n}\nmostrar(f())\n");
    sin_avisos("funcion f() {\n    sea usada = 1\n    devolver usada\n}\nmostrar(f())\n");
    // Solo asignarla no es leerla; asignar a un elemento sí cuenta como uso.
    assert_eq!(codigos("sea a = 1\na = 2\n"), vec!["A001"]);
    sin_avisos("sea lista = [1]\nlista[0] = 2\n");
    // En el nivel principal y dentro de bloques.
    assert_eq!(codigos("sea n = 3\nsi n > 1 {\n    sea dentro = 1\n}\n"), vec!["A001"]);
}

#[test]
fn a002_parametro_que_nunca_se_usa() {
    let con = "funcion f(a, b) {\n    devolver a\n}\nmostrar(f(1, 2))\n";
    assert_eq!(avisos(con), vec![("A002", 1)]);
    sin_avisos("funcion f(a, _b) {\n    devolver a\n}\nmostrar(f(1, 2))\n");
    sin_avisos("funcion f(a, b) {\n    devolver a + b\n}\nmostrar(f(1, 2))\n");
}

#[test]
fn a003_funcion_que_nadie_llama() {
    let con = "funcion suelta() {\n    devolver 1\n}\nmostrar(2)\n";
    assert_eq!(avisos(con), vec![("A003", 1)]);
    // Usada como valor, desde otra función, desde una ruta o desde una prueba: no avisa.
    sin_avisos("funcion doble(x) {\n    devolver x * 2\n}\nsea f = doble\nmostrar(f(2))\n");
    sin_avisos("funcion a() {\n    devolver 1\n}\nfuncion b() {\n    devolver a()\n}\nmostrar(b())\n");
    sin_avisos("funcion a() {\n    devolver \"x\"\n}\nruta \"/\" {\n    devolver a()\n}\n");
    sin_avisos("funcion a() {\n    devolver 1\n}\nprueba \"a\" {\n    comprobar_igual(a(), 1)\n}\nmostrar(2)\n");
    // Una función que solo se llama a sí misma sigue sin usarse.
    assert_eq!(codigos("funcion r(n) {\n    si n > 0 {\n        devolver r(n - 1)\n    }\n    devolver 0\n}\nmostrar(1)\n"), vec!["A003"]);
    // Una pieza con solo funciones (para `usar`) no avisa: la llama otro archivo.
    sin_avisos("funcion suelta() {\n    devolver 1\n}\n");
    // Un tipo propio no es una función olvidada.
    sin_avisos("tipo Persona {\n    nombre: texto\n}\nmostrar(1)\n");
}

#[test]
fn a004_codigo_que_nunca_se_ejecuta() {
    let con = "funcion f() {\n    devolver 1\n    mostrar(2)\n}\nmostrar(f())\n";
    assert_eq!(avisos(con), vec![("A004", 3)]);
    // Un devolver dentro de un si seguido de código sí se ejecuta a veces.
    sin_avisos("funcion f(n) {\n    si n > 1 {\n        devolver 1\n    }\n    devolver 2\n}\nmostrar(f(1))\n");
    assert_eq!(codigos("para i en rango(1, 3) {\n    romper\n    mostrar(i)\n}\n"), vec!["A004"]);
    assert_eq!(codigos("para i en rango(1, 3) {\n    continuar\n    mostrar(i)\n}\n"), vec!["A004"]);
    assert_eq!(codigos("provocar_error(\"mal\")\nmostrar(1)\n"), vec!["A004"]);
    sin_avisos("para i en rango(1, 3) {\n    si i == 2 {\n        romper\n    }\n    mostrar(i)\n}\n");
}

#[test]
fn a005_variable_que_tapa_a_otra() {
    let con = "sea x = 1\nsi x > 0 {\n    sea x = 2\n    mostrar(x)\n}\nmostrar(x)\n";
    assert_eq!(avisos(con), vec![("A005", 3)]);
    let aviso = &deimon::avisos_archivo(con, Path::new("estricto.dmn"))[0];
    assert!(aviso.sugerencia.as_deref().unwrap().contains("x = ..."));
    // Cambiar la de fuera con = no avisa, ni declarar otra con otro nombre.
    sin_avisos("sea x = 1\nsi x > 0 {\n    x = 2\n}\nmostrar(x)\n");
    sin_avisos("sea x = 1\nsi x > 0 {\n    sea otra = 2\n    mostrar(otra)\n}\nmostrar(x)\n");
    // Otra función con la misma variable no cuenta: cada función tiene su ámbito.
    sin_avisos("funcion f() {\n    sea x = 1\n    devolver x\n}\nsea x = 2\nmostrar(f() + x)\n");
}

#[test]
fn a006_funcion_que_a_veces_no_devuelve() {
    let con = "funcion f(n) {\n    si n > 1 {\n        devolver 1\n    }\n}\nmostrar(f(2))\n";
    assert_eq!(avisos(con), vec![("A006", 1)]);
    sin_avisos("funcion f(n) {\n    si n > 1 {\n        devolver 1\n    } sino {\n        devolver 2\n    }\n}\nmostrar(f(2))\n");
    sin_avisos("funcion f(n) {\n    si n > 1 {\n        devolver 1\n    }\n    provocar_error(\"no\")\n}\nmostrar(f(2))\n");
    // Una función que nunca devuelve un valor es un procedimiento: no avisa.
    sin_avisos("funcion f(n) {\n    si n > 1 {\n        mostrar(n)\n    }\n}\nf(2)\n");
}

#[test]
fn a007_resultado_descartado() {
    let con = "sea nombre = \"ana\"\nmayusculas(nombre)\nmostrar(nombre)\n";
    assert_eq!(avisos(con), vec![("A007", 2)]);
    let aviso = &deimon::avisos_archivo(con, Path::new("estricto.dmn"))[0];
    assert!(aviso.sugerencia.as_deref().unwrap().contains("nombre = mayusculas(nombre)"));
    sin_avisos("sea nombre = \"ana\"\nsea m = mayusculas(nombre)\nmostrar(m)\n");
    sin_avisos("sea nombre = \"ana\"\nmostrar(mayusculas(nombre))\n");
    assert_eq!(codigos("sea l = [2, 1]\nordenar(l)\nagregar(l, 3)\nmostrar(l)\n"), vec!["A007", "A007"]);
    assert_eq!(codigos("sea t = \"a\"\na_texto(t)\nmedir(t)\nmostrar(t)\n"), vec!["A007", "A007"]);
    // Las que sí hacen algo (mostrar, guardar, esperar) nunca avisan.
    sin_avisos("mostrar(1)\nguardar(\"c\", \"k\", 1)\nesperar(1)\n");
    // Probar si algo falla dentro de intentar es un uso normal de a_numero.
    sin_avisos("intentar {\n    a_numero(\"x\")\n} si falla e {\n    mostrar(e[\"mensaje\"])\n}\n");
    // Una función propia con el nombre de una pura del módulo manda: puede tener efectos.
    sin_avisos("funcion recortar(t) {\n    mostrar(t)\n}\nrecortar(\"a\")\n");
}

#[test]
fn a008_comparar_tipos_distintos() {
    let con = "sea n = 1\nsea t = \"1\"\nsi n == t {\n    mostrar(1)\n}\n";
    assert_eq!(avisos(con), vec![("A008", 3)]);
    assert_eq!(codigos("sea n = 1\nmostrar(n != \"a\")\n"), vec!["A008"]);
    sin_avisos("sea n = 1\nsea m = 2\nsi n == m {\n    mostrar(1)\n}\n");
    // Comparar con nada es normal.
    sin_avisos("sea n = 1\nsi n == nada {\n    mostrar(1)\n}\n");
    // Si no se sabe el tipo, no avisa.
    sin_avisos("funcion f(a) {\n    devolver a == \"x\"\n}\nmostrar(f(\"x\"))\n");
}

#[test]
fn a009_condicion_constante() {
    assert_eq!(codigos("si verdadero {\n    mostrar(1)\n}\n"), vec!["A009"]);
    assert_eq!(codigos("si falso {\n    mostrar(1)\n}\n"), vec!["A009"]);
    assert_eq!(codigos("si 1 == 1 {\n    mostrar(1)\n}\n"), vec!["A009"]);
    assert_eq!(codigos("mientras falso {\n    mostrar(1)\n}\n"), vec!["A009"]);
    // El bucle sin fin con romper es normal; y una condición con variables también.
    sin_avisos("mientras verdadero {\n    romper\n}\n");
    sin_avisos("sea n = 1\nsi n > 0 {\n    mostrar(1)\n}\nmientras n < 3 {\n    n = n + 1\n}\n");
}

#[test]
fn sin_estricto_nada_cambia() {
    let carpeta = carpeta_tmp("igual");
    // Tiene avisos (variable sin leer y código muerto) pero ningún error.
    let programa = "funcion f() {\n    sea sobra = 1\n    devolver 2\n    mostrar(3)\n}\nmostrar(f())\n";
    fs::write(carpeta.join("a.dmn"), programa).unwrap();

    let normal = deimon(&carpeta, &["comprobar", "a.dmn"]);
    assert_eq!(salida(&normal), "✓ a.dmn no tiene errores\n");
    assert_eq!(normal.status.code(), Some(0));
    let json = deimon(&carpeta, &["comprobar", "a.dmn", "--json"]);
    assert_eq!(salida(&json), "{\"archivo\":\"a.dmn\",\"correcto\":true,\"errores\":[]}\n");
    assert_eq!(json.status.code(), Some(0));

    // Un error de siempre se escribe igual, sin "nivel".
    fs::write(carpeta.join("b.dmn"), "mostrar(x)\n").unwrap();
    let con_error = deimon(&carpeta, &["comprobar", "b.dmn", "--json"]);
    assert!(!salida(&con_error).contains("nivel"), "{}", salida(&con_error));
    assert_eq!(con_error.status.code(), Some(1));
}

#[test]
fn con_estricto_los_avisos_dan_codigo_1() {
    let carpeta = carpeta_tmp("estricto");
    let programa = "funcion f() {\n    sea sobra = 1\n    devolver 2\n    mostrar(3)\n}\nmostrar(f())\n";
    fs::write(carpeta.join("a.dmn"), programa).unwrap();

    let texto = deimon(&carpeta, &["comprobar", "a.dmn", "--estricto"]);
    let s = salida(&texto);
    assert_eq!(texto.status.code(), Some(1), "{}", s);
    assert!(s.contains("Aviso en la línea 2:") && s.contains("Código: A001"), "{}", s);
    assert!(s.contains("Aviso en la línea 4:") && s.contains("Código: A004"), "{}", s);
    assert!(s.contains("Sugerencia:") && s.contains("^"), "{}", s);
    assert!(s.contains("✗ a.dmn tiene 2 avisos"), "{}", s);

    let json = deimon(&carpeta, &["comprobar", "a.dmn", "--estricto", "--json"]);
    let j = salida(&json);
    assert_eq!(json.status.code(), Some(1), "{}", j);
    assert!(j.contains("\"correcto\":false"), "{}", j);
    assert!(j.contains("\"codigo\":\"A001\",\"nivel\":\"aviso\""), "{}", j);
    assert!(j.contains("\"codigo\":\"A004\",\"nivel\":\"aviso\""), "{}", j);

    // Un error y un aviso a la vez: el error lleva nivel "error".
    fs::write(carpeta.join("b.dmn"), "sea sobra = 1\nmostrar(x)\n").unwrap();
    let mezcla = deimon(&carpeta, &["comprobar", "b.dmn", "--estricto", "--json"]);
    let m = salida(&mezcla);
    assert!(m.contains("\"codigo\":\"E005\",\"nivel\":\"error\""), "{}", m);
    assert_eq!(mezcla.status.code(), Some(1));

    // Sin nada dudoso: código 0.
    fs::write(carpeta.join("c.dmn"), "sea _x = 1\nmostrar(2)\n").unwrap();
    let limpio = deimon(&carpeta, &["comprobar", "c.dmn", "--estricto"]);
    assert_eq!(salida(&limpio), "✓ c.dmn no tiene errores ni avisos\n");
    assert_eq!(limpio.status.code(), Some(0));
    let limpio_json = deimon(&carpeta, &["comprobar", "c.dmn", "--estricto", "--json"]);
    assert_eq!(salida(&limpio_json), "{\"archivo\":\"c.dmn\",\"correcto\":true,\"errores\":[]}\n");
}

#[test]
fn estricto_se_combina_con_arreglar() {
    let carpeta = carpeta_tmp("arreglar");
    fs::write(carpeta.join("a.dmn"), "sea sobra = 1\nmostrar(2)\n").unwrap();
    let o = deimon(&carpeta, &["comprobar", "a.dmn", "--arreglar", "--estricto", "--json"]);
    let s = salida(&o);
    assert!(s.contains("\"arreglos_aplicados\":0") && s.contains("A001"), "{}", s);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn la_guia_y_las_tablas_conocen_los_avisos() {
    for (codigo, _) in deimon::error::CODIGOS_AVISO {
        let docs = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/pruebas-y-errores.md")).unwrap();
        assert!(docs.contains(&format!("| {} |", codigo)), "{} no está en docs/pruebas-y-errores.md", codigo);
    }
    let guia = deimon::guia::texto_con_extensiones(&[]);
    assert!(guia.contains("deimon comprobar --estricto") && guia.contains("A009"));
}
