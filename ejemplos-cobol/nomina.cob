       IDENTIFICATION DIVISION.
       PROGRAM-ID. NOMINA.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 EMPLEADO.
          05 NOMBRE        PIC X(12) VALUE "Marta Perez".
          05 HORAS         PIC 9(3)V9 VALUE 160.5.
          05 TARIFA        PIC 9(3)V99 VALUE 12.75.
       77 BRUTO            PIC 9(6)V99.
       77 RETENCION        PIC 9(6)V99.
       77 NETO             PIC S9(6)V99.
       77 EXTRA            PIC 9(4)V99 VALUE 0.
       PROCEDURE DIVISION.
       INICIO.
           MULTIPLY HORAS BY TARIFA GIVING BRUTO.
           COMPUTE RETENCION = BRUTO * 0.15.
           SUBTRACT RETENCION FROM BRUTO GIVING NETO.
           PERFORM MOSTRAR-RECIBO.
           ADD 250.40 TO EXTRA.
           ADD EXTRA TO NETO.
           DIVIDE NETO BY 3 GIVING EXTRA.
           DISPLAY "Una tercera parte: " EXTRA.
           PERFORM MOSTRAR-RECIBO.
           STOP RUN.
       MOSTRAR-RECIBO.
           DISPLAY "Empleado: " NOMBRE.
           DISPLAY "Horas:    " HORAS.
           DISPLAY "Bruto:    " BRUTO.
           DISPLAY "Retencion:" RETENCION.
           DISPLAY "Neto:     " NETO.
