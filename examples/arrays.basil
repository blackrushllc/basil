REM Arrays example for BASIL
REM Demonstrates DIM for string ($), integer (%), and float arrays, with up to 2 dimensions

REM --- 1D integer array (0..5 inclusive => length 6) ---
DIM N%(5);
LET N%(0) = 10;
LET N%(5) = 99;
PRINTLN "N%(0)=", N%(0), ", N%(5)=", N%(5);
PRINTLN "LEN(N%)=", LEN(N%);

REM --- 1D float array (0..3 inclusive => length 4) ---
DIM X(3);
LET X(0) = 1.5;
LET X(3) = 2.5;
PRINTLN "X(0)=", X(0), ", X(3)=", X(3);
PRINTLN "LEN(X)=", LEN(X);

REM --- 2D string array (0..2 by 0..1 => 3 x 2 = 6 elements) ---
DIM S$(2,1);
LET S$(0,0) = "Hello";
LET S$(2,1) = "World";
PRINTLN "S$(0,0)=", S$(0,0), ", S$(2,1)=", S$(2,1);
PRINTLN "LEN(S$)=", LEN(S$);

REM Show that re-DIM resets the array
DIM S$(1,0); REM now capacity is 2 x 1 = 2 elements, previous contents cleared
LET S$(1,0) = "Reset";
PRINTLN "After re-DIM, LEN(S$)=", LEN(S$), "; S$(1,0)=", S$(1,0);
