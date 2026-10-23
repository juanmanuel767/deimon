<?php
$error = "";
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $correo = trim($_POST['correo']);
    $mensaje = trim($_POST['mensaje']);
    if ($correo == "" || $mensaje == "") {
        $error = "Rellena los dos campos.";
    } else {
        header("Location: gracias.php");
        exit;
    }
}
?>
<h1>Contacto</h1>
<?php if ($error != "") { echo "<p class='error'>" . $error . "</p>"; } ?>
<form method="post">
  <input name="correo"> <textarea name="mensaje"></textarea> <button>Enviar</button>
</form>
