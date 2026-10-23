//! Tabla única de funciones integradas. La usan el generador (para validar llamadas),
//! `deimon guia` (para documentarlas) y las pruebas (para que nunca se desincronicen).

pub struct Integrada {
    pub nombre: &'static str,
    /// Nombres de los parámetros; su cantidad es el número de argumentos.
    pub parametros: &'static [&'static str],
    pub categoria: &'static str,
    pub descripcion: &'static str,
    pub ejemplo: &'static str,
}

impl Integrada {
    /// Firma legible, por ejemplo `guardar(coleccion, clave, valor)`.
    pub fn firma(&self) -> String {
        format!("{}({})", self.nombre, self.parametros.join(", "))
    }
}

pub const INTEGRADAS: &[Integrada] = &[
    Integrada {
        nombre: "mostrar",
        parametros: &["valor"],
        categoria: "básicas",
        descripcion: "escribe el valor y un salto de línea",
        ejemplo: "mostrar(\"Hola \" + 5)   # escribe Hola 5",
    },
    Integrada {
        nombre: "escribir",
        parametros: &["valor"],
        categoria: "básicas",
        descripcion: "escribe el valor sin salto de línea al final (como echo en PHP)",
        ejemplo: "escribir(\"Hola \")   # y lo siguiente sigue en la misma línea",
    },
    Integrada {
        nombre: "elegir",
        parametros: &["condicion", "si_verdad", "si_falso"],
        categoria: "básicas",
        descripcion: "devuelve si_verdad cuando la condición se cumple y si_falso cuando no; solo calcula la opción elegida",
        ejemplo: "mostrar(elegir(edad >= 18, \"adulto\", \"menor\"))",
    },
    Integrada {
        nombre: "a_numero",
        parametros: &["texto"],
        categoria: "conversiones",
        descripcion: "convierte un texto en número; error si no es un número válido",
        ejemplo: "mostrar(a_numero(\"21\") * 2)   # escribe 42",
    },
    Integrada {
        nombre: "a_texto",
        parametros: &["valor"],
        categoria: "conversiones",
        descripcion: "convierte cualquier valor a su texto, igual que al mostrarlo",
        ejemplo: "mostrar(a_texto(2.5) + \"!\")   # escribe 2.5!",
    },
    Integrada {
        nombre: "medir",
        parametros: &["valor"],
        categoria: "listas y textos",
        descripcion: "cantidad de elementos de una lista o de caracteres de un texto",
        ejemplo: "mostrar(medir([1, 2, 3]))   # escribe 3",
    },
    Integrada {
        nombre: "agregar",
        parametros: &["lista", "valor"],
        categoria: "listas y textos",
        descripcion: "devuelve una lista nueva con el valor añadido al final (la original no cambia)",
        ejemplo: "sea mas = agregar([1, 2], 3)   # [1, 2, 3]",
    },
    Integrada {
        nombre: "incluye",
        parametros: &["lista", "valor"],
        categoria: "listas y textos",
        descripcion: "dice si la lista tiene ese valor (en una lista con claves, entre sus valores) o si un texto contiene otro",
        ejemplo: "si incluye([\"rojo\", \"azul\"], color) { mostrar(\"me gusta\") }",
    },
    Integrada {
        nombre: "ordenar",
        parametros: &["lista"],
        categoria: "listas y textos",
        descripcion: "devuelve la lista ordenada de menor a mayor (solo números o solo textos)",
        ejemplo: "mostrar(ordenar([3, 1, 2]))   # [1, 2, 3]",
    },
    Integrada {
        nombre: "invertir",
        parametros: &["lista"],
        categoria: "listas y textos",
        descripcion: "devuelve la lista (o el texto) al revés",
        ejemplo: "mostrar(invertir([1, 2, 3]))   # [3, 2, 1]",
    },
    Integrada {
        nombre: "cortar",
        parametros: &["lista", "inicio", "cantidad"],
        categoria: "listas y textos",
        descripcion: "trozo de una lista o texto desde inicio (negativo cuenta desde el final); cantidad nada llega hasta el final",
        ejemplo: "mostrar(cortar(\"Deimon\", 0, 3))   # Dei",
    },
    Integrada {
        nombre: "rango",
        parametros: &["inicio", "fin"],
        categoria: "listas y textos",
        descripcion: "lista de números enteros de inicio a fin, los dos incluidos",
        ejemplo: "para i en rango(1, 5) { mostrar(i) }   # 1 2 3 4 5",
    },
    Integrada {
        nombre: "sumar_lista",
        parametros: &["lista"],
        categoria: "listas y textos",
        descripcion: "suma los números de una lista",
        ejemplo: "mostrar(sumar_lista([1, 2, 3]))   # 6",
    },
    Integrada {
        nombre: "tiene_clave",
        parametros: &["mapa", "clave"],
        categoria: "listas y textos",
        descripcion: "dice si una lista con claves tiene esa clave",
        ejemplo: "si tiene_clave(persona, \"edad\") { mostrar(persona[\"edad\"]) }",
    },
    Integrada {
        nombre: "quitar_clave",
        parametros: &["mapa", "clave"],
        categoria: "listas y textos",
        descripcion: "devuelve la lista con claves sin esa clave (la original no cambia)",
        ejemplo: "persona = quitar_clave(persona, \"edad\")",
    },
    Integrada {
        nombre: "guardar",
        parametros: &["coleccion", "clave", "valor"],
        categoria: "datos",
        descripcion: "guarda un número, texto, lógico, nada o una lista bajo una clave de una colección",
        ejemplo: "guardar(\"amigos\", \"ana\", 30)",
    },
    Integrada {
        nombre: "leer",
        parametros: &["coleccion", "clave"],
        categoria: "datos",
        descripcion: "devuelve el valor guardado, o nada si la clave no existe",
        ejemplo: "mostrar(leer(\"amigos\", \"ana\"))   # escribe 30",
    },
    Integrada {
        nombre: "borrar",
        parametros: &["coleccion", "clave"],
        categoria: "datos",
        descripcion: "borra una clave de la colección (no pasa nada si no existe)",
        ejemplo: "borrar(\"amigos\", \"ana\")",
    },
    Integrada {
        nombre: "contar",
        parametros: &["coleccion"],
        categoria: "datos",
        descripcion: "cuántas claves tiene la colección",
        ejemplo: "mostrar(contar(\"amigos\"))",
    },
    Integrada {
        nombre: "listar_claves",
        parametros: &["coleccion"],
        categoria: "datos",
        descripcion: "lista ordenada de las claves de la colección; si le das una lista con claves, devuelve sus claves en orden",
        ejemplo: "para clave en listar_claves(\"amigos\") { mostrar(clave) }",
    },
    Integrada {
        nombre: "incrementar",
        parametros: &["coleccion", "clave", "cantidad"],
        categoria: "datos",
        descripcion: "suma una cantidad a un número guardado sin que otro cambio se cuele a la vez, y devuelve el resultado (si no existía, empieza en 0)",
        ejemplo: "sea visitas = incrementar(\"contadores\", \"inicio\", 1)",
    },
    Integrada {
        nombre: "campo",
        parametros: &["nombre"],
        categoria: "web",
        descripcion: "valor de un campo del formulario enviado (o de ?nombre= en la dirección), ya protegido como HTML; nada si no viene",
        ejemplo: "sea nombre = campo(\"nombre\")",
    },
    Integrada {
        nombre: "es_envio",
        parametros: &[],
        categoria: "web",
        descripcion: "verdadero si la ruta se atiende porque se envió un formulario (método POST)",
        ejemplo: "si es_envio() { guardar(\"mensajes\", campo(\"titulo\"), campo(\"texto\")) }",
    },
    Integrada {
        nombre: "proteger_html",
        parametros: &["texto"],
        categoria: "web",
        descripcion: "devuelve el texto con < > & \" ' convertidos para mostrarlo en una página sin riesgo (campo() ya lo hace solo)",
        ejemplo: "escribir(\"<p>\" + proteger_html(leer(\"notas\", \"1\")) + \"</p>\")",
    },
    Integrada {
        nombre: "redirigir",
        parametros: &["camino"],
        categoria: "web",
        descripcion: "manda al navegador a otra página de este sitio (el camino empieza por '/')",
        ejemplo: "devolver redirigir(\"/gracias\")",
    },
    Integrada {
        nombre: "crear_usuario",
        parametros: &["nombre", "clave"],
        categoria: "usuarios",
        descripcion: "crea un usuario con la contraseña cifrada (PBKDF2-SHA256); falso si ya existe o si la contraseña tiene menos de 8 caracteres",
        ejemplo: "si crear_usuario(campo(\"nombre\"), campo(\"clave\")) { ... }",
    },
    Integrada {
        nombre: "comprobar_clave",
        parametros: &["nombre", "clave"],
        categoria: "usuarios",
        descripcion: "verdadero si el usuario existe y la contraseña es la suya",
        ejemplo: "si comprobar_clave(campo(\"nombre\"), campo(\"clave\")) { iniciar_sesion(campo(\"nombre\")) }",
    },
    Integrada {
        nombre: "iniciar_sesion",
        parametros: &["nombre"],
        categoria: "usuarios",
        descripcion: "abre la sesión de un usuario existente en este navegador (galleta segura de 7 días)",
        ejemplo: "iniciar_sesion(\"ana\")",
    },
    Integrada {
        nombre: "usuario_actual",
        parametros: &[],
        categoria: "usuarios",
        descripcion: "nombre del usuario con la sesión abierta, o nada si nadie ha entrado",
        ejemplo: "si usuario_actual() == nada { devolver redirigir(\"/entrar\") }",
    },
    Integrada {
        nombre: "cerrar_sesion",
        parametros: &[],
        categoria: "usuarios",
        descripcion: "cierra la sesión de este navegador",
        ejemplo: "cerrar_sesion()",
    },
    Integrada {
        nombre: "cuenta_bloqueada",
        parametros: &["nombre"],
        categoria: "usuarios",
        descripcion: "verdadero si la cuenta está bloqueada 15 minutos por 5 contraseñas equivocadas seguidas (mientras tanto comprobar_clave da falso)",
        ejemplo: "si cuenta_bloqueada(campo(\"nombre\")) { devolver \"Espera 15 minutos\" }",
    },
    Integrada {
        nombre: "cambiar_clave",
        parametros: &["nombre", "actual", "nueva"],
        categoria: "usuarios",
        descripcion: "cambia la contraseña sabiendo la actual y cierra las demás sesiones; falso si la actual no es correcta o la nueva es corta",
        ejemplo: "cambiar_clave(usuario_actual(), campo(\"actual\"), campo(\"nueva\"))",
    },
    Integrada {
        nombre: "crear_codigo_recuperacion",
        parametros: &["nombre"],
        categoria: "usuarios",
        descripcion: "crea un código de un solo uso, válido 1 hora, para recuperar la contraseña (tu programa decide cómo hacérselo llegar); nada si el usuario no existe",
        ejemplo: "sea codigo = crear_codigo_recuperacion(\"ana\")",
    },
    Integrada {
        nombre: "restablecer_clave",
        parametros: &["nombre", "codigo", "nueva"],
        categoria: "usuarios",
        descripcion: "pone una contraseña nueva con un código de recuperación válido (el código deja de valer y se cierran las sesiones)",
        ejemplo: "restablecer_clave(campo(\"nombre\"), campo(\"codigo\"), campo(\"nueva\"))",
    },
    Integrada {
        nombre: "guardar_en_sesion",
        parametros: &["clave", "valor"],
        categoria: "sesión del visitante",
        descripcion: "guarda un valor (también listas) en la sesión de este navegador, aunque nadie haya entrado como usuario (como $_SESSION de PHP); dura 24 horas",
        ejemplo: "guardar_en_sesion(\"idioma\", \"es\")",
    },
    Integrada {
        nombre: "leer_de_sesion",
        parametros: &["clave"],
        categoria: "sesión del visitante",
        descripcion: "lee un valor de la sesión de este navegador, o nada",
        ejemplo: "sea idioma = leer_de_sesion(\"idioma\")",
    },
    Integrada {
        nombre: "agregar_a_sesion",
        parametros: &["clave", "valor"],
        categoria: "sesión del visitante",
        descripcion: "añade un valor a una lista de la sesión y la crea si no existe (un carrito, por ejemplo)",
        ejemplo: "agregar_a_sesion(\"carrito\", campo(\"producto\"))",
    },
    Integrada {
        nombre: "borrar_de_sesion",
        parametros: &["clave"],
        categoria: "sesión del visitante",
        descripcion: "borra un valor de la sesión de este navegador",
        ejemplo: "borrar_de_sesion(\"carrito\")",
    },
    Integrada {
        nombre: "vaciar_sesion",
        parametros: &[],
        categoria: "sesión del visitante",
        descripcion: "borra todo lo guardado en la sesión de este navegador",
        ejemplo: "vaciar_sesion()",
    },
    Integrada {
        nombre: "servir",
        parametros: &["puerto"],
        categoria: "web",
        descripcion: "arranca el servidor web con las rutas definidas (DEIMON_PUERTO cambia el puerto)",
        ejemplo: "servir(8080)   # al final del programa, después de las rutas",
    },
    Integrada {
        nombre: "visitar",
        parametros: &["camino"],
        categoria: "pruebas",
        descripcion: "visita una ruta sin red y devuelve el texto de la página (solo útil en pruebas)",
        ejemplo: "comprobar_igual(visitar(\"/saludo/Ana\"), \"<h1>Hola, Ana</h1>\")",
    },
    Integrada {
        nombre: "enviar",
        parametros: &["camino", "datos"],
        categoria: "pruebas",
        descripcion: "como visitar(), pero enviando un formulario; datos es un texto como \"nombre=Ana&clave=secreta1\" (guarda las galletas de sesión como un navegador)",
        ejemplo: "comprobar_igual(enviar(\"/entrar\", \"nombre=ana&clave=secreta12\"), \"redirige a /\")",
    },
    Integrada {
        nombre: "comprobar",
        parametros: &["condicion"],
        categoria: "pruebas",
        descripcion: "falla si la condición no es verdadera",
        ejemplo: "comprobar(suma(2, 3) == 5)",
    },
    Integrada {
        nombre: "comprobar_igual",
        parametros: &["obtenido", "esperado"],
        categoria: "pruebas",
        descripcion: "falla si los dos valores son distintos y dice cuáles eran",
        ejemplo: "comprobar_igual(suma(2, 2), 4)",
    },
];

pub fn buscar(nombre: &str) -> Option<&'static Integrada> {
    INTEGRADAS.iter().find(|integrada| integrada.nombre == nombre)
}

pub fn aridad(nombre: &str) -> Option<usize> {
    buscar(nombre).map(|integrada| integrada.parametros.len())
}
