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

// Inserta un valor en el árbol siguiendo las partes de la ruta.
function insertar(array $nodo, array $partes, string $valor): array
{
    $k = array_shift($partes);
    if (empty($partes)) {
        $nodo[$k] = $valor;
        return $nodo;
    }
    if (!isset($nodo[$k]) || !is_array($nodo[$k])) {
        $nodo[$k] = [];
    }
    $nodo[$k] = insertar($nodo[$k], $partes, $valor);
    return $nodo;
}

function imprimir(array $nodo, int $nivel): void
{
    $claves = [];
    foreach (array_keys($nodo) as $k) {
        $claves[] = (string) $k;
    }
    sort($claves, SORT_STRING);
    $sangria = str_repeat('  ', $nivel);
    foreach ($claves as $c) {
        $v = $nodo[$c];
        if (is_array($v)) {
            echo $sangria, $c, ":\n";
            imprimir($v, $nivel + 1);
        } else {
            echo $sangria, $c, ': ', $v, "\n";
        }
    }
}

$arbol = [];
foreach (leer_lineas() as $linea) {
    $pos = strpos($linea, '=');
    if ($pos === false) {
        continue;
    }
    $ruta = substr($linea, 0, $pos);
    $valor = substr($linea, $pos + 1);
    $arbol = insertar($arbol, explode('.', $ruta), $valor);
}

imprimir($arbol, 0);
