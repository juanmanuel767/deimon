<?php
$titulo = "Mi tienda";
$productos = ["camiseta", "taza", "gorra", "mochila"];
?>
<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title><?= $titulo ?></title></head>
<body>
<h1><?php echo $titulo; ?></h1>
<form method="get" action="index.php">
  <input name="buscar" placeholder="Buscar">
  <button>Buscar</button>
</form>
<ul>
<?php
$buscar = "";
if (isset($_GET['buscar'])) {
    $buscar = strtolower(trim($_GET['buscar']));
}
$encontrados = 0;
foreach ($productos as $p) {
    if ($buscar == "" || str_contains($p, $buscar)) {
        echo "<li>" . ucfirst_falso($p) . "</li>\n";
        $encontrados++;
    }
}
function ucfirst_falso($t) {
    return strtoupper($t);
}
?>
</ul>
<p><?= $encontrados ?> productos</p>
<p><a href="contacto.php">Contacto</a> · <a href="saludo.php?nombre=Ana">Saludo</a></p>
</body>
</html>
