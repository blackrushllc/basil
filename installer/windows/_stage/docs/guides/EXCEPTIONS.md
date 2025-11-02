# Exceptions in Basil: TRY, CATCH, FINALLY, and RAISE

This guide describes the new structured exception handling features in Basil.

Keywords: TRY, CATCH, FINALLY, RAISE

Status: Core (no feature flag). Keywords are case‑insensitive.


## Overview

Basil supports BASIC‑flavored structured exceptions with the following constructs:

- TRY … CATCH … FINALLY … END TRY
- TRY … CATCH … END TRY
- TRY … FINALLY … END TRY
- Optional CATCH variable (must be a string variable with a `$` suffix)
- RAISE [expr] to throw a user exception, or rethrow inside CATCH with `RAISE` alone

FINALLY always runs, whether the TRY body completes normally, throws a user exception, or a runtime error occurs. If an exception isn’t caught, pending FINALLY blocks are still run on the unwind path before the interpreter terminates with the error.


## Syntax

```
TRY
  ' protected code
  IF x% = 0 THEN RAISE "Divide by zero"
  PRINT 10 / x%
CATCH err$
  PRINT "Oops: ", err$
FINALLY
  PRINT "Always runs"
END TRY
```

Variants:

- `TRY … CATCH … END TRY` (no FINALLY)
- `TRY … FINALLY … END TRY` (no CATCH)
- `TRY … CATCH e$ … FINALLY … END TRY`
- `RAISE "message"` anywhere
- `RAISE` (bare) only inside a CATCH block to rethrow the current exception


## Semantics

- Entering TRY sets up an exception region. If the protected code completes normally, FINALLY (if present) executes and control continues after END TRY.
- If a RAISE occurs (or a runtime error happens) inside the TRY:
  - FINALLY (if present) runs.
  - If CATCH exists, control transfers to it. The optional CATCH variable (must end with `$`) receives the exception message string. On normal exit from CATCH, control continues after END TRY.
  - If there is no CATCH, the exception propagates outward after running FINALLY.
- Nested TRYs: the innermost applicable handler runs first.
- Rethrow: `RAISE` with no expression inside CATCH rethrows the current exception.
- Uncaught: if no handler catches the exception, all pending FINALLY blocks on the stack are executed and Basil terminates with its standard fatal error printout.


## Types and values

- `RAISE <expr>` converts its value to String using the same rules as PRINT concatenation. That string becomes the exception message.
- The CATCH variable, if present, must be a string‐typed variable name (with `$` suffix).


## Errors (exact messages)

- Missing END TRY:
  `Expected 'END TRY' to terminate TRY block.`
- Missing both CATCH and FINALLY:
  `TRY must contain a CATCH or FINALLY block.`
- Multiple CATCH blocks:
  `Only one CATCH block is allowed per TRY.`
- Multiple FINALLY blocks:
  `Only one FINALLY block is allowed per TRY.`
- CATCH variable not a string:
  `CATCH variable must be a string (use '$' suffix).`
- Bare rethrow outside CATCH:
  `RAISE without an expression is only valid inside CATCH.`


## Examples

Divide by zero guard:

```basil
DIM x% = 0
TRY
  IF x% = 0 THEN RAISE "Divide by zero"
  PRINT 10 / x%
CATCH err$
  PRINT "Caught: ", err$
FINALLY
  PRINT "Finally!"
END TRY
```

Catching a runtime error:

```basil
DIM A%(2)
TRY
  LET A%(3) = 99  ' out of range at runtime
CATCH e$
  PRINT "Caught runtime error: ", e$
END TRY
```

Rethrow:

```basil
TRY
  RAISE "first"
CATCH e$
  PRINT "Handling: ", e$
  RAISE  ' rethrow
FINALLY
  PRINT "Always runs"
END TRY
```

Finally‑only:

```basil
TRY
  PRINT "Work…"
FINALLY
  PRINT "Always runs even without catch"
END TRY
```


## Notes

- Runtime errors originating in Basil code (e.g., array out‑of‑bounds) are surfaced as exceptions that can be caught by TRY/CATCH.
- Keep FINALLY blocks side‑effect‑safe and short; they should be for cleanup (closing files, resetting state) rather than normal control flow.
- The new keywords are case‑insensitive (TRY, Try, try all work).
