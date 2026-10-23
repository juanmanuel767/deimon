// Cliente de MySQL/MariaDB escrito solo con la biblioteca estándar: habla el protocolo de
// MySQL directamente (inicio de sesión, consultas de texto y resultados). Los valores de la
// lista de parámetros se convierten en literales SQL protegidos antes de enviar la consulta,
// como hace PDO de PHP por defecto. Cada hilo guarda su conexión abierta para reutilizarla.

enum ExtMyFlujo {
    Red(std::net::TcpStream),
    #[cfg(unix)]
    Local(std::os::unix::net::UnixStream),
}

impl std::io::Read for ExtMyFlujo {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ExtMyFlujo::Red(f) => f.read(b),
            #[cfg(unix)]
            ExtMyFlujo::Local(f) => f.read(b),
        }
    }
}

impl std::io::Write for ExtMyFlujo {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self {
            ExtMyFlujo::Red(f) => f.write(b),
            #[cfg(unix)]
            ExtMyFlujo::Local(f) => f.write(b),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ExtMyFlujo::Red(f) => f.flush(),
            #[cfg(unix)]
            ExtMyFlujo::Local(f) => f.flush(),
        }
    }
}

struct ExtMyConexion {
    flujo: ExtMyFlujo,
    secuencia: u8,
    /// El servidor trata '\' como un carácter normal (modo NO_BACKSLASH_ESCAPES).
    sin_barras: bool,
    ultimo_id: u64,
}

struct ExtMyDestino {
    usuario: String,
    clave: String,
    servidor: String,
    puerto: u16,
    base: String,
    enchufe: Option<String>,
}

thread_local! {
    static EXT_MYSQL_CONEXIONES: std::cell::RefCell<std::collections::HashMap<String, ExtMyConexion>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn ext_my_decodificar(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut salida = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(n) = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                salida.push(n);
                i += 3;
                continue;
            }
        }
        salida.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&salida).into_owned()
}

fn ext_my_destino(conexion: &str) -> Result<ExtMyDestino, String> {
    let texto = if conexion.is_empty() {
        std::env::var("DEIMON_MYSQL").map_err(|_| "la conexión está vacía y no existe la variable DEIMON_MYSQL (pon en ella mysql://usuario:clave@servidor/base)".to_string())?
    } else {
        conexion.to_string()
    };
    let resto = texto
        .strip_prefix("mysql://")
        .or_else(|| texto.strip_prefix("mariadb://"))
        .ok_or("la conexión debe empezar por mysql://, por ejemplo mysql://usuario:clave@localhost/tienda")?;
    let (resto, opciones) = resto.split_once('?').unwrap_or((resto, ""));
    let (credenciales, lugar) = resto.rsplit_once('@').ok_or("falta el usuario: mysql://usuario:clave@servidor/base")?;
    let (usuario, clave) = credenciales.split_once(':').unwrap_or((credenciales, ""));
    let (direccion, base) = lugar.split_once('/').unwrap_or((lugar, ""));
    let (servidor, puerto) = match direccion.rsplit_once(':') {
        Some((s, p)) => (s, p.parse::<u16>().map_err(|_| format!("el puerto '{}' no es válido", p))?),
        None => (direccion, 3306),
    };
    let mut enchufe = None;
    for opcion in opciones.split('&').filter(|o| !o.is_empty()) {
        if let Some(ruta) = opcion.strip_prefix("socket=") {
            enchufe = Some(ext_my_decodificar(ruta));
        }
    }
    if servidor == "localhost" && enchufe.is_none() {
        enchufe = ["/run/mysqld/mysqld.sock", "/var/run/mysqld/mysqld.sock", "/tmp/mysql.sock"]
            .iter()
            .find(|r| std::path::Path::new(r).exists())
            .map(|r| r.to_string());
    }
    Ok(ExtMyDestino {
        usuario: ext_my_decodificar(usuario),
        clave: ext_my_decodificar(clave),
        servidor: if servidor == "localhost" && enchufe.is_none() { "127.0.0.1".to_string() } else { servidor.to_string() },
        puerto,
        base: ext_my_decodificar(base),
        enchufe,
    })
}

fn ext_my_sha1(datos: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut mensaje = datos.to_vec();
    let bits = (datos.len() as u64).wrapping_mul(8);
    mensaje.push(0x80);
    while mensaje.len() % 64 != 56 {
        mensaje.push(0);
    }
    mensaje.extend_from_slice(&bits.to_be_bytes());
    for bloque in mensaje.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([bloque[4 * i], bloque[4 * i + 1], bloque[4 * i + 2], bloque[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, palabra) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*palabra);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut salida = [0u8; 20];
    for (i, parte) in h.iter().enumerate() {
        salida[4 * i..4 * i + 4].copy_from_slice(&parte.to_be_bytes());
    }
    salida
}

/// Respuesta de mysql_native_password: SHA1(clave) XOR SHA1(semilla + SHA1(SHA1(clave))).
fn ext_my_nativa(clave: &str, semilla: &[u8]) -> Vec<u8> {
    if clave.is_empty() {
        return Vec::new();
    }
    let uno = ext_my_sha1(clave.as_bytes());
    let dos = ext_my_sha1(&uno);
    let mut mezcla = semilla.to_vec();
    mezcla.extend_from_slice(&dos);
    let tres = ext_my_sha1(&mezcla);
    uno.iter().zip(tres.iter()).map(|(a, b)| a ^ b).collect()
}

/// Respuesta de caching_sha2_password: SHA256(clave) XOR SHA256(SHA256(SHA256(clave)) + semilla).
fn ext_my_sha2(clave: &str, semilla: &[u8]) -> Vec<u8> {
    if clave.is_empty() {
        return Vec::new();
    }
    let uno = sha256(clave.as_bytes());
    let dos = sha256(&uno);
    let mut mezcla = dos.to_vec();
    mezcla.extend_from_slice(semilla);
    let tres = sha256(&mezcla);
    uno.iter().zip(tres.iter()).map(|(a, b)| a ^ b).collect()
}

fn ext_my_entero_largo(datos: &[u8], pos: &mut usize) -> Option<u64> {
    let primero = *datos.get(*pos)?;
    *pos += 1;
    let bytes = match primero {
        0..=0xFA => return Some(primero as u64),
        0xFC => 2,
        0xFD => 3,
        0xFE => 8,
        _ => return None,
    };
    let trozo = datos.get(*pos..*pos + bytes)?;
    *pos += bytes;
    let mut n = 0u64;
    for (i, b) in trozo.iter().enumerate() {
        n |= (*b as u64) << (8 * i);
    }
    Some(n)
}

fn ext_my_texto_largo(datos: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let largo = ext_my_entero_largo(datos, pos)? as usize;
    let trozo = datos.get(*pos..*pos + largo)?.to_vec();
    *pos += largo;
    Some(trozo)
}

// ---- Cifrado RSA-OAEP para mandar la contraseña de caching_sha2_password por la red ----
// Números grandes como listas de cifras de 32 bits (la menos significativa primero).

fn ext_my_num_desde(bytes: &[u8]) -> Vec<u32> {
    let mut cifras = Vec::with_capacity(bytes.len() / 4 + 1);
    for trozo in bytes.rchunks(4) {
        let mut n = 0u32;
        for b in trozo {
            n = (n << 8) | *b as u32;
        }
        cifras.push(n);
    }
    ext_my_num_limpiar(&mut cifras);
    cifras
}

fn ext_my_num_limpiar(n: &mut Vec<u32>) {
    while n.len() > 1 && *n.last().unwrap() == 0 {
        n.pop();
    }
}

fn ext_my_num_comparar(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    std::cmp::Ordering::Equal
}

fn ext_my_num_multiplicar(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut r = vec![0u32; a.len() + b.len()];
    for (i, x) in a.iter().enumerate() {
        let mut acarreo = 0u64;
        for (j, y) in b.iter().enumerate() {
            let t = r[i + j] as u64 + (*x as u64) * (*y as u64) + acarreo;
            r[i + j] = t as u32;
            acarreo = t >> 32;
        }
        r[i + b.len()] = acarreo as u32;
    }
    ext_my_num_limpiar(&mut r);
    r
}

/// Resto de a entre m, restando m desplazado bit a bit (sencillo y suficiente para e = 65537).
fn ext_my_num_resto(a: &[u32], m: &[u32]) -> Vec<u32> {
    let mut r: Vec<u32> = vec![0];
    for i in (0..a.len() * 32).rev() {
        // r = r * 2 + bit i de a
        let mut acarreo = (a[i / 32] >> (i % 32)) & 1;
        for cifra in r.iter_mut() {
            let nuevo = (*cifra >> 31) & 1;
            *cifra = (*cifra << 1) | acarreo;
            acarreo = nuevo;
        }
        if acarreo != 0 {
            r.push(acarreo);
        }
        if ext_my_num_comparar(&r, m) != std::cmp::Ordering::Less {
            let mut prestado = 0i64;
            for k in 0..r.len() {
                let t = r[k] as i64 - *m.get(k).unwrap_or(&0) as i64 - prestado;
                if t < 0 {
                    r[k] = (t + (1i64 << 32)) as u32;
                    prestado = 1;
                } else {
                    r[k] = t as u32;
                    prestado = 0;
                }
            }
            ext_my_num_limpiar(&mut r);
        }
    }
    r
}

fn ext_my_num_potencia(base: &[u32], exponente: &[u32], m: &[u32]) -> Vec<u32> {
    let mut resultado = vec![1u32];
    let base = ext_my_num_resto(base, m);
    for i in (0..exponente.len() * 32).rev() {
        resultado = ext_my_num_resto(&ext_my_num_multiplicar(&resultado, &resultado), m);
        if (exponente[i / 32] >> (i % 32)) & 1 == 1 {
            resultado = ext_my_num_resto(&ext_my_num_multiplicar(&resultado, &base), m);
        }
    }
    resultado
}

fn ext_my_base64(texto: &str) -> Vec<u8> {
    let mut salida = Vec::new();
    let mut acumulado = 0u32;
    let mut bits = 0;
    for c in texto.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        } as u32;
        acumulado = (acumulado << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            salida.push((acumulado >> bits) as u8);
        }
    }
    salida
}

/// Lee un elemento DER: devuelve (etiqueta, contenido, posición siguiente).
fn ext_my_der(datos: &[u8], pos: usize) -> Option<(u8, &[u8], usize)> {
    let etiqueta = *datos.get(pos)?;
    let mut i = pos + 1;
    let primero = *datos.get(i)? as usize;
    i += 1;
    let largo = if primero < 0x80 {
        primero
    } else {
        let n = primero & 0x7F;
        let mut l = 0usize;
        for _ in 0..n {
            l = (l << 8) | *datos.get(i)? as usize;
            i += 1;
        }
        l
    };
    Some((etiqueta, datos.get(i..i + largo)?, i + largo))
}

/// Saca módulo y exponente de una llave pública PEM (SubjectPublicKeyInfo o RSAPublicKey).
fn ext_my_llave(pem: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let cuerpo: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = ext_my_base64(&cuerpo);
    let (_, exterior, _) = ext_my_der(&der, 0)?;
    let (etiqueta, primero, siguiente) = ext_my_der(exterior, 0)?;
    let rsa: &[u8] = if etiqueta == 0x30 {
        // SubjectPublicKeyInfo: algoritmo y después BIT STRING con la llave RSA.
        let (_, bits, _) = ext_my_der(exterior, siguiente)?;
        let (_, secuencia, _) = ext_my_der(bits.get(1..)?, 0)?;
        secuencia
    } else {
        let _ = primero;
        exterior
    };
    let (_, modulo, despues) = ext_my_der(rsa, 0)?;
    let (_, exponente, _) = ext_my_der(rsa, despues)?;
    let modulo = modulo.iter().skip_while(|b| **b == 0).cloned().collect();
    Some((modulo, exponente.to_vec()))
}

fn ext_my_mgf1(semilla: &[u8], largo: usize) -> Vec<u8> {
    let mut salida = Vec::with_capacity(largo + 20);
    let mut contador = 0u32;
    while salida.len() < largo {
        let mut d = semilla.to_vec();
        d.extend_from_slice(&contador.to_be_bytes());
        salida.extend_from_slice(&ext_my_sha1(&d));
        contador += 1;
    }
    salida.truncate(largo);
    salida
}

/// Cifra el mensaje con RSA-OAEP (SHA-1), como espera MySQL 8.
fn ext_my_rsa_oaep(pem: &str, mensaje: &[u8]) -> Result<Vec<u8>, String> {
    let (modulo, exponente) = ext_my_llave(pem).ok_or("la llave pública de MySQL no se entiende")?;
    let k = modulo.len();
    if mensaje.len() + 42 > k {
        return Err("la contraseña es demasiado larga para la llave de MySQL".to_string());
    }
    let mut bloque = ext_my_sha1(b"").to_vec();
    bloque.resize(k - mensaje.len() - 22, 0);
    bloque.push(1);
    bloque.extend_from_slice(mensaje);
    let semilla = bytes_aleatorios(20);
    let mascara = ext_my_mgf1(&semilla, k - 21);
    let bloque: Vec<u8> = bloque.iter().zip(mascara).map(|(a, b)| a ^ b).collect();
    let mascara_semilla = ext_my_mgf1(&bloque, 20);
    let mut em = vec![0u8];
    em.extend(semilla.iter().zip(mascara_semilla).map(|(a, b)| a ^ b));
    em.extend_from_slice(&bloque);
    let c = ext_my_num_potencia(&ext_my_num_desde(&em), &ext_my_num_desde(&exponente), &ext_my_num_desde(&modulo));
    let mut salida = vec![0u8; k];
    for (i, cifra) in c.iter().enumerate() {
        for j in 0..4 {
            let posicion = i * 4 + j;
            if posicion < k {
                salida[k - 1 - posicion] = (cifra >> (8 * j)) as u8;
            }
        }
    }
    Ok(salida)
}

impl ExtMyConexion {
    fn leer(&mut self) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let mut datos = Vec::new();
        loop {
            let mut cabecera = [0u8; 4];
            self.flujo.read_exact(&mut cabecera).map_err(|e| format!("se cortó la conexión con MySQL: {}", e))?;
            let largo = cabecera[0] as usize | (cabecera[1] as usize) << 8 | (cabecera[2] as usize) << 16;
            self.secuencia = cabecera[3].wrapping_add(1);
            let inicio = datos.len();
            datos.resize(inicio + largo, 0);
            self.flujo.read_exact(&mut datos[inicio..]).map_err(|e| format!("se cortó la conexión con MySQL: {}", e))?;
            if largo < 0xFF_FFFF {
                return Ok(datos);
            }
        }
    }

    fn enviar(&mut self, datos: &[u8]) -> Result<(), String> {
        use std::io::Write;
        let mut trozos = datos.chunks(0xFF_FFFF).peekable();
        let mut paquete = Vec::with_capacity(datos.len() + 8);
        if trozos.peek().is_none() {
            paquete.extend_from_slice(&[0, 0, 0, self.secuencia]);
            self.secuencia = self.secuencia.wrapping_add(1);
        }
        let mut ultimo_lleno = false;
        for trozo in trozos {
            let n = trozo.len();
            paquete.extend_from_slice(&[n as u8, (n >> 8) as u8, (n >> 16) as u8, self.secuencia]);
            paquete.extend_from_slice(trozo);
            self.secuencia = self.secuencia.wrapping_add(1);
            ultimo_lleno = n == 0xFF_FFFF;
        }
        if ultimo_lleno {
            paquete.extend_from_slice(&[0, 0, 0, self.secuencia]);
            self.secuencia = self.secuencia.wrapping_add(1);
        }
        self.flujo.write_all(&paquete).map_err(|e| format!("no se pudo enviar a MySQL: {}", e))
    }

    fn error_de(paquete: &[u8]) -> String {
        let codigo = paquete.get(1..3).map(|c| u16::from_le_bytes([c[0], c[1]])).unwrap_or(0);
        let inicio = if paquete.get(3) == Some(&b'#') { 9 } else { 3 };
        let mensaje = String::from_utf8_lossy(paquete.get(inicio..).unwrap_or(&[]));
        format!("MySQL respondió con un error ({}): {}", codigo, mensaje)
    }

    /// Lee un paquete OK: filas cambiadas, último id y estado del servidor.
    fn leer_ok(&mut self, paquete: &[u8]) -> u64 {
        let mut pos = 1;
        let filas = ext_my_entero_largo(paquete, &mut pos).unwrap_or(0);
        self.ultimo_id = ext_my_entero_largo(paquete, &mut pos).unwrap_or(0);
        if let Some(estado) = paquete.get(pos..pos + 2) {
            self.sin_barras = u16::from_le_bytes([estado[0], estado[1]]) & 0x0200 != 0;
        }
        filas
    }

    fn abrir(destino: &ExtMyDestino) -> Result<ExtMyConexion, String> {
        let espera = Some(std::time::Duration::from_secs(30));
        let es_local;
        let flujo = match &destino.enchufe {
            #[cfg(unix)]
            Some(ruta) => {
                let f = std::os::unix::net::UnixStream::connect(ruta).map_err(|e| format!("no se pudo abrir el enchufe de MySQL '{}': {}", ruta, e))?;
                let _ = f.set_read_timeout(espera);
                let _ = f.set_write_timeout(espera);
                es_local = true;
                ExtMyFlujo::Local(f)
            }
            #[cfg(not(unix))]
            Some(_) => return Err("los enchufes locales de MySQL solo existen en Linux y macOS; usa 127.0.0.1".to_string()),
            None => {
                let f = std::net::TcpStream::connect((destino.servidor.as_str(), destino.puerto))
                    .map_err(|e| format!("no se pudo conectar con MySQL en {}:{}: {}", destino.servidor, destino.puerto, e))?;
                let _ = f.set_read_timeout(espera);
                let _ = f.set_write_timeout(espera);
                let _ = f.set_nodelay(true);
                es_local = false;
                ExtMyFlujo::Red(f)
            }
        };
        let mut c = ExtMyConexion { flujo, secuencia: 0, sin_barras: false, ultimo_id: 0 };

        // Saludo del servidor (protocolo 10).
        let saludo = c.leer()?;
        if saludo.first() == Some(&0xFF) {
            return Err(Self::error_de(&saludo));
        }
        if saludo.first() != Some(&10) {
            return Err("el servidor no habla el protocolo de MySQL 4.1 o posterior".to_string());
        }
        let mut pos = 1;
        while saludo.get(pos).map_or(false, |b| *b != 0) {
            pos += 1;
        }
        pos += 1 + 4; // fin de la versión + id de conexión
        let mut semilla = saludo.get(pos..pos + 8).ok_or("saludo de MySQL incompleto")?.to_vec();
        pos += 8 + 1;
        let capacidades_bajas = saludo.get(pos..pos + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32).unwrap_or(0);
        pos += 2 + 1 + 2; // capacidades bajas, juego de caracteres, estado
        let capacidades_altas = saludo.get(pos..pos + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32).unwrap_or(0);
        pos += 2;
        let capacidades_servidor = capacidades_bajas | (capacidades_altas << 16);
        let largo_semilla = *saludo.get(pos).unwrap_or(&0) as usize;
        pos += 1 + 10;
        let resto_semilla = largo_semilla.saturating_sub(8).max(13);
        if let Some(trozo) = saludo.get(pos..pos + resto_semilla) {
            semilla.extend_from_slice(trozo.strip_suffix(&[0]).unwrap_or(trozo));
        }
        pos += resto_semilla;
        let mut metodo = String::from_utf8_lossy(saludo.get(pos..).unwrap_or(&[])).trim_end_matches('\0').to_string();
        if metodo.is_empty() {
            metodo = "mysql_native_password".to_string();
        }

        const PROTOCOLO_41: u32 = 0x200;
        const CONEXION_SEGURA: u32 = 0x8000;
        const CON_BASE: u32 = 0x8;
        const CLAVE_LARGA: u32 = 0x1;
        const TRANSACCIONES: u32 = 0x2000;
        const METODO_ACCESO: u32 = 0x80000;
        const DATOS_LARGOS: u32 = 0x200000;
        if capacidades_servidor & PROTOCOLO_41 == 0 {
            return Err("el servidor MySQL es demasiado antiguo".to_string());
        }
        let mut capacidades = PROTOCOLO_41 | CONEXION_SEGURA | CLAVE_LARGA | TRANSACCIONES | METODO_ACCESO | DATOS_LARGOS;
        if !destino.base.is_empty() {
            capacidades |= CON_BASE;
        }
        let respuesta = match metodo.as_str() {
            "caching_sha2_password" => ext_my_sha2(&destino.clave, &semilla),
            _ => ext_my_nativa(&destino.clave, &semilla),
        };
        let mut paquete = Vec::new();
        paquete.extend_from_slice(&capacidades.to_le_bytes());
        paquete.extend_from_slice(&(16u32 * 1024 * 1024).to_le_bytes());
        paquete.push(45); // utf8mb4
        paquete.extend_from_slice(&[0u8; 23]);
        paquete.extend_from_slice(destino.usuario.as_bytes());
        paquete.push(0);
        paquete.push(respuesta.len() as u8);
        paquete.extend_from_slice(&respuesta);
        if !destino.base.is_empty() {
            paquete.extend_from_slice(destino.base.as_bytes());
            paquete.push(0);
        }
        paquete.extend_from_slice(metodo.as_bytes());
        paquete.push(0);
        c.enviar(&paquete)?;

        // Ida y vuelta del inicio de sesión: cambio de método, datos extra, OK o error.
        loop {
            let respuesta = c.leer()?;
            match respuesta.first() {
                Some(0x00) => {
                    c.leer_ok(&respuesta);
                    return Ok(c);
                }
                Some(0xFF) => return Err(Self::error_de(&respuesta)),
                Some(0xFE) => {
                    let mut fin = 1;
                    while respuesta.get(fin).map_or(false, |b| *b != 0) {
                        fin += 1;
                    }
                    metodo = String::from_utf8_lossy(&respuesta[1..fin]).to_string();
                    let nueva = respuesta.get(fin + 1..).unwrap_or(&[]);
                    semilla = nueva.strip_suffix(&[0]).unwrap_or(nueva).to_vec();
                    let datos = match metodo.as_str() {
                        "mysql_native_password" => ext_my_nativa(&destino.clave, &semilla),
                        "caching_sha2_password" => ext_my_sha2(&destino.clave, &semilla),
                        "mysql_clear_password" if es_local => {
                            let mut d = destino.clave.as_bytes().to_vec();
                            d.push(0);
                            d
                        }
                        otro => return Err(format!("MySQL pide el método de acceso '{}', que Deimon aún no sabe usar; crea el usuario con mysql_native_password o caching_sha2_password", otro)),
                    };
                    c.enviar(&datos)?;
                }
                Some(0x01) => match respuesta.get(1) {
                    Some(0x03) => {} // acceso rápido aceptado: llega un OK
                    Some(0x04) => {
                        // caching_sha2 pide la clave completa: por el enchufe local va tal cual; por la
                        // red se cifra con la llave pública del servidor (RSA-OAEP).
                        let mut d = destino.clave.as_bytes().to_vec();
                        d.push(0);
                        if es_local {
                            c.enviar(&d)?;
                        } else {
                            c.enviar(&[0x02])?;
                            let llave = c.leer()?;
                            if llave.first() != Some(&0x01) {
                                return Err("MySQL no envió su llave pública para mandar la contraseña".to_string());
                            }
                            let pem = String::from_utf8_lossy(&llave[1..]).into_owned();
                            let mezclada: Vec<u8> = d.iter().enumerate().map(|(i, b)| b ^ semilla[i % semilla.len().max(1)]).collect();
                            let cifrada = ext_my_rsa_oaep(&pem, &mezclada)?;
                            c.enviar(&cifrada)?;
                        }
                    }
                    _ => return Err("respuesta de inicio de sesión de MySQL que no se entiende".to_string()),
                },
                _ => return Err("respuesta de inicio de sesión de MySQL que no se entiende".to_string()),
            }
        }
    }

    /// Envía una consulta. Devuelve (filas cambiadas, columnas, filas).
    fn consulta(&mut self, sql: &str) -> Result<(u64, Vec<(String, u8)>, Vec<Vec<Option<Vec<u8>>>>), String> {
        self.secuencia = 0;
        let mut paquete = vec![0x03];
        paquete.extend_from_slice(sql.as_bytes());
        self.enviar(&paquete)?;
        let primero = self.leer()?;
        match primero.first() {
            Some(0x00) => return Ok((self.leer_ok(&primero), Vec::new(), Vec::new())),
            Some(0xFF) => return Err(Self::error_de(&primero)),
            Some(0xFB) => return Err("MySQL pidió leer un archivo local (LOAD DATA LOCAL), y Deimon no lo permite".to_string()),
            _ => {}
        }
        let mut pos = 0;
        let cantidad = ext_my_entero_largo(&primero, &mut pos).ok_or("respuesta de MySQL que no se entiende")? as usize;
        let mut columnas = Vec::with_capacity(cantidad);
        for _ in 0..cantidad {
            let definicion = self.leer()?;
            let mut pos = 0;
            for _ in 0..4 {
                ext_my_texto_largo(&definicion, &mut pos).ok_or("columna de MySQL mal formada")?; // catálogo, base, tabla, tabla original
            }
            let nombre = ext_my_texto_largo(&definicion, &mut pos).ok_or("columna de MySQL mal formada")?;
            ext_my_texto_largo(&definicion, &mut pos).ok_or("columna de MySQL mal formada")?; // nombre original
            ext_my_entero_largo(&definicion, &mut pos).ok_or("columna de MySQL mal formada")?; // 0x0c
            pos += 2 + 4; // juego de caracteres y longitud
            let tipo = *definicion.get(pos).ok_or("columna de MySQL mal formada")?;
            columnas.push((String::from_utf8_lossy(&nombre).into_owned(), tipo));
        }
        let fin = self.leer()?; // EOF tras las columnas
        if fin.first() == Some(&0xFF) {
            return Err(Self::error_de(&fin));
        }
        let mut filas = Vec::new();
        loop {
            let fila = self.leer()?;
            match fila.first() {
                Some(0xFE) if fila.len() < 9 => break,
                Some(0xFF) => return Err(Self::error_de(&fila)),
                _ => {}
            }
            let mut pos = 0;
            let mut valores = Vec::with_capacity(cantidad);
            for _ in 0..cantidad {
                if fila.get(pos) == Some(&0xFB) {
                    pos += 1;
                    valores.push(None);
                } else {
                    valores.push(Some(ext_my_texto_largo(&fila, &mut pos).ok_or("fila de MySQL mal formada")?));
                }
            }
            filas.push(valores);
        }
        Ok((0, columnas, filas))
    }
}

/// Convierte un valor de Deimon en un literal SQL seguro.
fn ext_my_literal(valor: &Valor, sin_barras: bool) -> Result<String, String> {
    match valor {
        Valor::Nada => Ok("NULL".to_string()),
        Valor::Logico(b) => Ok(if *b { "TRUE" } else { "FALSE" }.to_string()),
        Valor::Numero(n) if n.is_finite() => Ok(texto_de(valor)),
        Valor::Numero(_) => Err("no se puede guardar en MySQL un número infinito".to_string()),
        Valor::Texto(t) => {
            let mut salida = String::with_capacity(t.len() + 2);
            salida.push('\'');
            for c in t.chars() {
                match c {
                    '\'' => salida.push_str("''"),
                    '\\' if !sin_barras => salida.push_str("\\\\"),
                    '\0' if !sin_barras => salida.push_str("\\0"),
                    '\x1a' if !sin_barras => salida.push_str("\\Z"),
                    otro => salida.push(otro),
                }
            }
            salida.push('\'');
            Ok(salida)
        }
        otro => Err(format!("un parámetro de MySQL no puede ser {}", otro.tipo())),
    }
}

/// Pone cada parámetro en el lugar de su '?', sin tocar los '?' que estén dentro de textos,
/// nombres entre comillas invertidas o comentarios.
fn ext_my_rellenar(sql: &str, parametros: &[Valor], sin_barras: bool) -> Result<String, String> {
    let letras: Vec<char> = sql.chars().collect();
    let mut salida = String::with_capacity(sql.len() + 16);
    let mut usados = 0;
    let mut i = 0;
    while i < letras.len() {
        let c = letras[i];
        match c {
            '\'' | '"' | '`' => {
                salida.push(c);
                i += 1;
                while i < letras.len() {
                    salida.push(letras[i]);
                    if letras[i] == '\\' && c != '`' && !sin_barras && i + 1 < letras.len() {
                        salida.push(letras[i + 1]);
                        i += 2;
                        continue;
                    }
                    if letras[i] == c {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            '-' if letras.get(i + 1) == Some(&'-') => {
                while i < letras.len() && letras[i] != '\n' {
                    salida.push(letras[i]);
                    i += 1;
                }
                continue;
            }
            '#' => {
                while i < letras.len() && letras[i] != '\n' {
                    salida.push(letras[i]);
                    i += 1;
                }
                continue;
            }
            '/' if letras.get(i + 1) == Some(&'*') => {
                while i < letras.len() && !(letras[i] == '*' && letras.get(i + 1) == Some(&'/')) {
                    salida.push(letras[i]);
                    i += 1;
                }
                if i < letras.len() {
                    salida.push_str("*/");
                    i += 2;
                }
                continue;
            }
            '?' => {
                let valor = parametros.get(usados).ok_or_else(|| {
                    format!("la consulta tiene más '?' que valores en la lista ({})", parametros.len())
                })?;
                salida.push_str(&ext_my_literal(valor, sin_barras)?);
                usados += 1;
            }
            otro => salida.push(otro),
        }
        i += 1;
    }
    if usados != parametros.len() {
        return Err(format!("la consulta tiene {} '?' y la lista {} valores", usados, parametros.len()));
    }
    Ok(salida)
}

/// Usa la conexión de este hilo (la abre si hace falta) y descarta las que fallen.
fn ext_my_con<T>(conexion: &str, accion: impl FnOnce(&mut ExtMyConexion) -> Result<T, String>) -> Result<T, String> {
    let destino = ext_my_destino(conexion)?;
    let clave = format!("{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{:?}", destino.usuario, destino.clave, destino.servidor, destino.puerto, destino.base, destino.enchufe);
    let mut abierta = EXT_MYSQL_CONEXIONES.with(|c| c.borrow_mut().remove(&clave));
    if abierta.is_none() {
        abierta = Some(ExtMyConexion::abrir(&destino)?);
    }
    let mut c = abierta.expect("conexión abierta");
    let resultado = accion(&mut c);
    // Si la conexión se cortó no se reutiliza; si MySQL solo respondió con un error, sí.
    let reutilizable = match &resultado {
        Err(e) => e.starts_with("MySQL respondió") || e.starts_with("la consulta") || e.starts_with("un parámetro") || e.starts_with("no se puede guardar"),
        Ok(_) => true,
    };
    if reutilizable {
        EXT_MYSQL_CONEXIONES.with(|m| m.borrow_mut().insert(clave, c));
    }
    resultado
}

fn ext_my_argumentos(args: &[Valor], funcion: &str) -> Result<(String, String, Vec<Valor>), String> {
    let conexion = args.first().and_then(|v| v.como_texto()).ok_or(format!("{}() espera el texto de la conexión como primer argumento", funcion))?;
    let sql = args.get(1).and_then(|v| v.como_texto()).ok_or(format!("{}() espera la consulta SQL como segundo argumento", funcion))?;
    let parametros = match args.get(2) {
        Some(Valor::Lista(l)) => l.iter().cloned().collect(),
        Some(Valor::Nada) | None => Vec::new(),
        Some(otro) => return Err(format!("{}() espera los valores en una lista, no en {}", funcion, otro.tipo())),
    };
    Ok((conexion.to_string(), sql.to_string(), parametros))
}

fn ext_ejecutar_mysql(args: Vec<Valor>) -> Result<Valor, String> {
    let (conexion, sql, parametros) = ext_my_argumentos(&args, "ejecutar_mysql")?;
    ext_my_con(&conexion, |c| {
        let consulta = ext_my_rellenar(&sql, &parametros, c.sin_barras)?;
        let (filas, _, _) = c.consulta(&consulta)?;
        Ok(Valor::Numero(filas as f64))
    })
    .map_err(|e| format!("ejecutar_mysql(): {}", e))
}

fn ext_consultar_mysql(args: Vec<Valor>) -> Result<Valor, String> {
    let (conexion, sql, parametros) = ext_my_argumentos(&args, "consultar_mysql")?;
    ext_my_con(&conexion, |c| {
        let consulta = ext_my_rellenar(&sql, &parametros, c.sin_barras)?;
        let (_, columnas, filas) = c.consulta(&consulta)?;
        let mut lista = Vec::with_capacity(filas.len());
        for fila in filas {
            let mut pares = Vec::with_capacity(columnas.len());
            for ((nombre, tipo), dato) in columnas.iter().zip(fila) {
                let valor = match dato {
                    None => Valor::Nada,
                    Some(bytes) => {
                        let texto = String::from_utf8_lossy(&bytes).into_owned();
                        // Tipos numéricos de MySQL: decimales, enteros, flotantes y año.
                        let numerico = matches!(tipo, 0 | 1 | 2 | 3 | 4 | 5 | 8 | 9 | 13 | 246);
                        match texto.parse::<f64>() {
                            Ok(n) if numerico => Valor::Numero(n),
                            _ => Valor::texto(texto),
                        }
                    }
                };
                pares.push((nombre.clone(), valor));
            }
            lista.push(Valor::mapa(pares));
        }
        Ok(Valor::lista(lista))
    })
    .map_err(|e| format!("consultar_mysql(): {}", e))
}

fn ext_ultimo_id_mysql(args: Vec<Valor>) -> Result<Valor, String> {
    let conexion = args.first().and_then(|v| v.como_texto()).ok_or("ultimo_id_mysql() espera el texto de la conexión")?.to_string();
    ext_my_con(&conexion, |c| Ok(Valor::Numero(c.ultimo_id as f64))).map_err(|e| format!("ultimo_id_mysql(): {}", e))
}
