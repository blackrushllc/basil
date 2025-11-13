
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


## Exceptions

### TRY / CATCH / FINALLY
Structured exception handling. TRY begins a protected region, CATCH handles an exception (optionally binding a string variable like err$), and FINALLY always runs on exit from the TRY.
```basil
TRY
  IF x% = 0 THEN RAISE "Divide by zero"
CATCH err$
  PRINT "Caught: ", err$
FINALLY
  PRINT "Cleanup"
END TRY
```

### RAISE
Throws a user exception with an optional message expression converted to String. Bare `RAISE` is only valid inside CATCH (rethrow).
```basil
IF A = 0 THEN RAISE "A must be non-zero"
```

