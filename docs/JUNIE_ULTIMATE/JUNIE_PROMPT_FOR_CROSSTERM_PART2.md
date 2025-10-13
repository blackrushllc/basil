Got it — here’s a tight “Phase 2” add-on brief you can paste to Junie later. It keeps your existing `obj-term` API intact and adds only the minimal power features.

---

# JUNIE TASK — `obj-term` Phase 2 (super-minimal upgrade)

**Goal:** Extend the existing **`obj-term`** Crossterm module with just enough features to support responsive TUIs without a full event system.

## Scope (add, don’t break)

**New statements**

* `TERM.INIT` — Initialize terminal session state; prepare stdout handle; no-op if already initialized.
* `TERM.END` — Restore terminal to a sane state (show cursor, disable raw, leave alt-screen, disable mouse if you enabled it later); safe to call multiple times.
* `TERM.RAW ON|OFF` — Enter/exit raw mode (no line buffering).
* `ALTSCREEN_ON` / `ALTSCREEN_OFF` — Enter/leave alternate screen buffer.
* `TERM.FLUSH` — Flush any queued terminal writes (pair with queued `PRINT`/cursor ops to reduce flicker).

**New function**

* `TERM.POLLKEY$()` → Non-blocking key read. Returns `""` if no key is available; otherwise normalized strings like:

  * `"Enter"`, `"Esc"`, `"Tab"`, `"Backspace"`, `"Up"`, `"Down"`, `"Left"`, `"Right"`
  * `"Char:a"` (lowercase single chars), `"Char:A"` (if Shifted), etc.

> Keep all existing commands (`CLS`, `LOCATE`, `COLOR`, cursor save/restore, size, hide/show) unchanged.

## Behavior & impl notes

* Use Crossterm for all operations (no raw ANSI).
* Maintain a small global `TerminalState` behind a `Mutex`:

  * flags: `initialized`, `raw_on`, `alt_on`
  * cached stdout handle / buffer for `TERM.FLUSH`
* Make `TERM.INIT` idempotent; `TERM.END` restores any active modes and resets colors/attributes (if you track them).
* `TERM.POLLKEY$()` implementation:

  * Use `crossterm::event::poll(Duration::from_millis(0..5))` then `event::read()`.
  * Normalize keys to strings as described.
  * Return `""` if no event or if the event is not a key (ignore others for Phase 2).
* **No-TTY / CI**: If stdout isn’t a TTY, commands should succeed as no-ops (return OK) so examples/tests don’t explode.
* Windows + Unix supported; Crossterm handles ANSI enabling on Windows.

## Examples (put under `examples/term_phase2/`)

**`01_pollkey_echo.basil`**

```basic
TERM.INIT; TERM.RAW ON; CURSOR_HIDE; CLS;
PRINT "Press ESC to quit…"; TERM.FLUSH;
DO
  k$ = TERM.POLLKEY$();
  IF k$ <> "" THEN
     IF k$ = "Esc" THEN EXIT DO;
     PRINT "\rKey: "; PRINT k$; PRINT "     "; TERM.FLUSH;
  ENDIF
LOOP
CURSOR_SHOW; TERM.RAW OFF; TERM.END;
```

**`02_altscreen_title.basil`**

```basic
TERM.INIT; ALTSCREEN_ON; CLS;
PRINTLN "Basil + Crossterm (Alt Screen)";
PRINTLN "Press any key to exit…"; TERM.FLUSH;
DO : k$ = TERM.POLLKEY$() : LOOP WHILE k$ = ""
ALTSCREEN_OFF; TERM.END;
```

**`03_buffered_redraw.basil`**

```basic
TERM.INIT; ALTSCREEN_ON; CLS;
FOR i% = 1 TO 20
  LOCATE(1,1); PRINT "Frame "; PRINT i%;
  LOCATE(1,3); PRINT STRING$(i% MOD 60, "#");
  TERM.FLUSH; SLEEP 33;
NEXT i%;
ALTSCREEN_OFF; TERM.END;
```

## Tests

* **Unit (pure)**

  * `TerminalState` lifecycle: `INIT` idempotent; `END` resets flags.
* **Integration (TTY-only; mark `#[ignore]` unless isatty)**

  * `TERM.RAW ON` then OFF returns OK.
  * `ALTSCREEN_ON` then `_OFF` returns OK.
  * `POLLKEY$()` returns `""` when no key pressed within short poll window.

## Acceptance

* Builds on stable Rust with the existing `obj-term` feature.
* No breaking changes to Phase 1 API.
* Examples run on Windows Terminal and typical Linux/macOS terminals.
* `TERM.END` always restores the console even after errors (use Drop guard if needed).

**Deliverables:** Code changes, the three examples above, brief updates to `docs/OBJ_TERM.md` describing Phase 2 additions and key normalization table.
