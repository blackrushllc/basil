Love this idea. Here’s a battle-tested way to add a linter and formatter to Basil that stays fast, stable, and easy to hook into your toolchain (and leaves room for optional AI assists).

# High-level approach

* **One core library, multiple frontends.**
  Create two crates:

    * `basil_syntax` — your lossless parser + CST/AST (tokens include whitespace/comments “trivia”).
    * `basil_tools` — houses both **linter** and **formatter** engines that operate on `basil_syntax`.
      Then expose frontends:
    * `basil-lint` CLI
    * `basil-fmt` CLI
    * LSP server (diagnostics + format)
    * Optional integration hooks inside `bcc` (compiler) / `bvm` (interpreter)

* **Operate on a lossless CST.**
  The formatter must preserve comments and trivia reliably; the linter should prefer AST for semantics but use CST ranges for edits. If your current parser drops trivia, upgrade it now—formatters that reconstruct comments get brittle.

* **Stable, idempotent, incremental.**

    * Formatter must be *idempotent* (`basil-fmt` run twice yields no diff).
    * Keep a small incremental layer (hash file → skip unchanged) so “always-on” stays snappy.

* **Configurable, opt-in in pipelines.**
  Default: **off** in `bcc/bvm` (compile speed and breakage concerns). Add flags to opt in.

# What to build first (order of work)

1. **Parser tightening (lossless CST)**

    * Include exact tokens, trivia, and node ranges.
    * Export a tiny “green tree” (immutable) and “red” API for convenience (like rust-analyzer does).

2. **Formatter (MVP)**

    * Deterministic pretty-printer from CST:

        * indentation rules
        * spacing around operators
        * line breaks for statement lists and long argument lists
        * comment preservation with sensible attachment (leading/trailing)
    * CLI: `basil-fmt`

        * `--write` (default), `--check` (CI), `--stdin`/`--stdin-filepath`, `--config .basilfmt.toml`
    * Config (examples):

      ```toml
      indent_style = "space"    # "tab" | "space"
      indent_size  = 4
      max_line_len = 100
      newline_at_eof = true
      line_endings = "lf"       # "lf" | "crlf" | "native"
      space_around_operators = true
      ```
    * Goal: freeze basic formatting decisions early to avoid churn. Add knobs later.

3. **Linter (MVP)**

    * Rule framework + diagnostics + quick-fixes.
    * Start with **syntax & hygiene** rules that are parser-only (no heavy semantics):

        * trailing whitespace
        * mixed tabs/spaces
        * missing newline at EOF
        * unreachable after `END` / `EXIT` (simple control-flow scan)
        * empty blocks
        * duplicate labels
        * suspicious numeric suffixes/types (if Basil has `%`, `$`, etc.)
    * Then add light semantic checks (name resolution pass):

        * unused variables / labels
        * unused parameters
        * shadowing
        * duplicate case labels
        * always-true/false conditions (simple const fold)
    * CLI: `basil-lint`

        * `--config .basillint.toml` | `--rule allow/warn/error`
        * `--fix` (apply safe quick-fixes: remove trailing spaces, insert newline at EOF, auto-insert missing comma/semicolon where unambiguous, etc.)
        * `--format-before` to run formatter first for normalized token stream (optional).
    * Inline suppressions:

        * next-line: `REM basil-lint:disable-next-line no-unused`
        * block: `REM basil-lint:disable no-shadow ...` / `REM basil-lint:enable`

4. **LSP server**

    * Capabilities:

        * `textDocument/diagnostic` (push lint findings)
        * `textDocument/formatting` (whole-doc)
        * `textDocument/rangeFormatting` (range)
        * `textDocument/codeAction` (quick-fixes from lint engine)
    * Ship a minimal VS Code extension that launches `basil-lsp`.

5. **Compiler/Interpreter hooks (opt-in)**

    * `bcc --lint` (non-zero exit on error-level diagnostics)
    * `bcc --lint --fix`
    * `bcc --format --check`
    * `bvm --lint-on-run` for scripts
    * Respect `.basillint.toml`/`.basilfmt.toml` discovered via project root.

6. **Optional AI assist (strictly opt-in)**

    * In `basil_tools::ai_advice` implement a provider that accepts:

        * the diagnostic, the code slice (±N lines), and the Basil version
    * If the user enables AI (and your `obj-ai` env/config is present), attach **an additional “Suggested fix (AI)”** code action alongside the regular quick-fixes. Make this read-only; the human chooses to apply the patch.

# Minimal Rust scaffolding (shape, not full code)

```rust
// crates/basil_syntax/src/lib.rs
pub mod syntax;   // tokens, kinds
pub mod cst;      // lossless nodes with trivia
pub mod ast;      // ergonomic typed wrappers
pub mod parse;    // parser producing CST (+errors with ranges)

// crates/basil_tools/src/lib.rs
pub mod format;
pub mod lint;
pub mod lsp;      // optional
pub mod ai_advice; // optional; behind "ai" feature

// Formatter sketch
pub struct FormatOptions { /* from .basilfmt.toml */ }
pub fn format_document(cst: &CstNode, opts: &FormatOptions) -> EditSet { /* ... */ }

// Linter sketch
pub struct LintOptions { /* rule toggles, severities */ }
pub struct Diagnostic { pub code: String, pub message: String, pub range: TextRange, pub fixes: Vec<EditSet>, severity: Severity }
pub trait LintRule { fn name(&self) -> &'static str; fn check(&self, ast: &AstNode, ctx: &mut LintCtx); }
pub fn run_lints(ast: &AstNode, opts: &LintOptions) -> Vec<Diagnostic> { /* ... */ }
```

# CLI & flags (recommended)

* `basil-fmt`

    * `--write` | `--check` | `--diff`
    * `--stdin` `--stdin-filepath <path>`
    * `--config <file>` (default: discover upward)
* `basil-lint`

    * `--fix` | `--no-fix`
    * `--format-before`
    * `--config <file>`
    * `--report json|stylish|sarif`
* Integrations:

    * `bcc --lint[=warn|error] --fix --format[=write|check]`
    * Exit codes: 0 OK, 1 violations, 2 internal error.

# Editor & CI integration

* **VS Code**: ship the extension that calls your LSP. Add `"editor.formatOnSave": true` for Basil files.
* **Git pre-commit**:

  ```bash
  #!/usr/bin/env bash
  files=$(git diff --cached --name-only --diff-filter=ACMR | grep -E '\.basil$' || true)
  [ -z "$files" ] && exit 0
  basil-fmt --write $files
  basil-lint --fix $files
  git add $files
  ```
* **CI**: `basil-fmt --check` and `basil-lint` (no `--fix`).

# Rule ideas tailored to Basil (initial set)

* `no-trailing-space`, `newline-at-eof`, `no-mixed-indent`
* `no-empty-block`, `no-duplicate-label`, `no-unreachable-after-end/exit`
* `no-implicit-var` (if Basil prefers `DIM`)
* `no-shadow`, `no-unused-var`, `no-unused-label`
* `prefer-upper-keywords` (if your style guide wants `PRINT`, `DIM`, …)
* `consistent-string-quotes` (if language permits both)
* `magic-number` (warn outside consts)
* `max-line-length` (warnings, with formatter wrapping heuristics)

# Configuration files

* `.basillint.toml`

  ```toml
  [rules]
  no_trailing_space = "error"
  no_unused_var     = "warn"
  max_line_length   = { level = "warn", value = 100 }

  [ignore]
  paths = ["third_party/", "generated/"]
  ```
* `.basilfmt.toml` as above.

# Tests

* **Golden snapshot tests** for the formatter (input → expected output).
* **Rule fixtures** for lints (source with markers → expected diagnostics + fixes).
* **Idempotence tests**: format twice, assert unchanged.
* **Fuzz parsing** (optional later) to harden the CST.

# Performance notes

* Keep formatting decisions purely structural (token adjacency, node kinds).
* Avoid symbol-heavy passes in `--format`; reserve those for lints that need them.
* Cache per-file hashes; skip files when unchanged.

# How to stage this without breaking dev flow

1. Land lossless CST upgrades.
2. Ship `basil-fmt` with a *small* stable style; encourage `--check` in CI but don’t gate yet.
3. Add `basil-lint` with a conservative default ruleset (mostly hygiene) and `--fix`.
4. Release LSP; publish VS Code extension.
5. Add `bcc` opt-in flags.
6. (Optional) Wire AI suggestions behind `--ai` or config flag; default off.

---

If you want, I can draft ready-to-paste **Junie prompts** for:

* upgrading the parser to lossless CST,
* scaffolding `basil-fmt` (with options + tests),
* scaffolding `basil-lint` (rule framework + 5 starter rules),
* and a minimal **Basil LSP** with diagnostics + formatting.
