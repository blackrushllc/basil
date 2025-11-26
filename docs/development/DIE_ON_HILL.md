You are working in a Rust-based BASIC interpreter project.
There are **two sibling projects** that are almost identical:

* **Basil** (full-featured): interpreter + compiler + web server.
* **Basic** (stripped-down “naked” BASIC): interpreter only.

This prompt is designed to work in either repo.
When I run it in one repo, treat *that* repo as the active project and ignore the other.

You are working on our BASIC dialect and its Rust-based cousin Basil.

## High-level goals

We want to:

(Exploratory) Investigate making `BEGIN` **optional** for multi-line blocks such as `IF ... THEN` / `END IF`.

Please keep these changes consistent between the “classic” BASIC interpreter and the Rust-based Basil interpreter.

---


## 4. (Exploratory) BEGIN-less blocks for multi-line IF and other constructs

Current requirement (simplified):

```basic
IF targetDir$ = "" THEN BEGIN
    PRINT "No target directory specified. Aborting."
    EXIT SUB
END IF
```

or:

```basic
IF targetDir$ = "" THEN
    BEGIN
        PRINT "No target directory specified. Aborting."
        RETURN
END IF
```

We would *like* to support a more classic BASIC style:

```basic
IF targetDir$ = "" THEN
    PRINT "No target directory specified. Aborting."
    RETURN
END IF
```

**Important**: This is **exploratory**. If it turns out to be too tightly coupled to the existing block parser (where `BEGIN`/`END` act like `{}`), then:

* Do not ship a half-working version.
* Prefer to leave:

    * Clear TODO comments,
    * A small design note describing what would need to change, and
    * Possibly a feature flag or parser hook that we can revisit later.

### Desired behavior (if feasible)

* Still support the existing `BEGIN`/`END` style for backwards compatibility.

* Additionally support “implicit blocks” for constructs like:

  ```basic
  IF cond THEN
      ' body...
  END IF

  WHILE cond
      ' body...
  END WHILE

  SUB Foo()
      ' body...
  END SUB
  ```

* In other words, treat:

  ```basic
  IF cond THEN BEGIN
      ...
  END IF
  ```

  and

  ```basic
  IF cond THEN
      ...
  END IF
  ```

  as equivalent.


* FOR/NEXT loops also require special handling since they require `END` before `NEXT` when `BEGIN` is used:

* Existing:

```basic
FOR i = 1 TO 5
    PRINTLN i;
NEXT i;

FOR j = 5 TO 1 STEP -1
    BEGIN
        PRINT j;
        FOR i = 1 TO 5
            PRINTLN i;
        NEXT i;
    END
NEXT j;

```

* Updated (BEGIN optional):

```basic
FOR i = 1 TO 5
    PRINTLN i;
NEXT i;

FOR j = 5 TO 1 STEP -1
    PRINT j;
    FOR i = 1 TO 5
        PRINTLN i;
    NEXT i;
NEXT j;
```


### Implementation sketch

We ONLY want to do this if the parser structure allows it without massive surgery.

General idea:

* Wherever we parse a **block** now, we currently expect something like `BEGIN ... END` (or the internal equivalents).
* Enhance the block parser so that:

    * It recognizes the existing `BEGIN ... END` braced form, **and also**
    * Recognizes an implicit “block until matching END tag” form when `BEGIN` is omitted.
* For IF specifically:

    * After parsing `IF <expr> THEN`:

        * If the next token is `BEGIN`, parse a braced block as we do now.
        * Otherwise, parse a block that continues until `END IF` (handling `ELSE`/`ELSEIF` as needed, if we support them).

If this requires making the parser newline-sensitive in ways that conflict with the rest of the language, or if it risks breaking existing behavior, then:

1. Document what you found in a short design note / comment block.
2. Leave the current `BEGIN` requirement in place.
3. Leave the current {...} braced block requirement (alternate syntax) in place.
4. Advise on whether or not this change is feasible without major surgery, or whether it should be revisited later with a more extensive parser overhaul.

