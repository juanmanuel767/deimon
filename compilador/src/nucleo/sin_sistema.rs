// ---------------------------------------------------------------------------
// Código de apoyo para el destino «pc-sin-sistema»: el programa es el sistema operativo.
// Lo arranca GRUB (multiboot2) en un PC con BIOS o en QEMU. No hay Linux debajo: la
// pantalla, el teclado, la memoria y los errores los lleva este archivo.
// El comportamiento de los valores (cuentas, textos, listas, mapas) es el de nucleo/base.rs:
// el traductor pega aquí sus partes puras, entre las marcas @PURO_INICIO@ y @PURO_FIN@.
// ---------------------------------------------------------------------------

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// Lo poco de `std` que usa el código traducido, hecho para una máquina sin sistema.
#[allow(dead_code)]
mod std {
    pub use alloc::{borrow, boxed, rc, string, vec};
    pub use core::{any, cell, cmp, fmt, iter, mem, ops, ptr};
    pub mod collections {
        pub use alloc::collections::*;
        /// Sin sistema no hay azar para HashMap: el orden lo guarda MapaValores aparte.
        pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    }
    pub mod env {
        pub fn var_os(_nombre: &str) -> Option<()> {
            None
        }
        pub fn current_exe() -> Result<(), ()> {
            Err(())
        }
    }
    pub mod fs {
        pub fn remove_file(_ruta: ()) -> Result<(), ()> {
            Err(())
        }
    }
    pub mod panic {
        pub use super::super::{catch_unwind, panic_any, resume_unwind, AssertUnwindSafe};
    }
}

use std::rc::Rc;

// --- Arranque: cabecera multiboot2, paso a 64 bits y salto a principal_nucleo ----------

core::arch::global_asm!(
    r#"
.section .multiboot, "a"
.align 8
cab_ini:
    .long 0xe85250d6
    .long 0
    .long cab_fin - cab_ini
    .long -(0xe85250d6 + 0 + (cab_fin - cab_ini))
    .short 0
    .short 0
    .long 8
cab_fin:

.section .bss
.align 4096
tabla_p4: .skip 4096
tabla_p3: .skip 4096
tabla_p2: .skip 4096 * 4
pila_fondo: .skip 16 * 1024 * 1024
pila_cima:

.section .rodata
.align 8
gdt64:
    .quad 0
    .quad (1<<43) | (1<<44) | (1<<47) | (1<<53)
gdt64_ptr:
    .short . - gdt64 - 1
    .quad gdt64

.section .text
.code32
.global _start
_start:
    mov esp, offset pila_cima
    mov eax, offset tabla_p3
    or eax, 3
    mov [tabla_p4], eax
    mov ecx, 0
1:
    mov eax, ecx
    shl eax, 12
    add eax, offset tabla_p2
    or eax, 3
    mov [tabla_p3 + ecx * 8], eax
    inc ecx
    cmp ecx, 4
    jne 1b
    mov ecx, 0
2:
    mov eax, 0x200000
    mul ecx
    or eax, 0x83
    mov [tabla_p2 + ecx * 8], eax
    mov [tabla_p2 + ecx * 8 + 4], edx
    inc ecx
    cmp ecx, 2048
    jne 2b
    mov eax, offset tabla_p4
    mov cr3, eax
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    lgdt [gdt64_ptr]
    .att_syntax prefix
    ljmp $0x08, $largo
    .intel_syntax noprefix
.code64
largo:
    xor ax, ax
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov rsp, offset pila_cima
    call principal_nucleo
3:
    cli
    hlt
    jmp 3b

# deimon_proteger(funcion, datos, contexto) -> 0 si terminó, 1 si alguien saltó fuera
.global deimon_proteger
deimon_proteger:
    mov [rdx + 0], rbx
    mov [rdx + 8], rbp
    mov [rdx + 16], r12
    mov [rdx + 24], r13
    mov [rdx + 32], r14
    mov [rdx + 40], r15
    lea rax, [rsp + 8]
    mov [rdx + 48], rax
    mov rax, [rsp]
    mov [rdx + 56], rax
    mov rax, rdi
    mov rdi, rsi
    sub rsp, 8
    call rax
    add rsp, 8
    xor eax, eax
    ret

# deimon_saltar(contexto): vuelve de su deimon_proteger devolviendo 1
.global deimon_saltar
deimon_saltar:
    mov rbx, [rdi + 0]
    mov rbp, [rdi + 8]
    mov r12, [rdi + 16]
    mov r13, [rdi + 24]
    mov r14, [rdi + 32]
    mov r15, [rdi + 40]
    mov rsp, [rdi + 48]
    mov eax, 1
    jmp qword ptr [rdi + 56]
"#
);

extern "C" {
    fn deimon_proteger(funcion: extern "C" fn(*mut u8), datos: *mut u8, contexto: *mut [u64; 8]) -> u64;
    fn deimon_saltar(contexto: *const [u64; 8]) -> !;
}

// --- Un solo procesador y sin hilos: «celdas globales» en vez de thread_local --------

struct Global<T>(core::cell::UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}
impl<T> Global<T> {
    const fn new(valor: T) -> Self {
        Global(core::cell::UnsafeCell::new(valor))
    }
    #[allow(clippy::mut_from_ref)]
    fn tomar(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

// --- Memoria: listas libres por tamaño (de 16 B a 4 KB) y bloques grandes aparte ---------

const TAMANO_MONTON: usize = 192 << 20;

#[repr(C, align(4096))]
struct Monton([u8; TAMANO_MONTON]);
static MONTON: Global<Monton> = Global::new(Monton([0; TAMANO_MONTON]));
static USADO: Global<usize> = Global::new(0);
static LIBRES: Global<[usize; 9]> = Global::new([0; 9]);
static GRANDES: Global<usize> = Global::new(0);

struct Asignador;

fn clase_de(tamano: usize) -> Option<usize> {
    let mut clase = 0;
    let mut medida = 16;
    while medida < tamano {
        medida *= 2;
        clase += 1;
    }
    if clase < 9 {
        Some(clase)
    } else {
        None
    }
}

unsafe fn tomar_del_monton(tamano: usize, alineacion: usize) -> *mut u8 {
    let base = MONTON.tomar().0.as_mut_ptr() as usize;
    let usado = USADO.tomar();
    let inicio = (base + *usado + alineacion - 1) & !(alineacion - 1);
    if inicio + tamano > base + TAMANO_MONTON {
        return core::ptr::null_mut();
    }
    *usado = inicio + tamano - base;
    inicio as *mut u8
}

unsafe impl core::alloc::GlobalAlloc for Asignador {
    unsafe fn alloc(&self, forma: core::alloc::Layout) -> *mut u8 {
        let tamano = forma.size().max(forma.align()).max(16);
        if let Some(clase) = clase_de(tamano) {
            let libres = &mut LIBRES.tomar()[clase];
            if *libres != 0 {
                let bloque = *libres;
                *libres = *(bloque as *const usize);
                return bloque as *mut u8;
            }
            let medida = 16usize << clase;
            return tomar_del_monton(medida, medida.min(4096));
        }
        // Grande: cabecera de 16 bytes con el tamaño; primero se busca en los liberados.
        let mut anterior: *mut usize = GRANDES.tomar();
        let mut actual = *anterior;
        while actual != 0 {
            let cabecera = (actual - 16) as *mut usize;
            let siguiente = *(actual as *const usize);
            if *cabecera >= tamano && actual % forma.align() == 0 {
                *anterior = siguiente;
                return actual as *mut u8;
            }
            anterior = actual as *mut usize;
            actual = siguiente;
        }
        let alineacion = forma.align().max(16);
        let crudo = tomar_del_monton(tamano + alineacion.max(16), alineacion);
        if crudo.is_null() {
            return crudo;
        }
        let bloque = crudo.add(alineacion.max(16));
        *((bloque as usize - 16) as *mut usize) = tamano;
        bloque
    }

    unsafe fn dealloc(&self, puntero: *mut u8, forma: core::alloc::Layout) {
        let tamano = forma.size().max(forma.align()).max(16);
        let lista = match clase_de(tamano) {
            Some(clase) => &mut LIBRES.tomar()[clase],
            None => GRANDES.tomar(),
        };
        *(puntero as *mut usize) = *lista;
        *lista = puntero as usize;
    }
}

#[global_allocator]
static ASIGNADOR: Asignador = Asignador;


// --- Puertos de entrada y salida --------------------------------------------------------

fn escribir_puerto(puerto: u16, valor: u8) {
    unsafe { core::arch::asm!("out dx, al", in("dx") puerto, in("al") valor, options(nomem, nostack)) }
}

fn leer_puerto(puerto: u16) -> u8 {
    let valor: u8;
    unsafe { core::arch::asm!("in al, dx", out("al") valor, in("dx") puerto, options(nomem, nostack)) }
    valor
}

fn escribir_puerto16(puerto: u16, valor: u16) {
    unsafe { core::arch::asm!("out dx, ax", in("dx") puerto, in("ax") valor, options(nomem, nostack)) }
}

// --- Puerto serie COM1: copia de la salida y segunda entrada (para pruebas con QEMU) ----

const COM1: u16 = 0x3f8;

fn preparar_serie() {
    escribir_puerto(COM1 + 1, 0x00);
    escribir_puerto(COM1 + 3, 0x80);
    escribir_puerto(COM1, 0x01);
    escribir_puerto(COM1 + 1, 0x00);
    escribir_puerto(COM1 + 3, 0x03);
    // Sin cola (FIFO): activarla vaciaría lo que ya se haya tecleado antes de arrancar.
    escribir_puerto(COM1 + 2, 0x00);
    escribir_puerto(COM1 + 4, 0x03);
}

fn serie_escribir(byte: u8) {
    let mut espera = 0;
    while leer_puerto(COM1 + 5) & 0x20 == 0 && espera < 100_000 {
        espera += 1;
    }
    escribir_puerto(COM1, byte);
}

// --- Pantalla de texto VGA (80 × 25) ------------------------------------------------------

const ANCHO: usize = 80;
const ALTO: usize = 25;

struct Pantalla {
    fila: usize,
    columna: usize,
    color: u8,
}

static PANTALLA: Global<Pantalla> = Global::new(Pantalla { fila: 0, columna: 0, color: 0x07 });

fn celda(fila: usize, columna: usize) -> *mut u16 {
    (0xb8000 + (fila * ANCHO + columna) * 2) as *mut u16
}

/// Carácter de la tabla 437 de la pantalla (las letras con acento del español están).
fn a_cp437(c: char) -> u8 {
    match c {
        'á' => 0xa0, 'é' => 0x82, 'í' => 0xa1, 'ó' => 0xa2, 'ú' => 0xa3,
        'ñ' => 0xa4, 'Ñ' => 0xa5, 'ü' => 0x81, 'Ü' => 0x9a, 'É' => 0x90,
        'Á' => b'A', 'Í' => b'I', 'Ó' => b'O', 'Ú' => b'U',
        '¿' => 0xa8, '¡' => 0xad, 'º' => 0xa7, 'ª' => 0xa6, '·' => 0xfa,
        '█' => 0xdb, '▓' => 0xb2, '▒' => 0xb1, '░' => 0xb0, '─' => 0xc4, '│' => 0xb3,
        '┌' => 0xda, '┐' => 0xbf, '└' => 0xc0, '┘' => 0xd9, '•' => 0xf9,
        c if (c as u32) < 0x7f && (c as u32) >= 0x20 => c as u8,
        _ => b'?',
    }
}

fn mover_cursor(fila: usize, columna: usize) {
    let posicion = (fila * ANCHO + columna) as u16;
    escribir_puerto(0x3d4, 0x0f);
    escribir_puerto(0x3d5, (posicion & 0xff) as u8);
    escribir_puerto(0x3d4, 0x0e);
    escribir_puerto(0x3d5, (posicion >> 8) as u8);
}

fn subir_pantalla(p: &mut Pantalla) {
    unsafe {
        for fila in 1..ALTO {
            for columna in 0..ANCHO {
                celda(fila - 1, columna).write_volatile(celda(fila, columna).read_volatile());
            }
        }
        for columna in 0..ANCHO {
            celda(ALTO - 1, columna).write_volatile((p.color as u16) << 8 | b' ' as u16);
        }
    }
    p.fila = ALTO - 1;
}

fn pantalla_caracter(c: char) {
    let p = PANTALLA.tomar();
    match c {
        '\n' => {
            p.columna = 0;
            p.fila += 1;
        }
        '\r' => p.columna = 0,
        '\t' => {
            p.columna = (p.columna / 4 + 1) * 4;
        }
        '\u{8}' => {
            if p.columna > 0 {
                p.columna -= 1;
                unsafe { celda(p.fila, p.columna).write_volatile((p.color as u16) << 8 | b' ' as u16) }
            }
        }
        c => {
            unsafe { celda(p.fila, p.columna).write_volatile((p.color as u16) << 8 | a_cp437(c) as u16) }
            p.columna += 1;
        }
    }
    if p.columna >= ANCHO {
        p.columna = 0;
        p.fila += 1;
    }
    if p.fila >= ALTO {
        subir_pantalla(p);
    }
    mover_cursor(p.fila, p.columna);
}

fn salida_texto(texto: &str) {
    for c in texto.chars() {
        pantalla_caracter(c);
        if c == '\n' {
            serie_escribir(b'\r');
        }
        let mut bytes = [0u8; 4];
        for b in c.encode_utf8(&mut bytes).bytes() {
            serie_escribir(b);
        }
    }
}

fn limpiar_toda_la_pantalla() {
    let p = PANTALLA.tomar();
    for fila in 0..ALTO {
        for columna in 0..ANCHO {
            unsafe { celda(fila, columna).write_volatile((p.color as u16) << 8 | b' ' as u16) }
        }
    }
    p.fila = 0;
    p.columna = 0;
    mover_cursor(0, 0);
}

// --- Teclado PS/2 (distribución española) y entrada por el puerto serie --------------------

static MAYUSCULAS: Global<bool> = Global::new(false);
static BLOQ_MAYUS: Global<bool> = Global::new(false);
static EXTENDIDA: Global<bool> = Global::new(false);
static SERIE_ESCAPE: Global<u8> = Global::new(0);

const TECLAS: &[u8; 58] = b"\x00\x1b1234567890'\x00\x08\tqwertyuiop`+\n\x00asdfghjkl\x00\x00\x00\x00\x00zxcvbnm,.-\x00*\x00 ";
const TECLAS_MAYUS: &[u8; 58] = b"\x00\x1b!\"\x00$%&/()=?\x00\x08\tQWERTYUIOP^*\n\x00ASDFGHJKL\x00\x00\x00\x00\x00ZXCVBNM;:_\x00*\x00 ";

fn tecla_de_codigo(codigo: u8) -> Option<String> {
    let extendida = core::mem::replace(EXTENDIDA.tomar(), false);
    match codigo {
        0xe0 => {
            *EXTENDIDA.tomar() = true;
            return None;
        }
        0x2a | 0x36 => *MAYUSCULAS.tomar() = true,
        0xaa | 0xb6 => *MAYUSCULAS.tomar() = false,
        0x3a => *BLOQ_MAYUS.tomar() = !*BLOQ_MAYUS.tomar(),
        _ => {}
    }
    if codigo & 0x80 != 0 {
        return None;
    }
    if extendida {
        return match codigo {
            0x48 => Some("arriba".to_string()),
            0x50 => Some("abajo".to_string()),
            0x4b => Some("izquierda".to_string()),
            0x4d => Some("derecha".to_string()),
            0x1c => Some("enter".to_string()),
            0x53 => Some("suprimir".to_string()),
            0x35 => Some("/".to_string()),
            _ => None,
        };
    }
    let mayus = *MAYUSCULAS.tomar();
    let texto = match codigo {
        0x01 => "escape".to_string(),
        0x0e => "borrar".to_string(),
        0x0f => "tab".to_string(),
        0x1c => "enter".to_string(),
        0x27 => if mayus ^ *BLOQ_MAYUS.tomar() { "Ñ" } else { "ñ" }.to_string(),
        0x28 => if mayus { "¨" } else { "´" }.to_string(),
        0x2b => if mayus { "Ç" } else { "ç" }.to_string(),
        0x29 => if mayus { "ª" } else { "º" }.to_string(),
        0x0d => if mayus { "¿" } else { "¡" }.to_string(),
        0x56 => if mayus { ">" } else { "<" }.to_string(),
        0x37 => "*".to_string(),
        0x4a => "-".to_string(),
        0x4e => "+".to_string(),
        c if (c as usize) < TECLAS.len() => {
            let letra = TECLAS[c as usize];
            let es_letra = letra.is_ascii_lowercase();
            let alta = if es_letra { mayus ^ *BLOQ_MAYUS.tomar() } else { mayus };
            let byte = if alta { TECLAS_MAYUS[c as usize] } else { letra };
            if byte == 0 {
                return None;
            }
            (byte as char).to_string()
        }
        _ => return None,
    };
    Some(texto)
}

fn tecla_de_serie(byte: u8) -> Option<String> {
    let estado = SERIE_ESCAPE.tomar();
    match (*estado, byte) {
        (0, 0x1b) => {
            *estado = 1;
            None
        }
        (1, b'[') => {
            *estado = 2;
            None
        }
        (2, letra) => {
            *estado = 0;
            match letra {
                b'A' => Some("arriba".to_string()),
                b'B' => Some("abajo".to_string()),
                b'C' => Some("derecha".to_string()),
                b'D' => Some("izquierda".to_string()),
                _ => None,
            }
        }
        (_, b'\r') | (_, b'\n') => {
            *estado = 0;
            Some("enter".to_string())
        }
        (_, 0x7f) | (_, 0x08) => {
            *estado = 0;
            Some("borrar".to_string())
        }
        (_, b'\t') => Some("tab".to_string()),
        (_, byte) if byte >= 0x20 && byte < 0x7f => {
            *estado = 0;
            Some((byte as char).to_string())
        }
        _ => {
            *estado = 0;
            None
        }
    }
}

static SERIE_UTF8: Global<Vec<u8>> = Global::new(Vec::new());

/// Espera hasta que se pulse una tecla (en el teclado o llegue por el puerto serie).
fn esperar_tecla() -> String {
    loop {
        if leer_puerto(0x64) & 1 != 0 {
            let codigo = leer_puerto(0x60);
            if let Some(tecla) = tecla_de_codigo(codigo) {
                return tecla;
            }
        }
        if leer_puerto(COM1 + 5) & 1 != 0 {
            let byte = leer_puerto(COM1);
            if byte >= 0x80 {
                let pendiente = SERIE_UTF8.tomar();
                pendiente.push(byte);
                if let Ok(texto) = core::str::from_utf8(pendiente) {
                    let texto = texto.to_string();
                    pendiente.clear();
                    return texto;
                }
                if pendiente.len() >= 4 {
                    pendiente.clear();
                }
                continue;
            }
            if let Some(tecla) = tecla_de_serie(byte) {
                return tecla;
            }
        }
        core::hint::spin_loop();
    }
}

// --- Matemáticas de f64 que en std vienen del sistema ------------------------------------

#[allow(dead_code)]
trait MatesF64 {
    fn trunc(self) -> f64;
    fn floor(self) -> f64;
    fn ceil(self) -> f64;
    fn round(self) -> f64;
    fn fract(self) -> f64;
    fn sqrt(self) -> f64;
    fn powi(self, n: i32) -> f64;
}

impl MatesF64 for f64 {
    fn trunc(self) -> f64 {
        if !self.is_finite() || self.abs() >= 4503599627370496.0 {
            self
        } else {
            (self as i64) as f64
        }
    }
    fn floor(self) -> f64 {
        let t = MatesF64::trunc(self);
        if t > self { t - 1.0 } else { t }
    }
    fn ceil(self) -> f64 {
        let t = MatesF64::trunc(self);
        if t < self { t + 1.0 } else { t }
    }
    fn round(self) -> f64 {
        if self < 0.0 { -MatesF64::floor(-self + 0.5) } else { MatesF64::floor(self + 0.5) }
    }
    fn fract(self) -> f64 {
        self - MatesF64::trunc(self)
    }
    fn sqrt(self) -> f64 {
        if self < 0.0 {
            return f64::NAN;
        }
        if self == 0.0 || !self.is_finite() {
            return self;
        }
        let mut x = if self > 1.0 { self / 2.0 } else { 1.0 };
        for _ in 0..80 {
            x = 0.5 * (x + self / x);
        }
        x
    }
    fn powi(self, n: i32) -> f64 {
        let mut resultado = 1.0;
        for _ in 0..n.unsigned_abs() {
            resultado *= self;
        }
        if n < 0 { 1.0 / resultado } else { resultado }
    }
}

// --- Saltos para `intentar` (lo que en std hace catch_unwind) ------------------------------

pub struct AssertUnwindSafe<T>(pub T);

struct Punto {
    contexto: [u64; 8],
}

static PUNTOS: Global<Vec<*mut Punto>> = Global::new(Vec::new());
static CARGA: Global<Option<Box<dyn core::any::Any + Send>>> = Global::new(None);

struct EstadoLlamada<F, R> {
    funcion: Option<F>,
    resultado: Option<R>,
}

extern "C" fn llamar_protegida<F: FnOnce() -> R, R>(datos: *mut u8) {
    let estado = unsafe { &mut *(datos as *mut EstadoLlamada<F, R>) };
    if let Some(funcion) = estado.funcion.take() {
        estado.resultado = Some(funcion());
    }
}

/// Ejecuta `funcion`; si dentro se lanza un error (panic_any), vuelve aquí con él.
/// Las partes abandonadas no liberan su memoria; los contadores de llamadas se restauran.
pub fn catch_unwind<F: FnOnce() -> R, R>(funcion: AssertUnwindSafe<F>) -> Result<R, Box<dyn core::any::Any + Send>> {
    let mut estado = EstadoLlamada { funcion: Some(funcion.0), resultado: None };
    let mut punto = Box::new(Punto { contexto: [0; 8] });
    let puntero: *mut Punto = &mut *punto;
    let profundidad = *PROFUNDIDAD.tomar();
    let intentos = *INTENTAR.tomar();
    PUNTOS.tomar().push(puntero);
    let saltado = unsafe {
        deimon_proteger(
            llamar_protegida::<F, R>,
            &mut estado as *mut EstadoLlamada<F, R> as *mut u8,
            &mut (*puntero).contexto,
        )
    };
    let puntos = PUNTOS.tomar();
    while let Some(ultimo) = puntos.pop() {
        if ultimo == puntero {
            break;
        }
    }
    if saltado == 0 {
        Ok(estado.resultado.take().expect("resultado"))
    } else {
        *PROFUNDIDAD.tomar() = profundidad;
        *INTENTAR.tomar() = intentos;
        Err(CARGA.tomar().take().unwrap_or_else(|| Box::new(())))
    }
}

pub fn resume_unwind(carga: Box<dyn core::any::Any + Send>) -> ! {
    let punto = PUNTOS.tomar().last().copied();
    match punto {
        Some(punto) => {
            *CARGA.tomar() = Some(carga);
            unsafe { deimon_saltar(&(*punto).contexto) }
        }
        None => {
            salida_texto("\nError interno: un salto sin intentar que lo recoja\n");
            detener()
        }
    }
}

pub fn panic_any<T: core::any::Any + Send>(carga: T) -> ! {
    resume_unwind(Box::new(carga))
}

// --- Estado del programa (lo que en base.rs son thread_local) ----------------------------

static LINEA: Global<usize> = Global::new(0);
static INTENTAR: Global<usize> = Global::new(0);
static PROFUNDIDAD: Global<usize> = Global::new(0);
static VALOR_DEVOLVER: Global<Option<Valor>> = Global::new(None);

const LIMITE_LLAMADAS: usize = 10_000;

#[cold]
fn demasiadas_llamadas() -> ! {
    fallo(format!(
        "hay más de {} llamadas a funciones una dentro de otra; revisa que la función tenga un caso que termine sin volver a llamarse",
        LIMITE_LLAMADAS
    ))
}

struct Llamada;

impl Drop for Llamada {
    fn drop(&mut self) {
        *PROFUNDIDAD.tomar() -= 1;
    }
}

#[inline]
fn entrar_llamada() -> Llamada {
    let profundidad = PROFUNDIDAD.tomar();
    *profundidad += 1;
    let llamada = Llamada;
    if *profundidad > LIMITE_LLAMADAS {
        demasiadas_llamadas();
    }
    llamada
}

#[inline]
fn profundidad_actual() -> usize {
    *PROFUNDIDAD.tomar()
}

#[inline(always)]
fn llamada_numerica(profundidad: usize, linea: usize) -> usize {
    if profundidad >= LIMITE_LLAMADAS {
        en_linea(linea);
        demasiadas_llamadas();
    }
    profundidad + 1
}

#[inline]
fn en_linea(linea: usize) {
    *LINEA.tomar() = linea;
}

#[inline]
fn con_linea(valor: Valor, linea: usize) -> Valor {
    en_linea(linea);
    valor
}

struct ErrorIntento {
    mensaje: String,
    linea: usize,
}
struct ControlRomper;
struct ControlContinuar;
struct ControlDevolver;

fn intentar_entrar() {
    *INTENTAR.tomar() += 1;
}

fn intentar_salir() {
    let intentos = INTENTAR.tomar();
    *intentos = intentos.saturating_sub(1);
}

fn error_intento(valor: &ErrorIntento) -> Valor {
    Valor::mapa(vec![
        ("mensaje".to_string(), Valor::Texto(Rc::new(valor.mensaje.clone()))),
        ("linea".to_string(), Valor::Numero(valor.linea as f64)),
    ])
}

fn devolver_de_intentar(valor: Valor) -> ! {
    *VALOR_DEVOLVER.tomar() = Some(valor);
    panic_any(ControlDevolver);
}

fn tomar_valor_devuelto() -> Valor {
    VALOR_DEVOLVER.tomar().take().unwrap_or(Valor::Nada)
}

/// Un error sin `intentar` alrededor: se escribe en rojo y la máquina se detiene.
fn fallo(mensaje: String) -> ! {
    let linea = *LINEA.tomar();
    if *INTENTAR.tomar() > 0 {
        panic_any(ErrorIntento { mensaje, linea });
    }
    PANTALLA.tomar().color = 0x4f;
    if linea > 0 {
        salida_texto(&format!("\nError en la línea {} (en ejecución): {}\n", linea, mensaje));
    } else {
        salida_texto(&format!("\nError en tiempo de ejecución: {}\n", mensaje));
    }
    salida_texto("La máquina se ha detenido.\n");
    apagar_si_es_qemu(1);
    detener()
}

#[inline]
fn provocar_error(mensaje: &Valor) -> Valor {
    fallo(texto_de(mensaje))
}

// Las pruebas se corren con `deimon probar` (en Linux); aquí solo se comprueba al ejecutar.
#[inline]
fn comprobar(condicion: &Valor, linea: usize) -> Valor {
    match condicion {
        Valor::Logico(true) => Valor::Nada,
        Valor::Logico(false) => {
            en_linea(linea);
            fallo(format!("línea {}: la condición es falsa", linea))
        }
        otro => fallo(format!("comprobar() espera verdadero o falso, no {}", otro.tipo())),
    }
}

#[inline]
fn comprobar_igual(obtenido: &Valor, esperado: &Valor, linea: usize) -> Valor {
    if obtenido == esperado {
        return Valor::Nada;
    }
    en_linea(linea);
    fallo(format!("línea {}: se esperaba {} y se obtuvo {}", linea, texto_de(esperado), texto_de(obtenido)))
}

#[inline]
fn mostrar(valor: &Valor) -> Valor {
    salida_texto(&texto_de(valor));
    salida_texto("\n");
    Valor::Nada
}

fn escribir(valor: &Valor) -> Valor {
    salida_texto(&texto_de(valor));
    Valor::Nada
}

fn listar_claves(coleccion: &Valor) -> Valor {
    match coleccion {
        Valor::Mapa(mapa) => claves_de_mapa(mapa),
        otro => fallo(format!(
            "listar_claves() en un programa sin sistema necesita una lista con claves, no {}",
            otro.tipo()
        )),
    }
}

fn proteger_html(texto: &Valor) -> Valor {
    let mut salida = String::new();
    for c in texto_de(texto).chars() {
        match c {
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '&' => salida.push_str("&amp;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#39;"),
            c => salida.push(c),
        }
    }
    Valor::Texto(Rc::new(salida))
}

// --- Funciones de la máquina (en Linux hacen lo mismo en la terminal) ----------------------

fn numero_de(valor: &Valor, funcion: &str) -> f64 {
    match valor {
        Valor::Numero(n) => *n,
        otro => fallo(format!("{}() espera un número, no {}", funcion, otro.tipo())),
    }
}

fn codigo_color(valor: &Valor) -> u8 {
    match texto_de(valor).as_str() {
        "negro" => 0, "azul" => 1, "verde" => 2, "cian" => 3, "rojo" => 4,
        "morado" => 5, "marron" => 6, "gris" => 7, "gris_oscuro" => 8,
        "azul_claro" => 9, "verde_claro" => 10, "cian_claro" => 11,
        "rojo_claro" => 12, "rosa" => 13, "amarillo" => 14, "blanco" => 15,
        otro => fallo(format!(
            "color desconocido '{}'; usa negro, azul, verde, cian, rojo, morado, marron, gris, gris_oscuro, azul_claro, verde_claro, cian_claro, rojo_claro, rosa, amarillo o blanco",
            otro
        )),
    }
}

fn limpiar_pantalla() -> Valor {
    limpiar_toda_la_pantalla();
    for b in b"\x1b[2J\x1b[H" {
        serie_escribir(*b);
    }
    Valor::Nada
}

fn escribir_en(fila: &Valor, columna: &Valor, texto: &Valor) -> Valor {
    let f = numero_de(fila, "escribir_en");
    let c = numero_de(columna, "escribir_en");
    if f < 0.0 || f >= ALTO as f64 || c < 0.0 || c >= ANCHO as f64 {
        fallo(format!("escribir_en(): la fila va de 0 a {} y la columna de 0 a {}", ALTO - 1, ANCHO - 1));
    }
    let p = PANTALLA.tomar();
    p.fila = f as usize;
    p.columna = c as usize;
    for b in format!("\x1b[{};{}H", p.fila + 1, p.columna + 1).bytes() {
        serie_escribir(b);
    }
    salida_texto(&texto_de(texto));
    Valor::Nada
}

fn poner_color(letra: &Valor, fondo: &Valor) -> Valor {
    let l = codigo_color(letra);
    let f = codigo_color(fondo);
    PANTALLA.tomar().color = (f << 4) | l;
    Valor::Nada
}

fn leer_tecla() -> Valor {
    Valor::Texto(Rc::new(esperar_tecla()))
}

fn pedir_texto(aviso: &Valor) -> Valor {
    salida_texto(&texto_de(aviso));
    let mut linea = String::new();
    loop {
        let tecla = esperar_tecla();
        match tecla.as_str() {
            "enter" => {
                salida_texto("\n");
                return Valor::Texto(Rc::new(linea));
            }
            "borrar" => {
                if linea.pop().is_some() {
                    pantalla_caracter('\u{8}');
                    for b in b"\x08 \x08" {
                        serie_escribir(*b);
                    }
                }
            }
            t if t.chars().count() == 1 => {
                linea.push_str(t);
                salida_texto(t);
            }
            _ => {}
        }
    }
}

fn apagar_si_es_qemu(codigo: u8) {
    // QEMU con -device isa-debug-exit,iobase=0xf4 termina aquí (salida = codigo*2+1).
    escribir_puerto(0xf4, codigo);
    // QEMU y Bochs modernos (ACPI) y VirtualBox.
    escribir_puerto16(0x604, 0x2000);
    escribir_puerto16(0xb004, 0x2000);
    escribir_puerto16(0x4004, 0x3400);
}

fn detener() -> ! {
    loop {
        unsafe { core::arch::asm!("cli", "hlt", options(nomem, nostack)) }
    }
}

fn apagar_maquina() -> Valor {
    salida_texto("\nApagando...\n");
    apagar_si_es_qemu(0);
    salida_texto("Ya puedes apagar el ordenador.\n");
    detener()
}

fn reiniciar_maquina() -> Valor {
    let mut espera = 0;
    while leer_puerto(0x64) & 2 != 0 && espera < 100_000 {
        espera += 1;
    }
    escribir_puerto(0x64, 0xfe);
    detener()
}

#[panic_handler]
fn panico(informacion: &core::panic::PanicInfo) -> ! {
    PANTALLA.tomar().color = 0x4f;
    salida_texto(&format!("\nError interno de Deimon: {}\n", informacion.message()));
    apagar_si_es_qemu(1);
    detener()
}

fn arrancar_nucleo(programa: fn()) -> ! {
    preparar_serie();
    PANTALLA.tomar().color = 0x07;
    limpiar_toda_la_pantalla();
    programa();
    apagar_si_es_qemu(0);
    detener()
}
