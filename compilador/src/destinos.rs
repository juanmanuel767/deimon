use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::android;
use crate::utilidades::json_texto;

type Fallo = (String, i32);

#[derive(Clone, Copy)]
pub struct Capacidades {
    pub red: bool,
    pub archivos: bool,
    pub hilos: bool,
}

#[derive(Clone, Copy)]
pub enum Enlazador {
    /// El que usa Rust para el ordenador donde se compila.
    PorDefecto,
    /// Un programa que tiene que estar en el PATH.
    Programa(&'static str),
    /// Un clang que viene con el NDK de Android.
    Ndk(&'static str),
}

pub struct Destino {
    pub nombre: &'static str,
    pub triple: &'static str,
    pub enlazador: Enlazador,
    pub capacidades: Capacidades,
    pub descripcion: &'static str,
    /// Órdenes exactas para instalarlo: primero el destino de Rust y luego el enlazador.
    pub instalar: &'static [&'static str],
}

const TODAS_LAS_CAPACIDADES: Capacidades = Capacidades {
    red: true,
    archivos: true,
    hilos: true,
};

/// La única tabla de destinos de compilación.
pub const DESTINOS: &[Destino] = &[
    Destino {
        nombre: "linux-x64",
        triple: "x86_64-unknown-linux-gnu",
        enlazador: Enlazador::PorDefecto,
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Linux de 64 bits para este ordenador (x86_64).",
        instalar: &["rustup target add x86_64-unknown-linux-gnu"],
    },
    Destino {
        nombre: "linux-x64-estatico",
        triple: "x86_64-unknown-linux-musl",
        enlazador: Enlazador::PorDefecto,
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Linux de 64 bits estático: funciona en cualquier Linux de 64 bits sin dependencias.",
        instalar: &["rustup target add x86_64-unknown-linux-musl"],
    },
    Destino {
        nombre: "android-arm64",
        triple: "aarch64-linux-android",
        enlazador: Enlazador::Ndk("aarch64-linux-android24-clang"),
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Android en teléfonos de 64 bits (arm64).",
        instalar: &[
            "rustup target add aarch64-linux-android",
            "sdkmanager \"ndk;27.1.12297006\"",
        ],
    },
    Destino {
        nombre: "android-arm32",
        triple: "armv7-linux-androideabi",
        enlazador: Enlazador::Ndk("armv7a-linux-androideabi24-clang"),
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Android en teléfonos antiguos de 32 bits (armv7).",
        instalar: &[
            "rustup target add armv7-linux-androideabi",
            "sdkmanager \"ndk;27.1.12297006\"",
        ],
    },
    Destino {
        nombre: "linux-arm64",
        triple: "aarch64-unknown-linux-gnu",
        enlazador: Enlazador::Programa("aarch64-linux-gnu-gcc"),
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Linux de 64 bits para procesadores ARM (por ejemplo Raspberry Pi).",
        instalar: &[
            "rustup target add aarch64-unknown-linux-gnu",
            "sudo apt install gcc-aarch64-linux-gnu",
        ],
    },
    Destino {
        nombre: "windows-x64",
        triple: "x86_64-pc-windows-gnu",
        enlazador: Enlazador::Programa("x86_64-w64-mingw32-gcc"),
        capacidades: TODAS_LAS_CAPACIDADES,
        descripcion: "Windows de 64 bits.",
        instalar: &[
            "rustup target add x86_64-pc-windows-gnu",
            "sudo apt install gcc-mingw-w64-x86-64",
        ],
    },
    Destino {
        nombre: SIN_SISTEMA,
        triple: "x86_64-unknown-none",
        enlazador: Enlazador::PorDefecto,
        capacidades: Capacidades {
            red: false,
            archivos: false,
            hilos: false,
        },
        descripcion: "PC de 64 bits sin sistema operativo: el programa ES el sistema. Crea una imagen .iso que arranca con GRUB (en QEMU, en una máquina virtual o grabada en un USB); usa la pantalla y el teclado de verdad.",
        instalar: &[
            "rustup target add x86_64-unknown-none",
            "sudo apt install grub-pc-bin xorriso mtools",
        ],
    },
];

/// Nombre del destino en el que el programa es el sistema operativo.
pub const SIN_SISTEMA: &str = "pc-sin-sistema";

/// Guion del enlazador para el núcleo: se carga en 1 MB, con la cabecera multiboot2 delante.
const ENLACE_SIN_SISTEMA: &str = "ENTRY(_start)
SECTIONS {
    . = 1M;
    .boot : { KEEP(*(.multiboot)) }
    .text : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    .bss : { *(.bss .bss.*) *(COMMON) }
}
";

/// Menú de GRUB de la imagen: arranca el programa sin esperar.
const GRUB_SIN_SISTEMA: &str = "set timeout=0
set default=0
menuentry \"Deimon\" {
    multiboot2 /boot/deimon.elf
    boot
}
";

struct Faltante {
    que: String,
    instalar: &'static str,
}

struct Disponibilidad {
    enlazador: Option<PathBuf>,
    faltantes: Vec<Faltante>,
}

impl Disponibilidad {
    fn disponible(&self) -> bool {
        self.faltantes.is_empty()
    }
}

pub fn buscar(nombre: &str) -> Option<&'static Destino> {
    DESTINOS.iter().find(|destino| destino.nombre == nombre)
}

/// `deimon destinos [--json]`
pub fn listar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon destinos [--json]";
    let mut json = false;
    for argumento in argumentos {
        match argumento.as_str() {
            "--json" => json = true,
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro => return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2)),
        }
    }

    if json {
        let elementos: Vec<String> = DESTINOS
            .iter()
            .map(|destino| {
                let estado = disponibilidad(destino);
                let falta: Vec<String> = estado
                    .faltantes
                    .iter()
                    .map(|faltante| json_texto(&faltante.que))
                    .collect();
                let instalar: Vec<String> = estado
                    .faltantes
                    .iter()
                    .map(|faltante| json_texto(faltante.instalar))
                    .collect();
                format!(
                    "{{\"nombre\":{},\"triple\":{},\"disponible\":{},\"red\":{},\"archivos\":{},\"hilos\":{},\"descripcion\":{},\"falta\":[{}],\"instalar\":[{}]}}",
                    json_texto(destino.nombre),
                    json_texto(destino.triple),
                    estado.disponible(),
                    destino.capacidades.red,
                    destino.capacidades.archivos,
                    destino.capacidades.hilos,
                    json_texto(destino.descripcion),
                    falta.join(","),
                    instalar.join(",")
                )
            })
            .collect();
        println!("{{\"destinos\":[{}]}}", elementos.join(","));
        return Ok(());
    }

    println!("Destinos para compilar programas Deimon");
    for destino in DESTINOS {
        let estado = disponibilidad(destino);
        let capacidades = format!(
            "red: {} · archivos: {} · hilos: {}",
            si_no(destino.capacidades.red),
            si_no(destino.capacidades.archivos),
            si_no(destino.capacidades.hilos)
        );
        if estado.disponible() {
            println!(
                "✓ {} ({}): {} {}",
                destino.nombre, destino.triple, destino.descripcion, capacidades
            );
        } else {
            println!(
                "✗ {} ({}): {} {}",
                destino.nombre, destino.triple, destino.descripcion, capacidades
            );
            for faltante in estado.faltantes {
                println!("  Falta: {}.", faltante.que);
                println!("  Instálalo con: {}", faltante.instalar);
            }
        }
    }
    Ok(())
}

fn si_no(valor: bool) -> &'static str {
    if valor {
        "sí"
    } else {
        "no"
    }
}

/// Nombres de los destinos disponibles y de los que faltan (para `deimon entorno`).
pub fn resumen() -> (Vec<&'static str>, Vec<&'static str>) {
    let mut disponibles = Vec::new();
    let mut faltan = Vec::new();
    for destino in DESTINOS {
        if disponibilidad(destino).disponible() {
            disponibles.push(destino.nombre);
        } else {
            faltan.push(destino.nombre);
        }
    }
    (disponibles, faltan)
}

/// `deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil NOMBRE]`
/// Capacidades que piden los módulos y que el destino no tiene.
pub fn capacidades_que_faltan(capacidades: &Capacidades, requiere: &[String]) -> Vec<String> {
    requiere
        .iter()
        .filter(|capacidad| match capacidad.as_str() {
            "red" => !capacidades.red,
            "archivos" => !capacidades.archivos,
            "hilos" => !capacidades.hilos,
            _ => false,
        })
        .cloned()
        .collect()
}

/// Falla, antes de compilar, si algún módulo usado necesita algo que el destino no tiene.
pub fn comprobar_capacidades(destino: &Destino, fuente: &str, ruta: &Path) -> Result<(), Fallo> {
    let modulos = deimon::extensiones_usadas_archivo(fuente, ruta)
        .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;
    for modulo in modulos {
        let faltan = capacidades_que_faltan(&destino.capacidades, &modulo.requiere);
        if !faltan.is_empty() {
            return Err((
                format!(
                    "Error: el módulo '{}' necesita {} y el destino '{}' no lo tiene. Elige otro destino (mira 'deimon destinos') o quita 'usar modulo \"{}\"'.",
                    modulo.nombre,
                    faltan.join(", "),
                    destino.nombre,
                    modulo.nombre
                ),
                1,
            ));
        }
    }
    Ok(())
}

pub fn compilar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon compilar archivo.dmn [--destino NOMBRE] [--salida RUTA] [--perfil pequeño|equilibrado|servidor]";
    let mut archivo = None;
    let mut elegido = None;
    let mut salida = None;
    let mut perfil = deimon::perfil::Perfil::Equilibrado;
    let mut i = 0;
    while i < argumentos.len() {
        let actual = argumentos[i].as_str();
        match actual {
            "--destino" | "--salida" | "--perfil" => {
                i += 1;
                let valor = argumentos
                    .get(i)
                    .cloned()
                    .ok_or_else(|| (format!("Error: falta el valor de '{}'.\n{}", actual, uso), 2))?;
                match actual {
                    "--destino" => elegido = Some(valor),
                    "--perfil" => {
                        perfil = deimon::perfil::Perfil::desde_texto(&valor)
                            .ok_or_else(|| deimon::perfil::error_desconocido(&valor))?
                    }
                    _ => salida = Some(valor),
                }
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

    let destino = match &elegido {
        Some(nombre) => buscar(nombre).ok_or_else(|| error_desconocido(nombre))?,
        None => destino_actual().ok_or_else(|| {
            (
                format!(
                    "Error: no sé qué destino corresponde a este ordenador ({}-{}). Usa --destino y mira 'deimon destinos'.",
                    env::consts::OS,
                    env::consts::ARCH
                ),
                2,
            )
        })?,
    };
    let estado = disponibilidad(destino);
    if !estado.disponible() {
        return Err((mensaje_no_disponible(destino, &estado), 1));
    }

    let ruta_fuente = PathBuf::from(&archivo);
    let fuente = fs::read_to_string(&ruta_fuente).map_err(|error| {
        (
            format!("Error: no se pudo leer el archivo '{}': {}", archivo, deimon::error::explicar_lectura(&error)),
            1,
        )
    })?;
    let sin_sistema = destino.nombre == SIN_SISTEMA;
    let rust = if sin_sistema {
        deimon::traducir_archivo_sin_sistema(&fuente, &ruta_fuente, perfil)
    } else {
        deimon::traducir_archivo_varios_con_perfil(&fuente, &ruta_fuente, perfil)
    }
    .map_err(|errores| (deimon::texto_de_errores(&errores), 1))?;
    comprobar_capacidades(destino, &fuente, &ruta_fuente)?;

    // Antes de compilar se ejecutan las pruebas del programa en el ordenador.
    let pruebas = crate::probar::correr_archivo(&fuente, &ruta_fuente)
        .map_err(|mensaje| (mensaje, 1))?;
    if pruebas.is_empty() {
        println!("! Aviso: este programa no tiene pruebas.");
    } else {
        crate::probar::imprimir(&pruebas);
        let falladas = pruebas.iter().filter(|p| !p.correcta).count();
        if falladas > 0 {
            return Err((
                format!(
                    "No se compila el programa porque {} {} fallando. Arregla lo que falla y vuelve a intentarlo.",
                    falladas,
                    if falladas == 1 { "prueba está" } else { "pruebas están" }
                ),
                1,
            ));
        }
    }

    let salida = match &salida {
        // En Windows el enlazador añade .exe si falta: se pone ya para informar bien.
        Some(ruta) if destino.triple.contains("windows") && !ruta.to_lowercase().ends_with(".exe") => {
            PathBuf::from(format!("{}.exe", ruta))
        }
        Some(ruta) => PathBuf::from(ruta),
        None => ruta_por_defecto(&ruta_fuente, destino),
    };
    if let Some(padre) = salida.parent() {
        if !padre.as_os_str().is_empty() {
            fs::create_dir_all(padre).map_err(|error| {
                (
                    format!("Error: no se pudo crear la carpeta '{}': {}", padre.display(), error),
                    1,
                )
            })?;
        }
    }

    let ruta_rust = archivo_temporal();
    fs::write(&ruta_rust, &rust).map_err(|error| {
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
    orden.arg("--target").arg(destino.triple);
    if let Some(enlazador) = &estado.enlazador {
        orden.arg("-C").arg(format!("linker={}", enlazador.display()));
    }
    let ruta_enlace = ruta_rust.with_extension("ld");
    let ruta_elf = ruta_rust.with_extension("elf");
    if sin_sistema {
        fs::write(&ruta_enlace, ENLACE_SIN_SISTEMA).map_err(|error| {
            (format!("Error: no se pudo escribir '{}': {}", ruta_enlace.display(), error), 1)
        })?;
        orden.args(["--edition", "2021", "-C", "panic=abort", "-C", "relocation-model=static"]);
        orden.arg("-C").arg(format!("link-arg=-T{}", ruta_enlace.display()));
        orden.args(["-C", "link-arg=-zmax-page-size=0x1000"]);
        orden.arg(&ruta_rust).arg("-o").arg(&ruta_elf);
    } else {
        orden.arg(&ruta_rust).arg("-o").arg(&salida);
    }

    let compilacion = orden.output();
    let _ = fs::remove_file(&ruta_rust);
    let _ = fs::remove_file(&ruta_enlace);
    match compilacion {
        Ok(salida_rustc) if salida_rustc.status.success() => {}
        Ok(salida_rustc) => {
            return Err((
                mensaje_de_compilacion(&rust, &salida_rustc.stderr),
                1,
            ))
        }
        Err(error) => {
            return Err((
                format!("Error: no se pudo ejecutar 'rustc': {}", error),
                1,
            ))
        }
    }

    if sin_sistema {
        let resultado = crear_imagen_arrancable(&ruta_elf, &salida);
        let _ = fs::remove_file(&ruta_elf);
        resultado?;
    }

    println!(
        "Programa compilado para {} ({}) con perfil {}: {}",
        destino.nombre,
        destino.triple,
        perfil.nombre(),
        salida.display()
    );
    match fs::metadata(&salida) {
        Ok(datos) => println!(
            "Tamaño del ejecutable: {:.1} KB",
            datos.len() as f64 / 1024.0
        ),
        Err(_) => println!("Tamaño del ejecutable: desconocido"),
    }
    Ok(())
}

/// `deimon arrancar archivo.dmn [--sin-ventana] [--memoria MB]`: compila el programa para
/// `pc-sin-sistema` y lo enciende en QEMU, como si fuera el sistema de un PC.
pub fn arrancar(argumentos: &[String]) -> Result<(), Fallo> {
    let uso = "Uso: deimon arrancar archivo.dmn [--sin-ventana] [--memoria MB]";
    let mut archivo = None;
    let mut sin_ventana = false;
    let mut memoria = 512u32;
    let mut i = 0;
    while i < argumentos.len() {
        match argumentos[i].as_str() {
            "--sin-ventana" => sin_ventana = true,
            "--memoria" => {
                i += 1;
                memoria = argumentos
                    .get(i)
                    .and_then(|valor| valor.parse().ok())
                    .filter(|mb| *mb >= 256)
                    .ok_or_else(|| ("Error: --memoria necesita los megas, 256 o más (el programa reserva 192 MB).".to_string(), 2))?;
            }
            otra if otra.starts_with("--") => {
                return Err((format!("Error: la opción '{}' no existe.\n{}", otra, uso), 2))
            }
            otro if archivo.is_none() => archivo = Some(otro.to_string()),
            otro => return Err((format!("Error: sobra el argumento '{}'.\n{}", otro, uso), 2)),
        }
        i += 1;
    }
    let archivo = archivo.ok_or_else(|| (uso.to_string(), 2))?;
    if !android::existe_en_path("qemu-system-x86_64") {
        return Err((
            "Error: falta QEMU, el programa que hace de PC de pruebas.\nInstálalo con: sudo apt install qemu-system-x86".to_string(),
            1,
        ));
    }
    let iso = archivo_temporal().with_extension("iso");
    let yo = env::current_exe().map_err(|error| (format!("Error: no sé dónde está deimon: {}", error), 1))?;
    let compilacion = Command::new(yo)
        .arg("compilar")
        .arg(&archivo)
        .args(["--destino", SIN_SISTEMA, "--salida"])
        .arg(&iso)
        .output()
        .map_err(|error| (format!("Error: no se pudo compilar: {}", error), 1))?;
    if !compilacion.status.success() {
        let _ = fs::remove_file(&iso);
        let mut mensaje = String::from_utf8_lossy(&compilacion.stdout).into_owned();
        mensaje.push_str(&String::from_utf8_lossy(&compilacion.stderr));
        return Err((mensaje.trim_end().to_string(), 1));
    }
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-m").arg(format!("{}M", memoria));
    qemu.arg("-cdrom").arg(&iso);
    // apagar_maquina() cierra QEMU por este puerto: 0 → código 1, error → código 3.
    qemu.args(["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-no-reboot"]);
    if sin_ventana {
        qemu.args(["-display", "none", "-serial", "stdio"]);
    } else {
        eprintln!("Arrancando {} en QEMU (cierra la ventana o usa apagar_maquina() para terminar)...", archivo);
    }
    let estado = qemu.status();
    let _ = fs::remove_file(&iso);
    match estado.map(|estado| estado.code()) {
        Ok(Some(0)) | Ok(Some(1)) | Ok(None) => Ok(()),
        Ok(Some(3)) => Err((String::new(), 1)),
        Ok(Some(codigo)) => Err((format!("QEMU terminó con el código {}.", codigo), 1)),
        Err(error) => Err((format!("Error: no se pudo ejecutar QEMU: {}", error), 1)),
    }
}

fn error_desconocido(nombre: &str) -> Fallo {
    let validos: Vec<&str> = DESTINOS.iter().map(|destino| destino.nombre).collect();
    let mut mensaje = format!(
        "Error: el destino '{}' no existe.\nDestinos válidos: {}.",
        nombre,
        validos.join(", ")
    );
    if let Some(parecido) = sugerir(nombre) {
        mensaje.push_str(&format!("\n¿Querías decir '{}'?", parecido));
    }
    (mensaje, 2)
}

fn mensaje_no_disponible(destino: &Destino, estado: &Disponibilidad) -> String {
    let mut mensaje = format!(
        "Error: el destino '{}' no está disponible en este ordenador.",
        destino.nombre
    );
    for faltante in &estado.faltantes {
        mensaje.push_str(&format!(
            "\nFalta: {}.\nInstálalo con: {}",
            faltante.que, faltante.instalar
        ));
    }
    mensaje.push_str("\nDespués vuelve a ejecutar 'deimon compilar'.");
    mensaje
}

fn mensaje_de_compilacion(rust: &str, stderr: &[u8]) -> String {
    let mut mensaje = String::new();
    for nombre in deimon::extensiones_del_rust(rust) {
        mensaje.push_str(&format!(
            "El módulo '{}' tiene un error en su código Rust:\n",
            nombre
        ));
    }
    if mensaje.is_empty() {
        mensaje.push_str("Error: no se pudo compilar el programa para ese destino:\n");
    }
    mensaje.push_str(String::from_utf8_lossy(stderr).trim_end());
    mensaje
}

fn ruta_por_defecto(ruta: &Path, destino: &Destino) -> PathBuf {
    let nombre = ruta
        .file_stem()
        .map(|nombre| nombre.to_string_lossy().to_string())
        .unwrap_or_else(|| "programa".to_string());
    let extension = if destino.triple.contains("windows") {
        ".exe"
    } else if destino.nombre == SIN_SISTEMA {
        ".iso"
    } else {
        ""
    };
    ruta.with_file_name(format!("{}-{}{}", nombre, destino.nombre, extension))
}

fn archivo_temporal() -> PathBuf {
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duracion| duracion.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("ln_compilar_{}_{}.rs", process::id(), marca))
}

fn destino_actual() -> Option<&'static Destino> {
    let nombre = match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("windows", "x86_64") => "windows-x64",
        ("android", "aarch64") => "android-arm64",
        ("android", "arm") => "android-arm32",
        _ => return None,
    };
    buscar(nombre)
}

fn disponibilidad(destino: &Destino) -> Disponibilidad {
    let mut faltantes = Vec::new();
    if !objetivo_instalado(destino.triple) {
        faltantes.push(Faltante {
            que: format!("el destino de Rust ({})", destino.triple),
            instalar: destino.instalar[0],
        });
    }

    if destino.nombre == SIN_SISTEMA {
        for programa in ["grub-mkrescue", "xorriso"] {
            if !android::existe_en_path(programa) {
                faltantes.push(Faltante {
                    que: format!("el programa '{}' (crea la imagen que arranca)", programa),
                    instalar: destino.instalar[1],
                });
            }
        }
        if !Path::new("/usr/lib/grub/i386-pc").is_dir() {
            faltantes.push(Faltante {
                que: "el arranque de GRUB para PC (grub-pc-bin)".to_string(),
                instalar: destino.instalar[1],
            });
        }
    }

    let enlazador = match destino.enlazador {
        Enlazador::PorDefecto => None,
        Enlazador::Programa(programa) => {
            if android::existe_en_path(programa) {
                Some(PathBuf::from(programa))
            } else {
                faltantes.push(Faltante {
                    que: format!("el enlazador '{}'", programa),
                    instalar: *destino.instalar.last().unwrap_or(&""),
                });
                None
            }
        }
        Enlazador::Ndk(clang) => match ruta_ndk(clang) {
            Some(ruta) => Some(ruta),
            None => {
                faltantes.push(Faltante {
                    que: format!("el compilador del NDK de Android '{}'", clang),
                    instalar: *destino.instalar.last().unwrap_or(&""),
                });
                None
            }
        },
    };

    Disponibilidad {
        enlazador,
        faltantes,
    }
}

/// Mete el núcleo en una imagen .iso con GRUB (multiboot2) que arranca en un PC con BIOS.
fn crear_imagen_arrancable(elf: &Path, iso: &Path) -> Result<(), Fallo> {
    let carpeta = elf.with_extension("iso.d");
    let grub = carpeta.join("boot").join("grub");
    let preparar = fs::create_dir_all(&grub)
        .and_then(|_| fs::copy(elf, carpeta.join("boot").join("deimon.elf")).map(|_| ()))
        .and_then(|_| fs::write(grub.join("grub.cfg"), GRUB_SIN_SISTEMA));
    if let Err(error) = preparar {
        let _ = fs::remove_dir_all(&carpeta);
        return Err((format!("Error: no se pudo preparar la imagen: {}", error), 1));
    }
    let resultado = Command::new("grub-mkrescue").arg("-o").arg(iso).arg(&carpeta).output();
    let _ = fs::remove_dir_all(&carpeta);
    match resultado {
        Ok(salida) if salida.status.success() => Ok(()),
        Ok(salida) => Err((
            format!(
                "Error: grub-mkrescue no pudo crear la imagen:\n{}",
                String::from_utf8_lossy(&salida.stderr).trim_end()
            ),
            1,
        )),
        Err(error) => Err((format!("Error: no se pudo ejecutar 'grub-mkrescue': {}", error), 1)),
    }
}

fn objetivo_instalado(triple: &str) -> bool {
    Command::new("rustc")
        .args(["--print", "target-libdir", "--target", triple])
        .output()
        .map(|salida| {
            salida.status.success()
                && Path::new(String::from_utf8_lossy(&salida.stdout).trim()).is_dir()
        })
        .unwrap_or(false)
}

fn ruta_ndk(clang: &str) -> Option<PathBuf> {
    let sdk = android::carpeta_sdk().ok()?;
    let ndk = android::ultimo_ndk(&sdk)?;
    let prebuilt = sdk
        .join("ndk")
        .join(&ndk)
        .join("toolchains")
        .join("llvm")
        .join("prebuilt");
    let ruta = prebuilt
        .join("linux-x86_64")
        .join("bin")
        .join(clang);
    if ruta.is_file() {
        return Some(ruta);
    }
    for carpeta in android::subcarpetas(&prebuilt) {
        let ruta = prebuilt.join(carpeta).join("bin").join(clang);
        if ruta.is_file() {
            return Some(ruta);
        }
    }
    None
}

fn sugerir(nombre: &str) -> Option<&'static str> {
    let buscado = nombre.to_lowercase();
    let limite = (buscado.chars().count() / 3).max(2);
    DESTINOS
        .iter()
        .map(|destino| (distancia(&buscado, destino.nombre), destino.nombre))
        .filter(|(distancia, _)| *distancia <= limite)
        .min_by_key(|(distancia, _)| *distancia)
        .map(|(_, nombre)| nombre)
}

/// Distancia de Levenshtein, para sugerir el destino que más se parece a uno escrito mal.
fn distancia(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut anterior: Vec<usize> = (0..=b.len()).collect();
    let mut actual = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        actual[0] = i;
        for j in 1..=b.len() {
            let coste = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            actual[j] = (anterior[j] + 1)
                .min(actual[j - 1] + 1)
                .min(anterior[j - 1] + coste);
        }
        std::mem::swap(&mut anterior, &mut actual);
    }
    anterior[b.len()]
}

#[cfg(test)]
mod pruebas_capacidades {
    use super::*;

    #[test]
    fn avisa_de_las_capacidades_que_el_destino_no_tiene() {
        let sin_red = Capacidades {
            red: false,
            archivos: true,
            hilos: true,
        };
        let pide = vec!["red".to_string(), "archivos".to_string()];
        assert_eq!(capacidades_que_faltan(&sin_red, &pide), vec!["red".to_string()]);
        assert!(capacidades_que_faltan(&TODAS_LAS_CAPACIDADES, &pide).is_empty());
    }
}
