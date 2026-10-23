#[allow(non_camel_case_types)]
type ext_sql_db = std::ffi::c_void;
#[allow(non_camel_case_types)]
type ext_sql_stmt = std::ffi::c_void;

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open_v2(nombre: *const std::ffi::c_char, db: *mut *mut ext_sql_db, flags: i32, vfs: *const std::ffi::c_char) -> i32;
    fn sqlite3_close_v2(db: *mut ext_sql_db) -> i32;
    fn sqlite3_busy_timeout(db: *mut ext_sql_db, ms: i32) -> i32;
    fn sqlite3_errmsg(db: *mut ext_sql_db) -> *const std::ffi::c_char;
    fn sqlite3_prepare_v2(db: *mut ext_sql_db, sql: *const std::ffi::c_char, bytes: i32, stmt: *mut *mut ext_sql_stmt, resto: *mut *const std::ffi::c_char) -> i32;
    fn sqlite3_bind_parameter_count(stmt: *mut ext_sql_stmt) -> i32;
    fn sqlite3_bind_double(stmt: *mut ext_sql_stmt, i: i32, v: f64) -> i32;
    fn sqlite3_bind_int64(stmt: *mut ext_sql_stmt, i: i32, v: i64) -> i32;
    fn sqlite3_bind_text(stmt: *mut ext_sql_stmt, i: i32, t: *const std::ffi::c_char, n: i32, destructor: isize) -> i32;
    fn sqlite3_bind_null(stmt: *mut ext_sql_stmt, i: i32) -> i32;
    fn sqlite3_step(stmt: *mut ext_sql_stmt) -> i32;
    fn sqlite3_column_count(stmt: *mut ext_sql_stmt) -> i32;
    fn sqlite3_column_type(stmt: *mut ext_sql_stmt, i: i32) -> i32;
    fn sqlite3_column_double(stmt: *mut ext_sql_stmt, i: i32) -> f64;
    fn sqlite3_column_int64(stmt: *mut ext_sql_stmt, i: i32) -> i64;
    fn sqlite3_column_text(stmt: *mut ext_sql_stmt, i: i32) -> *const u8;
    fn sqlite3_column_bytes(stmt: *mut ext_sql_stmt, i: i32) -> i32;
    fn sqlite3_finalize(stmt: *mut ext_sql_stmt) -> i32;
    fn sqlite3_changes(db: *mut ext_sql_db) -> i32;
}

const EXT_SQL_FILA: i32 = 100;
const EXT_SQL_HECHO: i32 = 101;
/// SQLITE_TRANSIENT: SQLite copia el texto enseguida.
const EXT_SQL_COPIAR: isize = -1;

struct ExtSqlBase(*mut ext_sql_db);

impl Drop for ExtSqlBase {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close_v2(self.0);
        }
    }
}

fn ext_sql_error(db: *mut ext_sql_db) -> String {
    unsafe {
        let mensaje = sqlite3_errmsg(db);
        if mensaje.is_null() {
            "error desconocido".to_string()
        } else {
            std::ffi::CStr::from_ptr(mensaje).to_string_lossy().into_owned()
        }
    }
}

/// Abre la base, prepara la consulta con sus parámetros y la recorre.
/// Devuelve (filas, filas cambiadas).
fn ext_sql_correr(args: &[Valor], funcion: &str) -> Result<(Vec<Valor>, i32), String> {
    let archivo = match args.first().and_then(|v| v.como_texto()) {
        Some(a) if !a.is_empty() => a.to_string(),
        _ => return Err(format!("{}() espera el archivo de la base de datos, por ejemplo \"tienda.db\"", funcion)),
    };
    let sql = match args.get(1).and_then(|v| v.como_texto()) {
        Some(s) => s.to_string(),
        None => return Err(format!("{}() espera la orden SQL como texto", funcion)),
    };
    let parametros: Vec<Valor> = match args.get(2) {
        Some(v) => match v.como_lista() {
            Some(lista) => lista.to_vec(),
            None => return Err(format!("{}(): los parámetros van en una lista, por ejemplo [\"Ana\", 30] (o [] si no hay)", funcion)),
        },
        None => Vec::new(),
    };
    let nombre = std::ffi::CString::new(archivo.clone()).map_err(|_| format!("{}(): nombre de archivo no válido", funcion))?;
    let texto_sql = std::ffi::CString::new(sql).map_err(|_| format!("{}(): la orden SQL tiene un carácter nulo", funcion))?;
    unsafe {
        let mut db: *mut ext_sql_db = std::ptr::null_mut();
        // LEER | ESCRIBIR | CREAR
        let estado = sqlite3_open_v2(nombre.as_ptr(), &mut db, 0x2 | 0x4, std::ptr::null());
        let base = ExtSqlBase(db);
        if estado != 0 {
            return Err(format!("{}(): no se pudo abrir '{}': {}", funcion, archivo, ext_sql_error(base.0)));
        }
        sqlite3_busy_timeout(base.0, 5000);
        let mut stmt: *mut ext_sql_stmt = std::ptr::null_mut();
        if sqlite3_prepare_v2(base.0, texto_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != 0 {
            return Err(format!("{}(): la orden SQL tiene un error: {}", funcion, ext_sql_error(base.0)));
        }
        if stmt.is_null() {
            return Ok((Vec::new(), 0));
        }
        let esperados = sqlite3_bind_parameter_count(stmt);
        if esperados as usize != parametros.len() {
            sqlite3_finalize(stmt);
            return Err(format!("{}(): la orden tiene {} '?' pero se pasaron {} parámetros", funcion, esperados, parametros.len()));
        }
        let mut textos = Vec::new();
        for (i, valor) in parametros.iter().enumerate() {
            let indice = i as i32 + 1;
            if let Some(n) = valor.como_numero() {
                if n.fract() == 0.0 && n.abs() < 9.0e15 {
                    sqlite3_bind_int64(stmt, indice, n as i64);
                } else {
                    sqlite3_bind_double(stmt, indice, n);
                }
            } else if let Some(t) = valor.como_texto() {
                let c = std::ffi::CString::new(t).map_err(|_| format!("{}(): un parámetro tiene un carácter nulo", funcion))?;
                sqlite3_bind_text(stmt, indice, c.as_ptr(), -1, EXT_SQL_COPIAR);
                textos.push(c);
            } else {
                match texto_de(valor).as_str() {
                    "verdadero" => {
                        sqlite3_bind_int64(stmt, indice, 1);
                    }
                    "falso" => {
                        sqlite3_bind_int64(stmt, indice, 0);
                    }
                    _ => {
                        sqlite3_bind_null(stmt, indice);
                    }
                }
            }
        }
        let mut filas = Vec::new();
        loop {
            match sqlite3_step(stmt) {
                EXT_SQL_FILA => {
                    let columnas = sqlite3_column_count(stmt);
                    let mut fila = Vec::new();
                    for c in 0..columnas {
                        fila.push(match sqlite3_column_type(stmt, c) {
                            1 => Valor::Numero(sqlite3_column_int64(stmt, c) as f64),
                            2 => Valor::Numero(sqlite3_column_double(stmt, c)),
                            5 => Valor::Nada,
                            _ => {
                                let p = sqlite3_column_text(stmt, c);
                                let n = sqlite3_column_bytes(stmt, c) as usize;
                                if p.is_null() {
                                    Valor::Nada
                                } else {
                                    Valor::texto(String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned())
                                }
                            }
                        });
                    }
                    filas.push(Valor::lista(fila));
                }
                EXT_SQL_HECHO => break,
                _ => {
                    let error = ext_sql_error(base.0);
                    sqlite3_finalize(stmt);
                    return Err(format!("{}(): {}", funcion, error));
                }
            }
        }
        sqlite3_finalize(stmt);
        drop(textos);
        Ok((filas, sqlite3_changes(base.0)))
    }
}

fn ext_ejecutar_sql(args: Vec<Valor>) -> Result<Valor, String> {
    let (_, cambios) = ext_sql_correr(&args, "ejecutar_sql")?;
    Ok(Valor::Numero(cambios as f64))
}

fn ext_consultar_sql(args: Vec<Valor>) -> Result<Valor, String> {
    let (filas, _) = ext_sql_correr(&args, "consultar_sql")?;
    Ok(Valor::lista(filas))
}
