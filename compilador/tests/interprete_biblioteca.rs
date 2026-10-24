// Pruebas de la biblioteca estándar en el intérprete propio: cada módulo da la misma salida,
// los mismos errores y el mismo código de salida que el modo Rust, y no cae al modo Rust.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta_ejemplos() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("ejemplos")
}

fn carpeta_datos(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_ibib_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta de datos");
    carpeta
}

fn archivo_temporal(nombre: &str, contenido: &str) -> PathBuf {
    let carpeta = std::env::temp_dir();
    let ruta = carpeta.join(format!("ln_ibib_{}_{}.dmn", std::process::id(), nombre));
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

fn ejecutar_con(motor: &str, ruta: &PathBuf, datos: &PathBuf) -> Output {
    Command::new(binario())
        .arg("ejecutar")
        .arg(ruta)
        .env("DEIMON_MOTOR", motor)
        .env("DEIMON_MOTOR_MOTIVO", "1")
        .env("DEIMON_DATOS", datos)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Ejecuta con los dos motores: salida, errores y código iguales, y el intérprete no cae al
/// modo Rust. Devuelve (salida, errores, código).
fn comparar(nombre: &str, programa: &str) -> (String, String, Option<i32>) {
    let ruta = archivo_temporal(nombre, programa);
    let datos = carpeta_datos(nombre);
    let interprete = ejecutar_con("interprete", &ruta, &datos);
    let rust = ejecutar_con("rust", &ruta, &datos);
    let _ = fs::remove_file(&ruta);
    assert!(
        !texto(&interprete.stderr).contains("(modo Rust:"),
        "'{}' cayó al modo Rust: {}",
        nombre,
        texto(&interprete.stderr)
    );
    assert_eq!(texto(&interprete.stdout), texto(&rust.stdout), "'{}': salida distinta", nombre);
    assert_eq!(texto(&interprete.stderr), texto(&rust.stderr), "'{}': errores distintos", nombre);
    assert_eq!(interprete.status.code(), rust.status.code(), "'{}': código distinto", nombre);
    (
        texto(&interprete.stdout),
        texto(&interprete.stderr),
        interprete.status.code(),
    )
}

fn error_igual(nombre: &str, programa: &str, esperado: &str) {
    let (_, errores, codigo) = comparar(nombre, programa);
    assert_eq!(codigo, Some(1), "'{}'", nombre);
    assert!(errores.contains(esperado), "'{}': {}", nombre, errores);
}

fn ruta_de_datos(nombre: &str) -> String {
    let carpeta = std::env::temp_dir().join(format!("ln_ibib_f_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta.to_string_lossy().replace('\\', "/")
}

#[test]
fn matematicas_texto_y_tiempo() {
    let (salida, _, codigo) = comparar(
        "mat_txt_tiempo",
        r#"usar modulo "matematicas"
usar modulo "texto"
usar modulo "tiempo"
mostrar(raiz(81) + potencia(2, 8) + absoluto(-3))
mostrar(redondear(2.5) + suelo(2.7) + techo(2.1) + minimo(4, 9) + maximo(4, 9))
mostrar(factorial(6))
mostrar(es_par(7))
sea a = aleatorio()
mostrar(a >= 0 y a < 1)
sea d = entero_al_azar(1, 6)
mostrar(d >= 1 y d <= 6)
mostrar(mayusculas("hola") + minusculas("MUNDO") + "[" + recortar("  x  ") + "]")
mostrar(contiene("hola mundo", "mundo"))
mostrar(reemplazar("a-b-c", "-", "+"))
mostrar(partir("a,b,c", ","))
mostrar(unir(["a", "b"], "-"))
mostrar(empieza_con("hola", "ho") y termina_con("hola", "la"))
mostrar(repetir("ja", 3))
mostrar(formatear_texto("%s tiene %d años", ["Ana", 30]))
mostrar(formatear_numero(1234.5678, 2, ".", ","))
mostrar(rellenar_texto("hola", 8, "-", "izquierda"))
mostrar(primera_mayuscula("ñandú"))
mostrar(texto_completo({"a": 2}))
mostrar(fecha(0))
mostrar(crear_fecha(2024, 2, 29, 12, 0, 0))
mostrar(formatear_fecha(0, "Y-m-d H:i:s", "UTC"))
mostrar(sumar_tiempo(0, 1, "dias"))
mostrar(diferencia_fechas(86400, 0, "horas"))
mostrar(dia_de_la_semana(0, "UTC"))
mostrar(es_bisiesto(2024))
mostrar(dias_del_mes(2024, 2))
mostrar(leer_fecha("2026-03-29T01:30:00+01:00"))
esperar(1)
mostrar(ahora() > 1700000000)
"#,
    );
    assert_eq!(codigo, Some(0));
    assert!(salida.contains("720"));
    assert!(salida.contains("1970-01-01 00:00:00"));
}

#[test]
fn json_y_patrones() {
    let (salida, _, codigo) = comparar(
        "json_patrones",
        r#"usar modulo "json"
usar modulo "patrones"
sea datos = desde_json("{\"usuario\": {\"nombre\": \"Ana\", \"edad\": 30}, \"cosas\": [1, 2]}")
mostrar(datos["usuario"]["nombre"])
mostrar(leer_json("{\"a\": {\"b\": 5}}", "a.b"))
mostrar(listar_claves_json("{\"x\": 1, \"y\": 2}", ""))
mostrar(a_json([1, "dos", {"k": verdadero}]))
mostrar(coincide_patron("pedido 42", "\\d+"))
mostrar(buscar_patron("ana@x.es", "(\\w+)@(\\w+)"))
mostrar(buscar_todos_patron("a1 b22", "[a-z](\\d+)"))
mostrar(reemplazar_patron("2024-05", "(\\d+)-(\\d+)", "$2/$1"))
mostrar(partir_patron("a, b;c", "[,;]\\s*"))
mostrar(buscar_columnas_patron("a1 b2", "([a-z])(\\d)"))
"#,
    );
    assert_eq!(codigo, Some(0));
    assert!(salida.contains("Ana"));
    assert!(salida.contains("05/2024"));
}

#[test]
fn archivos_excel_graficos_y_pdf() {
    let carpeta = ruta_de_datos("archivos");
    let programa = format!(
        r#"usar modulo "archivos"
usar modulo "excel"
usar modulo "graficos"
usar modulo "pdf"
sea c = "{c}"
escribir_archivo(c + "/nota.txt", "Hola desde Deimon")
mostrar(leer_archivo(c + "/nota.txt"))
mostrar(existe_archivo(c + "/nota.txt"))
crear_csv(c + "/v.csv", [["Mes", "Total"], ["enero", 120]])
mostrar(leer_csv(c + "/v.csv"))
crear_excel(c + "/v.xlsx", [["Mes", "Total"], ["enero", 120]])
mostrar(existe_archivo(c + "/v.xlsx"))
mostrar(medir(grafico_barras("Ventas", ["ene", "feb"], [120, 90])) > 0)
mostrar(medir(grafico_lineas("Visitas", ["lun", "mar"], [5, 9])) > 0)
mostrar(medir(grafico_tarta("Gastos", ["casa", "comida"], [600, 300])) > 0)
crear_pdf(c + "/f.pdf", "Factura", ["Cliente: Ana", "Total: 30"])
mostrar(existe_archivo(c + "/f.pdf"))
mostrar(listar_archivos(c))
borrar_archivo(c + "/nota.txt")
mostrar(existe_archivo(c + "/nota.txt"))
"#,
        c = carpeta
    );
    let (salida, _, codigo) = comparar("archivos_excel", &programa);
    assert_eq!(codigo, Some(0));
    assert!(salida.contains("Hola desde Deimon"));
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn los_errores_de_la_biblioteca_son_iguales() {
    error_igual(
        "err_raiz",
        "usar modulo \"matematicas\"\nmostrar(1)\nmostrar(raiz(-4))\n",
        "raiz() no puede calcular",
    );
    error_igual(
        "err_potencia",
        "usar modulo \"matematicas\"\nmostrar(potencia(\"a\", 2))\n",
        "potencia() espera un número",
    );
    error_igual(
        "err_json",
        "usar modulo \"json\"\nmostrar(desde_json(\"{no es json\"))\n",
        "",
    );
    error_igual(
        "err_unir",
        "usar modulo \"texto\"\nmostrar(unir(\"no es lista\", \"-\"))\n",
        "",
    );
    error_igual(
        "err_patron",
        "usar modulo \"patrones\"\nmostrar(coincide_patron(\"a\", \"(\"))\n",
        "",
    );
}

#[test]
fn los_modulos_de_red_no_caen_al_modo_rust() {
    // Sin red no se pueden llamar: basta con declararlos (comparten el código con el modo Rust).
    for modulo in [
        "web", "correo", "google", "mercadopago", "mysql", "pagos", "paypal", "whatsapp",
    ] {
        let programa = format!("usar modulo \"{}\"\nmostrar(\"listo\")\n", modulo);
        let (salida, errores, codigo) = comparar(&format!("red_{}", modulo), &programa);
        assert_eq!(salida, "listo\n", "{}", modulo);
        assert_eq!(errores, "", "{}", modulo);
        assert_eq!(codigo, Some(0), "{}", modulo);
    }
}

#[test]
fn pantallas_y_modulos_propios_caen_al_modo_rust_con_motivo() {
    let datos = carpeta_datos("caidas");
    for (modulo, motivo) in [("pantallas", "es de Android")] {
        let ruta = archivo_temporal(&format!("caida_{}", modulo), &format!("usar modulo \"{}\"\nmostrar(1)\n", modulo));
        let salida = ejecutar_con("interprete", &ruta, &datos);
        let _ = fs::remove_file(&ruta);
        assert!(
            texto(&salida.stderr).contains(&format!("(modo Rust: usa el módulo '{}' ({}", modulo, motivo)),
            "{}: {}",
            modulo,
            texto(&salida.stderr)
        );
    }
    let ruta = carpeta_ejemplos().join("modulo-propio.dmn");
    let salida = ejecutar_con("interprete", &ruta, &datos);
    assert!(texto(&salida.stderr).contains("(modo Rust: usa el módulo propio 'saludos'"));
    let esperado = fs::read_to_string(carpeta_ejemplos().join("modulo-propio.esperado")).unwrap();
    assert_eq!(texto(&salida.stdout), esperado);
}

#[test]
fn el_ejemplo_biblioteca_se_interpreta_y_da_lo_esperado() {
    let datos = carpeta_datos("ej_biblioteca");
    let ruta = carpeta_ejemplos().join("biblioteca.dmn");
    let salida = ejecutar_con("interprete", &ruta, &datos);
    assert!(salida.status.success(), "{}", texto(&salida.stderr));
    assert_eq!(texto(&salida.stderr), "", "no debería caer al modo Rust");
    let esperado = fs::read_to_string(carpeta_ejemplos().join("biblioteca.esperado")).unwrap();
    assert_eq!(texto(&salida.stdout), esperado);
}

/// `sql`: con SQLite dentro de `deimon` (lo normal al prepararlo en Linux con libsqlite3-dev)
/// se interpreta; si no, pasa al modo Rust con su motivo. En los dos casos, mismo resultado y
/// los valores van siempre aparte (nadie puede colar órdenes SQL).
#[test]
fn sql_da_lo_mismo_con_los_dos_motores() {
    let base = std::env::temp_dir().join(format!("deimon_sql_{}.db", std::process::id()));
    let _ = fs::remove_file(&base);
    let programa = r#"usar modulo "sql"
ejecutar_sql("prueba.db", "CREATE TABLE IF NOT EXISTS gente (nombre TEXT, edad INTEGER)", [])
ejecutar_sql("prueba.db", "DELETE FROM gente", [])
ejecutar_sql("prueba.db", "INSERT INTO gente VALUES (?, ?)", ["Ana", 30])
ejecutar_sql("prueba.db", "INSERT INTO gente VALUES (?, ?)", ["Luis'; DROP TABLE gente; --", 41])
mostrar(consultar_sql("prueba.db", "SELECT nombre, edad FROM gente ORDER BY edad", []))
mostrar(consultar_sql("prueba.db", "SELECT count(*) AS n FROM gente WHERE edad > ?", [35]))
intentar {
    consultar_sql("prueba.db", "SELECT * FROM no_existe", [])
} si falla e {
    mostrar("atrapado")
}
"#
    .replace("prueba.db", &base.display().to_string());
    let (salida, _, codigo) = comparar("sql_dos_motores", &programa);
    let _ = fs::remove_file(&base);
    assert_eq!(salida, "[[Ana, 30], [Luis'; DROP TABLE gente; --, 41]]\n[[1]]\natrapado\n");
    assert_eq!(codigo, Some(0));
    let ruta = archivo_temporal("sql_motivo", "usar modulo \"sql\"\nmostrar(1)\n");
    let salida = ejecutar_con("interprete", &ruta, &carpeta_datos("sql_motivo"));
    let _ = fs::remove_file(&ruta);
    let errores = texto(&salida.stderr);
    assert!(
        errores.is_empty() || errores.contains("(modo Rust: usa el módulo 'sql' (este deimon se preparó sin SQLite dentro))"),
        "{}",
        errores
    );
}
