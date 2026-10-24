<?php
$lineas = preg_split('/\r?\n/', (string) file_get_contents('php://stdin'));
$personas = [];
foreach ($lineas as $linea) {
    $t = trim($linea);
    if ($t === '') {
        continue;
    }
    [$nombre, $edad] = array_map('trim', explode(',', $t, 2));
    $personas[] = ['nombre' => $nombre, 'edad' => (int) $edad];
}
usort($personas, function (array $a, array $b): int {
    if ($a['edad'] !== $b['edad']) {
        return $b['edad'] <=> $a['edad'];
    }
    return strcmp($a['nombre'], $b['nombre']);
});
$suma = 0;
foreach ($personas as $p) {
    echo $p['nombre'], ' (', $p['edad'], ")\n";
    $suma += $p['edad'];
}
$media = count($personas) > 0 ? $suma / count($personas) : 0;
echo 'media: ', number_format($media, 1, '.', ''), "\n";
