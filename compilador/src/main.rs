mod android;
mod compartir_modulos;
mod comprobar;
mod destinos;
mod empaquetar;
mod entorno;
mod jugar;
mod mcp;
mod listar_modulos;
mod traducir_cobol;
mod traducir_php;
mod publicar;
mod probar;
mod utilidades;

use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

/// Emblema de Deimon (el demonio con su terminal) y el lema, en morado si es una terminal.
fn emblema() -> String {
    use std::io::IsTerminal;
    let (morado, blanco, fin) = if std::io::stdout().is_terminal() {
        ("\x1b[38;5;135m", "\x1b[1;97m", "\x1b[0m")
    } else {
        ("", "", "")
    };
    format!(
        "{m}   ▲           ▲{f}\n\
         {m}  ▐█▙▄▄▄▄▄▄▄▄▄▟█▌{f}\n\
         {m}  ▐▌ {b}◥▆▆◤ ◥▆▆◤{m} ▐▌{f}    {b}deimon {v}{f}\n\
         {m}  ▐▌ ┌───────┐ ▐▌{f}    {m}Código simple. Ideas grandes.{f}\n\
         {m}   ▀ │ {b}>_{m}    │ ▀{f}\n\
         {m}     └───────┘{f}\n",
        m = morado,
        b = blanco,
        f = fin,
        v = env!("CARGO_PKG_VERSION")
    )
}

fn main() {
    // Un programa empaquetado (`deimon empaquetar`) se ejecuta directamente.
    empaquetar::ejecutar_si_es_paquete();
    let argumentos: Vec<String> = env::args().collect();
    if matches!(
        argumentos.get(1).map(String::as_str),
        Some("--version") | Some("version") | Some("ayuda") | Some("--ayuda") | Some("--help")
    ) {
        print!("{}", emblema());
        println!("\nEmpieza con: deimon ejecutar hola.dmn · todas las órdenes: deimon guia");
        return;
    }
    if argumentos.get(1).map(String::as_str) == Some(probar::ORDEN_INTERNA) {
        if let Err((mensaje, codigo)) = probar::probar_con_interprete(&argumentos[2..]) {
            eprintln!("{}", mensaje);
            process::exit(codigo);
        }
        return;
    }
    if argumentos.get(1).map(String::as_str) == Some("app") {
        if let Err((mensaje, codigo)) = android::app(&argumentos[2..]) {
            eprintln!("{}", mensaje);
            process::exit(codigo);
        }
        return;
    }
    let subordenes: [(&str, fn(&[String]) -> Result<(), (String, i32)>); 16] = [
        ("entorno", entorno::entorno),
        ("probar", probar::probar),
        ("comprobar", comprobar::comprobar),
        ("formatear", deimon::formato::comando),
        ("guia", comprobar::guia),
        ("destinos", destinos::listar),
        ("modulos", listar_modulos::listar),
        ("modulo", compartir_modulos::modulo),
        ("desde-php", traducir_php::desde_php),
        ("desde-cobol", traducir_cobol::desde_cobol),
        ("publicar", publicar::publicar),
        ("compilar", destinos::compilar),
        ("arrancar", destinos::arrancar),
        ("jugar", jugar::jugar),
        ("empaquetar", empaquetar::empaquetar),
        ("mcp", mcp::mcp),
    ];
    for (nombre, orden) in subordenes {
        if argumentos.get(1).map(String::as_str) == Some(nombre) {
            if let Err((mensaje, codigo)) = orden(&argumentos[2..]) {
                if !mensaje.is_empty() {
                    eprintln!("{}", mensaje);
                }
                process::exit(codigo);
            }
            return;
        }
    }
    let resultado = match argumentos.get(1).map(String::as_str) {
        Some("traducir") => {
            if argumentos.len() != 3 {
                eprintln!("Uso: deimon traducir archivo.dmn");
                process::exit(2);
            }
            traducir(&argumentos[2])
        }
        Some("ejecutar") => ejecutar(&argumentos[2..]),
        otra => {
            if otra.is_none() {
                eprint!("{}", emblema());
            }
            if let Some(otra) = otra {
                eprintln!(
                    "Error: la orden '{}' no existe. Usa 'traducir', 'ejecutar', 'comprobar', 'formatear', 'probar', 'compilar', 'arrancar', 'destinos', 'modulos', 'modulo', 'desde-php', 'desde-cobol', 'publicar', 'jugar', 'empaquetar', 'mcp', 'entorno', 'guia' o 'app'.",
                    otra
                );
            }
            eprintln!("Uso: deimon <traducir|ejecutar> archivo.dmn [--perfil NOMBRE] | deimon app archivo.dmn [--nombre N] [--paquete P] [--perfil NOMBRE] | deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil NOMBRE] | deimon arrancar archivo.dmn [--sin-ventana] [--memoria MB] | deimon probar archivo.dmn [--json] | deimon comprobar archivo.dmn [--json] [--arreglar] | deimon formatear archivo.dmn [--comprobar] | deimon desde-cobol archivo.cob [--salida x.dmn] [--comprobar] | deimon destinos [--json] | deimon modulos [archivo.dmn] [--json] | deimon modulo <instalar|actualizar|quitar> <origen|nombre> [--aqui] | deimon entorno [archivo.dmn] [--json] | deimon guia [archivo.dmn] [--json] | deimon jugar [--puerto N] [--sin-abrir] | deimon empaquetar archivo.dmn [--salida RUTA] [--perfil NOMBRE] | deimon mcp");
            process::exit(2);
        }
    };

    if let Err((mensaje, codigo)) = resultado {
        eprintln!("{}", mensaje);
        process::exit(codigo);
    }
}

fn traducir(archivo: &str) -> Result<(), (String, i32)> {
    let fuente = leer_archivo(archivo)?;
    let rust = deimon::traducir_archivo_varios(&fuente, Path::new(archivo))
        .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;
    print!("{}", rust);
    Ok(())
}

fn ejecutar(argumentos: &[String]) -> Result<(), (String, i32)> {
    let uso = "Uso: deimon ejecutar archivo.dmn [--perfil pequeño|equilibrado|servidor]";
    let mut archivo = None;
    let mut perfil = deimon::perfil::Perfil::Equilibrado;
    let mut i = 0;
    while i < argumentos.len() {
        let actual = argumentos[i].as_str();
        match actual {
            "--perfil" => {
                i += 1;
                let valor = argumentos
                    .get(i)
                    .ok_or_else(|| (format!("Error: falta el valor de '--perfil'.\n{}", uso), 2))?;
                perfil = deimon::perfil::Perfil::desde_texto(valor)
                    .ok_or_else(|| deimon::perfil::error_desconocido(valor))?;
            }
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => {
                if archivo.is_some() {
                    return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2));
                }
                archivo = Some(otro.to_string());
            }
        }
        i += 1;
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;

    let fuente = leer_archivo(&archivo)?;
    let rust = deimon::traducir_archivo_varios_con_perfil(&fuente, Path::new(&archivo), perfil)
        .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;

    // Motor: el intérprete propio (instantáneo) cuando el programa solo usa lo que sabe hacer;
    // si no, el modo Rust de siempre. `DEIMON_MOTOR=rust|interprete` fuerza uno.
    if env::var("DEIMON_MOTOR").as_deref() != Ok("rust") {
        match deimon::compilar_para_interprete(&fuente, Path::new(&archivo)) {
            Ok(codigo) => {
                deimon::interprete::elegir_perfil(perfil);
                deimon::interprete::ejecutar(&codigo);
                return Ok(());
            }
            Err(motivo) => {
                if env::var_os("DEIMON_MOTOR_MOTIVO").is_some() {
                    eprintln!("(modo Rust: {})", motivo);
                }
            }
        }
    }

    let base = nombre_temporal();
    let ruta_rust = base.with_extension("rs");
    let ruta_ejecutable = base.with_extension("bin");
    let extensiones = deimon::extensiones_del_rust(&rust);

    fs::write(&ruta_rust, rust).map_err(|error| {
        (
            format!(
                "Error: no se pudo escribir el archivo temporal '{}': {}",
                ruta_rust.display(),
                error
            ),
            1,
        )
    })?;

    let mut orden = Command::new("rustc");
    for opcion in perfil.argumentos_rustc() {
        orden.arg(opcion);
    }
    let compilacion = orden
        .arg(&ruta_rust)
        .arg("-o")
        .arg(&ruta_ejecutable)
        .output();

    let compilacion = match compilacion {
        Ok(salida) => salida,
        Err(error) => {
            limpiar(&ruta_rust);
            limpiar(&ruta_ejecutable);
            return Err((
                format!("Error: no se pudo ejecutar 'rustc': {}", error),
                1,
            ));
        }
    };

    if !compilacion.status.success() {
        limpiar(&ruta_rust);
        limpiar(&ruta_ejecutable);
        return Err((mensaje_de_compilacion(&extensiones, &compilacion.stderr), 1));
    }

    limpiar(&ruta_rust);

    // Se reemplaza este proceso por el programa: así, al detener `deimon`, se detiene también
    // el programa (por ejemplo un servidor web) y no queda un puerto ocupado.
    let error = Command::new(&ruta_ejecutable)
        .env("DEIMON_BORRAR_EJECUTABLE", "1")
        .env("DEIMON_ARCHIVO", archivo)
        .exec();
    limpiar(&ruta_ejecutable);
    Err((
        format!("Error: no se pudo ejecutar el programa generado: {}", error),
        1,
    ))
}

/// Mensaje cuando el Rust generado no compila: si hay extensiones, lo dice antes del error.
fn mensaje_de_compilacion(extensiones: &[String], stderr: &[u8]) -> String {
    if extensiones.is_empty() {
        return "Error: no se pudo compilar el Rust generado".to_string();
    }
    let mut mensaje = String::new();
    for nombre in extensiones {
        mensaje.push_str(&format!(
            "El módulo '{}' tiene un error en su código Rust:\n",
            nombre
        ));
    }
    mensaje.push_str(String::from_utf8_lossy(stderr).trim_end());
    mensaje
}

fn leer_archivo(archivo: &str) -> Result<String, (String, i32)> {
    fs::read_to_string(archivo).map_err(|error| {
        (
            format!("Error: no se pudo leer el archivo '{}': {}", archivo, error),
            1,
        )
    })
}

fn nombre_temporal() -> PathBuf {
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duracion| duracion.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("ln_{}_{}", process::id(), marca))
}

fn limpiar(ruta: &PathBuf) {
    let _ = fs::remove_file(ruta);
}
