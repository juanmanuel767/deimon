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

$cierre = [')' => '(', ']' => '[', '}' => '{'];
foreach (leer_lineas() as $linea) {
    $pila = [];
    $ok = true;
    $n = strlen($linea);
    for ($i = 0; $i < $n && $ok; $i++) {
        $c = $linea[$i];
        if ($c === '(' || $c === '[' || $c === '{') {
            $pila[] = $c;
        } elseif (isset($cierre[$c])) {
            if (empty($pila) || array_pop($pila) !== $cierre[$c]) {
                $ok = false;
            }
        }
    }
    if (!empty($pila)) {
        $ok = false;
    }
    echo $ok ? "si\n" : "no\n";
}
