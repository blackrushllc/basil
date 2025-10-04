# Basil — 15‑Minute Presentation Handout

**Speaker:** Erik Olson
**Audience:** Intro CS / scripting students
**Duration:** 15 minutes (live demo included)

---

## Session goals

* Explain **what Basil is** and **why it exists**.
* Show how Basil is **easy to learn** and **AI‑friendly** to generate and edit.
* Share the **collaboration workflow** (RustRover + ChatGPT + Junie Ultimate).
* Demonstrate **CLI** and **CGI templating** with small, readable examples.

---

## What is Basil?

Basil is a tiny, modern **BASIC‑inspired** language designed to be:

* **Readable for beginners** (clear `BEGIN … END` blocks, minimal punctuation).
* Comparable to Php, Python but **simpler and more readable** for beginners.
* **Practical for AI co‑creation** (predictable patterns that tools can generate reliably).
* **Versatile in runtime**: runs as **CLI** or **CGI** (auto‑selects, or override via env var).

**Good fits:** teaching fundamentals, quick utilities, templated web pages, and AI‑assisted prototyping.

---

## Why Basil is AI‑friendly

* **Block style:** single `BEGIN … END` (no multiple END‑keywords), great for generation and review.
* **Straightforward control flow:** `IF`, `WHILE`, `BREAK`, `CONTINUE`, `TRUE`/`FALSE`.
* **Predictable templating:** PHP‑style delimiters but cleaner defaults.
* **Low ceremony:** fewer edge cases → fewer AI mistakes.

---

## How Basil was developed (collab workflow)

* **RustRover** as the IDE (custom Run configs + External Tool hotkeys/buttons).
* **ChatGPT + Junie Ultimate** for rapid drafting, codegen, and spec iterations.
* **Junie Ultimate**: self‑hosted AI, privacy‑focused, tuned for coding tasks.
* **Human review:** vetting AI output, testing, and refining.
* **Tight loop:** propose → implement → run → refine.  Lots of small tests and circular feedback.

Key takeaway: **AI accelerates**, but **human guidance and testing** shape quality.

---

## BASIC vs. Basil (quick contrasts)

* **Blocks:** `BEGIN … END` (Basil) vs. many dialects using `END IF`, `WEND`, etc.
* **Loops:** `WHILE … BEGIN … END` with `BREAK`/`CONTINUE`; `TRUE`/`FALSE` literals.
* **CGI templating:**

    * Code blocks: `<?basil … ?>` (and optional short `<?bas … ?>` if enabled).
    * Echo shorthand: `<?= expr ?>` → prints expression result.
    * Built‑in `HTML(x)` escapes for safe embedding.
* **Directives (at file start):**

    * `#CGI_NO_HEADER` (manual headers),
    * `#CGI_DEFAULT_HEADER "Content-Type: …"` (override default),
    * `#CGI_SHORT_TAGS_ON` (allow `<?bas … ?>`), plus reserved `#BASIL_DEV`, `#BASIL_DEBUG`.
* **Caching:** precompiled bytecode `.basilx` lives beside the source.

---

## Talk outline (15 minutes)

**1) Framing: what & why (2 min)**

* Basil’s purpose and audience; AI + beginner synergy.

**2) AI‑friendly design (3 min)**

* `BEGIN/END`, simple loops & booleans, predictable syntax.

**3) Collaboration story (3 min)**

* RustRover + ChatGPT + Junie workflow; examples of spec→impl cycles.

**4) BASIC vs Basil contrasts (3 min)**

* Show readability and modern touches (templating, directives).

**5) Live demos (4 min)**

* CLI hello + quick loop.
* CGI page with echo, a loop over request params, and a custom header.

---

## Demo cheat‑sheet

### CLI

1. Run a file:

   ```
   cargo run -q -p basilc --features obj-bmx -- run examples/hello.basil
   ```
2. Show a tiny loop (WHILE + BREAK) and `PRINT` output.

### CGI (templating)

* Open the live demo showing custom header + `FOREACH` over query params:
  **[https://yobasic.com/basil/cgi.basil?This=That&Moo=Cow](https://yobasic.com/basil/cgi.basil?This=That&Moo=Cow)**
* Show a minimal template shape:

  ```
#CGI_NO_HEADER
<?basil
  // Manual header mode: send headers explicitly, then a blank line
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
?>
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Basil CGI Demo</title>
</head>
<body>
  <h1>Hello, World</h1>
  <p>This page is rendered by a Basil CGI template.</p>

<h2>Request parameters</h2>
  <p>Any GET or POST parameters will be listed below.</p>
  <ul>
  <?basil
    FOR EACH p$ IN REQUEST$()
      PRINT "<li>" + HTML$(p$) + "</li>\n";
    NEXT
  ?>
  </ul>
</body>
</html>

  ```
* Mention `#CGI_NO_HEADER` if you want to emit headers yourself.

---

## Key takeaways

* **Readable + learnable**: great for first programs and quick wins.
* **AI‑centric**: easy for tools to suggest, and for humans to edit.
* **Two modes** (CLI/CGI) with one codebase; templating is familiar yet tidy.
* **Small surface area** → faster mastery, fewer foot‑guns.

---

## Optional discussion prompts

* Where could Basil fit in your projects? (scripts, teaching, small web tools)
* What features would you want next (packages, standard lib helpers, testing)?

---

## Resources

https://yobasic.com/basil/cgi.basil?moo=cow&cat=meow  (live CGI demo)

https://github.com/blackrushllc/basil (source code)

https://www.jetbrains.com/rust/download/?section=windows (RustRover IDE)

https://rust-lang.org/tools/install/ (Rust toolchain)



