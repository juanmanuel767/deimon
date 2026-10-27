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

$stock = [];
foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    $orden = $p[0] ?? '';

    if ($orden === 'poner') {
        $producto = $p[1];
        $n = (int) $p[2];
        $stock[$producto] = ($stock[$producto] ?? 0) + $n;
    } elseif ($orden === 'quitar') {
        $producto = $p[1];
        $n = (int) $p[2];
        $actual = $stock[$producto] ?? 0;
        if ($actual >= $n) {
            $stock[$producto] = $actual - $n;
        } else {
            echo "error: no hay $n de $producto\n";
        }
    } elseif ($orden === 'ver') {
        $nombres = [];
        foreach ($stock as $k => $v) {
            if ($v > 0) {
                $nombres[] = (string) $k;
            }
        }
        if (empty($nombres)) {
            echo "vacio\n";
            continue;
        }
        sort($nombres, SORT_STRING);
        foreach ($nombres as $k) {
            echo $k, '=', $stock[$k], "\n";
        }
    }
}
