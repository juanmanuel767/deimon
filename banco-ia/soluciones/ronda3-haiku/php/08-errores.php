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

$suma = 0;
$numero = 0;
foreach (leer_lineas() as $linea) {
    $numero++;
    if (preg_match('/\A-?[0-9]+\z/', $linea) === 1) {
        $suma += (int) $linea;
    } else {
        echo "linea $numero: no es un numero: $linea\n";
    }
}
echo "suma: $suma\n";
