<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
if (end($lineas) === '') {
    array_pop($lineas);
}
$suma = 0;
$salida = [];
foreach ($lineas as $i => $linea) {
    $t = trim($linea);
    if (preg_match('/^-?\d+$/', $t) === 1) {
        $suma += (int) $t;
    } else {
        $salida[] = 'linea ' . ($i + 1) . ': no es un numero: ' . $linea;
    }
}
$salida[] = 'suma: ' . $suma;
echo implode("\n", $salida), "\n";
