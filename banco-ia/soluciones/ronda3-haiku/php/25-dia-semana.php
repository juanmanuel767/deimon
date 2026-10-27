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

function es_bisiesto(int $y): bool
{
    return ($y % 4 === 0 && $y % 100 !== 0) || $y % 400 === 0;
}

function dias_del_mes(int $y, int $m): int
{
    $dias = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if ($m === 2 && es_bisiesto($y)) {
        return 29;
    }
    return $dias[$m - 1];
}

// 0 = lunes ... 6 = domingo (el día juliano 0 fue lunes).
$nombres = ['lunes', 'martes', 'miercoles', 'jueves', 'viernes', 'sabado', 'domingo'];

foreach (leer_lineas() as $linea) {
    $t = trim($linea);
    if (preg_match('/\A([0-9]{4})-([0-9]{2})-([0-9]{2})\z/', $t, $m) !== 1) {
        echo "$linea: invalida\n";
        continue;
    }
    $y = (int) $m[1];
    $mes = (int) $m[2];
    $d = (int) $m[3];
    if ($mes < 1 || $mes > 12 || $d < 1 || $d > dias_del_mes($y, $mes)) {
        echo "$linea: invalida\n";
        continue;
    }
    $dia = $nombres[dias_jdn($y, $mes, $d) % 7];
    echo "$linea: $dia\n";
}
