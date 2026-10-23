use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct SalidaProceso {
    pub exito: bool,
    pub codigo: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

static SECUENCIA: AtomicU64 = AtomicU64::new(0);

/// Ejecuta una orden y espera como mucho `limite`; si tarda más, la mata y devuelve un error.
/// La salida se guarda en archivos temporales (no en tuberías) para no quedarse nunca colgado.
pub fn ejecutar_con_limite(
    orden: &mut Command,
    limite: Duration,
) -> Result<SalidaProceso, String> {
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
    let base = env::temp_dir().join(format!("ln_proc_{}_{}_{}", std::process::id(), marca, numero));
    let ruta_salida = base.with_extension("out");
    let ruta_error = base.with_extension("err");
    let limpiar = || {
        let _ = fs::remove_file(&ruta_salida);
        let _ = fs::remove_file(&ruta_error);
    };
    let archivo_salida = fs::File::create(&ruta_salida)
        .map_err(|e| format!("no se pudo crear un archivo temporal: {}", e))?;
    let archivo_error = fs::File::create(&ruta_error)
        .map_err(|e| format!("no se pudo crear un archivo temporal: {}", e))?;
    let mut hijo = match orden
        .stdin(Stdio::null())
        .stdout(archivo_salida)
        .stderr(archivo_error)
        .spawn()
    {
        Ok(hijo) => hijo,
        Err(error) => {
            limpiar();
            return Err(error.to_string());
        }
    };
    let inicio = Instant::now();
    let estado = loop {
        match hijo.try_wait() {
            Ok(Some(estado)) => break estado,
            Ok(None) => {
                if inicio.elapsed() >= limite {
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    limpiar();
                    return Err(format!("tardó más de {} segundos y se detuvo", limite.as_secs()));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                let _ = hijo.kill();
                let _ = hijo.wait();
                limpiar();
                return Err(error.to_string());
            }
        }
    };
    let stdout = fs::read(&ruta_salida)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let stderr = fs::read(&ruta_error)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    limpiar();
    Ok(SalidaProceso {
        exito: estado.success(),
        codigo: estado.code(),
        stdout,
        stderr,
    })
}

/// Escapa un texto para ponerlo entre comillas en JSON.
pub fn json_texto(texto: &str) -> String {
    deimon::json::texto(texto)
}

pub fn puerto_libre(puerto: u16) -> bool {
    TcpListener::bind(("127.0.0.1", puerto)).is_ok()
}

/// Primer puerto libre después de `puerto` (busca hasta 200 puertos más arriba).
pub fn siguiente_puerto_libre(puerto: u16) -> Option<u16> {
    (puerto.saturating_add(1)..=puerto.saturating_add(200)).find(|p| puerto_libre(*p))
}

/// Comprueba si se puede crear (y borrar) un archivo dentro de la carpeta.
pub fn carpeta_escribible(carpeta: &Path) -> bool {
    let prueba = carpeta.join(format!(".ln_comprobar_{}", std::process::id()));
    match fs::write(&prueba, b"") {
        Ok(()) => {
            let _ = fs::remove_file(&prueba);
            true
        }
        Err(_) => false,
    }
}
