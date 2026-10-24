       IDENTIFICATION DIVISION.
       PROGRAM-ID. CLASIFICAR.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 NOTA      PIC 99V9.
       77 CATEGORIA PIC X(10).
       77 DIA       PIC 9 VALUE 3.
       77 CONTADOR  PIC 9 VALUE 0.
       PROCEDURE DIVISION.
       PRINCIPAL.
           PERFORM CLASIFICAR-NOTA VARYING NOTA FROM 0 BY 2.5
               UNTIL NOTA > 10.
           PERFORM NOMBRE-DIA VARYING DIA FROM 1 BY 1 UNTIL DIA > 8.
           STOP RUN.
       CLASIFICAR-NOTA.
           EVALUATE TRUE
               WHEN NOTA < 5
                   MOVE "Suspenso" TO CATEGORIA
               WHEN NOTA >= 5 AND NOTA < 7
                   MOVE "Aprobado" TO CATEGORIA
               WHEN NOTA >= 7 AND NOTA < 9
                   MOVE "Notable" TO CATEGORIA
               WHEN OTHER
                   MOVE "Sobresalte" TO CATEGORIA
           END-EVALUATE.
           DISPLAY "Nota " NOTA " -> " CATEGORIA "|".
       NOMBRE-DIA.
           EVALUATE DIA
               WHEN 1 THRU 5
                   DISPLAY DIA " es laborable"
               WHEN 6
               WHEN 7
                   DISPLAY DIA " es fin de semana"
               WHEN OTHER
                   DISPLAY DIA " no existe"
           END-EVALUATE.
