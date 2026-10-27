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

// Nota decimal a centésimas (entero exacto): "7.5" -> 750, "10" -> 1000.
function a_centesimas(string $p): int
{
    $partes = explode('.', $p, 2);
    $entero = (int) $partes[0];
    $frac = isset($partes[1]) ? substr(str_pad($partes[1], 2, '0'), 0, 2) : '00';
    return $entero * 100 + (int) $frac;
}

// Texto con 2 decimales a partir de centésimas.
function fmt(int $c): string
{
    return intdiv($c, 100) . '.' . str_pad((string) ($c % 100), 2, '0', STR_PAD_LEFT);
}

// Calificación según el valor mostrado (en centésimas), como en el ejemplo del enunciado.
function calificacion(int $r): string
{
    if ($r < 500) {
        return 'suspenso';
    }
    if ($r < 700) {
        return 'aprobado';
    }
    if ($r < 900) {
        return 'notable';
    }
    return 'sobresaliente';
}

$alumnos = [];
$asignaturas = [];

foreach (leer_lineas() as $linea) {
    $p = explode(';', trim($linea));
    if (count($p) < 3) {
        continue;
    }
    $nota = a_centesimas(trim($p[2]));
    $alumnos[$p[0]]['suma'] = ($alumnos[$p[0]]['suma'] ?? 0) + $nota;
    $alumnos[$p[0]]['n'] = ($alumnos[$p[0]]['n'] ?? 0) + 1;
    $asignaturas[$p[1]]['suma'] = ($asignaturas[$p[1]]['suma'] ?? 0) + $nota;
    $asignaturas[$p[1]]['n'] = ($asignaturas[$p[1]]['n'] ?? 0) + 1;
}

// Valor mostrado = 10 x media, redondeado a 2 decimales (mitad hacia arriba).
// Con G = suma de notas en centésimas y n = número de notas: R = round(10 * G / n).
function valor_mostrado(int $G, int $n): int
{
    return intdiv(20 * $G + $n, 2 * $n);
}

function ordenar_claves(array $arr): array
{
    $claves = [];
    foreach (array_keys($arr) as $k) {
        $claves[] = (string) $k;
    }
    sort($claves, SORT_STRING);
    return $claves;
}

echo "== alumnos ==\n";
$mejor = null;
$mejor_valor = -1;
foreach (ordenar_claves($alumnos) as $nombre) {
    $a = $alumnos[$nombre];
    $r = valor_mostrado($a['suma'], $a['n']);
    echo $nombre, ': ', fmt($r), ' ', calificacion($r), "\n";
    if ($r > $mejor_valor) {
        $mejor_valor = $r;
        $mejor = $nombre;
    }
}

echo "== asignaturas ==\n";
foreach (ordenar_claves($asignaturas) as $asig) {
    $a = $asignaturas[$asig];
    echo $asig, ': ', fmt(valor_mostrado($a['suma'], $a['n'])), "\n";
}

echo 'mejor alumno: ', $mejor ?? 'nadie', "\n";
