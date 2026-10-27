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

foreach (leer_lineas() as $linea) {
    if (strlen($linea) < 2) {
        continue;
    }
    $orden = $linea[0];
    $texto = substr($linea, 2);

    if ($orden === 'C') {
        // Letras UTF-8 completas (acentos y ñ incluidos).
        $letras = preg_split('//u', $texto, -1, PREG_SPLIT_NO_EMPTY);
        $salida = '';
        $i = 0;
        $n = count($letras);
        while ($i < $n) {
            $j = $i;
            while ($j < $n && $letras[$j] === $letras[$i]) {
                $j++;
            }
            $salida .= ($j - $i) . $letras[$i];
            $i = $j;
        }
        echo $salida, "\n";
    } elseif ($orden === 'D') {
        // Pares número + letra; la letra puede ser multibyte (/u).
        preg_match_all('/([0-9]+)([^0-9])/u', $texto, $m, PREG_SET_ORDER);
        $salida = '';
        foreach ($m as $par) {
            $salida .= str_repeat($par[2], (int) $par[1]);
        }
        echo $salida, "\n";
    }
}
