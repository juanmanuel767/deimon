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

// Número grande como grupos de 4 cifras, menos significativo primero.
function a_limbs(string $s): array
{
    $r = [];
    for ($i = strlen($s); $i > 0; $i -= 4) {
        $ini = max(0, $i - 4);
        $r[] = (int) substr($s, $ini, $i - $ini);
    }
    return $r;
}

function a_texto(array $a): string
{
    while (count($a) > 1 && $a[count($a) - 1] === 0) {
        array_pop($a);
    }
    $n = count($a);
    $s = (string) $a[$n - 1];
    for ($i = $n - 2; $i >= 0; $i--) {
        $s .= str_pad((string) $a[$i], 4, '0', STR_PAD_LEFT);
    }
    return $s;
}

function multiplicar(array $a, array $b): array
{
    $r = array_fill(0, count($a) + count($b) + 1, 0);
    foreach ($a as $i => $x) {
        foreach ($b as $j => $y) {
            $r[$i + $j] += $x * $y;
        }
    }
    $carry = 0;
    for ($k = 0; $k < count($r); $k++) {
        $v = $r[$k] + $carry;
        $r[$k] = $v % 10000;
        $carry = intdiv($v, 10000);
    }
    return $r;
}

foreach (leer_lineas() as $linea) {
    $n = (int) trim($linea);
    $f = [1];
    for ($i = 2; $i <= $n; $i++) {
        $f = multiplicar($f, a_limbs((string) $i));
    }
    echo a_texto($f), "\n";
}
