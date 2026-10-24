<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
$suma = 0;
foreach ($lineas as $linea) {
    $t = trim($linea);
    if ($t === '') {
        continue;
    }
    $suma += (int) $t;
}
echo $suma, "\n";
