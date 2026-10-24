       IDENTIFICATION DIVISION.
       PROGRAM-ID. HOLA.
       ENVIRONMENT DIVISION.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 NOMBRE   PIC X(10) VALUE "Ana".
       77 EDAD     PIC 99 VALUE 7.
       77 CONTADOR PIC S9(3) VALUE -5.
       PROCEDURE DIVISION.
       PRINCIPAL.
           DISPLAY "Hola, mundo".
           DISPLAY "Me llamo " NOMBRE "y tengo " EDAD " años".
           MOVE "Luis Fernando" TO NOMBRE.
           ADD 120 TO EDAD.
           DISPLAY NOMBRE "|" EDAD "|" CONTADOR.
           IF EDAD NOT = 27 AND CONTADOR < 0
               DISPLAY "Distinta de 27 y contador negativo"
           ELSE
               DISPLAY "Otra cosa"
           END-IF.
           STOP RUN.
