use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use deimon::perfil::Perfil;

const PROGRAMA: &str = r#"
ruta "/" {
    devolver "<h1>ok</h1>"
}

ruta "/falla" {
    devolver a_numero("esto no es un número")
}

servir(8080)
"#;

fn carpeta_temporal() -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp");
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn compilar(perfil: Perfil, nombre: &str) -> (PathBuf, PathBuf) {
    let rust = deimon::traducir_archivo_varios_con_perfil(PROGRAMA, Path::new("recursos.dmn"), perfil)
        .expect("no se pudo traducir el programa de prueba");
    let marca = format!("{}_{}", std::process::id(), nombre);
    let ruta_rust = carpeta_temporal().join(format!("recursos_{}.rs", marca));
    let ruta_bin = carpeta_temporal().join(format!("recursos_{}", marca));
    fs::write(&ruta_rust, rust).expect("no se pudo escribir el Rust generado");

    let mut orden = Command::new("rustc");
    for opcion in perfil.argumentos_rustc() {
        orden.arg(opcion);
    }
    let salida = orden
        .arg(&ruta_rust)
        .arg("-o")
        .arg(&ruta_bin)
        .output()
        .expect("no se pudo ejecutar 'rustc'");
    assert!(
        salida.status.success(),
        "no se pudo compilar con perfil {}: {}",
        perfil.nombre(),
        String::from_utf8_lossy(&salida.stderr)
    );
    (ruta_rust, ruta_bin)
}

fn puerto_libre() -> u16 {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("no se pudo reservar un puerto");
    escucha
        .local_addr()
        .expect("no se pudo leer el puerto")
        .port()
}

struct Servidor {
    proceso: Child,
    archivos: Vec<PathBuf>,
    _lector: Option<BufReader<ChildStdout>>,
    arranque: String,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.proceso.kill();
        let _ = self.proceso.wait();
        for archivo in &self.archivos {
            let _ = fs::remove_file(archivo);
        }
    }
}

fn arrancar(ruta: &Path, puerto: u16, variables: &[(&str, &str)]) -> Servidor {
    let mut orden = Command::new(ruta);
    for variable in [
        "DEIMON_HILOS",
        "DEIMON_MAX_CONEXIONES",
        "DEIMON_MAX_PETICION",
        "DEIMON_TIEMPO_ESPERA",
        "DEIMON_PUERTO",
        "DEIMON_DATOS",
        "DEIMON_ARCHIVO",
        "DEIMON_BORRAR_EJECUTABLE",
    ] {
        orden.env_remove(variable);
    }
    orden.env("DEIMON_PUERTO", puerto.to_string());
    for (variable, valor) in variables {
        orden.env(variable, valor);
    }
    let mut proceso = orden
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");

    let mut lector = BufReader::new(
        proceso
            .stdout
            .take()
            .expect("el servidor no tiene salida estándar"),
    );
    let mut arranque = String::new();
    let _ = lector.read_line(&mut arranque);
    Servidor {
        proceso,
        archivos: Vec::new(),
        _lector: Some(lector),
        arranque: arranque.trim().to_string(),
    }
}

fn enviar(puerto: u16, texto: &str) -> String {
    let mut conexion =
        TcpStream::connect(("127.0.0.1", puerto)).expect("no se pudo conectar con el servidor");
    conexion
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("no se pudo configurar el tiempo de espera");
    conexion
        .write_all(texto.as_bytes())
        .expect("no se pudo enviar la petición");
    let mut respuesta = String::new();
    conexion
        .read_to_string(&mut respuesta)
        .expect("no se pudo leer la respuesta");
    respuesta
}

fn peticion(puerto: u16, camino: &str) -> String {
    enviar(
        puerto,
        &format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            camino
        ),
    )
}

#[test]
fn el_perfil_pequeno_genera_un_ejecutable_mas_pequeno() {
    let (rust_pequeno, bin_pequeno) = compilar(Perfil::Pequeno, "pequeno");
    let (rust_servidor, bin_servidor) = compilar(Perfil::Servidor, "servidor");

    let tamano_pequeno = fs::metadata(&bin_pequeno)
        .expect("no se pudo leer el tamaño del ejecutable pequeño")
        .len();
    let tamano_servidor = fs::metadata(&bin_servidor)
        .expect("no se pudo leer el tamaño del ejecutable de servidor")
        .len();

    assert!(
        tamano_pequeno < tamano_servidor,
        "el perfil pequeño debería ocupar menos ({} bytes) que el de servidor ({} bytes)",
        tamano_pequeno,
        tamano_servidor
    );

    for archivo in [rust_pequeno, bin_pequeno, rust_servidor, bin_servidor] {
        let _ = fs::remove_file(archivo);
    }
}

#[test]
fn un_perfil_invalido_da_error_con_sugerencia() {
    let archivo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ejemplos")
        .join("hola.dmn");
    let salida = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("compilar")
        .arg(&archivo)
        .args(["--perfil", "equilibrado2"])
        .output()
        .expect("no se pudo ejecutar 'deimon compilar'");

    assert_eq!(salida.status.code(), Some(2));
    let errores = String::from_utf8_lossy(&salida.stderr);
    assert!(errores.contains("no existe"), "{}", errores);
    assert!(errores.contains("Sugerencia"), "{}", errores);
    assert!(errores.contains("equilibrado"), "{}", errores);
}

#[test]
fn el_servidor_responde_413_a_una_peticion_demasiado_grande() {
    let (ruta_rust, ruta_bin) = compilar(Perfil::Servidor, "grande");
    let puerto = puerto_libre();
    let mut servidor = arrancar(&ruta_bin, puerto, &[("DEIMON_MAX_PETICION", "100")]);
    servidor.archivos = vec![ruta_rust, ruta_bin];

    assert!(
        servidor.arranque.contains("perfil servidor"),
        "el arranque no muestra el perfil: {}",
        servidor.arranque
    );

    let relleno = "a".repeat(200);
    let respuesta = enviar(
        puerto,
        &format!(
            "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Relleno: {}\r\n\r\n",
            relleno
        ),
    );
    assert!(
        respuesta.starts_with("HTTP/1.1 413"),
        "una petición grande no respondió 413: {}",
        respuesta
    );
    assert!(
        respuesta.contains("Petición demasiado grande"),
        "la página 413 no está en español: {}",
        respuesta
    );
}

#[test]
fn una_conexion_que_no_envia_nada_se_cierra_pronto() {
    let (ruta_rust, ruta_bin) = compilar(Perfil::Equilibrado, "espera");
    let puerto = puerto_libre();
    let mut servidor = arrancar(&ruta_bin, puerto, &[("DEIMON_TIEMPO_ESPERA", "1")]);
    servidor.archivos = vec![ruta_rust, ruta_bin];

    let mut conexion = TcpStream::connect(("127.0.0.1", puerto))
        .expect("no se pudo conectar con el servidor");
    conexion
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("no se pudo configurar el tiempo de espera");
    let inicio = Instant::now();
    let mut byte = [0u8; 1];
    let _ = conexion.read(&mut byte);
    let pasado = inicio.elapsed();
    assert!(
        pasado < Duration::from_secs(3),
        "la conexión tardó demasiado en cerrarse: {:?}",
        pasado
    );
}

#[test]
fn con_la_cola_llena_otra_conexion_recibe_503() {
    let (ruta_rust, ruta_bin) = compilar(Perfil::Pequeno, "cola");
    let puerto = puerto_libre();
    let mut servidor = arrancar(
        &ruta_bin,
        puerto,
        &[
            ("DEIMON_HILOS", "1"),
            ("DEIMON_MAX_CONEXIONES", "1"),
            ("DEIMON_TIEMPO_ESPERA", "30"),
        ],
    );
    servidor.archivos = vec![ruta_rust, ruta_bin];

    let primera = TcpStream::connect(("127.0.0.1", puerto))
        .expect("no se pudo abrir la primera conexión");
    std::thread::sleep(Duration::from_millis(150));
    let segunda = TcpStream::connect(("127.0.0.1", puerto))
        .expect("no se pudo abrir la segunda conexión");
    std::thread::sleep(Duration::from_millis(250));

    let respuesta = peticion(puerto, "/");
    assert!(
        respuesta.starts_with("HTTP/1.1 503"),
        "con la cola llena no se respondió 503: {}",
        respuesta
    );
    assert!(
        respuesta.contains("Servidor ocupado, inténtalo de nuevo"),
        "la página 503 no está en español: {}",
        respuesta
    );

    drop(primera);
    drop(segunda);
}

#[test]
fn el_arranque_muestra_el_perfil_y_los_hilos_y_la_ruta_que_falla_da_500() {
    let (ruta_rust, ruta_bin) = compilar(Perfil::Pequeno, "pequeno_500");
    let puerto = puerto_libre();
    let mut servidor = arrancar(&ruta_bin, puerto, &[]);
    servidor.archivos = vec![ruta_rust, ruta_bin];

    assert!(
        servidor.arranque.contains("perfil pequeño"),
        "el arranque no muestra el perfil: {}",
        servidor.arranque
    );
    assert!(
        servidor.arranque.contains("2 hilos"),
        "el arranque no muestra los hilos: {}",
        servidor.arranque
    );

    let fallo = peticion(puerto, "/falla");
    assert!(
        fallo.starts_with("HTTP/1.1 500"),
        "una ruta que falla no respondió 500: {}",
        fallo
    );
    assert!(
        fallo.contains("Error interno"),
        "la página 500 no está en español: {}",
        fallo
    );

    let sigue = peticion(puerto, "/");
    assert!(
        sigue.starts_with("HTTP/1.1 200") && sigue.contains("<h1>ok</h1>"),
        "el servidor se cayó después del error: {}",
        sigue
    );
}
