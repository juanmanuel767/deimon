<?php
session_start();
unset($_SESSION['ultimo']);
session_destroy();
header("Location: index.php");
