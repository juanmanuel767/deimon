<?php
$resultado = [];
$i = 1;
while ($i <= 15) {
    if ($i % 15 == 0) {
        $resultado[] = "FizzBuzz";
    } elseif ($i % 3 == 0) {
        $resultado[] = "Fizz";
    } elseif ($i % 5 == 0) {
        $resultado[] = "Buzz";
    } else {
        $resultado[] = strval($i);
    }
    $i++;
}
echo implode(" ", $resultado) . "\n";
echo "Hay " . count($resultado) . " elementos\n";
