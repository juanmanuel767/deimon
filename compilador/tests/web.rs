use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn carpeta_web() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("ejemplos-web")
}

fn puerto_libre() -> u16 {
    let escucha =
        TcpListener::bind("127.0.0.1:0").expect("no se pudo reservar un puerto libre");
    escucha
        .local_addr()
        .expect("no se pudo leer la dirección del puerto")
        .port()
}

struct Servidor {
    proceso: Child,
    archivos: Vec<PathBuf>,
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

fn compilar_sitio(archivo: &str) -> (PathBuf, PathBuf) {
    let ruta_fuente = carpeta_web().join(archivo);
    let fuente = fs::read_to_string(&ruta_fuente)
        .unwrap_or_else(|error| panic!("no se pudo leer '{}': {}", ruta_fuente.display(), error));
    let rust = deimon::traducir(&fuente)
        .unwrap_or_else(|error| panic!("no se pudo traducir el ejemplo web: {}", error));

    let carpeta = std::env::temp_dir();
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    // Cada llamada usa su propio nombre: dos pruebas pueden compilar el mismo sitio a la vez.
    static SECUENCIA: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let marca = format!(
        "{}_{}_{}",
        std::process::id(),
        archivo.replace('.', "_"),
        SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let ruta_rust = carpeta.join(format!("ln_web_{}.rs", marca));
    let ruta_bin = carpeta.join(format!("ln_web_{}", marca));
    fs::write(&ruta_rust, rust).expect("no se pudo escribir el Rust generado");

    let salida = Command::new("rustc")
        .arg("-O")
        .arg(&ruta_rust)
        .arg("-o")
        .arg(&ruta_bin)
        .output()
        .expect("no se pudo ejecutar 'rustc'");
    assert!(
        salida.status.success(),
        "no se pudo compilar el sitio web: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    (ruta_rust, ruta_bin)
}

fn esperar_servidor(puerto: u16) {
    let limite = Instant::now() + Duration::from_secs(15);
    while Instant::now() < limite {
        if TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("el servidor no aceptó conexiones en el puerto {}", puerto);
}

fn peticion(puerto: u16, camino: &str) -> String {
    let mut conexion = TcpStream::connect(("127.0.0.1", puerto))
        .expect("no se pudo conectar con el servidor");
    conexion
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("no se pudo configurar el tiempo de espera");
    let solicitud = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        camino
    );
    conexion
        .write_all(solicitud.as_bytes())
        .expect("no se pudo enviar la petición");
    let mut respuesta = String::new();
    conexion
        .read_to_string(&mut respuesta)
        .expect("no se pudo leer la respuesta");
    respuesta
}

#[test]
fn el_sitio_web_responde_las_paginas() {
    let (ruta_rust, ruta_bin) = compilar_sitio("sitio.dmn");
    let puerto = puerto_libre();
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .stdin(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");
    let _servidor = Servidor {
        proceso,
        archivos: vec![ruta_rust, ruta_bin],
    };

    esperar_servidor(puerto);

    let inicio = peticion(puerto, "/");
    assert!(
        inicio.starts_with("HTTP/1.1 200"),
        "la página de inicio no respondió 200: {}",
        inicio
    );
    assert!(
        inicio.contains("<h1>Inicio</h1>"),
        "la página de inicio no tiene el HTML esperado: {}",
        inicio
    );

    let saludo = peticion(puerto, "/saludo/Ana");
    assert!(
        saludo.contains("Hola, Ana"),
        "la página de saludo no contiene el nombre: {}",
        saludo
    );

    let malicioso = peticion(puerto, "/saludo/%3Cscript%3E");
    assert!(
        malicioso.contains("&lt;script&gt;"),
        "el parámetro no se escapó como HTML: {}",
        malicioso
    );
    assert!(
        !malicioso.contains("<script>"),
        "el parámetro se entregó sin escapar: {}",
        malicioso
    );

    let cuadrado = peticion(puerto, "/cuadrado/5");
    assert!(
        cuadrado.contains("El cuadrado de 5 es 25"),
        "la página calculada no es correcta: {}",
        cuadrado
    );

    let error_interno = peticion(puerto, "/cuadrado/abc");
    assert!(
        error_interno.starts_with("HTTP/1.1 500"),
        "un error de la ruta no respondió 500: {}",
        error_interno
    );
    assert!(
        error_interno.contains("Error interno"),
        "la página de error no está en español: {}",
        error_interno
    );

    let sigue_vivo = peticion(puerto, "/");
    assert!(
        sigue_vivo.starts_with("HTTP/1.1 200"),
        "el servidor se cayó después de un error de ruta: {}",
        sigue_vivo
    );

    let sin_ruta = peticion(puerto, "/no-existe");
    assert!(
        sin_ruta.starts_with("HTTP/1.1 404"),
        "una ruta desconocida no respondió 404: {}",
        sin_ruta
    );
    assert!(
        sin_ruta.contains("Página no encontrada"),
        "la página 404 no está en español: {}",
        sin_ruta
    );
}

#[test]
fn el_libro_de_visitas_guarda_las_firmas() {
    let (ruta_rust, ruta_bin) = compilar_sitio("libro.dmn");
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_libro_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .env("DEIMON_DATOS", &datos)
        .stdin(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");
    let _servidor = Servidor {
        proceso,
        archivos: vec![ruta_rust, ruta_bin],
    };

    esperar_servidor(puerto);

    let primera = peticion(puerto, "/");
    assert!(
        primera.contains("Visitas totales: 1"),
        "la primera visita no se contó: {}",
        primera
    );
    let segunda = peticion(puerto, "/");
    assert!(
        segunda.contains("Visitas totales: 2"),
        "la segunda visita no se contó: {}",
        segunda
    );

    let firma = peticion(puerto, "/firmar/Ana");
    assert!(
        firma.starts_with("HTTP/1.1 200") && firma.contains("Gracias, Ana"),
        "la firma no se confirmó: {}",
        firma
    );
    let firmas = peticion(puerto, "/firmas");
    assert!(
        firmas.contains("<li>Ana</li>"),
        "la firma de Ana no aparece: {}",
        firmas
    );

    let malicioso = peticion(puerto, "/firmar/%3Cb%3E");
    assert!(
        malicioso.starts_with("HTTP/1.1 200"),
        "la firma maliciosa falló: {}",
        malicioso
    );
    let firmas = peticion(puerto, "/firmas");
    assert!(
        firmas.contains("&lt;b&gt;"),
        "la firma no se escapó como HTML: {}",
        firmas
    );
    assert!(
        !firmas.contains("<b>"),
        "la firma se entregó sin escapar: {}",
        firmas
    );
    let _ = fs::remove_dir_all(&datos);
}

fn peticion_completa(puerto: u16, texto: &str) -> String {
    let mut conexion = TcpStream::connect(("127.0.0.1", puerto)).expect("no se pudo conectar");
    conexion.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    conexion.write_all(texto.as_bytes()).unwrap();
    let mut respuesta = String::new();
    let _ = conexion.read_to_string(&mut respuesta);
    respuesta
}

fn enviar_formulario(puerto: u16, camino: &str, datos: &str, galleta: &str) -> String {
    peticion_completa(
        puerto,
        &format!(
            "POST {} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nCookie: {}\r\nConnection: close\r\n\r\n{}",
            camino,
            datos.len(),
            galleta,
            datos
        ),
    )
}

#[test]
fn las_cuentas_registran_entran_y_protegen_la_pagina_privada() {
    let (ruta_rust, ruta_bin) = compilar_sitio("cuentas.dmn");
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_cuentas_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .env("DEIMON_DATOS", &datos)
        .env("DEIMON_MAX_PETICION", "4096")
        .stdin(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");
    let _servidor = Servidor {
        proceso,
        archivos: vec![ruta_rust, ruta_bin],
    };
    esperar_servidor(puerto);

    let registro = enviar_formulario(puerto, "/registro", "nombre=ana&clave=secreta%2612", "");
    assert!(registro.starts_with("HTTP/1.1 303"), "{}", registro);
    assert!(registro.contains("Location: /privado"), "{}", registro);
    let galleta = registro
        .lines()
        .find_map(|linea| linea.strip_prefix("Set-Cookie: "))
        .and_then(|valor| valor.split(';').next())
        .expect("el registro no abrió sesión")
        .to_string();
    assert!(registro.contains("HttpOnly") && registro.contains("SameSite=Lax"), "{}", registro);

    let privado = peticion_completa(
        puerto,
        &format!("GET /privado HTTP/1.1\r\nHost: x\r\nCookie: {}\r\nConnection: close\r\n\r\n", galleta),
    );
    assert!(privado.contains("ana, has entrado aquí 1 veces"), "{}", privado);

    let sin_sesion = peticion(puerto, "/privado");
    assert!(sin_sesion.contains("Location: /entrar"), "{}", sin_sesion);

    let mala = enviar_formulario(puerto, "/entrar", "nombre=ana&clave=otra", "");
    assert!(mala.contains("incorrectos"), "{}", mala);
    let buena = enviar_formulario(puerto, "/entrar", "nombre=ana&clave=secreta%2612", "");
    assert!(buena.contains("Location: /privado"), "{}", buena);

    let enorme = enviar_formulario(puerto, "/entrar", &format!("nombre={}", "a".repeat(8000)), "");
    assert!(enorme.starts_with("HTTP/1.1 413"), "{}", enorme);

    let borrar = peticion_completa(puerto, "DELETE / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
    assert!(borrar.starts_with("HTTP/1.1 405"), "{}", borrar);
}

#[test]
fn la_sesion_sigue_abierta_tras_reiniciar_el_servidor() {
    let (ruta_rust, ruta_bin) = compilar_sitio("cuentas.dmn");
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_reinicio_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let arrancar = |puerto: u16| {
        Command::new(&ruta_bin)
            .env("DEIMON_PUERTO", puerto.to_string())
            .env("DEIMON_DATOS", &datos)
            .stdin(Stdio::null())
            .spawn()
            .expect("no se pudo arrancar el servidor")
    };
    let puerto = puerto_libre();
    let mut primero = arrancar(puerto);
    esperar_servidor(puerto);
    let registro = enviar_formulario(puerto, "/registro", "nombre=ana&clave=secreta12", "");
    let galleta = registro
        .lines()
        .find_map(|linea| linea.strip_prefix("Set-Cookie: "))
        .and_then(|valor| valor.split(';').next())
        .expect("no abrió sesión")
        .to_string();
    let _ = primero.kill();
    let _ = primero.wait();

    let puerto = puerto_libre();
    let segundo = arrancar(puerto);
    let _servidor = Servidor {
        proceso: segundo,
        archivos: vec![ruta_rust, ruta_bin],
    };
    esperar_servidor(puerto);
    let privado = peticion_completa(
        puerto,
        &format!("GET /privado HTTP/1.1\r\nHost: x\r\nCookie: {}\r\nConnection: close\r\n\r\n", galleta),
    );
    assert!(privado.contains("ana, has entrado aquí 1 veces"), "{}", privado);
    let sesiones = fs::read_to_string(datos.join(".sesiones.registro")).unwrap();
    let codigo = galleta.split('=').nth(1).unwrap();
    assert!(!sesiones.contains(codigo), "el código de sesión se guardó tal cual");
}

#[test]
fn por_https_las_galletas_van_marcadas_como_seguras() {
    let (ruta_rust, ruta_bin) = compilar_sitio("cuentas.dmn");
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_seguras_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .env("DEIMON_DATOS", &datos)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let _servidor = Servidor { proceso, archivos: vec![ruta_rust, ruta_bin] };
    esperar_servidor(puerto);
    let cuerpo = "nombre=ana&clave=secreta12";
    let por_https = peticion_completa(puerto, &format!(
        "POST /registro HTTP/1.1\r\nHost: x\r\nX-Forwarded-Proto: https\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", cuerpo.len(), cuerpo));
    assert!(por_https.contains("SameSite=Lax; Max-Age=604800; Secure"), "{}", por_https);
    let cuerpo = "nombre=luis&clave=secreta12";
    let sin_https = enviar_formulario(puerto, "/registro", cuerpo, "");
    assert!(sin_https.contains("Set-Cookie:") && !sin_https.contains("Secure"), "{}", sin_https);
}
