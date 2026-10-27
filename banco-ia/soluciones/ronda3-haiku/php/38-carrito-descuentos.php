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

// Redondeo de M / 100 (M >= 0) al entero más cercano, mitad hacia arriba.
function redondear(int $m): int
{
    return intdiv(2 * $m + 100, 200);
}

$salida = [];
$subtotal = 0;

foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    if (count($p) < 3) {
        continue;
    }
    $producto = $p[0];
    $cantidad = (int) $p[1];
    $precio = a_centimos($p[2]);

    $bruto = $cantidad * $precio;
    if ($cantidad >= 10) {
        $descuento = 10;
    } elseif ($cantidad >= 5) {
        $descuento = 5;
    } else {
        $descuento = 0;
    }
    $importe = redondear($bruto * (100 - $descuento));
    $subtotal += $importe;

    $texto = $producto . ' x' . $cantidad . ': ' . fmt($importe);
    if ($descuento !== 0) {
        $texto .= ' (-' . $descuento . '%)';
    }
    $salida[] = $texto;
}

foreach ($salida as $s) {
    echo $s, "\n";
}
echo 'subtotal: ', fmt($subtotal), "\n";

$total = $subtotal;
if ($subtotal >= 10000) {
    $total = redondear($subtotal * 95);
    echo 'descuento pedido: -', fmt($subtotal - $total), "\n";
}

$envio = $total >= 5000 ? 0 : 495;
echo 'envio: ', fmt($envio), "\n";
echo 'total: ', fmt($total + $envio), "\n";
