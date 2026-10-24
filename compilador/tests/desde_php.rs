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
fn formato_de_numeros_y_textos_dan_lo_mismo_que_php() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("formato_php");
    let php = c.join("formato.php");
    fs::write(
        &php,
        "<?php\n$datos = [\"pan\" => 1.5, \"lista\" => [1, 2, [\"x\" => \"hola\", \"y\" => [true, false, null]]]];\necho number_format(1234.5678, 2) . \"|\" . number_format(-1234.567, 2, \",\", \".\") . \"\\n\";\necho number_format(1234) . \"|\" . number_format(-0.4, 2) . \"\\n\";\necho str_pad(\"hola\", 8) . \"|\" . str_pad(\"hola\", 8, \"-\", STR_PAD_LEFT) . \"|\" . str_pad(\"hola\", 9, \"ab\", STR_PAD_BOTH) . \"\\n\";\necho ucfirst(\"hola\") . \"\\n\";\nprint_r($datos);\n",
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
    let codigo = fs::read_to_string(c.join("formato.dmn")).unwrap();
    for trozo in ["formatear_numero(", "rellenar_texto(", "primera_mayuscula(", "texto_completo("] {
        assert!(codigo.contains(trozo), "falta {}: {}", trozo, codigo);
    }
    assert!(!codigo.contains("print_r"), "{}", codigo);
}

#[test]
fn var_dump_se_traduce_a_texto_completo() {
    let t = deimon::desde_php::traducir("<?php\n$a = [1, ['x' => 2]];\nvar_dump($a);\nprint_r($a);\n").unwrap();
    assert!(t.pendientes.is_empty(), "{:?}", t.pendientes);
    assert!(t.codigo.contains("escribir(texto_completo(a))"), "{}", t.codigo);
    assert!(deimon::traducir(&t.codigo).is_ok(), "el Deimon generado no compila:\n{}", t.codigo);
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

#[test]
fn las_fechas_de_php_se_traducen_y_dan_lo_mismo() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("fechas_php");
    let php = c.join("fechas.php");
    fs::write(
        &php,
        "<?php\n\
date_default_timezone_set('UTC');\n\
echo date('Y-m-d H:i:s', 0), \"\\n\";\n\
echo date('Y-m-d H:i:s', mktime(13, 5, 9, 2, 29, 2024)), \"\\n\";\n\
echo date('D N w z t L y U', mktime(0, 0, 0, 12, 31, 2026)), \"\\n\";\n\
echo checkdate(2, 29, 2024) ? 'si' : 'no', \"\\n\";\n\
echo checkdate(2, 29, 2023) ? 'si' : 'no', \"\\n\";\n\
echo checkdate(13, 1, 2026) ? 'si' : 'no', \"\\n\";\n\
echo cal_days_in_month(CAL_GREGORIAN, 2, 2024), \"\\n\";\n\
echo cal_days_in_month(CAL_GREGORIAN, 2, 2026), \"\\n\";\n\
echo strtotime('2026-03-29 01:30:00'), \"\\n\";\n\
echo strtotime('2026-03-29T01:30:00+01:00'), \"\\n\";\n\
echo date('Y-m-d H:i:s', strtotime('+1 month', mktime(0, 0, 0, 1, 31, 2026))), \"\\n\";\n\
echo date('Y-m-d H:i:s', strtotime('+1 year', mktime(0, 0, 0, 2, 29, 2024))), \"\\n\";\n\
echo date('Y-m-d H:i:s', strtotime('+2 weeks', mktime(0, 0, 0, 12, 31, 2026))), \"\\n\";\n",
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
fn date_usa_la_zona_de_date_default_timezone_set() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("fechas_madrid");
    let php = c.join("madrid.php");
    fs::write(
        &php,
        "<?php\n\
date_default_timezone_set('Europe/Madrid');\n\
echo date('Y-m-d H:i:s', 1774744200), \"\\n\";\n\
echo date('Y-m-d H:i:s', 1774746000), \"\\n\";\n\
echo date('Y-m-d H:i:s', 1767225600), \"\\n\";\n",
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
fn las_expresiones_regulares_de_php_dan_lo_mismo_que_php() {
    if Command::new("php").arg("-v").output().is_err() {
        return;
    }
    let c = carpeta("preg");
    let php = c.join("preg.php");
    fs::write(
        &php,
        r##"<?php
$correo = "ana.lopez@ejemplo.es";
if (preg_match('/^([\w.]+)@(\w+)\.(\w+)$/', $correo, $m)) {
    echo "usuario=" . $m[1] . " sitio=" . $m[2] . " fin=" . $m[3] . "\n";
}
if (preg_match('/^\d+$/', "12a")) {
    echo "numero\n";
} else {
    echo "no numero\n";
}
if (preg_match('#HOLA (\w+)#i', "he dicho hola Mundo", $g)) {
    echo $g[0] . "|" . $g[1] . "\n";
}
$n = preg_match_all('~\d+~', "a1 b22 c333", $todos);
echo $n . ":" . implode(",", $todos[0]) . "\n";
preg_match_all('/([a-z])(\d)/', "a1 b2 c3", $pares);
echo implode("", $pares[1]) . "-" . implode("", $pares[2]) . "\n";
echo preg_replace('/(\d+)-(\d+)/', '$2/$1', "10-20 y 30-40") . "\n";
echo preg_replace('/\s+/', " ", "a   b \t c") . "\n";
echo implode("|", preg_split('/[,;]\s*/', "a, b;c,d")) . "\n";
echo implode("|", preg_split('/\d/', "a1b2c")) . "\n";
echo (preg_match('/colou?r/', "color") ? "si" : "no") . "\n";
"##,
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
    let codigo = fs::read_to_string(c.join("preg.dmn")).unwrap();
    for trozo in ["usar modulo \"patrones\"", "buscar_patron(", "buscar_columnas_patron(", "reemplazar_patron(", "partir_patron(", "coincide_patron(", "(?i)HOLA"] {
        assert!(codigo.contains(trozo), "falta {}: {}", trozo, codigo);
    }
}

#[test]
fn lo_que_los_patrones_no_soportan_se_avisa() {
    let traduccion = deimon::desde_php::traducir(
        "<?php\nif (preg_match('/a(?=b)/', $x)) { echo 1; }\npreg_match('/(a)\\1/', $x);\npreg_match('/a b/x', $x);\npreg_match($p, $x);\npreg_replace('/a/', 'b', $x, 1);\n",
    )
    .unwrap();
    let lineas: Vec<usize> = traduccion.pendientes.iter().map(|(l, _)| *l).collect();
    for linea in [2, 3, 4, 5, 6] {
        assert!(lineas.contains(&linea), "falta el aviso de la línea {}: {:?}", linea, traduccion.pendientes);
    }
    assert!(traduccion.codigo.contains("# PENDIENTE (línea 2 del PHP): preg_match: los grupos especiales"));
}

#[test]
fn try_catch_y_throw_se_traducen_y_dan_la_misma_salida_que_php() {
    if !hay_php() {
        eprintln!("se salta: no hay PHP instalado");
        return;
    }
    let c = carpeta("atrapar");
    let php = c.join("atrapar.php");
    fs::write(
        &php,
        "<?php\nfunction dividir($a, $b) {\n  if ($b == 0) {\n    throw new Exception(\"no se divide entre cero\");\n  }\n  return $a / $b;\n}\ntry {\n  echo dividir(6, 3) . \"\\n\";\n  echo dividir(1, 0) . \"\\n\";\n  echo \"no llega\\n\";\n} catch (Exception $e) {\n  echo \"error: \" . $e->getMessage() . \"\\n\";\n}\ntry {\n  try {\n    throw new Exception(\"interno\");\n  } catch (Throwable $e) {\n    echo $e->getMessage() . \"\\n\";\n    throw new Exception(\"otro\");\n  }\n} catch (Exception $f) {\n  echo $f->getMessage() . \"\\n\";\n}\necho \"fin\\n\";\n",
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
fn finally_y_varios_catch_se_avisan_como_pendientes() {
    for php in [
        "<?php\ntry { echo 1; } catch (Exception $e) { echo 2; } finally { echo 3; }\necho \"sigue\\n\";\n",
        "<?php\ntry { echo 1; } catch (InvalidArgumentException $e) { echo 2; }\necho \"sigue\\n\";\n",
        "<?php\ntry { echo 1; } catch (A | B $e) { echo 2; }\necho \"sigue\\n\";\n",
        "<?php\ntry { echo 1; } catch (Exception $e) { echo 2; } catch (Error $x) { echo 3; }\necho \"sigue\\n\";\n",
    ] {
        let t = deimon::desde_php::traducir(php).unwrap();
        assert_eq!(t.pendientes.len(), 1, "{}: {:?}", php, t.pendientes);
        assert_eq!(t.pendientes[0].0, 2, "{:?}", t.pendientes);
        assert!(t.codigo.contains("# PENDIENTE (línea 2 del PHP)"), "{}", t.codigo);
        assert!(t.codigo.contains("mostrar(\"sigue\")"), "{}", t.codigo);
    }
}
