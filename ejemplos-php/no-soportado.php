<?php
class Gato { public $nombre; }
$edad = $_GET['edad'];
$tipo = $edad > 18 ? "adulto" : "menor";
switch ($tipo) { case "adulto": echo "a"; break; }
$datos = ["nombre" => "Ana"];
echo "seguimos\n";
echo sprintf("%d", 5);
