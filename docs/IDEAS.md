# Lenguaje nuevo: lista de ventajas

Objetivo: un lenguaje sencillo y seguro, tan rápido como Rust (por dentro se convierte en Rust), pensado para webs y datos, que sustituya a PHP.

**Meta**: convertirse en el centro de todos los lenguajes. Servir para lo que más se hace (webs, datos, aplicaciones) y traducir a sí mismo el código de los demás lenguajes (ventaja 6).

## Principios (decididos por el usuario)

1. **Diseñado para la IA**: una sola forma de hacer cada cosa; errores que dicen exactamente cómo arreglarse, también en formato legible por máquinas (`deimon comprobar --json`).
2. **Resolver un problema real**: sustituir lo frágil de PHP (piezas sueltas que se rompen al actualizar) por un solo programa seguro por defecto.
3. **Entender el entorno de ejecución**: orden `deimon entorno` que informa de qué hay disponible (puertos ocupados, Rust, SDK de Android, móvil conectado, carpeta de datos) antes de ejecutar.
4. **Verificar antes de dar el trabajo por terminado**: pruebas dentro del propio lenguaje (`prueba "nombre" { comprobar(...) }`) y orden `deimon probar`; `deimon app` no genera nada si alguna prueba falla.
5. **Fácil de adaptar**: núcleo pequeño y ampliable sin tocar lo demás.

## Objetivos de arquitectura (decididos por el usuario)

1. **Sintaxis coherente**: una sola forma de escribir cada cosa, con reglas que no tengan excepciones.
2. **Compilador inteligente**: el mismo código se transforma para distintas arquitecturas y plataformas (`deimon compilar --destino ...`).
3. **Núcleo multiplataforma**: los programas funcionan en distintos sistemas operativos y procesadores.
4. **Sistema de módulos**: capacidades especializadas en módulos aparte, sin complicar el lenguaje principal.
5. **Recursos adaptables**: funcionar tanto en un dispositivo limitado como en un servidor potente (perfiles «pequeño» y «servidor»).

## Ventajas que integrar

1. **Usar lo que ya existe**: poder usar las librerías de Rust desde el primer día.
2. **Fácil de aprender**: escribir mucho con poco, como Python.
3. **Seguro sin ser difícil**: la seguridad de Rust sin su dificultad.
4. **Hecho para que la IA lo escriba bien**: reglas claras y regulares, sin trampas.
5. **Un solo paso para instalar**: y con las herramientas incluidas (probar, formatear, publicar).
6. **Traductor de sistemas viejos**: convierte COBOL (bancos) o PHP a este lenguaje y **demuestra** que el resultado hace exactamente lo mismo que el original. Permite migrar por partes, con el sistema viejo y el nuevo funcionando a la vez.
