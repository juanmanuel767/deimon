<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
$tabla = [
    1000 => 'M', 900 => 'CM', 500 => 'D', 400 => 'CD',
    100 => 'C', 90 => 'XC', 50 => 'L', 40 => 'XL',
    10 => 'X', 9 => 'IX', 5 => 'V', 4 => 'IV', 1 => 'I',
];
foreach ($lineas as $linea) {
    if (trim($linea) === '') {
        continue;
    }
    $n = (int) trim($linea);
    $romano = '';
    foreach ($tabla as $valor => $simbolo) {
        while ($n >= $valor) {
            $romano .= $simbolo;
            $n -= $valor;
        }
    }
    echo $romano, "\n";
}
