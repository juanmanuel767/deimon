use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Servidor {
    proceso: Child,
    puerto: u16,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.proceso.kill();
        let _ = self.proceso.wait();
    }
}

fn arrancar() -> Servidor {
    // `--puerto 0`: el propio servidor elige un puerto libre y lo anuncia. Así no puede pasar
    // que otra prueba que corre a la vez se quede con el puerto entre medias.
    let mut proceso = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["jugar", "--puerto", "0", "--sin-abrir"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("no se pudo arrancar 'deimon jugar'");
    let mut lector = std::io::BufReader::new(proceso.stdout.take().expect("sin salida"));
    let mut primera = String::new();
    std::io::BufRead::read_line(&mut lector, &mut primera).expect("no se pudo leer la dirección");
    // El resto de lo que escriba se lee aparte, para que nunca se quede sin dónde escribir.
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut lector, &mut std::io::sink());
    });
    let puerto = primera
        .trim()
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or_else(|| panic!("no se entendió la dirección: {:?}", primera));
    let servidor = Servidor { proceso, puerto };
    let limite = Instant::now() + Duration::from_secs(15);
    while Instant::now() < limite {
        if TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            return servidor;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("el servidor no aceptó conexiones en el puerto {}", puerto);
}

fn enviar(puerto: u16, solicitud: &str, cuerpo: &[u8]) -> String {
    let mut conexion = TcpStream::connect(("127.0.0.1", puerto)).expect("no se pudo conectar");
    conexion
        .set_read_timeout(Some(Duration::from_secs(60)))
        .expect("no se pudo configurar el tiempo de espera");
    conexion.write_all(solicitud.as_bytes()).expect("no se pudo enviar");
    let _ = conexion.write_all(cuerpo);
    let mut respuesta = Vec::new();
    let _ = conexion.read_to_end(&mut respuesta);
    String::from_utf8_lossy(&respuesta).to_string()
}

fn pagina(puerto: u16) -> String {
    enviar(
        puerto,
        &format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n", puerto),
        b"",
    )
}

fn clave(puerto: u16) -> String {
    let p = pagina(puerto);
    let inicio = p.find("const CLAVE = \"").expect("la página no lleva clave") + 15;
    let fin = p[inicio..].find('"').expect("clave sin cerrar");
    p[inicio..inicio + fin].to_string()
}

fn ejecutar(puerto: u16, clave: Option<&str>, host: &str, codigo: &[u8]) -> String {
    let cabecera_clave = clave
        .map(|c| format!("X-Deimon-Clave: {}\r\n", c))
        .unwrap_or_default();
    let solicitud = format!(
        "POST /ejecutar HTTP/1.1\r\nHost: {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n",
        host,
        cabecera_clave,
        codigo.len()
    );
    enviar(puerto, &solicitud, codigo)
}

#[test]
fn la_pagina_lleva_clave_y_ejemplos() {
    let s = arrancar();
    let p = pagina(s.puerto);
    assert!(p.starts_with("HTTP/1.1 200"), "la página no respondió 200: {}", p);
    assert!(!clave(s.puerto).is_empty());
    assert!(p.contains("\"nombre\":\"hola\""), "faltan los ejemplos");
    assert!(!p.contains("\"nombre\":\"modulo-propio\""), "no debe ofrecer ejemplos con archivos ajenos");
}

#[test]
fn ejecuta_un_programa_y_devuelve_la_salida() {
    let s = arrancar();
    let c = clave(s.puerto);
    let host = format!("127.0.0.1:{}", s.puerto);
    let r = ejecutar(s.puerto, Some(&c), &host, b"mostrar(\"Hola jugar\")\n");
    assert!(r.starts_with("HTTP/1.1 200"), "{}", r);
    assert!(r.contains("\"salida\":\"Hola jugar\\n\""), "{}", r);
    assert!(r.contains("\"codigo\":0"), "{}", r);
}

#[test]
fn un_programa_con_error_devuelve_el_codigo_de_error() {
    let s = arrancar();
    let c = clave(s.puerto);
    let host = format!("localhost:{}", s.puerto);
    let r = ejecutar(s.puerto, Some(&c), &host, b"mostrar(nombre_que_no_existe)\n");
    assert!(r.starts_with("HTTP/1.1 200"), "{}", r);
    assert!(r.contains("\"correcto\":false"), "{}", r);
    assert!(r.contains("\"codigo\":\""), "falta el código de error: {}", r);
    assert!(r.contains("\"linea\":1"), "{}", r);
}

#[test]
fn un_bucle_infinito_se_corta_pronto() {
    let s = arrancar();
    let c = clave(s.puerto);
    let host = format!("127.0.0.1:{}", s.puerto);
    let inicio = Instant::now();
    let r = ejecutar(s.puerto, Some(&c), &host, b"mientras verdadero {\n}\n");
    let tardo = inicio.elapsed();
    // Lo corta el límite de pasos del intérprete (con mensaje claro) o, si no, el tiempo máximo.
    assert!(r.contains("pasó de 2000000000 pasos") || r.contains("\"agotado\":true"), "{}", r);
    assert!(tardo < Duration::from_secs(25), "tardó demasiado: {:?}", tardo);
}

#[test]
fn sin_clave_o_con_host_falso_se_rechaza() {
    let s = arrancar();
    let c = clave(s.puerto);
    let host = format!("127.0.0.1:{}", s.puerto);
    let sin = ejecutar(s.puerto, None, &host, b"mostrar(1)\n");
    assert!(sin.starts_with("HTTP/1.1 403"), "{}", sin);
    let mala = ejecutar(s.puerto, Some("0000"), &host, b"mostrar(1)\n");
    assert!(mala.starts_with("HTTP/1.1 403"), "{}", mala);
    let falso = ejecutar(s.puerto, Some(&c), "malvado.example.com", b"mostrar(1)\n");
    assert!(falso.starts_with("HTTP/1.1 403"), "{}", falso);
    let pagina_falsa = enviar(
        s.puerto,
        "GET / HTTP/1.1\r\nHost: malvado.example.com\r\nConnection: close\r\n\r\n",
        b"",
    );
    assert!(pagina_falsa.starts_with("HTTP/1.1 403"), "{}", pagina_falsa);
    let origen = enviar(
        s.puerto,
        &format!(
            "POST /ejecutar HTTP/1.1\r\nHost: {}\r\nOrigin: http://malvado.example.com\r\nX-Deimon-Clave: {}\r\nContent-Length: 1\r\n\r\n",
            host, c
        ),
        b"x",
    );
    assert!(origen.starts_with("HTTP/1.1 403"), "{}", origen);
}

#[test]
fn un_cuerpo_demasiado_grande_se_rechaza() {
    let s = arrancar();
    let c = clave(s.puerto);
    let host = format!("127.0.0.1:{}", s.puerto);
    let grande = vec![b'a'; 101 * 1024];
    let r = ejecutar(s.puerto, Some(&c), &host, &grande);
    assert!(r.starts_with("HTTP/1.1 413"), "{}", r);
}
