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

$personas = [];
foreach (leer_lineas() as $linea) {
    $t = trim($linea);
    if ($t === '') {
        continue;
    }
    $pos = strrpos($t, ',');
    if ($pos === false) {
        continue;
    }
    $personas[] = [
        'nombre' => substr($t, 0, $pos),
        'edad' => (int) substr($t, $pos + 1),
    ];
}

usort($personas, function ($a, $b) {
    if ($a['edad'] !== $b['edad']) {
        return $b['edad'] <=> $a['edad'];
    }
    return strcmp($a['nombre'], $b['nombre']);
});

$suma = 0;
foreach ($personas as $p) {
    echo $p['nombre'], ' (', $p['edad'], ")\n";
    $suma += $p['edad'];
}

$n = count($personas);
if ($n > 0) {
    // Media con 1 decimal, redondeo hacia arriba en la mitad: round(10*S/n).
    $r = intdiv(20 * $suma + $n, 2 * $n);
    echo 'media: ', intdiv($r, 10), '.', $r % 10, "\n";
}
