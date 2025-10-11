Here’s a fresh, drop-in **Basil Bootstrap Prompt** you can reuse anytime, plus the compact syntax summary I’ll follow.

---

# Basil Bootstrap Prompt (paste this first)

**You are starting with zero prior context about Basil.**
I will provide (or have provided) a single Basil “Kitchen Sink” source file named **`bigtest.basil`** that defines the **current canonical syntax and idioms** for the language.

## What Basil is (quick facts)

* BASIC-style language with a Rust interpreter/VM.
* Type suffixes: `%` = integer/number, `$` = string, `@` = object/handle.
* Statements end with semicolons `;`. Comments: `REM ...`, `' ...`, `// ...`.
* Blocks use `BEGIN … END`. Multi-line loop bodies and conditionals require blocks.
* Core flow: `IF … THEN … ELSE …`, `FOR … TO … [STEP …] / NEXT`, `FOR EACH … IN … / NEXT`, `WHILE … BEGIN … END`, `BREAK`, `CONTINUE`.
* Printing: `PRINT` (no newline), `PRINTLN` (newline).
* Feature Objects exist (compiled behind Cargo features), e.g. `obj-audio`, `obj-midi`, `obj-daw`, `obj-term`.

## What to do on receipt

1. **If `bigtest.basil` is attached:** read it and reply with a **≤15-line syntax summary** (terminators, blocks, loops, arrays, objects, printing, operators, comments). Use that style for all outputs.
2. **If it is NOT attached:** ask once for it; if I don’t have it, proceed using the “quick facts” and summary below.
3. **Feature Objects:** future programs may call helpers such as `AUDIO_RECORD%()`, `MIDI_CAPTURE%()`, `SYNTH_LIVE%()`, or lower-level APIs. Assume the requested features are compiled in.
4. **When I ask for a Basil program next:** produce a **single, copy-paste-runnable** file in Kitchen-Sink style; keep comments concise; prefer non-blocking main loops with a clean exit when relevant; include error checks like `DAW_ERR$()` where appropriate.

## Response style

* Be concise and practical.
* Don’t over-question; pick reasonable defaults and proceed.

**Ready?**
Reply with either **“Waiting for Kitchen Sink file”** or a **10–15 line syntax summary** extracted from it, and a one-liner that you’re ready to generate Basil examples using `obj-audio`, `obj-midi`, and/or `obj-daw` on request.

---

## Compact syntax summary I’ll follow

1. Statements end with `;`. Comments: `REM …`, `' …`, `// …`.
2. Type suffixes: `%` numeric, `$` string, `@` object/handle.
3. **Assignments require `LET`** (objects may be constructed without `LET` in `DIM … AS Type(...)`).
4. Blocks are `BEGIN … END`; required for multi-line `FOR/NEXT` bodies and other multi-line constructs.
5. Conditionals: one-line `IF … THEN …` or block form `IF … THEN BEGIN … END [ELSE BEGIN … END]`.
6. Loops: `FOR i% = a TO b [STEP s] BEGIN … END; NEXT i%`.
7. **No `DO..LOOP`**; use `WHILE TRUE BEGIN … END` (or a real condition) with `BREAK`/`CONTINUE`.
8. `FOR EACH v% IN arr% BEGIN … END; NEXT`.
9. Arrays: `DIM a%(n);` (0-based). 2D strings supported, e.g., `DIM s$(x,y);`. `LEN()` for length.
10. Strings: `LEFT$`, `RIGHT$`, `MID$`, `INSTR`, `UCASE$`, concatenation with `+`.
11. Printing: `PRINT` (no newline), `PRINTLN` (newline); may combine with variables/expressions.
12. Objects: `DIM r@ AS TYPE(args…); r@.Prop$ = …; r@.Method$()`; arrays of objects allowed.
13. Feature calls assume their objects are compiled in; use helpers and check error strings where provided.
14. Whitespace is flexible; semicolons terminate; use clear, minimal comments.

If you ever want me to re-ingest `bigtest.basil` and refresh this summary, just say “Kitchen Sink refresh” and hand me the file—I’ll restate the 10–15 lines and we’re off.
