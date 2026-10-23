use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::android;
use crate::destinos;
use crate::utilidades::{
    carpeta_escribible, ejecutar_con_limite, json_texto, puerto_libre, siguiente_puerto_libre,
};

type Fallo = (String, i32);

#[derive(Clone, Copy, PartialEq)]
enum Estado {
    Ok,
    Error,
    Aviso,
}

struct Comprobacion {
    nombre: String,
    estado: Estado,
    detalle: String,
}

fn comprobacion(nombre: &str, estado: Estado, detalle: impl Into<String>) -> Comprobacion {
    Comprobacion {
        nombre: nombre.to_string(),
        estado,
        detalle: detalle.into(),
    }
}

/// `deimon entorno [archivo.dmn] [--json]`
pub fn entorno(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon entorno [archivo.dmn] [--json]";
    let mut archivo = None;
    let mut json = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
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
    }

    let mut lista = Vec::new();
    comprobar_rustc(&mut lista);
    comprobar_recursos(&mut lista);
    comprobar_destinos(&mut lista);
    comprobar_destino_android(&mut lista);
    comprobar_android(&mut lista);
    comprobar_adb(&mut lista);
    comprobar_datos(&mut lista);
    comprobar_temporal(&mut lista);
    if let Some(archivo) = &archivo {
        comprobar_archivo(archivo, &mut lista);
    }

    let listo = !lista.iter().any(|c| c.estado == Estado::Error);
    if json {
        let partes: Vec<String> = lista
            .iter()
            .map(|c| {
                format!(
                    "{{\"nombre\":{},\"estado\":{},\"detalle\":{}}}",
                    json_texto(&c.nombre),
                    json_texto(match c.estado {
                        Estado::Ok => "ok",
                        Estado::Error => "error",
                        Estado::Aviso => "aviso",
                    }),
                    json_texto(&c.detalle)
                )
            })
            .collect();
        println!(
            "{{\"comprobaciones\":[{}],\"listo\":{}}}",
            partes.join(","),
            listo
        );
    } else {
        println!("Entorno de Deimon");
        for c in &lista {
            let marca = match c.estado {
                Estado::Ok => "✓",
                Estado::Error => "✗",
                Estado::Aviso => "!",
            };
            println!("{} {}: {}", marca, c.nombre, c.detalle);
        }
        println!();
        if listo {
            println!("Todo lo imprescindible está listo. Los avisos (!) no impiden trabajar.");
        } else {
            println!("Falta algo imprescindible (líneas con ✗). Arréglalo y vuelve a comprobar.");
        }
    }
    if listo {
        Ok(())
    } else {
        Err((String::new(), 1))
    }
}

fn comprobar_rustc(lista: &mut Vec<Comprobacion>) {
    let salida = ejecutar_con_limite(Command::new("rustc").arg("--version"), Duration::from_secs(10));
    match salida {
        Ok(s) if s.exito => lista.push(comprobacion("rustc", Estado::Ok, s.stdout.trim())),
        Ok(s) => lista.push(comprobacion(
            "rustc",
            Estado::Error,
            format!("'rustc --version' falló: {}", s.stderr.trim()),
        )),
        Err(error) => lista.push(comprobacion(
            "rustc",
            Estado::Error,
            format!(
                "no se encontró 'rustc' ({}). Instálalo desde https://rustup.rs",
                error
            ),
        )),
    }
}

fn comprobar_recursos(lista: &mut Vec<Comprobacion>) {
    let nucleos = deimon::perfil::nucleos();
    let memoria = deimon::perfil::memoria_total_bytes();
    let recomendado = deimon::perfil::recomendado(nucleos, memoria);
    let memoria_texto = match memoria {
        Some(bytes) => format!(
            "{:.1} GB de memoria",
            bytes as f64 / (1024.0 * 1024.0 * 1024.0)
        ),
        None => "memoria desconocida".to_string(),
    };
    lista.push(comprobacion(
        "recursos",
        Estado::Ok,
        format!(
            "{} núcleos · {} · perfil recomendado: {} (deimon compilar --perfil {})",
            nucleos,
            memoria_texto,
            recomendado.nombre(),
            recomendado.nombre()
        ),
    ));
}

fn comprobar_destinos(lista: &mut Vec<Comprobacion>) {
    let (disponibles, faltan) = destinos::resumen();
    if disponibles.is_empty() {
        lista.push(comprobacion(
            "destinos de compilación",
            Estado::Aviso,
            "no hay ningún destino disponible; mira 'deimon destinos'",
        ));
        return;
    }
    let detalle = if faltan.is_empty() {
        format!(
            "{} disponibles: {}",
            disponibles.len(),
            disponibles.join(", ")
        )
    } else {
        format!(
            "{} disponibles: {}; no disponibles: {} (mira 'deimon destinos')",
            disponibles.len(),
            disponibles.join(", "),
            faltan.join(", ")
        )
    };
    lista.push(comprobacion(
        "destinos de compilación",
        if faltan.is_empty() {
            Estado::Ok
        } else {
            Estado::Aviso
        },
        detalle,
    ));
}

fn comprobar_destino_android(lista: &mut Vec<Comprobacion>) {
    let nombre = "destino aarch64-linux-android";
    let salida = ejecutar_con_limite(
        Command::new("rustup").args(["target", "list", "--installed"]),
        Duration::from_secs(10),
    );
    match salida {
        Ok(s) if s.exito => {
            if s.stdout.lines().any(|l| l.trim() == "aarch64-linux-android") {
                lista.push(comprobacion(nombre, Estado::Ok, "instalado"));
            } else {
                lista.push(comprobacion(
                    nombre,
                    Estado::Aviso,
                    "no está instalado (solo hace falta para 'deimon app'). Instálalo con: rustup target add aarch64-linux-android",
                ));
            }
        }
        Ok(_) | Err(_) => lista.push(comprobacion(
            nombre,
            Estado::Aviso,
            "no se pudo consultar con 'rustup' (¿no está instalado?); solo hace falta para 'deimon app'",
        )),
    }
}

fn comprobar_android(lista: &mut Vec<Comprobacion>) {
    let sdk = match android::carpeta_sdk() {
        Ok(sdk) => sdk,
        Err(_) => {
            lista.push(comprobacion(
                "SDK de Android",
                Estado::Aviso,
                "no se encontró (solo hace falta para 'deimon app'). Indica su carpeta con ANDROID_HOME o instálalo en ~/Android/Sdk",
            ));
            for nombre in ["build-tools", "plataforma de Android", "NDK"] {
                lista.push(comprobacion(
                    nombre,
                    Estado::Aviso,
                    "no se puede comprobar sin el SDK de Android",
                ));
            }
            return;
        }
    };
    lista.push(comprobacion(
        "SDK de Android",
        Estado::Ok,
        sdk.display().to_string(),
    ));
    match android::ultimas_build_tools(&sdk) {
        Some(version) => lista.push(comprobacion("build-tools", Estado::Ok, version)),
        None => lista.push(comprobacion(
            "build-tools",
            Estado::Aviso,
            "no hay 'build-tools' en el SDK; instálalas con el gestor del SDK",
        )),
    }
    match android::ultima_plataforma(&sdk) {
        Some(nivel) => lista.push(comprobacion(
            "plataforma de Android",
            Estado::Ok,
            format!("android-{}", nivel),
        )),
        None => lista.push(comprobacion(
            "plataforma de Android",
            Estado::Aviso,
            "no hay una plataforma de Android 35 o superior con android.jar",
        )),
    }
    match android::ultimo_ndk(&sdk) {
        Some(version) => lista.push(comprobacion("NDK", Estado::Ok, version)),
        None => lista.push(comprobacion(
            "NDK",
            Estado::Aviso,
            "no hay NDK en el SDK; instálalo con el gestor del SDK",
        )),
    }
}

fn comprobar_adb(lista: &mut Vec<Comprobacion>) {
    let mut programa: Option<PathBuf> = None;
    if android::existe_en_path("adb") {
        programa = Some(PathBuf::from("adb"));
    } else if let Ok(sdk) = android::carpeta_sdk() {
        let ruta = sdk.join("platform-tools").join("adb");
        if ruta.is_file() {
            programa = Some(ruta);
        }
    }
    let Some(programa) = programa else {
        lista.push(comprobacion(
            "adb",
            Estado::Aviso,
            "no se encontró (solo hace falta para instalar apps en un móvil)",
        ));
        lista.push(comprobacion(
            "móviles conectados",
            Estado::Aviso,
            "no se puede comprobar sin adb",
        ));
        return;
    };
    lista.push(comprobacion("adb", Estado::Ok, programa.display().to_string()));
    match ejecutar_con_limite(Command::new(&programa).arg("devices"), Duration::from_secs(5)) {
        Ok(s) if s.exito => {
            let dispositivos: Vec<&str> = s
                .stdout
                .lines()
                .skip_while(|l| !l.starts_with("List of devices"))
                .skip(1)
                .filter(|l| !l.trim().is_empty() && !l.starts_with('*'))
                .collect();
            if dispositivos.is_empty() {
                lista.push(comprobacion(
                    "móviles conectados",
                    Estado::Aviso,
                    "ninguno (conecta un móvil con la depuración USB activada)",
                ));
            } else {
                let listos = dispositivos.iter().filter(|l| l.ends_with("device")).count();
                lista.push(comprobacion(
                    "móviles conectados",
                    Estado::Ok,
                    format!("{} ({} listos para usar)", dispositivos.len(), listos),
                ));
            }
        }
        Ok(s) => lista.push(comprobacion(
            "móviles conectados",
            Estado::Aviso,
            format!("'adb devices' falló: {}", s.stderr.trim()),
        )),
        Err(error) => lista.push(comprobacion(
            "móviles conectados",
            Estado::Aviso,
            format!("no se pudo consultar a adb: {}", error),
        )),
    }
}

fn comprobar_datos(lista: &mut Vec<Comprobacion>) {
    let carpeta = match env::var("DEIMON_DATOS") {
        Ok(c) if !c.is_empty() => PathBuf::from(c),
        _ => PathBuf::from("./datos"),
    };
    let nombre = "carpeta de datos";
    if carpeta.is_dir() {
        if carpeta_escribible(&carpeta) {
            lista.push(comprobacion(
                nombre,
                Estado::Ok,
                format!("{} (existe y se puede escribir)", carpeta.display()),
            ));
        } else {
            lista.push(comprobacion(
                nombre,
                Estado::Aviso,
                format!("{} existe pero no se puede escribir en ella", carpeta.display()),
            ));
        }
    } else if carpeta.exists() {
        lista.push(comprobacion(
            nombre,
            Estado::Aviso,
            format!("{} existe pero no es una carpeta", carpeta.display()),
        ));
    } else {
        let padre = match carpeta.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        if carpeta_escribible(&padre) {
            lista.push(comprobacion(
                nombre,
                Estado::Ok,
                format!("{} no existe todavía; se creará al guardar el primer dato", carpeta.display()),
            ));
        } else {
            lista.push(comprobacion(
                nombre,
                Estado::Aviso,
                format!(
                    "{} no existe y no se puede crear en {}",
                    carpeta.display(),
                    padre.display()
                ),
            ));
        }
    }
}

fn comprobar_temporal(lista: &mut Vec<Comprobacion>) {
    let carpeta = env::temp_dir();
    if carpeta_escribible(&carpeta) {
        lista.push(comprobacion(
            "carpeta temporal",
            Estado::Ok,
            format!("{} (se puede escribir)", carpeta.display()),
        ));
    } else {
        lista.push(comprobacion(
            "carpeta temporal",
            Estado::Error,
            format!("no se puede escribir en {}", carpeta.display()),
        ));
    }
}

fn entre_comillas_si_hace_falta(archivo: &str) -> String {
    if archivo.chars().any(|c| c.is_whitespace()) {
        format!("\"{}\"", archivo)
    } else {
        archivo.to_string()
    }
}

fn comprobar_archivo(archivo: &str, lista: &mut Vec<Comprobacion>) {
    let nombre_traduccion = format!("traducción de {}", archivo);
    let fuente = match fs::read_to_string(Path::new(archivo)) {
        Ok(f) => f,
        Err(error) => {
            lista.push(comprobacion(
                &nombre_traduccion,
                Estado::Error,
                format!("no se pudo leer el archivo: {}", error),
            ));
            return;
        }
    };
    if let Err(error) = deimon::traducir(&fuente) {
        lista.push(comprobacion(&nombre_traduccion, Estado::Error, error.primera_linea()));
        return;
    }
    lista.push(comprobacion(&nombre_traduccion, Estado::Ok, "el programa se traduce bien"));
    let informacion = match deimon::informacion_archivo(&fuente, Path::new(archivo)) {
        Ok(i) => i,
        Err(error) => {
            lista.push(comprobacion("pruebas", Estado::Error, error.to_string()));
            return;
        }
    };

    if informacion.pruebas.is_empty() {
        lista.push(comprobacion(
            "pruebas",
            Estado::Aviso,
            "este programa no tiene pruebas",
        ));
    } else {
        lista.push(comprobacion(
            "pruebas",
            Estado::Ok,
            format!("{} (se ejecutan con: deimon probar)", informacion.pruebas.len()),
        ));
    }

    match informacion.servir {
        deimon::UsoServir::Ninguno => {}
        deimon::UsoServir::Calculado => lista.push(comprobacion(
            "puerto",
            Estado::Aviso,
            "el puerto de 'servir' se decide al ejecutar y no se puede comprobar antes",
        )),
        deimon::UsoServir::Puerto(escrito) => {
            let puerto = match env::var("DEIMON_PUERTO") {
                Ok(texto) => match texto.trim().parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        lista.push(comprobacion(
                            "puerto",
                            Estado::Error,
                            format!("la variable DEIMON_PUERTO no es un puerto válido: '{}'", texto),
                        ));
                        return;
                    }
                },
                Err(_) => escrito,
            };
            if puerto_libre(puerto) {
                lista.push(comprobacion(
                    "puerto",
                    Estado::Ok,
                    format!("la puerta {} está libre", puerto),
                ));
            } else {
                let detalle = match siguiente_puerto_libre(puerto) {
                    Some(libre) => format!(
                        "la puerta {} está ocupada por otro programa. Prueba con: DEIMON_PUERTO={} deimon ejecutar {}",
                        puerto,
                        libre,
                        entre_comillas_si_hace_falta(archivo)
                    ),
                    None => format!(
                        "la puerta {} está ocupada por otro programa y no se encontró una libre cerca",
                        puerto
                    ),
                };
                lista.push(comprobacion("puerto", Estado::Error, detalle));
            }
        }
    }
}
