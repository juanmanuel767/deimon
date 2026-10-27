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

#[test]
fn texto_con_valores_basico_y_con_expresiones() {
    igual_en_los_dos_motores(
        "texto_valores",
        r#"sea nombre = "Ana"
sea edad = 30
sea p = {"nombre": "Luis"}
funcion suma(a, b) {
    devolver a + b
}
mostrar(f"Hola {nombre}, tienes {edad + 1} años")
mostrar(f"{p["nombre"]} suma {suma(2, 3)} y mide {medir([1, 2])}")
mostrar(f"lista {[1, nombre]} mapa { {"x": 1} } x {2.5} {verdadero} {falso} {nada}")
mostrar(f"grande {9007199254740993 + 0} y {123456789012345678901234567890}")
mostrar(f"{{llaves}} y }}{{ \"q\" \\ \t|")
mostrar(f"{nombre}{nombre}")
mostrar(f"sin valores")
sea f = 3
mostrar(f)
mostrar("body{color:red}")
mostrar("{nombre}")
"#,
        "Hola Ana, tienes 31 años\nLuis suma 5 y mide 2\nlista [1, Ana] mapa {x: 1} x 2.5 verdadero falso nada\ngrande 9007199254740993 y 123456789012345678901234567890\n{llaves} y }{ \"q\" \\ \t|\nAnaAna\nsin valores\n3\nbody{color:red}\n{nombre}\n",
    );
}

#[test]
fn f_sigue_siendo_un_nombre_normal() {
    igual_en_los_dos_motores(
        "texto_f_nombre",
        "funcion doble(x) {\n    devolver x * 2\n}\nsea f = doble\nmostrar(f(4))\nmostrar(f(4) + 1)\n",
        "8\n9\n",
    );
}

#[test]
fn texto_de_varias_lineas_recorta_la_sangria() {
    igual_en_los_dos_motores(
        "texto_lineas",
        "sea t = \"\"\"\n    Hola\n      Ana\n    \"\"\"\nmostrar(t)\nmostrar(medir(t))\nsea u = \"\"\"a \"b\" \\n c\"\"\"\nmostrar(u)\nsea n = 5\nsea v = f\"\"\"\n    Total: {n}\n      {\"x\" + \"y\"} {{ok}}\n    \"\"\"\nmostrar(v)\nmostrar(1 + 1)\n",
        "Hola\n  Ana\n10\na \"b\" \n c\nTotal: 5\n  xy {ok}\n2\n",
    );
}

#[test]
fn las_lineas_siguen_bien_despues_de_un_texto_de_varias_lineas() {
    let programa = "sea t = \"\"\"\n  uno\n  dos\n  \"\"\"\nmostrar(sin_declarar)\n";
    let error = error_de_ejecutar("texto_lineas_error", programa);
    assert!(error.contains("línea 5"), "{}", error);
}

#[test]
fn variable_inexistente_dentro_de_llaves_se_avisa_antes_de_ejecutar() {
    let texto = revisar(
        "texto_variable_falta",
        "sea nombre = \"Ana\"\nmostrar(\"antes\")\nmostrar(f\"Hola {nombre} {edda}\")\n",
    );
    assert!(texto.contains("edda"), "{}", texto);
    assert!(texto.contains("línea 3"), "{}", texto);
}

#[test]
fn funcion_inexistente_dentro_de_llaves_sugiere_la_parecida() {
    let texto = revisar("texto_funcion_falta", "mostrar(f\"{mediir([1])}\")\n");
    assert!(texto.contains("medir"), "{}", texto);
}

#[test]
fn llave_sin_cerrar_vacia_o_suelta_son_errores_claros() {
    let sin_cerrar = revisar("texto_llave_abierta", "sea a = 1\nmostrar(f\"hola {a\")\n");
    assert!(sin_cerrar.contains("no está cerrada"), "{}", sin_cerrar);
    assert!(sin_cerrar.contains("línea 2") && sin_cerrar.contains("E028"), "{}", sin_cerrar);
    let vacia = revisar("texto_llave_vacia", "mostrar(f\"hola {}\")\n");
    assert!(vacia.contains("vacías") && vacia.contains("E028"), "{}", vacia);
    let suelta = revisar("texto_llave_suelta", "mostrar(f\"hola }\")\n");
    assert!(suelta.contains("suelta") && suelta.contains("E028"), "{}", suelta);
}

#[test]
fn tres_comillas_sin_cerrar_dice_la_linea_donde_se_abrio() {
    let texto = revisar("texto_triple_abierto", "mostrar(1)\nsea t = \"\"\"\nhola\nmundo\n");
    assert!(texto.contains("línea 2"), "{}", texto);
    assert!(texto.contains("varias líneas no está cerrado"), "{}", texto);
}

#[test]
fn formatear_conserva_los_textos_con_valores_y_de_varias_lineas() {
    let fuente = "sea n = 1\nsea a = f\"x {n+1} {n}\"\nsea b = \"\"\"\n    hola\n      {n}\n    \"\"\"\nsea c = f\"\"\"\n    v: {n}\n    \"\"\"\nmostrar(a)\n";
    let una = deimon::formato::formatear(fuente).unwrap();
    assert!(una.contains("f\"x {n+1} {n}\""), "{}", una);
    assert!(una.contains("\"\"\"\n    hola\n      {n}\n    \"\"\""), "{}", una);
    assert!(una.contains("f\"\"\"\n    v: {n}\n    \"\"\""), "{}", una);
    assert_eq!(deimon::formato::formatear(&una).unwrap(), una);
}

// --- Punto 6: comprobar tipos antes de ejecutar en todo el programa ---

/// Revisa el programa con `deimon comprobar` y devuelve lo que escribe, sin exigir que falle.
fn salida_de_comprobar(nombre: &str, programa: &str) -> (bool, String) {
    let carpeta = carpeta(nombre);
    let archivo = carpeta.join(format!("{}.dmn", nombre));
    fs::write(&archivo, programa).expect("no se pudo escribir el programa");
    let salida = Command::new(binario()).arg("comprobar").arg(&archivo).output().expect("deimon comprobar");
    let texto = format!("{}{}", String::from_utf8_lossy(&salida.stdout), String::from_utf8_lossy(&salida.stderr));
    (salida.status.success(), texto)
}

/// El programa es válido: `deimon comprobar` no da ningún aviso de tipos.
fn sin_aviso_de_tipos(nombre: &str, programa: &str) {
    let (valido, texto) = salida_de_comprobar(nombre, programa);
    assert!(valido && !texto.contains("E025"), "no debía avisar:\n{}", texto);
}

#[test]
fn lo_que_devuelve_una_funcion_propia_se_conoce_antes_de_ejecutar() {
    let directo = revisar("devuelve_directo", "funcion nombre() {\n    devolver \"Ana\"\n}\nmostrar(nombre() - 1)\n");
    assert!(directo.contains("línea 4: no se puede restar texto y número (se ve sin ejecutar el programa)"), "{}", directo);
    assert!(directo.contains("E025"), "{}", directo);
    let con_variable = revisar("devuelve_variable", "funcion g() {\n    devolver \"x\"\n}\nsea v = g()\nmostrar(v * 2)\n");
    assert!(con_variable.contains("línea 5: no se puede multiplicar texto y número"), "{}", con_variable);
    // La función se define después de la llamada.
    let despues = revisar("devuelve_despues", "mostrar(g() - 1)\nfuncion g() {\n    devolver \"x\"\n}\n");
    assert!(despues.contains("línea 1: no se puede restar texto y número"), "{}", despues);
    // Una función que devuelve lo que devuelve otra.
    let encadenada = revisar("devuelve_cadena", "funcion a() {\n    devolver b()\n}\nfuncion b() {\n    devolver 1 < 2\n}\nmostrar(a() + 1)\n");
    assert!(encadenada.contains("no se puede sumar lógico y número"), "{}", encadenada);
    // Al ejecutar falla igual.
    assert!(error_de_ejecutar("devuelve_ejecutar", "funcion nombre() {\n    devolver \"Ana\"\n}\nmostrar(nombre() - 1)\n").contains("no se puede restar texto y número"));
}

#[test]
fn los_tipos_de_los_argumentos_se_siguen_dentro_de_la_funcion() {
    let programa = "funcion doble(x) {\n    devolver x * 2\n}\nmostrar(doble(\"a\"))\nmostrar(doble(\"b\"))\nmostrar(doble(3))\n";
    let texto = revisar("entra_en_funcion", programa);
    assert!(
        texto.contains("línea 2: no se puede multiplicar texto y número (con doble(\"a\") de la línea 4; se ve sin ejecutar el programa)"),
        "{}",
        texto
    );
    assert_eq!(texto.matches("no se puede multiplicar").count(), 1, "el mismo aviso no se repite:\n{}", texto);
    assert!(error_de_ejecutar("entra_en_funcion_ejecutar", programa).contains("no se puede multiplicar texto y número"));
    // Pasa de una función a otra.
    let encadenada = revisar(
        "entra_en_cadena",
        "funcion doble(x) {\n    devolver x * 2\n}\nfuncion uno(valor) {\n    devolver doble(valor)\n}\nmostrar(uno(verdadero))\n",
    );
    assert!(encadenada.contains("línea 2: no se puede multiplicar lógico y número (con uno(verdadero) de la línea 7;"), "{}", encadenada);
}

#[test]
fn argumentos_de_las_integradas_y_indices_se_avisan_con_el_mensaje_de_ejecutar() {
    let medir = revisar("integrada_medir", "mostrar(medir(5))\n");
    assert!(medir.contains("medir() espera una lista o un texto, no número (se ve sin ejecutar el programa)"), "{}", medir);
    assert!(error_de_ejecutar("integrada_medir_ejecutar", "mostrar(medir(5))\n").contains("medir() espera una lista o un texto, no número"));
    let indice = revisar("indice_texto_en_lista", "sea l = [1, 2]\nmostrar(l[\"a\"])\n");
    assert!(indice.contains("línea 2: el índice debe ser un número entero, no texto"), "{}", indice);
    assert!(error_de_ejecutar("indice_texto_en_lista_ejecutar", "sea l = [1, 2]\nmostrar(l[\"a\"])\n").contains("el índice debe ser un número entero, no texto"));
    let en_texto = revisar("indice_texto_en_texto", "sea t = \"hola\"\nmostrar(t[\"a\"])\n");
    assert!(en_texto.contains("el índice debe ser un número entero, no texto"), "{}", en_texto);
}

/// Un valor de cada tipo, escrito tal cual (así el revisor sabe seguro de qué tipo es).
fn muestra_de_tipo(tipo: &str) -> &'static str {
    match tipo {
        "número" => "5",
        "texto" => "\"abc\"",
        "lógico" => "verdadero",
        "nada" => "nada",
        "lista" => "[1, 2]",
        "lista con claves" => "{\"a\": 1}",
        otro => panic!("tipo sin muestra: {}", otro),
    }
}

const TIPOS_DE_MUESTRA: [&str; 6] = ["número", "texto", "lógico", "nada", "lista", "lista con claves"];

/// Los argumentos de la llamada: el de la regla es `probado`; los demás, valores que la tabla admite.
fn argumentos_para(integrada: &str, reglas: &[deimon::tipos::ReglaArgumento], regla: &deimon::tipos::ReglaArgumento, probado: &str) -> String {
    let cuantos = deimon::integradas::buscar(integrada).expect("la integrada existe").parametros.len();
    let argumentos: Vec<String> = (0..cuantos)
        .map(|posicion| {
            if posicion == regla.posicion {
                return muestra_de_tipo(probado).to_string();
            }
            if let Some((de, tipo)) = regla.cuando {
                if de == posicion {
                    return muestra_de_tipo(tipo).to_string();
                }
            }
            match reglas.iter().find(|otra| otra.posicion == posicion) {
                Some(otra) => muestra_de_tipo(otra.permitidos[0]).to_string(),
                None => "\"x\"".to_string(),
            }
        })
        .collect();
    argumentos.join(", ")
}

#[test]
fn la_tabla_de_integradas_no_miente() {
    let mut casos = 0;
    for (integrada, reglas) in deimon::tipos::TABLA_ARGUMENTOS {
        for regla in reglas.iter() {
            for tipo in TIPOS_DE_MUESTRA {
                let programa = format!("mostrar({}({}))\n", integrada, argumentos_para(integrada, reglas, regla, tipo));
                let nombre = format!("tabla_{}_{}_{}", integrada, regla.posicion, casos);
                casos += 1;
                let (_, comprobado) = salida_de_comprobar(&nombre, &programa);
                if regla.permitidos.contains(&tipo) {
                    // Tipo permitido: no hay aviso.
                    assert!(!comprobado.contains("E025"), "{} no debía avisar:\n{}", programa, comprobado);
                    continue;
                }
                let mensaje = regla.mensaje.replace("{}", tipo);
                assert!(comprobado.contains("E025") && comprobado.contains(&mensaje), "{} debía avisar «{}»:\n{}", programa, mensaje, comprobado);
                let ejecutado = error_de_ejecutar(&format!("{}_ejecutar", nombre), &programa);
                assert!(ejecutado.contains(&mensaje), "{} al ejecutar dio otro mensaje (esperado «{}»):\n{}", programa, mensaje, ejecutado);
            }
        }
    }
    assert!(casos > 50, "la tabla parece vacía");
}

#[test]
fn la_tabla_calla_cuando_no_esta_segura() {
    // Un argumento de tipo desconocido (un parámetro) nunca avisa.
    sin_aviso_de_tipos("tabla_desconocido", "funcion f(x) {\n    devolver medir(x)\n}\nmostrar(f(\"abc\"))\n");
    // `incluye` en un texto busca un texto, pero en una lista vale cualquier cosa.
    sin_aviso_de_tipos("tabla_incluye_lista", "mostrar(incluye([1, 2], 1))\nmostrar(incluye(\"abc\", \"b\"))\n");
    // Una lista vacía admite un índice de texto (es también una lista con claves vacía).
    sin_aviso_de_tipos("indice_en_lista_vacia", "sea d = []\nmostrar(d[\"a\"])\nmostrar(d[\"a\"] == nada)\n");
    sin_aviso_de_tipos("indice_en_lista_con_claves", "sea d = {\"a\": 1}\nmostrar(d[\"a\"])\nmostrar(d[0])\n");
}

#[test]
fn lo_que_no_se_sabe_con_seguridad_no_avisa() {
    // A veces texto y a veces número.
    sin_aviso_de_tipos(
        "mezcla_de_tipos",
        "funcion a(x) {\n    si x > 1 {\n        devolver \"s\"\n    }\n    devolver 3\n}\nmostrar(a(1) - 1)\n",
    );
    // No devuelve en todos los caminos (devuelve nada en alguno).
    sin_aviso_de_tipos(
        "sin_devolver_en_todo",
        "funcion b(x) {\n    si x > 1 {\n        devolver \"s\"\n    }\n}\nmostrar(b(1) - 1)\n",
    );
    // Dos devolver de tipos distintos.
    sin_aviso_de_tipos(
        "dos_devolver_distintos",
        "funcion c(x) {\n    si x > 1 {\n        devolver \"s\"\n    } sino {\n        devolver 2\n    }\n}\nmostrar(c(1) - 1)\n",
    );
    // Parámetro usado como número y llamado con número.
    sin_aviso_de_tipos("parametro_numero", "funcion doble(x) {\n    devolver x * 2\n}\nmostrar(doble(4) + 1)\n");
    // Recursiva llamada con número.
    sin_aviso_de_tipos(
        "recursiva",
        "funcion fact(n) {\n    si n <= 1 {\n        devolver 1\n    }\n    devolver n * fact(n - 1)\n}\nmostrar(fact(5) - 1)\n",
    );
    // Funciones como valores: no se sabe qué función es.
    sin_aviso_de_tipos(
        "funcion_como_valor",
        "funcion doble(x) {\n    devolver x * 2\n}\nsea f = doble\nmostrar(f(\"a\"))\nmostrar(f(3) - 1)\n",
    );
    // Una función que se llama con un parámetro de tipo desconocido.
    sin_aviso_de_tipos(
        "argumento_desconocido",
        "funcion doble(x) {\n    devolver x * 2\n}\nfuncion triple(valor) {\n    devolver doble(valor) + valor\n}\nmostrar(triple(2))\n",
    );
}

/// Todos los `.dmn` de una carpeta (y sus subcarpetas).
fn programas_de(carpeta: &std::path::Path, lista: &mut Vec<PathBuf>) {
    let Ok(entradas) = fs::read_dir(carpeta) else { return };
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.is_dir() {
            if ruta.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            programas_de(&ruta, lista);
        } else if ruta.extension().is_some_and(|e| e == "dmn") {
            lista.push(ruta);
        }
    }
}

#[test]
fn los_programas_del_repositorio_no_ganan_avisos_de_tipos() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut programas = Vec::new();
    for carpeta in ["ejemplos", "ejemplos-web", "ejemplos-nucleo", "proyectos"] {
        programas_de(&raiz.join(carpeta), &mut programas);
    }
    assert!(programas.len() > 20, "no se encontraron los ejemplos");
    for programa in programas {
        let salida = Command::new(binario())
            .arg("comprobar")
            .arg(programa.file_name().expect("nombre"))
            .current_dir(programa.parent().expect("carpeta"))
            .output()
            .expect("deimon comprobar");
        let texto = format!("{}{}", String::from_utf8_lossy(&salida.stdout), String::from_utf8_lossy(&salida.stderr));
        assert!(!texto.contains("E025"), "{} no debía tener avisos de tipos:\n{}", programa.display(), texto);
    }
}

#[test]
fn solo_avisa_de_lo_que_se_ejecuta_seguro_en_la_llamada() {
    // Con modo 2 nunca se llega a `x - 1`.
    sin_aviso_de_tipos(
        "llamada_con_si",
        "funcion p(x, modo) {\n    si modo == 1 {\n        devolver x - 1\n    }\n    devolver x\n}\nmostrar(p(\"a\", 2))\n",
    );
    // Lo que falla dentro de `intentar` lo atrapa `si falla`.
    sin_aviso_de_tipos(
        "dentro_de_intentar",
        "intentar {\n    mostrar(\"a\" - 1)\n} si falla e {\n    mostrar(e[\"mensaje\"])\n}\n",
    );
    sin_aviso_de_tipos(
        "llamada_dentro_de_intentar",
        "funcion doble(x) {\n    devolver x * 2\n}\nintentar {\n    mostrar(doble(\"a\"))\n} si falla e {\n    mostrar(e[\"mensaje\"])\n}\n",
    );
    // Una sentencia del nivel superior del cuerpo sí se ejecuta seguro.
    let texto = revisar(
        "llamada_nivel_superior",
        "funcion q(x) {\n    sea r = x - 1\n    mostrar(r)\n    devolver r\n}\nmostrar(q(\"a\"))\n",
    );
    assert!(texto.contains("línea 2: no se puede restar texto y número (con q(\"a\") de la línea 6;"), "{}", texto);
}
