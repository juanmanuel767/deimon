//! Funciones de la máquina (pantalla y teclado) en la terminal: el intérprete (`deimon
//! ejecutar`) y el programa compilado con Rust tienen que escribir exactamente lo mismo.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("maquina_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn con_entrada(orden: &mut Command, entrada: &str) -> Output {
    let mut hijo = orden
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("no se pudo lanzar el programa");
    hijo.stdin
        .take()
        .expect("entrada")
        .write_all(entrada.as_bytes())
        .expect("no se pudo escribir la entrada");
    hijo.wait_with_output().expect("no se pudo esperar al programa")
}

const PROGRAMA: &str = r#"limpiar_pantalla()
poner_color("amarillo", "azul")
escribir_en(2, 3, "hola")
poner_color("gris", "negro")
sea nombre = pedir_texto("¿Nombre? ")
mostrar("[" + nombre + "]")
mostrar(leer_tecla())
mostrar(leer_tecla())
mostrar(leer_tecla() == nada)
mostrar(pedir_texto("otra: ") == nada)
apagar_maquina()
mostrar("esto no se ve")
"#;

const ESPERADO: &str = "\x1b[2J\x1b[H\x1b[93;44m\x1b[3;4Hhola\x1b[37;40m¿Nombre? [Ana]\nñ\nenter\nverdadero\notra: verdadero\n";

fn comprobar(salida: &Output) {
    assert!(salida.status.success(), "{}", String::from_utf8_lossy(&salida.stderr));
    assert_eq!(String::from_utf8_lossy(&salida.stdout), ESPERADO);
}

fn escribir_programa(carpeta: &Path) -> PathBuf {
    let programa = carpeta.join("maquina.dmn");
    fs::write(&programa, PROGRAMA).expect("no se pudo escribir el programa");
    programa
}

#[test]
fn el_interprete_usa_la_terminal_como_pantalla_y_teclado() {
    let programa = escribir_programa(&carpeta("interprete"));
    comprobar(&con_entrada(Command::new(binario()).arg("ejecutar").arg(&programa), "Ana\nñ\n"));
}

#[test]
fn el_programa_compilado_hace_lo_mismo() {
    let carpeta = carpeta("compilado");
    let programa = escribir_programa(&carpeta);
    let ejecutable = carpeta.join("maquina");
    let compilacion = Command::new(binario())
        .arg("compilar")
        .arg(&programa)
        .arg("--salida")
        .arg(&ejecutable)
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");
    assert!(compilacion.status.success(), "{}", String::from_utf8_lossy(&compilacion.stderr));
    comprobar(&con_entrada(&mut Command::new(&ejecutable), "Ana\nñ\n"));
}

#[test]
fn un_color_que_no_existe_dice_cuales_hay() {
    let carpeta = carpeta("color");
    let programa = carpeta.join("color.dmn");
    fs::write(&programa, "poner_color(\"naranja\", \"negro\")\n").expect("no se pudo escribir el programa");
    let salida = con_entrada(Command::new(binario()).arg("ejecutar").arg(&programa), "");
    assert_eq!(salida.status.code(), Some(1));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(errores.contains("color desconocido 'naranja'; usa negro, azul"), "{}", errores);
}
