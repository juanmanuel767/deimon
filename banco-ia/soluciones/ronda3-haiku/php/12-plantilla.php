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

$lineas = leer_lineas();
$plantilla = $lineas[0] ?? '';

$vars = [];
foreach (array_slice($lineas, 1) as $linea) {
    $pos = strpos($linea, '=');
    if ($pos === false) {
        continue;
    }
    $vars[substr($linea, 0, $pos)] = substr($linea, $pos + 1);
}

$salida = '';
$n = strlen($plantilla);
$i = 0;
while ($i < $n) {
    if ($plantilla[$i] === '{') {
        $fin = strpos($plantilla, '}', $i + 1);
        if ($fin !== false) {
            $nombre = substr($plantilla, $i + 1, $fin - $i - 1);
            if (preg_match('/\A[A-Za-z0-9_]+\z/', $nombre) === 1) {
                if (array_key_exists($nombre, $vars)) {
                    $salida .= $vars[$nombre];
                } else {
                    $salida .= '{' . $nombre . '}';
                }
                $i = $fin + 1;
                continue;
            }
        }
    }
    $salida .= $plantilla[$i];
    $i++;
}

echo $salida, "\n";
