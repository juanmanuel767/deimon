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

function hanoi(int $n, string $origen, string $destino, string $auxiliar): void
{
    if ($n === 0) {
        return;
    }
    hanoi($n - 1, $origen, $auxiliar, $destino);
    echo "mover disco $n de $origen a $destino\n";
    hanoi($n - 1, $auxiliar, $destino, $origen);
}

$lineas = leer_lineas();
$n = (int) trim($lineas[0] ?? '0');
if ($n < 0) {
    $n = 0;
}

hanoi($n, 'A', 'C', 'B');
echo 'total: ', (1 << $n) - 1, "\n";
