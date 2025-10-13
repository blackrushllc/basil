# JUNIE TASK — Add `obj-term` (Crossterm terminal screen library for Basil)

**Goal:** Create a new Feature Object **`obj-term`** that exposes BASIC-style terminal screen manipulation backed by **Crossterm** (Windows/Linux/macOS). When the feature is enabled, Basil gains new commands that control the screen and colors and work naturally with `PRINT`, `PRINTLN`, and `INPUT`.

## 0) Project wiring

* Workspace feature flag:

  ```toml
  [features]
  obj-term = []
  ```
* Crate or module name: `obj-term` (or `crates/obj-term/`).
* On VM init (when feature enabled), call `obj_term::register(vm)`.

## 1) Public BASIC API (new commands)

### Screen clearing & home

* `CLS`
  Clears entire screen **using the current color** (foreground/background) and moves cursor to home (1,1).

  * **Aliases:** `CLEAR`, `HOME`
  * `CLEAR` and `HOME` both behave exactly like `CLS` for simplicity.

### Cursor movement

* `LOCATE(x%, y%)`
  Move cursor to column `x%` (1-based), row `y%` (1-based). Clamp to terminal bounds.

### Colors / attributes

* `COLOR(fg%, bg%)`
  Set **current** foreground/background color used by subsequent `PRINT`/`PRINTLN`/`INPUT` and by `CLS`.

  * Accept **either** numeric constants **or** string names (see mapping below).
  * `fg%` or `bg%` may be `-1` to mean “leave unchanged”.
* `COLOR_RESET`
  Reset to default terminal colors.
* (Optional but nice) `ATTR(bold%, underline%, reverse%)`
  Toggle common attributes; each arg: 0=off, 1=on, -1=no change.
* `ATTR_RESET`

### Save/restore cursor position

* `CURSOR_SAVE`
  Save current cursor position (stack depth ≥ 8 is fine).
* `CURSOR_RESTORE`
  Restore the most recently saved position (no-op if stack empty).

### Terminal size

* `TERM_COLS%()` → number of columns
* `TERM_ROWS%()` → number of rows

### Cursor visibility (handy for UIs)

* `CURSOR_HIDE`
* `CURSOR_SHOW`

> **Integration guarantee:** `PRINT`, `PRINTLN`, and `INPUT*` must **already honor** the terminal’s current cursor position and colors. If Basil’s runtime writes via a centralized output routine, hook the color/attr state there so these calls inherit the state automatically. No special `PRINT` variants needed.

---

## 2) Color names (mapping)

Support both **numbers** and **names** (case-insensitive) for `COLOR`. Map to Crossterm’s 16-color palette:

* Foreground/background integer values (0–15):
  `0=Black, 1=Red, 2=Green, 3=Yellow, 4=Blue, 5=Magenta, 6=Cyan, 7=White, 8=Grey, 9=BrightRed, 10=BrightGreen, 11=BrightYellow, 12=BrightBlue, 13=BrightMagenta, 14=BrightCyan, 15=BrightWhite`
* Accept these **strings** too: `"black","red","green","yellow","blue","magenta","cyan","white","grey","brightred","brightgreen","brightyellow","brightblue","brightmagenta","brightcyan","brightwhite"`

If a name/number is out of range, return error (see errors).

---

## 3) Behavior & platform notes

* Use **Crossterm** APIs (not raw escape strings) so Windows/old terminals are handled.

  * Enter/leave **raw mode** only when needed (not required for these APIs).
  * Use **alternate screen** only if we add a separate pair later (see Extras).
* Keep a small **global state** for current fg/bg + attrs (inside `obj-term`), so `CLS` clears with the active colors and `PRINT` naturally picks up current style via the output hook.
* `LOCATE` is 1-based; clamp to `[1..TERM_COLS]` and `[1..TERM_ROWS]`.
* `CURSOR_SAVE/RESTORE`: implement a simple **stack**; overflow drops oldest.
* **Thread-safety:** Tie state to the process/TTY (single-threaded UI assumed). If VM can be multi-threaded, gate calls on a UI mutex.

---

## 4) Errors

* All commands return `%` (0=OK, non-0=error) **or** are statements that set a last-error string:

  * If your Basil style prefers statements for these, then expose `TERM_ERR$()` to fetch last error.
  * Typical errors: “invalid color”, “terminal not available”, etc.

---

## 5) What we will **not** implement (and why)

* **Reading arbitrary screen contents** is **not portable**. Terminals don’t expose a standard “read from screen buffer” API. Some terminals support proprietary queries, but there’s no cross-platform guarantee.

  * We **can** add `TERM_GET_CURSOR%(OUT x%, OUT y%)` (portable), but **not** “read box of text from screen”.

---

## 6) Optional “nice to have” (add if quick)

* `CLEAR_LINE` (clear current line; cursor stays)
* `SCROLL(n%)` (scroll content up by `n%` lines; clamp 0..rows)
* `ALTSCREEN_ON` / `ALTSCREEN_OFF` (enter/leave alternate screen buffer)
* Borders & boxes are application-level (draw with box-drawing glyphs via `PRINT`).

---

## 7) Examples (put in `examples/term/`)

**`01_colors_and_cls.basil`**

```basic
COLOR("brightyellow", "blue"); CLS;
PRINTLN "Hello, colorful world!"; 
COLOR_RESET; PRINTLN "Back to defaults.";
```

**`02_locate_save_restore.basil`**

```basic
CLS; COLOR(15, 4);  REM bright white on blue;
LOCATE(10, 3); PRINT "Title"; 
CURSOR_SAVE;
LOCATE(5, 6);  PRINT "Menu item A";
LOCATE(5, 7);  PRINT "Menu item B";
CURSOR_RESTORE; PRINTLN "  (cursor restored next to Title)";
```

**`03_size_and_hide_cursor.basil`**

```basic
CLS; CURSOR_HIDE;
PRINT "Terminal: "; PRINT TERM_COLS%(); PRINT "x"; PRINTLN TERM_ROWS%();
SLEEP 1500;
CURSOR_SHOW;
```

**`04_home_clear_aliases.basil`**

```basic
COLOR(10, -1); PRINTLN "I’ll clear in 1s…"; SLEEP 1000;
HOME;  REM alias of CLS;
```

*(If `HOME` should only “move to 1,1” without clearing, implement that instead and keep `CLS` as clear+home; your call—document the choice.)*

---

## 8) Tests

* **Unit:**

  * Color parser (names → codes; bad names fail).
  * Cursor save/restore stack behavior (push, pop, underflow no-op).
* **Integration (ignored on CI if no TTY):**

  * `CLS` then `LOCATE(1,1)` then `PRINT` → no error.
  * `TERM_COLS%() >= 20` and `TERM_ROWS%() >= 10` (skip if not TTY).
  * `CURSOR_HIDE` then `CURSOR_SHOW` returns OK.

---

## 9) Implementation sketch (Rust)

* Crate dep:

  ```toml
  [dependencies]
  crossterm = "0.27"
  anyhow = "1"
  ```
* State:

  ```rust
  struct TermState {
      fg: Option<Color>, bg: Option<Color>,
      bold: bool, underline: bool, reverse: bool,
      pos_stack: Vec<(u16,u16)>,
  }
  static GLOBAL: OnceLock<Mutex<TermState>> = ...;
  ```
* Register functions:

  * `CLS`/`CLEAR`/`HOME` → `execute!(..., Clear(All), MoveTo(0,0))` and apply current colors before clearing.
  * `LOCATE(x,y)` → `MoveTo(x-1,y-1)` after clamping.
  * `COLOR(fg,bg)` → parse, update state, `SetForegroundColor/SetBackgroundColor`.
  * `COLOR_RESET`/`ATTR_RESET` → `ResetColor` + clear attrs in state.
  * `CURSOR_SAVE`/`CURSOR_RESTORE` → push/pop + `cursor::position()`/`MoveTo`.
  * `TERM_COLS%()`, `TERM_ROWS%()` → `terminal::size()`.
  * `CURSOR_HIDE`/`CURSOR_SHOW` → `cursor::Hide/Show`.

Expose Basil shims accordingly.

---

## 10) Acceptance

* Builds on stable Rust with `--features obj-term`.
* Examples run on Windows Terminal/PowerShell and common Linux terminals.
* `PRINT/PRINTLN/INPUT` respect current cursor/attributes without needing program changes.
* `HOME/CLEAR/CLS/LOCATE/COLOR/CURSOR_SAVE/CURSOR_RESTORE` all behave as specified.

**Please implement `obj-term` per above, include the examples & tests, and update README/docs with usage and color table.**
