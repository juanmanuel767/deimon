# IVA con redondeo exacto

Cada línea es "base tipo": la base imponible en euros con exactamente 2 decimales (por ejemplo
"12.35") y el tipo de IVA, un entero de 0 a 100 (%). El IVA, en céntimos, es base × tipo / 100
redondeado al céntimo más cercano, y si queda justo en la mitad, hacia arriba. El total es la
base más el IVA.

Para cada línea escribe "BASE TIPO% -> iva X total Y" con BASE, X e Y en euros con exactamente
2 decimales (BASE tal como se escribe con 2 decimales). Al final escribe "suma de totales: Z"
con la suma de todos los totales (2 decimales); con la entrada vacía escribe solo
"suma de totales: 0.00".

## Ejemplo

Entrada:

```
12.35 21
100.00 10
0.50 4
```

Salida:

```
12.35 21% -> iva 2.59 total 14.94
100.00 10% -> iva 10.00 total 110.00
0.50 4% -> iva 0.02 total 0.52
suma de totales: 125.46
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
