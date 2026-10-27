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

// Día juliano -> [año, mes, día] en calendario gregoriano.
function civil(int $j): array
{
    $a = $j + 32044;
    $b = intdiv(4 * $a + 3, 146097);
    $c = $a - intdiv(146097 * $b, 4);
    $d = intdiv(4 * $c + 3, 1461);
    $e = $c - intdiv(1461 * $d, 4);
    $m = intdiv(5 * $e + 2, 153);
    $dia = $e - intdiv(153 * $m + 2, 5) + 1;
    $mes = $m + 3 - 12 * intdiv($m, 10);
    $anio = 100 * $b + $d - 4800 + intdiv($m, 10);
    return [$anio, $mes, $dia];
}

foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    [$y, $m, $d] = array_map('intval', explode('-', $p[0]));
    $n = (int) $p[1];
    [$ay, $am, $ad] = civil(dias_jdn($y, $m, $d) + $n);
    printf("%04d-%02d-%02d\n", $ay, $am, $ad);
}
