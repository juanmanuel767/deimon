//! `deimon nuevo <programa|web|app|sistema> nombre`: crea un proyecto ya ordenado (programa
//! principal, piezas con sus pruebas, README, arrancar.sh y .gitignore). Hay una sola forma
//! de empezar, y lo que crea pasa `deimon comprobar` y `deimon probar` desde el primer momento.

use std::fs;
use std::path::Path;

type Fallo = (String, i32);

const TIPOS: &[(&str, &str)] = &[
    ("programa", "programa de terminal"),
    ("web", "sitio web con formulario"),
    ("app", "app de Android (una web dentro del móvil)"),
    ("sistema", "sistema operativo que arranca solo en un PC (destino pc-sin-sistema)"),
];

const PIEZA_PROGRAMA: &str = r#"# Cálculos del programa. Cada función lleva su prueba debajo.

funcion saludo(nombre) {
    devolver "Hola, " + nombre + "!"
}

funcion media(numeros) {
    si medir(numeros) == 0 {
        provocar_error("media() necesita al menos un número")
    }
    devolver sumar_lista(numeros) / medir(numeros)
}

prueba "saludo" {
    comprobar_igual(saludo("Ana"), "Hola, Ana!")
}

prueba "media" {
    comprobar_igual(media([2, 4, 6]), 4)
}
"#;

const PRINCIPAL_PROGRAMA: &str = r#"# @NOMBRE@: programa de terminal.
usar "piezas/calculos.dmn"

mostrar(saludo("mundo"))
mostrar("La media de 2, 4 y 6 es " + media([2, 4, 6]))
"#;

const PIEZA_WEB: &str = r#"# Páginas del sitio. Todas usan pagina() para tener el mismo marco.

funcion pagina(titulo, cuerpo) {
    devolver "<!doctype html><html lang='es'><meta charset='utf-8'><meta name='viewport' content='width=device-width'><title>" + titulo + "</title><body><h1>" + titulo + "</h1>" + cuerpo + "</body></html>"
}

ruta "/" {
    si es_envio() {
        devolver redirigir("/saludo/" + campo("nombre"))
    }
    devolver pagina("@NOMBRE@", "<form method='post'><input name='nombre' placeholder='Tu nombre'> <button>Saludar</button></form>")
}

ruta "/saludo/:nombre" {
    devolver pagina("Hola, " + nombre, "<p><a href='/'>Volver</a></p>")
}

prueba "la portada tiene el formulario" {
    comprobar(incluye(visitar("/"), "<form method='post'>"))
}

prueba "el formulario lleva al saludo" {
    comprobar_igual(enviar("/", "nombre=Ana"), "redirige a /saludo/Ana")
    comprobar(incluye(visitar("/saludo/Ana"), "<h1>Hola, Ana</h1>"))
}

prueba "lo que escribe el visitante no rompe la página" {
    comprobar(incluye(visitar("/saludo/%3Cb%3E"), "&lt;b&gt;"))
}
"#;

const PRINCIPAL_WEB: &str = r#"# @NOMBRE@: sitio web. Las páginas están en piezas/paginas.dmn.
usar "piezas/paginas.dmn"

servir(8090)
"#;

const PIEZA_SISTEMA: &str = r#"# Órdenes del sistema: atender() recibe lo tecleado y devuelve las notas.
usar modulo "texto"

funcion ayuda() {
    mostrar("Órdenes: ayuda, nota <texto>, notas, limpiar, apagar")
}

funcion atender(linea, notas) {
    sea limpia = recortar(linea)
    si limpia == "" {
        devolver notas
    }
    sea orden = partir(limpia, " ")[0]
    sea resto = recortar(cortar(limpia, medir(orden), nada))
    si orden == "ayuda" {
        ayuda()
    } sino si orden == "nota" {
        notas = agregar(notas, resto)
        mostrar("apuntada")
    } sino si orden == "notas" {
        para nota en notas {
            mostrar("- " + nota)
        }
    } sino si orden == "limpiar" {
        limpiar_pantalla()
    } sino si orden == "apagar" {
        apagar_maquina()
    } sino {
        mostrar("no conozco '" + orden + "'; escribe ayuda")
    }
    devolver notas
}

prueba "nota guarda el texto" {
    comprobar_igual(atender("nota comprar pan", []), ["comprar pan"])
}

prueba "una línea vacía no cambia nada" {
    comprobar_igual(atender("   ", ["a"]), ["a"])
}
"#;

const PRINCIPAL_SISTEMA: &str = r#"# @NOMBRE@: sistema operativo. En Linux: deimon ejecutar principal.dmn
# Como PC de verdad: deimon arrancar principal.dmn
usar modulo "texto"
usar "piezas/ordenes.dmn"

limpiar_pantalla()
poner_color("blanco", "azul")
escribir_en(0, 0, rellenar_texto(" @NOMBRE@", 80, " ", "derecha"))
poner_color("gris", "negro")
escribir_en(2, 0, "")
ayuda()
sea notas = []
mientras verdadero {
    sea linea = pedir_texto("> ")
    si linea == nada {
        romper
    }
    notas = atender(linea, notas)
}
"#;

const ARRANCAR: &str = r#"#!/usr/bin/env bash
# Atajos de @NOMBRE@. Uso: ./arrancar.sh [@ORDENES@]
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

if ! command -v deimon >/dev/null 2>&1; then
    echo "Falta la orden deimon: instálala con instalar.sh de Deimon." >&2
    exit 1
fi

case "${1:-@PRIMERA@}" in
@CASOS@    *)
        echo "Uso: ./arrancar.sh [@ORDENES@]" >&2
        exit 2
        ;;
esac
"#;

const GITIGNORE: &str = "# Lo que crea Deimon al ejecutar o compilar: no se sube.\ndatos/\n*.registro\n*.datos\n*.apk\n*.iso\n*-linux-*\n*-android-*\n*.exe\n.env*\n";

/// Orden de arrancar.sh y lo que hace (la primera es la que se usa sin argumentos).
fn ordenes(tipo: &str) -> Vec<(&'static str, &'static str)> {
    let mut lista = vec![
        ("probar", "deimon comprobar principal.dmn && deimon probar principal.dmn"),
        ("ejecutar", "deimon ejecutar principal.dmn"),
        ("formatear", "for f in principal.dmn piezas/*.dmn; do deimon formatear \"$f\"; done"),
    ];
    match tipo {
        "web" => lista[1] = ("ejecutar", "echo \"Abre http://localhost:8090\"; deimon ejecutar principal.dmn"),
        "app" => lista.push(("apk", "deimon app principal.dmn --nombre \"@NOMBRE@\"")),
        "sistema" => {
            lista.push(("pc", "deimon arrancar principal.dmn"));
            lista.push(("iso", "deimon compilar principal.dmn --destino pc-sin-sistema"));
        }
        _ => {}
    }
    lista
}

fn arrancar_sh(nombre: &str, tipo: &str) -> String {
    let lista = ordenes(tipo);
    let nombres: Vec<&str> = lista.iter().map(|(orden, _)| *orden).collect();
    let casos: String = lista
        .iter()
        .map(|(orden, accion)| format!("    {})\n        {}\n        ;;\n", orden, accion))
        .collect();
    ARRANCAR
        .replace("@CASOS@", &casos)
        .replace("@ORDENES@", &nombres.join("|"))
        .replace("@PRIMERA@", nombres[0])
        .replace("@NOMBRE@", nombre)
}

fn readme(nombre: &str, tipo: &str, pieza: &str) -> String {
    let descripcion = TIPOS.iter().find(|(t, _)| *t == tipo).map(|(_, d)| *d).unwrap_or("");
    let mut texto = format!(
        "# {}\n\n{} hecho en [Deimon](https://github.com/juanmanuel767/deimon).\n\n## Estructura\n\n```text\n{}/\n  principal.dmn      lo que se ejecuta: carga las piezas y arranca\n  piezas/{:<12} funciones con sus pruebas\n  arrancar.sh        atajos (mira abajo)\n  README.md          este archivo\n```\n\n## Cómo arrancar\n\n```bash\n",
        nombre, primera_mayuscula(descripcion), nombre, pieza
    );
    for (orden, accion) in ordenes(tipo) {
        texto.push_str(&format!("./arrancar.sh {:<10} # {}\n", orden, accion.replace("@NOMBRE@", nombre)));
    }
    texto.push_str("```\n\nCada cambio: `./arrancar.sh probar` antes de dar nada por terminado.\n");
    if tipo == "web" || tipo == "app" {
        texto.push_str("Los datos que guarde el programa quedan en `datos/` (fuera de git).\n");
    }
    texto
}

fn primera_mayuscula(texto: &str) -> String {
    let mut letras = texto.chars();
    match letras.next() {
        Some(primera) => primera.to_uppercase().collect::<String>() + letras.as_str(),
        None => String::new(),
    }
}

fn nombre_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && nombre.len() <= 64
        && !nombre.starts_with(['-', '.'])
        && nombre.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `deimon nuevo <programa|web|app|sistema> nombre`
pub fn nuevo(argumentos: &[String]) -> Result<(), Fallo> {
    let tipos: Vec<&str> = TIPOS.iter().map(|(tipo, _)| *tipo).collect();
    let uso = format!("Uso: deimon nuevo <{}> nombre", tipos.join("|"));
    let [tipo, nombre] = argumentos else {
        let lista: String = TIPOS.iter().map(|(tipo, d)| format!("\n  {:<9} {}", tipo, d)).collect();
        return Err((format!("{}{}", uso, lista), 2));
    };
    let (tipo, nombre) = (tipo.as_str(), nombre.as_str());
    if !tipos.contains(&tipo) {
        return Err((format!("Error: el tipo '{}' no existe; elige {}.\n{}", tipo, tipos.join(", "), uso), 2));
    }
    if !nombre_valido(nombre) {
        return Err((
            format!(
                "Error: el nombre '{}' no vale para una carpeta; usa letras sin acentos, números, '-' o '_' (por ejemplo mi-tienda).",
                nombre
            ),
            2,
        ));
    }
    let carpeta = Path::new(nombre);
    if carpeta.exists() {
        return Err((format!("Error: ya existe '{}'; elige otro nombre o bórrala tú antes.", nombre), 1));
    }
    let (pieza, codigo_pieza, principal) = match tipo {
        "programa" => ("calculos.dmn", PIEZA_PROGRAMA, PRINCIPAL_PROGRAMA),
        "sistema" => ("ordenes.dmn", PIEZA_SISTEMA, PRINCIPAL_SISTEMA),
        _ => ("paginas.dmn", PIEZA_WEB, PRINCIPAL_WEB),
    };
    let archivos = [
        ("principal.dmn".to_string(), principal.replace("@NOMBRE@", nombre)),
        (format!("piezas/{}", pieza), codigo_pieza.replace("@NOMBRE@", nombre)),
        ("README.md".to_string(), readme(nombre, tipo, pieza)),
        ("arrancar.sh".to_string(), arrancar_sh(nombre, tipo)),
        (".gitignore".to_string(), GITIGNORE.to_string()),
    ];
    let escribir = || -> std::io::Result<()> {
        fs::create_dir_all(carpeta.join("piezas"))?;
        for (ruta, contenido) in &archivos {
            fs::write(carpeta.join(ruta), contenido)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(carpeta.join("arrancar.sh"), fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    };
    if let Err(error) = escribir() {
        let _ = fs::remove_dir_all(carpeta);
        return Err((format!("Error: no se pudo crear '{}': {}", nombre, error), 1));
    }
    println!("✓ Proyecto '{}' creado ({}).", nombre, TIPOS.iter().find(|(t, _)| *t == tipo).map(|(_, d)| *d).unwrap_or(""));
    for (ruta, _) in &archivos {
        println!("  {}/{}", nombre, ruta);
    }
    println!("Siguiente paso: cd {} && ./arrancar.sh probar", nombre);
    Ok(())
}
