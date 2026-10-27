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

function sumar(array $x, array $y): array
{
    $r = [];
    $carry = 0;
    $n = max(count($x), count($y));
    for ($i = 0; $i < $n; $i++) {
        $s = ($x[$i] ?? 0) + ($y[$i] ?? 0) + $carry;
        $r[] = $s % 10000;
        $carry = intdiv($s, 10000);
    }
    if ($carry > 0) {
        $r[] = $carry;
    }
    return $r;
}

foreach (leer_lineas() as $linea) {
    $n = (int) trim($linea);
    $a = [0];
    $b = [1];
    for ($i = 0; $i < $n; $i++) {
        $c = sumar($a, $b);
        $a = $b;
        $b = $c;
    }
    echo a_texto($a), "\n";
}
