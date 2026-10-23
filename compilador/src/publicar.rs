//! `deimon publicar archivo.dmn --prueba | --dominio midominio.com`: pone HTTPS delante de
//! un programa web de Deimon con Caddy (que consigue y renueva solo los certificados).
//! Deimon no programa su propio cifrado: usar uno probado es lo seguro.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

type Fallo = (String, i32);

const USO: &str = "Uso: deimon publicar archivo.dmn --prueba [--puerto 8443]  |  deimon publicar archivo.dmn --dominio midominio.com [--salida carpeta]";

fn puerto_libre() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(8765)
}

fn hay_caddy() -> bool {
    Command::new("caddy").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn dominio_valido(dominio: &str) -> bool {
    !dominio.is_empty()
        && dominio.len() <= 253
        && dominio.contains('.')
        && dominio.split('.').all(|parte| {
            !parte.is_empty() && !parte.starts_with('-') && !parte.ends_with('-') && parte.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

pub fn publicar(argumentos: &[String]) -> Result<(), Fallo> {
    let mut archivo = None;
    let mut prueba = false;
    let mut dominio = None;
    let mut puerto_publico: u16 = 8443;
    let mut salida = None;
    let mut resto = argumentos.iter();
    while let Some(argumento) = resto.next() {
        match argumento.as_str() {
            "--prueba" => prueba = true,
            "--dominio" => dominio = resto.next().cloned(),
            "--salida" => salida = resto.next().map(PathBuf::from),
            "--puerto" => {
                puerto_publico = resto
                    .next()
                    .and_then(|p| p.parse().ok())
                    .ok_or_else(|| (format!("Error: --puerto necesita un número. {}", USO), 2))?
            }
            otro if otro.starts_with("--") => return Err((format!("Error: la opción '{}' no existe. {}", otro, USO), 2)),
            otro if archivo.is_none() => archivo = Some(PathBuf::from(otro)),
            otro => return Err((format!("Error: sobra el argumento '{}'. {}", otro, USO), 2)),
        }
    }
    let archivo = archivo.ok_or_else(|| (USO.to_string(), 2))?;
    if prueba == dominio.is_some() {
        return Err((format!("Error: elige --prueba (HTTPS en este ordenador) o --dominio (para publicarlo en internet). {}", USO), 2));
    }
    if !hay_caddy() {
        return Err((
            "Error: falta Caddy, el programa que pone el HTTPS. Instálalo con: sudo apt install caddy\n(Después puedes apagar su servicio automático con: sudo systemctl disable --now caddy)".to_string(),
            1,
        ));
    }
    let deimon = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("deimon"));
    let nombre = archivo.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "sitio".to_string());

    if let Some(dominio) = dominio {
        if !dominio_valido(&dominio) {
            return Err((format!("Error: '{}' no parece un dominio (por ejemplo: midominio.com)", dominio), 1));
        }
        let carpeta = salida.unwrap_or_else(|| PathBuf::from(format!("publicar-{}", nombre)));
        fs::create_dir_all(&carpeta).map_err(|e| (format!("Error: {}", e), 1))?;
        let ejecutable = carpeta.join(&nombre);
        let compilado = Command::new(&deimon)
            .arg("compilar")
            .arg(&archivo)
            .args(["--perfil", "servidor", "--salida"])
            .arg(&ejecutable)
            .status()
            .map_err(|e| (format!("Error: {}", e), 1))?;
        if !compilado.success() {
            return Err(("Error: el programa no se pudo compilar (mira el mensaje de arriba).".to_string(), 1));
        }
        fs::write(
            carpeta.join("Caddyfile"),
            format!(
                "# HTTPS automático para {d}: Caddy pide y renueva el certificado (Let's Encrypt).\n{d} {{\n    encode gzip\n    header Strict-Transport-Security \"max-age=31536000\"\n    reverse_proxy 127.0.0.1:8000\n}}\n",
                d = dominio
            ),
        )
        .map_err(|e| (format!("Error: {}", e), 1))?;
        let arranque = carpeta.join("arrancar.sh");
        fs::write(
            &arranque,
            format!(
                "#!/usr/bin/env bash\n# Arranca {n} detrás de Caddy con HTTPS para {d}.\nset -e\ncd \"$(dirname \"$0\")\"\nmkdir -p datos\nDEIMON_PUERTO=8000 DEIMON_DATOS=\"$PWD/datos\" ./{n} &\nprograma=$!\ntrap 'kill $programa' EXIT\nsudo caddy run --config Caddyfile\n",
                n = nombre,
                d = dominio
            ),
        )
        .map_err(|e| (format!("Error: {}", e), 1))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&arranque, fs::Permissions::from_mode(0o755));
        }
        println!("✓ Listo para publicar en https://{} : carpeta {}", dominio, carpeta.display());
        println!("  Pasos en el servidor de internet:");
        println!("  1. Apunta el dominio {} a la IP del servidor (registro A en tu proveedor de dominio).", dominio);
        println!("  2. Abre los puertos 80 y 443 del servidor.");
        println!("  3. Copia la carpeta al servidor y ejecuta ./arrancar.sh (instala antes caddy).");
        println!("  Caddy conseguirá el certificado HTTPS solo, y lo renovará cada mes.");
        return Ok(());
    }

    // --prueba: HTTPS en este ordenador con un certificado local de Caddy.
    let puerto_interno = puerto_libre();
    let datos = std::env::temp_dir().join(format!("deimon-publicar-{}", std::process::id()));
    let mut programa = Command::new(&deimon)
        .arg("ejecutar")
        .arg(&archivo)
        .env("DEIMON_PUERTO", puerto_interno.to_string())
        .env("DEIMON_DATOS", &datos)
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| (format!("Error: no se pudo arrancar el programa: {}", e), 1))?;
    let configuracion = datos.with_extension("Caddyfile");
    fs::write(
        &configuracion,
        format!(
            "{{\n    admin off\n    auto_https disable_redirects\n    storage file_system \"{}\"\n}}\nhttps://localhost:{} {{\n    tls internal\n    reverse_proxy 127.0.0.1:{}\n}}\n",
            datos.with_extension("caddy").display(),
            puerto_publico,
            puerto_interno
        ),
    )
    .map_err(|e| (format!("Error: {}", e), 1))?;
    let mut caddy = Command::new("caddy")
        .args(["run", "--adapter", "caddyfile", "--config"])
        .arg(&configuracion)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| (format!("Error: no se pudo arrancar Caddy: {}", e), 1))?;
    println!("✓ {} con HTTPS en https://localhost:{}", nombre, puerto_publico);
    println!("  (el certificado es de prueba: el navegador avisará la primera vez)");
    println!("  Pulsa Ctrl+C para pararlo.");
    let estado = programa.wait();
    let _ = caddy.kill();
    let _ = caddy.wait();
    let _ = fs::remove_file(&configuracion);
    let _ = fs::remove_dir_all(&datos);
    let _ = fs::remove_dir_all(datos.with_extension("caddy"));
    match estado {
        Ok(_) => Ok(()),
        Err(e) => Err((format!("Error: {}", e), 1)),
    }
}
