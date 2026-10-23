<?php
if (empty($_GET['nombre'])) {
    echo "<p>¿Cómo te llamas?</p>";
    exit;
}
$nombre = htmlspecialchars($_GET['nombre']);
echo "<p>Hola, $nombre</p>\n";
echo "<p>Tu nombre tiene " . strlen($nombre) . " letras</p>";
