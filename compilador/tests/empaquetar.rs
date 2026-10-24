//! `deimon empaquetar`: un solo ejecutable con `deimon` y el programa, que se abre sin Rust y
//! sin los archivos originales.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = std::env::temp_dir().join(format!("deimon_empaquetar_{}_{}", nombre, std::process::id()));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

fn escribir(ruta: &Path, texto: &str) {
    fs::create_dir_all(ruta.parent().unwrap()).unwrap();
    fs::write(ruta, texto).unwrap();
}

fn empaquetar(dentro: &Path, archivo: &str, salida: &Path, casa: &Path) -> Output {
    Command::new(binario())
        .arg("empaquetar")
        .arg(archivo)
        .arg("--salida")
        .arg(salida)
        .current_dir(dentro)
        .env("DEIMON_CASA", casa)
        .output()
        .unwrap()
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[test]
fn el_paquete_lleva_todo_y_no_necesita_los_originales() {
    let base = carpeta("todo");
    let proyecto = base.join("proyecto");
    let casa = base.join("casa");
    escribir(
        &proyecto.join("app.dmn"),
        "usar \"piezas/cuentas.dmn\"\nusar modulo \"saludar\"\nusar modulo \"texto\"\nmostrar(mayusculas(saludo(\"ana\")))\nmostrar(doble(21))\nmostrar(plantilla(\"vistas/hola.html\", {\"nombre\": \"Luis\"}))\nprueba \"doble\" { comprobar_igual(doble(2), 4) }\n",
    );
    escribir(&proyecto.join("piezas/cuentas.dmn"), "funcion doble(n) {\n    devolver n * 2\n}\n");
    escribir(&proyecto.join("vistas/hola.html"), "<p>Hola {{ nombre }}</p>\n");
    escribir(&casa.join(".deimon/modulos/saludar/modulo.txt"), "nombre: saludar\ndescripcion: saluda\nrequiere:\n");
    escribir(
        &casa.join(".deimon/modulos/saludar/saludo.dmn"),
        "funcion saludo(n) {\n    devolver \"hola \" + n\n}\n",
    );
    let paquete = base.join("app");
    let hecho = empaquetar(&proyecto, "app.dmn", &paquete, &casa);
    assert!(hecho.status.success(), "{}{}", texto(&hecho.stdout), texto(&hecho.stderr));
    assert!(texto(&hecho.stdout).contains("✓ Empaquetado"), "{}", texto(&hecho.stdout));

    // Se cambian y se borran los originales: el paquete sigue igual.
    escribir(&proyecto.join("piezas/cuentas.dmn"), "funcion doble(n) { devolver 0 }\n");
    fs::remove_dir_all(&casa).unwrap();
    fs::remove_dir_all(proyecto.join("vistas")).unwrap();
    let otra = base.join("otra");
    fs::create_dir_all(&otra).unwrap();
    let salida = Command::new(&paquete).current_dir(&otra).env("HOME", &otra).output().unwrap();
    assert_eq!(texto(&salida.stderr), "");
    assert_eq!(texto(&salida.stdout), "HOLA ANA\n42\n<p>Hola Luis</p>\n\n");
    assert!(salida.status.success());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn un_error_en_el_paquete_sale_igual_que_con_ejecutar() {
    let base = carpeta("error");
    escribir(&base.join("e.dmn"), "mostrar(\"antes\")\nsea l = [1]\nmostrar(l[3])\n");
    let paquete = base.join("e");
    let hecho = empaquetar(&base, "e.dmn", &paquete, &base);
    assert!(hecho.status.success(), "{}", texto(&hecho.stderr));
    let empaquetado = Command::new(&paquete).current_dir(&base).output().unwrap();
    let ejecutado = Command::new(binario()).arg("ejecutar").arg("e.dmn").current_dir(&base).output().unwrap();
    assert_eq!(texto(&empaquetado.stdout), texto(&ejecutado.stdout));
    assert_eq!(texto(&empaquetado.stderr), texto(&ejecutado.stderr));
    assert_eq!(empaquetado.status.code(), Some(1));
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn no_empaqueta_si_una_prueba_falla_o_si_necesita_rust() {
    let base = carpeta("rechazos");
    escribir(&base.join("f.dmn"), "mostrar(1)\nprueba \"mal\" { comprobar_igual(1, 2) }\n");
    let hecho = empaquetar(&base, "f.dmn", &base.join("f"), &base);
    assert_eq!(hecho.status.code(), Some(1));
    assert!(texto(&hecho.stderr).contains("No se empaqueta el programa porque 1 prueba está fallando"), "{}", texto(&hecho.stderr));
    assert!(!base.join("f").exists());

    escribir(&base.join("s.dmn"), "usar modulo \"pantallas\"\nmostrar(1)\n");
    let hecho = empaquetar(&base, "s.dmn", &base.join("s"), &base);
    assert_eq!(hecho.status.code(), Some(1));
    assert!(texto(&hecho.stderr).contains("no se puede empaquetar sin Rust"), "{}", texto(&hecho.stderr));
    assert!(texto(&hecho.stderr).contains("deimon compilar"), "{}", texto(&hecho.stderr));
    assert!(!base.join("s").exists());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn un_paquete_danado_no_rompe_nada() {
    let base = carpeta("danado");
    escribir(&base.join("h.dmn"), "mostrar(\"hola\")\n");
    let paquete = base.join("h");
    assert!(empaquetar(&base, "h.dmn", &paquete, &base).status.success());
    let mut bytes = fs::read(&paquete).unwrap();
    bytes.truncate(bytes.len() - 100);
    let roto = base.join("roto");
    fs::write(&roto, &bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&roto, fs::Permissions::from_mode(0o755)).unwrap();
    }
    // Sin su cola, es un `deimon` normal: enseña la ayuda y no ejecuta nada a medias.
    let salida = Command::new(&roto).arg("--version").output().unwrap();
    assert!(salida.status.success());
    assert!(!texto(&salida.stdout).contains("hola\n"));
    let _ = fs::remove_dir_all(&base);
}
