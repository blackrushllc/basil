Here’s a compact **bootstrap prompt** you (or anyone) can paste to me *before* making a Basil request. It tells AI what
Basil is, asks the AI to align to your latest “Kitchen Sink” syntax by reading an attached file, and sets expectations for
how the AI should respond.

---

# Basil Bootstrap Prompt (paste this first)

**You are starting with zero prior context about Basil.**
I’m going to provide (or soon upload) a single Basil “Kitchen Sink” source file that defines the **current canonical
syntax and idioms** for the language.

## What Basil is (quick facts)

* Basil is a BASIC-style language with a Rust interpreter/VM.
* Type suffixes: `%` = integer/number, `$` = string, `@` = object/handle.
* Programs use semicolons at end of statements and block pairs `BEGIN … END`.
* Common constructs: `IF … THEN … ELSE …`, `FOR … TO … STEP … / NEXT`, `FOR EACH … IN … / NEXT`, `WHILE … BEGIN … END`,
  `BREAK`, `CONTINUE`.
* Print: `PRINT` (no newline), `PRINTLN` (newline).
* We also use Feature Object libraries (compiled behind Cargo features) such as:

    * `obj-audio` (audio devices/streams/rings/WAV I/O),
    * `obj-midi` (MIDI in/out, event polling),
    * `obj-daw` (high-level helpers: record, play, monitor, MIDI capture, live synth).

## What I will give you

1. I may upload a file named **`bigtest.basil`** (aka “Kitchen Sink”) that reflects the **latest** syntax and style.
2. After you ingest it, I’ll ask you to write Basil programs or scaffolding (e.g., examples using
   `obj-audio/obj-midi/obj-daw`).

## What I want you to do on receipt (no back-and-forth unless essential)

1. **If the Kitchen Sink file is attached:**

    * Read it and extract the concrete syntax/idioms you observe (terminators, block forms, loops, array syntax,
      object/property/method style, printing, operators, comments).
    * Reply with a **brief syntax summary** (bullet points, ≤15 lines) you’ll follow for all outputs.
2. **If it is NOT attached:**

    * Ask me once to upload it (and proceed with sensible defaults listed above if I say I don’t have it).
3. **Acknowledge Feature Objects:**

    * Note that future programs may call functions like `AUDIO_RECORD%()`, `AUDIO_PLAY%()`, `AUDIO_MONITOR%()`,
      `MIDI_CAPTURE%()`, `SYNTH_LIVE%()` and/or lower-level APIs (`AUDIO_OPEN_IN@`, rings, WAV writer, `MIDI_OPEN_IN@`,
      etc.).
4. **When I ask for a Basil program next:**

    * Produce a **single, copy-paste-runnable** Basil file that strictly follows the Kitchen Sink style you summarized.
    * Include minimal in-code comments (concise), and use `DAW_ERR$()` checks where appropriate.
    * If a requested feature depends on a Feature Object, assume it is compiled in (don’t re-explain Cargo flags unless
      I ask).
    * Prefer a non-blocking loop with a clear exit (keypress or `DAW_STOP()`), when relevant.

## Response style

* Be concise and practical.
* No speculative features beyond what I request.
* Do not ask multiple clarification questions; pick reasonable defaults and proceed.

**Ready?**
Reply:

1. “Waiting for Kitchen Sink file” **or** a 10–15 line syntax summary extracted from the file if attached.
2. A one-liner stating you’re prepared to generate Basil examples using a particular feature library, i.e.
   `obj-audio/obj-midi/obj-daw` on request.

---

## (Optional) Quick follow-up template I’ll use after the bootstrap

> **Task:** Write a Basil program that **<goal>**.
> **Run Mode:** CLI | CGI.
> **Features:** i.e. `obj-daw` (helpers) | `obj-audio`/`obj-midi` (low-level) | both.
> **Notes:** i.e. "Use Kitchen Sink style; include `DAW_ERR$()` checks; non-blocking with clean exit."

