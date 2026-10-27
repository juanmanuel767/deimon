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

$lineas = leer_lineas();
[$F, $C] = array_map('intval', preg_split('/\s+/', trim($lineas[0]), -1, PREG_SPLIT_NO_EMPTY));

$g = [];
for ($i = 0; $i < $F; $i++) {
    $g[$i] = $lineas[$i + 1] ?? '';
}

$sf = $sc = $ef = $ec = -1;
for ($i = 0; $i < $F; $i++) {
    for ($j = 0; $j < $C; $j++) {
        $ch = substr($g[$i], $j, 1);
        if ($ch === 'S') {
            $sf = $i;
            $sc = $j;
        } elseif ($ch === 'E') {
            $ef = $i;
            $ec = $j;
        }
    }
}

$dist = [];
$dist[$sf][$sc] = 0;
$cola = [[$sf, $sc]];
$cabeza = 0;
$resultado = -1;
$movs = [[1, 0], [-1, 0], [0, 1], [0, -1]];

while ($cabeza < count($cola)) {
    [$f, $c] = $cola[$cabeza];
    $cabeza++;
    if ($f === $ef && $c === $ec) {
        $resultado = $dist[$f][$c];
        break;
    }
    foreach ($movs as [$df, $dc]) {
        $nf = $f + $df;
        $nc = $c + $dc;
        if ($nf < 0 || $nf >= $F || $nc < 0 || $nc >= $C) {
            continue;
        }
        $ch = substr($g[$nf], $nc, 1);
        if ($ch === '#' || $ch === '') {
            continue;
        }
        if (isset($dist[$nf][$nc])) {
            continue;
        }
        $dist[$nf][$nc] = $dist[$f][$c] + 1;
        $cola[] = [$nf, $nc];
    }
}

echo $resultado < 0 ? "sin camino\n" : $resultado . "\n";
