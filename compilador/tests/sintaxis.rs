use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn binario() -> &'static str {
    env!("CARGO_BIN_EXE_deimon")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join(format!("sintaxis_{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta temporal");
    carpeta
}

fn escribir(carpeta: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    fs::write(&ruta, contenido).expect("no se pudo escribir el archivo de prueba");
    ruta
}

fn deimon(carpeta: &Path, argumentos: &[&str]) -> Output {
    Command::new(binario())
        .args(argumentos)
        .current_dir(carpeta)
        .env_remove("DEIMON_PUERTO")
        .env_remove("DEIMON_DATOS")
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn ejecutar(ruta: &Path, datos: &Path) -> Output {
    Command::new(binario())
        .arg("ejecutar")
        .arg(ruta)
        .env("DEIMON_DATOS", datos)
        .output()
        .expect("no se pudo lanzar el binario 'Deimon'")
}

fn compilar(carpeta: &Path, archivo: &Path) -> PathBuf {
    let salida = deimon(carpeta, &["traducir", archivo.to_str().unwrap()]);
    assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
    let rust = carpeta.join("programa.rs");
    fs::write(&rust, &salida.stdout).expect("no se pudo escribir el Rust generado");
    let bin = carpeta.join("programa.bin");
    let compilacion = Command::new("rustc")
        .args(["--edition=2021", "-O"])
        .arg(&rust)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("no se pudo ejecutar 'rustc'");
    assert!(
        compilacion.status.success(),
        "no se pudo compilar: {}",
        String::from_utf8_lossy(&compilacion.stderr)
    );
    bin
}

fn contiene_llamada(fuente: &str, nombre: &str) -> bool {
    let patron = format!("{}(", nombre);
    fuente.match_indices(&patron).any(|(inicio, _)| {
        !matches!(
            fuente[..inicio].chars().next_back(),
            Some(caracter) if caracter.is_alphanumeric() || caracter == '_'
        )
    })
}

fn texto(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stdout).to_string()
}

fn errores(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stderr).to_string()
}

#[test]
fn los_nombres_antiguos_dan_e021_y_se_arreglan_solos() {
    let casos = [
        (
            "numero",
            "a_numero",
            "mostrar(numero(\"21\") * 2)\n",
            "42\n",
        ),
        ("largo", "medir", "mostrar(largo([1, 2, 3]))\n", "3\n"),
        (
            "claves",
            "listar_claves",
            "guardar(\"sintaxis_claves\", \"a\", 1)\nmostrar(medir(claves(\"sintaxis_claves\")))\n",
            "1\n",
        ),
    ];
    for (antiguo, nuevo, fuente, esperado) in casos {
        let carpeta = carpeta(&format!("antiguo_{}", antiguo));
        let archivo = escribir(&carpeta, "programa.dmn", fuente);

        let todos = deimon::comprobar(fuente);
        let error = todos
            .iter()
            .find(|error| error.codigo == "E021")
            .unwrap_or_else(|| panic!("falta E021 para '{}': {:?}", antiguo, todos));
        assert!(
            error
                .sugerencia
                .as_deref()
                .unwrap_or_default()
                .contains(&format!("Ahora se escribe '{}'", nuevo)),
            "{:?}",
            error.sugerencia
        );

        let salida = deimon(&carpeta, &["comprobar", archivo.to_str().unwrap(), "--arreglar"]);
        assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
        let arreglado = fs::read_to_string(&archivo).unwrap();
        assert!(contiene_llamada(&arreglado, nuevo), "{}", arreglado);
        assert!(!contiene_llamada(&arreglado, antiguo), "{}", arreglado);

        let salida = ejecutar(&archivo, &carpeta.join("datos"));
        assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
        assert_eq!(texto(&salida), esperado);
        let _ = fs::remove_dir_all(&carpeta);
    }
}

#[test]
fn a_texto_convierte_cualquier_valor_a_texto() {
    let carpeta = carpeta("a_texto");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "mostrar(a_texto(2.5))\nmostrar(a_texto([1, verdadero]))\nmostrar(a_texto(nada))\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
    assert_eq!(texto(&salida), "2.5\n[1, verdadero]\nnada\n");
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn prueba_y_extension_no_se_pueden_usar_como_nombre() {
    let casos = [
        "sea prueba = 1\n",
        "sea extension = 1\n",
        "funcion prueba() {\n    devolver 1\n}\n",
        "funcion extension() {\n    devolver 1\n}\n",
        "funcion f(prueba) {\n    devolver prueba\n}\n",
        "ruta \"/x/:prueba\" {\n    devolver \"x\"\n}\n",
    ];
    for fuente in casos {
        let todos = deimon::comprobar(fuente);
        let error = todos
            .iter()
            .find(|error| error.codigo == "E022")
            .unwrap_or_else(|| panic!("falta E022 para {:?}: {:?}", fuente, todos));
        assert!(
            error.mensaje.contains("palabra reservada"),
            "{}",
            error.mensaje
        );
    }
}

#[test]
fn se_puede_asignar_por_indice_a_una_lista() {
    let carpeta = carpeta("indice");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "sea lista = [1, 2, 3]\nlista[1] = 9\nmostrar(lista)\nlista[0] = lista[2] + 1\nmostrar(lista)\nsea matriz = [[1, 2], [3, 4]]\nmatriz[1][0] = 7\nmostrar(matriz)\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
    assert_eq!(texto(&salida), "[1, 9, 3]\n[4, 9, 3]\n[[1, 2], [7, 4]]\n");
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn el_indice_fuera_de_rango_dice_la_linea() {
    let carpeta = carpeta("fuera_de_rango");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "sea lista = [1]\nlista[2] = 3\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(!salida.status.success());
    let error = errores(&salida);
    assert!(
        error.contains("Error en la línea 2 (en ejecución)") && error.contains("fuera de rango"),
        "{}",
        error
    );
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn asignar_cien_mil_veces_por_indice_es_rapido() {
    let carpeta = carpeta("rapido");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "sea lista = []\nsea i = 0\nmientras i < 100000 {\n    lista = agregar(lista, 0)\n    i = i + 1\n}\ni = 0\nmientras i < 100000 {\n    lista[i] = i\n    i = i + 1\n}\nmostrar(lista[99999])\n",
    );
    let bin = compilar(&carpeta, &archivo);
    let inicio = Instant::now();
    let salida = Command::new(&bin)
        .env("DEIMON_DATOS", carpeta.join("datos"))
        .output()
        .expect("no se pudo ejecutar el programa compilado");
    let tiempo = inicio.elapsed();
    assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
    assert_eq!(texto(&salida), "99999\n");
    assert!(
        tiempo < Duration::from_secs(2),
        "tardó {:?} (más de 2 s)",
        tiempo
    );
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn un_texto_se_lee_por_caracteres_no_por_bytes() {
    let carpeta = carpeta("texto_indice");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "mostrar(\"año\"[1])\nsea texto = \"año\"\nmostrar(texto[1])\nmostrar(medir(texto))\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(salida.status.success(), "{}{}", texto(&salida), errores(&salida));
    assert_eq!(texto(&salida), "ñ\nñ\n3\n");
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn cambiar_un_texto_por_partes_es_un_error_de_traduccion() {
    let todos = deimon::comprobar("sea texto = \"año\"\ntexto[1] = \"x\"\n");
    let error = todos
        .iter()
        .find(|error| error.codigo == "E023")
        .unwrap_or_else(|| panic!("falta E023: {:?}", todos));
    assert!(
        error.mensaje.contains("no se pueden cambiar por partes"),
        "{}",
        error.mensaje
    );
}

#[test]
fn un_bucle_de_cuentas_da_el_mismo_resultado_y_conserva_los_errores() {
    let carpeta = carpeta("bucle_numerico");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "sea i = 0\nsea total = 0\nsea nombre = \"x\"\nmientras i < 10 {\n    total = total + i % 3\n    i = i + 1\n}\nmostrar(total)\nmostrar(nombre)\nsea d = 3\nmientras d > -1 {\n    total = total / d\n    d = d - 1\n}\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(!salida.status.success());
    assert_eq!(texto(&salida), "9\nx\n");
    let error = errores(&salida);
    assert!(
        error.contains("Error en la línea 12 (en ejecución)") && error.contains("dividir entre cero"),
        "{}",
        error
    );
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn listas_con_claves_romper_continuar_y_elegir() {
    let carpeta = carpeta("listas_con_claves");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "sea p = {\"nombre\": \"Ana\", \"edad\": 30}\np[\"edad\"] = p[\"edad\"] + 1\np[\"ciudad\"] = \"Lima\"\np = quitar_clave(p, \"nombre\")\nmostrar(p)\nmostrar(medir(p))\nmostrar(tiene_clave(p, \"edad\"))\nmostrar(p[\"falta\"] == nada)\npara clave en p {\n    mostrar(clave)\n}\nsea vacia = []\nvacia[\"x\"] = 1\nmostrar(vacia)\nguardar(\"gente\", \"p\", p)\nmostrar(leer(\"gente\", \"p\")[\"ciudad\"])\nsea i = 0\nmientras verdadero {\n    i = i + 1\n    si i == 2 {\n        continuar\n    }\n    si i > 3 {\n        romper\n    }\n    mostrar(i)\n}\nmostrar(elegir(i > 3, \"si\", 1 / 0))\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(salida.status.success(), "{}", errores(&salida));
    assert_eq!(
        texto(&salida),
        "{edad: 31, ciudad: Lima}\n2\nverdadero\nverdadero\nedad\nciudad\n{x: 1}\nLima\n1\n3\nsi\n"
    );
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn romper_fuera_de_un_bucle_es_un_error_de_traduccion() {
    let carpeta = carpeta("romper_fuera");
    let archivo = escribir(&carpeta, "programa.dmn", "sea a = 1\ncontinuar\n");
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(!salida.status.success());
    let error = errores(&salida);
    assert!(error.contains("línea 2") && error.contains("dentro de un bucle"), "{}", error);
    let _ = fs::remove_dir_all(&carpeta);
}

#[test]
fn funciones_de_listas_y_textos() {
    let carpeta = carpeta("funciones_listas");
    let archivo = escribir(
        &carpeta,
        "programa.dmn",
        "usar modulo \"json\"\nmostrar(incluye([1, 2], 2))\nmostrar(incluye({\"a\": \"x\"}, \"y\"))\nmostrar(ordenar([\"pera\", \"manzana\"]))\nmostrar(invertir(\"año\"))\nmostrar(cortar(\"Deimon\", -3, nada))\nmostrar(cortar([1, 2, 3, 4, 5], 1, -1))\nmostrar(sumar_lista(rango(1, 4)))\nmostrar(rango(3, 1))\nsea d = desde_json(\"{\\\"a\\\": {\\\"b\\\": [1, 2]}}\")\nmostrar(d[\"a\"][\"b\"][1])\n",
    );
    let salida = ejecutar(&archivo, &carpeta.join("datos"));
    assert!(salida.status.success(), "{}", errores(&salida));
    assert_eq!(
        texto(&salida),
        "verdadero\nfalso\n[manzana, pera]\noña\nmon\n[2, 3, 4]\n10\n[3, 2, 1]\n2\n"
    );
    let _ = fs::remove_dir_all(&carpeta);
}
