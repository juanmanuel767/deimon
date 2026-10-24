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

fn hay_php() -> bool {
    Command::new("php").arg("-v").output().map(|o| o.status.success()).unwrap_or(false)
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

/// Servidor de imitación en 127.0.0.1 que responde a cada petición con lo que diga `responder`.
/// Devuelve el puerto libre y las peticiones recibidas, para poder revisarlas.
fn servidor_imitacion<F>(responder: F) -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>)
where
    F: Fn(&str) -> String + Send + 'static,
{
    use std::io::{Read, Write};
    let servidor = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = servidor.local_addr().unwrap().port();
    let recibidas = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let copia = recibidas.clone();
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
            copia.lock().unwrap().push(peticion.clone());
            let cuerpo = responder(&peticion);
            let _ = conexion.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", cuerpo.len(), cuerpo).as_bytes());
        }
    });
    (puerto, recibidas)
}

fn esperar_puerto(puerto: u16) {
    let limite = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::time::Instant::now() < limite {
        if std::net::TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("el servidor no aceptó conexiones en el puerto {}", puerto);
}

fn peticion_http(puerto: u16, texto: &str) -> String {
    use std::io::{Read, Write};
    let mut conexion = std::net::TcpStream::connect(("127.0.0.1", puerto)).expect("no se pudo conectar");
    conexion.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
    conexion.write_all(texto.as_bytes()).unwrap();
    let mut respuesta = String::new();
    let _ = conexion.read_to_string(&mut respuesta);
    respuesta
}

struct ServidorDeimon(std::process::Child);

impl Drop for ServidorDeimon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
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
fn formato_de_numeros_y_textos_como_php() {
    let fuente = "usar modulo \"texto\"\n\
escribir(formatear_numero(1234.5678, 2, \".\", \",\") + \"|\" + formatear_numero(-1234.567, 2, \",\", \".\") + \"\\n\")\n\
escribir(formatear_numero(1234, 0, \".\", \",\") + \"|\" + formatear_numero(-0.4, 2, \".\", \",\") + \"\\n\")\n\
escribir(rellenar_texto(\"hola\", 8, \" \", \"derecha\") + \"|\" + rellenar_texto(\"hola\", 8, \"-\", \"izquierda\") + \"|\" + rellenar_texto(\"hola\", 9, \"ab\", \"ambos\") + \"\\n\")\n\
escribir(primera_mayuscula(\"hola\") + \"\\n\")\n\
escribir(texto_completo([1, {\"x\": 1, \"y\": [verdadero, falso, nada, 4.5, \"hola\"]}, 2]))\n";
    let o = ejecutar("formato_php", fuente);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    let salida = String::from_utf8_lossy(&o.stdout).to_string();
    if hay_php() {
        let c = carpeta("formato_php_real");
        let php = escribir(
            &c,
            "formato.php",
            "<?php\necho number_format(1234.5678, 2, \".\", \",\") . \"|\" . number_format(-1234.567, 2, \",\", \".\") . \"\\n\";\necho number_format(1234, 0, \".\", \",\") . \"|\" . number_format(-0.4, 2, \".\", \",\") . \"\\n\";\necho str_pad(\"hola\", 8, \" \", STR_PAD_RIGHT) . \"|\" . str_pad(\"hola\", 8, \"-\", STR_PAD_LEFT) . \"|\" . str_pad(\"hola\", 9, \"ab\", STR_PAD_BOTH) . \"\\n\";\necho ucfirst(\"hola\") . \"\\n\";\nprint_r([1, [\"x\" => 1, \"y\" => [true, false, null, 4.5, \"hola\"]], 2]);\n",
        );
        let p = Command::new("php").arg(&php).output().unwrap();
        assert_eq!(salida, String::from_utf8_lossy(&p.stdout).to_string(), "PHP y Deimon deben escribir lo mismo");
    }
    let acentos = ejecutar(
        "formato_acentos",
        "usar modulo \"texto\"\nmostrar(rellenar_texto(\"ñandú\", 8, \".\", \"ambos\"))\nmostrar(primera_mayuscula(\"ñandú\"))\n",
    );
    assert_eq!(acentos.status.code(), Some(0), "{}", todo(&acentos));
    assert_eq!(String::from_utf8_lossy(&acentos.stdout), ".ñandú..\nÑandú\n");
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

#[test]
fn google_entra_con_la_cuenta_y_devuelve_nada_si_no_cuadra() {
    let (puerto, recibidas) = servidor_imitacion(|peticion| {
        if peticion.starts_with("POST") {
            if peticion.contains("code=malo") {
                r#"{"error": "invalid_grant"}"#.to_string()
            } else {
                r#"{"access_token": "ya29.token", "token_type": "Bearer"}"#.to_string()
            }
        } else {
            r#"{"sub": "123", "email": "ana@example.com", "email_verified": true, "name": "Ana", "picture": "https://fotos.example/ana.png"}"#.to_string()
        }
    });
    let c = carpeta("google");
    escribir(
        &c,
        "programa.dmn",
        &format!(
            "usar modulo \"google\"\nusar modulo \"texto\"\nruta \"/entrar\" {{\n    devolver redirigir_fuera(direccion_google(\"cliente\", \"http://127.0.0.1:{puerto}/volver\"))\n}}\nruta \"/volver\" {{\n    sea u = usuario_google(\"cliente\", \"secreto\", \"http://127.0.0.1:{puerto}/volver\")\n    si u == nada {{\n        devolver \"NADA\"\n    }}\n    devolver u[\"email\"] + \"|\" + u[\"nombre\"] + \"|\" + u[\"id\"]\n}}\nmostrar(\"malo=\" + visitar(\"/volver?state=falso&code=bueno\"))\nsea d1 = visitar(\"/entrar\")\nsea e1 = partir(partir(d1, \"state=\")[1], \"&\")[0]\nmostrar(\"sin-token=\" + visitar(\"/volver?state=\" + e1 + \"&code=malo\"))\nsea d2 = visitar(\"/entrar\")\nsea e2 = partir(partir(d2, \"state=\")[1], \"&\")[0]\nmostrar(visitar(\"/volver?state=\" + e2 + \"&code=bueno\"))\n",
            puerto = puerto
        ),
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "programa.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_GOOGLE_URL", format!("http://127.0.0.1:{}", puerto))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "malo=NADA\nsin-token=NADA\nana@example.com|Ana|123\n",
        "{}",
        todo(&o)
    );
    let peticiones = recibidas.lock().unwrap().clone();
    assert!(
        peticiones.iter().any(|p| p.starts_with("POST") && p.contains("grant_type=authorization_code")),
        "{:?}",
        peticiones
    );
    assert!(
        peticiones.iter().any(|p| p.starts_with("GET") && p.contains("Authorization: Bearer ya29.token")),
        "{:?}",
        peticiones
    );
}

#[test]
fn mercadopago_crea_el_pago_y_consulta_si_se_aprobo() {
    let (puerto, recibidas) = servidor_imitacion(|peticion| {
        if peticion.starts_with("POST") {
            r#"{"id": "pref_1", "sandbox_init_point": "https://pago.mercadopago/pref_1", "init_point": "https://pago.mercadopago/real"}"#.to_string()
        } else {
            r#"{"status": "approved", "status_detail": "accredited", "external_reference": "pedido-15", "transaction_amount": 29.9, "currency_id": "PEN"}"#.to_string()
        }
    });
    let c = carpeta("mercadopago");
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"mercadopago\"\nmostrar(crear_pago_mercadopago(\"TEST-123\", \"Pedido 15\", 29.9, \"PEN\", \"https://mitienda.com/pago\", \"pedido-15\"))\nsea p = consultar_pago_mercadopago(\"TEST-123\", \"12345\")\nmostrar(p[\"estado\"])\nmostrar(p[\"referencia\"])\nmostrar(p[\"importe\"])\nmostrar(p[\"moneda\"])\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "programa.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_MERCADOPAGO_URL", format!("http://127.0.0.1:{}", puerto))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "https://pago.mercadopago/pref_1\napproved\npedido-15\n29.9\nPEN\n",
        "{}",
        todo(&o)
    );
    let peticiones = recibidas.lock().unwrap().clone();
    let creacion = peticiones
        .iter()
        .find(|p| p.starts_with("POST") && p.contains("/checkout/preferences"))
        .expect("no se pidió crear el pago");
    assert!(creacion.contains("Authorization: Bearer TEST-123"), "{}", creacion);
    assert!(creacion.contains("\"unit_price\":29.9"), "{}", creacion);
    assert!(creacion.contains("\"currency_id\":\"PEN\""), "{}", creacion);
    assert!(creacion.contains("\"external_reference\":\"pedido-15\""), "{}", creacion);
    let consulta = peticiones
        .iter()
        .find(|p| p.starts_with("GET") && p.contains("/v1/payments/12345"))
        .expect("no se consultó el pago");
    assert!(consulta.contains("Authorization: Bearer TEST-123"), "{}", consulta);
}

#[test]
fn paypal_crea_el_pedido_y_lo_cobra() {
    let (puerto, recibidas) = servidor_imitacion(|peticion| {
        if peticion.contains("/v1/oauth2/token") {
            r#"{"access_token": "A21.token", "token_type": "Bearer"}"#.to_string()
        } else if peticion.contains("/capture") {
            r#"{"id": "ORDER1", "status": "COMPLETED"}"#.to_string()
        } else {
            r#"{"id": "ORDER1", "status": "CREATED", "links": [{"rel": "approve", "href": "https://www.paypal.com/checkoutnow?token=ORDER1"}]}"#.to_string()
        }
    });
    let c = carpeta("paypal");
    escribir(
        &c,
        "programa.dmn",
        "usar modulo \"paypal\"\nmostrar(crear_pago_paypal(\"cliente\", \"secret\", 29.9, \"USD\", \"https://mitienda.com/paypal\", \"https://mitienda.com/carrito\"))\nmostrar(cobrar_pago_paypal(\"cliente\", \"secret\", \"ORDER1\"))\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "programa.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_PAYPAL_URL", format!("http://127.0.0.1:{}", puerto))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "https://www.paypal.com/checkoutnow?token=ORDER1\nverdadero\n",
        "{}",
        todo(&o)
    );
    let peticiones = recibidas.lock().unwrap().clone();
    let token = peticiones
        .iter()
        .find(|p| p.contains("/v1/oauth2/token"))
        .expect("no se pidió el token");
    assert!(token.contains("Authorization: Basic Y2xpZW50ZTpzZWNyZXQ="), "{}", token);
    let pedido = peticiones
        .iter()
        .find(|p| p.contains("/v2/checkout/orders") && !p.contains("/capture"))
        .expect("no se creó el pedido");
    assert!(pedido.contains("Authorization: Bearer A21.token"), "{}", pedido);
    assert!(pedido.contains("\"currency_code\":\"USD\""), "{}", pedido);
    assert!(pedido.contains("\"value\":\"29.90\""), "{}", pedido);
    assert!(pedido.contains("\"return_url\":\"https://mitienda.com/paypal\""), "{}", pedido);
}

#[test]
fn whatsapp_envia_mensajes_y_comprueba_la_firma_del_aviso() {
    let (puerto, recibidas) = servidor_imitacion(|_| {
        r#"{"messaging_product": "whatsapp", "messages": [{"id": "wamid.ABC"}]}"#.to_string()
    });
    let c = carpeta("whatsapp");
    escribir(
        &c,
        "envios.dmn",
        "usar modulo \"whatsapp\"\nmostrar(enviar_whatsapp(\"token\", \"123\", \"51987654321\", \"Tu pedido salió\"))\nmostrar(enviar_plantilla_whatsapp(\"token\", \"123\", \"51987654321\", \"hello_world\", \"es\"))\nruta \"/aviso\" {\n    sea r = verificar_whatsapp(\"mi-clave\")\n    si r == nada {\n        devolver \"NO\"\n    }\n    devolver r\n}\nmostrar(visitar(\"/aviso?hub.mode=subscribe&hub.verify_token=mi-clave&hub.challenge=reto_123\"))\nmostrar(visitar(\"/aviso?hub.mode=subscribe&hub.verify_token=otra&hub.challenge=reto_123\"))\n",
    );
    let o = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "envios.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_WHATSAPP_URL", format!("http://127.0.0.1:{}", puerto))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "wamid.ABC\nwamid.ABC\nreto_123\nNO\n",
        "{}",
        todo(&o)
    );
    let peticiones = recibidas.lock().unwrap().clone();
    assert_eq!(peticiones.len(), 2, "{:?}", peticiones);
    assert!(peticiones.iter().all(|p| p.contains("Authorization: Bearer token")), "{:?}", peticiones);
    assert!(peticiones[0].contains("\"type\":\"text\""), "{}", peticiones[0]);
    assert!(peticiones[0].contains("\"body\":\"Tu pedido salió\""), "{}", peticiones[0]);
    assert!(peticiones[1].contains("\"type\":\"template\""), "{}", peticiones[1]);
    assert!(peticiones[1].contains("\"name\":\"hello_world\""), "{}", peticiones[1]);
    assert!(peticiones[1].contains("\"code\":\"es\""), "{}", peticiones[1]);

    // El aviso (webhook) de Meta: la firma HMAC-SHA256 del cuerpo con el secreto de la app.
    escribir(
        &c,
        "aviso.dmn",
        "usar modulo \"whatsapp\"\nruta \"/aviso\" {\n    sea lista = mensajes_whatsapp(\"secreto_app\")\n    devolver lista[0][\"de\"] + \"|\" + lista[0][\"nombre\"] + \"|\" + lista[0][\"texto\"] + \"|\" + lista[0][\"id\"]\n}\nservir(8000)\n",
    );
    let libre = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto_web = libre.local_addr().unwrap().port();
    drop(libre);
    let proceso = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["ejecutar", "aviso.dmn"])
        .current_dir(&c)
        .env("TMPDIR", &c)
        .env("DEIMON_PUERTO", puerto_web.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _servidor = ServidorDeimon(proceso);
    esperar_puerto(puerto_web);
    let cuerpo = r#"{"object":"whatsapp_business_account","entry":[{"changes":[{"value":{"contacts":[{"profile":{"name":"Ana"}}],"messages":[{"from":"51987654321","id":"wamid.IN","type":"text","text":{"body":"Hola"}}]}}]}]}"#;
    let firma = "1f7eeaa0dc27f6e6646c70ef886f172c4ab8a0619818070dbdfd9c27a86b96c9";
    let falsa = format!(
        "POST /aviso HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-Hub-Signature-256: sha256={}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        "0".repeat(64),
        cuerpo.len(),
        cuerpo
    );
    let mala = peticion_http(puerto_web, &falsa);
    assert!(mala.starts_with("HTTP/1.1 500"), "{}", mala);
    let buena = format!(
        "POST /aviso HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-Hub-Signature-256: sha256={}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        firma,
        cuerpo.len(),
        cuerpo
    );
    let respuesta = peticion_http(puerto_web, &buena);
    assert!(respuesta.starts_with("HTTP/1.1 200"), "{}", respuesta);
    assert!(respuesta.contains("51987654321|Ana|Hola|wamid.IN"), "{}", respuesta);
}

#[test]
fn fechas_completas_bisiestos_y_formato_en_espanol() {
    let fuente = "usar modulo \"tiempo\"\n\
mostrar(es_bisiesto(2024))\n\
mostrar(es_bisiesto(1900))\n\
mostrar(es_bisiesto(2000))\n\
mostrar(dias_del_mes(2024, 2))\n\
mostrar(dias_del_mes(2026, 2))\n\
mostrar(crear_fecha(2024, 2, 29, 12, 0, 0))\n\
mostrar(crear_fecha(1969, 7, 20, 20, 17, 0))\n\
mostrar(formatear_fecha(crear_fecha(2026, 1, 5, 13, 5, 9), \"Y-m-d H:i:s\", \"UTC\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 1, 5, 13, 5, 9), \"l j \\\\d\\\\e F \\\\d\\\\e Y, D, M, N w z t L y A a g h U\", \"UTC\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 1, 5, 0, 5, 9), \"G:i:s A a g h\", \"UTC\"))\n\
mostrar(leer_fecha(\"2026-03-29\"))\n\
mostrar(leer_fecha(\"2026-03-29 01:30\"))\n\
mostrar(leer_fecha(\"2026-03-29T01:30:00Z\") == crear_fecha(2026, 3, 29, 1, 30, 0))\n\
mostrar(leer_fecha(\"2026-03-29T01:30:00+01:00\") == crear_fecha(2026, 3, 29, 0, 30, 0))\n\
mostrar(leer_fecha(\"29/03/2026\") == crear_fecha(2026, 3, 29, 0, 0, 0))\n\
mostrar(leer_fecha(\"2026-02-31\") == nada)\n";
    let o = ejecutar("fechas_basicas", fuente);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "verdadero\nfalso\nverdadero\n29\n28\n1709208000\n-14182980\n\
2026-01-05 13:05:09\n\
lunes 5 de enero de 2026, Mon, ene, 1 1 4 31 0 26 PM pm 1 01 1767618309\n\
0:05:09 AM am 12 12\n\
1774742400\n\
1774747800\nverdadero\nverdadero\nverdadero\nverdadero\n"
    );
}

#[test]
fn fechas_zonas_horarias_y_cambio_de_hora() {
    let fuente = "usar modulo \"tiempo\"\n\
mostrar(formatear_fecha(crear_fecha(2026, 3, 29, 0, 30, 0), \"Y-m-d H:i:s\", \"Europe/Madrid\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 3, 29, 1, 30, 0), \"Y-m-d H:i:s\", \"Europe/Madrid\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 3, 28, 12, 0, 0), \"Y-m-d H:i:s\", \"Europe/Madrid\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 3, 29, 12, 0, 0), \"Y-m-d H:i:s\", \"America/Lima\"))\n\
mostrar(formatear_fecha(crear_fecha(2026, 7, 29, 12, 0, 0), \"Y-m-d H:i:s\", \"America/Lima\"))\n\
mostrar(formatear_fecha(1774746000, \"Y-m-d H:i:s\", \"Europe/Madrid\"))\n\
mostrar(formatear_fecha(1774746000, \"Y-m-d H:i:s\", \"America/Lima\"))\n\
mostrar(formatear_fecha(crear_fecha(1969, 7, 20, 20, 17, 0), \"Y-m-d H:i:s l\", \"UTC\"))\n\
mostrar(dia_de_la_semana(crear_fecha(2026, 3, 29, 0, 30, 0), \"Europe/Madrid\"))\n\
mostrar(dia_de_la_semana(0, \"UTC\"))\n\
mostrar(formatear_fecha(0, \"Y-m-d H:i:s\", \"-05:00\"))\n\
mostrar(formatear_fecha(0, \"Y-m-d H:i:s\", \"+01:00\"))\n";
    let o = ejecutar("fechas_zonas", fuente);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "2026-03-29 01:30:00\n\
2026-03-29 03:30:00\n\
2026-03-28 13:00:00\n\
2026-03-29 07:00:00\n\
2026-07-29 07:00:00\n\
2026-03-29 03:00:00\n\
2026-03-28 20:00:00\n\
1969-07-20 20:17:00 domingo\n\
7\n\
4\n\
1969-12-31 19:00:00\n\
1970-01-01 01:00:00\n"
    );
}

#[test]
fn sumar_y_diferencia_de_fechas_como_php() {
    let fuente = "usar modulo \"tiempo\"\n\
mostrar(formatear_fecha(sumar_tiempo(crear_fecha(2026, 1, 31, 12, 0, 0), 1, \"meses\"), \"Y-m-d H:i:s\", \"UTC\"))\n\
mostrar(formatear_fecha(sumar_tiempo(crear_fecha(2024, 1, 31, 12, 0, 0), 1, \"meses\"), \"Y-m-d H:i:s\", \"UTC\"))\n\
mostrar(formatear_fecha(sumar_tiempo(crear_fecha(2024, 2, 29, 0, 0, 0), 1, \"años\"), \"Y-m-d H:i:s\", \"UTC\"))\n\
mostrar(formatear_fecha(sumar_tiempo(crear_fecha(2026, 12, 31, 23, 0, 0), 1, \"dias\"), \"Y-m-d H:i:s\", \"UTC\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 3, 1, 0, 0, 0), crear_fecha(2026, 1, 31, 0, 0, 0), \"dias\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 3, 1, 0, 0, 0), crear_fecha(2026, 1, 31, 0, 0, 0), \"meses\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 1, 31, 0, 0, 0), crear_fecha(2026, 3, 1, 0, 0, 0), \"meses\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 3, 3, 0, 0, 0), crear_fecha(2026, 1, 31, 0, 0, 0), \"meses\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 3, 1, 0, 0, 0), crear_fecha(2026, 1, 31, 0, 0, 0), \"horas\"))\n\
mostrar(diferencia_fechas(crear_fecha(2026, 3, 1, 0, 0, 0), crear_fecha(2026, 1, 31, 0, 0, 0), \"semanas\"))\n";
    let o = ejecutar("fechas_sumar", fuente);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "2026-03-03 12:00:00\n\
2024-03-02 12:00:00\n\
2025-03-01 00:00:00\n\
2027-01-01 23:00:00\n\
29\n0\n-1\n1\n696\n4\n"
    );
}

#[test]
fn una_fecha_que_no_existe_da_un_error_claro() {
    let o = ejecutar(
        "fecha_inexistente",
        "usar modulo \"tiempo\"\nmostrar(crear_fecha(2026, 2, 31, 0, 0, 0))\n",
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(
        todo(&o).contains("el 31 de febrero de 2026 no existe (febrero tiene 28 días)"),
        "{}",
        todo(&o)
    );
    let o = ejecutar(
        "fecha_29_febrero",
        "usar modulo \"tiempo\"\nmostrar(crear_fecha(2023, 2, 29, 0, 0, 0))\n",
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(
        todo(&o).contains("el 29 de febrero de 2023 no existe (febrero tiene 28 días)"),
        "{}",
        todo(&o)
    );
}

#[test]
fn una_zona_horaria_que_falta_da_un_error_claro() {
    let o = ejecutar(
        "zona_falta",
        "usar modulo \"tiempo\"\nmostrar(formatear_fecha(0, \"Y\", \"America/NoExiste\"))\n",
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(
        todo(&o).contains("no se encontró la zona horaria 'America/NoExiste'"),
        "{}",
        todo(&o)
    );
}

/// Ejecuta una expresión del módulo `patrones` por línea y compara lo que muestra cada una.
fn patrones_casos(nombre: &str, casos: &[(&str, &str)]) {
    let mut fuente = String::from("usar modulo \"patrones\"\n");
    for (expresion, _) in casos {
        fuente.push_str(&format!("mostrar({})\n", expresion));
    }
    let o = ejecutar(nombre, &fuente);
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    let salida = String::from_utf8_lossy(&o.stdout).to_string();
    let lineas: Vec<&str> = salida.lines().collect();
    assert_eq!(lineas.len(), casos.len(), "{}", salida);
    for (linea, (expresion, esperado)) in lineas.iter().zip(casos) {
        assert_eq!(linea, esperado, "{}", expresion);
    }
}

#[test]
fn patrones_basicos_y_clases() {
    patrones_casos(
        "patrones_basicos",
        &[
            (r#"coincide_patron("hola mundo", "mundo")"#, "verdadero"),
            (r#"coincide_patron("hola mundo", "^mundo")"#, "falso"),
            (r#"coincide_patron("hola mundo", "mundo$")"#, "verdadero"),
            (r#"coincide_patron("abc", "a.c")"#, "verdadero"),
            (r#"coincide_patron("a.c", "a\\.c")"#, "verdadero"),
            (r#"coincide_patron("abc", "a\\.c")"#, "falso"),
            (r#"buscar_patron("pedido 42 y 7", "\\d+")"#, "[42]"),
            (r#"buscar_patron("sin numeros", "\\d+")"#, "nada"),
            (r#"buscar_patron("abc123", "[a-c]+")"#, "[abc]"),
            (r#"buscar_patron("abc123", "[^a-c]+")"#, "[123]"),
            (r#"buscar_patron("a-b_c", "[\\w-]+")"#, "[a-b_c]"),
            (r#"buscar_patron("x  y", "\\s+")"#, "[  ]"),
            (r#"buscar_patron("ab 12", "\\D+")"#, "[ab ]"),
            (r#"buscar_patron("ab 12", "\\S+")"#, "[ab]"),
            (r#"buscar_patron("ab 12", "\\W")"#, "[ ]"),
            (r#"buscar_patron("un gato y gatos", "\\bgato\\b")"#, "[gato]"),
            (r#"buscar_todos_patron("un gato y gatos", "\\bgato\\b")"#, "[[gato]]"),
            (r#"buscar_patron("a/b", "a\\/b")"#, "[a/b]"),
            (r#"buscar_patron("a\\b", "a\\\\b")"#, "[a\\b]"),
        ],
    );
}

#[test]
fn patrones_acentos_y_mayusculas() {
    patrones_casos(
        "patrones_acentos",
        &[
            (r#"buscar_patron("Canción de España", "[a-zñáéíóú]+$")"#, "[spaña]"),
            (r#"buscar_patron("mañana", "ma.ana")"#, "[mañana]"),
            (r#"buscar_patron("áéí ñu", "\\w+")"#, "[áéí]"),
            (r#"buscar_patron("ÑANDÚ", "(?i)ñandú")"#, "[ÑANDÚ]"),
            (r#"buscar_patron("Hola", "(?i)^HOLA$")"#, "[Hola]"),
            (r#"buscar_patron("Hola", "^HOLA$")"#, "nada"),
            (r#"buscar_patron("ABC", "(?i)[a-c]+")"#, "[ABC]"),
            (r#"reemplazar_patron("Año nuevo", "(?i)AÑO", "día")"#, "día nuevo"),
        ],
    );
}

#[test]
fn patrones_grupos_alternativas_y_cuantificadores() {
    patrones_casos(
        "patrones_grupos",
        &[
            (r#"buscar_patron("ana@x.es", "(\\w+)@(\\w+)")"#, "[ana@x, ana, x]"),
            (r#"buscar_patron("2024-05-17", "(\\d{4})-(\\d{2})-(\\d{2})")"#, "[2024-05-17, 2024, 05, 17]"),
            (r#"buscar_patron("gato", "(?:ga|pe)to")"#, "[gato]"),
            (r#"buscar_patron("perro", "(gato|perro)")"#, "[perro, perro]"),
            (r#"buscar_patron("ac", "a(b)?c")"#, "[ac, ]"),
            (r#"buscar_patron("aaa", "a{2}")"#, "[aa]"),
            (r#"buscar_patron("aaaa", "a{2,}")"#, "[aaaa]"),
            (r#"buscar_patron("aaaa", "a{1,3}")"#, "[aaa]"),
            (r#"buscar_patron("aaaa", "a{1,3}?")"#, "[a]"),
            (r#"buscar_patron("<b>x</b><b>y</b>", "<b>(.*)</b>")"#, "[<b>x</b><b>y</b>, x</b><b>y]"),
            (r#"buscar_patron("<b>x</b><b>y</b>", "<b>(.*?)</b>")"#, "[<b>x</b>, x]"),
            (r#"buscar_patron("aaa", "a+?")"#, "[a]"),
            (r#"buscar_patron("abc", "ab??c")"#, "[abc]"),
            (r#"buscar_patron("abab", "(ab)+")"#, "[abab, ab]"),
            (r#"buscar_patron("a{b", "a{b")"#, "[a{b]"),
            (r#"buscar_patron("foobar", "foo|foobar")"#, "[foo]"),
            (r#"buscar_patron("", "^$")"#, "[]"),
            (r#"buscar_todos_patron("a1 b22 c333", "([a-z])(\\d+)")"#, "[[a1, a, 1], [b22, b, 22], [c333, c, 333]]"),
            (r#"buscar_columnas_patron("a1 b22", "([a-z])(\\d+)")"#, "[[a1, b22], [a, b], [1, 22]]"),
            (r#"buscar_columnas_patron("zzz", "(\\d)")"#, "[[], []]"),
        ],
    );
}

#[test]
fn patrones_reemplazar_y_partir() {
    patrones_casos(
        "patrones_reemplazar",
        &[
            (r#"reemplazar_patron("2024-05-17", "(\\d+)-(\\d+)-(\\d+)", "$3/$2/$1")"#, "17/05/2024"),
            (r#"reemplazar_patron("a b  c", "\\s+", "_")"#, "a_b_c"),
            (r#"reemplazar_patron("hola", "o", "$0$0")"#, "hoola"),
            (r#"reemplazar_patron("hola", "(o)", "${1}0")"#, "ho0la"),
            (r#"reemplazar_patron("hola", "(o)", "$5x")"#, "hxla"),
            (r#"reemplazar_patron("precio 5", "\\d", "$")"#, "precio $"),
            (r#"reemplazar_patron("abc", "x*", "-")"#, "-a-b-c-"),
            (r#"partir_patron("a, b;c", "[,;]\\s*")"#, "[a, b, c]"),
            (r#"partir_patron("abc", "")"#, "[a, b, c]"),
            (r#"partir_patron("a1b22c", "\\d+")"#, "[a, b, c]"),
            (r#"partir_patron("sin", ",")"#, "[sin]"),
            (r#"partir_patron(",a,", ",")"#, "[, a, ]"),
        ],
    );
}

#[test]
fn patrones_con_texto_largo_no_revienta() {
    let o = ejecutar(
        "patrones_largo",
        "usar modulo \"patrones\"\nusar modulo \"texto\"\nsea t = repetir(\"ab \", 10000)\nmostrar(medir(buscar_todos_patron(t, \"\\\\w+\")))\nmostrar(medir(buscar_patron(t, \"(?:ab )*x|^(?:ab )+\")[0]))\n",
    );
    assert_eq!(o.status.code(), Some(0), "{}", todo(&o));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "10000\n30000\n");
}

#[test]
fn patrones_para_con_un_patron_que_tardaria_una_eternidad() {
    let o = ejecutar(
        "patrones_limite",
        "usar modulo \"patrones\"\nmostrar(coincide_patron(\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac\", \"(a+)+b\"))\n",
    );
    assert_eq!(o.status.code(), Some(1), "{}", todo(&o));
    assert!(todo(&o).contains("demasiado complicado"), "{}", todo(&o));
}

#[test]
fn patrones_mal_escritos_dan_un_error_claro_con_la_posicion() {
    for (patron, esperado) in [
        ("(abc", "en la posición 1: falta cerrar este paréntesis con )"),
        ("abc)", "en la posición 4: este paréntesis ) no tiene su pareja ("),
        ("[abc", "en la posición 1: falta cerrar esta clase con ]"),
        ("a**", "en la posición 3: no se puede repetir dos veces"),
        ("*a", "en la posición 1: no hay nada que repetir"),
        ("a{3,2}", "en la posición 2: en {n,m}"),
        ("[z-a]", "en la posición 2: el rango está al revés"),
        ("ab\\\\", "en la posición 3: el patrón no puede acabar en"),
        ("(a)\\\\1", "las referencias a grupos"),
        ("a(?=b)", "en la posición 2: mirar adelante o atrás no está soportado"),
        ("\\\\q", "el escape \\q no existe"),
        ("(?i)ab(", "en la posición 7: falta cerrar este paréntesis"),
    ] {
        let o = ejecutar(
            "patrones_error",
            &format!("usar modulo \"patrones\"\nmostrar(coincide_patron(\"abc\", \"{}\"))\n", patron),
        );
        assert_eq!(o.status.code(), Some(1), "{}: {}", patron, todo(&o));
        assert!(todo(&o).contains("patrón mal escrito"), "{}: {}", patron, todo(&o));
        assert!(todo(&o).contains(esperado), "{}: {}", patron, todo(&o));
    }
}
