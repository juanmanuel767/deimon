use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

fn carpeta(nombre: &str) -> PathBuf {
    let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tmp")
        .join("pruebas_base_de_datos")
        .join(format!("{}_{}", std::process::id(), nombre));
    let _ = fs::remove_dir_all(&carpeta);
    fs::create_dir_all(&carpeta).unwrap();
    carpeta
}

/// Compila un programa una vez y devuelve la ruta del ejecutable.
fn compilar(c: &Path, nombre: &str, fuente: &str) -> PathBuf {
    let programa = c.join(format!("{}.dmn", nombre));
    fs::write(&programa, fuente).unwrap();
    let rust = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .args(["traducir", programa.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(rust.status.success(), "{}", String::from_utf8_lossy(&rust.stderr));
    let fuente_rust = c.join(format!("{}.rs", nombre));
    fs::write(&fuente_rust, &rust.stdout).unwrap();
    let binario = c.join(nombre);
    let compilado = Command::new("rustc")
        .args(["-O", "--crate-name", "prog"])
        .arg(&fuente_rust)
        .arg("-o")
        .arg(&binario)
        .output()
        .unwrap();
    assert!(compilado.status.success(), "{}", String::from_utf8_lossy(&compilado.stderr));
    binario
}

fn correr(binario: &Path, datos: &Path) -> Output {
    Command::new(binario).env("DEIMON_DATOS", datos).output().unwrap()
}

fn salida(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

#[test]
fn dos_programas_a_la_vez_no_pierden_incrementos() {
    let c = carpeta("a_la_vez");
    let datos = c.join("datos");
    let binario = compilar(
        &c,
        "sumar",
        "sea i = 0\nmientras i < 200 {\n    incrementar(\"contadores\", \"total\", 1)\n    i = i + 1\n}\n",
    );
    let hijos: Vec<_> = (0..4)
        .map(|_| Command::new(&binario).env("DEIMON_DATOS", &datos).spawn().unwrap())
        .collect();
    for mut hijo in hijos {
        assert!(hijo.wait().unwrap().success());
    }
    let leer = compilar(&c, "leer", "mostrar(leer(\"contadores\", \"total\"))\n");
    assert_eq!(String::from_utf8_lossy(&correr(&leer, &datos).stdout), "800\n");
}

#[test]
fn una_escritura_cortada_a_medias_no_pierde_lo_anterior() {
    let c = carpeta("corte");
    let datos = c.join("datos");
    let guardar = compilar(&c, "guardar", "guardar(\"cosas\", \"a\", 1)\nguardar(\"cosas\", \"b\", \"dos\")\n");
    assert!(correr(&guardar, &datos).status.success());
    // Simula un corte de luz a mitad de una línea.
    let registro = datos.join("cosas.registro");
    let mut contenido = fs::read(&registro).unwrap();
    contenido.extend_from_slice(b"s\tc\tn\t3");
    fs::write(&registro, contenido).unwrap();
    let leer = compilar(
        &c,
        "leer",
        "mostrar(leer(\"cosas\", \"a\"))\nmostrar(leer(\"cosas\", \"b\"))\nmostrar(leer(\"cosas\", \"c\"))\nguardar(\"cosas\", \"d\", verdadero)\nmostrar(contar(\"cosas\"))\n",
    );
    let o = correr(&leer, &datos);
    assert_eq!(String::from_utf8_lossy(&o.stdout), "1\ndos\nnada\n3\n", "{}", salida(&o));
}

#[test]
fn un_registro_danado_a_mitad_avisa_en_vez_de_inventar_datos() {
    let c = carpeta("danado");
    let datos = c.join("datos");
    let guardar = compilar(&c, "guardar", "guardar(\"cosas\", \"a\", 1)\nguardar(\"cosas\", \"b\", 2)\n");
    assert!(correr(&guardar, &datos).status.success());
    let registro = datos.join("cosas.registro");
    let texto = fs::read_to_string(&registro).unwrap().replacen("\ta\tn\t1\t", "\ta\tn\t9\t", 1);
    fs::write(&registro, texto).unwrap();
    let leer = compilar(&c, "leer", "mostrar(leer(\"cosas\", \"b\"))\n");
    let o = correr(&leer, &datos);
    assert_eq!(o.status.code(), Some(1));
    assert!(salida(&o).contains("dañado"), "{}", salida(&o));
}

#[test]
fn muchos_cambios_se_compactan_y_son_rapidos() {
    let c = carpeta("compactar");
    let datos = c.join("datos");
    let binario = compilar(
        &c,
        "muchos",
        "sea i = 0\nmientras i < 5000 {\n    guardar(\"cosas\", \"x\", i)\n    i = i + 1\n}\nmostrar(leer(\"cosas\", \"x\"))\nmostrar(contar(\"cosas\"))\n",
    );
    let inicio = Instant::now();
    let o = correr(&binario, &datos);
    let segundos = inicio.elapsed().as_secs_f64();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "4999\n1\n", "{}", salida(&o));
    let lineas = fs::read_to_string(datos.join("cosas.registro")).unwrap().lines().count();
    assert!(lineas < 2100, "el registro no se compactó: {} líneas", lineas);
    assert!(segundos < 30.0, "5000 escrituras tardaron {} s", segundos);
}

#[test]
fn los_datos_del_formato_antiguo_se_migran_solos() {
    let c = carpeta("migrar");
    let datos = c.join("datos");
    fs::create_dir_all(&datos).unwrap();
    fs::write(datos.join("viejo.datos"), "nombre\tt\tAna\nedad\tn\t30\n").unwrap();
    let leer = compilar(&c, "leer", "mostrar(leer(\"viejo\", \"nombre\"))\nmostrar(leer(\"viejo\", \"edad\") + 1)\n");
    assert_eq!(String::from_utf8_lossy(&correr(&leer, &datos).stdout), "Ana\n31\n");
    assert!(datos.join("viejo.datos.migrado").exists());
    assert!(datos.join("viejo.registro").exists());
}

#[test]
fn los_usuarios_no_se_pueden_leer_con_leer() {
    let c = carpeta("usuarios_ocultos");
    let datos = c.join("datos");
    let binario = compilar(
        &c,
        "usuarios",
        "mostrar(crear_usuario(\"ana\", \"secreta12\"))\nmostrar(comprobar_clave(\"ana\", \"secreta12\"))\nmostrar(comprobar_clave(\"ana\", \"otra\"))\nmostrar(leer(\".usuarios\", \"ana\"))\n",
    );
    let o = correr(&binario, &datos);
    assert_eq!(o.status.code(), Some(1), "{}", salida(&o));
    assert!(salida(&o).starts_with("verdadero\nverdadero\nfalso\n"), "{}", salida(&o));
    assert!(salida(&o).contains("el nombre de la colección solo puede tener"), "{}", salida(&o));
    let registro = fs::read_to_string(datos.join(".usuarios.registro")).unwrap();
    assert!(!registro.contains("secreta12"));
    assert!(registro.contains("pbkdf2-sha256$100000$"));
}

#[test]
fn el_cifrado_de_claves_coincide_con_los_valores_oficiales() {
    // Extrae SHA-256 y PBKDF2 del código que genera Deimon y los compara con RFC 7914.
    let generador = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generador.rs")).unwrap();
    let funcion = |nombre: &str| {
        let inicio = generador.find(&format!("fn {}(", nombre)).unwrap();
        let fin = generador[inicio..].find("\n}\n").unwrap() + inicio + 3;
        generador[inicio..fin].to_string()
    };
    let mut codigo: String = ["sha256", "hmac_sha256", "pbkdf2_sha256", "hexadecimal"]
        .iter()
        .map(|nombre| funcion(nombre))
        .collect::<Vec<_>>()
        .join("\n");
    codigo.push_str("fn main() { println!(\"{}\", hexadecimal(&sha256(b\"abc\"))); println!(\"{}\", hexadecimal(&pbkdf2_sha256(b\"password\", b\"salt\", 4096))); }\n");
    let c = carpeta("vectores");
    fs::write(c.join("vectores.rs"), codigo).unwrap();
    let compilado = Command::new("rustc").args(["-O"]).arg(c.join("vectores.rs")).arg("-o").arg(c.join("vectores")).output().unwrap();
    assert!(compilado.status.success(), "{}", String::from_utf8_lossy(&compilado.stderr));
    let o = Command::new(c.join("vectores")).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\nc5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a\n"
    );
}

#[test]
fn cinco_fallos_bloquean_la_cuenta_aunque_luego_acierte() {
    let c = carpeta("bloqueo");
    let datos = c.join("datos");
    let binario = compilar(
        &c,
        "bloqueo",
        "crear_usuario(\"ana\", \"secreta12\")\nsea i = 0\nmientras i < 5 {\n    comprobar_clave(\"ana\", \"mala\")\n    i = i + 1\n}\nmostrar(cuenta_bloqueada(\"ana\"))\nmostrar(comprobar_clave(\"ana\", \"secreta12\"))\nmostrar(cuenta_bloqueada(\"luis\"))\n",
    );
    let o = correr(&binario, &datos);
    assert_eq!(String::from_utf8_lossy(&o.stdout), "verdadero\nfalso\nfalso\n", "{}", salida(&o));
}

#[test]
fn se_puede_cambiar_y_recuperar_la_clave() {
    let c = carpeta("recuperar");
    let datos = c.join("datos");
    let binario = compilar(
        &c,
        "recuperar",
        "crear_usuario(\"ana\", \"secreta12\")\nmostrar(cambiar_clave(\"ana\", \"mala\", \"nueva1234\"))\nmostrar(cambiar_clave(\"ana\", \"secreta12\", \"corta\"))\nmostrar(cambiar_clave(\"ana\", \"secreta12\", \"nueva1234\"))\nmostrar(comprobar_clave(\"ana\", \"nueva1234\"))\nmostrar(crear_codigo_recuperacion(\"nadie\"))\nsea codigo = crear_codigo_recuperacion(\"ana\")\nmostrar(restablecer_clave(\"ana\", \"inventado\", \"otra12345\"))\nmostrar(restablecer_clave(\"ana\", codigo, \"otra12345\"))\nmostrar(restablecer_clave(\"ana\", codigo, \"tercera123\"))\nmostrar(comprobar_clave(\"ana\", \"otra12345\"))\n",
    );
    let o = correr(&binario, &datos);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "falso\nfalso\nverdadero\nverdadero\nnada\nfalso\nverdadero\nfalso\nverdadero\n",
        "{}",
        salida(&o)
    );
    let fichas = fs::read_to_string(datos.join(".recuperacion.registro")).unwrap_or_default();
    assert!(!fichas.contains("otra12345"));
}

#[test]
fn se_pueden_guardar_listas_anidadas() {
    let c = carpeta("listas");
    let datos = c.join("datos");
    let guardar = compilar(&c, "guardar", "guardar(\"c\", \"l\", [1, \"a:b\", [verdadero, nada], \"año\"])\n");
    assert!(correr(&guardar, &datos).status.success());
    let leer = compilar(&c, "leer", "sea l = leer(\"c\", \"l\")\nmostrar(l)\nmostrar(medir(l[2]))\n");
    assert_eq!(String::from_utf8_lossy(&correr(&leer, &datos).stdout), "[1, a:b, [verdadero, nada], año]\n2\n");
}
