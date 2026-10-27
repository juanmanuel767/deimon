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

// Número de día juliano (calendario gregoriano).
function dias_jdn(int $y, int $m, int $d): int
{
    $a = intdiv(14 - $m, 12);
    $yy = $y + 4800 - $a;
    $mm = $m + 12 * $a - 3;
    return $d + intdiv(153 * $mm + 2, 5) + 365 * $yy + intdiv($yy, 4)
        - intdiv($yy, 100) + intdiv($yy, 400) - 32045;
}

foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    [$y1, $m1, $d1] = array_map('intval', explode('-', $p[0]));
    [$y2, $m2, $d2] = array_map('intval', explode('-', $p[1]));
    echo dias_jdn($y2, $m2, $d2) - dias_jdn($y1, $m1, $d1), "\n";
}
