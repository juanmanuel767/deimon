# Agrupar y ordenar por varios campos

Cada línea es "departamento,nombre,salario" (salario entero). Agrupa por departamento. Escribe
los departamentos en orden alfabético; cada uno con una línea "departamento (N personas): total T",
donde N es cuántas personas tiene (siempre se escribe la palabra "personas") y T la suma de sus
salarios; y debajo una línea por persona, con dos espacios de sangría y el formato
"nombre: salario", ordenadas por salario de mayor a menor y, si empatan, por nombre en orden
alfabético. Si una misma persona aparece varias veces, cada línea cuenta como una persona más.

## Ejemplo

Entrada:

```
ventas,Ana,1500
compras,Luis,1200
ventas,Bea,1800
compras,Carmen,1200
ventas,Dani,1500
```

Salida:

```
compras (2 personas): total 2400
  Carmen: 1200
  Luis: 1200
ventas (3 personas): total 4800
  Bea: 1800
  Ana: 1500
  Dani: 1500
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
