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

$texto = implode("\n", leer_lineas());
preg_match_all('/[a-zA-ZáéíóúüñÁÉÍÓÚÜÑ]+/u', $texto, $m);

$cuenta = [];
$total = 0;
foreach ($m[0] as $palabra) {
    $p = minusculas($palabra);
    $cuenta[$p] = ($cuenta[$p] ?? 0) + 1;
    $total++;
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

echo "total: $total\n";
echo 'distintas: ', count($cuenta), "\n";
foreach (array_slice($lista, 0, 5) as $e) {
    echo $e['palabra'], ': ', $e['n'], "\n";
}
