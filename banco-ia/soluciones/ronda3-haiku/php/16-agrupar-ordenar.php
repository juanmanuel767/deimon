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

$grupos = [];
foreach (leer_lineas() as $linea) {
    $t = explode(',', trim($linea));
    if (count($t) < 3) {
        continue;
    }
    $grupos[$t[0]][] = ['nombre' => $t[1], 'salario' => (int) $t[2]];
}

$departamentos = [];
foreach (array_keys($grupos) as $k) {
    $departamentos[] = (string) $k;
}
sort($departamentos, SORT_STRING);

foreach ($departamentos as $d) {
    $personas = $grupos[$d];
    usort($personas, function ($a, $b) {
        if ($a['salario'] !== $b['salario']) {
            return $b['salario'] <=> $a['salario'];
        }
        return strcmp($a['nombre'], $b['nombre']);
    });
    $total = 0;
    foreach ($personas as $p) {
        $total += $p['salario'];
    }
    echo $d, ' (', count($personas), ' personas): total ', $total, "\n";
    foreach ($personas as $p) {
        echo '  ', $p['nombre'], ': ', $p['salario'], "\n";
    }
}
