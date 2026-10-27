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

$tokens = preg_split('/\s+/', trim(implode(' ', leer_lineas())), -1, PREG_SPLIT_NO_EMPTY);
$F = (int) ($tokens[0] ?? 0);
$C = (int) ($tokens[1] ?? 0);

$m = [];
$k = 2;
for ($i = 0; $i < $F; $i++) {
    for ($j = 0; $j < $C; $j++) {
        $m[$i][$j] = (int) ($tokens[$k] ?? 0);
        $k++;
    }
}

$res = [];
$t = 0;
$b = $F - 1;
$l = 0;
$r = $C - 1;
while ($t <= $b && $l <= $r) {
    for ($j = $l; $j <= $r; $j++) {
        $res[] = $m[$t][$j];
    }
    $t++;
    for ($i = $t; $i <= $b; $i++) {
        $res[] = $m[$i][$r];
    }
    $r--;
    if ($t <= $b) {
        for ($j = $r; $j >= $l; $j--) {
            $res[] = $m[$b][$j];
        }
        $b--;
    }
    if ($l <= $r) {
        for ($i = $b; $i >= $t; $i--) {
            $res[] = $m[$i][$l];
        }
        $l++;
    }
}

echo implode(' ', $res), "\n";
