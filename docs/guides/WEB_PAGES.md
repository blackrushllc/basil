# Basil CGI / Web Templating Design
# =================================

# High-level approach

**In CGI mode**, treat the input file as a *template stream* with two states:

1. **Text mode** — pass bytes through to stdout (unmodified).
2. **Code mode** — parse/execute Basil code until you hit the closing delimiter, then return to Text mode.

This can be done either by:

* **Streaming interpreter** (scan → emit/execute on the fly), or
* **Precompile to Basil** (transpile the mixed template into a pure Basil program, then run it).
  *Recommendation:* start with **precompile**; it gives you cleaner error reporting, caching, and reuse of your existing Basil parser/VM.

---

# Delimiters & syntax (recommendation)

* **Primary code block:** `<?basil … ?>`
  Rationale: avoids conflicts with XML/HTML processing instructions that use plain `<?`.
* **Short alias (optional, opt-in):** `<?bas … ?>`
  Gate behind an env var/flag (e.g., `BASIL_CGI_SHORT_TAGS=1`).
* **Echo shorthand:** `<?= expr ?>`
  Equivalent to `PRINT expr` (no `BEGIN/END` inside; just a single expression).
  Keeps simple embedding of variables/expressions ergonomic.

> Avoid bare `<? … ?>` to prevent collisions with XML and other tools. If you still want it, default it **off**.

---

# What runs inside `<?basil … ?>`

* It’s normal Basil code (statements/expressions), using your existing `PRINT`, `IF/BEGIN/END`, `WHILE/BEGIN/END`, `BREAK/CONTINUE`, etc.
* Text outside of code becomes literal output (no implicit escaping). This mirrors PHP/ASP and surprises no one.

**Output policy:**

* Don’t auto-escape HTML by default (stay PHP-like). Provide a helper `h(expr)` for HTML-escape if/when desired, so users can do `<?= h(name) ?>`.

---

# Compilation / Execution model

**Precompile approach (recommended):**

1. **Scanner** walks the file and emits:

  * For text segments → `PRINT "…"`, with proper string escaping and newline preservation.
  * For `<?= expr ?>` → `PRINT (expr)`.
  * For `<?basil … ?>` → inject that code as-is.
2. Wrap the whole thing in a minimal Basil “main” (if your runtime expects it).
3. **Source map**: keep an index mapping generated line/col → original template line/col for precise error messages.
4. **Cache**: hash the template file (mtime + size or content hash) and store the compiled Basil bytecode in a cache dir (e.g., `~/.basil/cache` or `/tmp/basil-cache`). Reuse until the source changes.

**Streaming approach (alternative):**

* Maintain a simple state machine (TEXT vs CODE vs ECHO), buffering only enough to resolve the closing `?>`.
* Execute Basil snippets using an embedded evaluator.
* Harder to get great error locations and control flow, but memory friendly.

---

# Header & CGI concerns

* On first output in CGI mode, if no header has been sent, default to:
  `Content-Type: text/html; charset=utf-8` + a blank line.
* Provide an API to set/override headers *before* body output (e.g., `cgi_header("Content-Type", "application/json")`). If a header is set after body started, warn or error.

---

# Edge cases to handle

* **Delimiters inside strings/comments:** Your scanner must ignore `?>` when it appears *inside* Basil string literals or comments (only terminate on `?>` found at top scanning level in Code/Echo state).
* **Whitespace control:** Do nothing fancy at first—emit exactly what’s in the file. Later, if you want, add optional trimming (e.g., `<?- … -?>`) to strip adjacent whitespace.
* **Binary safety:** Treat the outer text as bytes; only escape what’s required when injecting into `PRINT "…"`. Use UTF-8 by default.
* **Errors:** When code inside a block fails, show “TemplateError at foo.basil.html:line N, col M” using the source map. Include a caret snippet with a few chars of context.

---

# Minimal pseudo-pipeline (precompile flavor)

* **STATE = TEXT**

  * Read until `<?=` or `<?basil` (or `<?bas` if enabled) or EOF.
  * Emit `PRINT "…";` for the literal segment (escape `\`, `"`, newlines).
* **STATE = ECHO** (`<?=`)

  * Read until next un-nested `?>`.
  * Emit `PRINT (<expr>);`
* **STATE = CODE** (`<?basil`/`<?bas`)

  * Read until next un-nested `?>`.
  * Emit the inner code as-is.

Finally, run the generated Basil program in CGI mode.

---

# Security & performance notes

* **Security**: This is server-side execution of template code; only run trusted templates. Document this clearly.
* **Performance**: The compile cache will do most of the heavy lifting. Optionally add `BASIL_CGI_CACHE=0` to disable in dev.
* **Includes/layouts** (future): Provide a simple `include("partials/header.basl")` helper that preprocesses nested templates with the same scanner. Add a depth guard to prevent cycles.

---

# Naming & file extensions

* Consider a distinct extension for mixed templates (e.g., `.basil.html` or `.basl`) to keep syntax highlighting sane and to signal “this file is CGI-templated.”
* In **CLI mode**, you can still allow running them, but it’ll just print the rendered output to stdout—handy for offline rendering.

---

# Delimiter decision (final recommendation)

* **Default on:** `<?basil … ?>` and `<?= … ?>`
* **Optional (off by default):** `<?bas … ?>` via env/flag
* **Avoid:** bare `<? … ?>` (XML collision)

This gives us PHP-level ergonomics with far fewer foot-guns, clean BEGIN/END semantics inside code blocks, and a straightforward implementation path that reuses our existing compiler/VM.
