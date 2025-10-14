## Ready-to-use specs for common features

If helpful, you can copy one of these into the template above and run with it:

### A) String interpolation (lightweight, no new ops)

* **Syntax:** `"Hello #{name$}, wins=#{r@.Wins%}!"`
* **Semantics:** desugar to concatenation at compile time; expressions inside `#{…}` are full Basil expressions.
* **Tokens:** `#` `{` `}` (reuse braces if already present).
* **Errors:** unterminated interpolation; disallow multi-line without `""` continuation rules.

### B) `SELECT CASE` / `CASE` (multi-branch)

* **Syntax:**

  ```
  SELECT CASE x%
    CASE 0, 1: PRINT "small"
    CASE 2 TO 5: PRINT "medium"
    CASE ELSE: PRINT "other"
  END SELECT
  ```
* **Semantics:** chain of comparisons and ranges; falls through disabled by default.

### C) `WITH … END WITH` (object member sugar)

* **Syntax:**

  ```
  WITH r@
    .Wins% = .Wins% + 1
    PRINT .Describe$()
  END WITH
  ```
* **Lowering:** rewrite `.X` to `r@.X` within block.

### D) `TRY … CATCH … FINALLY` (exception handling)

* **Syntax:** classic `TRY`/`CATCH err$`/`FINALLY`; integrates with VM error channel.
* **VM:** introduce structured exception ops or reuse existing trap if you have one.

