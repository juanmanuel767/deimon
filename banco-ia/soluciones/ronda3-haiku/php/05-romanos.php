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

function romano(int $n): string
{
    $tabla = [
        1000 => 'M', 900 => 'CM', 500 => 'D', 400 => 'CD', 100 => 'C', 90 => 'XC',
        50 => 'L', 40 => 'XL', 10 => 'X', 9 => 'IX', 5 => 'V', 4 => 'IV', 1 => 'I',
    ];
    $r = '';
    foreach ($tabla as $valor => $simbolo) {
        while ($n >= $valor) {
            $r .= $simbolo;
            $n -= $valor;
        }
    }
    return $r;
}

foreach (leer_lineas() as $linea) {
    $t = trim($linea);
    if ($t === '') {
        continue;
    }
    echo romano((int) $t), "\n";
}
