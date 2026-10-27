# Días entre fechas

Cada línea tiene dos fechas válidas "AAAA-MM-DD AAAA-MM-DD" (años de 1600 a 2400, calendario
gregoriano: son bisiestos los años múltiplos de 4, salvo los múltiplos de 100 que no lo son de
400). Escribe los días que hay desde la primera fecha hasta la segunda (negativo si la segunda
es anterior; 0 si son iguales).

## Ejemplo

Entrada:

```
2024-01-01 2024-12-31
2000-02-28 2000-03-01
```

Salida:

```
365
2
```

Reglas comunes: lee TODA la entrada estándar (puede tener muchas líneas, o ninguna; la última
termina en salto de línea) y escribe exactamente la salida pedida, cada línea terminada en salto
de línea, sin espacios de más. «Orden alfabético» significa el orden de los códigos de los
caracteres (las mayúsculas van antes que las minúsculas y las letras con acento o la ñ, después de
la z), que es el que da ordenar() en Deimon. Se comprobará con varias entradas distintas, también
con casos límite que no se ven aquí.
