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

function minusculas(string $s): string
{
    return strtr(strtolower($s), [
        'Á' => 'á', 'É' => 'é', 'Í' => 'í', 'Ó' => 'ó', 'Ú' => 'ú', 'Ü' => 'ü', 'Ñ' => 'ñ',
    ]);
}

$cuenta = [];
foreach (leer_lineas() as $linea) {
    foreach (explode(' ', $linea) as $trozo) {
        if ($trozo === '') {
            continue;
        }
        $palabra = trim(minusculas($trozo), '.,;:!?');
        if ($palabra === '') {
            continue;
        }
        $cuenta[$palabra] = ($cuenta[$palabra] ?? 0) + 1;
    }
}

$lista = [];
foreach ($cuenta as $p => $n) {
    $lista[] = ['palabra' => (string) $p, 'n' => $n];
}
usort($lista, function ($a, $b) {
    if ($a['n'] !== $b['n']) {
        return $b['n'] <=> $a['n'];
    }
    return strcmp($a['palabra'], $b['palabra']);
});

foreach (array_slice($lista, 0, 3) as $e) {
    echo $e['palabra'], ': ', $e['n'], "\n";
}
