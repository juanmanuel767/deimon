<?php
// Un menú de opciones con switch.
function precio($plato) {
    switch ($plato) {
        case "sopa":
        case "ensalada":
            return 5;
        case "carne":
            return 12;
        default:
            return 0;
    }
}
$pedido = ["sopa", "carne", "agua", "ensalada"];
$total = 0;
foreach ($pedido as $p) {
    switch (strtoupper($p)) {
        case "AGUA":
            echo "$p: gratis\n";
            break;
        default:
            echo "$p: " . precio($p) . " euros\n";
            $total += precio($p);
            break;
    }
}
echo "Total: $total\n";
