<?php

function leer_lineas(): array
{
    $texto = stream_get_contents(STDIN);
    if ($texto === false || $texto === '') {
        return [];
    }
    $texto = str_replace("\r\n", "\n", $texto);
    $lineas = explode("\n", $texto);
    if (end($lineas) === '') {
        array_pop($lineas);
    }
    return $lineas;
}

$pila = [];
$cola = [];
foreach (leer_lineas() as $linea) {
    $p = explode(' ', trim($linea), 2);
    $orden = $p[0];
    $arg = trim($p[1] ?? '');

    switch ($orden) {
        case 'apilar':
            $pila[] = $arg;
            break;
        case 'desapilar':
            if (empty($pila)) {
                echo "error: pila vacia\n";
            } else {
                $x = array_pop($pila);
                echo "pila->$x\n";
            }
            break;
        case 'encolar':
            $cola[] = $arg;
            break;
        case 'desencolar':
            if (empty($cola)) {
                echo "error: cola vacia\n";
            } else {
                $x = array_shift($cola);
                echo "cola->$x\n";
            }
            break;
        case 'mostrar':
            echo 'pila: ', empty($pila) ? 'vacia' : implode(' ', $pila), "\n";
            echo 'cola: ', empty($cola) ? 'vacia' : implode(' ', $cola), "\n";
            break;
    }
}
