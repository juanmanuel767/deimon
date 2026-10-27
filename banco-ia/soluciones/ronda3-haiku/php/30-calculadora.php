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

// Analizador por descenso recursivo: expr -> termino -> unario -> primario.
function expr(array $t, int &$p): int
{
    $v = termino($t, $p);
    while ($p < count($t) && ($t[$p] === '+' || $t[$p] === '-')) {
        $op = $t[$p];
        $p++;
        $r = termino($t, $p);
        $v = $op === '+' ? $v + $r : $v - $r;
    }
    return $v;
}

function termino(array $t, int &$p): int
{
    $v = unario($t, $p);
    while ($p < count($t) && ($t[$p] === '*' || $t[$p] === '/')) {
        $op = $t[$p];
        $p++;
        $r = unario($t, $p);
        if ($op === '*') {
            $v = $v * $r;
        } else {
            if ($r === 0) {
                throw new RuntimeException('division por cero');
            }
            $v = intdiv($v, $r);
        }
    }
    return $v;
}

function unario(array $t, int &$p): int
{
    if ($t[$p] === '-') {
        $p++;
        return -unario($t, $p);
    }
    return primario($t, $p);
}

function primario(array $t, int &$p): int
{
    if ($t[$p] === '(') {
        $p++;
        $v = expr($t, $p);
        $p++; // paréntesis de cierre
        return $v;
    }
    $v = (int) $t[$p];
    $p++;
    return $v;
}

foreach (leer_lineas() as $linea) {
    $sin_espacios = preg_replace('/\s+/', '', $linea);
    preg_match_all('/[0-9]+|[-+*\/()]/', $sin_espacios, $m);
    $tokens = $m[0];
    if (empty($tokens)) {
        continue;
    }
    $p = 0;
    try {
        $v = expr($tokens, $p);
        echo $v, "\n";
    } catch (RuntimeException $e) {
        echo "error: division por cero\n";
    }
}
