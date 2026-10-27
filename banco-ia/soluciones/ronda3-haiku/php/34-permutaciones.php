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

// Todas las permutaciones de una lista de letras ya ordenada, en orden alfabético.
function permutaciones(array $letras): array
{
    if (empty($letras)) {
        return [''];
    }
    $res = [];
    foreach ($letras as $i => $letra) {
        $resto = $letras;
        unset($resto[$i]);
        $resto = array_values($resto);
        foreach (permutaciones($resto) as $p) {
            $res[] = $letra . $p;
        }
    }
    return $res;
}

$lineas = leer_lineas();
$palabra = trim($lineas[0] ?? '');
if ($palabra === '') {
    exit(0);
}

// Letras UTF-8 completas (acentos y ñ incluidos), ordenadas por código.
$letras = preg_split('//u', $palabra, -1, PREG_SPLIT_NO_EMPTY);
sort($letras, SORT_STRING);

foreach (permutaciones($letras) as $p) {
    echo $p, "\n";
}
