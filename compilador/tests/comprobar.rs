use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta_tmp(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("comprobar_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn deimon(carpeta: &PathBuf, argumentos: &[&str]) -> Output {
    Command::new(binario())
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

fn errores(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

fn primer_error(fuente: &str) -> deimon::error::ErrorTraduccion {
    let todos = deimon::comprobar(fuente);
    assert!(!todos.is_empty(), "se esperaba un error para: {}", fuente);
    todos.into_iter().next().unwrap()
}

#[test]
fn variable_inexistente_sugiere_el_nombre_parecido_con_arreglo() {
    let e = primer_error("sea total = 1\nmostrar(totl)\n");
    assert_eq!(e.codigo, "E005");
    assert_eq!((e.linea, e.columna), (2, 9));
    assert!(e.sugerencia.as_deref().unwrap().contains("¿Quisiste decir 'total'?"));
    let arreglo = e.arreglo.unwrap();
    assert_eq!(arreglo.reemplazo, "total");
    assert_eq!((arreglo.columna_inicio, arreglo.columna_fin), (9, 13));
    assert!(e.mensaje.contains("la variable 'totl' no existe"));
}

#[test]
fn funcion_inexistente_sugiere_el_nombre_parecido_con_arreglo() {
    let e = primer_error("funcion suma(a, b) {\n  devolver a + b\n}\nmostrar(sumaa(1, 2))\n");
    assert_eq!(e.codigo, "E006");
    assert!(e.sugerencia.as_deref().unwrap().contains("¿Quisiste decir 'suma'?"));
    assert_eq!(e.arreglo.unwrap().reemplazo, "suma");
    // También con las integradas.
    let e = primer_error("mostar(1)\n");
    assert!(e.sugerencia.as_deref().unwrap().contains("¿Quisiste decir 'mostrar'?"));
}

#[test]
fn el_nombre_muy_distinto_no_recibe_sugerencia_de_nombre() {
    let e = primer_error("sea total = 1\nmostrar(qwerty)\n");
    assert_eq!(e.codigo, "E005");
    assert!(e.arreglo.is_none());
    assert!(e.sugerencia.as_deref().unwrap().contains("sea qwerty = ..."));
}

#[test]
fn argumentos_incorrectos_muestran_la_firma() {
    let e = primer_error("funcion suma(a, b) {\n  devolver a + b\n}\nmostrar(suma(1))\n");
    assert_eq!(e.codigo, "E007");
    assert_eq!(e.linea, 4);
    assert!(e.mensaje.contains("espera 2 argumentos y se han pasado 1"));
    assert!(e.sugerencia.as_deref().unwrap().contains("suma(a, b)"));
    let e = primer_error("guardar(\"a\", 1)\n");
    assert!(e.sugerencia.as_deref().unwrap().contains("guardar(coleccion, clave, valor)"));
}

#[test]
fn las_palabras_de_otros_lenguajes_dicen_su_equivalente_exacto() {
    let casos: [(&str, &str, &str); 27] = [
        ("if verdadero {\n}\n", "if", "si"),
        ("si verdadero {\n} else {\n}\n", "else", "sino"),
        ("si verdadero {\n} elif falso {\n}\n", "elif", "sino si"),
        ("si verdadero {\n} elseif falso {\n}\n", "elseif", "sino si"),
        ("si verdadero {\n} else if falso {\n}\n", "else if", "sino si"),
        ("while verdadero {\n}\n", "while", "mientras"),
        ("for x in [1] {\n}\n", "for", "para"),
        ("function f() {\n}\n", "function", "funcion"),
        ("def f() {\n}\n", "def", "funcion"),
        ("fn f() {\n}\n", "fn", "funcion"),
        ("funcion f() {\n  return 1\n}\n", "return", "devolver"),
        ("let a = 1\n", "let", "sea"),
        ("var a = 1\n", "var", "sea"),
        ("const a = 1\n", "const", "sea"),
        ("print(1)\n", "print", "mostrar"),
        ("console.log(1)\n", "console.log", "mostrar"),
        ("sea a = true\n", "true", "verdadero"),
        ("sea a = false\n", "false", "falso"),
        ("sea a = null\n", "null", "nada"),
        ("sea a = None\n", "None", "nada"),
        ("sea a = verdadero and falso\n", "and", "y"),
        ("sea a = verdadero or falso\n", "or", "o"),
        ("sea a = not verdadero\n", "not", "no"),
        ("sea a = verdadero && falso\n", "&&", "y"),
        ("sea a = verdadero || falso\n", "||", "o"),
        ("sea a = !verdadero\n", "!", "no"),
        ("sea a = 1 === 1\n", "===", "=="),
    ];
    for (fuente, ajena, equivalente) in casos {
        let todos = deimon::comprobar(fuente);
        let e = todos
            .iter()
            .find(|e| e.codigo == "E008")
            .unwrap_or_else(|| panic!("sin E008 para {:?}: {:?}", fuente, todos));
        assert!(e.mensaje.contains(&format!("'{}'", ajena)), "{:?}: {}", fuente, e.mensaje);
        let sugerencia = e.sugerencia.as_deref().unwrap();
        assert!(
            sugerencia.contains(&format!("En Deimon se escribe '{}'", equivalente)),
            "{:?}: {}",
            fuente,
            sugerencia
        );
        assert!(e.arreglo.is_some(), "{:?} sin arreglo", fuente);
        // Cuando es un cambio directo, aplicar los arreglos deja el programa sin errores E008.
        let (arreglado, _, restantes) = deimon::arreglos::arreglar(fuente);
        assert!(
            restantes.iter().all(|r| r.codigo != "E008"),
            "{:?} -> {:?}: {:?}",
            fuente,
            arreglado,
            restantes
        );
    }
}

#[test]
fn llave_parentesis_y_comillas_sin_cerrar_dicen_donde_se_abrieron() {
    let e = primer_error("sea a = 1\nsi verdadero {\n  mostrar(a)\n");
    assert_eq!(e.codigo, "E004");
    assert!(e.mensaje.contains("se esperaba '}'"));
    assert!(e.mensaje.contains("línea 2"), "{}", e.mensaje);

    let e = primer_error("sea a = 1\nmostrar(a + 1\nmostrar(2)\n");
    assert_eq!(e.codigo, "E004");
    assert!(e.mensaje.contains("el '(' se abrió en la línea 2"), "{}", e.mensaje);

    let e = primer_error("sea a = [1, 2\n");
    assert_eq!(e.codigo, "E004");
    assert!(e.mensaje.contains("el '[' se abrió en la línea 1"), "{}", e.mensaje);

    let e = primer_error("sea a = 1\nmostrar(\"hola)\n");
    assert_eq!(e.codigo, "E001");
    assert_eq!(e.linea, 2);
    assert!(e.sugerencia.as_deref().unwrap().contains("línea 2"));
}

#[test]
fn asignar_a_una_variable_no_declarada_sugiere_sea() {
    let e = primer_error("cuenta = 5\n");
    assert_eq!(e.codigo, "E009");
    assert_eq!((e.linea, e.columna), (1, 1));
    assert!(e.sugerencia.as_deref().unwrap().contains("sea cuenta = ..."));
}

#[test]
fn el_formato_rico_conserva_la_primera_linea_y_dibuja_el_cursor() {
    let e = primer_error("sea total = 1\nmostrar(totl)\n");
    let texto = e.to_string();
    let lineas: Vec<&str> = texto.lines().collect();
    assert_eq!(lineas[0], "Error en la línea 2: la variable 'totl' no existe");
    assert_eq!(lineas[1], "  2 | mostrar(totl)");
    assert_eq!(lineas[2], "    |         ^");
    assert!(lineas[3].starts_with("Sugerencia: "));
    assert_eq!(lineas[4], "Código: E005");
}

#[test]
fn comprobar_recoge_varios_errores_a_la_vez() {
    let fuente = "funcion suma(a, b) {\n  devolver a + b\n}\nmostrar(totl)\nmostrar(suma(1))\nmostrar(sumaa(1, 2))\ncuenta = 1\n";
    let todos = deimon::comprobar(fuente);
    let codigos: Vec<&str> = todos.iter().map(|e| e.codigo).collect();
    assert_eq!(codigos, vec!["E005", "E007", "E006", "E009"], "{:?}", todos);
    let lineas: Vec<usize> = todos.iter().map(|e| e.linea).collect();
    assert_eq!(lineas, vec![4, 5, 6, 7]);

    let c = carpeta_tmp("varios");
    fs::write(c.join("varios.dmn"), fuente).unwrap();
    let o = deimon(&c, &["comprobar", "varios.dmn"]);
    assert_eq!(o.status.code(), Some(1));
    let texto = salida(&o);
    assert!(texto.contains("tiene 4 errores"), "{}", texto);
    for codigo in ["E005", "E006", "E007", "E009"] {
        assert!(texto.contains(codigo), "{}", texto);
    }
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn comprobar_un_programa_correcto_sale_con_cero() {
    let c = carpeta_tmp("bien");
    fs::write(c.join("bien.dmn"), "sea a = 1\nmostrar(a)\n").unwrap();
    let o = deimon(&c, &["comprobar", "bien.dmn"]);
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert!(salida(&o).contains("✓ bien.dmn no tiene errores"), "{}", salida(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn comprobar_json_tiene_la_forma_esperada() {
    let c = carpeta_tmp("json");
    fs::write(c.join("mal.dmn"), "mostrar(totl)\n").unwrap();
    fs::write(c.join("bien.dmn"), "mostrar(1)\n").unwrap();
    let o = deimon(&c, &["comprobar", "mal.dmn", "--json"]);
    assert_eq!(o.status.code(), Some(1));
    let texto = salida(&o);
    assert!(texto.starts_with('{'), "{}", texto);
    assert!(texto.contains("\"errores\""), "{}", texto);
    assert!(texto.contains("\"correcto\":false"), "{}", texto);
    assert!(texto.contains("\"codigo\":\"E005\""), "{}", texto);
    assert!(texto.contains("\"arreglo\":null"), "{}", texto);

    let o = deimon(&c, &["comprobar", "bien.dmn", "--json"]);
    assert_eq!(o.status.code(), Some(0));
    let texto = salida(&o);
    assert!(texto.contains("\"correcto\":true") && texto.contains("\"errores\":[]"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn arreglar_corrige_una_copia_que_despues_compila_y_se_ejecuta() {
    let c = carpeta_tmp("arreglar");
    let archivo = c.join("copia.dmn");
    fs::write(
        &archivo,
        "funcion doble(n) {\n    return n * 2\n}\nsea total = doble(4)\nif total == 8 {\n    mostrar(totl)\n} else {\n    mostrar(0)\n}\n",
    )
    .unwrap();
    let o = deimon(&c, &["comprobar", "copia.dmn", "--arreglar"]);
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert!(salida(&o).contains("Arreglos automáticos aplicados: 4"), "{}", salida(&o));
    let nuevo = fs::read_to_string(&archivo).unwrap();
    assert!(nuevo.contains("devolver n * 2"), "{}", nuevo);
    assert!(nuevo.contains("si total == 8 {"), "{}", nuevo);
    assert!(nuevo.contains("} sino {"), "{}", nuevo);
    assert!(nuevo.contains("mostrar(total)"), "{}", nuevo);

    let o = deimon(&c, &["comprobar", "copia.dmn"]);
    assert_eq!(o.status.code(), Some(0));
    let o = deimon(&c, &["ejecutar", "copia.dmn"]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "8\n");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn error_en_ejecucion_con_la_linea_de_la_sentencia() {
    let c = carpeta_tmp("ejecucion");
    // En el nivel principal.
    fs::write(c.join("a.dmn"), "mostrar(1)\n\nsea x = 5\nmostrar(x / 0)\n").unwrap();
    let o = deimon(&c, &["ejecutar", "a.dmn"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        errores(&o).starts_with("Error en la línea 4 (en ejecución): no se puede dividir entre cero"),
        "{}",
        errores(&o)
    );
    // Dentro de una función (y no la línea de la llamada).
    fs::write(
        c.join("b.dmn"),
        "funcion mala(n) {\n  sea a = 1\n  devolver a + n[0]\n}\n\nmostrar(mala(1))\n",
    )
    .unwrap();
    let o = deimon(&c, &["ejecutar", "b.dmn"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).starts_with("Error en la línea 3 (en ejecución): "), "{}", errores(&o));
    // Tras volver de una función, el error es de la línea de la llamada.
    fs::write(
        c.join("c.dmn"),
        "funcion uno() {\n  devolver 1\n}\n\nsea v = uno() / 0\n",
    )
    .unwrap();
    let o = deimon(&c, &["ejecutar", "c.dmn"]);
    assert!(errores(&o).starts_with("Error en la línea 5 (en ejecución): "), "{}", errores(&o));
    // En la condición de un bucle, tras ejecutar el cuerpo.
    fs::write(
        c.join("d.dmn"),
        "sea n = 2\nmientras 10 / n > 1 {\n  n = n - 1\n  mostrar(n)\n}\n",
    )
    .unwrap();
    let o = deimon(&c, &["ejecutar", "d.dmn"]);
    assert_eq!(salida(&o), "1\n0\n", "{}", errores(&o));
    assert!(errores(&o).starts_with("Error en la línea 2 (en ejecución): "), "{}", errores(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn error_en_ejecucion_dentro_de_una_prueba_dice_la_linea() {
    let c = carpeta_tmp("prueba");
    fs::write(
        c.join("p.dmn"),
        "funcion parte(a) {\n  devolver 10 / a\n}\n\nprueba \"cero\" {\n  sea r = 1\n  comprobar_igual(parte(0), 1)\n}\n",
    )
    .unwrap();
    let o = deimon(&c, &["probar", "p.dmn"]);
    assert_eq!(o.status.code(), Some(1));
    let texto = salida(&o);
    assert!(
        texto.contains("✗ cero — error en ejecución: no se puede dividir entre cero (línea 2)"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn la_guia_contiene_todas_las_integradas_y_el_generador_las_acepta() {
    let texto = deimon::guia::texto();
    let json = deimon::guia::json();
    assert!(json.starts_with('{') && json.contains("\"integradas\""));
    for integrada in deimon::integradas::INTEGRADAS {
        assert!(texto.contains(&integrada.firma()), "falta {} en la guía", integrada.nombre);
        assert!(json.contains(&format!("\"nombre\":\"{}\"", integrada.nombre)));
        // El generador acepta la integrada con su número de argumentos...
        let argumentos = vec!["1"; integrada.parametros.len()].join(", ");
        let fuente = format!(
            "ruta \"/\" {{\n  devolver \"x\"\n}}\n{}({})\n",
            integrada.nombre, argumentos
        );
        assert!(deimon::traducir(&fuente).is_ok(), "{}: {:?}", integrada.nombre, deimon::traducir(&fuente));
        // ...y rechaza un número distinto.
        let mal = format!("ruta \"/\" {{\n  devolver \"x\"\n}}\n{}()\n", integrada.nombre);
        if !integrada.parametros.is_empty() {
            let error = deimon::traducir(&mal).unwrap_err();
            assert_eq!(error.codigo, "E007", "{}", integrada.nombre);
        }
    }
    for palabra in ["DEIMON_PUERTO", "DEIMON_DATOS", "comprobar", "guia", "if -> si", "E005"] {
        assert!(texto.contains(palabra), "falta {} en la guía", palabra);
    }
}

#[test]
fn las_palabras_clave_de_la_guia_son_palabras_reservadas_del_lexer() {
    for (palabra, _) in deimon::guia::PALABRAS_CLAVE {
        let tokens = deimon::lexer::tokenizar(palabra).unwrap();
        assert!(
            !matches!(tokens[0].tipo, deimon::lexer::Tipo::Ident(_)),
            "'{}' no es palabra reservada",
            palabra
        );
    }
}

#[test]
fn la_orden_guia_imprime_texto_y_json() {
    let c = carpeta_tmp("guia");
    let o = deimon(&c, &["guia"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(salida(&o).contains("mostrar(valor)"));
    let o = deimon(&c, &["guia", "--json"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(salida(&o).starts_with('{'));
    let _ = fs::remove_dir_all(&c);
}
