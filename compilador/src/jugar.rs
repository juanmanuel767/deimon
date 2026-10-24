use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::utilidades::json_texto;

type Fallo = (String, i32);

const PUERTO_POR_DEFECTO: u16 = 8160;
const LIMITE_CUERPO: usize = 100 * 1024;
const LIMITE_SALIDA: usize = 64 * 1024;
const LIMITE_CABECERAS: usize = 16 * 1024;
const TIEMPO_MAXIMO: Duration = Duration::from_secs(15);
const PAGINA: &str = include_str!("jugar.html");

static SECUENCIA: AtomicU64 = AtomicU64::new(0);

struct Estado {
    puerto: u16,
    clave: String,
    ejemplos_json: String,
    ocupado: Mutex<()>,
}

/// `deimon jugar [--puerto N] [--sin-abrir]`
pub fn jugar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon jugar [--puerto N] [--sin-abrir]  (--puerto 0: elige uno libre)";
    let mut puerto = PUERTO_POR_DEFECTO;
    let mut abrir = true;
    let mut i = 0;
    while i < argumentos.len() {
        match argumentos[i].as_str() {
            "--puerto" => {
                i += 1;
                let valor = argumentos
                    .get(i)
                    .ok_or_else(|| (format!("Error: falta el valor de '--puerto'.\n{}", uso), 2))?;
                puerto = valor.parse::<u16>().ok().ok_or_else(|| {
                    (format!("Error: '{}' no es un número de puerto válido.\n{}", valor, uso), 2)
                })?;
            }
            "--sin-abrir" => abrir = false,
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2)),
        }
        i += 1;
    }

    // Solo en este ordenador: nunca se escucha en 0.0.0.0.
    let escucha = TcpListener::bind(("127.0.0.1", puerto)).map_err(|error| {
        (
            format!(
                "Error: no se pudo usar el puerto {} ({}). Prueba con otro: deimon jugar --puerto 8161",
                puerto, error
            ),
            1,
        )
    })?;
    // Con `--puerto 0` el sistema da uno libre: se anuncia el que de verdad se usa.
    let puerto = escucha.local_addr().map(|direccion| direccion.port()).unwrap_or(puerto);
    let clave = clave_aleatoria()?;
    let estado = Arc::new(Estado {
        puerto,
        clave,
        ejemplos_json: ejemplos_en_json(),
        ocupado: Mutex::new(()),
    });

    let direccion = format!("http://127.0.0.1:{}", puerto);
    println!("Deimon jugar está listo en {}", direccion);
    println!("Para terminar, pulsa Ctrl+C.");
    if abrir {
        let _ = Command::new("xdg-open")
            .arg(&direccion)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    for conexion in escucha.incoming().flatten() {
        let estado = Arc::clone(&estado);
        thread::spawn(move || atender(conexion, &estado));
    }
    Ok(())
}

fn clave_aleatoria() -> Result<String, Fallo> {
    let mut bytes = [0u8; 24];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| (format!("Error: no se pudo generar la clave de seguridad: {}", e), 1))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

fn carpeta_ejemplos() -> Option<PathBuf> {
    let mut candidatas = vec![PathBuf::from("ejemplos")];
    if let Ok(exe) = env::current_exe() {
        for padre in exe.ancestors().skip(1).take(4) {
            candidatas.push(padre.join("ejemplos"));
        }
    }
    candidatas.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("ejemplos"));
    candidatas.into_iter().find(|c| c.join("hola.dmn").is_file())
}

/// Ejemplos que se pueden ejecutar sin más: ni web, ni dependen de otros archivos.
fn ejemplos_en_json() -> String {
    let mut lista: Vec<(String, String)> = Vec::new();
    if let Some(carpeta) = carpeta_ejemplos() {
        if let Ok(entradas) = fs::read_dir(carpeta) {
            for entrada in entradas.flatten() {
                let ruta = entrada.path();
                if ruta.extension().and_then(|e| e.to_str()) != Some("dmn") {
                    continue;
                }
                let Ok(fuente) = fs::read_to_string(&ruta) else { continue };
                let nombre = ruta
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("ejemplo")
                    .to_string();
                let sin_web = matches!(
                    deimon::informacion(&fuente),
                    Ok(info) if matches!(info.servir, deimon::UsoServir::Ninguno)
                );
                let independiente = deimon::comprobar_archivo(&fuente, Path::new("/nada/programa.dmn"))
                    .is_empty();
                if sin_web && independiente {
                    lista.push((nombre, fuente));
                }
            }
        }
    }
    lista.sort();
    let trozos: Vec<String> = lista
        .iter()
        .map(|(n, c)| format!("{{\"nombre\":{},\"codigo\":{}}}", json_texto(n), json_texto(c)))
        .collect();
    // "</" escapado para que el texto nunca cierre la etiqueta <script>.
    format!("[{}]", trozos.join(",")).replace("</", "<\\/")
}

struct Peticion {
    metodo: String,
    camino: String,
    cabeceras: Vec<(String, String)>,
    resto: Vec<u8>,
}

impl Peticion {
    fn cabecera(&self, nombre: &str) -> Option<&str> {
        self.cabeceras
            .iter()
            .find(|(n, _)| n == nombre)
            .map(|(_, v)| v.as_str())
    }
}

fn leer_peticion(conexion: &mut TcpStream) -> Option<Peticion> {
    let mut datos = Vec::new();
    let mut trozo = [0u8; 4096];
    let fin = loop {
        if let Some(p) = datos.windows(4).position(|v| v == b"\r\n\r\n") {
            break p;
        }
        if datos.len() > LIMITE_CABECERAS {
            return None;
        }
        let n = conexion.read(&mut trozo).ok()?;
        if n == 0 {
            return None;
        }
        datos.extend_from_slice(&trozo[..n]);
    };
    let texto = String::from_utf8_lossy(&datos[..fin]).to_string();
    let mut lineas = texto.split("\r\n");
    let mut primera = lineas.next()?.split(' ');
    let metodo = primera.next()?.to_string();
    let camino = primera.next()?.to_string();
    let cabeceras = lineas
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Some(Peticion {
        metodo,
        camino,
        cabeceras,
        resto: datos[fin + 4..].to_vec(),
    })
}

fn responder(conexion: &mut TcpStream, codigo: u16, tipo: &str, cuerpo: &str) {
    let motivo = match codigo {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        _ => "Service Unavailable",
    };
    let cabecera = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",
        codigo,
        motivo,
        tipo,
        cuerpo.len()
    );
    let _ = conexion.write_all(cabecera.as_bytes());
    let _ = conexion.write_all(cuerpo.as_bytes());
    let _ = conexion.flush();
}

fn error_json(conexion: &mut TcpStream, codigo: u16, mensaje: &str) {
    let cuerpo = format!("{{\"error\":{}}}", json_texto(mensaje));
    responder(conexion, codigo, "application/json; charset=utf-8", &cuerpo);
}

fn atender(mut conexion: TcpStream, estado: &Estado) {
    let _ = conexion.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = conexion.set_write_timeout(Some(Duration::from_secs(10)));
    let Some(peticion) = leer_peticion(&mut conexion) else {
        error_json(&mut conexion, 400, "Petición no válida.");
        return;
    };

    // Contra DNS rebinding y webs ajenas: solo se acepta el propio servidor.
    let host_ok = matches!(peticion.cabecera("host"),
        Some(h) if h == format!("127.0.0.1:{}", estado.puerto) || h == format!("localhost:{}", estado.puerto));
    let origen_ok = match peticion.cabecera("origin") {
        None => true,
        Some(o) => {
            o == format!("http://127.0.0.1:{}", estado.puerto)
                || o == format!("http://localhost:{}", estado.puerto)
        }
    };
    if !host_ok || !origen_ok {
        error_json(&mut conexion, 403, "Petición rechazada: no viene de esta página.");
        return;
    }

    match (peticion.metodo.as_str(), peticion.camino.as_str()) {
        ("GET", "/") => {
            let pagina = PAGINA
                .replace("{{CLAVE}}", &estado.clave)
                .replace("{{EJEMPLOS}}", &estado.ejemplos_json);
            responder(&mut conexion, 200, "text/html; charset=utf-8", &pagina);
        }
        ("POST", "/ejecutar") => post_ejecutar(&mut conexion, &peticion, estado),
        (_, "/") | (_, "/ejecutar") => error_json(&mut conexion, 405, "Método no permitido."),
        _ => error_json(&mut conexion, 404, "No existe esa dirección."),
    }
}

fn post_ejecutar(conexion: &mut TcpStream, peticion: &Peticion, estado: &Estado) {
    let clave_ok = matches!(peticion.cabecera("x-deimon-clave"),
        Some(c) if c.len() == estado.clave.len()
            && c.bytes().zip(estado.clave.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0);
    if !clave_ok {
        error_json(conexion, 403, "Falta la clave de seguridad. Recarga la página.");
        return;
    }
    let Some(largo) = peticion.cabecera("content-length").and_then(|l| l.parse::<usize>().ok())
    else {
        error_json(conexion, 411, "Falta la longitud del cuerpo.");
        return;
    };
    if largo > LIMITE_CUERPO {
        error_json(conexion, 413, "El código es demasiado grande (máximo 100 KB).");
        // Se vacía lo que el navegador siga enviando para que reciba bien la respuesta.
        let _ = conexion.shutdown(Shutdown::Write);
        let _ = conexion.set_read_timeout(Some(Duration::from_millis(500)));
        let mut resto = [0u8; 8192];
        let mut leido = peticion.resto.len();
        while leido < largo.min(4 * 1024 * 1024) {
            match conexion.read(&mut resto) {
                Ok(0) | Err(_) => break,
                Ok(n) => leido += n,
            }
        }
        return;
    }
    let mut cuerpo = peticion.resto.clone();
    let mut trozo = [0u8; 8192];
    while cuerpo.len() < largo {
        match conexion.read(&mut trozo) {
            Ok(0) | Err(_) => {
                error_json(conexion, 400, "El cuerpo llegó incompleto.");
                return;
            }
            Ok(n) => cuerpo.extend_from_slice(&trozo[..n]),
        }
    }
    cuerpo.truncate(largo);
    let Ok(fuente) = String::from_utf8(cuerpo) else {
        error_json(conexion, 400, "El código debe ser texto UTF-8.");
        return;
    };

    // Una ejecución a la vez.
    let Ok(_turno) = estado.ocupado.try_lock() else {
        error_json(conexion, 503, "Ocupado: hay otro programa ejecutándose. Prueba en unos segundos.");
        return;
    };
    let respuesta = comprobar_y_ejecutar(&fuente);
    responder(conexion, 200, "application/json; charset=utf-8", &respuesta);
}

fn comprobar_y_ejecutar(fuente: &str) -> String {
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let carpeta = env::temp_dir().join(format!(
        "deimon_jugar_{}_{}_{}",
        std::process::id(),
        marca,
        SECUENCIA.fetch_add(1, Ordering::Relaxed)
    ));
    if fs::create_dir_all(&carpeta).is_err() {
        return "{\"error\":\"No se pudo crear la carpeta temporal.\"}".to_string();
    }
    let respuesta = en_carpeta(fuente, &carpeta);
    let _ = fs::remove_dir_all(&carpeta);
    respuesta
}

fn en_carpeta(fuente: &str, carpeta: &Path) -> String {
    let archivo = carpeta.join("programa.dmn");
    if fs::write(&archivo, fuente).is_err() {
        return "{\"error\":\"No se pudo guardar el programa.\"}".to_string();
    }
    let errores = deimon::comprobar_archivo(fuente, &archivo);
    if !errores.is_empty() {
        return crate::comprobar::a_json("programa.dmn", &errores, None);
    }
    let exe = match env::current_exe() {
        Ok(e) => e,
        Err(e) => return format!("{{\"error\":{}}}", json_texto(&e.to_string())),
    };
    let mut hijo = match Command::new(exe)
        .args(["ejecutar", "programa.dmn"])
        .current_dir(carpeta)
        .env("TMPDIR", carpeta)
        // Límites del intérprete: un bucle sin fin o una salida enorme se cortan con un
        // mensaje claro antes de llegar al tiempo máximo.
        .env("DEIMON_MAX_PASOS", "2000000000")
        .env("DEIMON_MAX_SALIDA", (64 * 1024).to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
    {
        Ok(h) => h,
        Err(e) => return format!("{{\"error\":{}}}", json_texto(&format!("No se pudo ejecutar: {}", e))),
    };

    let salida: Arc<Mutex<(Vec<u8>, bool)>> = Arc::new(Mutex::new((Vec::new(), false)));
    let mut lectores = Vec::new();
    if let Some(o) = hijo.stdout.take() {
        lectores.push(leer_tuberia(o, Arc::clone(&salida)));
    }
    if let Some(e) = hijo.stderr.take() {
        lectores.push(leer_tuberia(e, Arc::clone(&salida)));
    }

    let limite = Instant::now() + TIEMPO_MAXIMO;
    let mut agotado = false;
    let codigo = loop {
        match hijo.try_wait() {
            Ok(Some(estado)) => break estado.code(),
            Ok(None) if Instant::now() >= limite => {
                agotado = true;
                matar_grupo(hijo.id());
                let _ = hijo.kill();
                let _ = hijo.wait();
                break None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };
    // Si algún proceso nieto sigue vivo, se corta también.
    matar_grupo(hijo.id());
    for lector in lectores {
        let _ = lector.join();
    }
    let (bytes, recortado) = {
        let guardado = salida.lock().unwrap_or_else(|e| e.into_inner());
        (guardado.0.clone(), guardado.1)
    };
    let texto = String::from_utf8_lossy(&bytes).to_string();
    format!(
        "{{\"correcto\":true,\"salida\":{},\"codigo\":{},\"agotado\":{},\"recortado\":{}}}",
        json_texto(&texto),
        codigo.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string()),
        agotado,
        recortado
    )
}

fn matar_grupo(pid: u32) {
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg("--")
        .arg(format!("-{}", pid))
        .stderr(Stdio::null())
        .status();
}

fn leer_tuberia<R: Read + Send + 'static>(
    mut origen: R,
    salida: Arc<Mutex<(Vec<u8>, bool)>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut trozo = [0u8; 8192];
        // Se sigue leyendo aunque ya no quepa nada, para que el programa nunca se quede bloqueado.
        while let Ok(n) = origen.read(&mut trozo) {
            if n == 0 {
                break;
            }
            let mut guardado = salida.lock().unwrap_or_else(|e| e.into_inner());
            let sitio = LIMITE_SALIDA.saturating_sub(guardado.0.len());
            if n > sitio {
                guardado.1 = true;
            }
            guardado.0.extend_from_slice(&trozo[..n.min(sitio)]);
        }
    })
}
