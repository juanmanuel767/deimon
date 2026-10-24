<?php
// Formatos de números y textos: number_format, str_pad, ucfirst y print_r.
$precios = ["pan" => 1.5, "leche" => 0.99, "queso" => 12.345];
foreach ($precios as $producto => $precio) {
    echo str_pad(ucfirst($producto), 10, ".", STR_PAD_RIGHT) . number_format($precio, 2, ",", ".") . " €\n";
}
echo str_pad("Total", 10, " ", STR_PAD_LEFT) . number_format(14.835, 2, ",", ".") . " €\n";
print_r(["frutas" => ["pera", "uva"], "cantidad" => [3, 5]]);
