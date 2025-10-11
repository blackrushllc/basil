Here’s a tight, drop-in brief for Junie to implement **Phase 3** (mouse + events + quality-of-life) for the Crossterm terminal module. It assumes our existing Phase 1/2 API and Basil style (semicolons, `LET`, `BEGIN…END`, etc.).

---

# JUNIE TASK — `obj-term` Phase 3 (Mouse + Events & UX polish)

**Context:**
We have `obj-term` (Crossterm) with lifecycle, color, cursor, alt-screen, raw mode, buffered writes, and `TERM.POLLKEY$()`. Phase 3 adds **mouse input** and a **unified event stream**, plus a few small UX improvements and tests/demos.

## Goals (deliverables)

1. Add **mouse capture** API and a **unified event function**:

    * `TERM.MOUSE ON|OFF`
    * `TERM.NEXT_EVENT$()` → returns `""` if none; otherwise a compact JSON string for `key`, `mouse`, or `resize` (and `focus` if supported).
2. Normalize keys, buttons, modifiers, coordinates; 1-based (Basil-friendly).
3. Non-blocking polling with short internal timeouts; no busy spin.
4. Cross-platform correctness (Windows + Unix) via Crossterm.
5. Update docs and demos; extend the non-interactive harness to lightly touch the new APIs.

---

## Public BASIC API (additions)

**Statements**

* `TERM.MOUSE ON` / `TERM.MOUSE OFF`
  Enable/disable mouse capture (Crossterm mouse capture). Idempotent.

**Functions**

* `TERM.NEXT_EVENT$()`
  Non-blocking. Returns `""` if no event is available within a short poll window; otherwise **JSON**:

```json
// Key
{"type":"key","key":"Left","shift":false,"alt":false,"ctrl":false}

// Char keys use "Char:<c>" with the original case
{"type":"key","key":"Char:A","shift":true,"alt":false,"ctrl":false}

// Resize
{"type":"resize","w":120,"h":30}

// Mouse
{"type":"mouse","x":42,"y":10,"kind":"Down","button":"Left","shift":false,"alt":false,"ctrl":false}
{"type":"mouse","x":42,"y":10,"kind":"Up","button":"Right",...}
{"type":"mouse","x":42,"y":11,"kind":"Drag","button":"Left",...}
{"type":"mouse","x":40,"y":12,"kind":"Move","button":"None",...}
{"type":"mouse","x":40,"y":12,"kind":"Scroll","dy":1,"button":"None",...}

// (Optional if Crossterm emits)
{"type":"focus","focused":true}
```

**Conventions**

* **Coords**: `x` = column, `y` = row, **1-based** (clamp to `[1..TERM_COLS]`, `[1..TERM_ROWS]` on emission).
* **Buttons**: `"Left" | "Right" | "Middle" | "None"`.
* **Kind**: `"Down" | "Up" | "Drag" | "Move" | "Scroll"`.
* **Scroll**: `dy` integer (+1 up, −1 down).
* **Key** normalization (same set as Phase 2): `"Enter"|"Esc"|"Tab"|"Backspace"|"Up"|"Down"|"Left"|"Right"|"Home"|"End"|"PageUp"|"PageDown"|"F1"...|"F12"|"Char:<char>"`.
* **Modifiers**: `shift/alt/ctrl` booleans always present.

---

## Implementation notes

### Event loop integration

* Use `crossterm::event::{poll, read, Event, KeyEvent, MouseEvent, Resize}`.
* A single short poll (e.g., 0–5 ms) per call to `TERM.NEXT_EVENT$()`; if nothing, return `""`.
* **No blocking** inside `NEXT_EVENT$()` or `POLLKEY$()`.

### State & safety

* Reuse global `TerminalState` mutex; add flags: `mouse_on: bool`.
* Ensure `TERM.END` disables mouse capture, leaves alt screen, disables raw, shows cursor, flushes buffer. Idempotent.
* Graceful **no-TTY** behavior: APIs succeed as no-ops; `NEXT_EVENT$()` simply returns `""`.

### Normalization & mapping

* Convert Crossterm 0-based coords to **1-based** before JSON.
* Map mouse kinds/buttons to the string enums above.
* Map keys to the normalized set; chars use `"Char:<c>"`.
* Always include modifiers in key & mouse JSON.

### Performance

* Avoid allocations in the hot path: small fixed buffers for JSON (`arrayvec` optional) or pre-size a `String`.
* Keep the JSON minimal and consistently ordered keys to simplify downstream parsing (tests rely on exact output).

---

## Docs

Add/extend **`docs/OBJ_TERM.md`**:

* New functions with examples and the **JSON schemas** above.
* Table of supported keys/buttons/modifiers.
* Coordinate 1-based note and bounds clamping.
* Platform notes (wheel direction may vary across terminals; we normalize `dy`).

Add/extend **`docs/AOT_COMPILER.md`** (one sentence): compiled programs can use the same event API.

---

## Demos (`examples/term_phase3/`)

1. **`mouse_echo.basil`** — shows the next ~15 events in a strip; quits on `Esc`.

    * Enables mouse, prints compact lines: `x,y,kind,btn,key?`.
    * Uses `ALTSCREEN_ON` so screen is clean.

2. **`mouse_draw.basil`** — drag to draw `#`; right-click clears; wheel scroll prints a line.

    * Demonstrates `LOCATE`, color, `CLEAR_LINE`, and drag tracking.

3. **`resize_status.basil`** — prints current size; updates when `resize` events arrive.

4. **`key_vs_event.basil`** — compares `POLLKEY$()` vs `NEXT_EVENT$()` key output for a few keystrokes.

*(All samples in Kitchen-Sink style: `LET`, `BEGIN/END`, `WHILE TRUE`.)*

---

## Tests

### Unit (pure)

* Key mapping: ensure representative keys map to normalized strings.
* Mouse mapping: `Down/Up/Move/Drag/Scroll` + buttons + modifier flags.
* Coord conversion: 0-based → 1-based; clamping to bounds.

### Integration (TTY-only; mark `#[ignore]` unless isatty)

* Mouse echo: enable capture, poll for a short window, ensure no panic and valid JSON shape (regex).
* Resize: simulate via terminal resize where possible (allowed to be manual in local runs).
* Teardown: verify `TERM.END` restores modes regardless of prior state.

### Non-interactive harness add-on

Extend the **non-interactive** selftest to:

* Enable mouse for ~300ms and call `TERM.NEXT_EVENT$()` in a loop (discard results).
* Call `TERM.NEXT_EVENT$()` multiple times to ensure steady `""` when idle.

---

## Examples (Basil snippets for demos)

**`mouse_echo.basil`**

```basic
ALTSCREEN_ON; TERM.INIT; TERM.RAW ON; TERM.MOUSE ON; CLS;
PRINTLN "Mouse/Key/Resize events (Esc to quit)…";
WHILE TRUE BEGIN
  LET ev$ = TERM.NEXT_EVENT$();
  IF ev$ <> "" THEN PRINTLN ev$;
  IF ev$ <> "" AND INSTR(ev$, "\"type\":\"key\"") > 0 AND INSTR(ev$, "Esc") > 0 THEN BREAK;
  SLEEP 10;
END
TERM.MOUSE OFF; TERM.RAW OFF; ALTSCREEN_OFF; TERM.END;
```

**`mouse_draw.basil`**

```basic
ALTSCREEN_ON; TERM.INIT; TERM.RAW ON; TERM.MOUSE ON; CLS;
LET drawing% = 0;
WHILE TRUE BEGIN
  LET ev$ = TERM.NEXT_EVENT$();
  IF ev$ = "" THEN SLEEP 5; CONTINUE;
  IF INSTR(ev$, "\"type\":\"key\"") > 0 AND INSTR(ev$, "Esc") > 0 THEN BREAK;

  IF INSTR(ev$, "\"type\":\"mouse\"") > 0 THEN
    LET x% = VAL(MID$(ev$, INSTR(ev$, "\"x\":")+4));  REM naive parse ok for demo
    LET y% = VAL(MID$(ev$, INSTR(ev$, "\"y\":")+4));
    IF INSTR(ev$, "\"kind\":\"Down\"") > 0 AND INSTR(ev$, "\"button\":\"Left\"") > 0 THEN LET drawing% = 1;
    IF INSTR(ev$, "\"kind\":\"Up\"") > 0 THEN LET drawing% = 0;
    IF drawing% = 1 AND INSTR(ev$, "\"kind\":\"Drag\"") > 0 OR INSTR(ev$, "\"kind\":\"Move\"") > 0 THEN
      LOCATE(x%, y%); PRINT "#"; TERM.FLUSH;
    ENDIF
    IF INSTR(ev$, "\"button\":\"Right\"") > 0 AND INSTR(ev$, "\"kind\":\"Down\"") > 0 THEN CLS;
  ENDIF
END
TERM.MOUSE OFF; TERM.RAW OFF; ALTSCREEN_OFF; TERM.END;
```

---

## Acceptance

* Builds on stable Rust; passes unit tests; integration tests marked `#[ignore]` unless TTY.
* `TERM.MOUSE` toggles capture idempotently.
* `TERM.NEXT_EVENT$()` returns `""` when idle; returns normalized JSON for key/mouse/resize (+ focus if available).
* Coordinates 1-based and clamped.
* `TERM.END` always restores raw/alt/mouse/cursor state.
* Demos run on Windows Terminal, common Linux terminals, and macOS Terminal/iTerm2.

---

If you want, I can also provide:

* a tiny **JSON validator** (regex) for the demos/tests, and
* a **Basil micro-parser** helper snippet you can reuse to extract `x`/`y`/`key` fields without pulling a full JSON lib.
