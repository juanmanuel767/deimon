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
    compilar_fuente(archivo, &fuente)
}

fn compilar_fuente(archivo: &str, fuente: &str) -> (PathBuf, PathBuf) {
    let rust = deimon::traducir(fuente)
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

    let otro = peticion_completa(puerto, "OPTIONS / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
    assert!(otro.starts_with("HTTP/1.1 405"), "{}", otro);
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

#[test]
fn el_servidor_resiste_tamanos_trucados_y_clientes_lentos() {
    let (ruta_rust, ruta_bin) = compilar_sitio("cuentas.dmn");
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_ataques_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .env("DEIMON_DATOS", &datos)
        .env("DEIMON_HILOS", "2")
        .env("DEIMON_TIEMPO_ESPERA", "2")
        .stdin(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");
    let _servidor = Servidor {
        proceso,
        archivos: vec![ruta_rust, ruta_bin],
    };
    esperar_servidor(puerto);

    // Un tamaño que, sumado a la cabecera, daría la vuelta: antes se saltaba el límite.
    let vuelta = peticion_completa(puerto, "POST /entrar HTTP/1.1\r\nContent-Length: 18446744073709551606\r\n\r\n");
    assert!(vuelta.starts_with("HTTP/1.1 413"), "{}", vuelta);
    let doble = peticion_completa(puerto, "POST /entrar HTTP/1.1\r\nContent-Length: 3\r\nContent-Length: 9\r\n\r\nabc");
    assert!(doble.starts_with("HTTP/1.1 400"), "{}", doble);
    let trozos = peticion_completa(puerto, "POST /entrar HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");
    assert!(trozos.starts_with("HTTP/1.1 400"), "{}", trozos);

    // Clientes que mandan la cabecera gota a gota: no pueden retener los hilos más que el plazo.
    let mut lentos = Vec::new();
    for _ in 0..2 {
        let mut c = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
        c.write_all(b"GET / HTTP/1.1\r\n").unwrap();
        lentos.push(c);
    }
    let inicio = Instant::now();
    while inicio.elapsed() < Duration::from_secs(3) {
        for c in &mut lentos {
            let _ = c.write_all(b"X: y\r\n");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let normal = peticion(puerto, "/");
    assert!(normal.starts_with("HTTP/1.1 200"), "{}", normal);
}

fn subir(puerto: u16, camino: &str, campos: &[(&str, &str)], archivo: Option<(&str, &str, &[u8])>) -> Vec<u8> {
    let frontera = "----deimonprueba123";
    let mut cuerpo: Vec<u8> = Vec::new();
    for (nombre, valor) in campos {
        cuerpo.extend_from_slice(format!("--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n", frontera, nombre, valor).as_bytes());
    }
    if let Some((campo, nombre, datos)) = archivo {
        cuerpo.extend_from_slice(format!("--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: image/png\r\n\r\n", frontera, campo, nombre).as_bytes());
        cuerpo.extend_from_slice(datos);
        cuerpo.extend_from_slice(b"\r\n");
    }
    cuerpo.extend_from_slice(format!("--{}--\r\n", frontera).as_bytes());
    let mut conexion = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    conexion.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    conexion
        .write_all(format!("POST {} HTTP/1.1\r\nContent-Type: multipart/form-data; boundary={}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", camino, frontera, cuerpo.len()).as_bytes())
        .unwrap();
    conexion.write_all(&cuerpo).unwrap();
    let mut respuesta = Vec::new();
    let _ = conexion.read_to_end(&mut respuesta);
    respuesta
}

#[test]
fn se_suben_fotos_se_sirven_y_las_galletas_se_guardan() {
    let (ruta_rust, ruta_bin) = compilar_sitio("fotos.dmn");
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_fotos_{}", std::process::id()));
    let _ = fs::remove_dir_all(&datos);
    let proceso = Command::new(&ruta_bin)
        .env("DEIMON_PUERTO", puerto.to_string())
        .env("DEIMON_DATOS", &datos)
        .env("DEIMON_MAX_SUBIDA", "100000")
        .stdin(Stdio::null())
        .spawn()
        .expect("no se pudo arrancar el servidor");
    let _servidor = Servidor { proceso, archivos: vec![ruta_rust, ruta_bin] };
    esperar_servidor(puerto);

    // Una imagen PNG mínima (la cabecera basta para reconocerla) con bytes de todo tipo.
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend((0..=255u8).cycle().take(3000));
    let subida = String::from_utf8_lossy(&subir(puerto, "/subir", &[("titulo", "Mi <foto>")], Some(("foto", "C:\\fotos\\gato.png", &png)))).to_string();
    assert!(subida.starts_with("HTTP/1.1 303"), "{}", subida);
    let inicio = peticion(puerto, "/");
    assert!(inicio.contains("Mi &lt;foto&gt;"), "{}", inicio);
    let archivo = inicio.split("/foto/").nth(1).and_then(|r| r.split('\'').next()).expect("la foto no aparece").to_string();
    assert!(archivo.ends_with(".png") && !archivo.contains("gato"), "{}", archivo);

    let mut conexion = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    conexion.write_all(format!("GET /foto/{} HTTP/1.1\r\nConnection: close\r\n\r\n", archivo).as_bytes()).unwrap();
    let mut respuesta = Vec::new();
    conexion.read_to_end(&mut respuesta).unwrap();
    let fin = respuesta.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
    let cabeceras = String::from_utf8_lossy(&respuesta[..fin]).to_string();
    assert!(cabeceras.contains("Content-Type: image/png") && cabeceras.contains("nosniff"), "{}", cabeceras);
    assert_eq!(&respuesta[fin..], &png[..], "la foto no llegó idéntica");

    // Un HTML disfrazado de imagen no se acepta como foto.
    let trampa = String::from_utf8_lossy(&subir(puerto, "/subir", &[], Some(("foto", "x.png", b"<script>alert(1)</script>")))).to_string();
    assert!(trampa.contains("Elige una imagen"), "{}", trampa);
    // Ni salir de la carpeta ni pasar del tamaño máximo.
    assert!(peticion(puerto, "/foto/..%2F..%2Fsecreto").contains("no existe"));
    let enorme = String::from_utf8_lossy(&subir(puerto, "/subir", &[], Some(("foto", "g.png", &vec![0u8; 200_000])))).to_string();
    assert!(enorme.starts_with("HTTP/1.1 413"), "{}", enorme);

    let nombre = enviar_formulario(puerto, "/nombre", "nombre=Ana+%22la%22", "");
    let galleta = nombre.lines().find(|l| l.starts_with("Set-Cookie: nombre=")).expect("no guardó la galleta").to_string();
    assert!(galleta.contains("HttpOnly") && galleta.contains("Max-Age=2592000"), "{}", galleta);
    let valor = galleta.trim_start_matches("Set-Cookie: ").split(';').next().unwrap().to_string();
    let saludo = peticion_completa(puerto, &format!("GET / HTTP/1.1\r\nCookie: {}\r\nConnection: close\r\n\r\n", valor));
    assert!(saludo.contains("Hola, Ana &quot;la&quot;"), "{}", saludo);
    let _ = fs::remove_dir_all(&datos);
}

// ---------------------------------------------------------------------------
// API JSON: responder, responder_json, métodos PUT/PATCH/DELETE y enteros grandes.
// ---------------------------------------------------------------------------

const PROGRAMA_API: &str = r#"
ruta "/texto" {
    devolver responder("no existe", 404, "text/plain")
}
ruta "/css" {
    devolver responder("body{margin:0}", 200, "text/css; charset=iso-8859-1")
}
ruta "/imagen" {
    devolver responder("x", 200, "image/svg+xml")
}
ruta "/json" {
    devolver responder_json({"id": 1, "ok": verdadero, "nada": nada, "lista": [1, "a"], "z": 2, "a": 3}, 201)
}
ruta "/grande" {
    sea datos = desde_json(cuerpo_recibido())
    devolver responder_json({"n": datos["n"], "doble": datos["n"] * 2}, 200)
}
ruta "/metodo" {
    devolver responder_json({"metodo": metodo_recibido(), "cuerpo": cuerpo_recibido(), "envio": es_envio(), "camino": camino_recibido()}, 200)
}
ruta "/codigo/:c" {
    devolver responder("x", a_numero(c), "text/plain")
}
ruta "/tipo-vacio" {
    devolver responder("x", 200, "")
}
ruta "/cuerpo-numero" {
    sea x = leer("c", "c")
    si x == nada {
        x = 5
    }
    devolver responder(x, 200, "text/plain")
}
ruta "/json-codigo-texto" {
    sea x = leer("c", "c")
    si x == nada {
        x = "200"
    }
    devolver responder_json(1, x)
}
ruta "/*" {
    devolver responder("no encontrado", 404, "text/plain")
}
servir(8080)
"#;

/// Arranca la API con el motor indicado: "interprete" (`deimon ejecutar`) o "rust" (el programa
/// compilado). Devuelve el servidor (se para al soltarlo) y su puerto.
fn arrancar_api(motor: &str) -> (Servidor, u16) {
    let puerto = puerto_libre();
    let datos = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("datos_api_{}_{}", std::process::id(), motor));
    let servidor = if motor == "rust" {
        let (ruta_rust, ruta_bin) = compilar_fuente("api_json.dmn", PROGRAMA_API);
        let proceso = Command::new(&ruta_bin)
            .env("DEIMON_PUERTO", puerto.to_string())
            .env("DEIMON_DATOS", &datos)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("no se pudo arrancar el servidor");
        Servidor { proceso, archivos: vec![ruta_rust, ruta_bin] }
    } else {
        let fuente = std::env::temp_dir().join(format!("ln_api_{}_{}.dmn", std::process::id(), motor));
        fs::write(&fuente, PROGRAMA_API).expect("no se pudo escribir el programa");
        let proceso = Command::new(env!("CARGO_BIN_EXE_deimon"))
            .arg("ejecutar")
            .arg(&fuente)
            .env("DEIMON_MOTOR", motor)
            .env("DEIMON_PUERTO", puerto.to_string())
            .env("DEIMON_DATOS", &datos)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("no se pudo arrancar el servidor");
        Servidor { proceso, archivos: vec![fuente] }
    };
    esperar_servidor(puerto);
    (servidor, puerto)
}

fn solicitud(puerto: u16, metodo: &str, camino: &str, cuerpo: &str) -> String {
    peticion_completa(
        puerto,
        &format!(
            "{} {} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            metodo,
            camino,
            cuerpo.len(),
            cuerpo
        ),
    )
}

fn cuerpo_de(respuesta: &str) -> &str {
    respuesta.split_once("\r\n\r\n").map(|(_, cuerpo)| cuerpo).unwrap_or("")
}

fn comprobar_api(motor: &str) {
    let (_servidor, puerto) = arrancar_api(motor);

    let texto = solicitud(puerto, "GET", "/texto", "");
    assert!(texto.starts_with("HTTP/1.1 404 Not Found"), "{}", texto);
    assert!(texto.contains("Content-Type: text/plain; charset=utf-8\r\n"), "{}", texto);
    assert_eq!(cuerpo_de(&texto), "no existe");

    let css = solicitud(puerto, "GET", "/css", "");
    assert!(css.starts_with("HTTP/1.1 200 OK") && css.contains("Content-Type: text/css; charset=iso-8859-1\r\n"), "{}", css);
    let imagen = solicitud(puerto, "GET", "/imagen", "");
    assert!(imagen.contains("Content-Type: image/svg+xml\r\n"), "{}", imagen);

    let json = solicitud(puerto, "GET", "/json", "");
    assert!(json.starts_with("HTTP/1.1 201 Created"), "{}", json);
    assert!(json.contains("Content-Type: application/json\r\n"), "{}", json);
    assert_eq!(cuerpo_de(&json), r#"{"id":1,"ok":true,"nada":null,"lista":[1,"a"],"z":2,"a":3}"#);

    // Entero grande exacto al leer y al escribir.
    let grande = solicitud(puerto, "POST", "/grande", r#"{"n": 99999999999999999999}"#);
    assert_eq!(cuerpo_de(&grande), r#"{"n":99999999999999999999,"doble":199999999999999999998}"#, "{}", grande);

    // PUT, PATCH y DELETE llegan a la ruta, con su cuerpo y es_envio() verdadero.
    for metodo in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
        let cuerpo = if metodo == "GET" { "" } else { "{\"a\": 1}" };
        let respuesta = solicitud(puerto, metodo, "/metodo?x=1", cuerpo);
        assert!(respuesta.starts_with("HTTP/1.1 200 OK"), "{}: {}", metodo, respuesta);
        let esperado = format!(
            r#"{{"metodo":"{}","cuerpo":"{}","envio":{},"camino":"/metodo"}}"#,
            metodo,
            cuerpo.replace('"', "\\\""),
            metodo != "GET"
        );
        assert_eq!(cuerpo_de(&respuesta), esperado, "{}", respuesta);
    }
    let barra = solicitud(puerto, "GET", "/metodo/", "");
    assert!(cuerpo_de(&barra).contains(r#""camino":"/metodo/""#), "{}", barra);

    // Lo que no existe cae en la ruta "/*"; un método no admitido sigue dando 405.
    let nada = solicitud(puerto, "DELETE", "/no/existe/nunca", "");
    assert!(nada.starts_with("HTTP/1.1 404 Not Found") && cuerpo_de(&nada) == "no encontrado", "{}", nada);
    let raro = solicitud(puerto, "OPTIONS", "/texto", "");
    assert!(raro.starts_with("HTTP/1.1 405"), "{}", raro);
    let cabeza = solicitud(puerto, "HEAD", "/texto", "");
    assert!(cabeza.starts_with("HTTP/1.1 405"), "{}", cabeza);

    // Código fuera de rango o argumentos que no son del tipo pedido: error 500 y el servidor sigue vivo.
    for malo in ["/codigo/99", "/codigo/600", "/codigo/200.5", "/tipo-vacio", "/cuerpo-numero", "/json-codigo-texto"] {
        let respuesta = solicitud(puerto, "GET", malo, "");
        assert!(respuesta.starts_with("HTTP/1.1 500"), "{}: {}", malo, respuesta);
    }
    let bien = solicitud(puerto, "GET", "/codigo/418", "");
    assert!(bien.starts_with("HTTP/1.1 418"), "{}", bien);
    let vivo = solicitud(puerto, "GET", "/texto", "");
    assert!(vivo.starts_with("HTTP/1.1 404"), "{}", vivo);
}

#[test]
fn la_api_json_responde_codigo_tipo_y_metodos_con_el_interprete() {
    comprobar_api("interprete");
}

#[test]
fn la_api_json_responde_codigo_tipo_y_metodos_con_el_programa_compilado() {
    comprobar_api("rust");
}
