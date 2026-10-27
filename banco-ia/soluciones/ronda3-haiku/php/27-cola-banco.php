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

$lineas = leer_lineas();
$K = (int) trim($lineas[0] ?? '1');
$libre = array_fill(0, $K, 0);

$espera_total = 0;
$termina = 0;
$salida = [];

foreach (array_slice($lineas, 1) as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    if (count($p) < 3) {
        continue;
    }
    $llegada = (int) $p[0];
    $duracion = (int) $p[1];
    $nombre = $p[2];

    // Caja que queda libre antes; en empate, la de menor número.
    $elegida = 0;
    for ($c = 1; $c < $K; $c++) {
        if ($libre[$c] < $libre[$elegida]) {
            $elegida = $c;
        }
    }
    $empieza = max($llegada, $libre[$elegida]);
    $fin = $empieza + $duracion;
    $libre[$elegida] = $fin;
    $espera = $empieza - $llegada;

    $espera_total += $espera;
    if ($fin > $termina) {
        $termina = $fin;
    }
    $salida[] = $nombre . ': caja ' . ($elegida + 1) . ' empieza ' . $empieza . ' espera ' . $espera;
}

foreach ($salida as $s) {
    echo $s, "\n";
}
echo "espera total: $espera_total\n";
echo "termina: $termina\n";
