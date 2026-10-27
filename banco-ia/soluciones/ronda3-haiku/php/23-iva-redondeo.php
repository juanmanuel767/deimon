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

// Importe con como mucho 2 decimales a céntimos (entero exacto).
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

// Redondeo al entero más cercano, mitad hacia arriba, de M / 100 (M >= 0).
function redondear_centimos(int $m): int
{
    return intdiv(2 * $m + 100, 200);
}

$suma = 0;
foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    if (count($p) < 2) {
        continue;
    }
    $base = a_centimos($p[0]);
    $tipo = (int) $p[1];
    $iva = redondear_centimos($base * $tipo);
    $total = $base + $iva;
    $suma += $total;
    echo fmt($base), ' ', $tipo, '% -> iva ', fmt($iva), ' total ', fmt($total), "\n";
}
echo 'suma de totales: ', fmt($suma), "\n";
