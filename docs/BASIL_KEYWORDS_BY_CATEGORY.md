# Basil Keywords by Category

This document reorganizes the Basil language reference by category. Entries are grouped under their Type and listed alphabetically within each category. Descriptions and examples are taken verbatim from BASIL_REFERENCE.md.

## Statements

### AS
Specifies a type name for DIM of object variables or arrays.
```basil
DIM r1@ AS BMX_RIDER
```

### DESCRIBE
Prints a formatted description of an object instance or an array value.
```basil
DESCRIBE r1@;
```

### DIM
Declares a numeric array, object array, or object variable (with AS and optional constructor args).
```basil
DIM A(10, 20);
DIM riders@(10) AS BMX_RIDER;
DIM team@ AS BMX_TEAM("Rockets");
```

### FUNC
Declares a function with an optional BEGIN…END block or implicit block terminated by END [FUNC].
```basil
FUNC Add(a, b)
BEGIN
  RETURN a + b;
END
```

### LET
Assigns a value to a variable, array element, or object property (property assignment may also omit LET).
```basil
LET A = 42;  LET arr(1,2) = 7;  obj.Prop = 10;
```

### PRINT
Prints an expression (or expressions separated by commas, which insert TABs) without a trailing newline.
```basil
PRINT "Hello, "; PRINT "world!";
```

### PRINTLN
Prints an expression followed by a newline.
```basil
PRINTLN "Hello";
```

### RETURN
Returns from a function, optionally with a value.
```basil
RETURN 42;
```

## Functions

### ASC%
Returns the ASCII/Unicode code point of the first character of a string, or 0 if empty.
```basil
LET code% = ASC%("A");
```

### AUTHOR
Constant function-like keyword that yields the Basil author name; accepts optional empty parentheses.
```basil
PRINTLN AUTHOR;
```

### CHR$
Returns a one-character string for the given numeric code point (out of range yields "").
```basil
PRINTLN CHR$(65);
```

### CLASS
Constructs a class instance from a filename that defines a class.
```basil
LET x@ = CLASS("my_widget.cls");
```

### DESCRIBE$
Returns a formatted description string of an object instance or array value.
```basil
PRINTLN DESCRIBE$(r1@);
```

### GET$
Returns an array of GET query parameters (as strings) in CGI mode.
```basil
LET params$@ = GET$();
```

### HTML
Escapes special HTML characters of its argument; alias of HTML$.
```basil
PRINTLN HTML("<b>& ok</b>");
```

### HTML$
Escapes special HTML characters of its argument.
```basil
PRINTLN HTML$("<b>& ok</b>");
```

### INKEY%
Non-blocking key read; returns key code (0 if no key available).
```basil
LET k% = INKEY%();
```

### INKEY$
Non-blocking key read; returns one-character string ("" if no key available).
```basil
LET k$ = INKEY$();
```

### INPUT
Alias of INPUT$; reads a line from standard input without trailing CR/LF.
```basil
LET name$ = INPUT("Enter your name: ");
```

### INPUT$
Reads a line from standard input without trailing CR/LF (optionally prints a prompt first).
```basil
LET name$ = INPUT$("Enter your name: ");
```

### INPUTC$
Reads a single ASCII character (echoed once) from input; returns "" for non-ASCII or Enter.
```basil
LET ch$ = INPUTC$("Press a key: ");
```

### INSTR
Finds the position (0-based) of a substring within a string starting at an optional index (0 if not found).
```basil
LET p% = INSTR("banana", "na", 2);
```

### LCASE$
Returns the lowercase version of a string.
```basil
PRINTLN LCASE$("MiXeD");
```

### LEFT$
Returns the leftmost N characters of a string.
```basil
PRINTLN LEFT$("basil", 2);
```

### LEN
Returns string character length or total element count of an array; other values are converted to strings.
```basil
PRINTLN LEN("hello");
```

### MID$
Returns a substring starting at 1-based index, with optional length.
```basil
PRINTLN MID$("banana", 2, 3);
```

### NEW
Constructs a new object instance of a registered type with constructor arguments.
```basil
LET r@ = NEW BMX_RIDER("Alex", 12, 5);
```

### POST$
Returns an array of POST body parameters (as strings) in CGI mode.
```basil
LET form$@ = POST$();
```

### REQUEST$
Returns GET and POST parameters combined (as strings) in CGI mode.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

### RIGHT$
Returns the rightmost N characters of a string.
```basil
PRINTLN RIGHT$("basil", 3);
```

### TRIM$
Returns the input string with leading and trailing whitespace removed.
```basil
PRINTLN TRIM$("  hi  ");
```

### TYPE$
Returns a string that names the Basil type of its argument (e.g., "String", "Int", "Array", "Object").
```basil
PRINTLN TYPE$(42);
```

### UCASE$
Returns the uppercase version of a string.
```basil
PRINTLN UCASE$("basil");
```

## Flow Control

### BEGIN
Begins a block of statements to be terminated by END.
```basil
BEGIN
  PRINTLN 1;
  PRINTLN 2;
END
```

### BREAK
Exits the nearest enclosing loop.
```basil
FOR I = 1 TO 10 BEGIN IF I = 5 THEN BREAK; END NEXT
```

### CONTINUE
Skips to the next iteration of the nearest enclosing loop.
```basil
FOR I = 1 TO 5 BEGIN IF I = 3 THEN CONTINUE; PRINT I; END NEXT
```

### DO
Reserved for future DO/LOOP constructs; not currently implemented.
```basil
REM Reserved: DO ... LOOP UNTIL cond
```

### EACH
Used with FOR to begin a FOR EACH iteration over an enumerable value.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

### ELSE
Introduces the alternative branch of an IF statement.
```basil
IF X > 0 THEN PRINTLN "pos"; ELSE PRINTLN "non-pos";
```

### ENDFOR
Reserved synonym for closing a FOREACH loop; current syntax uses NEXT.
```basil
REM Reserved: FOREACH x IN arr ... ENDFOR
```

### FOR
Starts a numeric FOR…TO…[STEP]…NEXT loop or a FOR EACH…IN…NEXT enumeration loop.
```basil
FOR I = 1 TO 3 STEP 1 PRINT I; NEXT
```

### FOREACH
Reserved single-word form of FOR EACH; use "FOR EACH" in current Basil.
```basil
REM Reserved: FOREACH item IN arr ... ENDFOR
```

### GOSUB
Jumps to a LABEL and returns when a RETURN statement is encountered within the subroutine.
```basil
GOSUB work
PRINTLN "done";
LABEL work
PRINTLN "working...";
RETURN
```

### GOTO
Transfers control unconditionally to a LABEL.
```basil
GOTO after
PRINTLN "skipped";
LABEL after
PRINTLN "continued";
```

### IF
Begins a conditional; supports single-statement form or block form with THEN BEGIN … [ELSE …] END.
```basil
IF X > 0 THEN BEGIN PRINTLN "positive"; END
```

### IN
Used with FOR EACH to specify the enumerable collection.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

### LABEL
Declares a jump target that can be used with GOTO or GOSUB.
```basil
LABEL again
PRINTLN "hi";
GOTO again
```

### NEXT
Closes a FOR or FOR EACH loop.
```basil
FOR I = 1 TO 2 PRINT I; NEXT
```

### STEP
Specifies the increment for a numeric FOR loop.
```basil
FOR I = 10 TO 0 STEP -2 PRINT I; NEXT
```

### THEN
Separates the IF condition from its consequent statement or BEGIN block.
```basil
IF X > 0 THEN PRINTLN "positive";
```

### TO
Specifies the upper bound expression in a numeric FOR loop.
```basil
FOR I = 1 TO 5 PRINT I; NEXT
```

### WHILE
Begins a while loop; body must be a BEGIN … END block.
```basil
WHILE I < 3 BEGIN
  PRINTLN I;
  LET I = I + 1;
END
```

## Logical Operators

### AND
Boolean conjunction with short-circuit evaluation.
```basil
IF A > 0 AND B > 0 THEN PRINTLN "both positive";
```

### NOT
Boolean negation with truthiness semantics.
```basil
IF NOT (A = B) THEN PRINTLN "different";
```

### OR
Boolean disjunction with short-circuit evaluation.
```basil
IF A = 0 OR B = 0 THEN PRINTLN "has zero";
```

## Arithmetic Operators


## Comparison Operators


## Data Types

### FALSE
Boolean literal representing false.
```basil
IF FALSE THEN PRINTLN "won't print";
```

### NULL
Null literal representing “no value”.
```basil
LET x = NULL;
```

### TRUE
Boolean literal representing true.
```basil
IF TRUE THEN PRINTLN "ok";
```

## Directives

### #BASIL_DEBUG
Reserved directive for debugging in CGI template prelude; currently parsed and retained for future use.
```basil
#BASIL_DEBUG
<?basil PRINT "debug on"; ?>
```

### #BASIL_DEV
Reserved directive for development-mode behavior in CGI template prelude; currently parsed and retained for future use.
```basil
#BASIL_DEV
<?basil PRINT "dev mode"; ?>
```

### #CGI_DEFAULT_HEADER
Sets an explicit default CGI header string to emit for the response when using CGI templates.
```basil
#CGI_DEFAULT_HEADER "Status: 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n"
<?basil PRINT "<h1>Hello</h1>"; ?>
```

### #CGI_NO_HEADER
Disables automatic CGI headers; your program must print full headers followed by a blank line.
```basil
#CGI_NO_HEADER
<?basil
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  PRINT "Hello";
?>
```

### #CGI_SHORT_TAGS_ON
Enables short template code tags <?bas ... ?> in CGI templates (in addition to <?basil ... ?>).
```basil
#CGI_SHORT_TAGS_ON
<?bas PRINT "ok"; ?>
```

### #USE
Declares opt-in modules/types for the program or template; used by tools/front-ends and ignored by the core lexer.
```basil
#USE BMX_RIDER, BMX_TEAM
PRINTLN "Modules hinted.";
```

## System Commands


## File I/O and Filesystem

### APPENDFILE
Appends string data to a file, creating it if needed.
```basil
APPENDFILE "out.txt", "Gamma\n";
```

### COPY
Copies a file from src$ to dst$; raises on error.
```basil
COPY "a.txt", "b.txt";
```

### DELETE
Deletes a file; raises on error.
```basil
DELETE "temp.bin";
```

### DIR$
Returns an array of file names (no paths) matching a glob pattern.
```basil
LET names$@ = DIR$("*.basil");
```

### FEOF
Returns TRUE if at end-of-file for the handle.
```basil
IF FEOF(fh%) THEN PRINTLN "eof";
```

### FFLUSH
Flushes buffered data to disk for the handle.
```basil
FFLUSH fh%;
```

### FOPEN
Opens a file with mode (e.g., "r", "w", "a", "rb", "w+"), returns integer handle.
```basil
LET fh% = FOPEN("notes.txt", "w");
```

### FREAD$
Reads up to N bytes/characters from the file.
```basil
LET s$ = FREAD$(fh%, 16);
```

### FREADLINE$
Reads a single line (without trailing newline).
```basil
LET line$ = FREADLINE$(fh%);
```

### FSEEK
Moves file pointer: FSEEK fh%, offset&, whence% (0=SET,1=CURRENT,2=END).
```basil
FSEEK fh%, 0, 0;
```

### FTELL&
Returns current byte offset as a LONG.
```basil
LET pos& = FTELL&(fh%);
```

### FWRITE
Writes a string to the file without newline.
```basil
FWRITE fh%, "Hello";
```

### FWRITELN
Writes a string followed by newline to the file.
```basil
FWRITELN fh%, "Hello";
```

### MOVE
Moves/renames a file to a new path (can cross directories).
```basil
MOVE "from.txt", "to_dir/to.txt";
```

### READFILE$
Reads an entire file into a string.
```basil
PRINT READFILE$("out.txt");
```

### RENAME
Renames a file within its directory.
```basil
RENAME "data.csv", "data_old.csv";
```

### WRITEFILE
Overwrites/creates a file with the given string data.
```basil
WRITEFILE "out.txt", "Alpha\n";
```


## Environment

### ENV$
Returns the value of an environment variable named by its string argument, or an empty string if it does not exist.
```basil
PRINTLN "PATH=", ENV$("PATH");
```

### SETENV
Sets an environment variable for the current Basil process. The right-hand side may be a quoted string, number, or any scalar variable.
```basil
SETENV DEMO_VAR = "42";
PRINTLN ENV$("DEMO_VAR");
```

### EXPORTENV
Like SETENV, but also attempts to export/persist the value outside the current process when supported by the platform (on Windows, via SETX). Always sets the process-local value.
```basil
EXPORTENV DEMO_EXPORT = "HELLO WORLD";
PRINTLN ENV$("DEMO_EXPORT");
```

### SHELL
Executes a command string in the parent command environment and waits for it to complete.
```basil
SHELL "cmd /C echo Hi > out.txt";
```

### EXIT
Exits the interpreter with an optional numeric exit code (defaults to 0).
```basil
EXIT 0;
```
