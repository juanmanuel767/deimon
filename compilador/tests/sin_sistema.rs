//! Destino `pc-sin-sistema`: el programa arranca solo en un PC (GRUB + QEMU, sin Linux) y
//! tiene que escribir exactamente lo mismo que con `deimon ejecutar`. Si este ordenador no
//! tiene QEMU, GRUB o el destino de Rust, las pruebas que arrancan lo avisan y no fallan.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = raiz()
        .join("target")
        .join("tmp")
        .join(format!("sin_sistema_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

/// El destino está listo y además hay QEMU para arrancarlo.
fn se_puede_arrancar() -> bool {
    let destinos = Command::new(binario())
        .args(["destinos", "--json"])
        .output()
        .expect("no se pudo ejecutar 'deimon destinos'");
    let texto = String::from_utf8_lossy(&destinos.stdout);
    let disponible = texto.contains("\"nombre\":\"pc-sin-sistema\"")
        && texto
            .split("\"nombre\":\"pc-sin-sistema\"")
            .nth(1)
            .map(|resto| resto.contains("\"disponible\":true") && resto.find("\"disponible\":true") < resto.find('}'))
            .unwrap_or(false);
    let qemu = Command::new("qemu-system-x86_64").arg("--version").output().is_ok();
    if !(disponible && qemu) {
        eprintln!("aviso: falta QEMU, GRUB o el destino x86_64-unknown-none; no se arranca el núcleo");
    }
    disponible && qemu
}

fn ejecutar(programa: &Path, entrada: &str) -> Output {
    con_entrada(Command::new(binario()).arg("ejecutar").arg(programa), entrada)
}

fn arrancar(programa: &Path, entrada: &str) -> Output {
    con_entrada(
        Command::new(binario()).arg("arrancar").arg(programa).arg("--sin-ventana"),
        entrada,
    )
}

fn con_entrada(orden: &mut Command, entrada: &str) -> Output {
    let mut hijo = orden
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("no se pudo lanzar deimon");
    hijo.stdin
        .take()
        .expect("entrada")
        .write_all(entrada.as_bytes())
        .expect("no se pudo escribir la entrada");
    hijo.wait_with_output().expect("no se pudo esperar a deimon")
}

/// Quita lo que solo es del puerto serie: los \r y las órdenes de la pantalla (\x1b[...).
fn limpio(bytes: &[u8]) -> String {
    let texto = String::from_utf8_lossy(bytes).replace('\r', "");
    let mut salida = String::new();
    let mut caracteres = texto.chars().peekable();
    while let Some(c) = caracteres.next() {
        if c == '\u{1b}' && caracteres.peek() == Some(&'[') {
            caracteres.next();
            for siguiente in caracteres.by_ref() {
                if siguiente.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        salida.push(c);
    }
    salida
}

const PROGRAMA_COMPLETO: &str = r#"funcion profunda(n) {
    si n == 0 {
        devolver 0
    }
    devolver 1 + profunda(n - 1)
}

funcion buscar(lista, x) {
    para v en lista {
        intentar {
            si v == x {
                devolver "encontrado " + v
            }
            si v == 99 {
                provocar_error("malo")
            }
        } si falla e {
            devolver "error " + e["mensaje"]
        }
    }
    devolver "no"
}

sea total = 0
para i en rango(1, 200000) {
    sea t = "x" + i + "y" + i
    total = total + medir(t)
}
mostrar(total)
mostrar(profunda(9000))
mostrar(buscar([1, 2, 3], 2))
mostrar(buscar([1, 99, 3], 3))
sea i = 0
mientras verdadero {
    intentar {
        i = i + 1
        si i == 5 {
            romper
        }
    } si falla e {
        mostrar("no")
    }
}
mostrar("i=" + i)
intentar {
    mostrar(profunda(20000))
} si falla e {
    mostrar("atrapado: " + cortar(e["mensaje"], 0, 30))
}
sea m = {"a": 1}
m["b"] = [1, 2, {"c": 3}]
mostrar(m)
mostrar(listar_claves(m))
mostrar(ordenar([3.5, -1, 2]))
mostrar(1 / 3)
mostrar(a_numero("2.5e3") + 0.1)
mostrar(invertir("ñandú") + " " + medir("ñandú"))
"#;

#[test]
fn el_nucleo_escribe_lo_mismo_que_linux() {
    if !se_puede_arrancar() {
        return;
    }
    let carpeta = carpeta("igual");
    let programa = carpeta.join("completo.dmn");
    fs::write(&programa, PROGRAMA_COMPLETO).expect("no se pudo escribir el programa");
    let linux = ejecutar(&programa, "");
    assert!(linux.status.success(), "{}", String::from_utf8_lossy(&linux.stderr));
    let nucleo = arrancar(&programa, "");
    assert!(nucleo.status.success(), "{}", String::from_utf8_lossy(&nucleo.stderr));
    assert_eq!(limpio(&nucleo.stdout), limpio(&linux.stdout));
}

#[test]
fn la_consola_de_ejemplo_atiende_lo_que_se_teclea() {
    if !se_puede_arrancar() {
        return;
    }
    let consola = raiz().join("..").join("ejemplos-nucleo").join("consola.dmn");
    let entrada = "calc 2 + 3 * (4 - 1)\ncalc 1 / 0\nnota comprar pan\nnotas\neco ñandú\napagar\nesto ya no se lee\n";
    let salida = arrancar(&consola, entrada);
    assert!(salida.status.success(), "{}", String::from_utf8_lossy(&salida.stderr));
    let texto = limpio(&salida.stdout);
    for esperado in [
        "DeimonOS",
        "2 + 3 * (4 - 1) = 11",
        "error: no se puede dividir entre cero",
        "1. comprar pan",
        "deimon> eco ñandú\nñandú\n",
        "Apagando...",
    ] {
        assert!(texto.contains(esperado), "falta {:?} en:\n{}", esperado, texto);
    }
    assert!(!texto.contains("no conozco"), "{}", texto);
}

#[test]
fn un_error_sin_intentar_detiene_la_maquina_con_su_linea() {
    if !se_puede_arrancar() {
        return;
    }
    let carpeta = carpeta("error");
    let programa = carpeta.join("error.dmn");
    fs::write(&programa, "mostrar(\"antes\")\nsea l = [1]\nmostrar(l[5])\nmostrar(\"después\")\n")
        .expect("no se pudo escribir el programa");
    let salida = arrancar(&programa, "");
    assert_eq!(salida.status.code(), Some(1));
    let texto = limpio(&salida.stdout);
    assert!(texto.contains("antes\n"), "{}", texto);
    assert!(texto.contains("Error en la línea 3 (en ejecución)"), "{}", texto);
    assert!(texto.contains("La máquina se ha detenido."), "{}", texto);
    assert!(!texto.contains("después"), "{}", texto);
}

#[test]
fn lo_que_necesita_disco_red_o_navegador_da_e026() {
    if !se_puede_arrancar() {
        return;
    }
    let carpeta = carpeta("e026");
    let programa = carpeta.join("web.dmn");
    fs::write(
        &programa,
        "usar modulo \"json\"\nguardar(\"a\", \"b\", 1)\nruta \"/\" {\n    devolver \"hola\"\n}\nservir(8080)\n",
    )
    .expect("no se pudo escribir el programa");
    let salida = Command::new(binario())
        .arg("compilar")
        .arg(&programa)
        .args(["--destino", "pc-sin-sistema"])
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");
    assert!(!salida.status.success());
    let errores = String::from_utf8_lossy(&salida.stderr);
    for esperado in [
        "Error en la línea 1: el módulo 'json'",
        "Error en la línea 2: 'guardar' no funciona en el destino pc-sin-sistema",
        "Error en la línea 3: la ruta web \"/\" no funciona en el destino pc-sin-sistema",
        "Error en la línea 6: 'servir' no funciona",
        "Código: E026",
    ] {
        assert!(errores.contains(esperado), "falta {:?} en:\n{}", esperado, errores);
    }
}

#[test]
fn las_integradas_de_respuesta_web_dan_e026() {
    let programa = carpeta("e026_respuesta").join("respuesta.dmn");
    fs::write(
        &programa,
        "responder(\"a\", 200, \"text/plain\")\nresponder_json(1, 200)\nmostrar(metodo_recibido())\nmostrar(camino_recibido())\n",
    )
    .expect("no se pudo escribir el programa");
    let salida = Command::new(binario())
        .arg("compilar")
        .arg(&programa)
        .args(["--destino", "pc-sin-sistema"])
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");
    assert!(!salida.status.success());
    let errores = String::from_utf8_lossy(&salida.stderr);
    for nombre in ["responder", "responder_json", "metodo_recibido", "camino_recibido"] {
        let esperado = format!("'{}' no funciona en el destino pc-sin-sistema", nombre);
        assert!(errores.contains(&esperado), "falta {:?} en:\n{}", esperado, errores);
    }
    assert!(errores.contains("Código: E026"), "{}", errores);
}
