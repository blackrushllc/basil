REM File I/O demo: open, write, readline, seek, eof

LET path$ = "examples_demo.txt";

' Write a few lines
LET w% = FOPEN(path$, "w");
FWRITELN(w%, "Alpha");
FWRITELN(w%, "Beta");
FWRITELN(w%, "Gamma");
FFLUSH(w%);
FCLOSE(w%);

' Read back line by line
LET r% = FOPEN(path$, "r");
WHILE NOT FEOF(r%) BEGIN
  LET line$ = FREADLINE$(r%);
  IF LEN(line$) > 0 THEN PRINTLN "LINE:", line$; ELSE PRINTLN "(blank line)";
END

' Seek demonstration
PRINTLN "--- SEEK DEMO ---";
FSEEK(r%, 0, 0);                ' rewind to beginning
PRINTLN "Pos=", FTELL&(r%);
LET first5$ = FREAD$(r%, 5);
PRINTLN "First 5 bytes:", first5$;
PRINTLN "Pos after read=", FTELL&(r%);
FCLOSE(r%);

DELETE(path$);  ' cleanup
