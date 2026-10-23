<?php
// Casos raros: nombres que chocan, textos con escapes, recursión y '.' entre números.
function fib($n) {
    if ($n < 2) { return $n; }
    return fib($n - 1) + fib($n - 2);
}
function medir($x) { return $x * 2; }

$si = 1;
$prueba = 2;
echo $si . $prueba, "\n";
echo 'Texto con \n literal y \'comillas\'' . "\n";
echo "Tab:\tfin \"citado\" \$no_variable\n";
$lista = array(3, 1, 2);
$lista[1] = 10;
echo $lista[0] + $lista[1] . "\n";
echo fib(20) . " " . medir(21) . "\n";
$texto = "";
for ($i = 0, $j = 10; $i < 3; $i++, $j--) {
    $texto .= $i . "-" . $j . " ";
}
echo trim($texto) . PHP_EOL;
if (!($si > 5) && $prueba !== 3) { echo "condición ok\n"; }
echo -5 + 2 * 3, "\n";
echo (10 - 4) / 4, "\n";
?>
Fin en HTML <?= strtoupper("ok") ?>
