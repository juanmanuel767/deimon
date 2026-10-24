<?php
$texto = (string) file_get_contents('php://stdin');
$partes = preg_split('/\s+/u', $texto, -1, PREG_SPLIT_NO_EMPTY);
$conteo = [];
foreach ($partes as $parte) {
    $palabra = trim(mb_strtolower($parte, 'UTF-8'), '.,;:!?');
    if ($palabra === '') {
        continue;
    }
    $conteo[$palabra] = ($conteo[$palabra] ?? 0) + 1;
}
$lista = [];
foreach ($conteo as $palabra => $veces) {
    $lista[] = [(string) $palabra, $veces];
}
usort($lista, function (array $a, array $b): int {
    if ($a[1] !== $b[1]) {
        return $b[1] <=> $a[1];
    }
    return strcmp($a[0], $b[0]);
});
foreach (array_slice($lista, 0, 3) as [$palabra, $veces]) {
    echo $palabra, ': ', $veces, "\n";
}
