/// Valor de un campo del formulario (o de `?campo=` en la dirección), escapado como HTML.
fn campo(nombre: &Valor) -> Valor {
    let nombre = texto_argumento(nombre, "campo");
    PETICION.with(|peticion| {
        peticion
            .borrow()
            .campos
            .iter()
            .find(|(clave, _)| *clave == nombre)
            .map(|(_, valor)| Valor::Texto(Rc::new(escapar_html(valor))))
            .unwrap_or(Valor::Nada)
    })
}

/// El texto con los caracteres especiales de HTML protegidos.
fn proteger_html(valor: &Valor) -> Valor {
    Valor::Texto(Rc::new(escapar_html(&texto_de(valor))))
}

/// Verdadero si la ruta se está atendiendo porque se envió un formulario.
fn es_envio() -> Valor {
    Valor::Logico(PETICION.with(|peticion| peticion.borrow().envio))
}

/// Hace que el navegador vaya a otra página de este mismo sitio.
fn redirigir(camino: &Valor) -> Valor {
    let camino = texto_argumento(camino, "redirigir");
    if !camino.starts_with('/') || camino.starts_with("//") || camino.contains(['\r', '\n']) {
        fallo("redirigir() solo acepta caminos de este sitio que empiecen por '/', por ejemplo \"/inicio\"".to_string());
    }
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().redireccion = Some(camino));
    Valor::Texto(Rc::new(String::new()))
}

/// Redirige a otra web (pagar en Mercado Pago o PayPal, entrar con Google...). Solo https://,
/// para que nadie la use para mandar a la gente a sitios sin cifrar.
fn redirigir_fuera(direccion: &Valor) -> Valor {
    let direccion = texto_argumento(direccion, "redirigir_fuera");
    let local = direccion.starts_with("http://127.0.0.1") || direccion.starts_with("http://localhost");
    if !(direccion.starts_with("https://") || local) || direccion.contains(['\r', '\n', ' ']) {
        fallo("redirigir_fuera() solo acepta direcciones que empiecen por https://".to_string());
    }
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().redireccion = Some(direccion));
    Valor::Texto(Rc::new(String::new()))
}

fn preparar_peticion(envio: bool, campos: Vec<(String, String)>, galletas: Vec<(String, String)>) {
    PETICION.with(|peticion| {
        *peticion.borrow_mut() = Peticion {
            segura: false,
            envio,
            campos,
            galletas,
            subidas: Vec::new(),
            cuerpo: String::new(),
            cabeceras: Vec::new(),
        }
    });
    RESPUESTA.with(|respuesta| {
        *respuesta.borrow_mut() = Respuesta {
            capturando: true,
            ..Respuesta::default()
        }
    });
}

fn tomar_respuesta() -> Respuesta {
    RESPUESTA.with(|respuesta| std::mem::take(&mut *respuesta.borrow_mut()))
}

// ---------------------------------------------------------------------------
// Contraseñas: PBKDF2-HMAC-SHA256 con sal aleatoria (sin dependencias).
// ---------------------------------------------------------------------------

fn sha256(datos: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut mensaje = datos.to_vec();
    let bits = (datos.len() as u64).wrapping_mul(8);
    mensaje.push(0x80);
    while mensaje.len() % 64 != 56 {
        mensaje.push(0);
    }
    mensaje.extend_from_slice(&bits.to_be_bytes());
    for bloque in mensaje.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([bloque[4 * i], bloque[4 * i + 1], bloque[4 * i + 2], bloque[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    let mut salida = [0u8; 32];
    for i in 0..8 {
        salida[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    salida
}

fn hmac_sha256(clave: &[u8], mensaje: &[u8]) -> [u8; 32] {
    let mut bloque = [0u8; 64];
    if clave.len() > 64 {
        bloque[..32].copy_from_slice(&sha256(clave));
    } else {
        bloque[..clave.len()].copy_from_slice(clave);
    }
    let mut interior: Vec<u8> = bloque.iter().map(|b| b ^ 0x36).collect();
    interior.extend_from_slice(mensaje);
    let mut exterior: Vec<u8> = bloque.iter().map(|b| b ^ 0x5c).collect();
    exterior.extend_from_slice(&sha256(&interior));
    sha256(&exterior)
}

fn pbkdf2_sha256(clave: &[u8], sal: &[u8], vueltas: u32) -> [u8; 32] {
    let mut primero = sal.to_vec();
    primero.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(clave, &primero);
    let mut resultado = u;
    for _ in 1..vueltas {
        u = hmac_sha256(clave, &u);
        for (r, b) in resultado.iter_mut().zip(u.iter()) {
            *r ^= b;
        }
    }
    resultado
}

fn hexadecimal(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn desde_hexadecimal(texto: &str) -> Option<Vec<u8>> {
    if texto.len() % 2 != 0 {
        return None;
    }
    (0..texto.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(texto.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Bytes aleatorios del sistema (/dev/urandom); si no hay, se mezclan reloj, proceso y contador.
fn bytes_aleatorios(cantidad: usize) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = vec![0u8; cantidad];
    if let Ok(mut fuente) = std::fs::File::open("/dev/urandom") {
        if fuente.read_exact(&mut bytes).is_ok() {
            return bytes;
        }
    }
    let mut salida = Vec::new();
    while salida.len() < cantidad {
        let semilla = format!(
            "{:?}-{}-{}",
            std::time::SystemTime::now(),
            std::process::id(),
            DATOS_SECUENCIA.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        salida.extend_from_slice(&sha256(semilla.as_bytes()));
    }
    salida.truncate(cantidad);
    salida
}

fn iguales_en_tiempo_constante(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acumulado, (x, y)| acumulado | (x ^ y)) == 0
}

const VUELTAS_CLAVE: u32 = 100_000;
/// Colección interna de usuarios: empieza por '.', así que leer()/guardar() no pueden tocarla.
const COLECCION_USUARIOS: &str = ".usuarios";

fn nombre_usuario(valor: &Valor, funcion: &str) -> String {
    let nombre = texto_argumento(valor, funcion).trim().to_string();
    if nombre.is_empty() || nombre.chars().count() > 64 || nombre.chars().any(char::is_control) {
        return String::new();
    }
    nombre
}

/// Crea un usuario con su contraseña cifrada. Falso si ya existe, si el nombre no vale o si
/// la contraseña tiene menos de 8 caracteres.
fn crear_usuario(nombre: &Valor, clave: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "crear_usuario");
    let clave = texto_argumento(clave, "crear_usuario");
    if nombre.is_empty() || clave.chars().count() < 8 {
        return Valor::Logico(false);
    }
    let ficha = cifrar_clave(&clave);
    let creado = con_coleccion(COLECCION_USUARIOS, |entrada, ruta, archivo| {
        if entrada.datos.contains_key(&nombre) {
            return false;
        }
        let dato = Dato::Texto(ficha);
        anotar(entrada, archivo, COLECCION_USUARIOS, &linea_registro('s', &nombre, Some(&dato)));
        entrada.datos.insert(nombre.clone(), dato);
        compactar_si_hace_falta(entrada, ruta, COLECCION_USUARIOS);
        true
    });
    Valor::Logico(creado)
}

/// Verdadero si el usuario existe y la contraseña es la suya. Tras 5 fallos seguidos la
/// cuenta queda bloqueada 15 minutos y devuelve falso aunque la contraseña sea buena.
fn comprobar_clave(nombre: &Valor, clave: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "comprobar_clave");
    let clave = texto_argumento(clave, "comprobar_clave");
    if bloqueada(&nombre) {
        return Valor::Logico(false);
    }
    let correcta = clave_correcta(&nombre, &clave);
    con_fallos(|fallos| {
        if correcta {
            fallos.remove(&nombre);
        } else {
            let entrada = fallos.entry(nombre.clone()).or_insert((0, segundos_ahora()));
            entrada.0 += 1;
            entrada.1 = segundos_ahora();
        }
    });
    Valor::Logico(correcta)
}

// ---------------------------------------------------------------------------
// Sesiones: una galleta con un código aleatorio. En la base de datos solo se guarda la
// huella SHA-256 del código (quien copie el archivo no puede hacerse pasar por nadie),
// así que las sesiones siguen abiertas aunque el servidor se reinicie.
// ---------------------------------------------------------------------------

const GALLETA_SESION: &str = "deimon_sesion";
const DURACION_SESION: u64 = 7 * 24 * 3600;
const COLECCION_SESIONES: &str = ".sesiones";
const COLECCION_RECUPERACION: &str = ".recuperacion";
const DURACION_RECUPERACION: u64 = 3600;
const MAXIMO_FALLOS: u32 = 5;
const DURACION_BLOQUEO: u64 = 15 * 60;

fn segundos_ahora() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn huella(codigo: &str) -> String {
    hexadecimal(&sha256(codigo.as_bytes()))
}

fn borrar_dato(coleccion: &str, clave: &str) {
    con_coleccion(coleccion, |entrada, ruta, archivo| {
        if entrada.datos.remove(clave).is_some() {
            anotar(entrada, archivo, coleccion, &linea_registro('b', clave, None));
            compactar_si_hace_falta(entrada, ruta, coleccion);
        }
    });
}

/// Borra las fichas (sesiones o códigos) de un usuario, o todas las caducadas.
fn limpiar_fichas(coleccion: &str, de_usuario: Option<&str>) {
    let ahora = segundos_ahora();
    con_coleccion(coleccion, |entrada, ruta, archivo| {
        let sobran: Vec<String> = entrada
            .datos
            .iter()
            .filter(|(_, dato)| match dato {
                Dato::Texto(ficha) => {
                    let (nombre, caduca) = ficha.rsplit_once('\t').unwrap_or(("", "0"));
                    de_usuario.map(|usuario| usuario == nombre).unwrap_or(false)
                        || caduca.parse::<u64>().unwrap_or(0) <= ahora
                }
                _ => true,
            })
            .map(|(clave, _)| clave.clone())
            .collect();
        for clave in sobran {
            entrada.datos.remove(&clave);
            anotar(entrada, archivo, coleccion, &linea_registro('b', &clave, None));
        }
        compactar_si_hace_falta(entrada, ruta, coleccion);
    });
}

/// Nombre dueño de una ficha si no ha caducado.
fn dueno_de_ficha(coleccion: &str, codigo: &str) -> Option<String> {
    match leer_dato(coleccion, &huella(codigo)) {
        Some(Dato::Texto(ficha)) => {
            let (nombre, caduca) = ficha.rsplit_once('\t')?;
            (caduca.parse::<u64>().ok()? > segundos_ahora()).then(|| nombre.to_string())
        }
        _ => None,
    }
}

fn poner_galleta(mut cabecera: String) {
    // Por HTTPS, la galleta solo viaja cifrada.
    if PETICION.with(|peticion| peticion.borrow().segura) {
        cabecera.push_str("; Secure");
    }
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().cabeceras.push(cabecera));
}

/// Abre una sesión para un usuario existente: desde ahora usuario_actual() lo devuelve.
fn iniciar_sesion(nombre: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "iniciar_sesion");
    if leer_dato(COLECCION_USUARIOS, &nombre).is_none() {
        fallo(format!("iniciar_sesion(): el usuario '{}' no existe; créalo antes con crear_usuario()", nombre));
    }
    limpiar_fichas(COLECCION_SESIONES, None);
    let codigo = hexadecimal(&bytes_aleatorios(32));
    guardar_dato(
        COLECCION_SESIONES,
        &huella(&codigo),
        Dato::Texto(format!("{}\t{}", nombre, segundos_ahora() + DURACION_SESION)),
    );
    poner_galleta(format!(
        "Set-Cookie: {}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        GALLETA_SESION, codigo, DURACION_SESION
    ));
    Valor::Nada
}

/// Nombre del usuario con la sesión abierta, o nada si nadie ha entrado.
fn usuario_actual() -> Valor {
    galleta(GALLETA_SESION)
        .filter(|codigo| codigo.len() == 64 && codigo.chars().all(|c| c.is_ascii_hexdigit()))
        .and_then(|codigo| dueno_de_ficha(COLECCION_SESIONES, &codigo))
        .map(|nombre| Valor::Texto(Rc::new(nombre)))
        .unwrap_or(Valor::Nada)
}

/// Cierra la sesión de quien hace la petición.
fn cerrar_sesion() -> Valor {
    if let Some(codigo) = galleta(GALLETA_SESION) {
        borrar_dato(COLECCION_SESIONES, &huella(&codigo));
    }
    poner_galleta(format!(
        "Set-Cookie: {}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        GALLETA_SESION
    ));
    Valor::Nada
}

// ---------------------------------------------------------------------------
// Sesión del visitante (como $_SESSION de PHP): datos de cada navegador, aunque no haya
// entrado ningún usuario. Una galleta con un código aleatorio; en la base de datos solo
// se guarda su huella. Caduca 24 horas después del último cambio.
// ---------------------------------------------------------------------------

const GALLETA_VISITA: &str = "deimon_visita";
const COLECCION_VISITAS: &str = ".visitas";
const DURACION_VISITA: u64 = 24 * 3600;

fn codigo_de_visita(crear: bool) -> Option<String> {
    let valida = |c: &String| c.len() == 64 && c.chars().all(|x| x.is_ascii_hexdigit());
    if let Some(codigo) = galleta(GALLETA_VISITA).filter(valida) {
        return Some(codigo);
    }
    if !crear {
        return None;
    }
    let codigo = hexadecimal(&bytes_aleatorios(32));
    poner_galleta(format!(
        "Set-Cookie: {}={}; Path=/; HttpOnly; SameSite=Lax",
        GALLETA_VISITA, codigo
    ));
    // Para que el resto de esta misma petición ya la vea.
    PETICION.with(|peticion| peticion.borrow_mut().galletas.push((GALLETA_VISITA.to_string(), codigo.clone())));
    // De paso se borran las visitas caducadas.
    let ahora = segundos_ahora();
    con_coleccion(COLECCION_VISITAS, |entrada, ruta, archivo| {
        let caducadas: Vec<String> = entrada
            .datos
            .iter()
            .filter_map(|(clave, dato)| match (clave.strip_suffix('\t'), dato) {
                (Some(prefijo), Dato::Numero(caduca)) if (*caduca as u64) <= ahora => Some(prefijo.to_string()),
                _ => None,
            })
            .collect();
        for prefijo in caducadas {
            let sobran: Vec<String> = entrada
                .datos
                .keys()
                .filter(|clave| clave.starts_with(&format!("{}\t", prefijo)))
                .cloned()
                .collect();
            for clave in sobran {
                entrada.datos.remove(&clave);
                anotar(entrada, archivo, COLECCION_VISITAS, &linea_registro('b', &clave, None));
            }
        }
        compactar_si_hace_falta(entrada, ruta, COLECCION_VISITAS);
    });
    Some(codigo)
}

fn clave_de_visita(codigo: &str, clave: &str) -> String {
    format!("{}\t{}", huella(codigo), clave)
}

fn visita_viva(codigo: &str) -> bool {
    matches!(leer_dato(COLECCION_VISITAS, &format!("{}\t", huella(codigo))), Some(Dato::Numero(caduca)) if (caduca as u64) > segundos_ahora())
}

/// Guarda un valor (también listas) en la sesión de este visitante.
fn guardar_en_sesion(clave: &Valor, valor: &Valor) -> Valor {
    let clave = texto_argumento(clave, "guardar_en_sesion");
    let dato = Dato::desde(valor);
    let codigo = codigo_de_visita(true).expect("se acaba de crear");
    guardar_dato(COLECCION_VISITAS, &format!("{}\t", huella(&codigo)), Dato::Numero((segundos_ahora() + DURACION_VISITA) as f64));
    guardar_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave), dato);
    Valor::Nada
}

/// Lee un valor de la sesión de este visitante, o nada.
fn leer_de_sesion(clave: &Valor) -> Valor {
    let clave = texto_argumento(clave, "leer_de_sesion");
    match codigo_de_visita(false) {
        Some(codigo) if visita_viva(&codigo) => leer_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave))
            .map(|dato| dato.a_valor())
            .unwrap_or(Valor::Nada),
        _ => Valor::Nada,
    }
}

/// Añade un valor a una lista de la sesión (la crea si no existe), como $_SESSION['x'][] = v.
fn agregar_a_sesion(clave: &Valor, valor: &Valor) -> Valor {
    let actual = leer_de_sesion(clave);
    let lista = match actual {
        Valor::Nada => Valor::Lista(Rc::new(vec![valor.clone()])),
        otra => agregar(&otra, valor),
    };
    guardar_en_sesion(clave, &lista)
}

/// Borra un valor de la sesión de este visitante.
fn borrar_de_sesion(clave: &Valor) -> Valor {
    let clave = texto_argumento(clave, "borrar_de_sesion");
    if let Some(codigo) = codigo_de_visita(false) {
        borrar_dato(COLECCION_VISITAS, &clave_de_visita(&codigo, &clave));
    }
    Valor::Nada
}

/// Borra todos los datos de la sesión de este visitante (como session_destroy()).
fn vaciar_sesion() -> Valor {
    if let Some(codigo) = codigo_de_visita(false) {
        let prefijo = format!("{}\t", huella(&codigo));
        con_coleccion(COLECCION_VISITAS, |entrada, ruta, archivo| {
            let sobran: Vec<String> = entrada.datos.keys().filter(|c| c.starts_with(&prefijo)).cloned().collect();
            for clave in sobran {
                entrada.datos.remove(&clave);
                anotar(entrada, archivo, COLECCION_VISITAS, &linea_registro('b', &clave, None));
            }
            compactar_si_hace_falta(entrada, ruta, COLECCION_VISITAS);
        });
    }
    Valor::Nada
}

// ---------------------------------------------------------------------------
// Límite de intentos: tras 5 contraseñas equivocadas seguidas, la cuenta se bloquea
// 15 minutos (aunque la contraseña sea la buena), para frenar a quien prueba claves.
// ---------------------------------------------------------------------------

static FALLOS: Mutex<Option<std::collections::HashMap<String, (u32, u64)>>> = Mutex::new(None);

fn con_fallos<T>(operacion: impl FnOnce(&mut std::collections::HashMap<String, (u32, u64)>) -> T) -> T {
    let mut fallos = FALLOS.lock().unwrap_or_else(|error| error.into_inner());
    operacion(fallos.get_or_insert_with(std::collections::HashMap::new))
}

fn bloqueada(nombre: &str) -> bool {
    let ahora = segundos_ahora();
    con_fallos(|fallos| match fallos.get(nombre) {
        Some((veces, desde)) if *veces >= MAXIMO_FALLOS && ahora < desde + DURACION_BLOQUEO => true,
        Some((_, desde)) if ahora >= desde + DURACION_BLOQUEO => {
            fallos.remove(nombre);
            false
        }
        _ => false,
    })
}

/// Verdadero si la cuenta está bloqueada por demasiados intentos fallidos.
fn cuenta_bloqueada(nombre: &Valor) -> Valor {
    Valor::Logico(bloqueada(&nombre_usuario(nombre, "cuenta_bloqueada")))
}

fn cifrar_clave(clave: &str) -> String {
    let sal = bytes_aleatorios(16);
    let cifrada = pbkdf2_sha256(clave.as_bytes(), &sal, VUELTAS_CLAVE);
    format!("pbkdf2-sha256${}${}${}", VUELTAS_CLAVE, hexadecimal(&sal), hexadecimal(&cifrada))
}

fn clave_correcta(nombre: &str, clave: &str) -> bool {
    let ficha = match leer_dato(COLECCION_USUARIOS, nombre) {
        Some(Dato::Texto(ficha)) => ficha,
        _ => {
            // Se calcula igual para que no se note por el tiempo si el usuario existe.
            pbkdf2_sha256(clave.as_bytes(), b"deimon-sin-usuario", VUELTAS_CLAVE);
            return false;
        }
    };
    let partes: Vec<&str> = ficha.split('$').collect();
    match partes.as_slice() {
        ["pbkdf2-sha256", vueltas, sal, cifrada] => {
            match (vueltas.parse::<u32>(), desde_hexadecimal(sal), desde_hexadecimal(cifrada)) {
                (Ok(vueltas), Some(sal), Some(cifrada)) => {
                    iguales_en_tiempo_constante(&pbkdf2_sha256(clave.as_bytes(), &sal, vueltas), &cifrada)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Cambia la contraseña guardada y cierra todas las sesiones abiertas del usuario.
fn poner_clave(nombre: &str, nueva: &str) {
    guardar_dato(COLECCION_USUARIOS, nombre, Dato::Texto(cifrar_clave(nueva)));
    limpiar_fichas(COLECCION_SESIONES, Some(nombre));
    limpiar_fichas(COLECCION_RECUPERACION, Some(nombre));
}

/// Cambia la contraseña sabiendo la actual. Falso si la actual no es correcta o la nueva
/// tiene menos de 8 caracteres.
fn cambiar_clave(nombre: &Valor, actual: &Valor, nueva: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "cambiar_clave");
    let actual = texto_argumento(actual, "cambiar_clave");
    let nueva = texto_argumento(nueva, "cambiar_clave");
    if nueva.chars().count() < 8 || bloqueada(&nombre) || !clave_correcta(&nombre, &actual) {
        return Valor::Logico(false);
    }
    poner_clave(&nombre, &nueva);
    Valor::Logico(true)
}

/// Crea un código de un solo uso (válido 1 hora) para recuperar la contraseña; el programa
/// decide cómo hacérselo llegar al usuario. Nada si el usuario no existe.
fn crear_codigo_recuperacion(nombre: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "crear_codigo_recuperacion");
    if leer_dato(COLECCION_USUARIOS, &nombre).is_none() {
        return Valor::Nada;
    }
    limpiar_fichas(COLECCION_RECUPERACION, None);
    let codigo = hexadecimal(&bytes_aleatorios(16));
    guardar_dato(
        COLECCION_RECUPERACION,
        &huella(&codigo),
        Dato::Texto(format!("{}\t{}", nombre, segundos_ahora() + DURACION_RECUPERACION)),
    );
    Valor::Texto(Rc::new(codigo))
}

/// Pone una contraseña nueva con un código de recuperación (que deja de valer).
fn restablecer_clave(nombre: &Valor, codigo: &Valor, nueva: &Valor) -> Valor {
    let nombre = nombre_usuario(nombre, "restablecer_clave");
    let codigo = texto_argumento(codigo, "restablecer_clave");
    let nueva = texto_argumento(nueva, "restablecer_clave");
    if nueva.chars().count() < 8 || dueno_de_ficha(COLECCION_RECUPERACION, &codigo).as_deref() != Some(nombre.as_str()) {
        return Valor::Logico(false);
    }
    borrar_dato(COLECCION_RECUPERACION, &huella(&codigo));
    poner_clave(&nombre, &nueva);
    con_fallos(|fallos| {
        fallos.remove(&nombre);
    });
    Valor::Logico(true)
}

/// Suma `cantidad` a un número guardado de forma atómica (nadie más lo cambia a la vez).
fn incrementar(coleccion: &Valor, clave: &Valor, cantidad: &Valor) -> Valor {
    let coleccion = nombre_coleccion(coleccion);
    let clave = clave_texto(clave);
    let cantidad = match cantidad {
        Valor::Numero(numero) => *numero,
        otro => fallo(format!("incrementar() espera un número como cantidad, no {}", otro.tipo())),
    };
    let nuevo = con_coleccion(&coleccion, |entrada, ruta, archivo| {
        let actual = match entrada.datos.get(&clave) {
            None | Some(Dato::Nada) => 0.0,
            Some(Dato::Numero(numero)) => *numero,
            Some(_) => fallo(format!(
                "incrementar(): la clave '{}' no guarda un número",
                clave
            )),
        };
        let dato = Dato::Numero(actual + cantidad);
        anotar(entrada, archivo, &coleccion, &linea_registro('s', &clave, Some(&dato)));
        entrada.datos.insert(clave.clone(), dato);
        compactar_si_hace_falta(entrada, ruta, &coleccion);
        actual + cantidad
    });
    Valor::Numero(nuevo)
}

/// Atiende una ruta sin red (para visitar/enviar en las pruebas), guardando las galletas.
fn atender_sin_red(camino: &str, envio: bool, mut campos: Vec<(String, String)>, rutas: Vec<Ruta>) -> Valor {
    let (camino, consulta) = camino.split_once('?').unwrap_or((camino, ""));
    campos.extend(decodificar_formulario(consulta));
    let galletas = GALLETAS_PRUEBA.with(|galletas| galletas.borrow().clone());
    preparar_peticion(envio, campos, galletas);
    let cuerpo = CUERPO_PRUEBA.with(|cuerpo| std::mem::take(&mut *cuerpo.borrow_mut()));
    PETICION.with(|peticion| peticion.borrow_mut().cuerpo = cuerpo);
    let valor = match buscar_ruta(&rutas, camino) {
        Some((ruta, parametros)) => Some((ruta.manejador)(parametros)),
        None => None,
    };
    let respuesta = tomar_respuesta();
    let cuerpo = match valor {
        Some(valor) => cuerpo_pagina(&respuesta.escrito, &valor),
        None => PAGINA_404.to_string(),
    };
    for cabecera in &respuesta.cabeceras {
        if let Some(valor) = cabecera.strip_prefix("Set-Cookie: ") {
            let primera = valor.split(';').next().unwrap_or("");
            if let Some((nombre, contenido)) = primera.split_once('=') {
                GALLETAS_PRUEBA.with(|galletas| {
                    let mut galletas = galletas.borrow_mut();
                    galletas.retain(|(clave, _)| clave != nombre);
                    if !contenido.is_empty() && !valor.contains("Max-Age=0") {
                        galletas.push((nombre.to_string(), contenido.to_string()));
                    }
                });
            }
        }
    }
    if let Some(archivo) = &respuesta.archivo {
        return Valor::Texto(Rc::new(format!("archivo {} de {} bytes", archivo.tipo, archivo.datos.len())));
    }
    match respuesta.redireccion {
        Some(destino) => Valor::Texto(Rc::new(format!("redirige a {}", destino))),
        None => Valor::Texto(Rc::new(cuerpo)),
    }
}

/// Como visitar(), pero enviando un formulario: datos es un texto como "nombre=Ana&clave=123".
fn enviar(camino: &Valor, datos: &Valor, rutas: Vec<Ruta>) -> Valor {
    let camino = texto_argumento(camino, "enviar");
    let datos = texto_argumento(datos, "enviar");
    CUERPO_PRUEBA.with(|cuerpo| *cuerpo.borrow_mut() = datos.clone());
    atender_sin_red(&camino, true, decodificar_formulario(&datos), rutas)
}

// ---------------------------------------------------------------------------
// Archivos subidos (multipart/form-data), archivos servidos y galletas propias.
// ---------------------------------------------------------------------------

/// Frontera de un formulario con archivos: Content-Type: multipart/form-data; boundary=...
fn frontera_multipart(tipo: &str) -> Option<String> {
    let (principal, resto) = tipo.split_once(';')?;
    if !principal.trim().eq_ignore_ascii_case("multipart/form-data") {
        return None;
    }
    resto.split(';').find_map(|parte| {
        let (clave, valor) = parte.trim().split_once('=')?;
        let valor = valor.trim().trim_matches('"');
        (clave.trim().eq_ignore_ascii_case("boundary") && !valor.is_empty() && valor.len() <= 200).then(|| valor.to_string())
    })
}

fn buscar_bytes(datos: &[u8], buscado: &[u8], desde: usize) -> Option<usize> {
    if buscado.is_empty() || datos.len() < buscado.len() {
        return None;
    }
    (desde..=datos.len() - buscado.len()).find(|&i| &datos[i..i + buscado.len()] == buscado)
}

/// Valor de un parámetro de Content-Disposition (name="x", filename="y").
fn parametro_disposicion(linea: &str, nombre: &str) -> Option<String> {
    let minuscula = linea.to_ascii_lowercase();
    let buscado = format!("{}=\"", nombre);
    let mut desde = 0;
    while let Some(pos) = minuscula[desde..].find(&buscado) {
        let inicio = desde + pos;
        let antes = minuscula[..inicio].chars().last();
        if antes.map_or(true, |c| c == ' ' || c == ';') {
            let valor = &linea[inicio + buscado.len()..];
            return Some(valor[..valor.find('"').unwrap_or(valor.len())].to_string());
        }
        desde = inicio + buscado.len();
    }
    None
}

/// Separa un cuerpo multipart en campos de texto y archivos. None si está mal formado.
fn leer_multipart(cuerpo: &[u8], frontera: &str) -> Option<(Vec<(String, String)>, Vec<Subida>)> {
    let marca = format!("--{}", frontera).into_bytes();
    let separador = format!("\r\n--{}", frontera).into_bytes();
    let mut pos = buscar_bytes(cuerpo, &marca, 0)? + marca.len();
    let (mut textos, mut archivos) = (Vec::new(), Vec::new());
    loop {
        if cuerpo.get(pos..pos + 2) == Some(b"--") {
            return Some((textos, archivos));
        }
        if cuerpo.get(pos..pos + 2) != Some(b"\r\n") {
            return None;
        }
        pos += 2;
        let fin_cabeceras = buscar_bytes(cuerpo, b"\r\n\r\n", pos)?;
        let cabeceras = String::from_utf8_lossy(&cuerpo[pos..fin_cabeceras]).into_owned();
        let inicio = fin_cabeceras + 4;
        let fin = buscar_bytes(cuerpo, &separador, inicio)?;
        let contenido = &cuerpo[inicio..fin];
        let mut nombre = None;
        let mut archivo = None;
        let mut tipo = "application/octet-stream".to_string();
        for linea in cabeceras.split("\r\n") {
            let (clave, valor) = match linea.split_once(':') {
                Some(par) => par,
                None => continue,
            };
            if clave.trim().eq_ignore_ascii_case("Content-Disposition") {
                nombre = parametro_disposicion(valor, "name");
                archivo = parametro_disposicion(valor, "filename");
            } else if clave.trim().eq_ignore_ascii_case("Content-Type") {
                tipo = valor.trim().to_string();
            }
        }
        let nombre = nombre?;
        match archivo {
            Some(archivo) if !archivo.is_empty() || !contenido.is_empty() => archivos.push(Subida {
                campo: nombre,
                // Algunos navegadores mandan la ruta entera (C:\fotos\taza.jpg): solo vale el final.
                nombre: archivo.rsplit(['/', '\\']).next().unwrap_or("").to_string(),
                tipo,
                datos: contenido.to_vec(),
            }),
            Some(_) => {}
            None => textos.push((nombre, String::from_utf8_lossy(contenido).into_owned())),
        }
        pos = fin + separador.len();
    }
}

/// Tipo de imagen según sus primeros bytes (no según lo que diga el navegador).
fn tipo_de_imagen(datos: &[u8]) -> Option<&'static str> {
    if datos.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if datos.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if datos.starts_with(b"GIF87a") || datos.starts_with(b"GIF89a") {
        Some("gif")
    } else if datos.len() > 12 && &datos[..4] == b"RIFF" && &datos[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

fn con_subida<T>(campo: &Valor, funcion: &str, accion: impl FnOnce(&Subida) -> T) -> Option<T> {
    let campo = texto_argumento(campo, funcion);
    PETICION.with(|peticion| peticion.borrow().subidas.iter().find(|s| s.campo == campo).map(accion))
}

/// Datos del archivo que llegó en ese campo del formulario, o nada.
fn archivo_subido(campo: &Valor) -> Valor {
    con_subida(campo, "archivo_subido", |s| {
        let mut mapa = MapaValores::default();
        mapa.poner("nombre".to_string(), Valor::Texto(Rc::new(escapar_html(&s.nombre))));
        mapa.poner("tipo".to_string(), Valor::Texto(Rc::new(escapar_html(&s.tipo))));
        mapa.poner("tamano".to_string(), Valor::Numero(s.datos.len() as f64));
        mapa.poner("es_imagen".to_string(), Valor::Logico(tipo_de_imagen(&s.datos).is_some()));
        Valor::Mapa(Rc::new(mapa))
    })
    .unwrap_or(Valor::Nada)
}

fn nombre_de_carpeta(carpeta: &Valor, funcion: &str) -> String {
    let carpeta = texto_argumento(carpeta, funcion);
    if carpeta.is_empty() || carpeta.len() > 64 || !carpeta.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        fallo(format!("{}(): la carpeta solo puede tener letras, dígitos, '_' y '-'", funcion));
    }
    carpeta
}

/// Guarda el archivo subido en datos/archivos/<carpeta>/ con un nombre nuevo al azar
/// (nadie puede elegir dónde se escribe) y devuelve ese nombre, o nada si no llegó archivo.
/// Las imágenes se reconocen por su contenido; lo demás se guarda como .bin y se descarga.
fn guardar_subida(campo: &Valor, carpeta: &Valor) -> Valor {
    let carpeta = nombre_de_carpeta(carpeta, "guardar_subida");
    let (datos, extension) = match con_subida(campo, "guardar_subida", |s| {
        let extension = match tipo_de_imagen(&s.datos) {
            Some(imagen) => imagen.to_string(),
            None => {
                let pedida = s.nombre.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                if pedida == "pdf" && s.datos.starts_with(b"%PDF-") {
                    "pdf".to_string()
                } else {
                    "bin".to_string()
                }
            }
        };
        (s.datos.clone(), extension)
    }) {
        Some(par) => par,
        None => return Valor::Nada,
    };
    let destino = carpeta_datos().join("archivos").join(&carpeta);
    if let Err(error) = std::fs::create_dir_all(&destino) {
        fallo(format!("guardar_subida(): no se pudo crear la carpeta: {}", error));
    }
    let nombre = format!("{}.{}", hexadecimal(&bytes_aleatorios(12)), extension);
    let ruta = destino.join(&nombre);
    let temporal = destino.join(format!(".{}.tmp", nombre));
    let resultado = std::fs::write(&temporal, &datos).and_then(|_| std::fs::rename(&temporal, &ruta));
    if let Err(error) = resultado {
        let _ = std::fs::remove_file(&temporal);
        fallo(format!("guardar_subida(): no se pudo guardar el archivo: {}", error));
    }
    Valor::Texto(Rc::new(nombre))
}

/// Hace que la ruta responda con un archivo guardado con guardar_subida(). Devuelve falso
/// si no existe (la ruta puede entonces devolver un aviso).
fn responder_archivo(carpeta: &Valor, nombre: &Valor) -> Valor {
    let carpeta = nombre_de_carpeta(carpeta, "responder_archivo");
    let nombre = texto_argumento(nombre, "responder_archivo");
    let valido = !nombre.is_empty()
        && nombre.len() <= 100
        && !nombre.starts_with('.')
        && nombre.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !valido {
        return Valor::Logico(false);
    }
    let datos = match std::fs::read(carpeta_datos().join("archivos").join(&carpeta).join(&nombre)) {
        Ok(datos) => datos,
        Err(_) => return Valor::Logico(false),
    };
    let extension = nombre.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let (tipo, descargar) = match extension.as_str() {
        "jpg" | "jpeg" => ("image/jpeg", None),
        "png" => ("image/png", None),
        "gif" => ("image/gif", None),
        "webp" => ("image/webp", None),
        "pdf" => ("application/pdf", None),
        // Todo lo demás se descarga: así un archivo subido nunca se abre como página.
        _ => ("application/octet-stream", Some(nombre.clone())),
    };
    RESPUESTA.with(|respuesta| respuesta.borrow_mut().archivo = Some(ArchivoRespuesta { datos, tipo, descargar }));
    Valor::Logico(true)
}

fn responder_archivo_http(conexion: &mut std::net::TcpStream, archivo: ArchivoRespuesta, extra: &[String]) {
    use std::io::Write;
    let mut cabecera = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; img-src 'self'; style-src 'unsafe-inline'\r\nCache-Control: private, max-age=3600\r\n",
        archivo.tipo,
        archivo.datos.len()
    );
    if let Some(nombre) = &archivo.descargar {
        cabecera.push_str(&format!("Content-Disposition: attachment; filename=\"{}\"\r\n", nombre));
    }
    for linea in extra {
        cabecera.push_str(linea);
        cabecera.push_str("\r\n");
    }
    cabecera.push_str("Connection: close\r\n\r\n");
    let _ = conexion.write_all(cabecera.as_bytes());
    let _ = conexion.write_all(&archivo.datos);
    let _ = conexion.flush();
    let _ = conexion.shutdown(std::net::Shutdown::Write);
}

/// Cuerpo de la petición tal cual llegó, SIN proteger: sirve para los avisos (webhooks) de
/// otros servicios, que mandan JSON. No lo escribas en una página sin proteger_html().
fn cuerpo_recibido() -> Valor {
    PETICION.with(|peticion| Valor::Texto(Rc::new(peticion.borrow().cuerpo.clone())))
}

/// Valor de una cabecera de la petición (sin proteger), o nada.
fn cabecera_recibida(nombre: &Valor) -> Valor {
    let nombre = texto_argumento(nombre, "cabecera_recibida").to_ascii_lowercase();
    PETICION.with(|peticion| {
        peticion
            .borrow()
            .cabeceras
            .iter()
            .find(|(clave, _)| *clave == nombre)
            .map(|(_, valor)| Valor::Texto(Rc::new(valor.clone())))
            .unwrap_or(Valor::Nada)
    })
}

fn nombre_de_galleta(nombre: &Valor, funcion: &str) -> String {
    let nombre = texto_argumento(nombre, funcion);
    if nombre.is_empty() || nombre.len() > 64 || !nombre.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        fallo(format!("{}(): el nombre de la galleta solo puede tener letras, dígitos, '_' y '-'", funcion));
    }
    if nombre.starts_with("deimon_") {
        fallo(format!("{}(): los nombres que empiezan por 'deimon_' los usa Deimon para las sesiones", funcion));
    }
    nombre
}

/// Lee una galleta (cookie) del navegador, protegida como HTML, o nada. Si se guardó un
/// campo de formulario (ya protegido), no se protege dos veces.
fn leer_galleta(nombre: &Valor) -> Valor {
    let nombre = nombre_de_galleta(nombre, "leer_galleta");
    galleta(&nombre)
        .map(|valor| Valor::Texto(Rc::new(proteger_una_vez(&decodificar(&valor)))))
        .unwrap_or(Valor::Nada)
}

/// Guarda una galleta en el navegador durante unos días (0: hasta cerrar el navegador).
fn guardar_galleta(nombre: &Valor, valor: &Valor, dias: &Valor) -> Valor {
    let nombre = nombre_de_galleta(nombre, "guardar_galleta");
    let texto = texto_de(valor);
    if texto.len() > 3000 {
        fallo("guardar_galleta(): el valor es demasiado largo (máximo 3000 letras)".to_string());
    }
    let mut codificado = String::new();
    for byte in texto.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            codificado.push(byte as char);
        } else {
            codificado.push_str(&format!("%{:02X}", byte));
        }
    }
    let dias = match dias {
        Valor::Numero(n) if n.is_finite() && *n >= 0.0 && *n <= 3650.0 => *n,
        _ => fallo("guardar_galleta(): los días deben ser un número entre 0 y 3650".to_string()),
    };
    let caducidad = if dias > 0.0 { format!("; Max-Age={}", (dias * 86400.0) as u64) } else { String::new() };
    poner_galleta(format!("Set-Cookie: {}={}; Path=/; HttpOnly; SameSite=Lax{}", nombre, codificado, caducidad));
    PETICION.with(|peticion| {
        let mut peticion = peticion.borrow_mut();
        peticion.galletas.retain(|(clave, _)| clave != &nombre);
        peticion.galletas.push((nombre.clone(), codificado));
    });
    Valor::Nada
}

fn borrar_galleta(nombre: &Valor) -> Valor {
    let nombre = nombre_de_galleta(nombre, "borrar_galleta");
    poner_galleta(format!("Set-Cookie: {}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0", nombre));
    PETICION.with(|peticion| peticion.borrow_mut().galletas.retain(|(clave, _)| clave != &nombre));
    Valor::Nada
}

fn responder(conexion: &mut std::net::TcpStream, estado: &str, cuerpo: &str) {
    responder_con(conexion, estado, cuerpo, &[]);
}

fn responder_con(conexion: &mut std::net::TcpStream, estado: &str, cuerpo: &str, extra: &[String]) {
    use std::io::{Read, Write};
    let mut cabeceras = String::new();
    for cabecera in extra {
        cabeceras.push_str(cabecera);
        cabeceras.push_str("\r\n");
    }
    let respuesta = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
        estado,
        cuerpo.len(),
        cabeceras,
        cuerpo
    );
    let _ = conexion.write_all(respuesta.as_bytes());
    let _ = conexion.flush();
    // Se cierra el lado de escritura y se descarta lo que quede sin leer: así el cliente
    // recibe la respuesta completa en vez de un corte (RST) por datos pendientes.
    let _ = conexion.shutdown(std::net::Shutdown::Write);
    let _ = conexion.set_read_timeout(Some(std::time::Duration::from_millis(1)));
    let mut descarte = [0u8; 4096];
    loop {
        match conexion.read(&mut descarte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

/// Límites del servidor; salen del perfil grabado al compilar y de las variables DEIMON_*.
#[derive(Clone, Copy)]
struct Limites {
    hilos: usize,
    max_conexiones: usize,
    max_peticion: usize,
    /// Tamaño máximo de un formulario con archivos (multipart/form-data).
    max_subida: usize,
    tiempo_espera: std::time::Duration,
}

fn numero_de_entorno(nombre: &str) -> Option<usize> {
    std::env::var(nombre)
        .ok()
        .and_then(|texto| texto.trim().parse::<usize>().ok())
        .filter(|numero| *numero > 0)
}

fn limites() -> Limites {
    let por_perfil = |pequeno: usize, equilibrado: usize, servidor: usize| match PERFIL_LN {
        "pequeño" | "pequeno" => pequeno,
        "servidor" => servidor,
        _ => equilibrado,
    };
    let hilos = numero_de_entorno("DEIMON_HILOS").unwrap_or_else(|| {
        let nucleos = std::thread::available_parallelism()
            .map(|cantidad| cantidad.get())
            .unwrap_or(1);
        // Casi todo el tiempo de un hilo es esperar a la red, no calcular: con pocos hilos,
        // unos pocos clientes lentos bastarían para ocuparlos todos.
        match PERFIL_LN {
            "pequeño" | "pequeno" => 4,
            "servidor" => (nucleos * 8).max(64),
            _ => (nucleos * 4).max(16),
        }
    });
    let max_conexiones =
        numero_de_entorno("DEIMON_MAX_CONEXIONES").unwrap_or_else(|| por_perfil(16, 128, 1024));
    let max_peticion = numero_de_entorno("DEIMON_MAX_PETICION")
        .unwrap_or_else(|| por_perfil(16 * 1024, 256 * 1024, 1024 * 1024));
    let max_subida = numero_de_entorno("DEIMON_MAX_SUBIDA")
        .unwrap_or_else(|| por_perfil(2 * 1024 * 1024, 10 * 1024 * 1024, 50 * 1024 * 1024));
    let segundos = numero_de_entorno("DEIMON_TIEMPO_ESPERA").unwrap_or(10);
    Limites {
        hilos,
        max_conexiones,
        max_peticion,
        max_subida,
        tiempo_espera: std::time::Duration::from_secs(segundos as u64),
    }
}

enum Lectura {
    /// Cabeceras y los bytes del cuerpo que ya llegaron con ellas.
    Peticion(String, Vec<u8>),
    DemasiadoGrande,
    Cerrada,
}

fn fin_de_cabecera(datos: &[u8]) -> Option<usize> {
    datos
        .windows(4)
        .position(|ventana| ventana == b"\r\n\r\n")
        .map(|posicion| posicion + 4)
        .or_else(|| {
            datos
                .windows(2)
                .position(|ventana| ventana == b"\n\n")
                .map(|posicion| posicion + 2)
        })
}

/// Antes de cada lectura, ajusta la espera a lo que queda del plazo total de la petición.
/// Devuelve falso si el plazo ya terminó (un cliente que gotea datos no ocupa el hilo para siempre).
fn quedar_hasta(conexion: &std::net::TcpStream, limite: std::time::Instant) -> bool {
    match limite.checked_duration_since(std::time::Instant::now()) {
        Some(resto) if !resto.is_zero() => conexion.set_read_timeout(Some(resto)).is_ok(),
        _ => false,
    }
}

/// Lee solo las cabeceras, sin pasar del máximo ni del plazo: así un cliente lento o enorme
/// no agota la memoria ni retiene el hilo.
fn leer_cabecera(conexion: &mut std::net::TcpStream, maximo: usize, limite: std::time::Instant) -> Lectura {
    use std::io::Read;
    let mut datos = Vec::new();
    let mut bloque = [0u8; 4096];
    loop {
        if let Some(fin) = fin_de_cabecera(&datos) {
            if fin > maximo {
                return Lectura::DemasiadoGrande;
            }
            let resto = datos.split_off(fin);
            return Lectura::Peticion(String::from_utf8_lossy(&datos).into_owned(), resto);
        }
        if datos.len() > maximo {
            return Lectura::DemasiadoGrande;
        }
        let disponibles = maximo + 1 - datos.len();
        let tamano = bloque.len().min(disponibles);
        if !quedar_hasta(conexion, limite) {
            return Lectura::Cerrada;
        }
        match conexion.read(&mut bloque[..tamano]) {
            Ok(0) => return Lectura::Cerrada,
            Ok(leidos) => datos.extend_from_slice(&bloque[..leidos]),
            Err(_) => return Lectura::Cerrada,
        }
    }
}

fn atender(conexion: std::net::TcpStream, rutas: &[Ruta], limites: &Limites) {
    // Plazo total para recibir la petición entera (cabeceras y cuerpo), no por cada lectura.
    let limite = std::time::Instant::now() + limites.tiempo_espera;
    let _ = conexion.set_read_timeout(Some(limites.tiempo_espera));
    let _ = conexion.set_write_timeout(Some(std::time::Duration::from_secs(10)));
    let mut escritura = match conexion.try_clone() {
        Ok(copia) => copia,
        Err(_) => return,
    };
    let mut lectura = conexion;
    let (peticion, mut cuerpo) = match leer_cabecera(&mut lectura, limites.max_peticion, limite) {
        Lectura::Peticion(texto, resto) => (texto, resto),
        Lectura::DemasiadoGrande => {
            responder(
                &mut escritura,
                "413 Payload Too Large",
                "<h1>Petición demasiado grande</h1>",
            );
            return;
        }
        Lectura::Cerrada => return,
    };
    let primera = peticion.lines().next().unwrap_or("");
    let mut partes = primera.split_whitespace();
    let metodo = partes.next().unwrap_or("");
    let objetivo = partes.next().unwrap_or("");
    if metodo != "GET" && metodo != "POST" {
        responder(
            &mut escritura,
            "405 Method Not Allowed",
            "<h1>Método no permitido</h1>",
        );
        return;
    }
    let cabecera = |nombre: &str| -> Option<String> {
        peticion.lines().skip(1).find_map(|linea| {
            let (clave, valor) = linea.split_once(':')?;
            clave.trim().eq_ignore_ascii_case(nombre).then(|| valor.trim().to_string())
        })
    };
    let (camino, consulta) = objetivo.split_once('?').unwrap_or((objetivo, ""));
    let mut campos = decodificar_formulario(consulta);
    let mut subidas: Vec<Subida> = Vec::new();
    let mut cuerpo_texto = String::new();
    let envio = metodo == "POST";
    if envio {
        // El cuerpo del formulario cuenta para el tamaño máximo de la petición.
        // Un tamaño que no es un número, dos tamaños distintos o el envío por trozos (chunked,
        // que Deimon no entiende) se rechazan en vez de adivinar dónde termina el cuerpo.
        let tamanos: Vec<&str> = peticion
            .lines()
            .skip(1)
            .filter_map(|linea| linea.split_once(':'))
            .filter(|(clave, _)| clave.trim().eq_ignore_ascii_case("Content-Length"))
            .map(|(_, valor)| valor.trim())
            .collect();
        let largo: Option<usize> = match tamanos.as_slice() {
            [] => Some(0),
            [primero, resto @ ..] if resto.iter().all(|otro| otro == primero) => primero.parse().ok(),
            _ => None,
        };
        let largo = match largo {
            Some(largo) if cabecera("Transfer-Encoding").is_none() => largo,
            _ => {
                responder(&mut escritura, "400 Bad Request", "<h1>Petición mal formada</h1>");
                return;
            }
        };
        let frontera = cabecera("Content-Type").and_then(|tipo| frontera_multipart(&tipo));
        let maximo = if frontera.is_some() { limites.max_subida } else { limites.max_peticion };
        if largo > maximo.saturating_sub(peticion.len()) {
            responder(&mut escritura, "413 Payload Too Large", "<h1>Petición demasiado grande</h1>");
            return;
        }
        let mut limite = limite;
        let mut bloque = [0u8; 16384];
        while cuerpo.len() < largo {
            use std::io::Read;
            if !quedar_hasta(&lectura, limite) {
                responder(&mut escritura, "408 Request Timeout", "<h1>La petición tardó demasiado</h1>");
                return;
            }
            match lectura.read(&mut bloque) {
                Ok(0) | Err(_) => return,
                Ok(leidos) => {
                    cuerpo.extend_from_slice(&bloque[..leidos]);
                    // Quien manda datos de verdad gana tiempo (32 KB por segundo): una foto grande
                    // por una conexión lenta llega; quien manda gota a gota, no.
                    limite += std::time::Duration::from_secs_f64(leidos as f64 / 32768.0);
                }
            }
        }
        cuerpo.truncate(largo);
        match &frontera {
            Some(frontera) => match leer_multipart(&cuerpo, frontera) {
                Some((textos, archivos)) => {
                    campos.extend(textos);
                    subidas = archivos;
                }
                None => {
                    responder(&mut escritura, "400 Bad Request", "<h1>Formulario mal formado</h1>");
                    return;
                }
            },
            None => {
                cuerpo_texto = String::from_utf8_lossy(&cuerpo).into_owned();
                let es_formulario = cabecera("Content-Type")
                    .map(|tipo| tipo.to_ascii_lowercase().starts_with("application/x-www-form-urlencoded"))
                    .unwrap_or(true);
                if es_formulario {
                    campos.extend(decodificar_formulario(&cuerpo_texto));
                }
            }
        }
    }
    let galletas = cabecera("Cookie").map(|v| leer_galletas(&v)).unwrap_or_default();
    match buscar_ruta(rutas, camino) {
        Some((ruta, parametros)) => {
            let texto_peticion = peticion.clone();
            preparar_peticion(envio, campos, galletas);
            let segura = cabecera("X-Forwarded-Proto").map(|v| v.eq_ignore_ascii_case("https")).unwrap_or(false);
            PETICION.with(|peticion| {
                let mut peticion = peticion.borrow_mut();
                peticion.segura = segura;
                peticion.subidas = std::mem::take(&mut subidas);
                peticion.cuerpo = std::mem::take(&mut cuerpo_texto);
                peticion.cabeceras = texto_peticion
                    .lines()
                    .skip(1)
                    .filter_map(|linea| linea.split_once(':'))
                    .map(|(clave, valor)| (clave.trim().to_ascii_lowercase(), valor.trim().to_string()))
                    .collect();
            });
            EN_RUTA.with(|estado| estado.set(true));
            let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (ruta.manejador)(parametros)
            }));
            EN_RUTA.with(|estado| estado.set(false));
            let respuesta = tomar_respuesta();
            match resultado {
                Ok(_) if respuesta.archivo.is_some() => {
                    let archivo = respuesta.archivo.expect("hay archivo");
                    responder_archivo_http(&mut escritura, archivo, &respuesta.cabeceras);
                }
                Ok(valor) => match respuesta.redireccion {
                    Some(destino) => {
                        let mut extra = respuesta.cabeceras;
                        extra.push(format!("Location: {}", destino));
                        responder_con(&mut escritura, "303 See Other", "", &extra);
                    }
                    None => {
                        let cuerpo = cuerpo_pagina(&respuesta.escrito, &valor);
                        responder_con(&mut escritura, "200 OK", &cuerpo, &respuesta.cabeceras);
                    }
                },
                Err(_) => {
                    responder(
                        &mut escritura,
                        "500 Internal Server Error",
                        "<h1>Error interno</h1>",
                    );
                }
            }
        }
        None if camino == "/favicon.ico" || camino == "/favicon.png" => {
            responder_icono(&mut escritura);
        }
        None => responder(
            &mut escritura,
            "404 Not Found",
            PAGINA_404,
        ),
    }
}

fn responder_icono(conexion: &mut std::net::TcpStream) {
    use std::io::Write;
    let cabecera = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nCache-Control: max-age=86400\r\nConnection: close\r\n\r\n",
        ICONO_WEB.len()
    );
    let _ = conexion.write_all(cabecera.as_bytes());
    let _ = conexion.write_all(ICONO_WEB);
    let _ = conexion.flush();
    let _ = conexion.shutdown(std::net::Shutdown::Write);
}

fn servir(puerto: &Valor, rutas: Vec<Ruta>) -> Valor {
    let puerto = match puerto {
        Valor::Numero(n) => *n as u16,
        otro => fallo(format!(
            "servir() espera un número de puerto, no {}",
            otro.tipo()
        )),
    };
    let puerto = match std::env::var("DEIMON_PUERTO") {
        Ok(texto) => match texto.trim().parse::<u16>() {
            Ok(numero) => numero,
            Err(_) => fallo(format!(
                "la variable DEIMON_PUERTO no es un puerto válido: '{}'",
                texto
            )),
        },
        Err(_) => puerto,
    };
    let escucha = match std::net::TcpListener::bind(("127.0.0.1", puerto)) {
        Ok(escucha) => escucha,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let libre = (puerto.saturating_add(1)..=puerto.saturating_add(200))
                .find(|candidato| std::net::TcpListener::bind(("127.0.0.1", *candidato)).is_ok());
            let archivo = std::env::var("DEIMON_ARCHIVO").unwrap_or_else(|_| "tu_programa.dmn".to_string());
            let archivo = if archivo.chars().any(|c| c.is_whitespace()) {
                format!("\"{}\"", archivo)
            } else {
                archivo
            };
            match libre {
                Some(libre) => fallo(format!(
                    "la puerta {} está ocupada por otro programa. Prueba con: DEIMON_PUERTO={} deimon ejecutar {}",
                    puerto, libre, archivo
                )),
                None => fallo(format!(
                    "la puerta {} está ocupada por otro programa y no se encontró una libre cerca",
                    puerto
                )),
            }
        }
        Err(error) => fallo(format!("no se pudo abrir el puerto {}: {}", puerto, error)),
    };
    let puerto = match escucha.local_addr() {
        Ok(direccion) => direccion.port(),
        Err(_) => puerto,
    };
    let limites = limites();
    println!(
        "Servidor listo en http://127.0.0.1:{} (perfil {}, {} hilos)",
        puerto, PERFIL_LN, limites.hilos
    );
    let rutas = std::sync::Arc::new(rutas);
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |informacion| {
        if informacion.payload().downcast_ref::<ErrorRuta>().is_none() {
            anterior(informacion);
        }
    }));

    // Grupo fijo de hilos trabajadores: las conexiones esperan en una cola acotada.
    let (envio, recepcion) =
        std::sync::mpsc::sync_channel::<std::net::TcpStream>(limites.max_conexiones);
    let recepcion = std::sync::Arc::new(std::sync::Mutex::new(recepcion));
    for _ in 0..limites.hilos {
        let recepcion = std::sync::Arc::clone(&recepcion);
        let rutas = std::sync::Arc::clone(&rutas);
        let hilo = std::thread::Builder::new().stack_size(PILA_HILO_WEB).spawn(move || loop {
            if PILA.with(|p| p.get()).0 == 0 {
                marcar_pila(PILA_HILO_WEB);
            }
            let conexion = {
                let guardia = match recepcion.lock() {
                    Ok(guardia) => guardia,
                    Err(_) => break,
                };
                match guardia.recv() {
                    Ok(conexion) => conexion,
                    Err(_) => break,
                }
            };
            atender(conexion, &rutas, &limites);
        });
        if hilo.is_err() {
            eprintln!("Error: no se pudo crear un hilo del servidor (falta memoria)");
            std::process::exit(1);
        }
    }

    for conexion in escucha.incoming() {
        match conexion {
            Ok(conexion) => match envio.try_send(conexion) {
                Ok(()) => {}
                Err(std::sync::mpsc::TrySendError::Full(mut conexion)) => {
                    responder(
                        &mut conexion,
                        "503 Service Unavailable",
                        "<h1>Servidor ocupado, inténtalo de nuevo</h1>",
                    );
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => break,
            },
            Err(_) => continue,
        }
    }
    Valor::Nada
}
