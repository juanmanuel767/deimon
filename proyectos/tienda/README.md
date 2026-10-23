# Tienda Deimon

Tienda en línea completa escrita en Deimon: catálogo, carrito, cuentas de cliente,
pedidos con descuento seguro de existencias y un panel para el dueño.

## Arrancar

```bash
./arrancar.sh            # abre la tienda en http://127.0.0.1:8130
./arrancar.sh probar     # ejecuta las 6 pruebas automáticas
```

La **primera cuenta** que se crea es la del **dueño**: verá el enlace «Panel» para
añadir productos, cambiar existencias y marcar pedidos como enviados.

## Piezas

```
tienda.dmn     archivo principal: une las piezas y arranca el servidor (puerto 8130)
diseno.dmn     plantilla de las páginas, menú y formato del dinero (moneda)
catalogo.dmn   productos, existencias y validación de cantidades y precios
carrito.dmn    carrito en la sesión del visitante
pedidos.dmn    reservar existencias, crear pedidos y mostrarlos
paginas.dmn    todas las direcciones (catálogo, carrito, compra, cuentas, panel)
pruebas.dmn    pruebas automáticas (deimon probar tienda.dmn)
datos/         se crea al arrancar: productos, pedidos, cuentas y sesiones
```

## Cómo funciona

- **Productos** en la colección `productos` (`p0001`, `p0002`...); sus existencias van
  aparte, en `existencias`, para descontarlas con `incrementar` (seguro aunque compren
  dos personas a la vez). Si alguna no alcanza, se devuelve todo lo descontado.
- **Carrito**: lista con claves en la sesión (producto → unidades).
- **Pedidos** en `pedidos` (`00001`...), con líneas, total, dirección, fecha y estado
  (`pendiente` / `enviado`). El pago es al recibir el pedido.
- **Seguridad**: contraseñas con PBKDF2, bloqueo tras 5 intentos, campos escapados como
  HTML, panel solo para el dueño y cada cliente solo ve sus pedidos.
- **Moneda**: cámbiala en `moneda()` de `diseno.dmn`.

## Pendiente (opcional)

- Pago con tarjeta: el módulo `pagos` (Stripe) ya existe; hace falta una cuenta de Stripe.
- Correo de confirmación: el módulo `correo` necesita configurar `msmtp` en el ordenador.
- Para publicarla en internet con HTTPS: `deimon publicar tienda.dmn --dominio tu-dominio`.
