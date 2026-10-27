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

// Importe en céntimos a texto con 2 decimales.
function fmt(int $c): string
{
    return intdiv($c, 100) . '.' . str_pad((string) ($c % 100), 2, '0', STR_PAD_LEFT);
}

$productos = [];
$ingresos_totales = 0;

foreach (leer_lineas() as $linea) {
    $p = preg_split('/\s+/', trim($linea), -1, PREG_SPLIT_NO_EMPTY);
    $orden = $p[0] ?? '';

    if ($orden === 'alta') {
        $codigo = $p[1];
        if (isset($productos[$codigo])) {
            echo "error: codigo duplicado $codigo\n";
        } else {
            $productos[$codigo] = [
                'nombre' => $p[2],
                'precio' => (int) $p[3],
                'stock' => (int) $p[4],
                'ingresos' => 0,
            ];
        }
    } elseif ($orden === 'venta') {
        $codigo = $p[1];
        $n = (int) $p[2];
        if (!isset($productos[$codigo])) {
            echo "error: codigo desconocido $codigo\n";
        } elseif ($productos[$codigo]['stock'] < $n) {
            $hay = $productos[$codigo]['stock'];
            echo "error: stock insuficiente de $codigo (hay $hay, piden $n)\n";
        } else {
            $importe = $n * $productos[$codigo]['precio'];
            $productos[$codigo]['stock'] -= $n;
            $productos[$codigo]['ingresos'] += $importe;
            $ingresos_totales += $importe;
        }
    } elseif ($orden === 'reponer') {
        $codigo = $p[1];
        $n = (int) $p[2];
        if (!isset($productos[$codigo])) {
            echo "error: codigo desconocido $codigo\n";
        } else {
            $productos[$codigo]['stock'] += $n;
        }
    } elseif ($orden === 'informe') {
        $codigos = [];
        foreach (array_keys($productos) as $k) {
            $codigos[] = (string) $k;
        }
        sort($codigos, SORT_STRING);

        $sin_stock = [];
        foreach ($codigos as $c) {
            $pr = $productos[$c];
            echo $c, ' ', $pr['nombre'], ' stock=', $pr['stock'], ' ingresos=', fmt($pr['ingresos']), "\n";
            if ($pr['stock'] === 0) {
                $sin_stock[] = $c;
            }
        }
        echo 'ingresos totales: ', fmt($ingresos_totales), "\n";
        echo 'sin stock: ', empty($sin_stock) ? 'ninguno' : implode(', ', $sin_stock), "\n";
    }
}
