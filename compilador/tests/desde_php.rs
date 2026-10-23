use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_desde_php")
        .join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

fn hay_php() -> bool {
    Command::new("php").arg("-v").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn los_ejemplos_de_php_se_traducen_y_dan_la_misma_salida() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    for nombre in ["tienda", "fizzbuzz", "dificil"] {
        let c = carpeta(nombre);
        let php = c.join(format!("{}.php", nombre));
        fs::copy(raiz().join("ejemplos-php").join(format!("{}.php", nombre)), &php).unwrap();
        let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
            .args(["desde-php", php.to_str().unwrap(), "--comprobar"])
            .env("TMPDIR", &c)
            .output()
            .unwrap();
        let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(0), "{}: {}", nombre, texto);
        assert!(texto.contains("Todo el PHP se pudo traducir"), "{}: {}", nombre, texto);
        assert!(texto.contains("escriben exactamente lo mismo"), "{}: {}", nombre, texto);
    }
}

#[test]
fn lo_que_no_se_puede_traducir_se_avisa_con_su_linea() {
    let traduccion = deimon::desde_php::traducir(
        "<?php\nclass A {}\nbreak;\nswitch ($x) { case 1: break; }\n$m = [1, 'a' => 1];\nwhile (true) { break 2; }\necho \"sigue\\n\";\n",
    )
    .unwrap();
    let lineas: Vec<usize> = traduccion.pendientes.iter().map(|(l, _)| *l).collect();
    for linea in [2, 3, 5, 6] {
        assert!(lineas.contains(&linea), "falta el aviso de la línea {}: {:?}", linea, traduccion.pendientes);
    }
    assert!(traduccion.codigo.contains("# PENDIENTE (línea 2 del PHP): clases"));
    assert!(traduccion.codigo.contains("mostrar(\"sigue\")"));
}

#[test]
fn las_variables_se_declaran_donde_hace_falta() {
    let traduccion = deimon::desde_php::traducir(
        "<?php\n$a = 1;\nif ($a > 0) { $b = 2; }\necho $b;\n$l[] = 5;\n",
    )
    .unwrap();
    let codigo = traduccion.codigo;
    assert!(codigo.contains("sea b = nada"), "{}", codigo);
    assert!(codigo.contains("sea a = 1"), "{}", codigo);
    assert!(codigo.contains("sea l = [5]"), "{}", codigo);
    assert!(deimon::traducir(&codigo).is_ok(), "el Deimon generado no compila:\n{}", codigo);
}

#[test]
fn el_punto_entre_numeros_une_textos_como_en_php() {
    let traduccion = deimon::desde_php::traducir("<?php\n$a = 1;\n$b = 2;\necho $a . $b;\n").unwrap();
    assert!(traduccion.codigo.contains("a_texto(a) + b"), "{}", traduccion.codigo);
}

#[test]
fn un_sitio_web_de_php_responde_igual_traducido() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("sitio");
    let sitio = c.join("sitio");
    fs::create_dir_all(&sitio).unwrap();
    for entrada in fs::read_dir(raiz().join("ejemplos-php").join("sitio")).unwrap() {
        let ruta = entrada.unwrap().path();
        fs::copy(&ruta, sitio.join(ruta.file_name().unwrap())).unwrap();
    }
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["desde-php", sitio.to_str().unwrap(), "--comprobar"])
        .args(["--visitar", "/index.php?buscar=ta"])
        .args(["--visitar", "/saludo.php?nombre=%3Cb%3EEva%3C%2Fb%3E"])
        .args(["--enviar", "/contacto.php", "correo=&mensaje=hola"])
        .args(["--enviar", "/contacto.php", "correo=a%40b.com&mensaje=hola"])
        .env("TMPDIR", &c)
        .output()
        .unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("sitio web: 4 páginas"), "{}", texto);
    assert!(texto.contains("las 8 páginas responden exactamente igual"), "{}", texto);
    let codigo = fs::read_to_string(c.join("sitio.dmn")).unwrap();
    assert!(codigo.contains("si es_envio() {"), "{}", codigo);
    assert!(codigo.contains("devolver redirigir(\"/gracias.php\")"), "{}", codigo);
    assert!(codigo.contains("ruta \"/\" {"), "{}", codigo);
}

#[test]
fn htmlspecialchars_no_protege_dos_veces_lo_que_ya_protege_campo() {
    let t = deimon::desde_php::traducir_web(
        &[("/a.php".to_string(), "<?php\n$n = htmlspecialchars($_GET['n']);\n$m = htmlspecialchars($otra);\necho $n . $m;\n".to_string())],
        8000,
    )
    .unwrap();
    assert!(t.codigo.contains("sea n = campo(\"n\")"), "{}", t.codigo);
    assert!(t.codigo.contains("proteger_html(otra)"), "{}", t.codigo);
}

#[test]
fn switch_se_traduce_a_si_sino_si() {
    let t = deimon::desde_php::traducir(
        "<?php\nfunction f($x) {\n  switch ($x) {\n    case 1:\n    case 2:\n      return \"poco\";\n    case 3:\n      echo \"tres\";\n      break;\n    default:\n      return \"otro\";\n  }\n  return \"fin\";\n}\n",
    )
    .unwrap();
    assert!(t.pendientes.is_empty(), "{:?}", t.pendientes);
    assert!(t.codigo.contains("si x == 1 o x == 2 {"), "{}", t.codigo);
    assert!(t.codigo.contains("} sino si x == 3 {"), "{}", t.codigo);
    assert!(t.codigo.contains("} sino {"), "{}", t.codigo);
    let mal = deimon::desde_php::traducir("<?php\nswitch ($x) { case 1: echo 1; case 2: echo 2; break; }\n").unwrap();
    assert!(mal.pendientes.iter().any(|(_, m)| m.contains("sin break")), "{:?}", mal.pendientes);
}

#[test]
fn un_carrito_con_sesion_responde_igual_paso_a_paso() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("carrito");
    let sitio = c.join("carrito");
    fs::create_dir_all(&sitio).unwrap();
    for entrada in fs::read_dir(raiz().join("ejemplos-php").join("carrito")).unwrap() {
        let ruta = entrada.unwrap().path();
        fs::copy(&ruta, sitio.join(ruta.file_name().unwrap())).unwrap();
    }
    let mut orden = Command::new(env!("CARGO_BIN_EXE_deimon"));
    orden.args(["desde-php", sitio.to_str().unwrap(), "--comprobar"]);
    for camino in ["/index.php?agregar=taza", "/index.php?agregar=gorra", "/index.php", "/vaciar.php", "/index.php"] {
        orden.args(["--visitar", camino]);
    }
    let o = orden.env("TMPDIR", &c).output().unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("las 7 páginas responden exactamente igual"), "{}", texto);
    let codigo = fs::read_to_string(c.join("carrito.dmn")).unwrap();
    assert!(codigo.contains("agregar_a_sesion(\"carrito\", campo(\"agregar\"))"), "{}", codigo);
    assert!(codigo.contains("vaciar_sesion()"), "{}", codigo);
    assert!(!codigo.contains("_SESSION"), "{}", codigo);
}

#[test]
fn ternario_claves_sprintf_y_saltos_dan_lo_mismo_que_php() {
    if Command::new("php").arg("-v").output().is_err() {
        return;
    }
    let c = carpeta("novedades");
    let php = c.join("novedades.php");
    fs::write(
        &php,
        "<?php\n$precios = ['pan' => 1.5, 'leche' => 0.99];\n$precios['queso'] = 4;\nunset($precios['leche']);\nforeach ($precios as $p => $v) {\n    printf(\"%-6s %5.2f\\n\", $p, $v);\n}\n$conteo = [];\nforeach ([\"a\", \"b\", \"a\"] as $l) {\n    $conteo[$l] = ($conteo[$l] ?? 0) + 1;\n}\necho \"a={$conteo['a']} b=$conteo[b]\\n\";\n$t = 0;\nfor ($i = 0; $i < 20; $i++) {\n    if ($i % 3 == 0) { continue; }\n    if ($i > 10) { break; }\n    $t += $i;\n}\necho sprintf(\"%05d\", $t) . ($t > 20 ? \" grande\" : \" chico\") . \"\\n\";\n",
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["desde-php", php.to_str().unwrap(), "--comprobar"])
        .env("TMPDIR", &c)
        .output()
        .unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("Todo el PHP se pudo traducir"), "{}", texto);
    assert!(texto.contains("escriben exactamente lo mismo"), "{}", texto);
    let codigo = fs::read_to_string(c.join("novedades.dmn")).unwrap();
    for trozo in ["{\"pan\": 1.5", "quitar_clave(", "listar_claves(", "formatear_texto(", "continuar", "romper", "elegir("] {
        assert!(codigo.contains(trozo), "falta {}: {}", trozo, codigo);
    }
}

#[test]
fn funciones_de_arrays_y_textos_dan_lo_mismo_que_php() {
    if Command::new("php").arg("-v").output().is_err() {
        return;
    }
    let c = carpeta("funciones_php");
    let php = c.join("funciones.php");
    fs::write(
        &php,
        "<?php\n$f = [\"pera\", \"manzana\", \"uva\"];\nsort($f);\necho implode(\",\", $f) . \"\\n\";\necho in_array(\"uva\", $f) ? \"si\\n\" : \"no\\n\";\necho substr(\"Deimon\", -3) . substr(\"Deimon\", 0, 3) . \"\\n\";\necho array_sum(range(1, 10)) . \"\\n\";\n$d = json_decode('{\"a\":[1,2]}', true);\necho $d['a'][1] . json_encode(['x' => 1]) . \"\\n\";\n$n = rand(1, 6);\necho ($n >= 1 && $n <= 6) ? \"ok\\n\" : \"mal\\n\";\n",
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["desde-php", php.to_str().unwrap(), "--comprobar"])
        .env("TMPDIR", &c)
        .output()
        .unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("Todo el PHP se pudo traducir"), "{}", texto);
    assert!(texto.contains("escriben exactamente lo mismo"), "{}", texto);
}

#[test]
fn las_galletas_de_php_responden_igual_paso_a_paso() {
    if Command::new("php").arg("-v").output().is_err() {
        return;
    }
    let c = carpeta("galletas_php");
    let sitio = c.join("galletas");
    fs::create_dir_all(&sitio).unwrap();
    fs::write(
        sitio.join("index.php"),
        "<?php\nif (isset($_GET['idioma'])) {\n    setcookie('idioma', $_GET['idioma'], time() + 86400 * 30);\n    header('Location: /index.php');\n    exit;\n}\nif (isset($_GET['olvidar'])) {\n    setcookie('idioma', '', time() - 3600);\n    header('Location: /index.php');\n    exit;\n}\necho \"<p>Idioma: \" . htmlspecialchars($_COOKIE['idioma'] ?? 'sin elegir') . \"</p>\";\n",
    )
    .unwrap();
    let mut orden = Command::new(env!("CARGO_BIN_EXE_deimon"));
    orden.args(["desde-php", sitio.to_str().unwrap(), "--comprobar"]);
    for camino in ["/index.php", "/index.php?idioma=es", "/index.php", "/index.php?olvidar=1", "/index.php"] {
        orden.args(["--visitar", camino]);
    }
    let o = orden.env("TMPDIR", &c).output().unwrap();
    let texto = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    assert!(texto.contains("responden exactamente igual"), "{}", texto);
    let codigo = fs::read_to_string(c.join("galletas.dmn")).unwrap();
    assert!(codigo.contains("guardar_galleta(\"idioma\", campo(\"idioma\"), 30)"), "{}", codigo);
    assert!(codigo.contains("borrar_galleta(\"idioma\")") && codigo.contains("leer_galleta(\"idioma\")"), "{}", codigo);
}
