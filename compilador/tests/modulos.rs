use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta =
        std::env::temp_dir().join(format!("ln_modulos_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta de prueba");
    carpeta
}

fn escribir(carpeta: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    if let Some(padre) = ruta.parent() {
        fs::create_dir_all(padre).expect("no se pudo crear la subcarpeta de prueba");
    }
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

fn deimon(carpeta: &Path, argumentos: &[&str]) -> Output {
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
fn usar_carga_funciones_rutas_y_pruebas_del_archivo() {
    let c = carpeta("correcto");
    escribir(
        &c,
        "piezas/calculos.dmn",
        "funcion doble(n) {\n  devolver n * 2\n}\n\nruta \"/doble/:n\" {\n  devolver \"doble=\" + a_numero(n) * 2\n}\n\nprueba \"doble compartido\" {\n  comprobar_igual(doble(3), 6)\n}\n\nmostrar(\"no debe salir\")\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/calculos.dmn\"\nmostrar(doble(4))\nmostrar(visitar(\"/doble/5\"))\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "8\ndoble=10\n");
    assert!(errores(&o).is_empty(), "{}", errores(&o));
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn usar_anidado_resuelve_rutas_relativas_al_archivo_que_usa() {
    let c = carpeta("anidado");
    escribir(
        &c,
        "piezas/base.dmn",
        "funcion doble(n) {\n  devolver n * 2\n}\n",
    );
    escribir(
        &c,
        "piezas/otra.dmn",
        "usar \"base.dmn\"\nfuncion triple(n) {\n  devolver doble(n) + n\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/otra.dmn\"\nmostrar(triple(4))\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "12\n");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn el_mismo_archivo_por_caminos_distintos_se_carga_una_vez() {
    let c = carpeta("duplicado");
    escribir(
        &c,
        "piezas/base.dmn",
        "funcion doble(n) {\n  devolver n * 2\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/base.dmn\"\nusar \"piezas/../piezas/base.dmn\"\nmostrar(doble(6))\n",
    );
    let o = deimon(&c, &["ejecutar", principal.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", errores(&o));
    assert_eq!(salida(&o), "12\n");
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn el_uso_circular_muestra_la_cadena() {
    let c = carpeta("circular");
    escribir(
        &c,
        "a.dmn",
        "usar \"b.dmn\"\nfuncion a() {\n  devolver 1\n}\n",
    );
    escribir(
        &c,
        "b.dmn",
        "usar \"a.dmn\"\nfuncion b() {\n  devolver 2\n}\n",
    );
    let principal = escribir(&c, "principal.dmn", "usar \"a.dmn\"\nmostrar(a())\n");
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E015"), "{}", texto);
    assert!(
        texto.contains("a.dmn → b.dmn → a.dmn"),
        "no se ve la cadena del círculo: {}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn archivo_no_encontrado_dice_la_ruta_buscada() {
    let c = carpeta("no_encontrado");
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/falta.dmn\"\nmostrar(1)\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E014"), "{}", texto);
    assert!(
        texto.contains("no se encontró el archivo 'piezas/falta.dmn'"),
        "{}",
        texto
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_nombre_repetido_entre_archivos_dice_los_dos() {
    let c = carpeta("repetido");
    escribir(
        &c,
        "piezas/base.dmn",
        "funcion doble(n) {\n  devolver n * 2\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/base.dmn\"\nfuncion doble(n) {\n  devolver n + n\n}\nmostrar(doble(2))\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(texto.contains("Código: E016"), "{}", texto);
    assert!(texto.contains("piezas/base.dmn"), "{}", texto);
    assert!(texto.contains("principal.dmn"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn un_error_dentro_del_archivo_usado_muestra_su_nombre() {
    let c = carpeta("error_dentro");
    escribir(
        &c,
        "piezas/rota.dmn",
        "funcion rota() {\n  devolver no_existe(1)\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/rota.dmn\"\nmostrar(rota())\n",
    );
    let o = deimon(&c, &["comprobar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(1), "{}", texto);
    assert!(
        texto.contains("Error en piezas/rota.dmn, línea 2:"),
        "el error no indica el archivo usado: {}",
        texto
    );
    assert!(texto.contains("Código: E006"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn probar_ejecuta_tambien_las_pruebas_del_archivo_usado() {
    let c = carpeta("probar");
    escribir(
        &c,
        "piezas/pruebas.dmn",
        "funcion suma(a, b) {\n  devolver a + b\n}\n\nprueba \"suma compartida\" {\n  comprobar_igual(suma(2, 3), 5)\n}\n",
    );
    let principal = escribir(
        &c,
        "principal.dmn",
        "usar \"piezas/pruebas.dmn\"\nmostrar(\"no se ejecuta al probar\")\n",
    );
    let o = deimon(&c, &["probar", principal.to_str().unwrap()]);
    let texto = salida(&o);
    assert_eq!(o.status.code(), Some(0), "{}{}", texto, errores(&o));
    assert!(texto.contains("✓ suma compartida"), "{}", texto);
    assert!(
        texto.contains("Pruebas: 1 · correctas: 1 · fallidas: 0"),
        "{}",
        texto
    );
    assert!(!texto.contains("no se ejecuta al probar"), "{}", texto);
    let _ = fs::remove_dir_all(&c);
}
