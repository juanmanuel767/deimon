<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
$totales = [];
$total = 0.0;
foreach ($lineas as $linea) {
    $t = trim($linea);
    if ($t === '') {
        continue;
    }
    [$producto, $cantidad, $precio] = array_map('trim', explode(';', $t));
    $importe = round((int) $cantidad * (float) $precio, 2);
    $totales[$producto] = ($totales[$producto] ?? 0.0) + $importe;
    $total += $importe;
}
ksort($totales, SORT_STRING);
foreach ($totales as $producto => $importe) {
    echo $producto, ': ', number_format($importe, 2, '.', ''), "\n";
}
echo 'TOTAL: ', number_format($total, 2, '.', ''), "\n";
