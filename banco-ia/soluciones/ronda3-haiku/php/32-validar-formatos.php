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

function es_bisiesto(int $y): bool
{
    return ($y % 4 === 0 && $y % 100 !== 0) || $y % 400 === 0;
}

function dias_del_mes(int $y, int $m): int
{
    $dias = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if ($m === 2 && es_bisiesto($y)) {
        return 29;
    }
    return $dias[$m - 1];
}

function valido_correo(string $v): bool
{
    $partes = explode('@', $v);
    if (count($partes) !== 2) {
        return false;
    }
    if (preg_match('/\A[a-zA-Z0-9._-]+\z/', $partes[0]) !== 1) {
        return false;
    }
    $dominio = explode('.', $partes[1]);
    if (count($dominio) < 2) {
        return false;
    }
    foreach ($dominio as $parte) {
        if (preg_match('/\A[a-zA-Z0-9-]+\z/', $parte) !== 1) {
            return false;
        }
        if ($parte[0] === '-' || substr($parte, -1) === '-') {
            return false;
        }
    }
    return preg_match('/\A[a-zA-Z]{2,}\z/', end($dominio)) === 1;
}

function valido(string $tipo, string $v): bool
{
    switch ($tipo) {
        case 'correo':
            return valido_correo($v);
        case 'telefono':
            return preg_match('/\A[6-9][0-9]{8}\z/', $v) === 1;
        case 'fecha':
            if (preg_match('/\A([0-9]{2})\/([0-9]{2})\/([0-9]{4})\z/', $v, $m) !== 1) {
                return false;
            }
            $d = (int) $m[1];
            $mes = (int) $m[2];
            $anio = (int) $m[3];
            if ($anio < 1900 || $anio > 2100 || $mes < 1 || $mes > 12) {
                return false;
            }
            return $d >= 1 && $d <= dias_del_mes($anio, $mes);
        case 'cp':
            if (preg_match('/\A[0-9]{5}\z/', $v) !== 1) {
                return false;
            }
            $prefijo = (int) substr($v, 0, 2);
            return $prefijo >= 1 && $prefijo <= 52;
        default:
            return false;
    }
}

foreach (leer_lineas() as $linea) {
    $partes = explode(' ', trim($linea), 2);
    $tipo = $partes[0];
    $valor = $partes[1] ?? '';
    $resultado = valido($tipo, $valor) ? 'valido' : 'invalido';
    echo "$linea: $resultado\n";
}
