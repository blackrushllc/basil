# obj-term (Terminal screen library)

Phase 1 provided basic terminal control using Crossterm: CLS/CLEAR/HOME, LOCATE, COLOR/COLOR_RESET, ATTR/ATTR_RESET, CURSOR_SAVE/CURSOR_RESTORE, TERM_COLS%/TERM_ROWS%, CURSOR_HIDE/CURSOR_SHOW, and TERM_ERR$.

Phase 2 additions (non‑breaking):

Statements
- TERM.INIT — Initialize terminal session state; idempotent.
- TERM.END — Restore terminal to a sane state (show cursor, disable raw, leave alt-screen); safe to call multiple times.
- TERM.RAW ON|OFF — Enter/exit raw mode (no line buffering). Also accepts 0/1 or strings "ON"/"OFF".
- ALTSCREEN_ON / ALTSCREEN_OFF — Enter/leave alternate screen buffer.
- TERM.FLUSH — Flush any queued terminal writes (pair with PRINT/cursor ops to reduce flicker).

Function
- TERM.POLLKEY$() -> STRING — Non-blocking key read. Returns "" if no key available; otherwise normalized:
  - "Enter", "Esc", "Tab", "Backspace"
  - "Up", "Down", "Left", "Right"
  - "Char:x" for character keys (case preserved)

Notes
- Uses Crossterm for all operations; no raw ANSI.
- Small global TerminalState tracks initialized/raw_on/alt_on and style flags.
- No‑TTY/CI: If stdout isn’t a TTY, operations succeed as no‑ops (return OK).
- Windows + Unix supported.

Examples (examples/term_phase2/)
- 01_pollkey_echo.basil
- 02_altscreen_title.basil
- 03_buffered_redraw.basil
