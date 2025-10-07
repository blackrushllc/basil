# Basil Language Reference

This document lists all Basil keywords, built-in functions, flow-control words, and supported directives in strict A–Z order. Each entry includes a short description and a minimal example.

Note: Items marked as reserved are recognized but not currently active syntax; they are included for forward compatibility.

## #BASIL_DEBUG
*Type:* Directive  
Reserved directive for debugging in CGI template prelude; currently parsed and retained for future use.
```basil
#BASIL_DEBUG
<?basil PRINT "debug on"; ?>
```

## #BASIL_DEV
*Type:* Directive  
Reserved directive for development-mode behavior in CGI template prelude; currently parsed and retained for future use.
```basil
#BASIL_DEV
<?basil PRINT "dev mode"; ?>
```

## #CGI_DEFAULT_HEADER
*Type:* Directive  
Sets an explicit default CGI header string to emit for the response when using CGI templates.
```basil
#CGI_DEFAULT_HEADER "Status: 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n"
<?basil PRINT "<h1>Hello</h1>"; ?>
```

## #CGI_NO_HEADER
*Type:* Directive  
Disables automatic CGI headers; your program must print full headers followed by a blank line.
```basil
#CGI_NO_HEADER
<?basil
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
  PRINT "Hello";
?>
```

## #CGI_SHORT_TAGS_ON
*Type:* Directive  
Enables short template code tags <?bas ... ?> in CGI templates (in addition to <?basil ... ?>).
```basil
#CGI_SHORT_TAGS_ON
<?bas PRINT "ok"; ?>
```

## #USE
*Type:* Directive  
Declares opt-in modules/types for the program or template; used by tools/front-ends and ignored by the core lexer.
```basil
#USE BMX_RIDER, BMX_TEAM
PRINTLN "Modules hinted.";
```

## AND
*Type:* Logical Operator  
Boolean conjunction with short-circuit evaluation.
```basil
IF A > 0 AND B > 0 THEN PRINTLN "both positive";
```

## AS
*Type:* Statement  
Specifies a type name for DIM of object variables or arrays.
```basil
DIM r1@ AS BMX_RIDER
```

## ASC%
*Type:* Function (returns Integer)  
Returns the ASCII/Unicode code point of the first character of a string, or 0 if empty.
```basil
LET code% = ASC%("A");
```

## AUTHOR
*Type:* Function (returns String)  
Constant function-like keyword that yields the Basil author name; accepts optional empty parentheses.
```basil
PRINTLN AUTHOR;
```

## BEGIN
*Type:* Flow Control  
Begins a block of statements to be terminated by END.
```basil
BEGIN
  PRINTLN 1;
  PRINTLN 2;
END
```

## BREAK
*Type:* Flow Control  
Exits the nearest enclosing loop.
```basil
FOR I = 1 TO 10 BEGIN IF I = 5 THEN BREAK; END NEXT
```

## CHR$
*Type:* Function (returns String)  
Returns a one-character string for the given numeric code point (out of range yields "").
```basil
PRINTLN CHR$(65);
```

## CLASS
*Type:* Function (returns Object)  
Constructs a class instance from a filename that defines a class.
```basil
LET x@ = CLASS("my_widget.cls");
```

## CONTINUE
*Type:* Flow Control  
Skips to the next iteration of the nearest enclosing loop.
```basil
FOR I = 1 TO 5 BEGIN IF I = 3 THEN CONTINUE; PRINT I; END NEXT
```

## DESCRIBE
*Type:* Statement  
Prints a formatted description of an object instance or an array value.
```basil
DESCRIBE r1@;
```

## DESCRIBE$
*Type:* Function (returns String)  
Returns a formatted description string of an object instance or array value.
```basil
PRINTLN DESCRIBE$(r1@);
```

## DIM
*Type:* Statement  
Declares a numeric array, object array, or object variable (with AS and optional constructor args).
```basil
DIM A(10, 20);
DIM riders@(10) AS BMX_RIDER;
DIM team@ AS BMX_TEAM("Rockets");
```

## DO
*Type:* Flow Control  
Reserved for future DO/LOOP constructs; not currently implemented.
```basil
REM Reserved: DO ... LOOP UNTIL cond
```

## EACH
*Type:* Flow Control  
Used with FOR to begin a FOR EACH iteration over an enumerable value.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

## ELSE
*Type:* Flow Control  
Introduces the alternative branch of an IF statement.
```basil
IF X > 0 THEN PRINTLN "pos"; ELSE PRINTLN "non-pos";
```

## ENDFOR
*Type:* Flow Control  
Reserved synonym for closing a FOREACH loop; current syntax uses NEXT.
```basil
REM Reserved: FOREACH x IN arr ... ENDFOR
```

## FALSE
*Type:* Data Type  
Boolean literal representing false.
```basil
IF FALSE THEN PRINTLN "won't print";
```

## FOR
*Type:* Flow Control  
Starts a numeric FOR…TO…[STEP]…NEXT loop or a FOR EACH…IN…NEXT enumeration loop.
```basil
FOR I = 1 TO 3 STEP 1 PRINT I; NEXT
```

## FOREACH
*Type:* Flow Control  
Reserved single-word form of FOR EACH; use "FOR EACH" in current Basil.
```basil
REM Reserved: FOREACH item IN arr ... ENDFOR
```

## FUNC
*Type:* Statement  
Declares a function with an optional BEGIN…END block or implicit block terminated by END [FUNC].
```basil
FUNC Add(a, b)
BEGIN
  RETURN a + b;
END
```

## GET$
*Type:* Function (returns String Array)  
Returns an array of GET query parameters (as strings) in CGI mode.
```basil
LET params$@ = GET$();
```

## GOSUB
*Type:* Flow Control  
Jumps to a LABEL and returns when a RETURN statement is encountered within the subroutine.
```basil
GOSUB work
PRINTLN "done";
LABEL work
PRINTLN "working...";
RETURN
```

## GOTO
*Type:* Flow Control  
Transfers control unconditionally to a LABEL.
```basil
GOTO after
PRINTLN "skipped";
LABEL after
PRINTLN "continued";
```

## HTML
*Type:* Function (returns String)  
Escapes special HTML characters of its argument; alias of HTML$.
```basil
PRINTLN HTML("<b>& ok</b>");
```

## HTML$
*Type:* Function (returns String)  
Escapes special HTML characters of its argument.
```basil
PRINTLN HTML$("<b>& ok</b>");
```

## IF
*Type:* Flow Control  
Begins a conditional; supports single-statement form or block form with THEN BEGIN … [ELSE …] END.
```basil
IF X > 0 THEN BEGIN PRINTLN "positive"; END
```

## IN
*Type:* Flow Control  
Used with FOR EACH to specify the enumerable collection.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

## INKEY%
*Type:* Function (returns Integer)  
Non-blocking key read; returns key code (0 if no key available).
```basil
LET k% = INKEY%();
```

## INKEY$
*Type:* Function (returns String)  
Non-blocking key read; returns one-character string ("" if no key available).
```basil
LET k$ = INKEY$();
```

## INPUT
*Type:* Function (returns String)  
Alias of INPUT$; reads a line from standard input without trailing CR/LF.
```basil
LET name$ = INPUT("Enter your name: ");
```

## INPUT$
*Type:* Function (returns String)  
Reads a line from standard input without trailing CR/LF (optionally prints a prompt first).
```basil
LET name$ = INPUT$("Enter your name: ");
```

## INPUTC$
*Type:* Function (returns String)  
Reads a single ASCII character (echoed once) from input; returns "" for non-ASCII or Enter.
```basil
LET ch$ = INPUTC$("Press a key: ");
```

## INSTR
*Type:* Function (returns Integer)  
Finds the position (0-based) of a substring within a string starting at an optional index (0 if not found).
```basil
LET p% = INSTR("banana", "na", 2);
```

## LABEL
*Type:* Flow Control  
Declares a jump target that can be used with GOTO or GOSUB.
```basil
LABEL again
PRINTLN "hi";
GOTO again
```

## LCASE$
*Type:* Function (returns String)  
Returns the lowercase version of a string.
```basil
PRINTLN LCASE$("MiXeD");
```

## LEFT$
*Type:* Function (returns String)  
Returns the leftmost N characters of a string.
```basil
PRINTLN LEFT$("basil", 2);
```

## LEN
*Type:* Function (returns Integer)  
Returns string character length or total element count of an array; other values are converted to strings.
```basil
PRINTLN LEN("hello");
```

## LET
*Type:* Statement  
Assigns a value to a variable, array element, or object property (property assignment may also omit LET).
```basil
LET A = 42;  LET arr(1,2) = 7;  obj.Prop = 10;
```

## MID$
*Type:* Function (returns String)  
Returns a substring starting at 1-based index, with optional length.
```basil
PRINTLN MID$("banana", 2, 3);
```

## NEW
*Type:* Function (returns Object)  
Constructs a new object instance of a registered type with constructor arguments.
```basil
LET r@ = NEW BMX_RIDER("Alex", 12, 5);
```

## NEXT
*Type:* Flow Control  
Closes a FOR or FOR EACH loop.
```basil
FOR I = 1 TO 2 PRINT I; NEXT
```

## NOT
*Type:* Logical Operator  
Boolean negation with truthiness semantics.
```basil
IF NOT (A = B) THEN PRINTLN "different";
```

## NULL
*Type:* Data Type  
Null literal representing “no value”.
```basil
LET x = NULL;
```

## OR
*Type:* Logical Operator  
Boolean disjunction with short-circuit evaluation.
```basil
IF A = 0 OR B = 0 THEN PRINTLN "has zero";
```

## POST$
*Type:* Function (returns String Array)  
Returns an array of POST body parameters (as strings) in CGI mode.
```basil
LET form$@ = POST$();
```

## PRINT
*Type:* Statement  
Prints an expression (or expressions separated by commas, which insert TABs) without a trailing newline.
```basil
PRINT "Hello, "; PRINT "world!";
```

## PRINTLN
*Type:* Statement  
Prints an expression followed by a newline.
```basil
PRINTLN "Hello";
```

## REQUEST$
*Type:* Function (returns String Array)  
Returns GET and POST parameters combined (as strings) in CGI mode.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

## RETURN
*Type:* Statement  
Returns from a function, optionally with a value.
```basil
RETURN 42;
```

## RIGHT$
*Type:* Function (returns String)  
Returns the rightmost N characters of a string.
```basil
PRINTLN RIGHT$("basil", 3);
```

## STEP
*Type:* Flow Control  
Specifies the increment for a numeric FOR loop.
```basil
FOR I = 10 TO 0 STEP -2 PRINT I; NEXT
```

## THEN
*Type:* Flow Control  
Separates the IF condition from its consequent statement or BEGIN block.
```basil
IF X > 0 THEN PRINTLN "positive";
```

## TO
*Type:* Flow Control  
Specifies the upper bound expression in a numeric FOR loop.
```basil
FOR I = 1 TO 5 PRINT I; NEXT
```

## TRIM$
*Type:* Function (returns String)  
Returns the input string with leading and trailing whitespace removed.
```basil
PRINTLN TRIM$("  hi  ");
```

## TRUE
*Type:* Data Type  
Boolean literal representing true.
```basil
IF TRUE THEN PRINTLN "ok";
```

## TYPE$
*Type:* Function (returns String)  
Returns a string that names the Basil type of its argument (e.g., "String", "Int", "Array", "Object").
```basil
PRINTLN TYPE$(42);
```

## UCASE$
*Type:* Function (returns String)  
Returns the uppercase version of a string.
```basil
PRINTLN UCASE$("basil");
```

## WHILE
*Type:* Flow Control  
Begins a while loop; body must be a BEGIN … END block.
```basil
WHILE I < 3 BEGIN
  PRINTLN I;
  LET I = I + 1;
END
```
