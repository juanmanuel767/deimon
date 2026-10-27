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

$tierra = function (int $i, int $j) use ($g, $F, $C): bool {
    return $i >= 0 && $i < $F && $j >= 0 && $j < $C && substr($g[$i], $j, 1) === '#';
};

$movs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
$visto = [];
$islas = 0;
$mayor = 0;

for ($i = 0; $i < $F; $i++) {
    for ($j = 0; $j < $C; $j++) {
        if (!$tierra($i, $j) || isset($visto[$i][$j])) {
            continue;
        }
        $islas++;
        $visto[$i][$j] = true;
        $pila = [[$i, $j]];
        $tam = 0;
        while (!empty($pila)) {
            [$a, $b] = array_pop($pila);
            $tam++;
            foreach ($movs as [$da, $db]) {
                $na = $a + $da;
                $nb = $b + $db;
                if ($tierra($na, $nb) && !isset($visto[$na][$nb])) {
                    $visto[$na][$nb] = true;
                    $pila[] = [$na, $nb];
                }
            }
        }
        $mayor = max($mayor, $tam);
    }
}

echo "islas: $islas\n";
echo "mayor: $mayor\n";
