'use strict';

const { execFile } = require('child_process');

/**
 * Convierte la salida de `deimon comprobar --json` en una lista de errores
 * con posiciones empezando en 0 (como las usa VS Code).
 * Devuelve { errores: [...] } o { fallo: "texto" } si la salida no se entiende.
 */
function analizarSalida(texto) {
  let datos;
  try {
    datos = JSON.parse(String(texto).trim());
  } catch (e) {
    return { fallo: 'La salida de deimon no es JSON válido.' };
  }
  if (!datos || !Array.isArray(datos.errores)) {
    return { fallo: 'La salida de deimon no tiene la lista de errores.' };
  }
  const errores = datos.errores.map((e) => {
    const linea = Math.max((Number(e.linea) || 1) - 1, 0);
    const columna = Math.max((Number(e.columna) || 1) - 1, 0);
    let mensaje = String(e.mensaje || 'error');
    if (e.sugerencia) {
      mensaje += '\nSugerencia: ' + e.sugerencia;
    }
    return { codigo: e.codigo || '', linea, columna, mensaje };
  });
  return { errores };
}

function activate(context) {
  const vscode = require('vscode');
  const diagnosticos = vscode.languages.createDiagnosticCollection('deimon');
  context.subscriptions.push(diagnosticos);
  let avisado = false;

  function comprobar(documento) {
    if (documento.languageId !== 'deimon' || documento.uri.scheme !== 'file') {
      return;
    }
    const ruta = vscode.workspace.getConfiguration('deimon').get('ruta') || 'deimon';
    execFile(ruta, ['comprobar', '--json', documento.uri.fsPath], { maxBuffer: 10 * 1024 * 1024 }, (error, salida) => {
      if (error && error.code === 'ENOENT') {
        if (!avisado) {
          avisado = true;
          vscode.window.showWarningMessage(
            'No se encontró el programa deimon. Instálalo o indica su ruta en el ajuste "deimon.ruta".'
          );
        }
        return;
      }
      // deimon sale con código 1 cuando hay errores, pero igualmente escribe el JSON.
      const resultado = analizarSalida(salida);
      if (resultado.fallo) {
        return;
      }
      const lista = resultado.errores.map((e) => {
        const lineaTexto = e.linea < documento.lineCount ? documento.lineAt(e.linea) : null;
        const inicio = new vscode.Position(e.linea, e.columna);
        let fin = lineaTexto ? lineaTexto.range.end : inicio;
        if (lineaTexto && e.columna < lineaTexto.text.length) {
          const resto = lineaTexto.text.slice(e.columna);
          const palabra = /^[\p{L}\p{N}_]+/u.exec(resto);
          fin = new vscode.Position(e.linea, e.columna + (palabra ? palabra[0].length : 1));
        }
        const d = new vscode.Diagnostic(new vscode.Range(inicio, fin), e.mensaje, vscode.DiagnosticSeverity.Error);
        d.source = 'deimon';
        if (e.codigo) {
          d.code = e.codigo;
        }
        return d;
      });
      diagnosticos.set(documento.uri, lista);
    });
  }

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(comprobar),
    vscode.workspace.onDidSaveTextDocument(comprobar),
    vscode.workspace.onDidCloseTextDocument((d) => diagnosticos.delete(d.uri))
  );
  vscode.workspace.textDocuments.forEach(comprobar);
}

function deactivate() {}

module.exports = { activate, deactivate, analizarSalida };
