# Basil Keywords by Category

This document reorganizes the Basil language reference by category. Entries are grouped under their Type and listed alphabetically within each category. Descriptions and examples are taken verbatim from BASIL_REFERENCE.md.

Availability: Entries are part of the Core interpreter unless a Feature tag is shown (for example, "Feature: obj-term").

## Statements

### AS
Specifies a type name for DIM of object variables or arrays.
```basil
DIM r1@ AS BMX_RIDER
```

### DAW_RESET
Releases DAW resources (audio streams, MIDI connections, rings, WAV writers) for the current process.  
Feature: obj-daw
```basil
DAW_RESET
PRINT "DAW resources reset."
```

### DAW_STOP
Requests long‑running DAW helpers to stop; they return shortly after.  
Feature: obj-daw
```basil
DAW_STOP
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

### AUDIO_MONITOR%
Routes the first audio input device matching a substring to the first output device matching a substring until DAW_STOP() is called. Returns 0 on success.  
Feature: obj-daw
```basil
LET rc% = AUDIO_MONITOR%("scarlett", "scarlett")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
```

### AUDIO_PLAY%
Plays a WAV file to the first output device whose name contains the given substring (case-insensitive). Returns 0 on success.  
Feature: obj-daw
```basil
LET rc% = AUDIO_PLAY%("LC27T55 (NVIDIA High Definition Audio)", "alarm.wav")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
```

### AUDIO_RECORD%
Records audio from the first input device matching a substring to a WAV file for the given duration (seconds). Returns 0 on success.  
Feature: obj-daw
```basil
LET rc% = AUDIO_RECORD%("usb", "take1.wav", 10)
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
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

### DAW_ERR$
Returns the last error message set by a DAW helper (or "" if none).  
Feature: obj-daw
```basil
LET rc% = AUDIO_PLAY%("usb", "take1.wav")
IF rc% <> 0 THEN PRINTLN DAW_ERR$()
```

### DESCRIBE$
Returns a formatted description string of an object instance or array value.
```basil
PRINTLN DESCRIBE$(r1@);
```

### ESCAPE$
Escapes a string for safe inclusion in SQL string literals by doubling single quotes.
```basil
PRINTLN ESCAPE$("O'Reilly");
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

### MIDI_CAPTURE%
Captures incoming MIDI events from a selected input port and writes JSON Lines to a file until DAW_STOP() is called. Returns 0 on success.  
Feature: obj-daw
```basil
LET rc% = MIDI_CAPTURE%("LKMK3 MIDI", "midilog.jsonl")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
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

### SYNTH_LIVE%
Runs a simple live polyphonic synth driven by a MIDI input port and plays to the selected audio output device. Blocks until DAW_STOP() is called. Returns 0 on success.  
Feature: obj-daw
```basil
DAW_RESET
LET rc% = SYNTH_LIVE%("LKMK3 MIDI", "LC27T55 (NVIDIA High Definition Audio)", 8)
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
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

### UNESCAPE$
Reverses SQL string-escaping by collapsing doubled single quotes back to single quotes.
```basil
PRINTLN UNESCAPE$("O''Reilly");
```

### URLDECODE$
Decodes application/x-www-form-urlencoded text (e.g., from GET/POST): '+' becomes space and %HH bytes become UTF-8.
```basil
PRINTLN URLDECODE$("Bob+Smith%26Co");
```

### URLENCODE$
Encodes text for use as an HTTP GET parameter using application/x-www-form-urlencoded: spaces to '+', other bytes percent-encoded.
```basil
PRINTLN URLENCODE$("Bob Smith & Co");
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
Jumps to a LABEL and returns when a RETURN statement is encountered within the subroutine. Use `RETURN TO <label>` to resolve the GOSUB and continue at a label.
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

### RETURN
Resolves the most recent GOSUB frame; use `RETURN TO <label>` to resolve and continue at a label. Part of the Core interpreter.
```basil
GOSUB Work
PRINTLN "Back"
Work:
  RETURN
```
```basil
GOSUB Outer
Outer:
  GOSUB Inner
  PRINTLN "skip"
  RETURN
Inner:
  RETURN TO After
After:
  PRINTLN "after RETURN TO"
  RETURN
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

## Terminal (obj-term)

All of the following are part of the obj-term feature module.

### ALTSCREEN_OFF
*Feature:* obj-term  
Leaves the terminal's alternate screen buffer and returns to the main screen buffer.
```basil
TERM.INIT; ALTSCREEN_ON; PRINTLN "Alt"; ALTSCREEN_OFF; TERM.END;
```

### ALTSCREEN_ON
*Feature:* obj-term  
Enters the terminal's alternate screen buffer (a separate full-screen buffer).
```basil
TERM.INIT; ALTSCREEN_ON; PRINTLN "Hello (alt)"; TERM.FLUSH;
```

### ATTR
*Feature:* obj-term  
Sets text attributes: bold%, underline%, reverse% (each 0 or 1).
```basil
ATTR(1,0,0); PRINTLN "Bold"; ATTR_RESET;
```

### ATTR_RESET
*Feature:* obj-term  
Clears all text attributes to defaults.
```basil
ATTR_RESET;
```

### CLEAR
*Feature:* obj-term  
Clears the screen and moves the cursor to home. Alias of CLS and HOME.
```basil
CLEAR;
```

### CLS
*Feature:* obj-term  
Clears the screen and moves the cursor to home. Alias of CLEAR and HOME.
```basil
CLS;
```

### COLOR
*Feature:* obj-term  
Sets foreground/background colors by name or code (0..15), -1 to keep.
```basil
COLOR("yellow", -1);
```

### COLOR_RESET
*Feature:* obj-term  
Resets terminal colors to defaults.
```basil
COLOR_RESET;
```

### CURSOR_HIDE
*Feature:* obj-term  
Hides the text cursor.
```basil
CURSOR_HIDE;
```

### CURSOR_RESTORE
*Feature:* obj-term  
Restores the most recently saved cursor position; no-op if none.
```basil
CURSOR_RESTORE;
```

### CURSOR_SAVE
*Feature:* obj-term  
Saves the current cursor position (small stack maintained).
```basil
CURSOR_SAVE;
```

### CURSOR_SHOW
*Feature:* obj-term  
Shows the text cursor.
```basil
CURSOR_SHOW;
```

### HOME
*Feature:* obj-term  
Clears the screen and moves the cursor to home. Alias of CLEAR and CLS.
```basil
HOME;
```

### LOCATE
*Feature:* obj-term  
Moves the cursor to column x%, row y% (1-based), clamped to terminal size.
```basil
LOCATE(1,1);
```

### TERM.END
*Feature:* obj-term  
Restores console state (show cursor, raw off, leave alt-screen); idempotent.
```basil
TERM.END;
```

### TERM.FLUSH
*Feature:* obj-term  
Flushes any buffered terminal output.
```basil
PRINT "Ready"; TERM.FLUSH;
```

### TERM.INIT
*Feature:* obj-term  
Initializes terminal session state; idempotent.
```basil
TERM.INIT;
```

### TERM.POLLKEY$
*Feature:* obj-term  
Non-blocking key read. Returns "" if none; otherwise names like "Enter", "Esc", or "Char:a".
```basil
LET k$ = TERM.POLLKEY$(); IF k$ <> "" THEN PRINTLN k$;
```

### TERM.RAW
*Feature:* obj-term  
Enables/disables raw mode (TRUE/FALSE, 1/0, or "ON"/"OFF").
```basil
TERM.RAW(TRUE);  ' later…  TERM.RAW(FALSE);
```

### TERM_COLS%
*Feature:* obj-term  
Returns current terminal width (columns).
```basil
PRINTLN TERM_COLS%();
```

### TERM_ERR$
*Feature:* obj-term  
Returns and clears last terminal error string (or "").
```basil
LET e$ = TERM_ERR$(); IF e$ <> "" THEN PRINTLN e$;
```

### TERM_ROWS%
*Feature:* obj-term  
Returns current terminal height (rows).
```basil
PRINTLN TERM_ROWS%();
```


## AI (obj-ai)

All of the following are part of the obj-ai feature module.

### AI.CHAT$
*Feature:* obj-ai  
Sends a synchronous chat request and returns the response text.
```basil
PRINT AI.CHAT$("Explain bubble sort in 3 bullets");
```

### AI.EMBED
*Feature:* obj-ai  
Returns a 1-D float vector (embedding) for the given text.
```basil
LET vec = AI.EMBED("hello world");  ' vec is a numeric array of floats
```

### AI.LAST_ERROR$
*Feature:* obj-ai  
Returns the last AI error string (or "").
```basil
LET r$ = AI.CHAT$("Hi", "{ max_tokens:30 }");
IF r$ = "" THEN PRINTLN AI.LAST_ERROR$();
```

### AI.MODERATE%
*Feature:* obj-ai  
Moderation check: 0 = OK, 1 = flagged.
```basil
IF AI.MODERATE%("Write a polite meeting request") = 0 THEN
  PRINTLN AI.CHAT$("Write a 3-sentence meeting request.");
ELSE
  PRINTLN "Request blocked by moderation.";
END IF
```

### AI.STREAM
*Feature:* obj-ai  
Streams tokens to the console and returns the full concatenated text.
```basil
PRINT "AI says: ";
DIM full$ = AI.STREAM("Tell a one-liner about BASIC", "{ temperature:0.2 }");
PRINT "\n---\n"; PRINT full$;
```
