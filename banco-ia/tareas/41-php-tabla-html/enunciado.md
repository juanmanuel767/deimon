# Tabla HTML con datos escapados

Cada línea de la entrada tiene exactamente tres campos separados por «;»: nombre, cantidad y
comentario (cualquier texto salvo «;»; un campo puede estar vacío). Escribe una tabla HTML con
este formato exacto (cada etiqueta de fila en su propia línea, sin espacios ni sangría):

- primera línea `<table>`;
- la cabecera `<tr><th>Nombre</th><th>Cantidad</th><th>Comentario</th></tr>`;
- una fila por cada línea de entrada, en su orden: `<tr><td>NOMBRE</td><td>CANTIDAD</td><td>COMENTARIO</td></tr>`,
  pero las filas pares (la 2.ª, 4.ª, 6.ª... contando desde 1 las filas de datos, sin la cabecera)
  empiezan con `<tr class="par">` en vez de `<tr>`;
- última línea `</table>`.

Los tres campos se escriben escapados como HTML: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`,
`"` → `&quot;` y `'` → `&#039;` (cada carácter se sustituye una sola vez; un `&` ya escrito en la
entrada también se escapa). Sin entrada, solo salen `<table>`, la cabecera y `</table>`.

## Ejemplo

Entrada:

```
Pan;3;fresco & caliente
Leche;2;<b>entera</b>
Queso;1;"curado" de l'Alt
```

Salida:

```
<table>
<tr><th>Nombre</th><th>Cantidad</th><th>Comentario</th></tr>
<tr><td>Pan</td><td>3</td><td>fresco &amp; caliente</td></tr>
<tr class="par"><td>Leche</td><td>2</td><td>&lt;b&gt;entera&lt;/b&gt;</td></tr>
<tr><td>Queso</td><td>1</td><td>&quot;curado&quot; de l&#039;Alt</td></tr>
</table>
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
