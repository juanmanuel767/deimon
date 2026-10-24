<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
$inventario = [];
foreach ($lineas as $linea) {
    $partes = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    if ($partes === []) {
        continue;
    }
    $orden = $partes[0];
    if ($orden === 'poner' && count($partes) >= 3) {
        $producto = $partes[1];
        $n = (int) $partes[2];
        $inventario[$producto] = ($inventario[$producto] ?? 0) + $n;
    } elseif ($orden === 'quitar' && count($partes) >= 3) {
        $producto = $partes[1];
        $n = (int) $partes[2];
        if (($inventario[$producto] ?? 0) >= $n) {
            $inventario[$producto] -= $n;
        } else {
            echo "error: no hay $n de $producto\n";
        }
    } elseif ($orden === 'ver') {
        $con = array_filter($inventario, fn($u) => $u > 0);
        if ($con === []) {
            echo "vacio\n";
        } else {
            ksort($con, SORT_STRING);
            foreach ($con as $p => $u) {
                echo "$p=$u\n";
            }
        }
    }
}
