use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta_nueva(nombre: &str) -> PathBuf {
    let carpeta = std::env::temp_dir().join(format!("ln_pruebas_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn escribir(carpeta: &PathBuf, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
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

#[test]
fn probar_informa_una_prueba_correcta_y_otra_fallida() {
    let c = carpeta_nueva("mixto");
    let f = escribir(
        &c,
        "mixto.dmn",
        "funcion suma(a, b) {\n  devolver a + b\n}\n\nprueba \"bien\" {\n  comprobar_igual(suma(2, 2), 4)\n}\n\nprueba \"mal\" {\n  comprobar(suma(1, 1) == 2)\n  comprobar_igual(suma(2, 3), 4)\n}\n\nmostrar(\"esto no debe ejecutarse al probar\")\n",
    );
    let o = deimon(&c, &["probar", f.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}{}", texto, errores(&o));
    assert!(texto.contains("✓ bien"), "{}", texto);
    assert!(
        texto.contains("✗ mal — línea 11: se esperaba 4 y se obtuvo 5"),
        "{}",
        texto
    );
    assert!(texto.contains("Pruebas: 2 · correctas: 1 · fallidas: 1"), "{}", texto);
    assert!(!texto.contains("no debe ejecutarse"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn probar_con_todo_correcto_sale_con_cero() {
    let c = carpeta_nueva("todo_bien");
    let f = escribir(&c, "bien.dmn", "prueba \"uno\" {\n  comprobar(verdadero)\n}\n");
    let o = deimon(&c, &["probar", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert!(salida(&o).contains("Pruebas: 1 · correctas: 1 · fallidas: 0"));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn probar_sin_pruebas_avisa_y_sale_con_uno() {
    let c = carpeta_nueva("sin_pruebas");
    let f = escribir(&c, "sin.dmn", "mostrar(1)\n");
    let o = deimon(&c, &["probar", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(salida(&o).contains("no tiene pruebas"), "{}", salida(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_error_en_ejecucion_no_aborta_las_demas_pruebas() {
    let c = carpeta_nueva("error_ejecucion");
    let f = escribir(
        &c,
        "error.dmn",
        "prueba \"revienta\" {\n  sea x = 1 + verdadero\n  comprobar(verdadero)\n}\n\nprueba \"sigue\" {\n  comprobar_igual(1 + 1, 2)\n}\n",
    );
    let o = deimon(&c, &["probar", f.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}{}", texto, errores(&o));
    assert!(texto.contains("✗ revienta — error en ejecución: no se puede sumar"), "{}", texto);
    assert!(texto.contains("✓ sigue"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn visitar_decodifica_y_escapa_los_parametros() {
    let c = carpeta_nueva("visitar");
    let f = escribir(
        &c,
        "visitar.dmn",
        "ruta \"/saludo/:nombre\" {\n  devolver \"<h1>Hola, \" + nombre + \"</h1>\"\n}\n\nprueba \"saludo\" {\n  comprobar_igual(visitar(\"/saludo/Ana\"), \"<h1>Hola, Ana</h1>\")\n}\n\nprueba \"escape\" {\n  comprobar_igual(visitar(\"/saludo/%3Cb%3E\"), \"<h1>Hola, &lt;b&gt;</h1>\")\n}\n\nprueba \"no hay ruta\" {\n  comprobar_igual(visitar(\"/nada\"), \"<h1>Página no encontrada</h1>\")\n}\n\nservir(8081)\n",
    );
    let o = deimon(&c, &["probar", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert!(salida(&o).contains("Pruebas: 3 · correctas: 3 · fallidas: 0"), "{}", salida(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn probar_no_toca_los_datos_reales() {
    let c = carpeta_nueva("datos_reales");
    let datos = c.join("mis_datos");
    fs::create_dir_all(&datos).unwrap();
    fs::write(datos.join("cosas.datos"), "a\tt\tuno\n").unwrap();
    let f = escribir(
        &c,
        "datos.dmn",
        "prueba \"guarda\" {\n  guardar(\"cosas\", \"b\", \"dos\")\n  guardar(\"otra\", \"x\", 1)\n  comprobar_igual(contar(\"cosas\"), 1)\n}\n",
    );
    let o = Command::new(binario())
        .args(["probar", f.to_str().unwrap()])
        .current_dir(&c)
        .env("DEIMON_DATOS", &datos)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert_eq!(fs::read_to_string(datos.join("cosas.datos")).unwrap(), "a\tt\tuno\n");
    assert_eq!(fs::read_dir(&datos).unwrap().count(), 1, "se crearon archivos en los datos reales");
    assert!(!c.join("datos").exists(), "se creó ./datos");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn ejecutar_ignora_las_pruebas_y_comprobar_fuera_de_prueba_falla() {
    let c = carpeta_nueva("ejecutar");
    let f = escribir(&c, "a.dmn", "prueba \"falla\" {\n  comprobar(falso)\n}\nmostrar(\"hola\")\n");
    let o = deimon(&c, &["ejecutar", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o).trim(), "hola");
    let g = escribir(&c, "b.dmn", "comprobar(falso)\n");
    let o = deimon(&c, &["ejecutar", g.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(errores(&o).contains("línea 1: la condición es falsa"), "{}", errores(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn app_se_niega_si_una_prueba_falla() {
    let c = carpeta_nueva("app_falla");
    let f = escribir(
        &c,
        "app.dmn",
        "ruta \"/\" {\n  devolver \"hola\"\n}\n\nprueba \"rota\" {\n  comprobar_igual(visitar(\"/\"), \"adiós\")\n}\n\nservir(8081)\n",
    );
    let apk = c.join("app.apk");
    let o = deimon(&c, &["app", f.to_str().unwrap(), "--salida", apk.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1), "{}{}", salida(&o), errores(&o));
    assert!(errores(&o).contains("No se crea el APK"), "{}", errores(&o));
    assert!(salida(&o).contains("✗ rota"), "{}", salida(&o));
    assert!(!apk.exists());
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn entorno_sugiere_otro_puerto_si_esta_ocupado() {
    let c = carpeta_nueva("puerto");
    let ocupado = TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = ocupado.local_addr().unwrap().port();
    let f = escribir(
        &c,
        "web.dmn",
        &format!("ruta \"/\" {{\n  devolver \"hola\"\n}}\n\nservir({})\n", puerto),
    );
    let o = deimon(&c, &["entorno", f.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}{}", texto, errores(&o));
    assert!(texto.contains(&format!("la puerta {} está ocupada", puerto)), "{}", texto);
    assert!(texto.contains("DEIMON_PUERTO="), "{}", texto);
    assert!(texto.contains("deimon ejecutar"), "{}", texto);
    drop(ocupado);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn entorno_con_puerto_libre_sale_con_cero() {
    let c = carpeta_nueva("puerto_libre");
    let libre = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let f = escribir(
        &c,
        "web.dmn",
        &format!("ruta \"/\" {{\n  devolver \"hola\"\n}}\n\nservir({})\n", libre),
    );
    let o = deimon(&c, &["entorno", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}{}", salida(&o), errores(&o));
    assert!(salida(&o).contains("✓ rustc"), "{}", salida(&o));
    assert!(salida(&o).contains("está libre"), "{}", salida(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn entorno_informa_un_error_de_traduccion() {
    let c = carpeta_nueva("traduccion");
    let f = escribir(&c, "mal.dmn", "mostrar(\n");
    let o = deimon(&c, &["entorno", f.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(salida(&o).contains("✗ traducción"), "{}", salida(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn la_salida_json_es_basica_pero_valida() {
    let c = carpeta_nueva("json");
    let f = escribir(&c, "j.dmn", "prueba \"una \\\"con comillas\\\"\" {\n  comprobar(verdadero)\n}\n");
    let o = deimon(&c, &["entorno", f.to_str().unwrap(), "--json"]);
    let texto = salida(&o);
    assert!(texto.starts_with('{'), "{}", texto);
    assert!(texto.contains("\"listo\""), "{}", texto);
    assert!(texto.contains("\"estado\":\"ok\""), "{}", texto);
    let o = deimon(&c, &["probar", f.to_str().unwrap(), "--json"]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(0), "{}{}", texto, errores(&o));
    assert!(texto.starts_with('{'), "{}", texto);
    assert!(texto.contains("\"listo\":true"), "{}", texto);
    assert!(texto.contains("una \\\"con comillas\\\""), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}
