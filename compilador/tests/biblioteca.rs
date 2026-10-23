use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Carpeta de prueba dentro de target/tmp (nunca se escribe en /tmp).
fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_biblioteca")
        .join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).expect("no se pudo crear la carpeta de prueba");
    carpeta
}

fn escribir(carpeta: &Path, nombre: &str, contenido: &str) -> PathBuf {
    let ruta = carpeta.join(nombre);
    fs::create_dir_all(ruta.parent().unwrap()).unwrap();
    fs::write(&ruta, contenido).unwrap();
    ruta
}

fn ln_con(binario: &Path, carpeta: &Path, argumentos: &[&str]) -> Output {
    Command::new(binario)
        .args(argumentos)
        .current_dir(carpeta)
        .env("TMPDIR", carpeta)
        .env_remove("DEIMON_PUERTO")
        .env_remove("DEIMON_DATOS")
        .output()
        .expect("no se pudo lanzar deimon")
}

fn deimon(carpeta: &Path, argumentos: &[&str]) -> Output {
    ln_con(Path::new(env!("CARGO_BIN_EXE_deimon")), carpeta, argumentos)
}

fn todo(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn ejecutar(nombre: &str, fuente: &str) -> Output {
    let c = carpeta(nombre);
    escribir(&c, "programa.dmn", fuente);
    deimon(&c, &["ejecutar", "programa.dmn"])
}

#[test]
fn matematicas_calcula_bien() {
    let o = ejecutar(
        "matematicas",
        "usar modulo \"matematicas\"\nmostrar(raiz(81))\nmostrar(potencia(3, 4))\nmostrar(absoluto(-2.5))\nmostrar(redondear(1.4) + suelo(1.9) + techo(1.1))\nmostrar(minimo(-1, 1) + maximo(-1, 1))\nmostrar(factorial(6))\nmostrar(es_par(7))\nsea a = aleatorio()\nmostrar(a >= 0 y a < 1)\n",
    );
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "9\n81\n2.5\n4\n0\n720\nfalso\nverdadero\n"
    );
}

#[test]
fn raiz_de_un_negativo_es_un_error_con_linea() {
    let o = ejecutar("raiz_negativa", "usar modulo \"matematicas\"\nmostrar(raiz(-4))\n");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        todo(&o).contains("Error en la línea 2 (en ejecución): raiz() no puede calcular la raíz de un número negativo"),
        "{}",
        todo(&o)
    );
}

#[test]
fn texto_y_tiempo_funcionan() {
    let o = ejecutar(
        "texto_tiempo",
        "usar modulo \"texto\"\nusar modulo \"tiempo\"\nmostrar(unir(partir(\"x;y\", \";\"), \"+\"))\nmostrar(repetir(\"ab\", 2))\nmostrar(empieza_con(\"casa\", \"ca\"))\nmostrar(termina_con(\"casa\", \"x\"))\nmostrar(fecha(86400 * 365))\nmostrar(fecha(951782400))\n",
    );
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "x+y\nabab\nverdadero\nfalso\n1971-01-01 00:00:00\n2000-02-29 00:00:00\n"
    );
}

#[test]
fn un_modulo_que_no_existe_sugiere_el_parecido() {
    let c = carpeta("no_existe");
    escribir(&c, "programa.dmn", "usar modulo \"texo\"\n");
    let o = deimon(&c, &["comprobar", "programa.dmn"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(todo(&o).contains("Código: E017"), "{}", todo(&o));
    assert!(todo(&o).contains("¿Quisiste decir \"texto\"?"), "{}", todo(&o));
}

#[test]
fn un_modulo_sin_modulo_txt_o_mal_formado_da_e018() {
    let c = carpeta("ficha");
    escribir(&c, "modulos/sinficha/saludos.dmn", "funcion hola() {\n    devolver 1\n}\n");
    escribir(&c, "modulos/mala/modulo.txt", "nombre: mala\nesto no vale\n");
    escribir(&c, "modulos/otro/modulo.txt", "nombre: distinto\ndescripcion: x\n");
    escribir(&c, "modulos/pide/modulo.txt", "nombre: pide\ndescripcion: x\nrequiere: magia\n");
    for (modulo, texto) in [
        ("sinficha", "no tiene un modulo.txt"),
        ("mala", "se esperaba 'clave: valor'"),
        ("otro", "pero la carpeta del módulo se llama 'otro'"),
        ("pide", "la capacidad 'magia' no existe"),
    ] {
        escribir(&c, "programa.dmn", &format!("usar modulo \"{}\"\n", modulo));
        let o = deimon(&c, &["comprobar", "programa.dmn"]);
        assert_eq!(o.status.code(), Some(1), "{}", modulo);
        assert!(todo(&o).contains("Código: E018"), "{}: {}", modulo, todo(&o));
        assert!(todo(&o).contains(texto), "{}: {}", modulo, todo(&o));
    }
}

#[test]
fn un_modulo_propio_puede_tener_solo_funciones_en_ln() {
    let c = carpeta("solo_ln");
    escribir(&c, "modulos/dobles/modulo.txt", "nombre: dobles\ndescripcion: dobla\nrequiere:\n");
    escribir(&c, "modulos/dobles/dobles.dmn", "funcion doblar(n) {\n    devolver n * 2\n}\n");
    escribir(&c, "programa.dmn", "usar modulo \"dobles\"\nmostrar(doblar(21))\n");
    let o = deimon(&c, &["ejecutar", "programa.dmn"]);
    assert_eq!(String::from_utf8_lossy(&o.stdout), "42\n", "{}", todo(&o));
}

#[test]
fn ln_modulos_lista_la_biblioteca_y_los_propios() {
    let c = carpeta("listar");
    escribir(&c, "modulos/mio/modulo.txt", "nombre: mio\ndescripcion: mi módulo\n");
    escribir(&c, "programa.dmn", "mostrar(1)\n");
    let o = deimon(&c, &["modulos", "programa.dmn"]);
    let texto = todo(&o);
    assert_eq!(o.status.code(), Some(0), "{}", texto);
    for esperado in ["■ mio (junto al programa)", "■ matematicas", "■ texto", "■ tiempo", "factorial(n)"] {
        assert!(texto.contains(esperado), "falta {}: {}", esperado, texto);
    }
    let o = deimon(&c, &["modulos", "--json"]);
    let json = String::from_utf8_lossy(&o.stdout);
    assert!(json.starts_with("{\"modulos\":["), "{}", json);
    assert!(json.contains("\"nombre\":\"tiempo\""), "{}", json);
}

#[test]
fn la_biblioteca_va_dentro_del_binario() {
    let c = carpeta("copiado");
    let copia = c.join("deimon-copiado");
    fs::copy(env!("CARGO_BIN_EXE_deimon"), &copia).expect("no se pudo copiar Deimon");
    escribir(&c, "programa.dmn", "usar modulo \"texto\"\nmostrar(mayusculas(\"ok\"))\n");
    let o = ln_con(&copia, &c, &["ejecutar", "programa.dmn"]);
    assert_eq!(String::from_utf8_lossy(&o.stdout), "OK\n", "{}", todo(&o));
}

#[test]
fn pdf_crea_un_documento_con_acentos_y_varias_paginas() {
    let c = carpeta("pdf");
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"pdf\"\nsea lineas = [\"Señora Peña: 30 €\"]\nsea i = 0\nmientras i < 100 {\n    lineas = agregar(lineas, \"línea \" + i)\n    i = i + 1\n}\ncrear_pdf(\"salida.pdf\", \"Título\", lineas)\n",
    );
    let o = deimon(&c, &["ejecutar", "programa.dmn"]);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    let pdf = fs::read(c.join("salida.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    let texto = String::from_utf8_lossy(&pdf);
    assert!(texto.contains("/Count 3"), "deberían ser 3 páginas");
    // "Señora Peña: 30 €" en WinAnsi: ñ = 0xF1, € = 0x80.
    assert!(pdf.windows(4).any(|v| v == b"Se\xF1o"));
    assert!(pdf.windows(4).any(|v| v == b"30 \x80"));
    let mal = escribir(&c, "mal.dmn", "usar modulo \"pdf\"\ncrear_pdf(\"salida.txt\", \"x\", [])\n");
    let o = deimon(&c, &["ejecutar", mal.to_str().unwrap()]);
    assert!(todo(&o).contains("termine en .pdf"), "{}", todo(&o));
}

#[test]
fn correo_pasa_el_mensaje_a_sendmail_y_rechaza_cabeceras_inyectadas() {
    let c = carpeta("correo");
    let capturado = c.join("capturado.txt");
    let falso = escribir(&c, "falso-sendmail", &format!("#!/bin/sh\ncat > '{}'\n", capturado.display()));
    let mut permisos = fs::metadata(&falso).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permisos, 0o755);
    fs::set_permissions(&falso, permisos).unwrap();
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"correo\"\nmostrar(es_correo_valido(\"a@b.com\"))\nmostrar(es_correo_valido(\"a@b\"))\nmostrar(enviar_correo(\"ana@ejemplo.com\", \"Código ñ\", \"Hola\"))\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "programa.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_SENDMAIL", &falso)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "verdadero\nfalso\nverdadero\n", "{}", todo(&o));
    let mensaje = fs::read_to_string(&capturado).unwrap();
    assert!(mensaje.contains("To: ana@ejemplo.com\r\n"), "{}", mensaje);
    assert!(mensaje.contains("Subject: =?UTF-8?B?"), "{}", mensaje);
    escribir(&c, "inyeccion.dmn", "usar modulo \"correo\"\nenviar_correo(\"ana@ejemplo.com\", \"hola\\nBcc: todos@x.com\", \"x\")\n");
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "inyeccion.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_SENDMAIL", &falso)
        .output()
        .unwrap();
    assert!(todo(&o).contains("no puede tener saltos de línea"), "{}", todo(&o));
}

#[test]
fn pagos_crea_el_pago_en_stripe_y_comprueba_si_se_pago() {
    use std::io::{Read, Write};
    let servidor = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = servidor.local_addr().unwrap().port();
    let recibido = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let copia = recibido.clone();
    std::thread::spawn(move || {
        for conexion in servidor.incoming().flatten() {
            let mut conexion = conexion;
            let mut datos = vec![0u8; 16384];
            let mut total = 0;
            conexion.set_read_timeout(Some(std::time::Duration::from_millis(500))).unwrap();
            while let Ok(n) = conexion.read(&mut datos[total..]) {
                if n == 0 { break; }
                total += n;
                let texto = String::from_utf8_lossy(&datos[..total]).to_string();
                if let Some(fin) = texto.find("\r\n\r\n") {
                    let largo: usize = texto.lines().find_map(|l| l.strip_prefix("Content-Length: ").or_else(|| l.strip_prefix("content-length: "))).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
                    if total >= fin + 4 + largo { break; }
                }
            }
            let peticion = String::from_utf8_lossy(&datos[..total]).to_string();
            let cuerpo = if peticion.starts_with("POST") {
                *copia.lock().unwrap() = peticion.clone();
                r#"{"id": "cs_1", "url": "https:\/\/checkout.stripe.com\/pay\/cs_1"}"#.to_string()
            } else if peticion.contains("cs_pagado") {
                r#"{"id": "cs_pagado", "payment_status": "paid"}"#.to_string()
            } else {
                r#"{"id": "cs_otro", "payment_status": "unpaid"}"#.to_string()
            };
            let _ = conexion.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", cuerpo.len(), cuerpo).as_bytes());
        }
    });
    let c = carpeta("pagos");
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"pagos\"\nmostrar(crear_pago(\"sk_test_1\", 1500, \"eur\", \"Camiseta\", \"https://t.com/ok\", \"https://t.com/no\"))\nmostrar(pago_completado(\"sk_test_1\", \"cs_pagado\"))\nmostrar(pago_completado(\"sk_test_1\", \"cs_otro\"))\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "programa.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_STRIPE_URL", format!("http://127.0.0.1:{}", puerto))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "https://checkout.stripe.com/pay/cs_1\nverdadero\nfalso\n",
        "{}",
        todo(&o)
    );
    let peticion = recibido.lock().unwrap().clone();
    assert!(peticion.contains("Authorization: Bearer sk_test_1"), "{}", peticion);
    assert!(peticion.contains("unit_amount%5D=1500"), "{}", peticion);
    assert!(peticion.contains("success_url=https%3A%2F%2Ft.com%2Fok%3Fsesion%3D%7BCHECKOUT_SESSION_ID%7D"), "{}", peticion);
}

#[test]
fn json_lee_caminos_y_crea_texto_json() {
    let o = ejecutar(
        "json",
        "usar modulo \"json\"\nsea r = \"{\\\"u\\\": {\\\"n\\\": \\\"Año\\\", \\\"l\\\": [1, {\\\"x\\\": 2}]}, \\\"ok\\\": true}\"\nmostrar(leer_json(r, \"u.n\"))\nmostrar(leer_json(r, \"u.l[1].x\") + 1)\nmostrar(leer_json(r, \"u.l[1]\"))\nmostrar(leer_json(r, \"no.existe\"))\nmostrar(listar_claves_json(r, \"\"))\nmostrar(a_json([1, \"a\\\"b\", falso, nada]))\n",
    );
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "Año\n3\n{\"x\":2}\nnada\n[u, ok]\n[1,\"a\\\"b\",false,null]\n",
        "{}",
        todo(&o)
    );
    let mal = ejecutar("json_mal", "usar modulo \"json\"\nleer_json(\"{mal\", \"a\")\n");
    assert!(todo(&mal).contains("el JSON no es válido"), "{}", todo(&mal));
}

#[test]
fn archivos_excel_y_csv_funcionan() {
    let c = carpeta("excel");
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"archivos\"\nusar modulo \"excel\"\nescribir_archivo(\"a.txt\", \"hola\")\nmostrar(leer_archivo(\"a.txt\"))\nmostrar(listar_archivos(\".\"))\ncrear_excel(\"v.xlsx\", [[\"Mes\", \"Total\"], [\"año\", 1.5]])\ncrear_csv(\"v.csv\", [[\"a;b\", 2], [\"c\\\"d\", 3]])\nmostrar(leer_csv(\"v.csv\"))\n",
    );
    let o = deimon(&c, &["ejecutar", "programa.dmn"]);
    let salida = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(salida.starts_with("hola\n[a.txt, "), "{}", todo(&o));
    assert!(salida.contains("programa.dmn"), "{}", todo(&o));
    assert!(salida.ends_with("[[a;b, 2], [c\"d, 3]]\n"), "{}", todo(&o));
    // El .xlsx es un ZIP válido con la hoja dentro (lo comprueba Python, si está).
    let zip = Command::new("python3")
        .args(["-I", "-c", "import zipfile,sys; z=zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None; print(z.read('xl/worksheets/sheet1.xml').decode())"])
        .arg(c.join("v.xlsx"))
        .output();
    if let Ok(zip) = zip {
        let hoja = String::from_utf8_lossy(&zip.stdout);
        assert!(hoja.contains("<t xml:space=\"preserve\">año</t>") && hoja.contains("<v>1.5</v>"), "{}", hoja);
    }
}

#[test]
fn graficos_dan_svg_bien_formado_y_avisan_de_errores() {
    let o = ejecutar(
        "graficos",
        "usar modulo \"graficos\"\nsea g = grafico_tarta(\"A & B\", [\"x\", \"<y>\"], [1, 3])\nmostrar(medir(g) > 100)\nmostrar(g[0] + g[1] + g[2] + g[3])\ngrafico_barras(\"t\", [\"a\"], [1, 2])\n",
    );
    assert!(todo(&o).starts_with("verdadero\n<svg\n"), "{}", todo(&o));
    assert!(todo(&o).contains("mismo número de etiquetas que de valores"), "{}", todo(&o));
}

#[test]
fn sql_usa_parametros_y_no_se_deja_inyectar() {
    let o = ejecutar(
        "sql",
        "usar modulo \"sql\"\nejecutar_sql(\"t.db\", \"CREATE TABLE p (n TEXT, v REAL)\", [])\nmostrar(ejecutar_sql(\"t.db\", \"INSERT INTO p VALUES (?, ?)\", [\"x'); DROP TABLE p; --\", 2.5]))\nmostrar(consultar_sql(\"t.db\", \"SELECT n, v FROM p WHERE v > ?\", [1]))\nconsultar_sql(\"t.db\", \"SELECT * FROM p WHERE n = ?\", [])\n",
    );
    let texto = todo(&o);
    assert!(texto.starts_with("1\n[[x'); DROP TABLE p; --, 2.5]]\n"), "{}", texto);
    assert!(texto.contains("la orden tiene 1 '?' pero se pasaron 0 parámetros"), "{}", texto);
}
