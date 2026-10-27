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

$descartadas = 0;
$validas = 0;
$ciudades = [];
$numero = 0;

foreach (leer_lineas() as $linea) {
    $numero++;
    $campos = explode(';', $linea);
    $motivo = null;

    if (count($campos) !== 3) {
        $motivo = 'numero de campos incorrecto';
    } elseif ($campos[0] === '') {
        $motivo = 'nombre vacio';
    } elseif (preg_match('/\A[0-9]+\z/', $campos[1]) !== 1 || (int) $campos[1] > 120) {
        $motivo = 'edad invalida';
    } elseif ($campos[2] === '') {
        $motivo = 'ciudad vacia';
    }

    if ($motivo !== null) {
        $descartadas++;
        echo "linea $numero: $motivo\n";
    } else {
        $validas++;
        $ciudad = $campos[2];
        $ciudades[$ciudad] = ($ciudades[$ciudad] ?? 0) + 1;
    }
}

echo "validas: $validas\n";
echo "descartadas: $descartadas\n";

$nombres = [];
foreach (array_keys($ciudades) as $k) {
    $nombres[] = (string) $k;
}
sort($nombres, SORT_STRING);
foreach ($nombres as $c) {
    echo $c, ': ', $ciudades[$c], "\n";
}
