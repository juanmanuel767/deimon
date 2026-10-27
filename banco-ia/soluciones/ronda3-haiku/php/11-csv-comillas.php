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

// Separa una fila CSV con campos opcionalmente entre comillas simples.
function campos_csv(string $l): array
{
    $res = [];
    $n = strlen($l);
    $i = 0;
    while (true) {
        if ($i < $n && $l[$i] === "'") {
            $fin = strpos($l, "'", $i + 1);
            if ($fin === false) {
                $fin = $n;
            }
            $res[] = substr($l, $i + 1, $fin - $i - 1);
            $i = $fin + 1;
        } else {
            $fin = strpos($l, ',', $i);
            if ($fin === false) {
                $fin = $n;
            }
            $res[] = substr($l, $i, $fin - $i);
            $i = $fin;
        }
        if ($i >= $n) {
            break;
        }
        // Aquí hay una coma separadora: se salta.
        $i++;
        if ($i >= $n) {
            $res[] = '';
            break;
        }
    }
    return $res;
}

foreach (leer_lineas() as $linea) {
    $c = campos_csv($linea);
    $partes = '';
    foreach ($c as $v) {
        $partes .= '[' . $v . ']';
    }
    echo count($c), ' campos: ', $partes, "\n";
}
