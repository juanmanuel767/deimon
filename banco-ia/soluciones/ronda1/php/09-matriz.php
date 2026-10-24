<?php
$tokens = preg_split('/\s+/', trim((string) file_get_contents('php://stdin')), -1, PREG_SPLIT_NO_EMPTY);
$filas = (int) $tokens[0];
$columnas = (int) $tokens[1];
$m = [];
$k = 2;
for ($i = 0; $i < $filas; $i++) {
    for ($j = 0; $j < $columnas; $j++) {
        $m[$i][$j] = (int) ($tokens[$k] ?? 0);
        $k++;
    }
}
$res = [];
$top = 0;
$bottom = $filas - 1;
$left = 0;
$right = $columnas - 1;
while ($top <= $bottom && $left <= $right) {
    for ($j = $left; $j <= $right; $j++) {
        $res[] = $m[$top][$j];
    }
    $top++;
    for ($i = $top; $i <= $bottom; $i++) {
        $res[] = $m[$i][$right];
    }
    $right--;
    if ($top <= $bottom) {
        for ($j = $right; $j >= $left; $j--) {
            $res[] = $m[$bottom][$j];
        }
        $bottom--;
    }
    if ($left <= $right) {
        for ($i = $bottom; $i >= $top; $i--) {
            $res[] = $m[$i][$left];
        }
        $left++;
    }
}
echo implode(' ', $res), "\n";
