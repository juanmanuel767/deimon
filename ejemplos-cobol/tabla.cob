       IDENTIFICATION DIVISION.
       PROGRAM-ID. TABLA.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 N         PIC 99.
       77 I         PIC 99.
       77 RESULTADO PIC 9(3).
       77 VUELTAS   PIC 9 VALUE 0.
       PROCEDURE DIVISION.
       PRINCIPAL.
           PERFORM VARYING N FROM 2 BY 1 UNTIL N > 3
               DISPLAY "Tabla del " N
               PERFORM FILA VARYING I FROM 1 BY 1 UNTIL I > 4
           END-PERFORM.
           PERFORM 3 TIMES
               ADD 1 TO VUELTAS
           END-PERFORM.
           DISPLAY "Vueltas: " VUELTAS.
           MOVE 1 TO I.
           PERFORM UNTIL I > 3
               COMPUTE RESULTADO = I * I * I
               DISPLAY "Cubo de " I " = " RESULTADO
               ADD 1 TO I
           END-PERFORM.
           STOP RUN.
       FILA.
           COMPUTE RESULTADO = N * I.
           DISPLAY N " x " I " = " RESULTADO.
