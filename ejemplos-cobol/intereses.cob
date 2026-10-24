       IDENTIFICATION DIVISION.
       PROGRAM-ID. INTERESES.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 CAPITAL   PIC 9(7)V99 VALUE 10000.
       77 TASA      PIC 9V9999 VALUE 0.0375.
       77 ANIO      PIC 99.
       77 INTERES   PIC 9(7)V99.
       77 TOTAL     PIC 9(7)V99.
       77 REDONDO   PIC 9(7)V99.
       77 PEQUENO   PIC 9(3) VALUE 5.
       PROCEDURE DIVISION.
           MOVE CAPITAL TO TOTAL.
           PERFORM VARYING ANIO FROM 1 BY 1 UNTIL ANIO > 5
               COMPUTE INTERES = TOTAL * TASA
               COMPUTE TOTAL = TOTAL + INTERES
               DISPLAY "Año " ANIO ": interes " INTERES " total " TOTAL
           END-PERFORM.
           COMPUTE REDONDO ROUNDED = CAPITAL / 7.
           DISPLAY "Un septimo redondeado: " REDONDO.
           COMPUTE REDONDO = CAPITAL / 7.
           DISPLAY "Un septimo truncado:   " REDONDO.
           COMPUTE PEQUENO = (CAPITAL - 8765.50) * 2 / 3.
           DISPLAY "Cuenta con truncado: " PEQUENO.
           DIVIDE 17 BY 5 GIVING ANIO REMAINDER PEQUENO.
           DISPLAY ANIO " resto " PEQUENO.
           STOP RUN.
