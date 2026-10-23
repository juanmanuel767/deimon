<?php
// Calcula el total de un carrito con descuento.
function precio_con_iva($precio) {
    return $precio * 1.21;
}

function descuento($total) {
    if ($total > 100) {
        return 10;
    } elseif ($total > 50) {
        return 5;
    } else {
        return 0;
    }
}

$productos = ["camiseta", "taza", "gorra"];
$precios = [20, 8.5, 15];
$total = 0;

for ($i = 0; $i < count($precios); $i++) {
    $linea = precio_con_iva($precios[$i]);
    $total += $linea;
    echo "Producto: " . strtoupper($productos[$i]) . " -> " . round($linea) . "\n";
}

$rebaja = descuento($total);
echo "Descuento: $rebaja%\n";
echo "Total: " . round($total - $rebaja) . " euros\n";

$nombres = explode(",", "ana,luis,eva");
foreach ($nombres as $n) {
    if (str_starts_with($n, "e") || strlen($n) == 3) {
        echo "Hola, {$n}! ";
    }
}
echo PHP_EOL;
echo implode(" | ", $nombres), "\n";
