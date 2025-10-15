# Basil Language Reference

This document lists all Basil keywords, built-in functions, flow-control words, and supported directives in strict A–Z order. Each entry includes a short description and a minimal example.

Availability: Entries are part of the Core interpreter unless a Feature tag is shown (for example, "Feature: obj-term").

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

## AI.CHAT$
*Type:* Function (returns String)  
*Feature:* obj-ai  
Sends a synchronous chat request to the configured AI model and returns the response text. In test mode, returns a deterministic string like "[[TEST]] abcd1234".
```basil
PRINT AI.CHAT$("Explain bubble sort in 3 bullets");
```

## AI.EMBED
*Type:* Function (returns Float[])  
*Feature:* obj-ai  
Computes a vector embedding for the given text. In test mode, returns a deterministic length-16 vector.
```basil
' Simple usage: obtain an embedding vector
LET vec = AI.EMBED("hello world");  ' vec is a numeric array of floats
```

## AI.LAST_ERROR$
*Type:* Function (returns String)  
*Feature:* obj-ai  
Returns the last error message from an AI operation (or "" if none). Cleared at the start of each AI call.
```basil
LET reply$ = AI.CHAT$("Hi", "{ max_tokens: 30 }");
IF reply$ = "" THEN PRINTLN AI.LAST_ERROR$();
```

## AI.MODERATE%
*Type:* Function (returns Integer)  
*Feature:* obj-ai  
Runs a simple moderation check on the input. Returns 0 if OK, 1 if flagged. In test mode, inputs containing "FLAG_ME" are flagged.
```basil
IF AI.MODERATE%("Write a polite meeting request") = 0 THEN
  PRINTLN AI.CHAT$("Write a 3-sentence meeting request.");
ELSE
  PRINTLN "Request blocked by moderation.";
END IF
```

## AI.STREAM
*Type:* Function (returns String)  
*Feature:* obj-ai  
Streams tokens to the console as they arrive and returns the full text at the end. In test mode, prints the deterministic reply in 3 chunks.
```basil
PRINT "AI says: ";
DIM full$ = AI.STREAM("Tell a one-liner about BASIC", "{ temperature:0.2 }");
PRINT "\n---\n"; PRINT full$;
```

## AND
*Type:* Logical Operator  
Boolean conjunction with short-circuit evaluation.
```basil
IF A > 0 AND B > 0 THEN PRINTLN "both positive";
```

## APPENDFILE
*Type:* Statement  
Appends string data to an existing file or creates a new one.
```basil
APPENDFILE "out.txt", "Gamma\n";
```

## AS
*Type:* Statement  
Specifies a type name for DIM of object variables or arrays.
```basil
DIM r1@ AS BMX_RIDER
```

## ALTSCREEN_OFF
*Type:* Statement  
*Feature:* obj-term  
Leaves the terminal's alternate screen buffer and returns to the main screen buffer. Safe to call multiple times.
```basil
TERM.INIT; ALTSCREEN_ON; PRINTLN "Alt screen"; ALTSCREEN_OFF; TERM.END;
```

## ALTSCREEN_ON
*Type:* Statement  
*Feature:* obj-term  
Enters the terminal's alternate screen buffer (typically a blank screen separate from the main buffer).
```basil
TERM.INIT; ALTSCREEN_ON; PRINTLN "Hello (alt)"; TERM.FLUSH;
```

## ASC%
*Type:* Function (returns Integer)  
Returns the ASCII/Unicode code point of the first character of a string, or 0 if empty.
```basil
LET code% = ASC%("A");
```

## ATTR
*Type:* Statement  
*Feature:* obj-term  
Sets text attributes: bold%, underline%, reverse% (each 0=OFF, 1=ON). Use ATTR_RESET to clear.
```basil
ATTR(1,0,0); PRINTLN "Bold"; ATTR_RESET;
```

## ATTR_RESET
*Type:* Statement  
*Feature:* obj-term  
Clears all text attributes (bold, underline, reverse) to defaults.
```basil
ATTR_RESET;
```

## AUTHOR
*Type:* Function (returns String)  
Constant function-like keyword that yields the Basil author name; accepts optional empty parentheses.
```basil
PRINTLN AUTHOR;
```

## AUDIO_MONITOR%
*Type:* Function (returns Integer)  
*Feature:* obj-daw  
Routes the first audio input device matching a substring to the first output device matching a substring. Blocks until DAW_STOP() is called. Returns 0 on success.
```basil
LET rc% = AUDIO_MONITOR%("scarlett", "scarlett")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
```

## AUDIO_PLAY%
*Type:* Function (returns Integer)  
*Feature:* obj-daw  
Plays a WAV file to the first output device whose name contains the given substring (case-insensitive). Returns 0 on success.
```basil
LET rc% = AUDIO_PLAY%("LC27T55 (NVIDIA High Definition Audio)", "alarm.wav")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
```

## AUDIO_RECORD%
*Type:* Function (returns Integer)  
*Feature:* obj-daw  
Records audio from the first input device matching a substring to a WAV file for the given duration (seconds). Returns 0 on success.
```basil
LET rc% = AUDIO_RECORD%("usb", "take1.wav", 10)
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
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

## CLEAR
*Type:* Statement  
*Feature:* obj-term  
Clears the screen and moves the cursor to the home position (0,0). Alias of CLS and HOME.
```basil
CLEAR;
```

## CLS
*Type:* Statement  
*Feature:* obj-term  
Clears the screen and moves the cursor to the home position (0,0). Alias of CLEAR and HOME.
```basil
CLS;
```

## COLOR
*Type:* Statement  
*Feature:* obj-term  
Sets foreground and/or background colors. Accepts numeric codes 0..15, -1 to keep unchanged, or color names like "yellow".
```basil
COLOR("yellow", -1);
```

## COLOR_RESET
*Type:* Statement  
*Feature:* obj-term  
Resets the terminal colors to defaults.
```basil
COLOR_RESET;
```

## COPY
*Type:* Statement  
Copies a file from src$ to dst$; raises a runtime error on failure.
```basil
COPY "a.txt", "b.txt";
```

## CONTINUE
*Type:* Flow Control  
Skips to the next iteration of the nearest enclosing loop.
```basil
FOR I = 1 TO 5 BEGIN IF I = 3 THEN CONTINUE; PRINT I; END NEXT
```

## CURSOR_HIDE
*Type:* Statement  
*Feature:* obj-term  
Hides the text cursor.
```basil
CURSOR_HIDE;
```

## CURSOR_RESTORE
*Type:* Statement  
*Feature:* obj-term  
Restores the most recently saved cursor position; no-op if none saved.
```basil
CURSOR_RESTORE;
```

## CURSOR_SAVE
*Type:* Statement  
*Feature:* obj-term  
Saves the current cursor position (a small stack of positions is kept).
```basil
CURSOR_SAVE;
```

## CURSOR_SHOW
*Type:* Statement  
*Feature:* obj-term  
Shows the text cursor.
```basil
CURSOR_SHOW;
```

## DAW_ERR$
*Type:* Function (returns String)  
*Feature:* obj-daw  
Returns the last error message set by a DAW helper on this thread (or "" if none).
```basil
LET rc% = AUDIO_PLAY%("usb", "take1.wav")
IF rc% <> 0 THEN PRINTLN DAW_ERR$()
```

## DAW_RESET
*Type:* Statement  
*Feature:* obj-daw  
Releases DAW resources (audio streams, MIDI connections, rings, WAV writers) held by the current process. Idempotent and safe to call.
```basil
DAW_RESET
PRINT "DAW resources reset."
```

## DAW_STOP
*Type:* Statement  
*Feature:* obj-daw  
Requests long‑running DAW helpers to stop; they typically return within ~50–250ms.
```basil
REM In one console, start a helper:
REM   LET rc% = AUDIO_MONITOR%("usb", "usb")
REM In another console/script, signal stop:
DAW_STOP
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

## DELETE
*Type:* Statement  
Deletes a file; raises a runtime error on failure.
```basil
DELETE "temp.bin";
```

## DIR$
*Type:* Function (returns String Array)  
Returns file names (no paths) that match a glob pattern in a directory.
```basil
LET names$@ = DIR$("examples/*.basil");
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

## ENV$
*Type:* Function (returns String)  
Returns the value of an environment variable named by its string argument, or an empty string if it does not exist.
```basil
PRINTLN "PATH=", ENV$("PATH");
```

## LOADENV%
*Type:* Function (returns Integer)
Loads environment variables from a text file containing newline-separated `name=value` pairs. Lines starting with `#` or `;` are treated as comments; blank lines are ignored. Values are set for the current Basil process (so they are visible to ENV$ and to child processes you spawn).

- Parameters: optional `filename$` (String). If omitted or blank, defaults to `.env` in the current directory.
- Returns: `1` (TRUE) on success (file read and processed), `0` (FALSE) on error reading the file. Malformed lines cause warnings but do not make the call fail.
- Notes: Surrounding single or double quotes around values are removed when present.
```basil
' Load from default .env
IF LOADENV%() THEN PRINTLN "Loaded .env"; ELSE PRINTLN "No .env";

' Load from a specific file
IF LOADENV%("config.env") THEN PRINTLN "Loaded config.env";
PRINTLN "API_KEY=", ENV$("API_KEY");
```

## ESCAPE$
*Type:* Function (returns String)  
Escapes a string for safe inclusion in SQL string literals by doubling single quotes.
```basil
PRINTLN ESCAPE$("O'Reilly");  ' prints: O''Reilly
```

## EXIT
*Type:* Statement  
Exits the interpreter with an optional numeric exit code (defaults to 0).
```basil
EXIT 0;
```

## EXPORTENV
*Type:* Statement  
Sets an environment variable like SETENV and also attempts to export/persist it for future processes when supported (Windows via SETX). Always sets the process-local value.
```basil
EXPORTENV DEMO_EXPORT = "HELLO WORLD";
```

## FALSE
*Type:* Data Type  
Boolean literal representing false.
```basil
IF FALSE THEN PRINTLN "won't print";
```

## FEOF
*Type:* Function (returns Bool)  
Returns TRUE if the file handle is at end-of-file.
```basil
IF FEOF(fh%) THEN PRINTLN "eof";
```

## FFLUSH
*Type:* Function (returns Bool)  
Flushes buffered data to disk for the given file handle.
```basil
FFLUSH fh%;
```

## FOPEN
*Type:* Function (returns Integer handle)  
Opens a file and returns a handle (>=1) or raises on error. Modes: r, w, a, rb, wb, ab, r+, w+, a+, rb+, wb+, ab+.
```basil
LET fh% = FOPEN("notes.txt", "w");
```

## FREAD$
*Type:* Function (returns String)  
Reads up to N bytes/characters from a file.
```basil
LET s$ = FREAD$(fh%, 16);
```

## FREADLINE$
*Type:* Function (returns String)  
Reads a single line (without trailing newline) from a file.
```basil
LET line$ = FREADLINE$(fh%);
```

## FSEEK
*Type:* Function (returns Bool)  
Moves the file position: FSEEK fh%, offset&, whence% (0=SET,1=CURRENT,2=END).
```basil
FSEEK fh%, 0, 0;  ' rewind
```

## FTELL&
*Type:* Function (returns Long)  
Returns the current byte offset for the file handle.
```basil
LET pos& = FTELL&(fh%);
```

## FWRITE
*Type:* Function (returns Bool)  
Writes a string to a file without a newline.
```basil
FWRITE fh%, "Hello";
```

## FWRITELN
*Type:* Function (returns Bool)  
Writes a string followed by a newline to a file.
```basil
FWRITELN fh%, "Hello";
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
*Availability:* Core
Transfers control to a subroutine at a LABEL and returns when a matching GOSUB return is executed. Nestable; uses a dedicated GOSUB return stack.

Syntax:
- GOSUB <label>;
- RETURN;
- RETURN TO <label>;

Notes:
- Labels can be written either as `LabelName:` or `LABEL LabelName` on their own line.
- `RETURN` without a prior `GOSUB` is a runtime error.
- The GOSUB stack is capped at 4096 nested calls (configurable); exceeding it is a runtime error.
- If the program terminates with pending GOSUB frames, a warning is printed.

Example:
```basil
PRINTLN "Start";
GOSUB Work;
PRINTLN "Back";
GOTO Done;

Work:
  PRINTLN "In Work";
  RETURN;

Done:
PRINTLN "End";
```

Return-then-continue example:
```basil
Outer:
  GOSUB Inner;
  PRINTLN "This will be skipped";
  RETURN;

Inner:
  PRINTLN "Inner...";
  RETURN TO After;

After:
  PRINTLN "After label reached via RETURN TO";
  RETURN;
```

## GOTO
*Type:* Flow Control  
*Availability:* Core
Transfers control unconditionally to a LABEL (labels can be written as `Name:` or `LABEL Name`).
```basil
GOTO After
PRINTLN "skipped";
After:
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

## HOME
*Type:* Statement  
*Feature:* obj-term  
Clears the screen and moves the cursor to the home position (0,0). Alias of CLEAR and CLS.
```basil
HOME;
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

## LOCATE
*Type:* Statement  
*Feature:* obj-term  
Moves the cursor to column x% and row y% (1-based), clamped to the terminal size.
```basil
LOCATE(1, 1);
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

## MIDI_CAPTURE%
*Type:* Function (returns Integer)  
*Feature:* obj-daw  
Captures incoming MIDI events from the first input port matching a substring and appends JSON Lines to the given file until DAW_STOP() is called. Returns 0 on success.
```basil
LET rc% = MIDI_CAPTURE%("LKMK3 MIDI", "midilog.jsonl")
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$()
```

## MOVE
*Type:* Statement  
Moves/renames a file to a new path (can cross directories).
```basil
MOVE "from.txt", "to_dir/to.txt";
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

## READFILE$
*Type:* Function (returns String)  
Reads an entire file into a string.
```basil
PRINT READFILE$("out.txt");
```

## RENAME
*Type:* Statement  
Renames a file within its directory.
```basil
RENAME "data.csv", "data_old.csv";
```

## REQUEST$
*Type:* Function (returns String Array)  
Returns GET and POST parameters combined (as strings) in CGI mode.
```basil
FOR EACH p$ IN REQUEST$() PRINTLN p$; NEXT
```

## RETURN
*Type:* Statement  
*Availability:* Core
Two forms:
- Function return: `RETURN [expr];` — returns from a function with an optional value.
- GOSUB return: `RETURN;` — returns to the most recent `GOSUB`. Use `RETURN TO <label>;` to resolve the GOSUB and continue at a label (like `RETURN` followed by `GOTO`).

Examples:
```basil
FUNC Add(a, b)
BEGIN
  RETURN a + b;
END
```

```basil
GOSUB Work;
PRINTLN "Back";
Work:
  RETURN;
```

```basil
GOSUB A;
A:
  RETURN TO Done;
Done:
PRINTLN "after RETURN TO";
```

## RIGHT$
*Type:* Function (returns String)  
Returns the rightmost N characters of a string.
```basil
PRINTLN RIGHT$("basil", 3);
```

## SETENV
*Type:* Statement  
Sets an environment variable for the current Basil process. Syntax: SETENV NAME = value; the value may be a quoted string, number, or any scalar variable.
```basil
SETENV DEMO_VAR = "42";
```

## SHELL
*Type:* Statement  
Executes a command in the parent command environment and waits for it to complete.
```basil
SHELL "cmd /C dir > temp.txt";
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

## TERM.COLs%

## TERM_COLS%
*Type:* Function (returns Integer)  
*Feature:* obj-term  
Returns the current terminal width (columns).
```basil
PRINTLN TERM_COLS%();
```

## TERM.END
*Type:* Statement  
*Feature:* obj-term  
Restores the console to a sane state (show cursor, disable raw mode, leave alt-screen). Safe to call multiple times.
```basil
TERM.END;
```

## TERM_ERR$
*Type:* Function (returns String)  
*Feature:* obj-term  
Returns and clears the last terminal-error message (or "" if none).
```basil
LET err$ = TERM_ERR$(); IF err$ <> "" THEN PRINTLN err$;
```

## TERM.FLUSH
*Type:* Statement  
*Feature:* obj-term  
Flushes any buffered terminal output to reduce flicker during redraws.
```basil
PRINT "Ready"; TERM.FLUSH;
```

## TERM.INIT
*Type:* Statement  
*Feature:* obj-term  
Initializes terminal session state; idempotent and safe to call more than once.
```basil
TERM.INIT;
```

## TERM.POLLKEY$
*Type:* Function (returns String)  
*Feature:* obj-term  
Non-blocking key read. Returns "" if no key is available; otherwise returns normalized names like "Enter", "Esc", or "Char:a".
```basil
LET k$ = TERM.POLLKEY$(); IF k$ <> "" THEN PRINTLN k$;
```

## TERM.RAW
*Type:* Statement  
*Feature:* obj-term  
Enables or disables raw mode (no line buffering). Accepts TRUE/FALSE, 1/0, or "ON"/"OFF".
```basil
TERM.RAW(TRUE);  ' later…  TERM.RAW(FALSE);
```

## TERM_ROWS%
*Type:* Function (returns Integer)  
*Feature:* obj-term  
Returns the current terminal height (rows).
```basil
PRINTLN TERM_ROWS%();
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

## UNESCAPE$
*Type:* Function (returns String)  
Reverses SQL string-escaping by collapsing doubled single quotes back to single quotes.
```basil
PRINTLN UNESCAPE$("O''Reilly");  ' prints: O'Reilly
```

## URLDECODE$
*Type:* Function (returns String)  
Decodes application/x-www-form-urlencoded text (e.g., from GET/POST): '+' becomes space and %HH bytes become UTF-8.
```basil
PRINTLN URLDECODE$("Bob+Smith%26Co");  ' prints: Bob Smith&Co
```

## URLENCODE$
*Type:* Function (returns String)  
Encodes text for use as an HTTP GET parameter using application/x-www-form-urlencoded: spaces to '+', other bytes percent-encoded.
```basil
PRINTLN URLENCODE$("Bob Smith & Co");  ' prints: Bob+Smith+%26+Co
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
## WRITEFILE
*Type:* Statement  
Overwrites/creates a file with the given string data.
```basil
WRITEFILE "out.txt", "Alpha\n";
```


## CATCH
Type: Flow Control
Introduces an exception handler for a preceding TRY block. Optionally binds the exception message to a string variable (must end with '$').
```basil
TRY
  RAISE "boom"
CATCH err$
  PRINT "Caught: ", err$
END TRY
```

## FINALLY
Type: Flow Control
Cleanup block that always runs when leaving a TRY, regardless of success or exception.
```basil
TRY
  PRINT "Work"
FINALLY
  PRINT "Cleanup"
END TRY
```

## RAISE
Type: Statement
Throws a user exception with an optional message expression converted to String. A bare `RAISE` (no expression) is only valid inside CATCH and rethrows the current exception.
```basil
IF x% = 0 THEN RAISE "Divide by zero"
TRY
  RAISE "first"
CATCH e$
  RAISE   ' rethrow
END TRY
```

## TRY
Type: Flow Control
Begins a protected region optionally followed by CATCH and/or FINALLY, terminated by END TRY.
```basil
TRY
  IF x% = 0 THEN RAISE "Divide by zero"
  PRINT 10 / x%
CATCH err$
  PRINT "Oops: ", err$
FINALLY
  PRINT "Always runs"
END TRY
```
