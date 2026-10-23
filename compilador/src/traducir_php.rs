//! `deimon desde-php archivo.php [--salida archivo.dmn] [--comprobar]`: traduce PHP a Deimon
//! y, con --comprobar, ejecuta los dos y compara que escriben exactamente lo mismo.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Fallo = (String, i32);

const USO: &str = "Uso: deimon desde-php archivo.php|carpeta [--salida archivo.dmn] [--comprobar] [--visitar \"/pagina.php?a=1\"]... [--enviar \"/pagina.php\" \"a=1&b=2\"]...";

/// Una petición para comparar las dos páginas: (camino, datos del formulario si es envío).
type Visita = (String, Option<String>);

pub fn desde_php(argumentos: &[String]) -> Result<(), Fallo> {
    let mut archivo = None;
    let mut salida = None;
    let mut comprobar = false;
    let mut visitas: Vec<Visita> = Vec::new();
    let mut resto = argumentos.iter();
    while let Some(argumento) = resto.next() {
        match argumento.as_str() {
            "--comprobar" => comprobar = true,
            "--visitar" => match resto.next() {
                Some(camino) => visitas.push((camino.clone(), None)),
                None => return Err((format!("Error: falta el camino después de --visitar. {}", USO), 2)),
            },
            "--enviar" => match (resto.next(), resto.next()) {
                (Some(camino), Some(datos)) => visitas.push((camino.clone(), Some(datos.clone()))),
                _ => return Err((format!("Error: --enviar necesita el camino y los datos. {}", USO), 2)),
            },
            "--salida" => match resto.next() {
                Some(ruta) => salida = Some(PathBuf::from(ruta)),
                None => return Err((format!("Error: falta la ruta después de --salida. {}", USO), 2)),
            },
            otro if otro.starts_with("--") => return Err((format!("Error: la opción '{}' no existe. {}", otro, USO), 2)),
            otro if archivo.is_none() => archivo = Some(PathBuf::from(otro)),
            otro => return Err((format!("Error: sobra el argumento '{}'. {}", otro, USO), 2)),
        }
    }
    let archivo = archivo.ok_or_else(|| (USO.to_string(), 2))?;
    let paginas = paginas_php(&archivo)?;
    let es_web = archivo.is_dir() || paginas.iter().any(|(_, php)| parece_web(php));
    let traduccion = if es_web {
        deimon::desde_php::traducir_web(&paginas, 8000)
    } else {
        deimon::desde_php::traducir(&paginas[0].1)
    }
    .map_err(|error| (format!("Error: el PHP no se pudo leer: {}", error), 1))?;
    let salida = salida.unwrap_or_else(|| {
        if archivo.is_dir() {
            // Con "." o "carpeta/" se usa el nombre real de la carpeta: sitio/ → sitio.dmn al lado.
            let carpeta = std::fs::canonicalize(&archivo).unwrap_or_else(|_| archivo.clone());
            PathBuf::from(format!("{}.dmn", carpeta.display().to_string().trim_end_matches('/')))
        } else {
            archivo.with_extension("dmn")
        }
    });
    fs::write(&salida, &traduccion.codigo)
        .map_err(|error| (format!("Error: no se pudo escribir '{}': {}", salida.display(), error), 1))?;
    // Se deja con el estilo único de Deimon (si tiene errores de sintaxis, se queda como está).
    let _ = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon")))
        .arg("formatear")
        .arg(&salida)
        .output();

    println!(
        "✓ Traducido: {} → {}{}",
        archivo.display(),
        salida.display(),
        if es_web { format!(" (sitio web: {} páginas)", paginas.len()) } else { String::new() }
    );
    if traduccion.pendientes.is_empty() {
        println!("  Todo el PHP se pudo traducir.");
    } else {
        println!("! {} partes no se pudieron traducir (están anotadas al principio del archivo):", traduccion.pendientes.len());
        for (linea, motivo) in &traduccion.pendientes {
            if *linea > 0 {
                println!("  - línea {}: {}", linea, motivo);
            } else {
                println!("  - {}", motivo);
            }
        }
    }
    for (linea, aviso) in &traduccion.avisos {
        println!("  aviso, línea {}: {}", linea, aviso);
    }

    if comprobar {
        if es_web {
            let carpeta = if archivo.is_dir() {
                archivo.clone()
            } else {
                archivo.parent().map(|p| if p.as_os_str().is_empty() { PathBuf::from(".") } else { p.to_path_buf() }).unwrap_or_else(|| PathBuf::from("."))
            };
            let mut todas: Vec<Visita> = paginas.iter().map(|(camino, _)| (camino.clone(), None)).collect();
            todas.extend(visitas);
            return comparar_web(&carpeta, &salida, &todas);
        }
        return comparar(&archivo, &salida);
    }
    Ok(())
}

/// Lee un archivo .php o todos los .php de una carpeta como (camino web, código).
fn paginas_php(ruta: &Path) -> Result<Vec<(String, String)>, Fallo> {
    let leer = |archivo: &Path| {
        fs::read_to_string(archivo)
            .map_err(|error| (format!("Error: no se pudo leer '{}': {}", archivo.display(), error), 1))
    };
    if ruta.is_dir() {
        let mut archivos: Vec<PathBuf> = fs::read_dir(ruta)
            .map_err(|error| (format!("Error: no se pudo leer la carpeta '{}': {}", ruta.display(), error), 1))?
            .filter_map(|entrada| entrada.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|e| e == "php").unwrap_or(false))
            .collect();
        archivos.sort();
        if archivos.is_empty() {
            return Err((format!("Error: la carpeta '{}' no tiene archivos .php", ruta.display()), 1));
        }
        archivos
            .iter()
            .map(|archivo| Ok((format!("/{}", archivo.file_name().unwrap().to_string_lossy()), leer(archivo)?)))
            .collect()
    } else {
        let nombre = ruta.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        Ok(vec![(format!("/{}", nombre), leer(ruta)?)])
    }
}

fn parece_web(php: &str) -> bool {
    ["$_GET", "$_POST", "$_REQUEST", "$_SERVER", "header(", "<html", "<!DOCTYPE", "<form"]
        .iter()
        .any(|marca| php.contains(marca))
}

fn puerto_libre() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(8765)
}

/// Una petición HTTP sencilla que guarda las galletas como un navegador:
/// devuelve (código, Location si redirige, cuerpo).
fn pedir(puerto: u16, camino: &str, datos: Option<&str>, galletas: &mut Vec<(String, String)>) -> Option<(u16, Option<String>, String)> {
    use std::io::{Read, Write};
    let mut conexion = std::net::TcpStream::connect(("127.0.0.1", puerto)).ok()?;
    conexion.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok()?;
    let galleta = if galletas.is_empty() {
        String::new()
    } else {
        format!("Cookie: {}\r\n", galletas.iter().map(|(n, v)| format!("{}={}", n, v)).collect::<Vec<_>>().join("; "))
    };
    let peticion = match datos {
        Some(datos) => format!(
            "POST {} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
            camino,
            datos.len(),
            galleta,
            datos
        ),
        None => format!("GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\n{}Connection: close\r\n\r\n", camino, galleta),
    };
    conexion.write_all(peticion.as_bytes()).ok()?;
    let mut bytes = Vec::new();
    let _ = conexion.read_to_end(&mut bytes);
    let respuesta = String::from_utf8_lossy(&bytes).to_string();
    let (cabeza, cuerpo) = respuesta.split_once("\r\n\r\n")?;
    let codigo = cabeza.split_whitespace().nth(1)?.parse().ok()?;
    for linea in cabeza.lines() {
        if let Some((clave, valor)) = linea.split_once(':') {
            if clave.eq_ignore_ascii_case("set-cookie") {
                if let Some((nombre, contenido)) = valor.trim().split(';').next().and_then(|p| p.split_once('=')) {
                    galletas.retain(|(n, _)| n != nombre);
                    let caducada = valor.to_ascii_lowercase().contains("max-age=0") || valor.to_ascii_lowercase().contains("expires=thu, 01 jan 1970");
                    if !contenido.is_empty() && !caducada {
                        galletas.push((nombre.to_string(), contenido.to_string()));
                    }
                }
            }
        }
    }
    let destino = cabeza.lines().find_map(|l| {
        let (clave, valor) = l.split_once(':')?;
        clave.eq_ignore_ascii_case("location").then(|| valor.trim().to_string())
    });
    let cuerpo = if cabeza.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        desfragmentar(cuerpo)
    } else {
        cuerpo.to_string()
    };
    Some((codigo, destino, cuerpo))
}

fn desfragmentar(texto: &str) -> String {
    let mut salida = String::new();
    let mut resto = texto;
    while let Some((tamano, despues)) = resto.split_once("\r\n") {
        let tamano = usize::from_str_radix(tamano.trim(), 16).unwrap_or(0);
        if tamano == 0 || despues.len() < tamano {
            break;
        }
        salida.push_str(&despues[..tamano]);
        resto = despues[tamano..].trim_start_matches("\r\n");
    }
    salida
}

/// Espera a que el servidor abra su puerto. El sitio traducido se compila antes de
/// arrancar (más lento si el ordenador está ocupado), así que se espera hasta un minuto,
/// pero se deja de esperar en cuanto el programa termina (por ejemplo, por un error).
fn esperar(puerto: u16, programa: &mut std::process::Child) -> bool {
    for _ in 0..1200 {
        if std::net::TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            return true;
        }
        if matches!(programa.try_wait(), Ok(Some(_))) {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

/// Arranca el sitio PHP (php -S) y el traducido, visita las mismas páginas en los dos y
/// compara el código, la redirección y el HTML de cada respuesta.
fn comparar_web(carpeta: &Path, dmn: &Path, visitas: &[Visita]) -> Result<(), Fallo> {
    let puerto_php = puerto_libre();
    let mut php = Command::new("php")
        .args(["-S", &format!("127.0.0.1:{}", puerto_php), "-t"])
        .arg(carpeta)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| ("Error: para comprobar hace falta PHP instalado (orden 'php').".to_string(), 1))?;
    let datos = std::env::temp_dir().join(format!("deimon-comprobar-{}", std::process::id()));
    let puerto_deimon = puerto_libre();
    let mut deimon = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon")))
        .arg("ejecutar")
        .arg(dmn)
        .env("DEIMON_PUERTO", puerto_deimon.to_string())
        .env("DEIMON_DATOS", &datos)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| (format!("Error: no se pudo arrancar el sitio traducido: {}", error), 1))?;
    let listo = esperar(puerto_php, &mut php) && esperar(puerto_deimon, &mut deimon);
    let mut resultado = Ok(());
    if !listo {
        resultado = Err(("✗ No arrancó alguno de los dos servidores (¿el Deimon traducido tiene errores? pruébalo con deimon ejecutar).".to_string(), 1));
    } else {
        let mut iguales = 0;
        let (mut galletas_php, mut galletas_deimon) = (Vec::new(), Vec::new());
        for (camino, datos) in visitas {
            let a = pedir(puerto_php, camino, datos.as_deref(), &mut galletas_php);
            let b = pedir(puerto_deimon, camino, datos.as_deref(), &mut galletas_deimon);
            let descripcion = match datos {
                Some(d) => format!("enviar {} ({})", camino, d),
                None => format!("visitar {}", camino),
            };
            let normalizar = |r: Option<(u16, Option<String>, String)>| {
                r.map(|(codigo, destino, cuerpo)| match destino {
                    // PHP redirige con 302 y Deimon con 303: los dos llevan al mismo sitio.
                    Some(destino) if (300..400).contains(&codigo) => {
                        let destino = if destino.starts_with('/') || destino.starts_with("http") { destino } else { format!("/{}", destino) };
                        format!("redirige a {}", destino)
                    }
                    _ => format!("{}\n{}", codigo, cuerpo),
                })
            };
            let (a, b) = (normalizar(a), normalizar(b));
            if a == b && a.is_some() {
                iguales += 1;
                println!("  ✓ {}", descripcion);
            } else {
                println!("  ✗ {}", descripcion);
                println!("    PHP:    {}", a.unwrap_or_else(|| "(sin respuesta)".to_string()).replace('\n', " ⏎ ").chars().take(160).collect::<String>());
                println!("    Deimon: {}", b.unwrap_or_else(|| "(sin respuesta)".to_string()).replace('\n', " ⏎ ").chars().take(160).collect::<String>());
            }
        }
        if iguales == visitas.len() {
            println!("✓ Comprobado: las {} páginas responden exactamente igual en PHP y en Deimon.", iguales);
        } else {
            resultado = Err((format!("✗ {} de {} páginas responden distinto.", visitas.len() - iguales, visitas.len()), 1));
        }
    }
    let _ = php.kill();
    let _ = php.wait();
    let _ = deimon.kill();
    let _ = deimon.wait();
    let _ = fs::remove_dir_all(&datos);
    resultado
}

/// Ejecuta el PHP original y el Deimon traducido y compara lo que escriben.
fn comparar(php: &Path, dmn: &Path) -> Result<(), Fallo> {
    let original = Command::new("php")
        .arg(php)
        .output()
        .map_err(|_| ("Error: para comprobar hace falta PHP instalado (orden 'php').".to_string(), 1))?;
    let traducido = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon")))
        .arg("ejecutar")
        .arg(dmn)
        .output()
        .map_err(|error| (format!("Error: no se pudo ejecutar el programa traducido: {}", error), 1))?;
    let de_php = String::from_utf8_lossy(&original.stdout);
    let de_deimon = String::from_utf8_lossy(&traducido.stdout);
    if !traducido.status.success() {
        return Err((
            format!(
                "✗ El programa traducido no funciona todavía:\n{}",
                String::from_utf8_lossy(&traducido.stderr).trim()
            ),
            1,
        ));
    }
    if de_php == de_deimon {
        println!("✓ Comprobado: PHP y Deimon escriben exactamente lo mismo ({} líneas).", de_php.lines().count());
        return Ok(());
    }
    let mut mensaje = String::from("✗ La salida es distinta. Primera diferencia:\n");
    let lineas_php: Vec<&str> = de_php.lines().collect();
    let lineas_deimon: Vec<&str> = de_deimon.lines().collect();
    for i in 0..lineas_php.len().max(lineas_deimon.len()) {
        let a = lineas_php.get(i).copied().unwrap_or("(nada)");
        let b = lineas_deimon.get(i).copied().unwrap_or("(nada)");
        if a != b {
            mensaje.push_str(&format!("  línea {} de la salida\n  PHP:    {}\n  Deimon: {}", i + 1, a, b));
            break;
        }
    }
    if lineas_php.len() == lineas_deimon.len() && mensaje.ends_with(":\n") {
        mensaje.push_str("  (solo cambia el salto de línea final)");
    }
    Err((mensaje, 1))
}
