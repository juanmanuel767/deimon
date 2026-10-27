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

// Convierte un importe con como mucho 2 decimales a céntimos (entero exacto).
function a_centimos(string $p): int
{
    $partes = explode('.', $p, 2);
    $entero = (int) $partes[0];
    $frac = isset($partes[1]) ? substr(str_pad($partes[1], 2, '0'), 0, 2) : '00';
    return $entero * 100 + (int) $frac;
}

function fmt(int $c): string
{
    return intdiv($c, 100) . '.' . str_pad((string) ($c % 100), 2, '0', STR_PAD_LEFT);
}

$totales = [];
$total = 0;
foreach (leer_lineas() as $linea) {
    $t = explode(';', trim($linea));
    if (count($t) < 3) {
        continue;
    }
    $producto = $t[0];
    $importe = (int) $t[1] * a_centimos(trim($t[2]));
    $totales[$producto] = ($totales[$producto] ?? 0) + $importe;
    $total += $importe;
}

$nombres = [];
foreach (array_keys($totales) as $k) {
    $nombres[] = (string) $k;
}
sort($nombres, SORT_STRING);

foreach ($nombres as $p) {
    echo $p, ': ', fmt($totales[$p]), "\n";
}
echo 'TOTAL: ', fmt($total), "\n";
