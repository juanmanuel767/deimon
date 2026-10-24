<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
if (end($lineas) === '') {
    array_pop($lineas);
}
$pares = [')' => '(', ']' => '[', '}' => '{'];
foreach ($lineas as $linea) {
    $pila = [];
    $ok = true;
    for ($i = 0, $len = strlen($linea); $i < $len; $i++) {
        $c = $linea[$i];
        if ($c === '(' || $c === '[' || $c === '{') {
            $pila[] = $c;
        } elseif (isset($pares[$c])) {
            if ($pila === [] || array_pop($pila) !== $pares[$c]) {
                $ok = false;
                break;
            }
        }
    }
    if ($pila !== []) {
        $ok = false;
    }
    echo $ok ? "si\n" : "no\n";
}
