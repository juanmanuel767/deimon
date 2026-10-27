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

$nombres = ['norte', 'este', 'sur', 'oeste'];
// Desplazamientos por dirección: norte, este, sur, oeste.
$dxs = [0, 1, 0, -1];
$dys = [1, 0, -1, 0];

foreach (leer_lineas() as $linea) {
    $x = 0;
    $y = 0;
    $dir = 0;
    $max = 0;
    $n = strlen($linea);
    for ($i = 0; $i < $n; $i++) {
        $c = $linea[$i];
        if ($c === 'A') {
            $x += $dxs[$dir];
            $y += $dys[$dir];
            $d2 = $x * $x + $y * $y;
            if ($d2 > $max) {
                $max = $d2;
            }
        } elseif ($c === 'G') {
            $dir = ($dir + 1) % 4;
        } elseif ($c === 'I') {
            $dir = ($dir + 3) % 4;
        }
    }
    echo "$x,$y {$nombres[$dir]} max=$max\n";
}
