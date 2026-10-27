// Pruebas del intérprete propio (`deimon ejecutar` sin rustc): da la misma salida que el
// modo Rust y arranca al instante.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta_ejemplos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ejemplos")
}

fn carpeta_datos(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_interprete_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    carpeta
}

fn archivo_temporal(nombre: &str, contenido: &str) -> PathBuf {
    let carpeta = std::env::temp_dir();
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    let ruta = carpeta.join(format!("ln_interprete_{}_{}.dmn", std::process::id(), nombre));
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

/// Ejecuta un programa con el motor indicado ("interprete" o "rust").
fn ejecutar_con(motor: &str, ruta: &PathBuf, datos: &PathBuf, motivo: bool) -> Output {
    let mut orden = Command::new(binario());
    orden
        .arg("ejecutar")
        .arg(ruta)
        .env("DEIMON_MOTOR", motor)
        .env("DEIMON_DATOS", datos);
    if motivo {
        orden.env("DEIMON_MOTOR_MOTIVO", "1");
    }
    orden.output().expect("no se pudo lanzar el binario 'Deimon'")
}

/// Ejecuta el programa con los dos motores y exige salida, errores y código de salida iguales.
/// Devuelve (salida, errores, código).
fn comparar_motores(nombre: &str, programa: &str) -> (String, String, Option<i32>) {
    let ruta = archivo_temporal(nombre, programa);
    let datos = carpeta_datos(nombre);
    let interprete = ejecutar_con("interprete", &ruta, &datos, false);
    let rust = ejecutar_con("rust", &ruta, &datos, false);
    let _ = fs::remove_file(&ruta);

    let texto = |bytes: &[u8]| String::from_utf8_lossy(bytes).to_string();
    assert_eq!(
        texto(&interprete.stdout),
        texto(&rust.stdout),
        "'{}': la salida del intérprete no coincide con la del modo Rust",
        nombre
    );
    assert_eq!(
        texto(&interprete.stderr),
        texto(&rust.stderr),
        "'{}': los errores del intérprete no coinciden con los del modo Rust",
        nombre
    );
    assert_eq!(
        interprete.status.code(),
        rust.status.code(),
        "'{}': el código de salida no coincide",
        nombre
    );
    (
        texto(&interprete.stdout),
        texto(&interprete.stderr),
        interprete.status.code(),
    )
}

#[test]
fn los_ejemplos_dan_lo_esperado_con_el_interprete() {
    let datos = carpeta_datos("ejemplos");
    let mut ejemplos: Vec<PathBuf> = fs::read_dir(carpeta_ejemplos())
        .expect("no se pudo leer la carpeta de ejemplos")
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.is_file() && ruta.extension().map_or(false, |e| e == "dmn"))
        .collect();
    ejemplos.sort();
    assert!(!ejemplos.is_empty());

    for ruta in ejemplos {
        let nombre = ruta.file_stem().unwrap().to_string_lossy().to_string();
        let esperado = match fs::read_to_string(carpeta_ejemplos().join(format!("{}.esperado", nombre))) {
            Ok(esperado) => esperado,
            Err(_) => continue,
        };
        // Los que usan algo que el intérprete aún no soporta caen solos al modo Rust.
        let salida = ejecutar_con("interprete", &ruta, &datos, false);
        let errores = String::from_utf8_lossy(&salida.stderr).to_string();
        assert!(salida.status.success(), "el ejemplo '{}' falló: {}", nombre, errores);
        assert!(errores.is_empty(), "el ejemplo '{}' escribió en stderr: {}", nombre, errores);
        assert_eq!(
            String::from_utf8_lossy(&salida.stdout),
            esperado,
            "la salida del ejemplo '{}' no coincide",
            nombre
        );
    }
}

#[test]
fn lo_que_no_soporta_cae_al_modo_rust() {
    // `pantallas` es de Android: el intérprete no lo lleva y pasa al modo Rust.
    let ruta = archivo_temporal(
        "con_pantallas",
        "usar modulo \"pantallas\"\nmostrar(\"sigue funcionando\")\n",
    );
    let datos = carpeta_datos("con_pantallas");
    let salida = ejecutar_con("interprete", &ruta, &datos, true);
    let _ = fs::remove_file(&ruta);
    assert!(salida.status.success(), "{}", String::from_utf8_lossy(&salida.stderr));
    assert_eq!(String::from_utf8_lossy(&salida.stdout), "sigue funcionando\n");
    assert!(
        String::from_utf8_lossy(&salida.stderr).contains("(modo Rust: usa el módulo 'pantallas' (es de Android))"),
        "debería haber caído al modo Rust: {}",
        String::from_utf8_lossy(&salida.stderr)
    );

    // Y un programa sencillo sí se interpreta (sin aviso de caída).
    let ruta = archivo_temporal("sencillo", "mostrar(1 + 1)\n");
    let salida = ejecutar_con("interprete", &ruta, &datos, true);
    let _ = fs::remove_file(&ruta);
    assert_eq!(String::from_utf8_lossy(&salida.stdout), "2\n");
    assert_eq!(String::from_utf8_lossy(&salida.stderr), "");
}

#[test]
fn los_errores_de_analisis_son_los_mismos_en_los_dos_motores() {
    let programa = "sea a = 1\nmostrar(b)\n";
    let ruta = archivo_temporal("error_nombre", programa);
    let datos = carpeta_datos("error_nombre");
    let interprete = ejecutar_con("interprete", &ruta, &datos, false);
    let rust = ejecutar_con("rust", &ruta, &datos, false);
    let _ = fs::remove_file(&ruta);
    assert_eq!(interprete.status.code(), Some(1));
    assert_eq!(interprete.stderr, rust.stderr);
    assert!(String::from_utf8_lossy(&interprete.stderr).contains("E005"));
}

#[test]
fn cuentas_mezclando_enteros_y_decimales() {
    let (salida, errores, codigo) = comparar_motores(
        "cuentas",
        r#"mostrar(1 + 2.5)
mostrar(10 / 4)
mostrar(7 % 3)
mostrar(7.5 % 2)
mostrar(-5 % 3)
mostrar(0.1 + 0.2)
mostrar(1 / 3)
mostrar(2 * 3.0)
mostrar(100 / 3)
mostrar(1 / 7)
mostrar(1 / 4000000)
mostrar(1000000000000000 * 10)
mostrar(123456789012345678)
mostrar(9007199254740993 + 1)
mostrar(0.1 * 3)
mostrar(5 - 7.25)
mostrar(-0.5)
mostrar(3 == 3.0)
mostrar(0.1 + 0.2 == 0.3)
mostrar(2 < 2.5)
mostrar("n=" + 1.5 + 2)
mostrar(a_texto(2.5) + "!")
mostrar(a_numero("21") * 2)
mostrar(a_numero("2.50") + 1)
mostrar([1, 2.5, 3.0, -4])
mostrar({"a": 1.5, "b": 2})
funcion media(a, b) {
    devolver (a + b) / 2
}
funcion cubo(n) {
    devolver n * n * n
}
mostrar(media(3, 4))
mostrar(media(1.5, 2))
mostrar(cubo(1.5))
mostrar(cubo(100000))
"#,
    );
    assert_eq!(codigo, Some(0), "{}", errores);
    assert!(salida.contains("3.5\n2.5\n1\n1.5\n"), "{}", salida);
}

#[test]
fn desbordamiento_infinito_y_nan() {
    let (salida, errores, codigo) = comparar_motores(
        "infinito",
        r#"sea x = 1
sea i = 0
mientras i < 400 {
    x = x * 10.5
    i = i + 1
}
mostrar(x)
mostrar(-x)
mostrar(x - x)
mostrar(x / x)
mostrar(x == x)
sea nan = x - x
mostrar(nan == nan)
mostrar(nan < 1)
mostrar(x > 1)
funcion potencia(base, veces) {
    sea r = 1
    sea k = 0
    mientras k < veces {
        r = r * base
        k = k + 1
    }
    devolver r
}
mostrar(potencia(2, 62))
mostrar(potencia(2, 64))
mostrar(potencia(2, 1100))
mostrar(potencia(10, 15))
mostrar(potencia(10, 16))
mostrar(potencia(10, 22))
mostrar(potencia(0.5, 30))
"#,
    );
    assert_eq!(codigo, Some(0), "{}", errores);
    assert!(salida.contains("inf"), "{}", salida);
    assert!(salida.contains("NaN"), "{}", salida);
}

fn error_en_los_dos(nombre: &str, programa: &str, texto: &str) {
    let (_, errores, codigo) = comparar_motores(nombre, programa);
    assert_eq!(codigo, Some(1), "'{}'", nombre);
    assert!(errores.contains(texto), "'{}': {}", nombre, errores);
}

#[test]
fn errores_en_tiempo_de_ejecucion_iguales_en_los_dos_motores() {
    error_en_los_dos(
        "dividir_cero",
        "mostrar(\"antes\")\nsea a = 0\nmostrar(10 / a)\nmostrar(\"nunca\")\n",
        "no se puede dividir entre cero",
    );
    error_en_los_dos(
        "resto_cero",
        "sea a = 0\nmostrar(10 % a)\n",
        "entre cero",
    );
    error_en_los_dos(
        "dentro_de_funcion",
        "funcion mal(n) {\n    devolver 1 / n\n}\nsea i = 3\nmientras i >= 0 {\n    mostrar(mal(i))\n    i = i - 1\n}\n",
        "dividir entre cero",
    );
    error_en_los_dos(
        "indice_fuera",
        "sea l = [1, 2, 3]\nmostrar(l[1])\nmostrar(l[3])\n",
        "Error en la línea 3",
    );
    error_en_los_dos(
        "indice_negativo",
        "sea l = [1, 2, 3]\nmostrar(l[0 - 1])\n",
        "Error en la línea 2",
    );
    error_en_los_dos(
        "indice_decimal",
        "sea l = [1, 2, 3]\nmostrar(l[1.5])\n",
        "Error en la línea 2",
    );
    error_en_los_dos(
        "asignar_fuera",
        "sea l = [1, 2]\nl[5] = 1\n",
        "Error en la línea 2",
    );
    error_en_los_dos(
        "suma_rara",
        "mostrar(1)\nmostrar(nada + 1)\n",
        "no se puede sumar",
    );
    error_en_los_dos(
        "resta_texto",
        "mostrar(\"a\" - 1)\n",
        "no se puede restar",
    );
    error_en_los_dos(
        "condicion_no_logica",
        "si 1 {\n    mostrar(1)\n}\n",
        "la condición debe ser verdadero o falso",
    );
    error_en_los_dos(
        "y_no_logico",
        "mostrar(verdadero y 5)\n",
        "la condición debe ser verdadero o falso",
    );
    error_en_los_dos(
        "para_no_lista",
        "para x en 5 {\n    mostrar(x)\n}\n",
        "para necesita una lista, no número",
    );
    error_en_los_dos(
        "numero_invalido",
        "mostrar(a_numero(\"hola\"))\n",
        "Error en la línea 1",
    );
    error_en_los_dos(
        "negar_texto",
        "sea t = \"a\"\nmostrar(-t)\n",
        "no se puede negar",
    );
    error_en_los_dos(
        "comparar_raro",
        "mostrar(\"a\" < 1)\n",
        "no se pueden comparar",
    );
    error_en_los_dos(
        "linea_tras_llamada",
        "funcion f() {\n    devolver 1\n}\nsea a = f() + nada\n",
        "Error en la línea 4",
    );
}

#[test]
fn estructuras_de_control_y_colecciones() {
    let (salida, errores, codigo) = comparar_motores(
        "control",
        r#"funcion fib(n) {
    si n < 2 {
        devolver n
    }
    devolver fib(n - 1) + fib(n - 2)
}
mostrar(fib(20))
sea total = 0
para i en rango(0, 10) {
    si i % 2 == 0 {
        continuar
    }
    si i > 7 {
        romper
    }
    total = total + i
}
mostrar(total)
sea k = 0
mientras verdadero {
    k = k + 1
    sea j = 0
    mientras j < 5 {
        j = j + 1
        si j == 3 {
            continuar
        }
        si k * j > 10 {
            romper
        }
    }
    si k >= 4 {
        romper
    }
}
mostrar(k)
sea lista = []
para n en [3, 1, 2] {
    lista = agregar(lista, n * 2)
}
mostrar(lista)
mostrar(ordenar(lista))
mostrar(invertir(lista))
mostrar(cortar(lista, 1, 2))
mostrar(incluye(lista, 4))
mostrar(medir("hola") + medir(lista))
sea m = {"nombre": "Ana", "edad": 30}
m["edad"] = m["edad"] + 1
m["nuevo"] = [1, 2]
m["nuevo"][1] = 9
mostrar(m)
mostrar(tiene_clave(m, "nombre"))
para clave en m {
    escribir(clave + ";")
}
mostrar("")
sea matriz = [[1, 2], [3, 4]]
matriz[1][0] = 30
mostrar(matriz)
mostrar(matriz[1][0] + matriz[0][1])
sea texto = "a"
texto = texto + "b"
texto = texto + 1
mostrar(texto)
mostrar(elegir(total > 5, "grande", "pequeño"))
mostrar(verdadero y falso o verdadero)
mostrar(no falso)
mostrar(nada == nada)
mostrar("a" == "a")
sea x = 10
si x > 5 {
    sea x = 1
    mostrar(x)
}
mostrar(x)
"#,
    );
    assert_eq!(codigo, Some(0), "{}", errores);
    assert!(salida.starts_with("6765\n"), "{}", salida);
}

#[test]
fn el_interprete_arranca_en_menos_de_dos_decimas() {
    let ruta = archivo_temporal("hola", "mostrar(\"Hola, mundo\")\n");
    let datos = carpeta_datos("hola");
    let mut mejor = Duration::from_secs(60);
    for _ in 0..5 {
        let inicio = Instant::now();
        let salida = ejecutar_con("interprete", &ruta, &datos, false);
        let tiempo = inicio.elapsed();
        assert_eq!(String::from_utf8_lossy(&salida.stdout), "Hola, mundo\n");
        mejor = mejor.min(tiempo);
    }
    let _ = fs::remove_file(&ruta);
    assert!(
        mejor < Duration::from_millis(200),
        "el arranque tardó {:?} (máximo 0,2 s)",
        mejor
    );
}

// Pieza 11: atrapar errores da lo mismo en los dos motores.
const ATRAPAR: &str = r#"funcion fallar(n) {
    si n > 1 {
        provocar_error("falló " + a_texto(n))
    }
    devolver n
}
intentar {
    mostrar(10 / 0)
} si falla e {
    mostrar("1: " + e["mensaje"])
}
intentar {
    sea l = [1, 2]
    mostrar(l[5])
} si falla e {
    mostrar("2: " + e["mensaje"])
}
intentar {
    provocar_error("mío")
} si falla e {
    mostrar("3: " + e["mensaje"] + " línea " + a_texto(e["linea"]))
}
intentar {
    intentar {
        provocar_error("interno")
    } si falla e {
        mostrar("4a: " + e["mensaje"])
        provocar_error("otro")
    }
} si falla e {
    mostrar("4b: " + e["mensaje"])
}
intentar {
    mostrar(fallar(1))
    mostrar(fallar(2))
} si falla e {
    mostrar("5: " + e["mensaje"])
}
para i en rango(0, 5) {
    intentar {
        si i == 1 { continuar }
        si i == 3 { romper }
        mostrar("6: " + a_texto(i))
    } si falla e {
        mostrar("no")
    }
}
funcion buscar() {
    intentar {
        devolver "7: devuelto"
    } si falla e {
        devolver "no"
    }
    devolver "tampoco"
}
mostrar(buscar())
"#;

const ATRAPAR_SALIDA: &str = "1: no se puede dividir entre cero\n2: el índice 5 está fuera de rango para una lista de 2 elementos\n3: mío línea 19\n4a: interno\n4b: otro\n1\n5: falló 2\n6: 0\n6: 2\n7: devuelto\n";

#[test]
fn intentar_da_lo_mismo_con_los_dos_motores() {
    let (salida, errores, codigo) = comparar_motores("intentar_atrapa", ATRAPAR);
    assert_eq!(salida, ATRAPAR_SALIDA);
    assert!(errores.is_empty(), "{}", errores);
    assert_eq!(codigo, Some(0));
}

#[test]
fn un_error_fuera_de_intentar_da_lo_mismo_con_los_dos_motores() {
    let programa = format!("{}mostrar(1 / 0)\nmostrar(\"no llega\")\n", ATRAPAR);
    let (salida, errores, codigo) = comparar_motores("intentar_fuera", &programa);
    assert_eq!(salida, ATRAPAR_SALIDA);
    assert!(errores.contains("no se puede dividir entre cero"), "{}", errores);
    assert_eq!(codigo, Some(1));
}

/// `hondo` solo hace cuentas (camino rápido de números en el modo Rust); `cuenta` usa listas
/// (camino normal). Los dos deben parar en el mismo sitio y con el mismo mensaje.
const DEMASIADAS_LLAMADAS: &str = r#"funcion hondo(n) {
    si n == 0 { devolver 0 }
    devolver 1 + hondo(n - 1)
}
funcion cuenta(n) {
    si n == 0 { devolver "fin" }
    sea x = [n]
    devolver cuenta(n - 1)
}
mostrar(hondo(9999))
mostrar(cuenta(9999))
intentar {
    mostrar(hondo(20000))
} si falla e {
    mostrar("atrapado en " + a_texto(e["linea"]))
}
intentar {
    mostrar(cuenta(10000))
} si falla e {
    mostrar("atrapado en " + a_texto(e["linea"]))
}
mostrar(hondo(9999))
mostrar(cuenta(10000))
"#;

#[test]
fn las_llamadas_sin_fin_paran_igual_en_los_dos_motores() {
    let (salida, errores, codigo) = comparar_motores("demasiadas_llamadas", DEMASIADAS_LLAMADAS);
    assert_eq!(salida, "9999\nfin\natrapado en 3\natrapado en 8\n9999\n");
    assert!(errores.contains("Error en la línea 8"), "{}", errores);
    assert!(errores.contains("hay más de 10000 llamadas a funciones una dentro de otra"), "{}", errores);
    assert_eq!(codigo, Some(1));
}

/// Rutas web atendidas por el intérprete con el mismo servidor del núcleo que el modo Rust.
const RUTAS: &str = r#"ruta "/" {
    devolver "<h1>Hola</h1>"
}
ruta "/saludo/:nombre" {
    guardar_en_sesion("visto", nombre)
    devolver "Hola, " + nombre
}
ruta "/sesion" {
    devolver "Último: " + a_texto(leer_de_sesion("visto"))
}
ruta "/cuenta" {
    intentar {
        devolver a_texto(10 / 0)
    } si falla e {
        devolver "atrapado: " + e["mensaje"]
    }
}
mostrar(visitar("/"))
mostrar(visitar("/saludo/<b>Ana"))
mostrar(visitar("/sesion"))
mostrar(visitar("/cuenta"))
mostrar(visitar("/no-existe"))
"#;

#[test]
fn las_rutas_web_se_interpretan_y_dan_lo_mismo() {
    let ruta = archivo_temporal("rutas_sin_caer", RUTAS);
    let datos = carpeta_datos("rutas_sin_caer");
    let salida = ejecutar_con("interprete", &ruta, &datos, true);
    let _ = fs::remove_file(&ruta);
    assert_eq!(
        String::from_utf8_lossy(&salida.stderr),
        "",
        "las rutas deberían interpretarse sin caer al modo Rust"
    );
    let (salida, errores, codigo) = comparar_motores("rutas_web", RUTAS);
    assert!(salida.starts_with("<h1>Hola</h1>\nHola, &lt;b&gt;Ana\nÚltimo: "), "{}", salida);
    assert!(salida.contains("atrapado: no se puede dividir entre cero"), "{}", salida);
    assert!(errores.is_empty(), "{}", errores);
    assert_eq!(codigo, Some(0));
}

/// `deimon probar` con los dos motores: mismas pruebas correctas y fallidas, mismos detalles.
const PRUEBAS: &str = r#"usar modulo "texto"
funcion doble(n) {
    devolver n * 2
}
ruta "/" {
    devolver "hola " + a_texto(contar("visitas"))
}
prueba "doble funciona" {
    comprobar_igual(doble(4), 8)
}
prueba "falla a propósito" {
    comprobar_igual(doble(2), 5)
}
prueba "condición falsa" {
    comprobar(doble(1) == 3)
}
prueba "error dentro" {
    sea l = [1]
    mostrar(l[5])
}
prueba "datos vacíos en cada prueba" {
    guardar("visitas", "a", 1)
    comprobar_igual(contar("visitas"), 1)
}
prueba "otra vez vacíos" {
    comprobar_igual(contar("visitas"), 0)
    comprobar(contiene(visitar("/"), "hola 0"))
}
prueba "intentar en prueba" {
    intentar { provocar_error("x") } si falla e { comprobar_igual(e["mensaje"], "x") }
}
mostrar("esto no se ejecuta al probar")
"#;

fn probar_con(motor: &str, ruta: &PathBuf, json: bool) -> Output {
    let mut orden = Command::new(binario());
    orden.arg("probar").arg(ruta).env("DEIMON_MOTOR", motor);
    if json {
        orden.arg("--json");
    }
    orden.output().expect("no se pudo ejecutar deimon probar")
}

#[test]
fn probar_da_lo_mismo_con_los_dos_motores() {
    let ruta = archivo_temporal("pruebas_dos_motores", PRUEBAS);
    for json in [false, true] {
        let interprete = probar_con("interprete", &ruta, json);
        let rust = probar_con("rust", &ruta, json);
        assert_eq!(String::from_utf8_lossy(&interprete.stdout), String::from_utf8_lossy(&rust.stdout));
        assert_eq!(String::from_utf8_lossy(&interprete.stderr), String::from_utf8_lossy(&rust.stderr));
        assert_eq!(interprete.status.code(), rust.status.code());
    }
    let salida = String::from_utf8_lossy(&probar_con("interprete", &ruta, false).stdout).to_string();
    let _ = fs::remove_file(&ruta);
    assert!(salida.contains("Pruebas: 7 · correctas: 4 · fallidas: 3"), "{}", salida);
    assert!(salida.contains("línea 12: se esperaba 5 y se obtuvo 4"), "{}", salida);
}

#[test]
fn los_limites_de_pasos_y_de_salida_cortan_y_no_se_pueden_atrapar() {
    let bucle = archivo_temporal("sin_fin", "sea i = 0\nintentar {\n    mientras verdadero {\n        i = i + 1\n    }\n} si falla e {\n    mostrar(\"atrapado\")\n}\n");
    let salida = Command::new(binario())
        .arg("ejecutar")
        .arg(&bucle)
        .env("DEIMON_MOTOR", "interprete")
        .env("DEIMON_MAX_PASOS", "100_000")
        .output()
        .unwrap();
    let _ = fs::remove_file(&bucle);
    assert_eq!(salida.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&salida.stdout), "");
    assert!(String::from_utf8_lossy(&salida.stderr).contains("pasó de 100000 pasos"), "{}", String::from_utf8_lossy(&salida.stderr));

    let mucho = archivo_temporal("mucha_salida", "mientras verdadero {\n    escribir(\"ab\")\n}\n");
    let salida = Command::new(binario())
        .arg("ejecutar")
        .arg(&mucho)
        .env("DEIMON_MOTOR", "interprete")
        .env("DEIMON_MAX_SALIDA", "1000")
        .output()
        .unwrap();
    let _ = fs::remove_file(&mucho);
    assert_eq!(salida.status.code(), Some(1));
    assert_eq!(salida.stdout.len(), 1000);
    assert!(String::from_utf8_lossy(&salida.stderr).contains("DEIMON_MAX_SALIDA = 1000 bytes"));
}
