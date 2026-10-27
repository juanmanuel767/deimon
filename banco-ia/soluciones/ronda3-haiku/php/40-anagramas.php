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

// Clave de anagrama: letras (UTF-8 completas) ordenadas por código.
function clave(string $w): string
{
    $letras = preg_split('//u', $w, -1, PREG_SPLIT_NO_EMPTY);
    sort($letras, SORT_STRING);
    return implode('', $letras);
}

$vistas = [];
$grupos = [];
foreach (leer_lineas() as $linea) {
    $w = trim($linea);
    if ($w === '' || isset($vistas[$w])) {
        continue;
    }
    $vistas[$w] = true;
    $grupos[clave($w)][] = $w;
}

$lineas_salida = [];
foreach ($grupos as $miembros) {
    if (count($miembros) < 2) {
        continue;
    }
    sort($miembros, SORT_STRING);
    $lineas_salida[] = $miembros;
}

usort($lineas_salida, fn($a, $b) => strcmp($a[0], $b[0]));

if (empty($lineas_salida)) {
    echo "ninguno\n";
}
foreach ($lineas_salida as $miembros) {
    echo implode(' ', $miembros), "\n";
}
