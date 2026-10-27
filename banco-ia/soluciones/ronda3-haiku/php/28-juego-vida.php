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
[$F, $C, $G] = array_map('intval', preg_split('/\s+/', trim($lineas[0]), -1, PREG_SPLIT_NO_EMPTY));

$g = [];
for ($i = 0; $i < $F; $i++) {
    $fila = $lineas[$i + 1] ?? '';
    for ($j = 0; $j < $C; $j++) {
        $g[$i][$j] = substr($fila, $j, 1) === '#' ? 1 : 0;
    }
}

for ($gen = 0; $gen < $G; $gen++) {
    $nuevo = [];
    for ($i = 0; $i < $F; $i++) {
        for ($j = 0; $j < $C; $j++) {
            $vecinas = 0;
            for ($di = -1; $di <= 1; $di++) {
                for ($dj = -1; $dj <= 1; $dj++) {
                    if ($di === 0 && $dj === 0) {
                        continue;
                    }
                    $ni = $i + $di;
                    $nj = $j + $dj;
                    if ($ni >= 0 && $ni < $F && $nj >= 0 && $nj < $C) {
                        $vecinas += $g[$ni][$nj];
                    }
                }
            }
            if ($g[$i][$j] === 1) {
                $nuevo[$i][$j] = ($vecinas === 2 || $vecinas === 3) ? 1 : 0;
            } else {
                $nuevo[$i][$j] = ($vecinas === 3) ? 1 : 0;
            }
        }
    }
    $g = $nuevo;
}

for ($i = 0; $i < $F; $i++) {
    $s = '';
    for ($j = 0; $j < $C; $j++) {
        $s .= $g[$i][$j] === 1 ? '#' : '.';
    }
    echo $s, "\n";
}
