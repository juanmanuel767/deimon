<?php

// Convierte céntimos enteros a texto con 2 decimales.
function fmt(int $c): string
{
    return intdiv($c, 100) . '.' . str_pad((string) ($c % 100), 2, '0', STR_PAD_LEFT);
}

$datos = json_decode(stream_get_contents(STDIN), true);
if (!is_array($datos)) {
    $datos = [];
}

echo 'tienda: ', $datos['tienda'] ?? '', "\n";

$agotados = 0;
$total = 0;
foreach ($datos['productos'] ?? [] as $p) {
    $precio = (int) ($p['precio'] ?? 0);
    $stock = (int) ($p['stock'] ?? 0);
    if ($stock > 0) {
        $valor = $precio * $stock;
        $total += $valor;
        echo $p['nombre'], ': ', fmt($precio), ' x ', $stock, ' = ', fmt($valor), "\n";
    } elseif ($stock === 0) {
        $agotados++;
    }
}

echo "agotados: $agotados\n";
echo 'valor total: ', fmt($total), "\n";
