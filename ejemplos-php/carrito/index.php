<?php
session_start();
if (isset($_GET['agregar'])) {
    $_SESSION['carrito'][] = $_GET['agregar'];
    $_SESSION['ultimo'] = $_GET['agregar'];
    header("Location: index.php");
    exit;
}
echo "<h1>Tienda</h1>\n";
echo "<a href='index.php?agregar=taza'>Añadir taza</a> <a href='index.php?agregar=gorra'>Añadir gorra</a>\n";
if (isset($_SESSION['carrito'])) {
    echo "<p>Llevas " . count($_SESSION['carrito']) . " productos:</p><ul>\n";
    foreach ($_SESSION['carrito'] as $producto) {
        echo "<li>$producto</li>\n";
    }
    echo "</ul><p>El último fue: " . $_SESSION['ultimo'] . "</p>\n";
    echo "<a href='vaciar.php'>Vaciar</a>\n";
} else {
    echo "<p>Tu carrito está vacío.</p>\n";
}
