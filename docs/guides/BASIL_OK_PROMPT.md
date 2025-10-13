# The OK Prompt — Basil’s Old‑School CLI (Totally Tubular User Guide)

Welcome to the OK prompt — Basil’s rad, old‑school immediate mode CLI inspired by the 80’s. It’s like crackin’ open GW‑BASIC on a Saturday morning with a bowl of sugary cereal. Let’s jam.

Banner vibes:

BASIL - A BASIC Bytecode Interpreter and Compiler
Copyright (C) Blackrush LLC - All Rights Reserved.
Open Source Software under MIT License

Then you’ll see the prompt style:

OK


That’s the OK prompt. When you see it, you’re in the zone.

---

## Quickstart (No Bogus Steps)

- Start with an empty environment (clean slate):
  - `basilc cli`
- Start and preload a program (so its globals/functions/classes are ready to party):
  - `basilc cli path\to\main.basil`

Either way, you’ll drop into the OK prompt with the live VM. You can:
- Type immediate snippets that run on `;;` (double semicolon), or
- Enter old‑school numbered program lines (e.g., `10 PRINT "Yo"`), then `RUN`.

---

## The Vibe: Immediate Mode with `;;`

- Multiline input is buffered until you end a line with `;;` (whitespace after is cool).
- If your snippet is a single expression, its value is printed automatically. Bodacious!
- Example:
  - `1 + 2 * 3 ;;` → prints `7`
- Errors don’t harsh your mellow: diagnostics print and you’re right back at `OK`.
- `PRINT` shows output immediately (no newline required). `PRINTLN` adds a newline.

Tip: You can still use meta commands (`:help`, `:vars`, etc.) and shell escapes (`!dir`) at any time — those run immediately and don’t affect the snippet buffer.

---

## The Old‑School Program Buffer (Line Numbers FTW)

At the OK prompt, you can type numbered lines to build a program buffer, just like the glory days:

- `10 LET A% = 2`
- `20 PRINT "A="; A%`

Rules of the road:
- Entering `N <code>` inserts or replaces the line numbered `N`.
- Only lines you actually entered are stored — gaps are totally allowed.
- `LIST` prints the lines you entered in ascending order as `N <code>` — gaps are ignored.
- Save and load preserve numbering, filling gaps with blank lines when saving (so your layout looks choice when you reload).
- To delete a line, enter its number with no code (e.g., `20`) — if supported in your build; otherwise replace it with a blank statement.

### Commands for the Buffer

- `LIST`
  - Prints just the lines you entered. Example output:
    - `10 LET A% = 2`
    - `30 PRINTLN A%`
  - Gaps aren’t shown here.

- `STATUS`
  - Prints the program listing (same as `LIST`), then shows a catalog of globals currently in memory: variables, functions, classes/objects, arrays — with a tight value preview and where they came from (filenames or `<repl>`).
  - Think of it as LIST + “what’s live under the hood.” Totally clutch for debugging.

- `CLEAR`
  - Wipes the program buffer. Your live VM environment (variables/functions/classes already loaded) stays — not bogus.

- `RUN`
  - Saves the buffer to a temp file `_.basil` (filling numeric gaps with blank lines so the numbering stays intact) and runs it.
  - Does not clear the preloaded environment — your code can use previously defined variables, functions, and objects.

- `RUN <file>`
  - Clears the buffer, loads the file into the buffer (assigning each nonblank line a numbered slot preserving file order), runs it, then leaves it in the buffer so you can LIST, edit, and RUN again.

- `LOAD <file>`
  - Clears the buffer and loads the file, but does not run it. Perfect for tune‑ups before a big run.

- `SAVE [file]`
  - Saves the buffer to `file`; default is `_.basil` if you don’t specify.
  - Includes blank lines for numeric gaps to preserve your righteous line layout when you reload.

---

## Launch Modes (Bring Your Own Funk)

- `basilc cli`
  - Empty global environment. Build it up with numbered lines, immediate snippets, or by loading other files and class modules.

- `basilc cli path\to\main.basil`
  - Runs `main.basil` first (compiles to `.basilx` as needed) and keeps that live environment around.
  - Now your numbered program or immediate snippets can reference anything defined by the program — functions, classes, variables, the whole mixtape.

---

## Meta Commands (Colon Coolness)

These start with `:` and execute immediately:
- `:help` — quick summary of meta commands.
- `:vars [filter]` — list globals with type/value preview.
- `:types [name]` — show type info or summarize known types.
- `:methods <var>` — reflect methods/properties on a class instance.
- `:disasm <name>` — disassemble a function/method by symbol name.
- `:history` — shows recent snippet starts.
- `:save <file>` / `:load <file>` — save/load the current buffered snippet (not the numbered program buffer).
- `:bt on|off` — toggle backtraces on runtime errors.
- `:env` — show features, search paths, VM info.
- `:exit` — bail out gracefully. (Aliases: `quit`, `system`.)

Note: The numbered program buffer commands (`LIST`, `RUN`, `SAVE`, `LOAD`, `CLEAR`, `STATUS`) are OK‑prompt specials and don’t require a colon.

---

## Shell Escapes (Command Line, But Rad)

- Any line starting with `!` runs a system command immediately:
  - `!dir *.basil`
  - `!powershell`
- Output streams through and you’re back to `OK` when done. Use responsibly, dude.

---

## Examples (Showtime!)

### 1) Fresh Session, Quick Math
```
basilc cli

OK

1 + 2 * 3 ;;
7

OK
```

### 2) Build a Tiny Program with Lines
```
OK

10 LET A% = 2
20 LET B% = 5
30 PRINT "A+B="; A% + B%
LIST
10 LET A% = 2
20 LET B% = 5
30 PRINT "A+B="; A% + B%
RUN
A+B=7

OK
```

### 3) Load, Tweak, Run
```
LOAD examples\for.basil
LIST
... (shows the numbered listing from the file)
10 PRINTLN "Hello from for.basil"
SAVE mycopy.basil
RUN
```

### 4) Preload a Program, Keep Its Gear
```
basilc cli examples\init.basil
OK
PRINT GREET$ + " from REPL" ;;
Hi from REPL

OK
STATUS
(Shows your listing, then globals with origin files like examples\init.basil or <repl>)
```

---

## LIST vs STATUS (Don’t Space Out)

- `LIST` shows only what you typed into the numbered program buffer, line by line as `N <code>`. No gaps, no extras — just your script.
- `STATUS` shows the same listing AND dumps the currently instantiated globals (variables, arrays, objects) and known functions/classes, plus the filename(s) each came from. It’s the mega‑mix of “what I wrote” and “what’s live now.”

---

## Exiting

- `:exit` — clean exit.
- `quit` or `system` — same as `:exit`.

Catch you on the flip side.

---

## Tips & Gotchas (Totally Choice)

- Remember the `;;` rule for immediate snippets — nothing runs until you drop those double semicolons.
- `PRINT` output appears right away; use `PRINTLN` when you want the newline.
- Snippets and numbered programs share the same live environment — RUN doesn’t wipe your globals. Radical for live coding.
- When saving, blank lines fill gaps so your line numbering looks the same when reloaded.
- Shell escapes run as your current user — powerful, not bogus. Handle with care.

Cowabunga, coder. The OK prompt awaits.