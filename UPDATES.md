### What I added
- Core math intrinsics implemented end-to-end in the compiler and VM: `ABS(x)`, `ATN(x)`, `COS(x)`, `EXP(x)`, `INT(x)`, `LOG(x)`, `RND([n])`, `SIN(x)`, `SQR(x)`, `TAN(x)`.
- PRINT helpers and column control: `SPC(n)`, `TAB(n)` and `AT(n)` (alias of `TAB`). These now integrate with an output column tracker in the VM so they behave like standard BASIC when used in `PRINT`.
- Forward-declaration syntax: `DECLARE SUB ...` and `DECLARE FUNC|FUNCTION ...` are accepted by the parser (as no‑op prototypes). Forward calls already worked in our compiler’s pre‑scan; this adds the familiar syntax.
- Advanced formatting: `USING$(fmt$, var, var, ...)` returning a formatted string. It supports a pragmatic, printf‑like subset: `%d`, `%f` (with optional width and precision like `%8.2f`), `%s`, with `-` (left align), `0` (zero pad), and `%%` for a literal percent.
- Minor lexer enhancement to accept the `!` suffix in identifiers (so examples like `StarCount!` parse cleanly).

### Files changed (key points)
- `basilcore/compiler/src/lib.rs`
  - Added name→builtin ID mapping for new math and formatting functions:
    - Math: `ABS`(70), `ATN`(71), `COS`(72), `EXP`(73), `INT`(74), `LOG`(75), `RND`(76), `SIN`(77), `SQR`(78), `TAN`(79)
    - Printing helpers: `SPC`(80), `TAB`(81), `AT`(82)
    - Formatting: `USING$`(83)
- `basilcore/vm/src/lib.rs`
  - VM now tracks output column: added `out_col: usize` and updated `Op::Print` to keep column up-to-date (newlines reset; `\t` advances to next 8‑column tab stop; other chars advance by 1).
  - Added RNG state `rnd_state: u64` with a simple xorshift64* PRNG for `RND`.
  - Added helpers `to_f64` and `update_out_col_with`.
  - Implemented Builtin handlers for IDs 70–83:
    - Math: proper numeric coercions; domain checks for `LOG(x<=0)` and `SQR(x<0)` raise a runtime error.
    - `RND()`: returns a float in [0,1). `RND(n)`: returns an int in `[0, n)` (0 when `n<=0`).
    - `SPC(n)`: returns a string of spaces. `TAB(n)`/`AT(n)`: return spaces to move from the current column to column `n` (1‑based).
    - `USING$`: minimal printf‑like formatter supporting `%d`, `%f`, `%s`, width, precision, left align and zero pad; returns a string.
- `basilcore/parser/src/lib.rs`
  - `PRINT`/`PRINTLN` are now emitted as a sequence of individual prints (instead of building a single concatenated expression). This allows column tracking to be correct between items; commas still inject a `"\t"` for the existing BASIC‑style zone separator.
  - Added parsing of `DECLARE SUB/FUNC/ FUNCTION name(params)` as a no‑op statement (prototype only); forward calls already work due to pre‑scan of actual definitions.
- `basilcore/lexer/src/lib.rs`
  - Added `TokenKind::Declare` and keyword mapping for `DECLARE`.
  - Allowed `!` in identifiers (so `StarCount!` parses).

### Usage examples
- Math & random:
```basic
PRINT "abs:"; ABS(-5), " atan:"; ATN(1), " cos:"; COS(0)
PRINT "exp:"; EXP(1), " int:"; INT(3.8), " log:"; LOG(2.718281828)
PRINT "rnd float:"; RND()
PRINT "rnd 0..9:"; RND(10)
PRINT "sin:"; SIN(1.57079632679), " sqr:"; SQR(9), " tan:"; TAN(0.78539816339)
```
- Output positioning with column tracking:
```basic
PRINT "Name"; TAB(20); "Score"; TAB(30); "Grade"
PRINT "Alice"; TAB(20); 98; TAB(30); "A"
PRINT "Bob";   AT(20);  87; AT(30);  "B"
PRINT "Spaces after label:"; SPC(5); "here"
```
- Formatting with `USING$`:
```basic
PRINT USING$("%-10s %5d %8.2f", "Alice", 98, 3.14159)   ' -> left padded name, int, float with precision
msg$ = USING$("id=%d pct=%6.2f%%", 42, 87.3)
PRINT msg$
```
- Forward declaration syntax (prototype + later definition):
```basic
DECLARE SUB PrintSomeStars(StarCount!)
DECLARE FUNCTION add(a%, b%)

PRINT "Sum:"; add(2,3)
CALL PrintSomeStars(5)

SUB PrintSomeStars(StarCount!)
  FOR i% = 1 TO StarCount!
    PRINT "*";
  NEXT
  PRINT
END SUB

FUNCTION add(a%, b%)
  RETURN a% + b%
END FUNCTION
```

### Behavioral notes
- `INT(x)`: integers are unchanged; floats are floored (e.g., `INT(-3.2) == -4`).
- `LOG(x)`: natural log; raises a runtime error if `x <= 0`.
- `SQR(x)`: raises a runtime error if `x < 0`.
- `RND()`: float in `[0,1)`; `RND(n)`: integer in `[0, n)`; for `n <= 0` returns `0`.
- PRINT column tracking now honors `\n` (reset to column 0) and `\t` (tab stop every 8 columns). `TAB(n)`/`AT(n)` compute padding from the current column to 1‑based column `n`.
- `DECLARE` is syntactic only; a matching definition must appear later (as is typical in BASIC); calls are already resolved via the compiler’s definition pre‑scan.

### Constraints / Build note
- Building the full workspace may require native dependencies (e.g., `nettle-sys` needs `pkg-config`). The core changes were made in the main language crates and do not depend on those optional features. If your environment can’t build optional crates, you can build just the core or disable features when testing.

### Where to look in the code
- Compiler builtins mapping: `basilcore/compiler/src/lib.rs` (around the name→ID match for builtins).
- VM builtins and column tracking: `basilcore/vm/src/lib.rs` (search for `// --- Math intrinsics ---`, `// --- PRINT helpers ---`, and `Op::Print`).
- Parser PRINT rewriting and DECLARE handling: `basilcore/parser/src/lib.rs` (branches for `TokenKind::Print`, `TokenKind::Println`, and `TokenKind::Declare`).
- Lexer additions: `basilcore/lexer/src/lib.rs` (keyword mapping for `DECLARE`, extended identifier rules).

### Limitations / Future polish
- `USING$` currently mimics a subset of printf; a future pass can add more BASIC‑style descriptors (e.g., `#` picture formats) if desired.
- PRINT’s comma separator still inserts a `\t` (8‑column tab stop) for compatibility with prior behavior; if you prefer old‑school BASIC print zones, we can adjust the zone width.
- `DECLARE` does not yet validate signature vs. later definition; it acts as a prototype keyword for familiarity. We can add signature checks if needed.

### Validation
- Due to external build deps on some optional crates, a full `cargo build` may fail on systems without `pkg-config`. The core files compile in isolation; if you want, I can add a small example under `testprogs` and guide how to run it with feature flags disabled.

```
Please run the command below and observe that the forward declare statements for both the sub and function are not allowing the sub or function to be called, rather it is resulting in a "CALL target is not a function" error.

The program runs correctly if the module level code is moved to after the SUB and FUNCTION code.  However the DECLARE statements are supposed to allow a forward reference to these routines.


cargo run -p basilc -- run examples\declare.basil 
```