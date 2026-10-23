use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_plantillas_mysql")
        .join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

fn ejecutar(archivo: &PathBuf) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("ejecutar")
        .arg(archivo)
        .env("DEIMON_DATOS", archivo.parent().unwrap().join("datos"))
        .output()
        .unwrap()
}

fn texto(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stdout).to_string()
}

fn errores(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stderr).to_string()
}

#[test]
fn las_plantillas_rellenan_repiten_eligen_y_protegen() {
    let c = carpeta("plantillas");
    fs::create_dir_all(c.join("vistas")).unwrap();
    fs::write(
        c.join("vistas").join("lista.html"),
        "<h1>{{ usuario.nombre }}</h1>{# nota #}{% si productos %}{% para p en productos %}[{{ p.nombre }}{% si p.oferta %}!{% fin %}]{% fin %}{% sino %}vacío{% fin %}|{{ peligro }}|{{ ya }}|{{{ crudo }}}",
    )
    .unwrap();
    let archivo = c.join("programa.dmn");
    fs::write(
        &archivo,
        "mostrar(plantilla(\"vistas/lista.html\", {\"usuario\": {\"nombre\": \"Ana\"}, \"productos\": [{\"nombre\": \"Taza\", \"oferta\": verdadero}, {\"nombre\": \"Gorra\", \"oferta\": falso}], \"peligro\": \"<b>&\", \"ya\": \"a &amp; b\", \"crudo\": \"<i>\"}))\nmostrar(plantilla(\"vistas/lista.html\", {\"usuario\": {\"nombre\": \"Eva\"}, \"productos\": [], \"peligro\": \"\", \"ya\": \"\", \"crudo\": \"\"}))\n",
    )
    .unwrap();
    // Se ejecuta desde otra carpeta: la plantilla va dentro del programa.
    let salida = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("ejecutar")
        .arg(&archivo)
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(salida.status.success(), "{}", errores(&salida));
    assert_eq!(
        texto(&salida),
        "<h1>Ana</h1>[Taza!][Gorra]|&lt;b&gt;&amp;|a &amp; b|<i>\n<h1>Eva</h1>vacío|||\n"
    );
    let _ = fs::remove_dir_all(&c);
}

#[test]
fn una_plantilla_que_falta_o_esta_mal_se_avisa_al_traducir() {
    let c = carpeta("plantillas_mal");
    fs::write(c.join("mala.html"), "{% para x en lista %}sin cerrar").unwrap();
    let archivo = c.join("programa.dmn");
    fs::write(&archivo, "mostrar(plantilla(\"mala.html\", {}))\nmostrar(plantilla(\"no-existe.html\", {}))\n").unwrap();
    let salida = ejecutar(&archivo);
    assert!(!salida.status.success());
    let error = errores(&salida);
    assert!(error.contains("E024") && error.contains("falta {% fin %}"), "{}", error);
    assert!(error.contains("no se encontró la plantilla 'no-existe.html'"), "{}", error);
    let _ = fs::remove_dir_all(&c);
}

/// MariaDB privada en un puerto libre; None si el ordenador no tiene MariaDB.
struct MariaDb {
    proceso: std::process::Child,
    enchufe: PathBuf,
    puerto: u16,
}

impl Drop for MariaDb {
    fn drop(&mut self) {
        let _ = self.proceso.kill();
        let _ = self.proceso.wait();
        let _ = fs::remove_file(&self.enchufe);
    }
}

fn arrancar_mariadb(c: &PathBuf) -> Option<MariaDb> {
    if Command::new("mariadb-install-db").arg("--help").output().is_err() || Command::new("mariadbd").arg("--version").output().is_err() {
        return None;
    }
    let datos = c.join("mariadb");
    let instalar = Command::new("mariadb-install-db")
        .args(["--no-defaults", "--auth-root-authentication-method=normal", "--skip-test-db"])
        .arg(format!("--datadir={}", datos.display()))
        .output()
        .ok()?;
    if !instalar.status.success() {
        return None;
    }
    let puerto = std::net::TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok()?.port();
    // La ruta del enchufe no puede pasar de 107 letras: va en /tmp.
    let enchufe = PathBuf::from(format!("/tmp/dm-prueba-{}.sock", std::process::id()));
    let proceso = Command::new("mariadbd")
        .args(["--no-defaults", "--bind-address=127.0.0.1"])
        .arg(format!("--datadir={}", datos.display()))
        .arg(format!("--socket={}", enchufe.display()))
        .arg(format!("--port={}", puerto))
        .arg(format!("--log-error={}", c.join("error.log").display()))
        .spawn()
        .ok()?;
    let servidor = MariaDb { proceso, enchufe: enchufe.clone(), puerto };
    let limite = Instant::now() + Duration::from_secs(60);
    while !enchufe.exists() {
        if Instant::now() > limite {
            return None;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let preparar = Command::new("mariadb")
        .args(["--no-defaults", "-u", "root"])
        .arg(format!("--socket={}", enchufe.display()))
        .arg("-e")
        .arg("CREATE DATABASE tienda; CREATE USER 'deimon'@'%' IDENTIFIED BY 'cl@ve:prueba'; GRANT ALL ON tienda.* TO 'deimon'@'%';")
        .output()
        .ok()?;
    if !preparar.status.success() {
        return None;
    }
    Some(servidor)
}

#[test]
fn mysql_guarda_lee_y_no_se_deja_inyectar() {
    let c = carpeta("mysql");
    let servidor = match arrancar_mariadb(&c) {
        Some(servidor) => servidor,
        None => return, // sin MariaDB en este ordenador
    };
    let archivo = c.join("programa.dmn");
    fs::write(
        &archivo,
        format!(
            "usar modulo \"mysql\"\nsea c = \"mysql://deimon:cl%40ve%3Aprueba@127.0.0.1:{}/tienda\"\nejecutar_mysql(c, \"CREATE TABLE p (id INT AUTO_INCREMENT PRIMARY KEY, nombre VARCHAR(100), precio DECIMAL(8,2), nota TEXT NULL)\", [])\nmostrar(ejecutar_mysql(c, \"INSERT INTO p (nombre, precio) VALUES (?, ?), (?, ?)\", [\"Taza\", 8.5, \"Año ñ\", 12]))\nmostrar(ultimo_id_mysql(c))\nejecutar_mysql(c, \"INSERT INTO p (nombre, precio, nota) VALUES (?, ?, ?)\", [\"O'Brien \\\\ \\\"x\\\" ?\", 1, nada])\nmostrar(consultar_mysql(c, \"SELECT nombre, precio, nota FROM p WHERE precio < ? ORDER BY id\", [100]))\nmostrar(medir(consultar_mysql(c, \"SELECT * FROM p WHERE nombre = ?\", [\"x' OR '1'='1\"])))\nmostrar(consultar_mysql(c, \"SELECT '?' AS signo -- ?\", []))\nmostrar(consultar_mysql(\"mysql://deimon:cl%40ve%3Aprueba@localhost/tienda?socket={}\", \"SELECT COUNT(*) AS n FROM p\", []))\nejecutar_mysql(c, \"SET SESSION sql_mode = 'NO_BACKSLASH_ESCAPES'\", [])\nmostrar(medir(consultar_mysql(c, \"SELECT * FROM p WHERE nombre = ?\", [\"\\\\' OR 1=1 -- \"])))\n",
            servidor.puerto,
            servidor.enchufe.display()
        ),
    )
    .unwrap();
    let salida = ejecutar(&archivo);
    assert!(salida.status.success(), "{}", errores(&salida));
    assert_eq!(
        texto(&salida),
        "2\n1\n[{nombre: Taza, precio: 8.5, nota: nada}, {nombre: Año ñ, precio: 12, nota: nada}, {nombre: O'Brien \\ \"x\" ?, precio: 1, nota: nada}]\n0\n[{signo: ?}]\n[{n: 3}]\n0\n"
    );

    let mal = c.join("mal.dmn");
    fs::write(&mal, format!("usar modulo \"mysql\"\nmostrar(consultar_mysql(\"mysql://deimon:mala@127.0.0.1:{}/tienda\", \"SELECT 1\", []))\n", servidor.puerto)).unwrap();
    let salida = ejecutar(&mal);
    assert!(!salida.status.success());
    assert!(errores(&salida).contains("(1045)"), "{}", errores(&salida));
    drop(servidor);
    let _ = fs::remove_dir_all(&c);
}
