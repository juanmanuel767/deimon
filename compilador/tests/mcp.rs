//! `deimon mcp`: el servidor con el que una IA usa Deimon directamente.

use std::io::Write;
use std::process::{Command, Stdio};

fn conversar(mensajes: &[&str]) -> Vec<String> {
    let mut hijo = Command::new(env!("CARGO_BIN_EXE_deimon"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("no se pudo arrancar deimon mcp");
    {
        let entrada = hijo.stdin.as_mut().unwrap();
        for mensaje in mensajes {
            writeln!(entrada, "{}", mensaje).unwrap();
        }
    }
    let salida = hijo.wait_with_output().unwrap();
    String::from_utf8_lossy(&salida.stdout).lines().map(str::to_string).collect()
}

fn llamar(id: u32, herramienta: &str, argumentos: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"tools/call\",\"params\":{{\"name\":\"{}\",\"arguments\":{}}}}}",
        id, herramienta, argumentos
    )
}

#[test]
fn una_ia_puede_comprobar_ejecutar_probar_y_buscar() {
    let respuestas = conversar(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"prueba","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        &llamar(3, "deimon_ejecutar", r#"{"codigo":"sea n = 6\nmostrar(n * 7)\n"}"#),
        &llamar(4, "deimon_comprobar", r#"{"codigo":"mostrar(\"a\" - 1)\n"}"#),
        &llamar(5, "deimon_buscar_funcion", r#"{"texto":"mayusculas"}"#),
        &llamar(6, "deimon_buscar_funcion", r#"{"texto":"funcion_que_no_existe_xyz"}"#),
        &llamar(7, "deimon_probar", r#"{"codigo":"prueba \"suma\" {\n    comprobar_igual(1 + 1, 2)\n}\n"}"#),
        &llamar(8, "deimon_formatear", r#"{"codigo":"sea   x=1\n"}"#),
        "esto no es json",
    ]);
    // La notificación no lleva respuesta: 9 respuestas para 10 mensajes.
    assert_eq!(respuestas.len(), 9, "{:#?}", respuestas);
    assert!(respuestas[0].contains("\"protocolVersion\":\"2025-06-18\"") && respuestas[0].contains("\"tools\":{}"));
    for herramienta in ["deimon_comprobar", "deimon_ejecutar", "deimon_probar", "deimon_formatear", "deimon_guia", "deimon_buscar_funcion"] {
        assert!(respuestas[1].contains(herramienta), "falta {}", herramienta);
    }
    assert!(respuestas[2].contains(r#"\"salida\":\"42\\n\""#) && respuestas[2].contains("\"isError\":false"), "{}", respuestas[2]);
    assert!(respuestas[3].contains("E025") && respuestas[3].contains("no se puede restar texto y número"), "{}", respuestas[3]);
    assert!(respuestas[4].contains("mayusculas(texto)") && respuestas[4].contains(r#"usar modulo \\\"texto\\\""#), "{}", respuestas[4]);
    assert!(respuestas[5].contains("no la inventes") && respuestas[5].contains("\"isError\":true"), "{}", respuestas[5]);
    assert!(respuestas[6].contains("\"isError\":false") && respuestas[6].contains("suma"), "{}", respuestas[6]);
    assert!(respuestas[7].contains(r#"sea x = 1\n"#), "{}", respuestas[7]);
    assert!(respuestas[8].contains("\"code\":-32700"), "{}", respuestas[8]);
}

#[test]
fn un_programa_sin_fin_no_cuelga_el_servidor() {
    let respuestas = conversar(&[
        &llamar(1, "deimon_ejecutar", r#"{"codigo":"mientras verdadero {\n}\n"}"#),
        r#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#,
    ]);
    assert!(respuestas[0].contains("pasó de 2000000000 pasos") && respuestas[0].contains("\"isError\":true"), "{}", respuestas[0]);
    assert_eq!(respuestas[1], r#"{"jsonrpc":"2.0","id":2,"result":{}}"#);
}
