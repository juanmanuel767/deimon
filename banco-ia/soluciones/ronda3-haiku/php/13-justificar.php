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

// Número de caracteres UTF-8 (cada carácter cuenta 1).
function clen(string $s): int
{
    return strlen(preg_replace('/[\x80-\xBF]/', '', $s));
}

function justificar(array $palabras, int $ancho): string
{
    $n = count($palabras);
    if ($n === 1) {
        return $palabras[0];
    }
    $letras = 0;
    foreach ($palabras as $p) {
        $letras += clen($p);
    }
    $huecos = $n - 1;
    $espacios = $ancho - $letras;
    $base = intdiv($espacios, $huecos);
    $extra = $espacios % $huecos;
    $s = $palabras[0];
    for ($i = 1; $i < $n; $i++) {
        $s .= str_repeat(' ', $base + (($i - 1) < $extra ? 1 : 0)) . $palabras[$i];
    }
    return $s;
}

$lineas = leer_lineas();
if (empty($lineas)) {
    exit(0);
}
$ancho = (int) trim($lineas[0]);

$palabras = [];
foreach (array_slice($lineas, 1) as $l) {
    foreach (preg_split('/\s+/', trim($l), -1, PREG_SPLIT_NO_EMPTY) as $p) {
        $palabras[] = $p;
    }
}

// Reparto voraz de palabras en líneas.
$grupos = [];
$actual = [];
$longitud = 0;
foreach ($palabras as $p) {
    $pl = clen($p);
    if (empty($actual)) {
        $actual = [$p];
        $longitud = $pl;
    } elseif ($longitud + 1 + $pl <= $ancho) {
        $actual[] = $p;
        $longitud += 1 + $pl;
    } else {
        $grupos[] = $actual;
        $actual = [$p];
        $longitud = $pl;
    }
}
if (!empty($actual)) {
    $grupos[] = $actual;
}

$ultimo = count($grupos) - 1;
foreach ($grupos as $idx => $g) {
    if ($idx === $ultimo || count($g) === 1) {
        echo implode(' ', $g), "\n";
    } else {
        echo justificar($g, $ancho), "\n";
    }
}
